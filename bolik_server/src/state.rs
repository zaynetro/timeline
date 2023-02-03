use std::{
    net::SocketAddr,
    sync::{Arc, Mutex}, time::Duration,
};

use axum::{
    body::{self, Bytes, Full, HttpBody},
    extract::FromRequest,
    http::HeaderValue,
    response::{IntoResponse, Response},
    BoxError,
};
use bolik_migrations::rusqlite::{params, Connection, OptionalExtension};
use bolik_proto::prost::Message;
use chrono::{DateTime, Utc};
use hyper::{header, Request, StatusCode};
use s3::{Bucket, Region};
use tracing::instrument;

use crate::{
    error::{DbContext, JobError, SetupError},
    migration,
};

pub type AppState = Arc<State>;

pub struct AppConfig {
    pub db_path: String,
    pub s3_creds: s3::creds::Credentials,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: String,
    pub addr: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, SetupError> {
        let db_path = Self::optional_env("SQLITE_PATH", "timeline.db");
        let s3_bucket = Self::optional_env("S3_BUCKET", "alpha-bolik-fi");
        let s3_region = Self::optional_env("S3_REGION", "eu-central-003");
        let s3_endpoint = Self::required_env("S3_ENDPOINT")?;

        // keyID in Backblaze B2
        let access_key = Self::required_env("AWS_ACCESS_KEY_ID")?;
        // applicationKey in Backblaze B2
        let secret_key = Self::required_env("AWS_SECRET_ACCESS_KEY")?;

        let addr = match std::env::var("PORT") {
            Ok(port_str) => {
                let port: u16 = port_str.parse().map_err(|_| {
                    SetupError::invalid_env_var(format!("PORT must be a number: PORT={}", port_str))
                })?;
                SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port))
            }
            Err(_) => SocketAddr::from(([127, 0, 0, 1], 5030)),
        };

        Ok(Self {
            db_path,
            s3_creds: s3::creds::Credentials::new(
                Some(&access_key),
                Some(&secret_key),
                None,
                None,
                None,
            )?,
            s3_bucket,
            s3_region,
            s3_endpoint,
            addr,
        })
    }

    fn optional_env(key: &str, default: impl Into<String>) -> String {
        match std::env::var(key) {
            Ok(v) => v,
            Err(_) => default.into(),
        }
    }

    fn required_env(key: &str) -> Result<String, SetupError> {
        match std::env::var(key) {
            Ok(v) => Ok(v),
            Err(_) => Err(SetupError::missing_env_var(key)),
        }
    }
}

pub struct State {
    pub conn: Mutex<Connection>,
    pub bucket: Bucket,
}

impl State {
    pub fn new(conn: Connection, bucket: Bucket) -> Self {
        Self {
            conn: Mutex::new(conn),
            bucket,
        }
    }

    /// Mark unused blobs, blobs that are not referenced by doc_blobs table.
    pub fn mark_unused_blobs(&self) -> Result<(), JobError> {
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
UPDATE blobs AS b
   SET unused_since = ?
  FROM (SELECT b.id, b.device_id
          FROM blobs b
          LEFT JOIN doc_blobs db ON b.id = db.blob_id AND b.device_id = db.device_id
         WHERE db.doc_id IS NULL) as unused
 WHERE b.id = unused.id AND b.device_id = unused.device_id"#,
            [now],
        )
        .db_context("Mark unused blobs")?;
        Ok(())
    }

    /// Delete blobs that have been unused for some time.
    #[instrument(skip_all, fields(since))]
    pub async fn cleanup_blobs(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<BlobCleanupInfo, JobError> {
        let since = since.unwrap_or_else(|| Utc::now() - chrono::Duration::hours(1));
        let mut info = BlobCleanupInfo::default();

        struct BlobRow {
            id: String,
            device_id: String,
            #[allow(unused)]
            bucket: String,
            path: String,
        }

        loop {
            let blob = {
                let conn = self.conn.lock().unwrap();
                // Find unused blob
                let row = conn
                .query_row(
                    "SELECT id, device_id, bucket, path FROM blobs WHERE unused_since < ? LIMIT 1",
                    [&since],
                    |row| {
                        Ok(BlobRow {
                            id: row.get(0)?,
                            device_id: row.get(1)?,
                            bucket: row.get(2)?,
                            path: row.get(3)?,
                        })
                    },
                )
                .optional().db_context("Find unused blob")?;

                let Some(blob) = row else {
                    break;
                };

                // Verify that it is stil not used
                let doc_blob_row = conn
                    .query_row(
                        "SELECT doc_id FROM doc_blobs WHERE blob_id = ? AND device_id = ? LIMIT 1",
                        [&blob.id, &blob.device_id],
                        |_row| Ok(()),
                    )
                    .optional()
                    .db_context("Verify blob still unused")?;
                if doc_blob_row.is_some() {
                    // Blob is still referenced
                    conn.execute(
                        "UPDATE blobs SET unused_since = NULL WHERE id = ? AND device_id = ?",
                        [&blob.id, &blob.device_id],
                    )
                    .db_context("Mark blob as used")?;
                    info.restored += 1;
                    continue;
                }
                blob
            };

            tracing::debug!(
                id = blob.id,
                device_id = blob.device_id,
                "Trying to clean up blob"
            );
            match self.bucket.delete_object(&blob.path).await {
                Ok(res) if res.status_code() == 200 || res.status_code() == 404 => {
                    let conn = self.conn.lock().unwrap();
                    conn.execute(
                        "DELETE FROM blobs WHERE id = ? AND device_id = ?",
                        [&blob.id, &blob.device_id],
                    )
                    .db_context("Delete blob")?;
                    info.removed += 1;
                }
                res => {
                    tracing::warn!("S3 responded with {:?}", res);

                    {
                        // Delay blob cleanup. We don't want to get stuck in infinite loop.
                        let now = Utc::now();
                        let conn = self.conn.lock().unwrap();
                        conn.execute(
                            "UPDATE blobs SET unused_since = ?1 WHERE id = ?2 AND device_id = ?3",
                            params![now, &blob.id, &blob.device_id],
                        )
                        .db_context("Delay blob cleanup")?;
                    }

                    // Give S3 some time
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            }
        }

        Ok(info)
    }
}

pub async fn build_app_state(conf: AppConfig) -> Result<AppState, SetupError> {
    let bucket = Bucket::new(
        &conf.s3_bucket,
        Region::Custom {
            region: conf.s3_region,
            endpoint: conf.s3_endpoint,
        },
        conf.s3_creds,
    )?
    // Means that we include bucket name in the URL path.
    // Default behaviour is to use bucket name in subdomain.
    .with_path_style();

    tracing::info!("Connecting to SQLite on {}", conf.db_path);
    let conn = Connection::open(conf.db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migration::apply(&conn)?;

    let state = Arc::new(State::new(conn, bucket));
    Ok(state)
}

pub struct Protobuf<T>(pub T);

#[axum::async_trait]
impl<S, B, T> FromRequest<S, B> for Protobuf<T>
where
    T: Message + Default,
    S: Send + Sync,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = match Bytes::from_request(req, state).await {
            Ok(b) => b,
            Err(err) => {
                tracing::warn!("Failed to read body: {}", err);
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        let message = match T::decode(bytes) {
            Ok(m) => m,
            Err(err) => {
                tracing::warn!("Failed to decode message: {}", err);
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        Ok(Protobuf(message))
    }
}

impl<T> IntoResponse for Protobuf<T>
where
    T: Message,
{
    fn into_response(self) -> Response {
        let buf = self.0.encode_to_vec();
        let mut res = Response::new(body::boxed(Full::from(buf)));
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-protobuf"),
        );
        res
    }
}

#[derive(Default, Debug)]
pub struct BlobCleanupInfo {
    /// Amount of blobs that were removed
    pub removed: u32,
    /// Amount of blobs that were restored, a doc_blob reference was found
    pub restored: u32,
}
