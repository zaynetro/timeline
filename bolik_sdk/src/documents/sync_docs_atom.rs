use crate::{
    account::{
        AccNotification, AccNotifications, AccView, AccountDevice, NotificationStatus, ProfileView,
    },
    blobs::{self, BlobRef},
    client::Client,
    documents::{self, DbDocRow, DbDocRowMeta},
    output::OutputEvent,
    registry::{
        WithAccountAtom, WithBackend, WithBackendConn, WithBlobsAtom, WithBroadcast, WithDb,
        WithDeviceAtom, WithDocsAtom, WithEvents, WithInTxn, WithMailboxAtom, WithSecretGroupAtom,
        WithTimelineAtom, WithTxn,
    },
    secrets,
    timeline::{
        self,
        acl_doc::{AclDoc, AclOperationMode},
        card::{CardChange, CardView, ContentView},
        EditCardOpts, PermanentDeleteOpts,
    },
};
use anyhow::{anyhow, bail, Result};
use bolik_migrations::rusqlite::{params, OptionalExtension};
use bolik_proto::{
    prost::Message,
    sync::{
        acl_payload::AclSchema,
        doc_payload::DocSchema,
        request::{self, BlobRefMessage},
        response, AclPayload, DeviceVectorClock, DocPayload,
    },
};
use chacha20poly1305::aead::Aead;
use chrono::{DateTime, Duration, TimeZone, Utc};
use tracing::instrument;

use super::DocSecret;

pub trait SyncDocsCtx<'a, C: Clone>:
    WithDb
    + WithBackendConn<'a>
    + WithSecretGroupAtom<C>
    + WithInTxn<C>
    + WithBroadcast
    + WithMailboxAtom<C>
    + WithDeviceAtom
    + WithBlobsAtom<C>
{
}
impl<'a, T, C: Clone> SyncDocsCtx<'a, C> for T where
    T: WithDb
        + WithBackendConn<'a>
        + WithSecretGroupAtom<C>
        + WithInTxn<C>
        + WithBroadcast
        + WithMailboxAtom<C>
        + WithDeviceAtom
        + WithBlobsAtom<C>
{
}

/// Atom responsible for syncing documents. It fetches remote docs merges them and
/// pushes locally modified ones.
#[derive(Clone)]
pub struct SyncDocsAtom<C: Clone> {
    client: C,
}

impl<C: Client> SyncDocsAtom<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    pub async fn sync(&self, ctx: &impl SyncDocsCtx<'_, C>) -> Result<()> {
        if let None = ctx.in_txn(|tx_ctx| Ok(tx_ctx.account().get_account_id(tx_ctx)))? {
            tracing::info!("Device not connected to account, skipping doc sync...");
            return Ok(());
        }

        // Continuously sync until server responds with less docs than the limit.
        // Also limit the amount of rounds to prevent infinite loops in case of an error.
        let mut sync_err = None;
        for _ in 0..20 {
            match self.sync_roundtrip(ctx).await {
                Ok(res) if res.docs < res.limit => {
                    break;
                }
                Ok(_) => {}
                Err(err) => {
                    sync_err = Some(err);
                    break;
                }
            }
        }

        self.process_failed_docs(ctx).await?;

        ctx.in_txn(|tx_ctx| self.process_fetched_docs(tx_ctx))?;

        if let Some(err) = sync_err {
            Err(err)
        } else {
            Ok(())
        }
    }

    async fn sync_roundtrip(&self, ctx: &impl SyncDocsCtx<'_, C>) -> Result<FetchResult> {
        let device_id = &ctx.device().id;
        let (local_clock, acc_id) = ctx.in_txn(|tx_ctx| {
            let clock = tx_ctx.device().get_vector_clock(tx_ctx)?;
            let acc_id = tx_ctx.account().require_account_id(tx_ctx)?;
            Ok((clock, acc_id))
        })?;

        // Fetch remote changes
        tracing::debug!(?local_clock, "Fetching remote docs");
        let res = self.client.fetch_docs(&local_clock).await?;
        let fetch_res = FetchResult {
            docs: res.docs.len() as u32,
            limit: res.limit,
        };
        tracing::debug!("Remote has {} new docs", res.docs.len());
        for remote_doc in res.docs {
            if &remote_doc.author_device_id == device_id {
                continue;
            }

            ctx.in_txn(|tx_ctx| {
                tx_ctx.device().set_max_clock(
                    tx_ctx,
                    &remote_doc.author_device_id,
                    remote_doc.counter,
                )?;
                self.process_remote_doc(tx_ctx, &acc_id, remote_doc)?;
                Ok(())
            })?;
        }

        let (acc, mut local_clock) = ctx.in_txn(|ctx_tx| {
            let acc = ctx_tx.account().require_account(ctx_tx)?;
            let clock = ctx.device().get_vector_clock(ctx_tx)?;
            Ok((acc, clock))
        })?;

        // Find and upload locally modified docs
        let mut last_seen_counter = res.last_seen_counter;
        while let Some(modified_doc) =
            ctx.in_txn(|ctx_tx| ctx_tx.docs().find_local_after(ctx_tx, last_seen_counter))?
        {
            last_seen_counter = modified_doc.meta.counter;
            self.upload_encrypted_doc(ctx, modified_doc, local_clock.clone(), &acc)
                .await?;
        }

        // Upload other queued docs
        while let Some((rowid, mut message)) =
            ctx.in_txn(|tx_ctx| tx_ctx.docs().find_queued_doc(tx_ctx))?
        {
            tracing::debug!(doc_id = message.id, "Uploading queued doc");
            // Update local clock
            if let Some(c) = local_clock.vector.get_mut(device_id) {
                *c = message.counter;
            }
            message.current_clock = Some(local_clock.clone());
            self.client.push_doc(message).await?;
            ctx.in_txn(|tx_ctx| tx_ctx.docs().remove_queued_doc(tx_ctx, rowid))?;
        }

        Ok(fetch_res)
    }

    fn process_remote_doc<'a>(
        &self,
        ctx: &(impl WithTxn<'a>
              + WithDocsAtom
              + WithAccountAtom
              + WithBackend
              + WithTimelineAtom
              + WithDeviceAtom),
        acc_id: &str,
        remote_doc: response::DocVersion,
    ) -> Result<()> {
        match self.merge_remote_doc(ctx, acc_id, remote_doc)? {
            MergeResult::Merged(merged) => {
                self.complete_doc_fetching(ctx, &acc_id, merged)?;
            }
            MergeResult::Removed => {}
            MergeResult::Retry {
                doc_id,
                author_device_id,
            } => {
                self.mark_for_retry(ctx, &doc_id, &author_device_id)?;
            }
            MergeResult::Skip {
                doc_id,
                author_device_id,
            } => {
                self.mark_doc_skipped(ctx, &doc_id, &author_device_id)?;
            }
        }
        Ok(())
    }

    #[instrument(skip_all, fields(doc_id = remote_doc.doc_id, author = remote_doc.author_device_id, counter = remote_doc.counter))]
    fn merge_remote_doc<'a>(
        &self,
        ctx: &(impl WithTxn<'a>
              + WithDocsAtom
              + WithAccountAtom
              + WithBackend
              + WithTimelineAtom
              + WithDeviceAtom),
        acc_id: &str,
        remote_doc: response::DocVersion,
    ) -> Result<MergeResult> {
        let doc_id = &remote_doc.doc_id;
        let existing = ctx.docs().find(ctx, doc_id)?;

        // Verify doc payload
        let from_account_device = match Self::verify_doc_payload(ctx, &remote_doc) {
            Ok(d) => d,
            Err(err) => {
                // This error might be transient, so retry later.
                // There could have been a confict in Signature Chain and we couldn't find an account for author device.
                if self.can_retry_doc(ctx, doc_id, &remote_doc.author_device_id)? {
                    tracing::warn!("Doc payload is invalid (will retry): {:?}", err);
                    return Ok(MergeResult::retry(remote_doc));
                }

                tracing::error!("Doc payload is invalid: {:?}", err);
                return Ok(MergeResult::skip(remote_doc));
            }
        };
        let from_account_id = &from_account_device.account_id;
        let remote_doc_version = format!("{}@{}", remote_doc.author_device_id, remote_doc.counter);

        // Merge remote version with local doc
        let remote_body = match Self::read_remote_version(ctx, &remote_doc) {
            Ok(b) => b,
            Err(err) => {
                // This error might be transient, so retry later.
                if self.can_retry_doc(ctx, doc_id, &remote_doc.author_device_id)? {
                    tracing::warn!("Failed to read remote doc (will retry): {:?}", err);
                    return Ok(MergeResult::retry(remote_doc));
                }

                tracing::error!("Failed to read remote doc: {:?}", err);
                return Ok(MergeResult::skip(remote_doc));
            }
        };
        match remote_body {
            DocBody::Deleted(_deleted_at) => {
                // Allow permanent deletions only from own account
                if from_account_id != &acc_id {
                    return Ok(MergeResult::skip(remote_doc));
                }

                tracing::info!(doc_id, "Permanently deleting");
                ctx.timeline()
                    .permanently_delete(ctx, doc_id, PermanentDeleteOpts::default())?;
                Ok(MergeResult::Removed)
            }
            DocBody::Payload(payload) => {
                let author_device_id = remote_doc.author_device_id.clone();
                let (local_row, is_new) = match existing {
                    Some(mut local_row) => {
                        // Apply remote version
                        if payload.schema != local_row.meta.schema {
                            tracing::warn!(
                                "Remote schema={} doesn't match local={}",
                                payload.schema,
                                local_row.meta.schema
                            );
                            return Ok(MergeResult::skip(remote_doc));
                        }

                        // Filter schemas from other accounts
                        if from_account_id != &acc_id {
                            match DocSchema::from_i32(payload.schema) {
                                Some(DocSchema::CardV1 | DocSchema::ProfileV1) => {
                                    // Allowed
                                }
                                _ => {
                                    tracing::info!(
                                        doc_id,
                                        from_account_id,
                                        "Skipping doc for disallowed schema from another account"
                                    );
                                    return Ok(MergeResult::skip(remote_doc));
                                }
                            }
                        }

                        // ACL
                        match Self::merge_acls(from_account_id, &local_row.acl, &payload.acl) {
                            MergeAclResult::Applied => {}
                            MergeAclResult::Unauthorized => {
                                // This error might be transient, so retry later.
                                // This could happen when doc ACL changed, a new member edited the doc and new member's change
                                // came first. In this case we just skip this doc and refetch during next sync.
                                if self.can_retry_doc(ctx, doc_id, &remote_doc.author_device_id)? {
                                    tracing::warn!(
                                        from_account_id,
                                        "not allowed to edit this doc (will retry)",
                                    );
                                    return Ok(MergeResult::retry(remote_doc));
                                }

                                tracing::warn!(from_account_id, "not allowed to edit this doc",);
                                return Ok(MergeResult::skip(remote_doc));
                            }
                        }

                        // Merge docs
                        if let Err(err) = documents::merge_yrs_docs(&local_row.yrs, &payload.data) {
                            tracing::warn!("Failed to merge remote doc: {}", err);
                            return Ok(MergeResult::skip(remote_doc));
                        }

                        let remote_edited_at = Utc.timestamp(payload.edited_at_sec, 0);
                        if remote_edited_at > local_row.meta.edited_at {
                            local_row.meta.edited_at = remote_edited_at;
                        }

                        if local_row.meta.author_device_id == ctx.device().id {
                            // Doc was modified locally. Do not override the version.
                        } else {
                            local_row.meta.author_device_id = remote_doc.author_device_id;
                            local_row.meta.counter = remote_doc.counter;
                        }

                        (local_row, false)
                    }
                    None => {
                        // Create new doc row
                        let yrs_client_id = ctx.device().yrs_client_id;
                        let meta = DbDocRowMeta {
                            id: remote_doc.doc_id,
                            created_at: Utc.timestamp(remote_doc.created_at_sec, 0),
                            edited_at: Utc.timestamp(payload.edited_at_sec, 0),
                            schema: payload.schema,
                            author_device_id: remote_doc.author_device_id,
                            counter: remote_doc.counter,
                        };

                        let yrs_doc = documents::build_yrs_doc(yrs_client_id, &payload.data)?;
                        let acl = if let Some(acl) = payload.acl {
                            let acl_doc = documents::build_yrs_doc(yrs_client_id, &acl.data)?;
                            acl_doc
                        } else {
                            AclDoc::init(yrs_client_id, &from_account_device.account_id)
                        };

                        (
                            DbDocRow {
                                meta,
                                yrs: yrs_doc,
                                acl,
                            },
                            true,
                        )
                    }
                };
                ctx.docs().save(ctx, &local_row)?;
                tracing::debug!(
                    "Merged remote doc. Local version={}. Remote version={} schema={}",
                    local_row.meta.counter,
                    remote_doc_version,
                    local_row.meta.schema,
                );

                Ok(MergeResult::Merged(MergedDoc {
                    doc_id: local_row.meta.id,
                    is_new,
                    from_account_id: from_account_device.account_id,
                    author_device_id,
                    priority: MergedDoc::build_priority(local_row.meta.schema),
                }))
            }
        }
    }

    fn verify_doc_payload<'a>(
        ctx: &(impl WithTxn<'a> + WithAccountAtom + WithBackend),
        remote_doc: &response::DocVersion,
    ) -> Result<AccountDevice> {
        let account_device = ctx
            .account()
            .find_account_device(ctx, &remote_doc.author_device_id)?;

        if let Some(last_counter) = account_device.last_counter {
            // Author device has been removed from the account
            if remote_doc.counter > last_counter {
                bail!(
                    "Device was removed and doc counter={} is higher than allowed={}",
                    remote_doc.counter,
                    last_counter
                );
            }
        }

        let backend = ctx.backend();
        match &remote_doc.body {
            Some(response::doc_version::Body::Encrypted(body)) => {
                let mut buf = Vec::with_capacity(remote_doc.doc_id.len() + body.payload.len());
                buf.extend(remote_doc.doc_id.bytes());
                buf.extend(&body.payload);
                account_device.verify(&backend, &buf, &remote_doc.payload_signature)?;
            }
            Some(response::doc_version::Body::Deleted(body)) => {
                let payload = format!("{},{}", remote_doc.doc_id, body.deleted_at_sec);
                account_device.verify(
                    &backend,
                    payload.as_bytes(),
                    &remote_doc.payload_signature,
                )?;
            }
            _ => {}
        }
        Ok(account_device)
    }

    fn read_remote_version<'a>(
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
        remote_doc: &response::DocVersion,
    ) -> Result<DocBody> {
        match &remote_doc.body {
            Some(response::doc_version::Body::Encrypted(body)) => {
                let secret_row = ctx.docs().find_secret_by_id(ctx, &body.secret_id)?;
                let secret: DocSecret = match secret_row {
                    Some(s) => s.into(),
                    None => {
                        bail!(
                            "Secret not found. Cannot decrypt remote doc (doc_id={})",
                            remote_doc.doc_id
                        );
                    }
                };

                let (nonce, ciphertext) = secrets::ciphertext_into_parts(&body.payload)?;
                let doc_payload_bytes = secret
                    .cipher
                    .decrypt(nonce, ciphertext)
                    .map_err(|err| anyhow!("{:?}", err))?;
                let doc_payload = DocPayload::decode(doc_payload_bytes.as_ref())?;
                Ok(DocBody::Payload(doc_payload))
            }
            Some(response::doc_version::Body::Deleted(body)) => {
                let time = Utc
                    .timestamp_opt(body.deleted_at_sec, 0)
                    .earliest()
                    .ok_or(anyhow!("Invalid deleted_at {}", body.deleted_at_sec))?;
                Ok(DocBody::Deleted(time.into()))
            }
            _ => Err(anyhow!("Empty doc body")),
        }
    }

    fn merge_acls(
        from_account_id: &str,
        acl_doc: &yrs::Doc,
        remote_acl: &Option<AclPayload>,
    ) -> MergeAclResult {
        let local_acl = AclDoc::from_doc(acl_doc);

        if !local_acl.allowed_to_edit(from_account_id) {
            return MergeAclResult::Unauthorized;
        }

        if local_acl.allowed_to_admin(from_account_id) {
            if let Some(remote_acl) = remote_acl {
                // Merge ACLs
                if let Err(err) = documents::merge_yrs_docs(acl_doc, &remote_acl.data) {
                    tracing::warn!("Failed to apply remote acl: {}", err);
                }
            }
        }
        MergeAclResult::Applied
    }

    async fn upload_encrypted_doc<'a>(
        &self,
        ctx: &impl SyncDocsCtx<'_, C>,
        local_doc: DbDocRow,
        local_clock: DeviceVectorClock,
        acc: &AccView,
    ) -> Result<DbDocRowMeta> {
        // Build a list of participants
        let mut participants: Vec<String> = {
            let acl = AclDoc::from_doc(&local_doc.acl);
            match acl.mode {
                AclOperationMode::Custom => {
                    if local_doc.meta.schema == (DocSchema::ProfileV1 as i32) {
                        // Send profile to all account contacts and to other account devices
                        let mut ids: Vec<_> =
                            acc.contacts.iter().map(|c| c.account_id.clone()).collect();
                        ids.push(acc.id.clone());
                        ids
                    } else {
                        tracing::warn!(
                            "AclOperationMode::Custom not supported for schema={}, falling back to Normal",
                            local_doc.meta.schema
                        );
                        acl.participants()
                    }
                }
                AclOperationMode::Normal => acl.participants(),
            }
        };

        tracing::debug!(
            doc_id = local_doc.meta.id,
            counter = local_doc.meta.counter,
            schema = local_doc.meta.schema,
            participants = participants.len(),
            "Uploading locally modified doc"
        );

        // Create missing secret groups
        let created_groups = ctx
            .secret_group()
            .create_missing_groups(ctx, &acc, &mut participants)
            .await;
        if created_groups > 0 {
            ctx.mailbox().push_mailbox(ctx).await?;
        }

        // Find doc secret
        let (secret, is_secret_new) = ctx.in_txn(|tx_txn| {
            Self::find_doc_secret(tx_txn, &acc, &local_doc.meta.id, &mut participants)
        })?;
        if is_secret_new {
            ctx.mailbox().push_mailbox(ctx).await?;
        }

        let meta = local_doc.meta.clone();
        let acl_data = AclPayload {
            data: documents::encode_yrs_doc(&local_doc.acl),
            schema: AclSchema::YrsV1.into(),
        };

        // Build a list of blob refences (upload new blobs)
        let UploadBlobsResult { blob_refs, doc } = self.upload_card_blobs(ctx, local_doc).await?;

        // Prepare doc payload
        let doc_payload = DocPayload {
            data: documents::encode_yrs_doc(&doc),
            schema: meta.schema,
            edited_at_sec: meta.edited_at.timestamp(),
            acl: Some(acl_data),
        };
        let doc_payload_bytes = doc_payload.encode_to_vec();

        // Encrypt document data
        let nonce = secrets::generate_nonce();
        let ciphertext = secret
            .cipher
            .encrypt(&nonce, doc_payload_bytes.as_ref())
            .map_err(|err| anyhow!("Encrypt doc {:?}", err))?;
        let encrypted_payload = secrets::merge_nonce_ciphertext(nonce, ciphertext);
        let payload_signature = {
            let mut buf = Vec::with_capacity(meta.id.len() + encrypted_payload.len());
            buf.extend(meta.id.bytes());
            buf.extend(&encrypted_payload);
            ctx.in_txn(|tx_ctx| ctx.device().sign(tx_ctx, &buf))?
        };

        self.client
            .push_doc(request::DocMessage {
                id: meta.id.clone(),
                to_account_ids: participants,
                current_clock: Some(local_clock),
                counter: meta.counter,
                created_at_sec: meta.created_at.timestamp(),
                payload_signature,
                body: Some(request::doc_message::Body::Encrypted(
                    request::doc_message::EncryptedBody {
                        secret_id: secret.id,
                        payload: encrypted_payload,
                        blob_refs,
                    },
                )),
            })
            .await?;
        Ok(meta)
    }

    fn find_doc_secret<'a>(
        ctx: &(impl WithTxn<'a> + WithSecretGroupAtom<C> + WithBackend + WithDeviceAtom + WithDocsAtom),
        acc: &AccView,
        _doc_id: &str,
        participants: &mut [String],
    ) -> Result<(DocSecret, bool)> {
        let (secret_row, is_secret_new) = {
            // First search for secret using doc_id
            // TODO: Implement. My initial solution had an issue with not sending a secret to other accounts..

            // Otherwise fallback to account ids
            ctx.docs().get_secret_row_for_accounts(ctx, participants)?
        };
        if is_secret_new {
            ctx.secret_group()
                .queue_secrets(ctx, acc, vec![secret_row.clone()], participants)?;
        }

        Ok((secret_row.into(), is_secret_new))
    }

    async fn upload_card_blobs(
        &self,
        ctx: &impl SyncDocsCtx<'_, C>,
        local_row: DbDocRow,
    ) -> Result<UploadBlobsResult> {
        if local_row.meta.schema != DocSchema::CardV1 as i32 {
            return Ok(UploadBlobsResult {
                doc: local_row.yrs,
                blob_refs: vec![],
            });
        }

        // Find all files this card references
        let (card, mut doc) = CardView::from_db(local_row, None);

        let mut blob_refs = vec![];
        let mut queue = vec![];
        let mut card_changes = vec![];

        for block in &card.blocks {
            if let ContentView::File(ref file) = block.view {
                let blob_ref = {
                    let conn = ctx.db().conn.lock().unwrap();
                    blobs::find_by_id(&conn, &file.blob_id, &file.device_id)?
                };
                match blob_ref {
                    Some(blob) if blob.synced => {
                        blob_refs.push(request::BlobRefMessage {
                            id: blob.id,
                            device_id: blob.device_id,
                        });
                    }
                    Some(blob) => {
                        let blob_secret = if let Some(secret) = card.secrets.get(&file.blob_id) {
                            DocSecret::new(&card.id, &secret.secret)
                        } else {
                            // Generate a new one
                            let value = secrets::generate_key().to_vec();
                            let secret = DocSecret::new(&card.id, &value);
                            card_changes.push(CardChange::AddFileSecret {
                                blob_id: blob.id.clone(),
                                value,
                            });
                            secret
                        };

                        blob_refs.push(request::BlobRefMessage {
                            id: blob.id.clone(),
                            device_id: blob.device_id.clone(),
                        });
                        queue.push((blob, blob_secret));
                    }
                    None => {
                        // This might happen if we haven't downloaded a remote blob
                        blob_refs.push(request::BlobRefMessage {
                            id: file.blob_id.clone(),
                            device_id: file.device_id.clone(),
                        });
                    }
                };
            }
        }

        // Update card secrets
        if !card_changes.is_empty() {
            doc = ctx
                .in_txn(|ctx_tx| {
                    ctx_tx.timeline().edit_card_opts(
                        ctx_tx,
                        EditCardOpts {
                            id: &card.id,
                            changes: card_changes,
                            acl_changes: vec![],
                            created_at: None,
                            skip_counter: true,
                        },
                    )
                })?
                .1;
        }

        // Upload blobs
        for (blob, secret) in queue {
            self.upload_blob(ctx, &blob, &secret).await?;
        }

        Ok(UploadBlobsResult { doc, blob_refs })
    }

    #[instrument(skip_all, fields(blob_id = blob.id))]
    async fn upload_blob(
        &self,
        ctx: &impl SyncDocsCtx<'_, C>,
        blob: &BlobRef,
        secret: &DocSecret,
    ) -> Result<()> {
        ctx.blobs().upload_blob(ctx, blob, secret).await?;
        let conn = ctx.db().conn.lock().unwrap();
        blobs::mark_synced(&conn, &blob.id)?;
        Ok(())
    }

    fn process_fetched_docs<'a>(
        &self,
        ctx: &(impl WithTxn<'a>
              + WithEvents
              + WithAccountAtom
              + WithDocsAtom
              + WithTimelineAtom
              + WithDeviceAtom),
    ) -> Result<()> {
        let acc = ctx.account().require_account(ctx)?;
        let mut stmt = ctx
            .txn()
            .prepare("SELECT doc_id, is_new, from_account_id FROM process_fetched_docs_queue ORDER BY priority")?;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let merged = MergedDoc {
                doc_id: row.get(0)?,
                is_new: row.get(1)?,
                from_account_id: row.get(2)?,
                author_device_id: "".into(),
                priority: 0,
            };

            let row = ctx.docs().find(ctx, &merged.doc_id)?;
            if let Some(row) = row {
                self.process_fetched_doc(ctx, &acc, row, &merged)?;
            }

            // Mark as processed
            ctx.txn().execute(
                "DELETE FROM process_fetched_docs_queue WHERE doc_id = ?",
                [&merged.doc_id],
            )?;
        }
        Ok(())
    }

    fn process_fetched_doc<'a>(
        &self,
        ctx: &(impl WithTxn<'a>
              + WithDocsAtom
              + WithEvents
              + WithAccountAtom
              + WithTimelineAtom
              + WithDeviceAtom),
        acc: &AccView,
        row: DbDocRow,
        merged: &MergedDoc,
    ) -> Result<()> {
        match DocSchema::from_i32(row.meta.schema) {
            Some(DocSchema::AccountV1) => {
                let profile_row = ctx.docs().find(ctx, &format!("{}/profile", row.meta.id))?;
                let view = AccView::from_db(row, profile_row).0;
                ctx.queue_event(OutputEvent::AccUpdated { view });
            }
            Some(DocSchema::AccountNotificationsV1) => {
                // Another device could have acknowledged a notification.
                AccNotifications::iter_ids(&row.yrs, |notification_id, status| {
                    // Remove local notification
                    ctx.account()
                        .delete_local_notification(ctx, &notification_id)?;

                    if status != NotificationStatus::Accepted {
                        return Ok(());
                    }

                    // Add missing card to timeline
                    if let Some(card_id) = notification_id.strip_prefix(AccNotification::CARD_SHARE)
                    {
                        if ctx.timeline().index_missing(ctx, card_id)? {
                            ctx.queue_event(OutputEvent::TimelineUpdated);
                        }
                    }

                    Ok(())
                })?;

                ctx.queue_event(OutputEvent::NotificationsUpdated);
            }
            Some(DocSchema::CardV1) if merged.from_account_id != acc.id => {
                if ctx.timeline().is_indexed(ctx, &row.meta.id)? {
                    // We have accepted this card to timeline already --> Reindex
                    let labels_row = ctx.docs().find(ctx, &format!("{}/labels", row.meta.id))?;
                    let view = CardView::from_db(row, labels_row).0;
                    timeline::index_card(ctx.txn(), &view)?;
                    ctx.queue_event(OutputEvent::TimelineUpdated);
                    return Ok(());
                }

                // External card rules:
                // If not in notifications --> add notification, save to docs
                // If was accepted --> save to docs, index
                // If was ignored --> ignore

                let notification = AccNotification::CardShare {
                    doc_id: row.meta.id.clone(),
                };
                let notification_id = notification.id();

                let status = ctx.account().notification_status(ctx, &notification_id)?;
                match status {
                    NotificationStatus::Missing => {
                        let is_new = ctx
                            .account()
                            .create_notification_if_new(ctx, &notification)?;
                        if is_new {
                            ctx.queue_event(OutputEvent::Notification(notification));
                        }
                    }
                    NotificationStatus::Accepted => {
                        // Index the card
                        let labels_row =
                            ctx.docs().find(ctx, &format!("{}/labels", row.meta.id))?;
                        let view = CardView::from_db(row, labels_row).0;
                        timeline::index_card(ctx.txn(), &view)?;
                        ctx.queue_event(OutputEvent::TimelineUpdated);
                    }
                    NotificationStatus::Ignored => {
                        // TODO: send doc message to admin with ACL without us
                        ctx.docs().remove_external(ctx, &row.meta.id)?;
                    }
                }
            }
            Some(DocSchema::CardV1) => {
                // TODO: if this account is missing from ACL then create a notification about removed access from future updates

                // Index the card
                let labels_row = ctx.docs().find(ctx, &format!("{}/labels", row.meta.id))?;
                let view = CardView::from_db(row, labels_row).0;
                timeline::index_card(ctx.txn(), &view)?;
                ctx.queue_event(OutputEvent::TimelineUpdated);
            }
            Some(DocSchema::CardLabelsV1) => {
                // Index the card
                // First we need to find the parent id.
                if let Some(parent_id) = row.meta.id.split('/').next() {
                    let card_row = ctx.docs().find(ctx, parent_id)?;
                    if let Some(card_row) = card_row {
                        let view = CardView::from_db(card_row, Some(row)).0;
                        timeline::index_card(ctx.txn(), &view)?;
                    }
                }
                ctx.queue_event(OutputEvent::TimelineUpdated);
            }
            Some(DocSchema::ProfileV1) => {
                let profile = ProfileView::from_db(row).0;

                if acc.id == profile.account_id {
                    // Profile update for the same account
                    return Ok(());
                }

                // Check if contact exists
                let contact_exists = acc
                    .contacts
                    .iter()
                    .find(|c| c.account_id == profile.account_id)
                    .is_some();
                if contact_exists {
                    return Ok(());
                }

                // Create new notification
                let notification = AccNotification::ContactRequest {
                    account_id: profile.account_id,
                };
                let is_new = ctx
                    .account()
                    .create_notification_if_new(ctx, &notification)?;
                if is_new {
                    ctx.queue_event(OutputEvent::Notification(notification));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Return if we can retry this doc later.
    fn can_retry_doc<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        doc_id: &str,
        author_device_id: &str,
    ) -> Result<bool> {
        let tries: Option<u16> = ctx
            .txn()
            .query_row(
                "SELECT tries FROM failed_docs WHERE doc_id = ?1 AND author_device_id = ?2",
                [doc_id, author_device_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(tries.map(|n| n < 3).unwrap_or(true))
    }

    /// Mark this doc as needing a retry.
    fn mark_for_retry<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        doc_id: &str,
        author_device_id: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let future = now + Duration::seconds(60);
        ctx.txn().execute(
            r#"
INSERT INTO failed_docs (doc_id, author_device_id, retry_after) VALUES (?1, ?2, ?3)
       ON CONFLICT (doc_id, author_device_id)
          DO UPDATE SET tries = excluded.tries + 1, retry_after = ?4"#,
            params![doc_id, author_device_id, now, future],
        )?;
        Ok(())
    }

    fn mark_doc_skipped<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        doc_id: &str,
        author_device_id: &str,
    ) -> Result<()> {
        ctx.txn().execute(
            "DELETE FROM failed_docs WHERE doc_id = ?1 AND author_device_id = ?2",
            [doc_id, author_device_id],
        )?;
        Ok(())
    }

    /// Queue doc to be processed and remove from failed table.
    fn complete_doc_fetching<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        acc_id: &str,
        merged: MergedDoc,
    ) -> Result<()> {
        self.mark_doc_skipped(ctx, &merged.doc_id, &merged.author_device_id)?;
        ctx.txn().execute(
                r#"
INSERT INTO process_fetched_docs_queue (doc_id, is_new, from_account_id, priority) VALUES (?1, ?2, ?3, ?4)
       ON CONFLICT (doc_id) DO NOTHING"#,
                params![merged.doc_id, merged.is_new, merged.from_account_id, merged.priority],
            )?;

        if merged.from_account_id == acc_id {
            // It could happen that change from another account comes first.
            // Then if we receive a change from our own account we should override from_account_id.
            ctx.txn().execute(
                "UPDATE process_fetched_docs_queue SET from_account_id = ?1 WHERE doc_id = ?2",
                [acc_id, &merged.doc_id],
            )?;
        }
        Ok(())
    }

    async fn process_failed_docs(&self, ctx: &impl SyncDocsCtx<'_, C>) -> Result<()> {
        loop {
            let row: Option<(String, String)> = {
                let conn = ctx.db().conn.lock().unwrap();
                conn.query_row(
                    "SELECT doc_id, author_device_id FROM failed_docs WHERE retry_after < ? LIMIT 1",
                    [Utc::now()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
            };

            match row {
                Some((doc_id, author_device_id)) => {
                    tracing::info!(doc_id, author_device_id, "Retrying failed doc");
                    let remote_doc = match self
                        .client
                        .get_doc_version(&doc_id, &author_device_id)
                        .await
                    {
                        Ok(d) => d,
                        Err(err) => {
                            tracing::warn!("Failed to fetch doc version: {}", err);
                            ctx.in_txn(|tx_ctx| {
                                self.mark_for_retry(tx_ctx, &doc_id, &author_device_id)?;
                                Ok(())
                            })?;
                            continue;
                        }
                    };

                    ctx.in_txn(|tx_ctx| {
                        let acc_id = tx_ctx.account().require_account_id(tx_ctx)?;
                        self.process_remote_doc(tx_ctx, &acc_id, remote_doc)?;
                        Ok(())
                    })?;
                }
                None => {
                    break;
                }
            }
        }

        Ok(())
    }
}

enum MergeAclResult {
    Unauthorized,
    Applied,
}

enum DocBody {
    Payload(DocPayload),
    Deleted(DateTime<Utc>),
}

struct UploadBlobsResult {
    doc: yrs::Doc,
    blob_refs: Vec<BlobRefMessage>,
}

struct FetchResult {
    docs: u32,
    limit: u32,
}

struct MergedDoc {
    doc_id: String,
    /// True, when no doc was present locally
    is_new: bool,
    from_account_id: String,
    author_device_id: String,
    /// When processing the docs we want to process some docs first. Like account and account notifications.
    priority: u8,
}

impl MergedDoc {
    fn build_priority(schema: i32) -> u8 {
        match DocSchema::from_i32(schema) {
            Some(DocSchema::AccountV1) => 1,
            Some(DocSchema::AccountNotificationsV1) => 2,
            Some(DocSchema::ProfileV1) => 6,
            _ => 10,
        }
    }
}

enum MergeResult {
    /// Doc was successfully merged.
    Merged(MergedDoc),
    /// Permanently removed.
    Removed,
    /// Failed to merge the doc but should retry later.
    Retry {
        doc_id: String,
        author_device_id: String,
    },
    /// Doc is invalid.
    Skip {
        doc_id: String,
        author_device_id: String,
    },
}

impl MergeResult {
    fn retry(doc: response::DocVersion) -> Self {
        Self::Retry {
            doc_id: doc.doc_id,
            author_device_id: doc.author_device_id,
        }
    }

    fn skip(doc: response::DocVersion) -> Self {
        Self::Skip {
            doc_id: doc.doc_id,
            author_device_id: doc.author_device_id,
        }
    }
}
