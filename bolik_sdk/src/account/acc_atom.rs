use std::collections::HashSet;

use anyhow::{anyhow, bail, Context, Result};
use bolik_migrations::rusqlite::{params, OptionalExtension};
use bolik_proto::sync::{doc_payload::DocSchema, request, response, DeviceShareMessage};
use chrono::Utc;
use openmls::prelude::{Credential, KeyPackage, Signature, TlsDeserializeTrait, TlsSerializeTrait};
use openmls_traits::OpenMlsCryptoProvider;
use prost::Message;

use crate::{
    client::Client,
    device::{get_device_id, query_device_settings, DeviceShare},
    documents::{DbDocRow, DbDocRowMeta},
    mailbox,
    output::OutputEvent,
    registry::{
        WithBackend, WithBroadcast, WithDeviceAtom, WithDocsAtom, WithInTxn, WithSecretGroupAtom,
        WithTimelineAtom, WithTxn,
    },
    secrets,
    timeline::{
        self,
        acl_doc::{AclDoc, AclOperationMode},
    },
};

use super::{
    notifications::{AccNotifications, NotificationStatus},
    AccContact, AccDevice, AccView, ProfileView,
};

#[derive(Clone)]
pub struct AccountAtom {}

impl AccountAtom {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_account_id<'a>(&self, ctx: &impl WithTxn<'a>) -> Option<String> {
        let settings = query_device_settings(ctx.txn()).ok()?;
        settings.account_id
    }

    pub fn require_account_id<'a>(&self, ctx: &impl WithTxn<'a>) -> Result<String> {
        self.get_account_id(ctx)
            .ok_or(anyhow!("Device is not connected to account"))
    }

    pub fn get_account<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
    ) -> Result<Option<AccView>> {
        let Some(acc_id) = self.get_account_id(ctx) else {
            return Ok(None);
        };
        let doc_row = ctx.docs().find(ctx, &acc_id)?;
        let profile_row = ctx.docs().find(ctx, &format!("{}/profile", acc_id))?;
        match doc_row {
            Some(row) => Ok(Some(AccView::from_db(row, profile_row).0)),
            None => Ok(Some(AccView::new(acc_id))),
        }
    }

    pub fn require_account<'a>(&self, ctx: &(impl WithTxn<'a> + WithDocsAtom)) -> Result<AccView> {
        self.get_account(ctx)?
            .ok_or(anyhow!("Device is not connected to account"))
    }

    pub fn create_account<'a, C: Client>(
        &self,
        ctx: &(impl WithTxn<'a> + WithSecretGroupAtom<C> + WithDeviceAtom + WithDocsAtom + WithBackend),
        name: Option<String>,
    ) -> Result<AccView> {
        if self.get_account_id(ctx).is_some() {
            bail!("Account already created");
        }

        // Generate a list of KeyPackage
        ctx.device().generate_key_packages(ctx, 5)?;

        // Create MlsGroup for the account
        let group = ctx.secret_group().create(ctx)?;
        let account_id = group.chain.root();

        // Inform remote that new account was created
        mailbox::queue_mailbox(
            ctx,
            request::push_mailbox::Value::Account(request::NewAccount {
                chain: Some(group.chain.encode()?),
            }),
        )?;

        // Create an Account document
        let account_doc = AccView::init(ctx.device().yrs_client_id);
        AccView::add_device(
            &account_doc,
            AccDevice {
                id: ctx.device().id.clone(),
                name: ctx.device().name.clone(),
                added_at: Utc::now(),
            },
        );

        let acc_row = DbDocRow {
            meta: DbDocRowMeta {
                id: account_id.to_string(),
                author_device_id: ctx.device().id.clone(),
                counter: ctx.device().increment_clock(ctx)?,
                schema: DocSchema::AccountV1 as i32,
                created_at: Utc::now(),
                edited_at: Utc::now(),
            },
            yrs: account_doc,
            acl: AclDoc::init(ctx.device().yrs_client_id, account_id),
        };
        ctx.docs().save(ctx, &acc_row)?;

        ctx.txn().execute(
            "UPDATE device_settings SET account_id = ?",
            params![account_id],
        )?;

        // Init profile
        self.edit_profile(ctx, |doc| {
            ProfileView::set_name(
                doc,
                name.unwrap_or_else(|| ProfileView::default_name(account_id)),
            );
            Ok(())
        })?;

        tracing::debug!(?account_id, "Account created");
        self.require_account(ctx)
    }

    pub fn edit_account<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom + WithDeviceAtom),
        apply: impl FnOnce(&yrs::Doc) -> Result<()>,
    ) -> Result<AccView> {
        let acc = self.require_account(ctx)?;

        // Read from the database
        let mut doc_row = ctx
            .docs()
            .find(ctx, &acc.id)?
            .ok_or(anyhow!("Account doc is missing"))?;

        // Apply changes to it
        apply(&doc_row.yrs)?;

        doc_row.meta.author_device_id = ctx.device().id.clone();
        doc_row.meta.counter = ctx.device().increment_clock(ctx)?;
        ctx.docs().save(ctx, &doc_row)?;

        // Manage Profile doc
        let mut profile_row = ctx.docs().find(ctx, &format!("{}/profile", acc.id))?;
        if let Some(row) = &mut profile_row {
            let contacts = AccView::read_contacts(&doc_row.yrs).unwrap_or_default();
            let contact_ids: HashSet<_> = contacts.iter().map(|c| &c.account_id).collect();
            let old_contact_ids: HashSet<_> = acc.contacts.iter().map(|c| &c.account_id).collect();

            // Schedule Profile doc to be sent if list of contacts changed
            if old_contact_ids != contact_ids {
                row.meta.author_device_id = ctx.device().id.clone();
                row.meta.counter = ctx.device().increment_clock(ctx)?;
                ctx.docs().save(ctx, row)?;
            }
        }

        let (view, _) = AccView::from_db(doc_row, profile_row);
        Ok(view)
    }

    pub fn edit_profile<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom + WithDeviceAtom),
        apply: impl FnOnce(&yrs::Doc) -> Result<()>,
    ) -> Result<AccView> {
        let acc_id = self
            .get_account_id(ctx)
            .ok_or(anyhow!("Device is not connected to account"))?;

        // Read profile doc or create if missing
        let profile_id = format!("{}/profile", acc_id);
        let mut row = match ctx.docs().find(ctx, &profile_id)? {
            Some(row) => row,
            None => self.new_profile_row(ctx, &acc_id),
        };

        // Apply changes to it
        apply(&row.yrs)?;

        row.meta.author_device_id = ctx.device().id.clone();
        row.meta.counter = ctx.device().increment_clock(ctx)?;
        ctx.docs().save(ctx, &row)?;

        // Read updated acc
        let acc = self.require_account(ctx)?;
        Ok(acc)
    }

    pub fn get_share<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDeviceAtom + WithBackend),
    ) -> Result<String> {
        // Generate Key packages
        let mut key_packages = ctx.device().generate_key_packages(ctx, 6)?;
        let key_package = key_packages.pop().unwrap().1;

        // Include serialized key package in QR code
        let package_bytes = key_package.tls_serialize_detached()?;
        let message = DeviceShareMessage {
            key_package: package_bytes,
            device_name: ctx.device().name.clone(),
        }
        .encode_to_vec();
        let share = secrets::id_from_key(&message);
        Ok(share)
    }

    pub fn link_device<'a, C: Client>(
        &self,
        ctx: &(impl WithTxn<'a> + WithSecretGroupAtom<C> + WithDocsAtom + WithBackend + WithDeviceAtom),
        share: DeviceShare,
        other_device: response::DevicePackages,
    ) -> Result<String> {
        let acc = self.require_account(ctx)?;

        // Add device to account group
        tracing::debug!(?acc.id, "Loading group");
        let other_device_id = get_device_id(share.key_package.credential())?;
        let other_device_name = share.device_name;
        tracing::debug!(?other_device_id, "Adding new account member");

        let mut group = ctx.secret_group().load_latest(ctx, &acc.id)?;
        let commit = ctx
            .secret_group()
            .add(ctx, &mut group, vec![share.key_package])?
            .ok_or(anyhow!("Device is already in the group"))?;
        mailbox::queue_mls_commit(ctx, commit)?;

        // Send all doc secrets to new device
        let secrets = ctx.docs().list_secrets(ctx)?;
        if !secrets.is_empty() {
            ctx.secret_group()
                .queue_secrets(ctx, &acc, secrets, &mut [acc.id.clone()])?;
        }

        // Add device to each contact group
        let mut other_key_packages = Vec::new();
        for message in other_device.key_packages {
            let package = KeyPackage::tls_deserialize(&mut message.data.as_slice())?;
            let device_id = get_device_id(package.credential())?;
            if other_device_id == device_id {
                other_key_packages.push(package);
            }
        }

        // Add to all remaining groups
        ctx.secret_group()
            .add_to_all_groups(ctx, other_key_packages)?;

        // Add device to account document
        self.edit_account(ctx, |yrs_doc| {
            AccView::add_device(
                yrs_doc,
                AccDevice {
                    id: other_device_id,
                    name: other_device_name.clone(),
                    added_at: Utc::now(),
                },
            );
            Ok(())
        })?;

        Ok(other_device_name)
    }

    pub fn remove_device<'a, C: Client>(
        &self,
        ctx: &(impl WithTxn<'a> + WithSecretGroupAtom<C> + WithDeviceAtom + WithBackend + WithDocsAtom),
        remove_id: &str,
    ) -> Result<AccView> {
        let acc = self.require_account(ctx)?;
        ctx.secret_group().remove_from_all_groups(ctx, remove_id)?;
        ctx.docs().mark_secrets_obsolete(ctx, &acc.id)?;

        // Remove device from account document
        let acc = self.edit_account(ctx, |yrs_doc| {
            AccView::remove_device(yrs_doc, remove_id);
            Ok(())
        })?;

        Ok(acc)
    }

    pub async fn add_contact<C: Client>(
        &self,
        ctx: &(impl WithInTxn<C> + WithSecretGroupAtom<C> + WithDeviceAtom),
        mut contact: AccContact,
    ) -> Result<AccView> {
        let acc = ctx.in_txn(|tx_ctx| self.require_account(tx_ctx))?;

        // Create secret group
        ctx.secret_group()
            .create_for_accounts(ctx, &acc.id, &contact.account_id)
            .await?;

        // Add contact
        ctx.in_txn(|tx_ctx| {
            if contact.name.trim().is_empty() {
                // Try to find the name from Profile doc
                let profile_id = format!("{}/profile", contact.account_id);
                if let Some(profile_row) = tx_ctx.docs().find(tx_ctx, &profile_id)? {
                    contact.name = ProfileView::get_name(&profile_row);
                }
            }

            let updated_acc = self.edit_account(tx_ctx, |yrs_doc| {
                AccView::add_contact(
                    yrs_doc,
                    AccContact {
                        account_id: contact.account_id,
                        name: contact.name,
                    },
                );
                Ok(())
            })?;
            Ok(updated_acc)
        })
    }

    /// Find account id using device id. Note that this could return a device that was already removed from account.
    pub fn find_account_device<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        device_id: &str,
    ) -> Result<AccountDevice> {
        let acc_id = self.require_account_id(ctx)?;

        // First check if device is from this account
        let own_row: Option<(Vec<u8>, Option<u64>)> = ctx
            .txn()
            .query_row(
                r#"
SELECT credential, last_counter
  FROM signature_chain_devices
 WHERE chain_id = ?1 AND device_id = ?2"#,
                params![acc_id, device_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .with_context(|| format!("Find this account by device id={}", device_id))?;

        if let Some((cred_bytes, last_counter)) = own_row {
            let credential = Credential::tls_deserialize(&mut cred_bytes.as_slice())
                .context("Read credential")?;
            return Ok(AccountDevice {
                device_id: device_id.to_string(),
                account_id: acc_id,
                credential,
                last_counter,
            });
        }

        // Otherwise try to find device from other accounts
        let (cred_bytes, last_counter, found_id): (Vec<u8>, Option<u64>, String) = ctx
            .txn()
            .query_row(
                r#"
SELECT credential, last_counter, json_each.value
  FROM signature_chain_devices devs, signature_chains, json_each(signature_chains.account_ids)
 WHERE chain_id != ?1
       AND json_each.value != ?1
       AND device_id = ?2
       AND devs.chain_id = signature_chains.id"#,
                params![acc_id, device_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .with_context(|| format!("Find other account by device id={}", device_id))?;
        let credential =
            Credential::tls_deserialize(&mut cred_bytes.as_slice()).context("Read credential")?;
        Ok(AccountDevice {
            device_id: device_id.to_string(),
            account_id: found_id,
            credential,
            last_counter,
        })
    }

    fn new_profile_row<'a>(&self, ctx: &impl WithDeviceAtom, acc_id: &str) -> DbDocRow {
        let created_at = Utc::now();
        DbDocRow {
            meta: DbDocRowMeta {
                id: format!("{}/profile", acc_id),
                created_at: created_at.clone(),
                edited_at: created_at,
                schema: DocSchema::ProfileV1 as i32,
                author_device_id: "".into(),
                counter: 0,
            },
            yrs: ProfileView::init(ctx.device().yrs_client_id),
            acl: AclDoc::init_with_mode(
                ctx.device().yrs_client_id,
                acc_id,
                AclOperationMode::Custom,
            ),
        }
    }

    fn new_notifications_row<'a>(&self, ctx: &impl WithDeviceAtom, acc_id: &str) -> DbDocRow {
        let created_at = Utc::now();
        DbDocRow {
            meta: DbDocRowMeta {
                id: format!("{}/notifications", acc_id),
                created_at: created_at.clone(),
                edited_at: created_at,
                schema: DocSchema::AccountNotificationsV1 as i32,
                author_device_id: "".into(),
                counter: 0,
            },
            yrs: AccNotifications::init(ctx.device().yrs_client_id),
            acl: AclDoc::init(ctx.device().yrs_client_id, acc_id),
        }
    }

    fn create_notification<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        notification: &AccNotification,
    ) -> Result<()> {
        let id = notification.id();
        let now = Utc::now();
        ctx.txn().execute(
            r#"
INSERT INTO local_notifications (id, created_at) VALUES (?1, ?2)
    ON CONFLICT (id) DO NOTHING"#,
            params![id, now],
        )?;
        Ok(())
    }

    /// Create new local notification. Return true if new row was inserted.
    pub fn create_notification_if_new<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
        notification: &AccNotification,
    ) -> Result<bool> {
        let id = notification.id();
        // Check if this notification has already been handled by other device
        if self.notification_status(ctx, &id)? == NotificationStatus::Missing {
            let local = ctx
                .txn()
                .query_row(
                    "SELECT 1 FROM local_notifications WHERE id = ?",
                    [&id],
                    |_row| Ok(()),
                )
                .optional()?;
            if local.is_none() {
                self.create_notification(ctx, notification)?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    pub fn list_notifications<'a>(&self, ctx: &impl WithTxn<'a>) -> Result<Vec<String>> {
        let mut stmt = ctx
            .txn()
            .prepare("SELECT id FROM local_notifications ORDER BY created_at")?;
        let mut rows = stmt.query([])?;
        let mut ids = vec![];
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            ids.push(id);
        }

        Ok(ids)
    }

    pub fn delete_local_notification<'a>(&self, ctx: &impl WithTxn<'a>, id: &str) -> Result<()> {
        ctx.txn()
            .execute("DELETE FROM local_notifications WHERE id = ?", [id])?;
        Ok(())
    }

    fn ack_notification<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom + WithDeviceAtom),
        id: &str,
        accepted: bool,
    ) -> Result<()> {
        self.delete_local_notification(ctx, id)?;

        // Mark notification as acked in shared doc
        let acc_id = self.require_account_id(ctx)?;
        let row_id = format!("{}/notifications", acc_id);
        let mut row = match ctx.docs().find(ctx, &row_id)? {
            Some(row) => row,
            None => self.new_notifications_row(ctx, &acc_id),
        };

        // Apply changes to it
        if accepted {
            AccNotifications::accept(&row.yrs, id.into());
        } else {
            AccNotifications::ignore(&row.yrs, id.into());
        }

        row.meta.author_device_id = ctx.device().id.clone();
        row.meta.counter = ctx.device().increment_clock(ctx)?;
        ctx.docs().save(ctx, &row)?;

        Ok(())
    }

    pub fn notification_status<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
        id: &str,
    ) -> Result<NotificationStatus> {
        let acc_id = self.require_account_id(ctx)?;
        let row_id = format!("{}/notifications", acc_id);
        let status = if let Some(row) = ctx.docs().find(ctx, &row_id)? {
            AccNotifications::status(&row.yrs, id)
        } else {
            NotificationStatus::Missing
        };
        Ok(status)
    }

    pub async fn accept_notification<C: Client>(
        &self,
        ctx: &(impl WithInTxn<C> + WithSecretGroupAtom<C> + WithDeviceAtom + WithBroadcast),
        id: &str,
    ) -> Result<()> {
        if let Some(contact_id) = id.strip_prefix(AccNotification::CONTACT_REQUEST) {
            // Add new contact
            let view = self
                .add_contact(
                    ctx,
                    AccContact {
                        account_id: contact_id.into(),
                        name: "".into(),
                    },
                )
                .await?;
            ctx.broadcast(OutputEvent::AccUpdated { view });
        } else if let Some(card_id) = id.strip_prefix(AccNotification::CARD_SHARE) {
            // Index card
            ctx.in_txn(|tx_ctx| {
                let card = tx_ctx.timeline().get_card(tx_ctx, card_id)?;
                timeline::index_card(tx_ctx.txn(), &card)?;
                Ok(())
            })?;
            ctx.broadcast(OutputEvent::DocUpdated {
                doc_id: card_id.into(),
            });
        }

        ctx.in_txn(|tx_ctx| self.ack_notification(tx_ctx, id, true))?;
        Ok(())
    }

    pub fn ignore_notification<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom + WithDeviceAtom),
        id: &str,
    ) -> Result<()> {
        if let Some(card_id) = id.strip_prefix(AccNotification::CARD_SHARE) {
            ctx.docs().remove_external(ctx, card_id)?;
            // TODO: ideally, we would schedule an upload to remove ourselves from collaborators
        }
        self.ack_notification(ctx, id, false)
    }
}

pub struct AccountDevice {
    pub device_id: String,
    pub account_id: String,
    pub credential: Credential,
    /// Last counter is present for removed devices
    pub last_counter: Option<u64>,
}

impl AccountDevice {
    /// Verify signature
    pub fn verify(
        &self,
        backend: &impl OpenMlsCryptoProvider,
        payload: &[u8],
        signature_str: &str,
    ) -> Result<()> {
        let signature_bytes = secrets::key_from_id(signature_str)?;
        let signature = Signature::tls_deserialize(&mut signature_bytes.as_slice())
            .context("Read signature")?;
        self.credential
            .verify(backend, payload, &signature)
            .context("Verify signature")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccNotification {
    ContactRequest { account_id: String },
    CardShare { doc_id: String },
}

impl AccNotification {
    const CONTACT_REQUEST: &'static str = "contact-request/";
    pub const CARD_SHARE: &'static str = "card-share/";

    pub fn id(&self) -> String {
        match self {
            Self::ContactRequest { account_id } => {
                format!("{}{}", Self::CONTACT_REQUEST, account_id)
            }
            Self::CardShare { doc_id, .. } => format!("{}{}", Self::CARD_SHARE, doc_id),
        }
    }
}
