use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use bolik_migrations::rusqlite::Connection;
use bolik_proto::sync::doc_payload::DocSchema;
use chrono::Utc;
use tokio_stream::Stream;
use tracing::instrument;

use crate::{
    account::{AccContact, AccLabel, AccView, ProfileView},
    background::{BackgroundInput, BackgroundTask},
    blobs::{self, SaveFileParams},
    client::{Client, ClientConfig},
    db::{migrations, Db},
    device::{get_device_id, DeviceAtom, DeviceShare},
    export::ExportedCard,
    output::OutputEvent,
    registry::{Registry, SetupTxnCtx, WithTxn},
    secrets::DbCipher,
    timeline::{
        self,
        acl_doc::{AclChange, AclRights},
        card::{CardChange, CardFile, CardLabelsChange, CardView, CleanupResult},
        TimelineDay,
    },
    SecretGroupStatus, BIN_LABEL_ID, import::ImportResult,
};

pub struct Sdk<C: Clone> {
    #[allow(unused)]
    pub(crate) db_path: String,
    debug_name: String,
    pub(crate) registry: Registry<C>,
    background_tx: tokio::sync::mpsc::Sender<BackgroundInput>,
    pub client: C,
}

impl<C> Sdk<C>
where
    C: Client,
{
    pub(crate) fn new(
        db_path: &str,
        blobs_dir: PathBuf,
        device_name: impl Into<String>,
        db_encryption_key: chacha20poly1305::Key,
        background_tx: tokio::sync::mpsc::Sender<BackgroundInput>,
        client_conf: ClientConfig,
    ) -> Result<Self> {
        std::fs::create_dir_all(&blobs_dir).context("Ensure blob directory")?;

        tracing::debug!(?db_path, "Connecting to database");
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations::apply(&conn)?;

        let db_cipher = DbCipher::new(&db_encryption_key);
        let conn = Arc::new(Mutex::new(conn));
        let db = Db {
            conn: conn.clone(),
            db_cipher,
        };
        let (device, cred_bundle) = {
            let mut conn = db.conn.lock().unwrap();
            let ctx = SetupTxnCtx::new(&mut conn, &db)?;
            let device = DeviceAtom::new(&ctx, device_name.into(), blobs_dir)?;
            let bundle = device.get_credential_bundle(&ctx)?;
            ctx.commit()?;
            (device, bundle)
        };
        let debug_name = device.name.chars().take(3).collect();
        let client = C::new(client_conf, device.id.clone(), cred_bundle.into_parts().1)?;
        let registry = Registry::new(db, device, client.clone());

        Ok(Self {
            db_path: db_path.to_string(),
            debug_name,
            registry,
            background_tx,
            client,
        })
    }

    pub(crate) fn bg_task(&self) -> BackgroundTask<C> {
        BackgroundTask::new(self.registry.clone(), self.debug_name.clone())
    }

    pub fn broadcast_subscribe(&self) -> tokio::sync::broadcast::Receiver<OutputEvent> {
        self.registry.broadcast.subscribe()
    }

    pub fn get_account(&self) -> Option<AccView> {
        self.registry
            .in_txn(|ctx, r| r.account.get_account(ctx))
            .ok()?
    }

    pub fn get_device_id(&self) -> &str {
        &self.registry.device.id
    }

    #[instrument(skip_all, fields(d = self.debug_name, name))]
    pub fn create_account(&self, name: Option<String>) -> Result<AccView> {
        let acc_view = self
            .registry
            .in_txn(|ctx, r| r.account.create_account(ctx, name))?;
        self.sync();
        Ok(acc_view)
    }

    pub fn timeline_days(&self, label_ids: Vec<String>) -> Result<Vec<String>> {
        self.registry
            .in_txn(|ctx, _| timeline::timeline_days(ctx.txn(), label_ids))
    }

    pub fn timeline_by_day(&self, day: &str, label_ids: Vec<String>) -> Result<TimelineDay> {
        self.registry
            .in_txn(|ctx, _| timeline::timeline_by_day(ctx.txn(), day, label_ids))
    }

    pub fn sync(&self) {
        let tx = self.background_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(BackgroundInput::Sync).await;
        });
    }

    pub(crate) fn initial_sync(&self, delay: Duration) {
        let tx = self.background_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            let _ = tx.send(BackgroundInput::Sync).await;
            let _ = tx.send(BackgroundInput::EmptyBin).await;
        });
    }

    pub fn get_device_share(&self) -> Result<String> {
        let share = self.registry.in_txn(|ctx, r| r.account.get_share(ctx))?;
        self.sync();
        Ok(share)
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub async fn link_device(&self, share_str: &str) -> Result<String> {
        let share = DeviceShare::parse(&share_str)?;
        let other_device_id = get_device_id(share.key_package.credential())?;
        let other_device = self.client.get_device_packages(&other_device_id).await?;

        let other_device_name = self
            .registry
            .in_txn(|ctx, r| r.account.link_device(ctx, share, other_device))?;
        self.sync();

        Ok(other_device_name)
    }

    #[instrument(skip_all, fields(d = self.debug_name, remove_id))]
    pub fn remove_device(&self, remove_id: &str) -> Result<AccView> {
        let acc = self
            .registry
            .in_txn(|ctx, r| r.account.remove_device(ctx, remove_id))?;
        self.sync();
        Ok(acc)
    }

    pub fn create_card(&self) -> Result<CardView> {
        self.registry.in_txn(|ctx, r| {
            let acc_id = r.account.require_account_id(ctx)?;
            Ok(CardView::empty(acc_id))
        })
    }

    #[instrument(skip_all, fields(d = self.debug_name, card_id = id))]
    pub fn edit_card(&self, id: &str, changes: Vec<CardChange>) -> Result<CardView> {
        self.registry
            .in_txn(|ctx, r| r.timeline.edit_card(ctx, id, changes))
    }

    #[instrument(skip_all, fields(d = self.debug_name, card_id))]
    pub fn edit_card_labels(
        &self,
        card_id: &str,
        changes: Vec<CardLabelsChange>,
    ) -> Result<CardView> {
        self.registry
            .in_txn(|ctx, r| r.timeline.edit_card_labels(ctx, card_id, changes))
    }

    pub fn get_card(&self, id: &str) -> Result<CardView> {
        self.registry.in_txn(|ctx, r| r.timeline.get_card(ctx, id))
    }

    pub fn save_file(&self, card_id: &str, path: impl AsRef<Path>) -> Result<CardFile> {
        self.registry.in_txn(|ctx, r| {
            blobs::save_file(
                ctx.txn(),
                SaveFileParams {
                    blob_dir: &r.device.blobs_dir,
                    card_id,
                    path: &path.as_ref(),
                    original_file_name: None,
                    device_id: r.device.id.clone(),
                },
            )
        })
    }

    pub fn get_file_path(&self, blob_id: &str) -> Result<Option<String>> {
        self.registry.in_txn(|ctx, _r| {
            let path = blobs::get_file_path(ctx.txn(), blob_id)?;
            Ok(path.filter(|path| Path::new(&path).exists()))
        })
    }

    pub fn download_blob(
        &self,
        card_id: impl Into<String>,
        blob_id: &str,
        device_id: &str,
    ) -> Result<DownloadResult> {
        let card_id = card_id.into();

        let res = self.registry.in_txn(|ctx, r| {
            // First, check locally
            let blob = blobs::find_by_id(ctx.txn(), blob_id, device_id)?;
            if let Some(b) = blob {
                let path = Path::new(&b.path);
                if path.exists() {
                    return Ok(Some(DownloadResult {
                        path: Some(b.path),
                        download_started: false,
                    }));
                }
            }

            let card = r.timeline.get_card(ctx, &card_id)?;
            let card_file = card
                .get_file(blob_id)
                .ok_or(anyhow!("CardFile not found"))?;

            // If not found locally then try to download
            tracing::debug!(?card_file.blob_id, "Schedule file download");
            let tx = self.background_tx.clone();
            tokio::spawn(async move {
                let _ = tx
                    .send(BackgroundInput::DownloadFile { card_id, card_file })
                    .await;
            });
            Ok(None)
        })?;

        if let Some(res) = res {
            Ok(res)
        } else {
            Ok(DownloadResult {
                path: None,
                download_started: true,
            })
        }
    }

    pub fn close_card(&self, card_id: &str) -> Result<()> {
        let mut card = self.get_card(card_id)?;

        let CleanupResult { changes, .. } = card.cleanup();

        if !changes.is_empty() {
            card = self.edit_card(card_id, changes)?;
        }

        // Generate thumbnails asynchronously
        let tx = self.background_tx.clone();
        tokio::spawn(async move {
            let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
            let _ = tx
                .send(BackgroundInput::ProcessFiles(card, oneshot_tx))
                .await;
            let _ = oneshot_rx.await;
            let _ = tx.send(BackgroundInput::Sync).await;
        });

        Ok(())
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub fn account_group(&self) -> Result<SecretGroupStatus> {
        self.registry.in_txn(|ctx, r| {
            let acc_id = r.account.require_account_id(ctx)?;
            let group = r.secret_group.load_latest(ctx, &acc_id)?;
            group.status()
        })
    }

    #[instrument(skip_all, fields(d = self.debug_name, contact_id))]
    pub fn contact_group(&self, contact_id: &str) -> Result<SecretGroupStatus> {
        self.registry.in_txn(|ctx, r| {
            let acc = r.account.require_account(ctx)?;
            for contact in acc.contacts {
                if contact.account_id == contact_id {
                    let mut account_ids = vec![acc.id, contact.account_id];
                    let group = r
                        .secret_group
                        .load_latest_for_accounts(ctx, &mut account_ids)?;
                    return group.status();
                }
            }
            bail!("Contact not found id={}", contact_id)
        })
    }

    pub fn move_card_to_bin(&self, card_id: &str, scope: MoveToBinScope) -> Result<()> {
        if let Err(_) = self.get_card(card_id) {
            // Card is not found --> no op
            return Ok(());
        };

        match scope {
            MoveToBinScope::ThisAccount => {
                self.registry.in_txn(|ctx, r| {
                    r.timeline.edit_card_labels(
                        ctx,
                        card_id,
                        vec![CardLabelsChange::AddLabel {
                            label_id: BIN_LABEL_ID.to_string(),
                        }],
                    )
                })?;
            }
            MoveToBinScope::All => {
                self.registry.in_txn(|ctx, r| {
                    r.timeline
                        .edit_card_acl(ctx, card_id, vec![AclChange::MoveToBin])
                })?;
            }
        }

        self.sync();
        Ok(())
    }

    pub fn restore_from_bin(&self, card_id: &str) -> Result<CardView> {
        let card = self
            .registry
            .in_txn(|ctx, r| r.timeline.restore_from_bin(ctx, card_id))?;
        self.sync();
        Ok(card)
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub fn empty_bin(&self) -> Result<()> {
        tracing::debug!("User emptying bin");
        self.registry
            .in_txn(|ctx, r| r.timeline.empty_bin(ctx, Some(Utc::now())))?;
        self.sync();
        Ok(())
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub fn edit_name(&self, name: String) -> Result<AccView> {
        let view = self.registry.in_txn(|ctx, r| {
            r.account.edit_profile(ctx, |doc| {
                ProfileView::set_name(doc, name);
                Ok(())
            })
        })?;
        self.sync();
        Ok(view)
    }

    pub fn create_acc_label(&self, name: String) -> Result<CreateAccLabelResult> {
        let label = AccLabel::new(name);
        let updated_acc = self.edit_account(|yrs_doc| {
            AccView::create_label(yrs_doc, label.clone());
            Ok(())
        })?;

        Ok(CreateAccLabelResult {
            view: updated_acc,
            label,
        })
    }

    pub fn delete_acc_label(&self, label_id: &str) -> Result<AccView> {
        let updated_acc = self.edit_account(|yrs_doc| {
            AccView::delete_label(yrs_doc, label_id);
            Ok(())
        })?;
        Ok(updated_acc)
    }

    #[instrument(skip_all, fields(d = self.debug_name, contact_id = contact.account_id))]
    pub async fn add_contact(&self, contact: AccContact) -> Result<AccView> {
        let ctx = self.registry.db_ctx();
        let view = self.registry.account.add_contact(&ctx, contact).await?;
        self.sync();
        Ok(view)
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub fn edit_contact_name(&self, account_id: &str, name: &str) -> Result<AccView> {
        let updated_acc = self.edit_account(|yrs_doc| {
            AccView::edit_contact_name(yrs_doc, account_id, name)?;
            Ok(())
        })?;
        Ok(updated_acc)
    }

    #[instrument(skip_all, fields(d = self.debug_name, card_id, account_id))]
    pub fn edit_collaborators(
        &self,
        card_id: &str,
        changed: HashMap<String, Option<AclRights>>,
    ) -> Result<CardView> {
        let card = self.registry.in_txn(|ctx, r| {
            let acc = r.account.require_account(ctx)?;
            let mut changes = vec![];

            for (account_id, rights) in changed.into_iter() {
                // When adding a new collaborator we need to have it as a contact, so that we can deliver a new doc secret.
                if let Some(rights) = rights {
                    let contact = acc.contacts.iter().find(|c| c.account_id == account_id);
                    if contact.is_none() {
                        bail!("Account is missing the contact");
                    }

                    changes.push(AclChange::Add { account_id, rights });
                } else {
                    changes.push(AclChange::Remove { account_id });
                }
            }

            r.timeline.edit_card_acl(ctx, card_id, changes)
        })?;
        self.sync();
        Ok(card)
    }

    fn edit_account(&self, apply: impl FnOnce(&yrs::Doc) -> Result<()>) -> Result<AccView> {
        let view = self
            .registry
            .in_txn(|ctx, r| r.account.edit_account(ctx, apply))?;
        self.sync();
        Ok(view)
    }

    pub async fn export_cards_to_dir(&self, out_dir: impl Into<PathBuf>) -> Result<()> {
        self.registry
            .export
            .cards_to_dir(self.registry.clone(), out_dir.into())
            .await?;
        Ok(())
    }

    pub fn export_cards(&self) -> Result<impl Stream<Item = Result<ExportedCard>> + '_> {
        self.registry.export.cards(self.registry.clone())
    }

    pub async fn export_card(&self, card_id: &str) -> Result<ExportedCard> {
        let ctx = self.registry.db_ctx();
        self.registry.export.export_card(&ctx, card_id).await
    }

    pub fn import_data(&self, in_dir: impl Into<PathBuf>) -> Result<ImportResult> {
        let res = self
            .registry
            .in_txn(|ctx, r| r.import.run(ctx, in_dir.into()))?;
        self.sync();
        Ok(res)
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileView>> {
        let docs = self
            .registry
            .in_txn(|ctx, r| r.docs.list_by_schema(ctx, DocSchema::ProfileV1))?;
        let profiles = docs
            .into_iter()
            .map(|row| ProfileView::from_db(row).0)
            .collect();
        Ok(profiles)
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub async fn logout(self) {
        // Leave all groups
        if let Err(err) = self
            .registry
            .in_txn(|ctx, r| r.secret_group.leave_all_groups(ctx))
        {
            tracing::warn!("Failed to leave groups: {}", err);
        }

        // Push mailbox
        let ctx = self.registry.db_ctx();
        if let Err(err) = self.registry.mailbox.push_mailbox(&ctx).await {
            tracing::warn!("Failed to push leave group messages: {}", err);
        }

        // Logout
        tracing::info!("Logging out (manual)!");
        let conn = self.registry.db.conn.lock().unwrap();
        if let Err(err) = self.registry.device.logout(&conn) {
            tracing::warn!("Failed to clear db: {}", err);
        }
    }

    pub fn list_notification_ids(&self) -> Result<Vec<String>> {
        let n = self
            .registry
            .in_txn(|ctx, r| r.account.list_notifications(ctx))?;
        Ok(n)
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub async fn accept_notification(&self, id: &str) -> Result<()> {
        let ctx = self.registry.db_ctx();
        self.registry.account.accept_notification(&ctx, id).await?;
        self.sync();
        Ok(())
    }

    #[instrument(skip_all, fields(d = self.debug_name))]
    pub fn ignore_notification(&self, id: &str) -> Result<()> {
        self.registry
            .in_txn(|ctx, r| r.account.ignore_notification(ctx, id))?;
        self.sync();
        Ok(())
    }
}

#[derive(Clone)]
pub struct DownloadResult {
    // Will be present if file is already downloaded
    pub path: Option<String>,
    // Will be true when client started downloading the file
    pub download_started: bool,
}

#[derive(Clone)]
pub struct CreateAccLabelResult {
    pub view: AccView,
    pub label: AccLabel,
}

pub enum MoveToBinScope {
    /// Move this card to bin only for this account.
    ThisAccount,
    /// Move this card to bin for all accounts this card is shared with.
    All,
}
