use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension,
};
use bolik_chain::SignatureChain;
use bolik_migrations::rusqlite::{params, Connection, OptionalExtension};
use bolik_proto::sync::response;
use hyper::StatusCode;
use tracing::instrument;

use crate::{
    device::db_device_packages,
    error::{AppError, DbContext, ServerError, UserError},
    mls::CryptoProvider,
    router::CurrentDevice,
    state::{AppState, Protobuf},
};

#[axum::debug_handler]
#[instrument(skip(app, _current_device))]
pub async fn list_devices(
    State(app): State<AppState>,
    Extension(_current_device): Extension<CurrentDevice>,
    Path(account_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;
    let chain = get_account_chain(&txn, &account_id)?;

    let mut all_packages = vec![];
    let members = chain
        .members(&CryptoProvider::default())
        .map_err(ServerError::SignatureChain)?;
    for device_id in members.device_ids() {
        let packages = db_device_packages(&txn, device_id)?;
        all_packages.extend(packages);
    }

    let response = response::AccountDevices {
        chain: Some(chain.encode().map_err(ServerError::SignatureChain)?),
        key_packages: all_packages,
    };

    Ok((StatusCode::OK, Protobuf(response)))
}

pub fn find_account_id(conn: &Connection, device_id: &str) -> Result<String, AppError> {
    let account_id = conn
        .query_row(
            r#"
SELECT c.id
  FROM signature_chains c
  JOIN signature_chain_members m ON c.id = m.chain_id
 WHERE c.is_account = 1 AND m.device_id = ?"#,
            params![device_id],
            |row| row.get(0),
        )
        .optional()
        .db_context("Find account id")?;

    if let Some(id) = account_id {
        Ok(id)
    } else {
        Err(UserError::NoAccount.into())
    }
}

pub fn get_account_chain(conn: &Connection, account_id: &str) -> Result<SignatureChain, AppError> {
    let chain_bytes: Option<Vec<u8>> = conn
        .query_row(
            "SELECT chain FROM signature_chains WHERE id = ?",
            params![account_id],
            |row| row.get(0),
        )
        .optional()
        .db_context("Find chain")?;

    if let Some(bytes) = chain_bytes {
        let chain = SignatureChain::decode_bytes(&bytes).map_err(ServerError::SignatureChain)?;
        Ok(chain)
    } else {
        Err(AppError::User(UserError::NotFound("Account".into())))
    }
}
