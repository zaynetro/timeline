use std::{error::Error, fmt::Display};

use openmls::prelude::{
    Credential, KeyPackage, OpenMlsKeyStore, Signature, TlsDeserializeTrait, TlsSerializeTrait,
};
use openmls_rust_crypto::RustCrypto;
use openmls_traits::{
    key_store::{FromKeyStoreValue, ToKeyStoreValue},
    OpenMlsCryptoProvider,
};

use crate::error::{AppError, AuthError, UserError};

pub type CryptoProvider = RustCrypto;

/// Crypto provider that doesn't remember anything.
#[derive(Default)]
pub struct VoidCryptoProvider {
    crypto: CryptoProvider,
}

impl OpenMlsCryptoProvider for VoidCryptoProvider {
    type CryptoProvider = CryptoProvider;
    type RandProvider = CryptoProvider;
    type KeyStoreProvider = Self;

    fn crypto(&self) -> &Self::CryptoProvider {
        &self.crypto
    }

    fn rand(&self) -> &Self::RandProvider {
        &self.crypto
    }

    fn key_store(&self) -> &Self::KeyStoreProvider {
        self
    }
}

impl OpenMlsKeyStore for VoidCryptoProvider {
    type Error = VoidError;

    fn store<V: ToKeyStoreValue>(&self, _k: &[u8], _v: &V) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        Ok(())
    }

    fn read<V: FromKeyStoreValue>(&self, _k: &[u8]) -> Option<V>
    where
        Self: Sized,
    {
        None
    }

    fn delete(&self, _k: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct VoidError {}

impl Display for VoidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VoidError")
    }
}

impl Error for VoidError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub fn get_device_id(credential: &Credential) -> Result<String, AppError> {
    let device_id = id_from_key(
        credential
            .signature_key()
            .tls_serialize_detached()
            .map_err(UserError::KeyPackageDecode)?
            .as_slice(),
    );
    Ok(device_id)
}

pub fn get_key_package_ref(key_package: &KeyPackage) -> Result<String, AppError> {
    let key_package_ref = id_from_key(
        key_package
            .hash_ref(&CryptoProvider::default())
            .map_err(UserError::KeyPackageHash)?
            .as_slice(),
    );
    Ok(key_package_ref)
}

pub fn read_signature(id: &str) -> Result<Signature, AppError> {
    let bytes = key_from_id(id)?;
    let signature =
        Signature::tls_deserialize(&mut bytes.as_slice()).map_err(|_| AuthError::BadSignature)?;
    Ok(signature)
}

/// Encode bytes into base58 string
fn id_from_key(k: &[u8]) -> String {
    bs58::encode(k).into_string()
}

/// Decode base58 string into bytes
fn key_from_id(id: &str) -> Result<Vec<u8>, AppError> {
    let key = bs58::decode(id)
        .into_vec()
        .map_err(UserError::Base58Decode)?;
    Ok(key)
}
