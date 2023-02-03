#![feature(type_alias_impl_trait)]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, Result};
use chacha20poly1305::{ChaCha20Poly1305, KeySizeUser};
use client::{Client, ClientConfig};
pub use sdk::Sdk;
pub use tokio::runtime::Handle;
use tokio::runtime::Runtime;

pub mod account;
mod background;
mod blobs;
pub mod client;
mod db;
mod device;
mod documents;
mod export;
mod import;
mod input;
mod mailbox;
pub mod output;
mod registry;
mod sdk;
mod secret_group;
mod secrets;
mod signature_chain;
pub mod timeline;

pub use documents::BIN_LABEL_ID;
pub use import::ImportResult;
pub use sdk::{CreateAccLabelResult, DownloadResult, MoveToBinScope};
pub use secret_group::SecretGroupStatus;

use crate::client::HttpClient;

const DEFAULT_HOST: &'static str = std::env!("DEFAULT_BOLIK_HOST", "https://beta.bolik.fi/api");

pub type DefaultSdk = Sdk<HttpClient>;

pub fn start_runtime() -> Result<Runtime> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(3)
        .enable_all()
        .thread_name("bolik-timeline-client")
        .build()?;
    Ok(rt)
}

/// Run this SDK. Tokio Runtime should be started prior to calling this function.
pub async fn run(
    app_support_dir: impl Into<PathBuf>,
    files_dir: impl Into<PathBuf>,
    device_name: impl Into<String>,
    db_encryption_key: chacha20poly1305::Key,
) -> Result<Sdk<HttpClient>> {
    run_with(
        app_support_dir,
        files_dir,
        device_name,
        db_encryption_key,
        Duration::from_millis(100),
        ClientConfig::default().with_host(DEFAULT_HOST),
    )
    .await
}

pub async fn run_with<C>(
    app_support_dir: impl Into<PathBuf>,
    files_dir: impl Into<PathBuf>,
    device_name: impl Into<String>,
    db_encryption_key: chacha20poly1305::Key,
    sync_delay: Duration,
    client_conf: ClientConfig,
) -> Result<Sdk<C>>
where
    C: Client + 'static,
{
    let (background_tx, background_rx) = tokio::sync::mpsc::channel(20);
    let db_path = get_db_path(&app_support_dir.into());
    let blobs_dir = files_dir.into().join("Bolik Files");
    let sdk: Sdk<C> = Sdk::new(
        &db_path,
        blobs_dir,
        device_name,
        db_encryption_key,
        background_tx,
        client_conf,
    )?;

    let background_task = sdk.bg_task();
    tokio::spawn(async move { background::run(background_task, background_rx).await });
    // Give some time for subscribers to start listening
    sdk.initial_sync(sync_delay);

    Ok(sdk)
}

fn get_db_path(app_support_dir: &Path) -> String {
    let db_file = app_support_dir.join("app.db");
    // let _ = std::fs::remove_file(&db_file);
    format!("file:{}", db_file.display())
}

pub fn generate_db_key() -> chacha20poly1305::Key {
    secrets::generate_key()
}

pub fn key_from_slice(slice: &[u8]) -> Result<chacha20poly1305::Key> {
    if slice.len() != ChaCha20Poly1305::key_size() {
        Err(anyhow!(
            "Key must be of length {}",
            ChaCha20Poly1305::key_size()
        ))
    } else {
        Ok(chacha20poly1305::Key::from_slice(slice).clone())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::future::Future;
    use std::io::{Read, Write};
    use std::ops::{Deref, DerefMut};
    use std::path::{Path, PathBuf};
    use std::sync::Once;
    use std::time::Duration;

    use anyhow::{anyhow, bail, Result};
    use bolik_migrations::rusqlite::{params, Connection};
    use bolik_proto::sync::request;
    use chrono::Local;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use tracing_subscriber::EnvFilter;

    use crate::account::{AccContact, AccNotification, AccView};
    use crate::client::mock::{MockClient, MockServerArc};
    use crate::client::ClientConfig;
    use crate::documents::build_yrs_doc;
    use crate::output::OutputEvent;
    use crate::timeline::acl_doc::{AclDoc, AclRights};
    use crate::timeline::card::{
        CardBlock, CardChange, CardLabelsChange, CardText, CardTextAttrs, CardView, ContentView,
    };
    use crate::{blobs, run_with, timeline, CreateAccLabelResult, Sdk, BIN_LABEL_ID};
    use crate::{secrets, MoveToBinScope};

    static LOGGER_INIT: Once = Once::new();

    fn setup_tracing() {
        LOGGER_INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .with_env_filter(EnvFilter::new("info,bolik_sdk=info"))
                .init();
        });
    }

    struct DeviceSettings {
        device_id: String,
        account_id: Option<String>,
    }

    struct DocumentRow {
        #[allow(unused)]
        id: String,
        counter: u32,
        data: Vec<u8>,
    }

    fn query_device_settings(conn: &Connection) -> DeviceSettings {
        conn.query_row(
            "SELECT device_id, account_id FROM device_settings",
            params![],
            |row| {
                Ok(DeviceSettings {
                    device_id: row.get(0)?,
                    account_id: row.get(1)?,
                })
            },
        )
        .expect("Query device_settings")
    }

    async fn timeout<R>(future: impl Future<Output = R>) -> Result<R, tokio::time::error::Elapsed> {
        tokio::time::timeout(Duration::from_millis(1000), future).await
    }

    async fn recv_output(
        rx: &mut tokio::sync::broadcast::Receiver<OutputEvent>,
    ) -> Result<OutputEvent> {
        let event = timeout(rx.recv()).await??;
        Ok(event)
    }

    struct RunConfig {
        device_name: String,
        #[allow(unused)]
        temp_dir: TempDir,
        app_support_dir: PathBuf,
        db_key: chacha20poly1305::Key,
        mock_server: Option<MockServerArc>,
    }

    impl RunConfig {
        fn new() -> Self {
            let temp_dir = tempfile::tempdir().unwrap();
            Self {
                device_name: "Test device".to_string(),
                app_support_dir: temp_dir.path().into(),
                temp_dir,
                db_key: secrets::generate_key(),
                mock_server: None,
            }
        }

        fn with_name(mut self, device_name: impl Into<String>) -> Self {
            self.device_name = device_name.into();
            self
        }

        fn with_server(mut self, server: MockServerArc) -> Self {
            self.mock_server = Some(server);
            self
        }
    }

    async fn run_test_device() -> Result<(TestDevice, RunConfig)> {
        let config = RunConfig::new();
        let d = run_test_device_with(&config).await?;
        Ok((d, config))
    }

    async fn run_test_device_with(config: &RunConfig) -> Result<TestDevice> {
        let sdk = run_with(
            &config.app_support_dir,
            &config.app_support_dir,
            &config.device_name,
            config.db_key,
            Duration::from_millis(50),
            ClientConfig {
                host: "http://mock.local".to_string(),
                mock_server: config.mock_server.clone().unwrap_or_default(),
            },
        )
        .await?;

        let output_rx = sdk.broadcast_subscribe();
        let mut d = TestDevice { sdk, output_rx };
        d.expect_synced().await?;

        Ok(d)
    }

    struct TestDevice {
        sdk: Sdk<MockClient>,
        output_rx: tokio::sync::broadcast::Receiver<OutputEvent>,
    }

    impl TestDevice {
        async fn expect_synced(&mut self) -> Result<()> {
            let event = self.output().await?;
            if let OutputEvent::Synced = event {
                Ok(())
            } else {
                Err(anyhow!("Expected Synced but got {:?}", event))
            }
        }

        async fn expect_sync_failed(&mut self) -> Result<()> {
            let event = self.output().await?;
            if let OutputEvent::SyncFailed = event {
                Ok(())
            } else {
                Err(anyhow!("Expected SyncFailed but got {:?}", event))
            }
        }

        async fn expect_connected_to_acc(&mut self) -> Result<AccView> {
            let event = self.output().await?;
            if let OutputEvent::ConnectedToAccount { view } = event {
                Ok(view)
            } else {
                Err(anyhow!("Expected ConnectedToAccount but got {:?}", event))
            }
        }

        async fn expect_acc_updated(&mut self) -> Result<AccView> {
            let event = self.output().await?;
            if let OutputEvent::AccUpdated { view } = event {
                Ok(view)
            } else {
                Err(anyhow!("Expected AccUpdated but got {:?}", event))
            }
        }

        async fn expect_notification(&mut self) -> Result<AccNotification> {
            let event = self.output().await?;
            if let OutputEvent::Notification(n) = event {
                Ok(n)
            } else {
                Err(anyhow!("Expected Notification but got {:?}", event))
            }
        }

        async fn output(&mut self) -> Result<OutputEvent> {
            recv_output(&mut self.output_rx).await
        }

        async fn create_sample_account(&mut self) -> Result<AccView> {
            let view = self.sdk.create_account(None)?;
            self.expect_synced().await?;
            Ok(view)
        }

        fn create_sample_card(&mut self) -> Result<CardView> {
            let card = self.sdk.create_card()?;
            let card = self
                .sdk
                .edit_card(&card.id, vec![CardChange::append_text("From local")])?;
            Ok(card)
        }

        fn attach_file(&self, card_id: &str, file_path: impl AsRef<Path>) -> Result<CardView> {
            let file = self.save_file(card_id, file_path)?;
            self.edit_card(card_id, vec![CardChange::append(ContentView::File(file))])
        }
    }

    impl Deref for TestDevice {
        type Target = Sdk<MockClient>;

        fn deref(&self) -> &Self::Target {
            &self.sdk
        }
    }

    impl DerefMut for TestDevice {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.sdk
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_several_inits() {
        setup_tracing();

        let run_config = RunConfig::new();
        let test_device = {
            let d = run_test_device_with(&run_config).await.unwrap();
            assert!(d.get_account().is_none());
            d
        };

        {
            // Client can be initialized several times
            let d = run_test_device_with(&run_config).await.unwrap();
            assert!(d.get_account().is_none());
        }

        // Verify device settings are populated
        let db = Connection::open(&test_device.db_path).expect("Connect to DB");
        let settings = query_device_settings(&db);
        assert_eq!(settings.account_id, None);
        assert!(!settings.device_id.is_empty());

        // Verify only credential bundle is generated
        let mls_keys_count: u32 = db
            .query_row("SELECT count(*) FROM mls_keys", params![], |row| row.get(0))
            .expect("Query mls_keys");
        assert_eq!(mls_keys_count, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_create_account() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        assert!(d.get_account().is_none());

        let acc = d.create_account(None).expect("Create account");

        let db = Connection::open(&d.db_path).expect("Connect to DB");
        // Verify device_settings
        let settings = query_device_settings(&db);
        assert!(settings.account_id.is_some());

        // Verify key packages are saved
        let mls_keys_count: u32 = db
            .query_row("SELECT count(*) FROM mls_keys", params![], |row| row.get(0))
            .expect("Query mls_keys");
        assert_eq!(mls_keys_count, 7);

        // Verify mls_groups
        let group_id: String = db
            .query_row("SELECT id FROM mls_groups", params![], |row| row.get(0))
            .expect("MlsGroup present");
        assert_eq!(acc.id, group_id);

        // Verify output events
        d.expect_synced().await.unwrap();

        // Verify account document created and synced
        let account_row = db
            .query_row(
                "SELECT id, counter, data FROM documents",
                params![],
                |row| {
                    Ok(DocumentRow {
                        id: row.get(0)?,
                        counter: row.get(1)?,
                        data: row.get(2)?,
                    })
                },
            )
            .expect("Query documents");
        assert_eq!(account_row.counter, 1);
        assert!(!account_row.data.is_empty());

        // Verify sync queues are empty
        let key_packages_queue: u32 = db
            .query_row(
                "SELECT count(*) FROM key_packages_queue",
                params![],
                |row| row.get(0),
            )
            .expect("Query key_packages_queue");
        assert_eq!(key_packages_queue, 0);
        let push_mailbox_queue: u32 = db
            .query_row(
                "SELECT count(*) FROM push_mailbox_queue",
                params![],
                |row| row.get(0),
            )
            .expect("Query push_mailbox_queue");
        assert_eq!(push_mailbox_queue, 0);

        // Verify key packages were uploaded
        let uploaded_key_packages = d.client.uploaded_key_packages();
        assert_eq!(uploaded_key_packages.len(), 5);

        // Verify MlsMessage was pushed
        let pushed_mailbox = d.client.pushed_to_mailbox();
        assert_eq!(pushed_mailbox.len(), 2);

        // Verify account doc was uploaded
        let uploaded_docs = d.client.uploaded_docs();
        assert_eq!(uploaded_docs.len(), 2);
        assert_eq!(uploaded_docs[0].id, acc.id);

        // Account devices
        assert_eq!(1, d.account_group().unwrap().devices.len());
        assert_eq!(1, acc.devices.len());
        assert_eq!("Test device", acc.devices[0].name);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_edit_card() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let acc = d.create_sample_account().await.unwrap();

        // Create a doc
        let card = d.create_card().unwrap();

        // Should have no effect
        d.move_card_to_bin(&card.id, MoveToBinScope::ThisAccount)
            .unwrap();

        let card = d
            .edit_card(&card.id, vec![CardChange::append_text("Hello")])
            .unwrap();

        // Verify doc is saved locally
        let db = Connection::open(&d.db_path).unwrap();
        let (counter, saved_data) = db
            .query_row(
                "SELECT counter, data FROM documents WHERE id = ?",
                params![card.id],
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .unwrap();
        // Account has counter 1
        assert_eq!(counter, 3);
        assert!(!saved_data.is_empty());

        d.close_card(&card.id).unwrap();
        d.expect_synced().await.unwrap();

        // Verify secret was generated and sent
        let sent_mls = d.client.pushed_to_mailbox();
        // Initial message and doc secret message
        assert_eq!(2, sent_mls.len());

        db.query_row("SELECT 1 FROM doc_secrets", [], |_row| Ok(()))
            .expect("Doc secret was generated");

        let settings = query_device_settings(&db);
        let uploaded_docs = d.client.uploaded_docs();
        assert_eq!(uploaded_docs.len(), 3);
        assert_eq!(uploaded_docs[0].id, settings.account_id.clone().unwrap());
        assert_eq!(
            uploaded_docs[1].id,
            format!("{}/profile", settings.account_id.unwrap())
        );
        assert_eq!(uploaded_docs[2].id, card.id);
        assert_eq!(uploaded_docs[2].to_account_ids, vec![acc.id.clone()]);

        // Verify timeline
        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 1);

        let timeline_day = timeline::timeline_by_day(&db, &days[0], vec![]).unwrap();
        assert_eq!(timeline_day.cards.len(), 1);
        assert_eq!(timeline_day.cards[0].id, card.id);

        // Delete the doc
        d.move_card_to_bin(&card.id, MoveToBinScope::ThisAccount)
            .unwrap();
        d.expect_synced().await.unwrap();
        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert!(days.is_empty());

        // Restore from bin
        let restored_card = d.restore_from_bin(&card.id).unwrap();
        d.expect_synced().await.unwrap();

        // Restoration creates a new card
        assert_ne!(card.id, restored_card.id);
        assert_eq!(restored_card.labels.len(), 0);

        // Verify new card was saved
        let restored_card_counter: u64 = db
            .query_row(
                "SELECT counter FROM documents WHERE id = ?",
                params![restored_card.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(5, restored_card_counter);
        let labels_counter: u64 = db
            .query_row(
                "SELECT counter FROM documents WHERE id = ?",
                params![format!("{}/labels", restored_card.id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(6, labels_counter);

        // Verify account and local doc were uploaded
        assert_eq!(7, d.client.uploaded_docs().len());
        assert_eq!(acc.id, d.client.uploaded_docs()[0].id);
        assert_eq!(
            format!("{}/profile", acc.id),
            d.client.uploaded_docs()[1].id
        );
        assert_eq!(card.id, d.client.uploaded_docs()[2].id);
        assert_eq!(
            format!("{}/labels", card.id),
            d.client.uploaded_docs()[3].id
        );
        assert_eq!(restored_card.id, d.client.uploaded_docs()[4].id);
        assert_eq!(
            format!("{}/labels", restored_card.id),
            d.client.uploaded_docs()[5].id
        );
        assert_eq!(card.id, d.client.uploaded_docs()[6].id);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_edit_card_unicode() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Create a doc
        let card = d.create_card().unwrap();

        // Verify can insert unicode symbols
        let card = d
            .edit_card(&card.id, vec![CardChange::append_text("Привет")])
            .unwrap();
        let card = d
            .edit_card(
                &card.id,
                vec![CardChange::Insert(CardBlock {
                    position: 3,
                    view: ContentView::Text(CardText {
                        value: " звезда ★ ".into(),
                        attrs: None,
                    }),
                })],
            )
            .unwrap();

        if let ContentView::Text(t) = &card.blocks[0].view {
            assert_eq!(t.value, "При звезда ★ вет");
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[0]);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_empty_bin() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let db = Connection::open(&d.db_path).unwrap();
        let _acc = d.create_sample_account().await.unwrap();
        let CreateAccLabelResult { label, .. } = d.create_acc_label("One".to_string()).unwrap();
        d.expect_synced().await.unwrap();

        // Create a doc with labels
        let card_1 = d.create_card().unwrap();
        let card_1 = d
            .edit_card(&card_1.id, vec![CardChange::append_text("Test me")])
            .unwrap();
        let card_1 = d
            .edit_card_labels(
                &card_1.id,
                vec![CardLabelsChange::AddLabel {
                    label_id: label.id.clone(),
                }],
            )
            .unwrap();

        // Create a doc with a file
        let test_dir = tempfile::tempdir().unwrap();
        let tmp_attachment_path = test_dir.path().join("hello.txt");
        let mut tmp_attachment = std::fs::File::create(&tmp_attachment_path).unwrap();
        write!(&mut tmp_attachment, "Hello!").unwrap();

        // Attach a file to a doc
        let card_2 = d.create_card().unwrap();
        let card_2 = d.attach_file(&card_2.id, &tmp_attachment_path).unwrap();
        let ContentView::File(file) = &card_2.blocks[0].view else {
            panic!("Expected File but got {:?}", card_2.blocks[0].view)
        };
        let file_path = d
            .download_blob(&card_2.id, &file.blob_id, &file.device_id)
            .unwrap()
            .path
            .unwrap();

        // Sync cards
        d.close_card(&card_2.id).unwrap();
        d.expect_synced().await.unwrap();

        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 1);

        // Move both cards to bin
        d.move_card_to_bin(&card_1.id, MoveToBinScope::ThisAccount)
            .unwrap();
        d.move_card_to_bin(&card_2.id, MoveToBinScope::ThisAccount)
            .unwrap();
        d.expect_synced().await.unwrap();
        d.expect_synced().await.unwrap();

        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 0);
        let days = timeline::timeline_days(&db, vec![BIN_LABEL_ID.into()]).unwrap();
        assert_eq!(days.len(), 1);

        // Empty bin
        d.client.clear();
        d.empty_bin().unwrap();
        d.expect_synced().await.unwrap();

        // Verify docs are removed locally
        let doc_count: u32 = db
            .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(doc_count, 2); // Account and Profile docs

        // Verify files are removed
        assert!(!std::path::Path::new(&file_path).exists());

        // Verify blobs table is cleaned
        let blob_count: u32 = db
            .query_row("SELECT count(*) FROM blobs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 0);

        // Verify deleted docs are pushed
        let uploaded = d.client.uploaded_docs();
        let ids: HashSet<_> = uploaded.iter().map(|d| &d.id).collect();
        assert_eq!(
            ids,
            HashSet::from([
                &card_1.id,
                &format!("{}/labels", card_1.id),
                &card_2.id,
                &format!("{}/labels", card_2.id),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_linking_devices() {
        setup_tracing();

        let (mut d_a, _conf_a) = {
            let conf = RunConfig::new().with_name("A");
            let mut d = run_test_device_with(&conf).await.unwrap();
            d.create_sample_account().await.unwrap();
            (d, conf)
        };

        let (mut d_b, _conf_b) = {
            let conf = RunConfig::new()
                .with_name("B")
                .with_server(d_a.client.conf.mock_server.clone());
            let d = run_test_device_with(&conf).await.unwrap();
            (d, conf)
        };

        let uploaded_docs = d_a.client.uploaded_docs();
        assert_eq!(uploaded_docs.len(), 2);
        d_a.client.clear();

        // Generate a share on device B
        let share = d_b.get_device_share().unwrap();
        d_b.expect_synced().await.unwrap();
        // Verify key packages were uploaded
        assert_eq!(d_b.client.uploaded_key_packages().len(), 6);

        // Link new device
        d_a.link_device(&share).await.unwrap();
        d_a.expect_synced().await.unwrap();

        // Verify sent messages
        {
            let pushed_mailbox = d_a.client.pushed_to_mailbox();
            assert_eq!(pushed_mailbox.len(), 2);
            match &pushed_mailbox[0].value {
                Some(request::push_mailbox::Value::Commit(c)) => {
                    assert!(c.chain.is_some());
                    assert!(c.welcome.is_some());
                }
                _ => panic!("Expected PushMailbox::Commit"),
            }

            // Doc secrets
            match &pushed_mailbox[1].value {
                Some(request::push_mailbox::Value::Message(m)) => {
                    assert_eq!(2, m.to_device_ids.len());
                }
                _ => panic!("Expected PushMailbox::Message"),
            }

            // Account and Profile documents
            assert_eq!(2, uploaded_docs.len());
        }

        d_b.sync();

        d_b.expect_connected_to_acc().await.unwrap();
        let _ = d_b.output().await.unwrap(); // AccUpdated
        d_b.expect_synced().await.unwrap();

        // Verify account document is saved
        let (mut acc_a, acc_b) = match (d_a.get_account(), d_b.get_account()) {
            (Some(view_a), Some(view_b)) => {
                assert_eq!(view_a.id, view_b.id);

                // Assert device_settings
                let db_a = Connection::open(&d_a.db_path).unwrap();
                assert_eq!(
                    Some(view_a.id.clone()),
                    query_device_settings(&db_a).account_id
                );
                let db_b = Connection::open(&d_b.db_path).unwrap();
                assert_eq!(
                    Some(view_b.id.clone()),
                    query_device_settings(&db_b).account_id
                );
                (view_a, view_b)
            }
            _ => {
                panic!("Expected Client A and Client B to be in Account phases");
            }
        };

        // Account devices
        assert_eq!(2, d_a.account_group().unwrap().devices.len());
        assert_eq!(2, d_b.account_group().unwrap().devices.len());

        assert_eq!(2, acc_a.devices.len());
        assert_eq!(2, acc_b.devices.len());
        acc_a.devices.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!("A", acc_a.devices[0].name);
        assert_eq!("B", acc_a.devices[1].name);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_file_attachment() {
        setup_tracing();
        let (mut d, _conf) = run_test_device().await.unwrap();
        d.create_sample_account().await.unwrap();

        let db = Connection::open(&d.db_path).unwrap();

        // Create a temporary file
        let test_dir = tempfile::tempdir().unwrap();
        let tmp_attachment_path = test_dir.path().join("hello.txt");
        let mut tmp_attachment = std::fs::File::create(&tmp_attachment_path).unwrap();
        write!(&mut tmp_attachment, "Hello!").unwrap();

        // Attach a file to a doc
        let card = d.create_card().unwrap();
        let card = d.attach_file(&card.id, &tmp_attachment_path).unwrap();
        let ContentView::File(file) = &card.blocks[0].view else {
            panic!("Expected File but got {:?}", card.blocks[0].view)
        };
        assert_eq!(
            "6ZwErfGWpFBQ1GTz37jUGhZwDTcycDTEAojjJeMDRRHm",
            file.checksum
        );

        // Upload a file
        d.close_card(&card.id).unwrap();
        d.expect_synced().await.unwrap();

        let uploaded_blobs = d.client.uploaded_blobs();
        assert_eq!(1, uploaded_blobs.len());
        assert_eq!(file.blob_id, uploaded_blobs[0].0);
        let uploaded_blob = &uploaded_blobs[0].1;
        // File size (8) + authentication tag (16) = 22 bytes
        assert_eq!(uploaded_blob.len(), 22);
        // Blob is marked as synced
        let blob_ref = blobs::find_by_id(&db, &file.blob_id, &file.device_id)
            .unwrap()
            .unwrap();
        assert!(blob_ref.synced);

        // Download a file (should reuse file from disk)
        let path = d
            .download_blob(&card.id, &file.blob_id, &file.device_id)
            .unwrap();
        assert!(path.path.is_some());
        assert!(!path.download_started);

        // Verify saved file name
        let saved_path = Path::new(path.path.as_ref().unwrap());
        let saved_name = saved_path.file_name().unwrap().to_string_lossy();
        let short_id: String = card.id.chars().take(6).collect();
        assert_eq!(format!("hello (version {}).txt", short_id), saved_name);

        // Pretend that file was removed from disk and download it
        std::fs::remove_file(&path.path.unwrap()).unwrap();
        let path = d
            .download_blob(&card.id, &file.blob_id, &file.device_id)
            .unwrap();
        assert!(path.path.is_none());
        assert!(path.download_started);

        d.client
            .mock_blob_download(&file.blob_id, uploaded_blob.clone());
        let event = d.output().await.unwrap();
        if let OutputEvent::DownloadCompleted {
            blob_id,
            device_id,
            path,
        } = event
        {
            let blob_ref = blobs::find_by_id(&db, &blob_id, &device_id)
                .unwrap()
                .unwrap();
            assert_eq!(blob_ref.id, file.blob_id);
            assert!(blob_ref.synced);
            assert_eq!(blob_ref.path, path);
            assert!(Path::new(&path).exists());

            // Read file and verify contents
            let mut file = std::fs::File::open(&blob_ref.path).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            assert_eq!("Hello!", content);
        } else {
            panic!("Expected DownloadCompleted but received {:?}", event);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_big_file_attachment() {
        setup_tracing();
        let (mut d, _conf) = run_test_device().await.unwrap();
        d.create_sample_account().await.unwrap();

        let db = Connection::open(&d.db_path).unwrap();

        // Create a temporary file
        let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
        // Write enough data to span over several chunks when encrypting/decrypting
        for _ in 0..50000 {
            tmp_attachment.write(&[1]).unwrap();
        }
        let tmp_attachment_path = tmp_attachment.into_temp_path();

        // Attach a file to a doc
        let card = d.create_card().unwrap();
        let card = d.attach_file(&card.id, &tmp_attachment_path).unwrap();
        let ContentView::File(file) = &card.blocks[0].view else {
            panic!("Expected File but got {:?}", card.blocks[0].view)
        };
        assert_eq!(
            "Chpo8EQoL6C91RWQhJPU18gcLn25GUQJWMLB6przUCrT",
            file.checksum
        );
        assert_eq!(file.size_bytes, 50000);

        // Upload a file
        d.close_card(&card.id).unwrap();
        d.expect_synced().await.unwrap();

        let uploaded_blobs = d.client.uploaded_blobs();
        assert_eq!(1, uploaded_blobs.len());
        assert_eq!(file.blob_id, uploaded_blobs[0].0);
        let uploaded_blob = &uploaded_blobs[0].1;
        // File size (50000) + chunks (4) * authentication tag (16) = 50064 bytes
        assert_eq!(uploaded_blob.len(), 50064);
        // Blob is marked as synced
        let blob_ref = blobs::find_by_id(&db, &file.blob_id, &file.device_id)
            .unwrap()
            .unwrap();
        assert!(blob_ref.synced);

        // Pretend that file was removed from disk and download it
        std::fs::remove_file(&blob_ref.path).unwrap();
        let path = d
            .download_blob(&card.id, &file.blob_id, &file.device_id)
            .unwrap();
        assert!(path.path.is_none());
        assert!(path.download_started);

        d.client
            .mock_blob_download(&file.blob_id, uploaded_blob.clone());
        let event = d.output().await.unwrap();
        if let OutputEvent::DownloadCompleted {
            blob_id,
            device_id,
            path,
        } = event
        {
            let blob_ref = blobs::find_by_id(&db, &blob_id, &device_id)
                .unwrap()
                .unwrap();
            assert_eq!(blob_ref.id, file.blob_id);
            assert!(blob_ref.synced);
            assert_eq!(blob_ref.path, path);
            assert!(Path::new(&path).exists());

            // Verify file size
            let file = std::fs::File::open(&blob_ref.path).unwrap();
            assert_eq!(file.metadata().unwrap().len(), 50000);
        } else {
            panic!("Expected DownloadCompleted but received {:?}", event);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sdk_share_card() {
        setup_tracing();

        let (mut d_a, _conf_a, _acc_a) = {
            let conf = RunConfig::new().with_name("A");
            let mut d = run_test_device_with(&conf).await.unwrap();
            let acc_a = d.create_sample_account().await.unwrap();
            (d, conf, acc_a)
        };

        let (mut d_b, _conf_b, acc_b) = {
            let conf = RunConfig::new()
                .with_name("B")
                .with_server(d_a.client.conf.mock_server.clone());
            let mut d = run_test_device_with(&conf).await.unwrap();
            let acc_b = d.create_sample_account().await.unwrap();
            (d, conf, acc_b)
        };

        d_a.client.clear();
        d_b.client.clear();

        let card = d_a.create_sample_card().unwrap();

        let db_a = Connection::open(&d_a.db_path).unwrap();
        let _db_b = Connection::open(&d_b.db_path).unwrap();

        // Fail to share before adding a contact
        let share_res = d_a.edit_collaborators(
            &card.id,
            HashMap::from([(acc_b.id.clone(), Some(AclRights::Read))]),
        );
        assert!(share_res.is_err());

        // Establish MLS group with new contact
        let acc_a = d_a
            .add_contact(AccContact {
                name: "John".into(),
                account_id: acc_b.id.clone(),
            })
            .await
            .unwrap();
        d_a.expect_synced().await.unwrap();

        // Verify MLS welcome message was sent
        assert_eq!(3, d_a.client.pushed_to_mailbox().len());
        if let Some(request::push_mailbox::Value::Commit(c)) =
            &d_a.client.pushed_to_mailbox()[0].value
        {
            assert!(c.welcome.is_some());
        } else {
            panic!("Expected PushMailbox::Commit");
        }
        // Verify account was updated
        assert_eq!(1, acc_a.contacts.len());
        assert_eq!(acc_b.id, acc_a.contacts[0].account_id);
        let uploaded_docs = d_a.client.uploaded_docs();
        assert_eq!(3, uploaded_docs.len());
        assert_eq!(card.id, uploaded_docs[0].id);
        assert_eq!(acc_a.id, uploaded_docs[1].id);

        // Share a doc
        let card = d_a
            .edit_collaborators(
                &card.id,
                HashMap::from([(acc_b.id.clone(), Some(AclRights::Read))]),
            )
            .unwrap();
        d_a.expect_synced().await.unwrap();

        let acl = card.acl;
        assert_eq!(Some(&AclRights::Admin), acl.accounts.get(&acc_a.id));
        assert_eq!(Some(&AclRights::Read), acl.accounts.get(&acc_b.id));

        let uploaded = d_a.client.uploaded_docs();
        assert_eq!(4, uploaded.len());
        assert_eq!(card.id, uploaded[3].id);
        let mut expected_participants = vec![acc_a.id.clone(), acc_b.id.clone()];
        expected_participants.sort();
        assert_eq!(expected_participants, uploaded[3].to_account_ids);

        // Verify secret was sent to both accounts
        assert_eq!(3, d_a.client.pushed_to_mailbox().len());
        let accounts_hash = secrets::build_accounts_hash(&mut [acc_a.id.clone(), acc_b.id.clone()]);
        db_a.query_row(
            "SELECT 1 FROM doc_secrets WHERE accounts_hash = ?",
            params![accounts_hash],
            |_row| Ok(()),
        )
        .expect("Secret present");

        // Receive the share on device b
        d_b.sync();
        let _ = d_b.output().await.unwrap(); // Contact request notification
        let _ = d_b.output().await.unwrap(); // Card share notification
        d_b.expect_synced().await.unwrap();

        // Verify the card
        let card_b = d_b.get_card(&card.id).unwrap();
        let acl_b = card_b.acl;
        assert_eq!(Some(&AclRights::Admin), acl_b.accounts.get(&acc_a.id));
        assert_eq!(Some(&AclRights::Read), acl_b.accounts.get(&acc_b.id));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_filter_labels() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Create cards
        let card_1 = d.create_card().unwrap();
        let card_2 = d.create_card().unwrap();

        // Add label
        let CreateAccLabelResult { label, .. } = d.create_acc_label("One".to_string()).unwrap();
        let card_1 = d
            .edit_card(&card_1.id, vec![CardChange::append_text("First")])
            .unwrap();
        let card_1 = d
            .edit_card_labels(
                &card_1.id,
                vec![CardLabelsChange::AddLabel {
                    label_id: label.id.clone(),
                }],
            )
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let card_2 = d
            .edit_card(&card_2.id, vec![CardChange::append_text("Second")])
            .unwrap();

        // Verify timeline (no label filter)
        let db = Connection::open(&d.db_path).unwrap();
        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 1);
        let timeline_day = timeline::timeline_by_day(&db, &days[0], vec![]).unwrap();
        assert_eq!(timeline_day.cards.len(), 2);
        assert_eq!(timeline_day.cards[0].id, card_2.id);
        assert_eq!(timeline_day.cards[1].id, card_1.id);

        // Verify timeline (label filter)
        let days = timeline::timeline_days(&db, vec![label.id.clone()]).unwrap();
        assert_eq!(days.len(), 1);
        let timeline_day =
            timeline::timeline_by_day(&db, &days[0], vec![label.id.clone()]).unwrap();
        assert_eq!(timeline_day.cards.len(), 1);
        assert_eq!(timeline_day.cards[0].id, card_1.id);
        assert_eq!(timeline_day.cards[0].labels.len(), 1);

        // Verify timeline (unknown label filter)
        let days = timeline::timeline_days(&db, vec!["Unknown".to_string()]).unwrap();
        assert_eq!(days.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_export_single_card() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Create card
        let card_1 = d.create_card().unwrap();
        let CreateAccLabelResult { label, .. } = d.create_acc_label("One".to_string()).unwrap();
        let card_1 = {
            let mut changes = vec![CardChange::append_text(
                "Here is a sample card.\nMultiline paragraph.\n",
            )];
            changes.extend(CardChange::append_task("Task 1", true));
            changes.extend(CardChange::append_task("Task 2", false));
            changes.extend(CardChange::append_text_block("Item 1", "ul"));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "https://bolik.tech/item-2".into(),
                attrs: Some(CardTextAttrs {
                    link: Some("https://bolik.tech/item-2".into()),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "\n".into(),
                attrs: Some(CardTextAttrs {
                    block: Some("ul".into()),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text("Another paragraph.\n"));
            changes.extend(CardChange::append_text_block("Number 1", "ol"));
            changes.extend(CardChange::append_text_block("Number 2", "ol"));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "Number".into(),
                attrs: Some(CardTextAttrs {
                    bold: Some(true),
                    ..Default::default()
                }),
            })));
            changes.extend(CardChange::append_text_block(" 3", "ol"));
            changes.push(CardChange::append_text("New paragraph with "));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "link".into(),
                attrs: Some(CardTextAttrs {
                    link: Some("https://bolik.tech".into()),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text(" "));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "bold".into(),
                attrs: Some(CardTextAttrs {
                    bold: Some(true),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text(" "));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "italic".into(),
                attrs: Some(CardTextAttrs {
                    italic: Some(true),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text(" "));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "underline".into(),
                attrs: Some(CardTextAttrs {
                    underline: Some(true),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text(" "));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "strikethrough".into(),
                attrs: Some(CardTextAttrs {
                    strikethrough: Some(true),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text("\n"));
            changes.push(CardChange::append(ContentView::Text(CardText {
                value: "all".into(),
                attrs: Some(CardTextAttrs {
                    bold: Some(true),
                    italic: Some(true),
                    underline: Some(true),
                    strikethrough: Some(true),
                    ..Default::default()
                }),
            })));
            changes.push(CardChange::append_text("\n"));
            changes.extend(CardChange::append_text_heading("Heading", 1));
            changes.push(CardChange::append_text("\n"));
            changes.extend(CardChange::append_text_heading("Sub-Heading", 2));
            d.edit_card(&card_1.id, changes).unwrap()
        };
        let card_1 = d
            .edit_card_labels(
                &card_1.id,
                vec![CardLabelsChange::AddLabel {
                    label_id: label.id.clone(),
                }],
            )
            .unwrap();

        // Export card
        let exported = d.export_card(&card_1.id).await.unwrap();
        assert_eq!(exported.id, card_1.id);
        assert_eq!(exported.files.len(), 0);

        let lines: Vec<_> = exported.content.split('\n').collect();
        println!("{}", exported.content);
        // Verify metadata
        assert_eq!(lines[0], "# Bolik card v2");
        assert_eq!(lines[4], "* Labels: One");

        // Verify content
        let card_content = lines.iter().skip(7).cloned().collect::<Vec<_>>().join("\n");
        assert_eq!(
            card_content,
            r#"
Here is a sample card.
Multiline paragraph.

* [x] Task 1
* [ ] Task 2

* Item 1
* <https://bolik.tech/item-2>

Another paragraph.

1. Number 1
1. Number 2
1. **Number** 3

New paragraph with <https://bolik.tech> **bold** _italic_ <ins>underline</ins> ~~strikethrough~~
<ins>**_~~all~~_**</ins>

# Heading

## Sub-Heading

"#
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_export_data() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Create cards
        let card_1 = d.create_card().unwrap();
        let CreateAccLabelResult { label, .. } = d.create_acc_label("One".to_string()).unwrap();
        let card_1 = d
            .edit_card(&card_1.id, vec![CardChange::append_text("First")])
            .unwrap();
        let card_1 = d
            .edit_card_labels(
                &card_1.id,
                vec![CardLabelsChange::AddLabel {
                    label_id: label.id.clone(),
                }],
            )
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let card_2 = d.create_card().unwrap();
        let card_2 = d
            .edit_card(&card_2.id, vec![CardChange::append_text("Second")])
            .unwrap();

        // Export cards
        let out_dir = tempfile::tempdir().unwrap();
        d.export_cards_to_dir(out_dir.path()).await.unwrap();

        // Assert files
        let mut dirs = std::fs::read_dir(out_dir.path()).unwrap();
        let export_dir = dirs.next().unwrap().unwrap();
        assert!(export_dir
            .file_name()
            .to_str()
            .unwrap()
            .contains("Bolik Timeline export"));

        // There should be two files and one "Files" dir
        let files: Result<Vec<_>, _> = std::fs::read_dir(export_dir.path()).unwrap().collect();
        let files = files.unwrap();
        assert_eq!(3, files.len());

        let card_1_short_id: String = card_1.id.chars().take(6).collect();
        let card_2_short_id: String = card_2.id.chars().take(6).collect();

        for f in &files {
            let file_name = f.file_name();
            let name = file_name.to_str().unwrap();
            if name.contains(&card_1_short_id) {
                // Should have text and labels
                let mut file = std::fs::File::open(f.path()).unwrap();
                let mut contents = String::new();
                file.read_to_string(&mut contents).unwrap();

                assert!(contents.contains(&format!("* ID: {}", card_1.id)));
                assert!(contents.contains("First"));
                assert!(contents.contains("* Labels: One"));
            } else if name.contains(&card_2_short_id) {
                // Should have text and tasks
                let mut file = std::fs::File::open(f.path()).unwrap();
                let mut contents = String::new();
                file.read_to_string(&mut contents).unwrap();

                assert!(contents.contains(&format!("* ID: {}", card_2.id)));
                assert!(contents.contains("Second"));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_data_with_files() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Create card
        let card = d.create_card().unwrap();

        // Attach two files
        let test_dir = tempfile::tempdir().unwrap();
        let path_1 = test_dir.path().join("hello.txt");
        let mut file_1 = std::fs::File::create(&path_1).unwrap();
        writeln!(&mut file_1, "Hello!").unwrap();

        let path_2 = test_dir.path().join("second.txt");
        let mut file_2 = std::fs::File::create(&path_2).unwrap();
        writeln!(&mut file_2, "Second").unwrap();

        let card = d.attach_file(&card.id, &path_1).unwrap();
        let card = d.attach_file(&card.id, &path_2).unwrap();
        assert_eq!(2, card.blocks.len());

        // Sync
        d.close_card(&card.id).unwrap();
        d.expect_synced().await.unwrap();

        // Delete second local file
        let app_file_2 = if let ContentView::File(f) = &card.blocks[1].view {
            f
        } else {
            panic!("Expected File but got {:?}", card.blocks[1].view)
        };
        let app_path_2 = d
            .download_blob(&card.id, &app_file_2.blob_id, &app_file_2.device_id)
            .unwrap();
        assert!(app_path_2.path.is_some());
        assert!(!app_path_2.download_started);
        std::fs::remove_file(&app_path_2.path.unwrap()).unwrap();

        // Export data
        let out_dir = tempfile::tempdir().unwrap();
        d.export_cards_to_dir(out_dir.path()).await.unwrap();

        // Second file should be downloaded from remote
        assert_eq!(
            vec![app_file_2.blob_id.clone()],
            d.client.downloaded_blobs()
        );

        // Assert markdown
        let export_dir = out_dir.path().join(format!(
            "Bolik Timeline export {}",
            Local::now().format("%Y-%m-%d")
        ));
        let short_id = card.id.chars().take(6).collect::<String>();
        let md_path = export_dir.join(format!(
            "{} ({}).md",
            card.created_at.format("%Y-%m-%dT%H:%M:%S"),
            &short_id,
        ));
        assert!(md_path.exists());

        let mut md_file = std::fs::File::open(&md_path).unwrap();
        let mut md_contents = String::new();
        md_file.read_to_string(&mut md_contents).unwrap();
        println!("{}", md_contents);

        assert!(md_contents.contains(&format!("* ID: {}", card.id)));
        assert!(md_contents.contains(&format!(
            "* [File:hello.txt](./Files/hello%20%28version%20{}",
            short_id
        )));
        assert!(md_contents.contains("* [File:second.txt]"));

        // Assert attachments
        let export_files: Result<Vec<_>, _> = std::fs::read_dir(export_dir.join("Files"))
            .unwrap()
            .collect();
        let mut export_files = export_files.unwrap();
        assert_eq!(2, export_files.len());
        export_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        assert_eq!(
            format!("hello (version {}).txt", short_id),
            export_files[0].file_name().to_string_lossy().to_string()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_data_v1() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Import a test dir with dummy data
        d.import_data("../test_data/bolik_export_1").unwrap();
        d.expect_synced().await.unwrap();

        // Verify saved cards
        let db = Connection::open(&d.db_path).unwrap();
        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0], "2022-10-07");

        let timeline_day = timeline::timeline_by_day(&db, &days[0], vec![]).unwrap();
        let card = &timeline_day.cards[0];
        assert_eq!(card.id, "45941d0a-7836-443b-a430-a9518eca56b9");

        if let ContentView::Text(t) = &card.blocks[0].view {
            assert_eq!(
                t.value,
                r#"This is a sample card.
The text spawns multiple lines

and has empty lines.

Share this card with Sam"#
            );
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[0]);
        }

        if let ContentView::Text(t) = &card.blocks[1].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.checked, Some(true));
            assert_eq!(attrs.block, Some("cl".to_string()));
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[1]);
        }
        if let ContentView::Text(t) = &card.blocks[2].view {
            assert_eq!(t.value, "Something else");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[2]);
        }
        if let ContentView::Text(t) = &card.blocks[3].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.checked, None);
            assert_eq!(attrs.block, Some("cl".to_string()));
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[3]);
        }
        if let ContentView::Text(t) = &card.blocks[4].view {
            assert_eq!(t.value, "\n");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[4]);
        }

        if let ContentView::File(f) = &card.blocks[5].view {
            assert_eq!(f.name.as_ref().unwrap(), "hello world.txt");
            let file_path = d.get_file_path(&f.blob_id).unwrap();
            assert!(file_path.is_some());
        } else {
            panic!("Expected ContentView::File but got {:?}", card.blocks[5]);
        }

        if let ContentView::Text(t) = &card.blocks[6].view {
            assert_eq!(t.value, "\nThe End.\n");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", card.blocks[6]);
        }

        // Verify labels
        let acc = d.get_account().unwrap();
        let label_names: HashSet<&str> = acc.labels.iter().map(|l| l.name.as_ref()).collect();
        assert_eq!(label_names, HashSet::from(["Testing", "Writing"]));
        assert_eq!(card.labels.len(), 2);

        // Should be able to find by label
        let testing_label = acc.labels.iter().find(|l| l.name == "Testing").unwrap();
        let timeline_day =
            timeline::timeline_by_day(&db, &days[0], vec![testing_label.id.clone()]).unwrap();
        assert_eq!(timeline_day.cards.len(), 1);
        assert_eq!(
            timeline_day.cards[0].id,
            "45941d0a-7836-443b-a430-a9518eca56b9"
        );

        // Try to append text
        d.edit_card(&card.id, vec![CardChange::append_text("\nMore.")])
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_data_v2() {
        setup_tracing();
        let (mut d, _c) = run_test_device().await.unwrap();
        let _acc = d.create_sample_account().await.unwrap();

        // Import a test dir with dummy data
        let res = d.import_data("../test_data/bolik_export_2").unwrap();
        assert_eq!(res.imported, 1);
        assert_eq!(res.duplicates, Vec::<String>::new());
        assert_eq!(res.failed, Vec::<String>::new());
        d.expect_synced().await.unwrap();

        // Verify saved cards
        let db = Connection::open(&d.db_path).unwrap();
        let days = timeline::timeline_days(&db, vec![]).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0], "2022-12-26");

        let timeline_day = timeline::timeline_by_day(&db, &days[0], vec![]).unwrap();
        let card = &timeline_day.cards[0];
        assert_eq!(card.id, "9aa6b40a-d8c8-4bf0-8b36-a93436b14487");

        fn expect_text(blocks: &Vec<CardBlock>, index: usize) -> Result<&CardText> {
            if let ContentView::Text(t) = &blocks[index].view {
                Ok(t)
            } else {
                Err(anyhow!(
                    "Expected ContentView::Text but got {:?}",
                    blocks[index]
                ))
            }
        }

        fn expect_block(
            blocks: &Vec<CardBlock>,
            index: usize,
            block: &str,
        ) -> Result<CardTextAttrs> {
            let t = expect_text(blocks, index)?;
            if t.value != "\n" {
                bail!("Expected newline but got={}", t.value);
            }

            let attrs = t.attrs.clone().unwrap();
            if attrs.block != Some(block.into()) {
                bail!("Expected block={} but got={:?}", block, attrs.block);
            }
            Ok(attrs)
        }

        let t = expect_text(&card.blocks, 0).unwrap();
        assert_eq!(t.value, "Hello world!\nMultiline paragraph.\nItem 1");
        assert_eq!(t.attrs, None);

        expect_block(&card.blocks, 1, "ul").unwrap();

        let t = expect_text(&card.blocks, 2).unwrap();
        assert_eq!(t.value, "https://bolik.tech/item-2");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.link, Some("https://bolik.tech/item-2".into()));

        expect_block(&card.blocks, 3, "ul").unwrap();

        let t = expect_text(&card.blocks, 4).unwrap();
        assert_eq!(t.value, "\nNumber 1");
        assert_eq!(t.attrs, None);

        expect_block(&card.blocks, 5, "ol").unwrap();

        let t = expect_text(&card.blocks, 6).unwrap();
        assert_eq!(t.value, "Number");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.bold, Some(true));

        let t = expect_text(&card.blocks, 7).unwrap();
        assert_eq!(t.value, " 2");
        assert_eq!(t.attrs, None);

        expect_block(&card.blocks, 8, "ol").unwrap();

        let t = expect_text(&card.blocks, 9).unwrap();
        assert_eq!(t.value, "\nSomething in between.\nTask 1");
        assert_eq!(t.attrs, None);

        let attrs = expect_block(&card.blocks, 10, "cl").unwrap();
        assert_eq!(attrs.checked, None);

        let t = expect_text(&card.blocks, 11).unwrap();
        assert_eq!(t.value, "Task 2");
        assert_eq!(t.attrs, None);

        let attrs = expect_block(&card.blocks, 12, "cl").unwrap();
        assert_eq!(attrs.checked, Some(true));

        let t = expect_text(&card.blocks, 13).unwrap();
        assert_eq!(t.value, "\nNew paragraph with ");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 14).unwrap();
        assert_eq!(t.value, "https://bolik.tech");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.link, Some("https://bolik.tech".into()));

        let t = expect_text(&card.blocks, 15).unwrap();
        assert_eq!(t.value, " ");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 16).unwrap();
        assert_eq!(t.value, "bold");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.bold, Some(true));

        let t = expect_text(&card.blocks, 17).unwrap();
        assert_eq!(t.value, " ");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 18).unwrap();
        assert_eq!(t.value, "italic");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.italic, Some(true));

        let t = expect_text(&card.blocks, 19).unwrap();
        assert_eq!(t.value, " ");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 20).unwrap();
        assert_eq!(t.value, "underline");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.underline, Some(true));

        let t = expect_text(&card.blocks, 21).unwrap();
        assert_eq!(t.value, " ");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 22).unwrap();
        assert_eq!(t.value, "strikethrough");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.strikethrough, Some(true));

        let t = expect_text(&card.blocks, 23).unwrap();
        assert_eq!(t.value, "\n");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 24).unwrap();
        assert_eq!(t.value, "all");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.bold, Some(true));
        assert_eq!(attrs.italic, Some(true));
        assert_eq!(attrs.underline, Some(true));
        assert_eq!(attrs.strikethrough, Some(true));

        let t = expect_text(&card.blocks, 25).unwrap();
        assert_eq!(t.value, "\nHeading");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 26).unwrap();
        assert_eq!(t.value, "\n");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.heading, Some(1));

        let t = expect_text(&card.blocks, 27).unwrap();
        assert_eq!(t.value, "Sub-Heading");
        assert_eq!(t.attrs, None);

        let t = expect_text(&card.blocks, 28).unwrap();
        assert_eq!(t.value, "\n");
        let attrs = t.attrs.clone().unwrap();
        assert_eq!(attrs.heading, Some(2));

        if let ContentView::File(f) = &card.blocks[29].view {
            assert_eq!(f.name.as_ref().unwrap(), "hello-world.txt");
            assert_eq!(f.checksum, "DnC4z1jDcVJu6SqbKksaVB41Qc2iBLKHhQA657d9CTvp");
            let file_path = d.get_file_path(&f.blob_id).unwrap();
            assert!(file_path.is_some());
        } else {
            panic!("Expected ContentView::File but got {:?}", card.blocks[29]);
        }

        // Verify labels
        let acc = d.get_account().unwrap();
        let label_names: HashSet<&str> = acc.labels.iter().map(|l| l.name.as_ref()).collect();
        assert_eq!(label_names, HashSet::from(["Example", "Bolik"]));
        assert_eq!(card.labels.len(), 2);

        // Should be able to find by label
        let testing_label = acc.labels.iter().find(|l| l.name == "Bolik").unwrap();
        let timeline_day =
            timeline::timeline_by_day(&db, &days[0], vec![testing_label.id.clone()]).unwrap();
        assert_eq!(timeline_day.cards.len(), 1);
        assert_eq!(
            timeline_day.cards[0].id,
            "9aa6b40a-d8c8-4bf0-8b36-a93436b14487"
        );

        // Try to append text
        d.edit_card(&card.id, vec![CardChange::append_text("\nMore.")])
            .unwrap();

        // Import again --> should report duplicates
        let res = d.import_data("../test_data/bolik_export_2").unwrap();
        assert_eq!(res.imported, 0);
        assert_eq!(
            res.duplicates,
            vec!["2022-12-26T13:06:46 (9aa6b4).md".to_string()]
        );
        assert_eq!(res.failed, Vec::<String>::new());
        d.expect_synced().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sdk_profile_acl() {
        setup_tracing();

        // Account 1: Device A and B
        // Account 2: Device C
        // Account 3: Device D

        let (mut sdk_a, _conf_a, _acc_1) = {
            let conf = RunConfig::new().with_name("A");
            let mut d = run_test_device_with(&conf).await.unwrap();
            let acc = d.create_sample_account().await.unwrap();
            (d, conf, acc)
        };

        let (mut sdk_b, _conf_b, _acc_1) = {
            let conf = RunConfig::new()
                .with_name("B")
                .with_server(sdk_a.client.conf.mock_server.clone());
            let mut d = run_test_device_with(&conf).await.unwrap();

            let share = d.get_device_share().unwrap();
            d.expect_synced().await.unwrap();
            sdk_a.link_device(&share).await.unwrap();
            sdk_a.expect_synced().await.unwrap();

            d.sync();
            d.expect_connected_to_acc().await.unwrap();
            let acc = d.expect_acc_updated().await.unwrap();
            d.expect_synced().await.unwrap();

            (d, conf, acc)
        };

        let (mut sdk_c, _conf_c, acc_2) = {
            let conf = RunConfig::new()
                .with_name("C")
                .with_server(sdk_a.client.conf.mock_server.clone());
            let mut d = run_test_device_with(&conf).await.unwrap();
            let acc = d.create_sample_account().await.unwrap();
            (d, conf, acc)
        };

        let (mut sdk_d, _conf_d, acc_3) = {
            let conf = RunConfig::new()
                .with_name("D")
                .with_server(sdk_a.client.conf.mock_server.clone());
            let mut d = run_test_device_with(&conf).await.unwrap();
            let acc = d.create_sample_account().await.unwrap();
            (d, conf, acc)
        };

        let db_a = Connection::open(&sdk_a.db_path).unwrap();
        let db_b = Connection::open(&sdk_b.db_path).unwrap();
        let db_c = Connection::open(&sdk_c.db_path).unwrap();
        let db_d = Connection::open(&sdk_d.db_path).unwrap();

        // Add account 2 and 3 as contacts
        let _acc_1 = sdk_a
            .add_contact(AccContact {
                account_id: acc_2.id.clone(),
                name: "".into(),
            })
            .await
            .unwrap();
        sdk_a.expect_synced().await.unwrap();
        let acc_1 = sdk_a
            .add_contact(AccContact {
                account_id: acc_3.id.clone(),
                name: "".into(),
            })
            .await
            .unwrap();
        sdk_a.expect_synced().await.unwrap();

        sdk_b.sync();
        sdk_b.expect_acc_updated().await.unwrap();
        sdk_b.expect_synced().await.unwrap();

        sdk_c.sync();
        let _ = sdk_c.expect_notification().await.unwrap(); // Contact request notification
        sdk_c.expect_synced().await.unwrap();

        sdk_d.sync();
        let _ = sdk_d.expect_notification().await.unwrap(); // Contact request notification
        sdk_d.expect_synced().await.unwrap();

        fn query_profile_acl(conn: &Connection, profile_id: &str) -> Result<AclDoc> {
            let acl_data: Vec<u8> = conn.query_row(
                "SELECT acl_data FROM documents WHERE id = ?",
                [&profile_id],
                |row| row.get(0),
            )?;
            let doc = build_yrs_doc(1, &acl_data)?;
            let acl_doc = AclDoc::from_doc(&doc);
            Ok(acl_doc)
        }

        let profile_id = format!("{}/profile", acc_1.id);

        let acl_a = query_profile_acl(&db_a, &profile_id).unwrap();
        let acl_b = query_profile_acl(&db_b, &profile_id).unwrap();
        let acl_c = query_profile_acl(&db_c, &profile_id).unwrap();
        let acl_d = query_profile_acl(&db_d, &profile_id).unwrap();

        // Verify that profile doc ACL is the same on all devices and includes only admin account.
        let expected_accounts = HashMap::from([(acc_1.id.clone(), AclRights::Admin)]);
        assert_eq!(acl_a.accounts, expected_accounts);
        assert_eq!(acl_b.accounts, expected_accounts);
        assert_eq!(acl_c.accounts, expected_accounts);
        assert_eq!(acl_d.accounts, expected_accounts);
    }

    // TODO: test self-update in case the same key package was reused
}
