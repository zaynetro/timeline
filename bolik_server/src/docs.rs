use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension,
};
use bolik_migrations::rusqlite::{params, OptionalExtension, Row, ToSql};
use bolik_proto::sync::{request, response, DeviceVectorClock};
use chrono::{DateTime, TimeZone, Utc};
use hyper::StatusCode;
use tracing::instrument;

use crate::{
    account::find_account_id,
    error::{AppError, DbContext, ServerError, UserError},
    router::CurrentDevice,
    state::{AppState, Protobuf},
};

#[axum::debug_handler]
#[instrument(skip_all, fields(doc_id = doc.id, account_id))]
pub async fn save(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(doc): Protobuf<request::DocMessage>,
) -> Result<impl IntoResponse, AppError> {
    if doc.to_account_ids.is_empty() {
        return Err(UserError::MissingField {
            field: "to_account_ids".into(),
        }
        .into());
    }

    let Some(device_clock) = doc.current_clock else {
        return Err(UserError::MissingField {
            field: "current_clock".into(),
        }
        .into());
    };

    let created_at = Utc
        .timestamp_opt(doc.created_at_sec, 0)
        .earliest()
        .ok_or(UserError::InvalidCreatedAt(doc.created_at_sec, 0))?;

    // Verify blobs have been uploaded
    if let Some(request::doc_message::Body::Encrypted(body)) = &doc.body {
        for blob_ref in &body.blob_refs {
            let row: Option<(String, Option<bool>)> = {
                let conn = app.conn.lock().unwrap();
                conn.query_row(
                    "SELECT path, uploaded FROM blobs WHERE id = ? AND device_id = ?",
                    params![blob_ref.id, blob_ref.device_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .db_context("Find blob")?
            };

            match row {
                Some((_path, Some(true))) => {
                    // All good
                    continue;
                }
                Some((path, _)) => {
                    // Verify that client uploaded the file in the end
                    tracing::trace!(path, "Checking S3 if object exists");
                    let (info, code) = app
                        .bucket
                        .head_object(&path)
                        .await
                        .map_err(ServerError::from)?;
                    tracing::trace!(code, "Version of object in S3: {:?}", info.version_id);
                    if code == 200 {
                        let conn = app.conn.lock().unwrap();
                        conn.execute(
                            "UPDATE blobs SET uploaded = 1 WHERE id = ? AND device_id = ?",
                            params![blob_ref.id, blob_ref.device_id],
                        )
                        .db_context("Mark blob uploaded")?;
                        continue;
                    }
                }
                None => {}
            };

            return Err(UserError::MissingBlob {
                blob_id: blob_ref.id.clone(),
                device_id: blob_ref.device_id.clone(),
            }
            .into());
        }
    }

    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;

    let account_id = find_account_id(&txn, &current_device.device_id)?;
    tracing::Span::current().record("account_id", &account_id);

    for to_acc_id in &doc.to_account_ids {
        tracing::trace!(%to_acc_id, counter = doc.counter, "Saving document");

        if to_acc_id == &account_id {
            // Delete all versions that this doc replaces (only for own account)
            for (device_id, counter) in &device_clock.vector {
                txn.execute(
                    r#"
DELETE
  FROM account_docs
 WHERE account_id = ?1 AND doc_id = ?2
   AND author_device_id = ?3 AND counter <= ?4"#,
                    params![account_id, doc.id, device_id, counter],
                )
                .db_context("Delete account_docs (this account)")?;
            }
        } else {
            // For other accounts delete previous doc versions from the same device
            txn.execute(
                r#"
DELETE
  FROM account_docs
 WHERE account_id = ?1 AND doc_id = ?2 AND author_device_id = ?3"#,
                params![to_acc_id, doc.id, current_device.device_id],
            )
            .db_context("Delete account_docs (other account)")?;
        }

        match &doc.body {
            Some(request::doc_message::Body::Encrypted(body)) => {
                txn.execute(
            r#"
INSERT INTO account_docs (account_id, doc_id, author_device_id, counter, secret_id, payload, payload_signature, created_at)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
  ON CONFLICT (account_id, doc_id, author_device_id) DO NOTHING"#,
                    params![
                        to_acc_id,
                        doc.id,
                        current_device.device_id,
                        doc.counter,
                        body.secret_id,
                        body.payload,
                        doc.payload_signature,
                        created_at,
                    ],
                )
                   .db_context("Insert account_doc")?;

                // Save doc blob references
                for blob_ref in &body.blob_refs {
                    txn.execute(
                        r#"
INSERT INTO doc_blobs (blob_id, device_id, account_id, doc_id, author_device_id)
  VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT (blob_id, device_id, account_id, doc_id, author_device_id) DO NOTHING"#,
                        params![
                            blob_ref.id,
                            blob_ref.device_id,
                            to_acc_id,
                            doc.id,
                            current_device.device_id,
                        ],
                    )
                    .db_context("Insert doc_blob")?;
                }
            }
            Some(request::doc_message::Body::Deleted(body)) => {
                let deleted_at = Utc
                    .timestamp_opt(body.deleted_at_sec, 0)
                    .earliest()
                    .ok_or(UserError::InvalidDeletedAt(body.deleted_at_sec, 0))?;

                txn.execute(
            r#"
INSERT INTO account_docs (account_id, doc_id, author_device_id, counter, payload_signature, created_at, deleted_at)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
  ON CONFLICT (account_id, doc_id, author_device_id) DO NOTHING"#,
                    params![
                        to_acc_id,
                        doc.id,
                        current_device.device_id,
                        doc.counter,
                        doc.payload_signature,
                        created_at,
                        deleted_at,
                    ],
                )
                   .db_context("Insert deleted account_doc")?;
            }
            None => {
                return Err(UserError::MissingField {
                    field: "doc.body".into(),
                }
                .into());
            }
        }
    }

    txn.commit().db_txn()?;
    Ok(StatusCode::CREATED)
}

#[axum::debug_handler]
#[instrument(skip_all, fields(account_id))]
pub async fn list(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(clock): Protobuf<DeviceVectorClock>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;
    let account_id = find_account_id(&txn, &current_device.device_id)?;
    tracing::Span::current().record("account_id", &account_id);
    const LIMIT: u32 = 100;

    // Select last seen counter from this device
    let last_seen_counter: Option<u64> = txn
        .query_row(
            "SELECT MAX(counter) FROM account_docs WHERE account_id = ? AND author_device_id = ?",
            params![account_id, current_device.device_id],
            |row| row.get(0),
        )
        .db_context("Select max counter")?;

    let mut docs = vec![];
    let case_clause = if clock.vector.is_empty() {
        "".to_string()
    } else {
        // Build a case clause like:
        //
        // AND CASE author_device_id WHEN ? THEN counter > ?
        //                           WHEN ? THEN counter > ?
        //                           ELSE 1
        // END
        format!(
            "AND CASE author_device_id {} ELSE 1 END",
            clock
                .vector
                .iter()
                .map(|_| " WHEN ? THEN counter > ? ")
                .collect::<String>(),
        )
    };

    // Fetch docs that are in the clock with higher counters and docs that are not in the clock.
    let query = format!(
        r#"
SELECT doc_id, author_device_id, counter, secret_id, payload, payload_signature, created_at, deleted_at
  FROM account_docs
 WHERE account_id = ? {}
 ORDER BY counter
 LIMIT {}"#,
        case_clause, LIMIT
    );
    let mut query_params: Vec<&dyn ToSql> = vec![&account_id];
    for (device_id, counter) in &clock.vector {
        query_params.push(device_id);
        query_params.push(counter);
    }

    let mut stmt = txn
        .prepare(&query)
        .db_context("Prepare account_docs query")?;
    let mut rows = stmt
        .query(query_params.as_slice())
        .db_context("Query account_docs")?;
    while let Some(row) = rows.next().db_context("Read account_doc row")? {
        let doc = read_account_docs_row(row)?;
        docs.push(doc);
    }

    let res = response::AccountDocs {
        last_seen_counter: last_seen_counter.unwrap_or(0),
        limit: LIMIT,
        docs,
    };
    Ok((StatusCode::OK, Protobuf(res)))
}

#[axum::debug_handler]
#[instrument(skip_all, fields(account_id, doc_id))]
pub async fn get_version(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Path((doc_id, author_device_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;
    let account_id = find_account_id(&txn, &current_device.device_id)?;
    tracing::Span::current().record("account_id", &account_id);

    // Find doc from author device.
    let query = r#"
SELECT doc_id, author_device_id, counter, secret_id, payload, payload_signature, created_at, deleted_at
  FROM account_docs
 WHERE account_id = ?1 AND doc_id = ?2 AND author_device_id = ?3
 LIMIT 1"#;

    let mut stmt = txn
        .prepare(&query)
        .db_context("Prepare account_docs query")?;
    let mut rows = stmt
        .query([&account_id, &doc_id, &author_device_id])
        .db_context("Query account_docs")?;

    let Some(row) = rows.next().db_context("Read account_doc row")? else {
        return Err(UserError::NotFound(format!("Doc {}", doc_id)).into());
    };

    let doc = read_account_docs_row(&row)?;
    Ok((StatusCode::OK, Protobuf(doc)))
}

fn read_account_docs_row(row: &Row) -> Result<response::DocVersion, AppError> {
    let created_at: DateTime<Utc> = row.get(6).db_context("Read created_at")?;
    let mut doc = response::DocVersion {
        doc_id: row.get(0).db_context("Read doc_id")?,
        author_device_id: row.get(1).db_context("Read author_device_id")?,
        counter: row.get(2).db_context("Read counter")?,
        payload_signature: row.get(5).db_context("Read payload_signature")?,
        created_at_sec: created_at.timestamp(),
        body: None,
    };

    let deleted_at: Option<DateTime<Utc>> = row.get(7).db_context("Read deleted_at")?;
    if let Some(deleted_at) = deleted_at {
        doc.body = Some(response::doc_version::Body::Deleted(
            response::doc_version::DeletionBody {
                deleted_at_sec: deleted_at.timestamp(),
            },
        ));
    } else {
        doc.body = Some(response::doc_version::Body::Encrypted(
            response::doc_version::EncryptedBody {
                secret_id: row.get(3).db_context("Read secret_id")?,
                payload: row.get(4).db_context("Read payload")?,
            },
        ));
    }
    Ok(doc)
}
