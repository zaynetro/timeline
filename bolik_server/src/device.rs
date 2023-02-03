use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension,
};
use bolik_migrations::rusqlite::{params, Connection};
use bolik_proto::sync::{response, KeyPackageMessage};
use hyper::StatusCode;
use openmls::prelude::{KeyPackage, TlsDeserializeTrait, TlsSerializeTrait};
use tracing::instrument;

use crate::{
    error::{AppError, DbContext, UserError},
    mls::{get_device_id, get_key_package_ref},
    router::CurrentDevice,
    state::{AppState, Protobuf},
};

#[axum::debug_handler]
pub async fn save_key_package(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(package): Protobuf<KeyPackageMessage>,
) -> Result<impl IntoResponse, AppError> {
    let key_package = KeyPackage::tls_deserialize(&mut package.data.as_slice())
        .map_err(|err| UserError::KeyPackageDecode(err))?;

    let device_id = get_device_id(key_package.credential())?;
    let key_package_ref = get_key_package_ref(&key_package)?;
    let credential_data = key_package
        .credential()
        .tls_serialize_detached()
        .map_err(|err| UserError::KeyPackageEncode(err))?;

    if device_id != current_device.device_id {
        return Err(UserError::KeyPackageCredMismatch.into());
    }

    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;
    txn.execute(
        r#"
INSERT INTO credentials (device_id, data) VALUES (?, ?)
    ON CONFLICT (device_id) DO NOTHING"#,
        params![device_id, credential_data],
    )
    .db_context("Insert credential")?;

    txn.execute(
        r#"
INSERT INTO unused_key_packages (ref, device_id, data) VALUES (?, ?, ?)
    ON CONFLICT (ref) DO NOTHING"#,
        params![key_package_ref, device_id, package.data],
    )
    .db_context("Insert key package")?;
    txn.commit().db_commit()?;

    tracing::trace!(%key_package_ref, "Saved KeyPackage");
    Ok(StatusCode::CREATED)
}

#[axum::debug_handler]
#[instrument(skip(app, _current_device))]
pub async fn list_packages(
    State(app): State<AppState>,
    Extension(_current_device): Extension<CurrentDevice>,
    Path(device_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let conn = app.conn.lock().unwrap();
    let packages = db_device_packages(&conn, &device_id)?;
    let response = response::DevicePackages {
        key_packages: packages,
    };
    Ok((StatusCode::OK, Protobuf(response)))
}

pub fn db_device_packages(
    conn: &Connection,
    device_id: &str,
) -> Result<Vec<KeyPackageMessage>, AppError> {
    let mut stmt = conn
        .prepare("SELECT data FROM unused_key_packages WHERE device_id = ?")
        .db_context("Find key packages (prepare)")?;
    let mut rows = stmt
        .query(params![device_id])
        .db_context("Find key packages")?;

    let mut packages = vec![];
    while let Some(row) = rows.next().db_context("Read row")? {
        let data: Vec<u8> = row.get(0).db_context("Read data")?;
        packages.push(KeyPackageMessage { data });
    }
    Ok(packages)
}
