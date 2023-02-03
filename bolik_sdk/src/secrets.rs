use std::ops::Sub;

use anyhow::anyhow;
use bolik_migrations::rusqlite::{self, Connection};
use bolik_migrations::rusqlite::{params, OptionalExtension};
use chacha20poly1305::aead::generic_array::typenum::Unsigned;
use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{Aead, AeadCore, OsRng};
use chacha20poly1305::consts::U12;
use chacha20poly1305::consts::U7;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{Duration, Utc};
use multihash::{Blake3_256, Hasher};
use openmls::prelude::{Ciphersuite, OpenMlsCryptoProvider};
use openmls_rust_crypto::RustCrypto;
use openmls_traits::key_store::{FromKeyStoreValue, OpenMlsKeyStore, ToKeyStoreValue};
use rand::RngCore;

pub(crate) const CIPHERSUITES: &[Ciphersuite] = &[
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519,
    Ciphersuite::MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519,
    Ciphersuite::MLS_256_DHKEMX448_AES256GCM_SHA512_Ed448,
];
pub(crate) const DEFAULT_CIPHERSUITE: Ciphersuite = CIPHERSUITES[0];

const CHACHA20POLY1305_NONCE_SIZE: usize = 12;

/// Cipher for encrypting/decrypting cells in the database
#[derive(Clone)]
pub struct DbCipher {
    cipher: ChaCha20Poly1305,
}

impl DbCipher {
    pub fn new(key: &chacha20poly1305::Key) -> Self {
        Self {
            cipher: ChaCha20Poly1305::new(key),
        }
    }

    fn generate_nonce() -> chacha20poly1305::Nonce {
        ChaCha20Poly1305::generate_nonce(&mut OsRng)
    }

    /// Split ciphertext into nonce and encrypted data parts
    fn ciphertext_into_parts(
        nonce_ciphertext: &[u8],
    ) -> anyhow::Result<(&chacha20poly1305::Nonce, &[u8])> {
        let nonce_size = U12::to_usize();
        if nonce_ciphertext.len() <= nonce_size {
            return Err(anyhow!("Nonce_ciphertext is too small"));
        }
        let nonce = chacha20poly1305::Nonce::from_slice(&nonce_ciphertext[..nonce_size]);
        let ciphertext = &nonce_ciphertext[nonce_size..];
        Ok((nonce, ciphertext))
    }

    /// Encrypt value and return ciphertext with nonce prefix.
    pub fn encrypt(&self, value: &[u8]) -> anyhow::Result<Vec<u8>> {
        // Encrypt
        let nonce = Self::generate_nonce();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, value)
            .map_err(|err| anyhow!("{:?}", err))?;

        // Prefix ciphertext with nonce
        let mut nonce_ciphertext = Vec::with_capacity(nonce.len() + ciphertext.len());
        nonce_ciphertext.extend(nonce);
        nonce_ciphertext.extend(ciphertext);

        Ok(nonce_ciphertext)
    }

    pub fn decrypt(&self, nonce_ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let (nonce, ciphertext) = Self::ciphertext_into_parts(&nonce_ciphertext)?;
        let value = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(value)
    }
}

/// Generate base58 ID from bytes
pub fn id_from_key(k: &[u8]) -> String {
    bs58::encode(k).into_string()
}

pub fn key_from_id(id: &str) -> anyhow::Result<Vec<u8>> {
    let key = bs58::decode(id).into_vec()?;
    Ok(key)
}

pub fn merge_nonce_ciphertext(nonce: chacha20poly1305::Nonce, ciphertext: Vec<u8>) -> Vec<u8> {
    let mut res = Vec::with_capacity(nonce.len() + ciphertext.len());
    res.extend(nonce);
    res.extend(ciphertext);
    res
}

pub fn generate_key() -> chacha20poly1305::Key {
    ChaCha20Poly1305::generate_key(&mut OsRng)
}

pub fn generate_nonce() -> chacha20poly1305::Nonce {
    ChaCha20Poly1305::generate_nonce(&mut OsRng)
}

pub fn generate_stream_nonce() -> GenericArray<u8, U7> {
    let mut nonce = [0; 7];
    (&mut OsRng).fill_bytes(&mut nonce);
    nonce.into()
}

/// Split ciphertext into nonce and encrypted data parts
pub fn ciphertext_into_parts(
    nonce_ciphertext: &[u8],
) -> anyhow::Result<(&chacha20poly1305::Nonce, &[u8])> {
    if nonce_ciphertext.len() <= CHACHA20POLY1305_NONCE_SIZE {
        return Err(anyhow!("Nonce_ciphertext is too small"));
    }

    let (nonce, ciphertext) = nonce_ciphertext.split_at(CHACHA20POLY1305_NONCE_SIZE);
    Ok((chacha20poly1305::Nonce::from_slice(nonce), ciphertext))
}

pub struct SqliteCryptoProvider<'a> {
    crypto: &'a RustCrypto,
    pub(crate) db_cipher: &'a DbCipher,
    pub(crate) conn: &'a Connection,
}

impl<'a> SqliteCryptoProvider<'a> {
    pub fn new(db_cipher: &'a DbCipher, conn: &'a Connection, crypto: &'a RustCrypto) -> Self {
        Self {
            crypto,
            db_cipher,
            conn,
        }
    }

    pub fn read_by_id<V: FromKeyStoreValue>(&self, id: &str) -> Option<V> {
        let key = key_from_id(id).ok()?;
        self.read(&key)
    }

    fn from_row<V: FromKeyStoreValue>(&self, nonce_ciphertext: &[u8]) -> Option<V> {
        let bytes = self.db_cipher.decrypt(nonce_ciphertext).ok()?;
        let value = V::from_key_store_value(bytes.as_ref()).ok()?;
        Some(value)
    }

    /// Actually delete keys from the table
    pub fn purge_deleted(&self) -> anyhow::Result<()> {
        let yesterday = Utc::now().sub(Duration::days(1));
        self.conn.execute(
            "DELETE FROM mls_keys WHERE deleted_at < ?",
            params![yesterday],
        )?;
        Ok(())
    }
}

impl<'a> OpenMlsCryptoProvider for SqliteCryptoProvider<'a> {
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;
    type KeyStoreProvider = SqliteCryptoProvider<'a>;

    fn crypto(&self) -> &Self::CryptoProvider {
        self.crypto
    }

    fn rand(&self) -> &Self::RandProvider {
        self.crypto
    }

    fn key_store(&self) -> &Self::KeyStoreProvider {
        self
    }
}

impl<'a> OpenMlsKeyStore for SqliteCryptoProvider<'a> {
    type Error = KeyStoreError;

    fn store<V: ToKeyStoreValue>(&self, k: &[u8], v: &V) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        let bytes = v
            .to_key_store_value()
            .map_err(|_| Self::Error::SerializeValue)?;
        let id = id_from_key(k);
        let nonce_ciphertext = self.db_cipher.encrypt(bytes.as_ref())?;

        self.conn.execute(
            r#"
INSERT INTO mls_keys (id, encrypted_value) VALUES (?, ?)
  ON CONFLICT (id) DO UPDATE SET encrypted_value = excluded.encrypted_value"#,
            params![id, nonce_ciphertext],
        )?;

        // let type_name = std::any::type_name::<V>();
        // match type_name {
        //     "openmls::key_packages::KeyPackageBundle" => {}
        //     "openmls::credentials::CredentialBundle" => {}
        //     _ => unreachable!(
        //         "OpenMlsKeyStore::store: Unsupported ToKeyStoreValue type={}",
        //         type_name
        //     ),
        // }

        Ok(())
    }

    fn read<V: FromKeyStoreValue>(&self, k: &[u8]) -> Option<V>
    where
        Self: Sized,
    {
        let id = id_from_key(k);
        let nonce_ciphertext: Vec<u8> = self
            .conn
            .query_row(
                "SELECT encrypted_value FROM mls_keys WHERE id = ?",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .ok()??;
        self.from_row(&nonce_ciphertext)
    }

    fn delete(&self, k: &[u8]) -> Result<(), Self::Error> {
        // Due to the app nature we might need to access delete key later on. E.g when rejoining the group due to conflict.
        // Then actually delete entries when deleted_at is older than 1 day.
        let id = id_from_key(k);
        let now = Utc::now();
        self.conn.execute(
            "UPDATE mls_keys SET deleted_at = ? WHERE id = ?",
            params![now, id],
        )?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KeyStoreError {
    #[error("Cannot serialize value")]
    SerializeValue,
    #[error("{0}")]
    Database(#[from] rusqlite::Error),
    #[error("{0}")]
    Storage(#[from] anyhow::Error),
}

fn accounts_hash_transform<T>(account_ids: &mut [String], transform: impl FnOnce(&[u8]) -> T) -> T {
    account_ids.sort();
    let mut hasher = Blake3_256::default();
    for acc_id in account_ids {
        hasher.update(acc_id.as_bytes());
        hasher.update(b",");
    }
    transform(hasher.finalize())
}

pub(crate) fn build_accounts_hash(account_ids: &mut [String]) -> String {
    accounts_hash_transform(account_ids, |digest| id_from_key(digest))
}
