use std::hash::Hasher;

use anyhow::Result;
use bolik_migrations::rusqlite::Connection;
use bolik_proto::sync::DeviceShareMessage;
use openmls::prelude::{Credential, KeyPackage, TlsDeserializeTrait, TlsSerializeTrait};

mod device_atom;

pub use device_atom::{DeviceAtom, DeviceCtx};
use prost::Message;
use seahash::SeaHasher;

use crate::secrets;

pub struct DeviceSettings {
    pub device_id: String,
    pub device_name: String,
    pub account_id: Option<String>,
}

pub fn query_device_settings(conn: &Connection) -> Result<DeviceSettings> {
    let settings = conn.query_row(
        "SELECT device_id, device_name, account_id FROM device_settings",
        [],
        |row| {
            Ok(DeviceSettings {
                device_id: row.get(0)?,
                device_name: row.get(1)?,
                account_id: row.get(2)?,
            })
        },
    )?;
    Ok(settings)
}

fn yrs_client_id(device_id: &str) -> yrs::block::ClientID {
    // NOTE:
    // Despite Yrs' ClientID being u64 the lib supports only u53 due to compatibility with JS.
    // In JS MAX_SAFE_INTEGER is 9_007_199_254_740_991.
    // Hence we need to convert the number back and forth.
    // NOTE: Atm, Yrs actually supports only u32 client ID and u53 is planned: https://github.com/y-crdt/y-crdt/issues/110

    let mut hasher = SeaHasher::new();
    hasher.write(device_id.as_bytes());
    let hash = hasher.finish();
    let id = hash as u32;
    id as u64
}

pub struct DeviceShare {
    pub key_package: KeyPackage,
    pub device_name: String,
}

impl DeviceShare {
    pub fn parse(share: &str) -> Result<DeviceShare> {
        let share_bytes = secrets::key_from_id(&share)?;
        let message = DeviceShareMessage::decode(share_bytes.as_slice())?;
        let package = KeyPackage::tls_deserialize(&mut message.key_package.as_slice())?;
        Ok(DeviceShare {
            key_package: package,
            device_name: message.device_name,
        })
    }
}

pub fn get_device_id(credential: &Credential) -> Result<String> {
    Ok(secrets::id_from_key(
        get_credential_id_bytes(credential)?.as_slice(),
    ))
}

pub fn get_credential_id_bytes(credential: &Credential) -> Result<Vec<u8>> {
    let bytes = credential.signature_key().tls_serialize_detached()?;
    Ok(bytes)
}
