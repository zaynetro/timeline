use openmls::prelude::{Credential, TlsSerializeTrait};
use thiserror::Error;

pub fn get_device_id(credential: &Credential) -> Result<String, DeviceError> {
    let device_id = id_from_key(
        credential
            .signature_key()
            .tls_serialize_detached()
            .map_err(DeviceError::KeyPackageDecode)?
            .as_slice(),
    );
    Ok(device_id)
}

/// Encode bytes into base58 string
pub (crate) fn id_from_key(k: &[u8]) -> String {
    bs58::encode(k).into_string()
}

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("KeyPackageDecode: {0}")]
    KeyPackageDecode(tls_codec::Error),
}
