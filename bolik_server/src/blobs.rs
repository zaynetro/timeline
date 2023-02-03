use axum::{extract::State, response::IntoResponse, Extension};
use bolik_migrations::rusqlite::params;
use bolik_proto::sync::{request, response};
use chrono::Utc;
use hyper::{header::CONTENT_LENGTH, HeaderMap, StatusCode};
use tracing::instrument;

use crate::{
    account::find_account_id,
    error::{AppError, DbContext, ServerError, UserError},
    router::CurrentDevice,
    state::{AppState, Protobuf},
};

#[axum::debug_handler]
#[instrument(skip(app, current_device))]
pub async fn presign_upload(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(payload): Protobuf<request::PresignUpload>,
) -> Result<impl IntoResponse, AppError> {
    let blob_id = payload.blob_id;
    // Blob ID is a UUID
    if !blob_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(UserError::InvalidBlobId.into());
    }

    if payload.size_bytes > 20 * 1024 * 1024 {
        return Err(UserError::BlobTooBig.into());
    }

    // Presign upload URL
    let now = Utc::now();
    let day = now.format("%Y%m%d");
    let path = format!("{}/blob_{}_dev_{}", day, blob_id, current_device.device_id);

    // Restrict Content-Length header
    let mut custom_headers = HeaderMap::with_capacity(1);
    custom_headers.insert(CONTENT_LENGTH, payload.size_bytes.into());

    let upload_url = app
        .bucket
        .presign_put(&path, 60 * 5, Some(custom_headers))
        .map_err(ServerError::from)?;

    // Store blob in the db
    {
        tracing::debug!(
            bucket = app.bucket.name,
            path,
            "Blob will be available after upload"
        );
        let conn = app.conn.lock().unwrap();
        conn.execute(
            r#"
INSERT INTO blobs (id, device_id, bucket, path, size_bytes)
  VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT (id, device_id) DO UPDATE
     SET bucket = excluded.bucket,
         path = excluded.path,
         size_bytes = excluded.size_bytes"#,
            params![
                blob_id,
                current_device.device_id,
                app.bucket.name,
                path,
                payload.size_bytes
            ],
        )
        .db_context("Insert blob")?;
    }

    let res = response::PresignedUrl { url: upload_url };
    Ok((StatusCode::OK, Protobuf(res)))
}

#[axum::debug_handler]
#[instrument(skip(app, current_device), fields(blob_id = payload.blob_id, blob_device_id = payload.device_id, account_id))]
pub async fn presign_download(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(payload): Protobuf<request::PresignDownload>,
) -> Result<impl IntoResponse, AppError> {
    let path = {
        // ACL
        let conn = app.conn.lock().unwrap();
        let account_id = find_account_id(&conn, &current_device.device_id)?;
        tracing::Span::current().record("account_id", &account_id);

        // Check this account is allowed to access the doc and that doc references the blob
        conn.query_row(
            r#"
SELECT 1
  FROM doc_blobs
 WHERE account_id = ?1 AND doc_id = ?2 AND blob_id = ?3 AND device_id = ?4"#,
            params![
                account_id,
                payload.doc_id,
                payload.blob_id,
                payload.device_id
            ],
            |_row| Ok(()),
        )
        .db_context("Find doc_payload_blob")?;

        // Find a file
        let (path, is_uploaded): (String, Option<bool>) = conn
            .query_row(
                "SELECT path, uploaded FROM blobs WHERE id = ?1 AND device_id = ?2",
                params![payload.blob_id, payload.device_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .db_context("Find blob")?;

        if !is_uploaded.unwrap_or(false) {
            return Err(UserError::MissingBlob {
                blob_id: payload.blob_id,
                device_id: payload.device_id,
            }
            .into());
        }

        path
    };

    let url = app
        .bucket
        .presign_get(path, 60 * 5, None)
        .map_err(ServerError::from)?;
    let res = response::PresignedUrl { url };
    Ok((StatusCode::OK, Protobuf(res)))
}
