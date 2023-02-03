use std::{
    collections::{HashMap, HashSet},
    net::{SocketAddr, TcpListener},
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, Once},
    time::Duration,
};

use anyhow::{anyhow, bail, Result};
use axum::{
    body::Bytes,
    extract::{BodyStream, Path},
    response::IntoResponse,
    routing::put,
    Extension, Router,
};
use bolik_migrations::rusqlite::Connection;
use bolik_sdk::{
    account::{AccNotification, AccView},
    client::ClientConfig,
    output::OutputEvent,
    timeline::card::{CardChange, CardView, ContentView},
    DefaultSdk,
};
use bolik_server::{
    router::router,
    state::build_app_state,
    state::{AppConfig, AppState},
};
use hyper::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    HeaderMap, StatusCode,
};
use tempfile::TempDir;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tracing_subscriber::EnvFilter;

static LOGGER_INIT: Once = Once::new();

pub fn setup() {
    setup_tracing();
}

fn setup_tracing() {
    LOGGER_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter(EnvFilter::new("info,bolik_server=debug,bolik_sdk=debug"))
            .init();
    });
}

pub struct TestServerResult {
    pub addr: SocketAddr,
    pub s3: S3ServerState,
    pub app: AppState,
    #[allow(unused)]
    temp_dir: TempDb,
}

impl TestServerResult {
    pub fn get_conn(&self) -> Result<Connection> {
        Ok(Connection::open(&self.temp_dir.sqlite_path)?)
    }
}

pub async fn start_server() -> Result<TestServerResult> {
    let tempdb = TempDb::new();

    // Mock S3 server
    let s3_state = Arc::new(Mutex::new(S3StateInner::default()));
    let s3_addr = start_s3_server(s3_state.clone())?;

    // Start bolik server
    let app_state = build_app_state(AppConfig {
        db_path: tempdb.sqlite_path.clone(),
        s3_creds: s3::creds::Credentials::new(
            Some("access-key"),
            Some("secret-key"),
            None,
            None,
            None,
        )?,
        s3_bucket: "local-test".into(),
        s3_endpoint: format!("http://{}", s3_addr),
        s3_region: "eu-local".into(),
        addr: SocketAddr::from(([127, 0, 0, 1], 0)),
    })
    .await?;
    let listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?)?;
    let addr = listener.local_addr()?;
    let router = router(app_state.clone());

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(router.into_make_service())
            .await
            .unwrap();
    });

    Ok(TestServerResult {
        addr,
        s3: s3_state,
        temp_dir: tempdb,
        app: app_state,
    })
}

pub struct TestSdk {
    #[allow(unused)]
    temp_dir: TempDir,
    pub sdk: DefaultSdk,
    pub output_rx: tokio::sync::broadcast::Receiver<OutputEvent>,
}

impl TestSdk {
    pub async fn expect_synced(&mut self) -> Result<()> {
        match self.output().await? {
            OutputEvent::Synced => Ok(()),
            e => Err(anyhow!("Expected Synced but got {:?}", e)),
        }
    }

    pub async fn expect_sync_failed(&mut self) -> Result<()> {
        match self.output().await.unwrap() {
            OutputEvent::SyncFailed => Ok(()),
            e => Err(anyhow!("Expected SyncFailed but got {:?}", e)),
        }
    }

    pub async fn expect_connected_account(&mut self) -> Result<AccView> {
        match self.output().await? {
            OutputEvent::ConnectedToAccount { view } => Ok(view),
            e => Err(anyhow!("Expected ConnectedToAccount but got {:?}", e)),
        }
    }

    pub async fn expect_acc_updated(&mut self) -> Result<AccView> {
        match self.output().await? {
            OutputEvent::AccUpdated { view } => Ok(view),
            e => Err(anyhow!("Expected AccUpdated but got {:?}", e)),
        }
    }

    pub async fn expect_notification(&mut self) -> Result<AccNotification> {
        match self.output().await? {
            OutputEvent::Notification(n) => Ok(n),
            e => Err(anyhow!("Expected Notification but got {:?}", e)),
        }
    }

    pub async fn expect_notifications(&mut self) -> Result<()> {
        match self.output().await? {
            OutputEvent::NotificationsUpdated => Ok(()),
            e => Err(anyhow!("Expected NotificationsUpdated but got {:?}", e)),
        }
    }

    pub async fn expect_doc_updated(&mut self) -> Result<String> {
        match self.output().await? {
            OutputEvent::DocUpdated { doc_id } => Ok(doc_id),
            e => Err(anyhow!("Expected DocUpdated but got {:?}", e)),
        }
    }

    pub async fn expect_timeline_updated(&mut self) -> Result<()> {
        match self.output().await? {
            OutputEvent::TimelineUpdated => Ok(()),
            e => Err(anyhow!("Expected TimelineUpdated but got {:?}", e)),
        }
    }

    pub async fn output(&mut self) -> Result<OutputEvent> {
        let event =
            tokio::time::timeout(Duration::from_millis(1000), self.output_rx.recv()).await??;
        Ok(event)
    }

    pub fn create_sample_card(&mut self, text: impl Into<String>) -> Result<CardView> {
        let card = self.create_card()?;
        let card = self.edit_card(&card.id, vec![CardChange::append_text(text)])?;
        self.close_card(&card.id)?;
        Ok(card)
    }

    pub async fn link_devices(&mut self, other: &mut Self) -> Result<()> {
        let share = other.get_device_share()?;
        let _ = other.output().await?;

        self.link_device(&share).await?;
        self.expect_synced().await?;

        other.sync();
        other.expect_connected_account().await?;
        other.expect_acc_updated().await?;
        Ok(())
    }

    pub fn attach_file(
        &self,
        card_id: &str,
        file_path: impl AsRef<std::path::Path>,
    ) -> Result<CardView> {
        let file = self.save_file(card_id, file_path)?;
        self.edit_card(card_id, vec![CardChange::append(ContentView::File(file))])
    }

    pub fn expect_timeline_days(&mut self, days_count: usize) -> Result<()> {
        let days = self.timeline_days(vec![])?;
        if days.len() != days_count {
            bail!("Expected {} but got {}", days_count, days.len());
        }
        Ok(())
    }

    pub fn expect_timeline_cards(&mut self, card_ids: &[&str]) -> Result<()> {
        let expect_ids: HashSet<_> = card_ids.iter().map(|id| *id).collect();
        let days = self.timeline_days(vec![])?;
        if days.is_empty() {
            bail!("Timeline is empty");
        }
        let cards = self.timeline_by_day(&days[0], vec![])?;
        let ids: HashSet<&str> = cards.cards.iter().map(|c| c.id.as_ref()).collect();
        if ids != expect_ids {
            bail!("Expected {:?} but got {:?}", expect_ids, ids);
        }
        Ok(())
    }
}

impl Deref for TestSdk {
    type Target = DefaultSdk;

    fn deref(&self) -> &Self::Target {
        &self.sdk
    }
}

impl DerefMut for TestSdk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sdk
    }
}

pub async fn run_sdk(device_name: &str, addr: &SocketAddr) -> Result<TestSdk> {
    let temp_dir = tempfile::tempdir()?;
    let db_key = bolik_sdk::generate_db_key();
    let sdk: DefaultSdk = bolik_sdk::run_with(
        temp_dir.path(),
        temp_dir.path(),
        device_name,
        db_key,
        Duration::from_millis(100),
        ClientConfig::default().with_host(format!("http://{}/api", addr)),
    )
    .await?;
    let output_rx = sdk.broadcast_subscribe();

    let mut sdk = TestSdk {
        temp_dir,
        sdk,
        output_rx,
    };
    // Expect sync to fail (we haven't uploaded any key packages)
    sdk.expect_sync_failed().await?;
    Ok(sdk)
}

struct TempDb {
    /// Temporary directory on disk
    #[allow(dead_code)]
    dir: TempDir,

    /// Sqlite database path
    sqlite_path: String,
}

impl TempDb {
    fn new() -> Self {
        // We create a temp directory so that SQLite process has permissions to write there.
        let file_name = format!("test-db-{}", rand::random::<u16>());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(&file_name);
        Self {
            dir,
            sqlite_path: format!("file:{}", path.display()),
        }
    }
}

pub type S3ServerState = Arc<Mutex<S3StateInner>>;

#[derive(Default)]
pub struct S3StateInner {
    pub blobs: HashMap<String, BlobData>,
}

pub struct BlobData {
    pub bytes: Vec<u8>,
}

fn start_s3_server(state: S3ServerState) -> Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?)?;
    let addr = listener.local_addr()?;
    let router = s3_router(state);

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(router.into_make_service())
            .await
            .unwrap();
    });

    Ok(addr)
}

fn s3_router(state: S3ServerState) -> Router {
    let state_ext = Extension(state.clone());
    let app = Router::new()
        // Example request
        // PUT /local-test/20221123/blob_484e0fbe-ccfb-407a-89be-2d1bcb135eb9_dev_1Afc21iWXL7KChgnP5gTDbxaCCYqYABffmDFWfxi1JFG8N
        .route(
            "/*path",
            put(s3_upload_file)
                .head(s3_head_file)
                .get(s3_get_file)
                .delete(s3_delete_file),
        )
        .layer(ServiceBuilder::new().layer(state_ext));
    app
}

#[axum::debug_handler]
async fn s3_upload_file(
    Extension(state): Extension<S3ServerState>,
    Path(path): Path<String>,
    mut body: BodyStream,
) -> impl IntoResponse {
    let mut buf: Vec<u8> = vec![];
    while let Some(chunk) = body.next().await {
        let Ok(bytes) = chunk else {
            tracing::warn!("Failed to read body chunk");
            return StatusCode::INTERNAL_SERVER_ERROR;
        };
        buf.extend(&bytes);
    }

    let mut state = state.lock().unwrap();
    state.blobs.insert(path, BlobData { bytes: buf });
    StatusCode::OK
}

#[axum::debug_handler]
async fn s3_head_file(
    Extension(state): Extension<S3ServerState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let state = state.lock().unwrap();
    if let Some(_blob) = state.blobs.get(&path) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

#[axum::debug_handler]
async fn s3_get_file(
    Extension(state): Extension<S3ServerState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let state = state.lock().unwrap();
    if let Some(blob) = state.blobs.get(&path) {
        let body = Bytes::from(blob.bytes.clone());

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());
        headers.insert(
            CONTENT_LENGTH,
            blob.bytes.len().to_string().parse().unwrap(),
        );
        (StatusCode::OK, headers, body)
    } else {
        let body = Bytes::new();
        (StatusCode::NOT_FOUND, HeaderMap::new(), body)
    }
}

#[axum::debug_handler]
async fn s3_delete_file(
    Extension(state): Extension<S3ServerState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let mut state = state.lock().unwrap();
    let value = state.blobs.remove(&path);
    if value.is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
