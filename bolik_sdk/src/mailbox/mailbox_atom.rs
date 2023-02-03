use crate::{
    account::AccView,
    client::Client,
    documents::DocSecretRow,
    output::OutputEvent,
    registry::{
        WithAccountAtom, WithBackend, WithBackendConn, WithBroadcast, WithDb, WithDeviceAtom,
        WithDocsAtom, WithEvents, WithInTxn, WithSecretGroupAtom, WithTxn,
    },
    secret_group::{GroupApplyResult, SecretGroup},
};
use anyhow::{bail, Context, Result};
use bolik_chain::DeviceRemovedOp;
use bolik_migrations::rusqlite::{params, OptionalExtension};
use bolik_proto::sync::{app_message, request, response, AppMessage, KeyPackageMessage};
use chrono::{Days, TimeZone, Utc};
use openmls::prelude::{KeyPackageRef, Sender};
use openmls_traits::OpenMlsCryptoProvider;
use prost::Message;

pub trait MailboxCtx<'a, C: Clone>:
    WithDb
    + WithBackendConn<'a>
    + WithDeviceAtom
    + WithSecretGroupAtom<C>
    + WithInTxn<C>
    + WithBroadcast
{
}
impl<'a, T, C: Clone> MailboxCtx<'a, C> for T where
    T: WithDb
        + WithBackendConn<'a>
        + WithDeviceAtom
        + WithSecretGroupAtom<C>
        + WithInTxn<C>
        + WithBroadcast
{
}

#[derive(Clone)]
pub struct MailboxAtom<C: Clone> {
    client: C,
}

impl<C> MailboxAtom<C>
where
    C: Client,
{
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Sync mailbox
    pub async fn sync(&self, ctx: &impl MailboxCtx<'_, C>) -> Result<()> {
        let mut events = SyncEvents::default();

        self.push_key_packages(ctx)
            .await
            .context("Push KeyPackages")?;
        self.ack_mailbox(ctx).await.context("Ack mailbox")?;
        self.fetch_mailbox(ctx, &mut events)
            .await
            .context("Fetch mailbox")?;

        if events.logged_out {
            tracing::info!("Logging out (removed by another device)!");
            let conn = ctx.db().conn.lock().unwrap();
            if let Err(err) = ctx.device().logout(&conn) {
                tracing::warn!("Failed to clear db: {}", err);
            }
            ctx.broadcast(OutputEvent::LogOut);
            return Ok(());
        }

        if !events.remove_members.is_empty() {
            if let Err(err) = ctx.in_txn(|tx_ctx| self.remove_members(tx_ctx, &mut events)) {
                tracing::warn!("Failed to remove members by suggestion: {}", err);
            }
        }

        if let Some(view) = events.updated_acc {
            ctx.broadcast(OutputEvent::AccUpdated { view });
        }

        self.push_mailbox(ctx).await.context("Push mailbox")?;

        Ok(())
    }

    /// Go through key packages
    async fn push_key_packages(&self, ctx: &impl MailboxCtx<'_, C>) -> Result<()> {
        loop {
            let row: Option<(u64, Vec<u8>)> = {
                let conn = ctx.db().conn.lock().unwrap();
                conn.query_row(
                    "SELECT rowid, message FROM key_packages_queue LIMIT 1",
                    params![],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
            };

            match row {
                Some((row_id, bytes)) => {
                    tracing::debug!("Uploading key package");
                    let package = KeyPackageMessage::decode(bytes.as_slice())?;
                    self.client.upload_key_package(package).await?;

                    let conn = ctx.db().conn.lock().unwrap();
                    conn.execute(
                        "DELETE FROM key_packages_queue WHERE rowid = ?",
                        params![row_id],
                    )?;
                }
                None => {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Ack all received mailbox messages
    async fn ack_mailbox(&self, ctx: &impl MailboxCtx<'_, C>) -> Result<()> {
        loop {
            // Find unacknowledged message
            let row: Option<(String, Option<String>)> = {
                let conn = ctx.db().conn.lock().unwrap();
                conn.query_row(
                    "SELECT message_id, error FROM ack_mailbox_queue LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
            };

            match row {
                Some((id, error)) => {
                    tracing::debug!(id, "Acking mailbox message");
                    self.client
                        .ack_mailbox_message(&id, request::AckMailboxInfo { error })
                        .await?;

                    let conn = ctx.db().conn.lock().unwrap();
                    conn.execute(
                        "DELETE FROM ack_mailbox_queue WHERE message_id = ?",
                        params![id],
                    )?;
                }
                None => {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Fetch and apply remote mailbox
    async fn fetch_mailbox(
        &self,
        ctx: &impl MailboxCtx<'_, C>,
        events: &mut SyncEvents,
    ) -> Result<()> {
        tracing::debug!("Fetching mailbox");
        let mailbox = self.client.fetch_mailbox().await?;
        for entry in mailbox.entries {
            let was_processed = {
                let conn = ctx.db().conn.lock().unwrap();
                let row = conn
                    .query_row(
                        "SELECT 1 FROM ack_mailbox_queue WHERE message_id = ?",
                        [&entry.id],
                        |_row| Ok(()),
                    )
                    .optional()?;
                row.is_some()
            };

            // Skip if message was already processed but not yet acked
            let mut error: Option<String> = None;
            if !was_processed {
                match entry.value {
                    Some(response::mailbox::entry::Value::Message(message)) => {
                        tracing::debug!(id = entry.id, "Received MlsMessage");
                        if let Err(err) = self.process_mls_message(ctx, events, message) {
                            tracing::error!("Failed to process MlsMessage: {:?}", err);
                            error = Some("MlsMessage".into());
                        };
                    }
                    Some(response::mailbox::entry::Value::Welcome(message)) => {
                        tracing::debug!(id = entry.id, "Received Welcome");
                        if let Err(err) = self.process_welcome(ctx, message) {
                            tracing::error!("Failed to process Welcome: {:?}", err);
                            error = Some("Welcome".into());
                        };
                    }
                    _ => {}
                }
            }

            // Mark this message as processed
            {
                let conn = ctx.db().conn.lock().unwrap();
                conn.execute(
                    r#"
INSERT INTO ack_mailbox_queue (message_id, error) VALUES (?, ?)
    ON CONFLICT (message_id) DO NOTHING"#,
                    params![entry.id, error],
                )?;
            }

            self.ack_mailbox(ctx).await?;
        }

        Ok(())
    }

    /// Go through queued push mailbox messages
    pub async fn push_mailbox(&self, ctx: &impl MailboxCtx<'_, C>) -> Result<()> {
        loop {
            let row: Option<(String, Vec<u8>)> = {
                let conn = ctx.db().conn.lock().unwrap();
                conn.query_row(
                    "SELECT id, message FROM push_mailbox_queue LIMIT 1",
                    params![],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
            };

            match row {
                Some((id, bytes)) => {
                    let message = request::PushMailbox::decode(bytes.as_slice())?;
                    {
                        let d = DisplayPushMailbox(&message.value);
                        tracing::debug!("Uploading PushMailbox id={}: {}", message.id, d);
                    }
                    self.client.push_mailbox(message).await?;

                    let conn = ctx.db().conn.lock().unwrap();
                    conn.execute("DELETE FROM push_mailbox_queue WHERE id = ?", [&id])?;
                }
                None => {
                    break;
                }
            }
        }

        Ok(())
    }

    fn process_mls_message<'a>(
        &self,
        ctx: &impl MailboxCtx<'a, C>,
        events: &mut SyncEvents,
        message: response::SecretGroupMessage,
    ) -> Result<()> {
        ctx.in_txn(|tx_ctx| {
            let group = ctx.secret_group().apply(tx_ctx, message)?;

            match group {
                GroupApplyResult::AppMessage {
                    message: m,
                    sender,
                    group_id,
                } => {
                    let content = AppMessage::decode(m.into_bytes().as_slice())?;
                    {
                        let d = DisplayAppMsg(&content.value);
                        tracing::debug!("Process MLS message {}", d);
                    }
                    match content.value {
                        Some(app_message::Value::Secrets(secrets)) => {
                            for secret in secrets.values {
                                tracing::debug!(?secret.id, doc_id = secret.doc_id, accounts = secret.account_ids.len(), "Persisting new secret");
                                let created_at = Utc
                                    .timestamp_opt(secret.created_at_sec, 0)
                                    .earliest()
                                    .unwrap_or_else(|| Utc::now());
                                let obsolete_at = created_at
                                    .checked_add_days(Days::new(30))
                                    .unwrap_or(created_at);

                                tx_ctx.docs().save_secret(
                                    tx_ctx,
                                    DocSecretRow {
                                        id: secret.id,
                                        key: secret.secret,
                                        account_ids: secret.account_ids,
                                        doc_id: secret.doc_id,
                                        algorithm: secret.algorithm,
                                        created_at,
                                        obsolete_at,
                                    },
                                )?;
                            }
                        }
                        Some(app_message::Value::RemoveMe(_)) => {
                            if let Sender::Member(key_ref) = sender {
                                events.remove_members.push(MemberRef { group_id, key_ref });
                            }
                        }
                        _ => {}
                    }
                }
                GroupApplyResult::Commit {
                    messages_out,
                    group,
                    stats,
                } => {
                    // Check if this device was removed from the group
                    if !group.mls.is_active() {
                        if group.chain.account_ids().is_empty() {
                            match tx_ctx.account().get_account_id(tx_ctx) {
                                Some(account_id) if account_id == group.id() => {
                                    events.logged_out = true;
                                }
                                _ => {}
                            }
                        } else {
                            // No op for contact groups.
                            // NOTE: What should happen here?
                        }
                    } else {
                        if stats.removed > 0 {
                            tracing::info!(
                                group_id = group.id(),
                                "Remote removed devices={}",
                                stats.removed
                            );
                            self.rotate_doc_secrets(tx_ctx, &group)?;
                        }

                        for message in messages_out {
                            super::queue_mls_commit(tx_ctx, message)?;
                        }
                    }
                }
                GroupApplyResult::Nothing => {}
                GroupApplyResult::UnknownGroup => {}
            }
            Ok(())
        })?;

        Ok(())
    }

    fn process_welcome<'a>(
        &self,
        ctx: &impl MailboxCtx<'a, C>,
        message: response::SecretGroupWelcome,
    ) -> Result<()> {
        ctx.in_txn(|tx_ctx| {
            if let Some(group) = ctx.secret_group().join(tx_ctx, message)? {
                let account_id = tx_ctx.account().get_account_id(tx_ctx);

                if group.chain.account_ids().is_empty() {
                    // Group is for a single account
                    match account_id {
                        Some(id) if id == group.id() => {
                            // This could happen if there was a conflict and group continued from a different chain.
                            // In this case we don't need to do anything
                        }
                        Some(_) => {
                            // We already connected to account --> remove this group
                            // TODO: self-remove
                            bail!("Ignoring the group: Already connected to account");
                        }
                        None => {
                            // Connected to account
                            let account_id = group.id();
                            tracing::info!("Device connected to account {}", account_id);
                            tx_ctx
                                .txn()
                                .execute(
                                    "UPDATE device_settings SET account_id = ?",
                                    params![account_id],
                                )
                                .context("Set account id")?;
                            tx_ctx.queue_event(OutputEvent::ConnectedToAccount {
                                view: AccView::new(account_id),
                            });
                        }
                    }
                } else {
                    // This is contact group.
                }
            }
            Ok(())
        })
    }

    fn rotate_doc_secrets<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
        group: &SecretGroup,
    ) -> Result<()> {
        let account_ids = group.chain.account_ids();
        if account_ids.is_empty() {
            let account_id = group.chain.root();
            tracing::info!("Rotating secret for account={}", account_id);
            ctx.docs().mark_secrets_obsolete(ctx, &account_id)?;
        } else {
            for account_id in account_ids {
                tracing::info!("Rotating secret for account={}", account_id);
                ctx.docs().mark_secrets_obsolete(ctx, &account_id)?;
            }
        }
        Ok(())
    }

    fn remove_members<'a>(
        &self,
        ctx: &(impl WithTxn<'a>
              + WithSecretGroupAtom<C>
              + WithBackend
              + WithDeviceAtom
              + WithAccountAtom
              + WithDocsAtom),
        events: &mut SyncEvents,
    ) -> Result<()> {
        for MemberRef { group_id, key_ref } in events.remove_members.drain(..) {
            // Remove device from the group
            let mut group = ctx.secret_group().load_latest(ctx, &group_id)?;
            let Some(device_id) = group
                .find_id_by_ref(key_ref, ctx.backend().crypto()) else {
                    continue;
                };
            let commit = ctx.secret_group().remove(
                ctx,
                &mut group,
                vec![DeviceRemovedOp {
                    key_ref,
                    last_counter: ctx.device().get_clock(ctx, &device_id)?,
                }],
            )?;

            if let Some(commit) = commit {
                tracing::info!(
                    group_id = group.id(),
                    device_id,
                    "Removing device by suggestion"
                );
                super::queue_mls_commit(ctx, commit)?;
                self.rotate_doc_secrets(ctx, &group)?;
            }

            // If group is from this account then remove device from account doc
            if group.chain.account_ids().is_empty() {
                match ctx.account().get_account_id(ctx) {
                    Some(account_id) if account_id == group.id() => {
                        let acc = ctx.account().edit_account(ctx, |doc| {
                            AccView::remove_device(doc, &device_id);
                            Ok(())
                        })?;
                        events.updated_acc = Some(acc);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

struct DisplayAppMsg<'a>(&'a Option<app_message::Value>);

impl<'a> std::fmt::Display for DisplayAppMsg<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(app_message::Value::Secrets(m)) => {
                f.write_fmt(format_args!("Secrets(count={})", m.values.len()))
            }
            Some(app_message::Value::RemoveMe(_)) => f.write_str("RemoveMe"),
            _ => f.write_str("Unknown"),
        }
    }
}

struct DisplayPushMailbox<'a>(&'a Option<request::push_mailbox::Value>);

impl<'a> std::fmt::Display for DisplayPushMailbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(v) => match v {
                request::push_mailbox::Value::Account(_) => f.write_str("Account"),
                request::push_mailbox::Value::Message(c) => {
                    f.write_fmt(format_args!("Message(chain_hash={})", c.chain_hash))
                }
                request::push_mailbox::Value::Commit(c) => {
                    let chain_hash = c
                        .chain
                        .as_ref()
                        .and_then(|chain| chain.blocks.last())
                        .map(|b| &b.hash);
                    f.write_fmt(format_args!(
                        "Commit(chain_hash={:?} welcome={})",
                        chain_hash,
                        c.welcome.is_some()
                    ))
                }
            },
            None => f.write_str("Empty"),
        }
    }
}

#[derive(Default)]
struct SyncEvents {
    updated_acc: Option<AccView>,
    logged_out: bool,
    remove_members: Vec<MemberRef>,
}

struct MemberRef {
    group_id: String,
    key_ref: KeyPackageRef,
}
