use std::path::PathBuf;

use anyhow::{anyhow, Result};
use bolik_migrations::rusqlite::{config::DbConfig, params, Connection, OptionalExtension};
use bolik_proto::sync::{DeviceVectorClock, KeyPackageMessage};
use openmls::prelude::{
    CredentialBundle, CredentialType, KeyPackage, KeyPackageBundle, KeyPackageRef, OpenMlsKeyStore,
    TlsSerializeTrait,
};
use openmls_traits::OpenMlsCryptoProvider;
use prost::Message;
use uuid::Uuid;

use crate::{
    blobs,
    device::{get_credential_id_bytes, get_device_id, yrs_client_id},
    registry::{WithBackend, WithTxn},
    secrets::{self, SqliteCryptoProvider, CIPHERSUITES, DEFAULT_CIPHERSUITE},
};

pub trait DeviceCtx<'a>: WithTxn<'a> + WithBackend {}
impl<'a, T> DeviceCtx<'a> for T where T: WithTxn<'a> + WithBackend {}

#[derive(Clone)]
pub struct DeviceAtom {
    pub id: String,
    pub name: String,
    pub yrs_client_id: yrs::block::ClientID,
    pub blobs_dir: PathBuf,
}

impl DeviceAtom {
    pub fn new<'a>(
        ctx: &impl DeviceCtx<'a>,
        device_name: String,
        blobs_dir: PathBuf,
    ) -> Result<Self> {
        let row: Option<(String, String)> = ctx
            .txn()
            .query_row(
                "SELECT device_id, device_name FROM device_settings",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let backend = &ctx.backend();
        let (device_id, device_name) = match row {
            Some((id, name)) => (id, name),
            None => {
                let id = Self::init(&device_name, backend)?;
                (id, device_name)
            }
        };
        backend.purge_deleted()?;

        tracing::debug!(id = device_id, name = device_name, "Set up");
        let yrs_client_id = yrs_client_id(&device_id);

        Ok(Self {
            id: device_id,
            name: device_name,
            yrs_client_id,
            blobs_dir,
        })
    }

    /// Set up this device for the first time.
    fn init(device_name: &str, backend: &SqliteCryptoProvider) -> Result<String> {
        tracing::info!("Initializing device for the first time...");
        let identity = Uuid::new_v4().to_string();
        let credential_bundle = CredentialBundle::new(
            identity.as_bytes().to_vec(),
            CredentialType::Basic,
            DEFAULT_CIPHERSUITE.signature_algorithm(),
            backend,
        )?;
        let credential_id_bytes = get_credential_id_bytes(credential_bundle.credential())?;
        backend
            .key_store()
            .store(&credential_id_bytes, &credential_bundle)?;
        let device_id = get_device_id(credential_bundle.credential())?;

        backend.conn.execute(
            "INSERT INTO device_settings (device_id, device_name) VALUES (?, ?)",
            params![device_id, device_name],
        )?;

        Ok(device_id)
    }

    pub fn get_credential_bundle<'a>(&self, ctx: &impl DeviceCtx<'a>) -> Result<CredentialBundle> {
        let backend = &ctx.backend();
        let credential_bundle: CredentialBundle = backend
            .read_by_id(&self.id)
            .ok_or(anyhow!("Couldn't find CredentialBundle"))?;
        Ok(credential_bundle)
    }

    /// Generate several KeyPackageBundles and save them in opemls' key store.
    pub fn generate_key_packages<'a>(
        &self,
        ctx: &impl DeviceCtx<'a>,
        amount: u8,
    ) -> Result<Vec<(KeyPackageRef, KeyPackage)>> {
        let bundle = self.get_credential_bundle(ctx)?;
        let mut packages = vec![];
        for _ in 0..amount {
            packages.push(Self::generate_key_package(
                ctx.txn(),
                &bundle,
                &ctx.backend(),
            )?);
        }
        Ok(packages)
    }

    /// Generate a KeyPackageBundle and save it in opemls' key store.
    fn generate_offline_key_package(
        credential_bundle: &CredentialBundle,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<(KeyPackageRef, KeyPackage)> {
        let key_bundle = KeyPackageBundle::new(&CIPHERSUITES, credential_bundle, backend, vec![])?;
        let key_id = key_bundle.key_package().hash_ref(backend.crypto())?;
        backend
            .key_store()
            .store(key_id.value(), &key_bundle)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok((key_id, key_bundle.into_parts().0))
    }

    /// Generate a KeyPackageBundle, save it in opemls' key store and queue to be sent to backend.
    fn generate_key_package(
        conn: &Connection,
        credential_bundle: &CredentialBundle,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<(KeyPackageRef, KeyPackage)> {
        let (key_ref, package) = Self::generate_offline_key_package(credential_bundle, backend)?;
        let data = package.tls_serialize_detached()?;

        let message = KeyPackageMessage { data }.encode_to_vec();
        conn.execute(
            "INSERT INTO key_packages_queue (message) VALUES (?)",
            params![message],
        )?;
        Ok((key_ref, package))
    }

    /// Sign payload with device credential bundle's signature private key
    pub fn sign<'a>(&self, ctx: &impl DeviceCtx<'a>, payload: &[u8]) -> Result<String> {
        let sign_key = self.get_credential_bundle(ctx)?.into_parts().1;
        let signature = sign_key.sign(&ctx.backend(), payload)?;
        let signature_str = secrets::id_from_key(signature.tls_serialize_detached()?.as_slice());
        Ok(signature_str)
    }

    /// Get last seen device clocks
    pub fn get_vector_clock<'a>(&self, ctx: &impl WithTxn<'a>) -> Result<DeviceVectorClock> {
        let mut stmt = ctx
            .txn()
            .prepare("SELECT device_id, counter FROM device_vector_clock")?;
        let mut rows = stmt.query([])?;
        let mut clock = DeviceVectorClock::default();
        while let Some(row) = rows.next()? {
            clock.vector.insert(row.get(0)?, row.get(1)?);
        }
        Ok(clock)
    }

    /// Increment local clock and return new value
    pub fn increment_clock<'a>(&self, ctx: &impl WithTxn<'a>) -> Result<u64> {
        let counter: u64 = ctx.txn().query_row(
            r#"
INSERT INTO device_vector_clock (device_id, counter) VALUES (?, 1)
    ON CONFLICT (device_id) DO UPDATE SET counter = counter + 1
    RETURNING counter"#,
            [&self.id],
            |row| row.get(0),
        )?;
        Ok(counter)
    }

    /// Find last seen clock for a device
    pub fn get_clock<'a>(&self, ctx: &impl WithTxn<'a>, device_id: &str) -> Result<u64> {
        let counter = ctx
            .txn()
            .query_row(
                "SELECT counter FROM device_vector_clock WHERE device_id = ?",
                [device_id],
                |row| row.get::<_, u64>(0),
            )
            .optional()?;
        Ok(counter.unwrap_or(0))
    }

    /// Update last seen clock for a device id
    pub fn set_max_clock<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        device_id: &str,
        counter: u64,
    ) -> Result<()> {
        ctx.txn().execute(
            r#"
INSERT INTO device_vector_clock (device_id, counter) VALUES (?, ?)
    ON CONFLICT (device_id) DO UPDATE SET counter = excluded.counter"#,
            params![device_id, counter],
        )?;
        Ok(())
    }

    pub fn logout<'a>(&self, conn: &Connection) -> Result<()> {
        tracing::info!("Deleting local blobs");
        if let Err(err) = blobs::dangerous_delete_all(&self.blobs_dir) {
            tracing::warn!("Failed to delete blobs directory: {}", err);
        }

        tracing::info!("Clearing SQLite database");
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
        conn.execute("VACUUM", [])?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;

        Ok(())
    }
}
