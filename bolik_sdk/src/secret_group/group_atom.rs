use std::collections::HashMap;

use crate::{
    account::AccView,
    client::Client,
    device::{get_device_id, DeviceCtx},
    documents::DocSecretRow,
    mailbox,
    registry::{WithBackend, WithDeviceAtom, WithInTxn, WithTxn},
    secrets::{build_accounts_hash, CIPHERSUITES},
    signature_chain::SignatureChainStorage,
};
use anyhow::{anyhow, bail, Context, Result};
use bolik_chain::{ChainUsed, ChangeAuthor, DeviceRemovedOp, SignatureChain};
use bolik_migrations::rusqlite::{params, OptionalExtension, Params, Row};
use bolik_proto::sync::{
    app_message,
    app_message::{doc_secrets_message, DocSecretsMessage, RemoveMe},
    request, response, AppMessage,
};
use openmls::prelude::{
    ApplicationMessage, GroupId, InnerState, KeyPackage, KeyPackageBundle, MlsGroup,
    MlsGroupConfig, MlsGroupConfigBuilder, MlsMessageIn, MlsMessageOut, OpenMlsKeyStore,
    ProcessedMessage, Sender, SenderRatchetConfiguration, TlsDeserializeTrait, TlsSerializeTrait,
    Welcome,
};
use openmls_traits::OpenMlsCryptoProvider;
use prost::Message;

use super::SecretGroup;

pub trait SecretGroupCtx<'a>: WithDeviceAtom + DeviceCtx<'a> {}
impl<'a, T> SecretGroupCtx<'a> for T where T: WithDeviceAtom + DeviceCtx<'a> {}

pub trait SecretGroupAsyncCtx<C: Clone>: WithInTxn<C> + WithDeviceAtom {}
impl<'a, T, C: Clone> SecretGroupAsyncCtx<C> for T where T: WithInTxn<C> + WithDeviceAtom {}

#[derive(Clone)]
pub struct SecretGroupAtom<C: Clone> {
    client: C,
}

impl<C: Client> SecretGroupAtom<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Create new secret group for a single account.
    pub fn create<'a>(&self, ctx: &impl SecretGroupCtx<'a>) -> Result<SecretGroup> {
        self.do_create(ctx, None)
    }

    fn do_create<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        account_ids: Option<Vec<String>>,
    ) -> Result<SecretGroup> {
        let backend = &ctx.backend();
        let chain_storage = SignatureChainStorage::new(
            ChangeAuthor {
                bundle: ctx.device().get_credential_bundle(ctx)?,
            },
            backend,
        );
        let key_bundle = KeyPackageBundle::new(
            &CIPHERSUITES,
            &chain_storage.author.bundle,
            &ctx.backend(),
            vec![],
        )?;
        let key_ref = key_bundle.key_package().hash_ref(ctx.backend().crypto())?;
        ctx.backend()
            .key_store()
            .store(key_ref.value(), &key_bundle)
            .map_err(|err| anyhow!("{:?}", err))?;

        let chain = chain_storage.create_with_key(key_bundle, account_ids)?;
        let mut group = MlsGroup::new(
            &ctx.backend(),
            &Self::mls_group_config(),
            GroupId::from_slice(chain.root().as_bytes()),
            key_ref.as_slice(),
        )?;
        tracing::trace!(
            "Group created epoch={:?} state={:?} account_ids={:?}",
            group.epoch(),
            group_state(&group),
            chain.account_ids(),
        );

        self.save(ctx, &mut group, &chain)?;
        Ok(SecretGroup { mls: group, chain })
    }

    /// Join existing secret group
    pub fn join<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        message: response::SecretGroupWelcome,
    ) -> Result<Option<SecretGroup>> {
        let chain_message = match message.chain {
            Some(c) => c,
            None => {
                tracing::warn!("Received Welcome without chain. Skipping...");
                return Ok(None);
            }
        };

        let backend = &ctx.backend();
        let chain_storage = SignatureChainStorage::new(
            ChangeAuthor {
                bundle: ctx.device().get_credential_bundle(ctx)?,
            },
            backend,
        );

        let (chain, chain_used) = chain_storage.merge_remote(chain_message)?;
        if let ChainUsed::Local = chain_used {
            tracing::warn!(
                chain_hash = chain.head(),
                "Received Welcome with outdated remote chain. Skipping..."
            );
            return Ok(None);
        }

        let welcome = Welcome::tls_deserialize(&mut message.welcome.as_slice())?;
        let mut group =
            MlsGroup::new_from_welcome(&ctx.backend(), &Self::mls_group_config(), welcome, None)?;
        tracing::trace!(
            epoch = group.epoch().as_u64(),
            state = group_state(&group),
            chain_hash = chain.head(),
            "Group joined"
        );

        ctx.device().generate_key_packages(ctx, 1)?;
        self.save(ctx, &mut group, &chain)?;
        Ok(Some(SecretGroup { mls: group, chain }))
    }

    pub fn apply<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        message: response::SecretGroupMessage,
    ) -> Result<GroupApplyResult> {
        let mls_message = MlsMessageIn::tls_deserialize(&mut message.mls.as_slice())?;
        let group_id = Self::group_id_str(mls_message.group_id());
        let msg_epoch = mls_message.epoch().as_u64();
        let Some(mut local_group) = self.load_mls(ctx, &group_id, Some(&message.chain_hash))? else {
                tracing::warn!(
                    id = group_id,
                    chain_hash = message.chain_hash,
                    is_commit = mls_message.is_handshake_message(),
                    "Received secret message from unknown group"
                );
                return Ok(GroupApplyResult::UnknownGroup);
        };

        tracing::trace!(
            "Loaded local group epoch={} state={:?}",
            msg_epoch,
            group_state(&local_group),
        );
        let backend = &ctx.backend();
        let unverified_msg = local_group
            .parse_message(mls_message, backend)
            .context("Parse Mls message")?;
        let sender = unverified_msg.sender().clone();
        let processed = local_group
            .process_unverified_message(unverified_msg, None, backend)
            .context("Verify Mls message")?;

        match processed {
            ProcessedMessage::ApplicationMessage(message) => Ok(GroupApplyResult::AppMessage {
                group_id: Self::group_id_str(local_group.group_id()),
                sender,
                message,
            }),
            ProcessedMessage::ProposalMessage(_proposal) => {
                // How to use proposals: https://github.com/openmls/openmls/blob/main/openmls/tests/test_mls_group.rs
                Ok(GroupApplyResult::Nothing)
            }
            ProcessedMessage::StagedCommitMessage(commit) => {
                // Merge the chains
                let remote_chain_msg = message
                    .chain
                    .ok_or(anyhow!("SignatureChain is missing for commit"))?;
                let backend = &ctx.backend();
                let chain_storage = SignatureChainStorage::new(
                    ChangeAuthor {
                        bundle: ctx.device().get_credential_bundle(ctx)?,
                    },
                    backend,
                );
                let (chain, chain_used) = chain_storage.merge_remote(remote_chain_msg)?;
                match chain_used {
                    ChainUsed::Remote => {
                        tracing::debug!(
                            "Continue remote chain: local group epoch={:?} state={:?}",
                            local_group.epoch(),
                            group_state(&local_group),
                        );
                        let stats = GroupCommitStats {
                            added: commit.add_proposals().count(),
                            removed: commit.remove_proposals().count(),
                            updated: commit.update_proposals().count(),
                        };

                        local_group.merge_staged_commit(*commit)?;
                        self.save(ctx, &mut local_group, &chain)?;
                        self.delete_newer_epochs(ctx, &local_group)?;
                        tracing::debug!(
                            "Merged remote commit: epoch={:?} state={:?} stats={:?}",
                            local_group.epoch(),
                            group_state(&local_group),
                            stats,
                        );

                        // Skip applying local changes to the group. They will be applied by remote device
                        // that won the chain.

                        Ok(GroupApplyResult::Commit {
                            group: SecretGroup {
                                mls: local_group,
                                chain,
                            },
                            messages_out: vec![],
                            stats,
                        })
                    }
                    ChainUsed::Local => {
                        // We picked local chain and applied remote changes atop
                        let mut latest_group = self.load_latest_mls(ctx, &group_id)?;
                        // Get all changes that were added from remote chain.
                        let (messages_out, stats) =
                            self.edit_group(ctx, &mut latest_group, &chain)?;
                        tracing::debug!(
                            "Continued local chain: latest group epoch={:?} state={:?} stats={:?}",
                            latest_group.epoch(),
                            group_state(&latest_group),
                            stats,
                        );

                        Ok(GroupApplyResult::Commit {
                            group: SecretGroup {
                                mls: latest_group,
                                chain,
                            },
                            messages_out,
                            stats,
                        })
                    }
                }
            }
        }
    }

    pub fn encrypt_message<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut SecretGroup,
        message: &AppMessage,
    ) -> Result<request::SecretGroupMessage> {
        let mls_out = group
            .mls
            .create_message(&ctx.backend(), message.encode_to_vec().as_slice())?;
        self.save(ctx, &mut group.mls, &group.chain)?;
        Ok(request::SecretGroupMessage {
            mls: mls_out.tls_serialize_detached()?,
            chain_hash: group.chain.head().to_string(),
            to_device_ids: group.device_ids()?.into_iter().map(|(id, _)| id).collect(),
        })
    }

    pub fn add<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut SecretGroup,
        packages: Vec<KeyPackage>,
    ) -> Result<Option<request::SecretGroupCommit>> {
        let backend = &ctx.backend();
        let chain_storage = SignatureChainStorage::new(
            ChangeAuthor {
                bundle: ctx.device().get_credential_bundle(ctx)?,
            },
            backend,
        );
        let block = chain_storage.add(&mut group.chain, packages)?;
        if let Some(block) = block {
            let (mls_commit, welcome) = group.mls.add_members(backend, &block.body.ops.add)?;
            group.mls.merge_pending_commit()?;
            tracing::trace!(
                "Added to group epoch={:?} state={:?}",
                group.mls.epoch(),
                group_state(&group.mls),
            );
            self.save(ctx, &mut group.mls, &group.chain)?;

            let message = build_commit_message(mls_commit, &group.chain, Some(welcome))?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    pub fn remove<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut SecretGroup,
        members: Vec<DeviceRemovedOp>,
    ) -> Result<Option<request::SecretGroupCommit>> {
        let backend = &ctx.backend();
        let chain_storage = SignatureChainStorage::new(
            ChangeAuthor {
                bundle: ctx.device().get_credential_bundle(ctx)?,
            },
            backend,
        );
        let block = chain_storage.remove(&mut group.chain, members)?;
        if let Some(block) = block {
            let removed = &block.body.ops.remove;
            let remove_refs: Vec<_> = removed.iter().map(|d| d.key_ref.clone()).collect();
            let (mls_commit, _) = group.mls.remove_members(backend, &remove_refs)?;
            group.mls.merge_pending_commit()?;
            self.save(ctx, &mut group.mls, &group.chain)?;

            let message = build_commit_message(mls_commit, &group.chain, None)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    pub fn self_update<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut SecretGroup,
        kpb: KeyPackageBundle,
    ) -> Result<Option<request::SecretGroupCommit>> {
        let backend = &ctx.backend();
        let chain_storage = SignatureChainStorage::new(
            ChangeAuthor {
                bundle: ctx.device().get_credential_bundle(ctx)?,
            },
            backend,
        );
        let block = chain_storage.update(&mut group.chain, vec![kpb.key_package().clone()])?;
        if let Some(_block) = block {
            let (mls_commit, _) = group.mls.self_update(backend, Some(kpb))?;
            group.mls.merge_pending_commit()?;
            self.save(ctx, &mut group.mls, &group.chain)?;

            let message = build_commit_message(mls_commit, &group.chain, None)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    fn edit_group<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        mls: &mut MlsGroup,
        chain: &SignatureChain,
    ) -> Result<(Vec<request::SecretGroupCommit>, GroupCommitStats)> {
        let mut messages_out = vec![];
        let mut chain_account_ids = chain.account_ids().to_vec();
        let backend = &ctx.backend();
        let mut stats = GroupCommitStats::default();

        for block in chain.changes_since(mls.epoch().as_u64()) {
            let body = &block.body;
            if !body.ops.add.is_empty() {
                stats.added += body.ops.add.len();
                let (mls_commit, welcome) = mls.add_members(backend, &body.ops.add)?;
                mls.merge_pending_commit()?;
                tracing::trace!(
                    "Edited group epoch={:?} state={:?}",
                    mls.epoch(),
                    group_state(&mls),
                );

                let message = build_commit_message(mls_commit, chain, Some(welcome))?;
                messages_out.push(message);
            } else if !body.ops.remove.is_empty() {
                stats.removed += body.ops.remove.len();
                let removed = &block.body.ops.remove;
                let remove_refs: Vec<_> = removed.iter().map(|d| d.key_ref.clone()).collect();
                let (mls_commit, _) = mls.remove_members(backend, &remove_refs)?;
                mls.merge_pending_commit()?;

                let message = build_commit_message(mls_commit, chain, None)?;
                messages_out.push(message);
            } else if !body.ops.update.is_empty() {
                // We can't really do this here...
                // Device should retry this operation :/
            } else {
                continue;
            }

            // Save group for each chain hash
            self.save_with_hash(ctx, mls, &block.hash, &mut chain_account_ids)?;
        }
        Ok((messages_out, stats))
    }

    fn save<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut MlsGroup,
        chain: &SignatureChain,
    ) -> Result<()> {
        let mut chain_account_ids = chain.account_ids().to_vec();
        self.save_with_hash(ctx, group, chain.head(), &mut chain_account_ids)
    }

    fn save_with_hash<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        group: &mut MlsGroup,
        chain_hash: &str,
        chain_account_ids: &mut [String],
    ) -> Result<()> {
        if let InnerState::Persisted = group.state_changed() {
            return Ok(());
        }

        let id = Self::group_id_str(group.group_id());
        let mut state = vec![];
        group.save(&mut state)?;

        let nonce_ciphertext = ctx.db_cipher().encrypt(state.as_ref())?;
        let epoch = group.epoch().as_u64();
        tracing::trace!(
            "Saving MLS group={} epoch={} state={:?}",
            id,
            epoch,
            group_state(&group)
        );

        let accounts_hash = if chain_account_ids.len() > 1 {
            Some(build_accounts_hash(chain_account_ids))
        } else {
            None
        };

        ctx.txn()
            .execute(
                r#"
INSERT INTO mls_groups (id, chain_hash, epoch, encrypted_state, accounts_hash) VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT (id, chain_hash) DO UPDATE
     SET encrypted_state = excluded.encrypted_state"#,
                params![id, chain_hash, epoch, nonce_ciphertext, accounts_hash],
            )
            .context("Insert mls_group")?;

        // Delete old epochs
        let last_epoch_to_keep = epoch.checked_sub(3).unwrap_or(0);
        ctx.txn().execute(
            "DELETE FROM mls_groups WHERE id = ? AND epoch < ?",
            params![id, last_epoch_to_keep],
        )?;

        Ok(())
    }

    fn delete_newer_epochs<'a>(&self, ctx: &impl WithTxn<'a>, group: &MlsGroup) -> Result<()> {
        let id = Self::group_id_str(group.group_id());
        ctx.txn().execute(
            "DELETE FROM mls_groups WHERE id = ? AND epoch > ?",
            params![id, group.epoch().as_u64()],
        )?;
        Ok(())
    }

    fn load_mls<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        id: &str,
        chain_hash: Option<&str>,
    ) -> Result<Option<MlsGroup>> {
        tracing::trace!(id, chain_hash, "Loading MLS");
        if let Some(hash) = chain_hash {
            Self::query_row(
                ctx,
                r#"
SELECT encrypted_state
  FROM mls_groups
 WHERE id = ? AND chain_hash = ?
 LIMIT 1"#,
                params![id, hash],
            )
            .with_context(|| format!("Find MlsGroup id={} chain_hash={}", id, hash))
        } else {
            Self::query_row(
                ctx,
                r#"
SELECT encrypted_state
  FROM mls_groups
 WHERE id = ?
 ORDER BY epoch DESC
 LIMIT 1"#,
                params![id],
            )
            .with_context(|| format!("Find latest MlsGroup id={}", id))
        }
    }

    fn query_row<'a>(
        ctx: &impl WithTxn<'a>,
        query: &str,
        params: impl Params,
    ) -> Result<Option<MlsGroup>> {
        let mut stmt = ctx.txn().prepare(query)?;
        let mut rows = stmt.query(params)?;
        if let Some(row) = rows.next()? {
            let group = Self::read_row(ctx, row)?;
            Ok(Some(group))
        } else {
            Ok(None)
        }
    }

    fn read_row<'a>(ctx: &impl WithTxn<'a>, row: &Row) -> Result<MlsGroup> {
        let nonce_ciphertext: Vec<u8> = row.get(0)?;
        let state = ctx.db_cipher().decrypt(nonce_ciphertext.as_ref())?;
        let group = MlsGroup::load::<&[u8]>(state.as_ref())?;
        Ok(group)
    }

    fn load_latest_mls<'a>(&self, ctx: &impl WithTxn<'a>, id: &str) -> Result<MlsGroup> {
        let group = self
            .load_mls(ctx, id, None)?
            .ok_or(anyhow!("MlsGroup not found id={}", id))?;
        Ok(group)
    }

    pub fn load_latest<'a>(&self, ctx: &impl WithTxn<'a>, id: &str) -> Result<SecretGroup> {
        let mls = self.load_latest_mls(ctx, id)?;
        let chain = SignatureChainStorage::load(ctx.txn(), id)?.ok_or(anyhow!("Missing chain"))?;
        Ok(SecretGroup { mls, chain })
    }

    pub fn load_latest_for_accounts<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        account_ids: &mut [String],
    ) -> Result<SecretGroup> {
        let accounts_hash = build_accounts_hash(account_ids);
        let mls = Self::query_row(
            ctx,
            r#"
SELECT encrypted_state
  FROM mls_groups
 WHERE accounts_hash = ?
 ORDER BY epoch DESC
 LIMIT 1"#,
            params![accounts_hash],
        )
        .with_context(|| format!("Find latest MlsGroup accounts={:?}", account_ids))?
        .ok_or(anyhow!("MlsGroup not found"))?;
        let id = Self::group_id_str(mls.group_id());
        let chain = SignatureChainStorage::load(ctx.txn(), &id)?.ok_or(anyhow!("Missing chain"))?;
        Ok(SecretGroup { mls, chain })
    }

    pub fn exists<'a>(&self, ctx: &impl SecretGroupCtx<'a>, id: &str) -> Result<bool> {
        let row = ctx
            .txn()
            .query_row(
                "SELECT 1 FROM mls_groups WHERE id = ?",
                params![id],
                |_row| Ok(()),
            )
            .optional()?;
        Ok(row.is_some())
    }

    pub fn exists_for_accounts<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        account_ids: &mut [String],
    ) -> Result<bool> {
        if account_ids.len() == 1 {
            self.exists(ctx, &account_ids[0])
        } else {
            let accounts_hash = build_accounts_hash(account_ids);
            let row = ctx
                .txn()
                .query_row(
                    "SELECT 1 FROM mls_groups WHERE accounts_hash = ?",
                    params![accounts_hash],
                    |_row| Ok(()),
                )
                .optional()?;
            Ok(row.is_some())
        }
    }

    fn mls_group_config() -> MlsGroupConfig {
        MlsGroupConfigBuilder::new()
            // Send ratchet tree in handshake messages
            .use_ratchet_tree_extension(true)
            .sender_ratchet_configuration(SenderRatchetConfiguration::new(
                // out_of_order_tolerance:
                // This parameter defines a window for which decryption secrets are kept.
                // This is useful in case the DS cannot guarantee that all application messages
                // have total order within an epoch. Use this carefully, since keeping decryption
                // secrets affects forward secrecy within an epoch. The default value is 0
                100,
                // maximum_forward_distance: This parameter defines how many incoming messages can be skipped.
                // This is useful if the DS drops application messages. The default value is 1000.
                SenderRatchetConfiguration::default().maximum_forward_distance(),
            ))
            .build()
    }

    pub fn group_id_str(group_id: &GroupId) -> String {
        String::from_utf8_lossy(group_id.as_slice()).to_string()
    }

    /// For each participant find a group, create a mls message with all secrets
    pub fn queue_secrets<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        acc: &AccView,
        secrets: Vec<DocSecretRow>,
        accounts: &mut [String],
    ) -> Result<()> {
        for account in accounts {
            let mut group = if &acc.id == account {
                self.load_latest(ctx, &acc.id)?
            } else {
                // Load group between this account and target account
                let mut chain_account_ids = vec![acc.id.clone(), account.clone()];
                match self.load_latest_for_accounts(ctx, &mut chain_account_ids) {
                    Ok(group) => group,
                    Err(err) => {
                        tracing::warn!("Not sending secret to account={}: {}", account, err);
                        continue;
                    }
                }
            };

            let message = self.encrypt_message(
                ctx,
                &mut group,
                &AppMessage {
                    value: Some(app_message::Value::Secrets(DocSecretsMessage {
                        values: secrets
                            .clone()
                            .into_iter()
                            .map(|s| doc_secrets_message::Secret {
                                id: s.id,
                                secret: s.key,
                                algorithm: s.algorithm,
                                created_at_sec: s.created_at.timestamp(),
                                account_ids: s.account_ids,
                                doc_id: s.doc_id,
                            })
                            .collect(),
                    })),
                },
            )?;
            mailbox::queue_mls_message(ctx, message)?;
        }

        Ok(())
    }

    /// Create a secret group with another account if it is missing. Returns how many groups were created.
    pub async fn create_missing_groups<'a>(
        &self,
        ctx: &impl SecretGroupAsyncCtx<C>,
        acc: &AccView,
        accounts: &mut [String],
    ) -> u32 {
        let mut created = 0;
        for account in accounts {
            if &acc.id == account {
                continue;
            } else {
                match self.create_for_accounts(ctx, &acc.id, &account).await {
                    Ok(_) => {
                        created += 1;
                    }
                    Err(err) => {
                        tracing::warn!("Failed to create a group with acc_id={}: {}", account, err);
                    }
                }
            }
        }

        created
    }

    /// Create a new secret group for given account ids. Doesn't do anything if group already exists.
    pub async fn create_for_accounts(
        &self,
        ctx: &impl SecretGroupAsyncCtx<C>,
        this_acc_id: &str,
        other_acc_id: &str,
    ) -> Result<()> {
        let group_exists = ctx.in_txn(|tx_ctx| {
            let mut account_ids = vec![this_acc_id.to_string(), other_acc_id.to_string()];
            self.exists_for_accounts(tx_ctx, &mut account_ids)
        })?;

        if group_exists {
            return Ok(());
        }

        // Fetch account devices (key packages)
        let this_account = self.client.get_account_devices(this_acc_id).await?;
        // Fetch other account's chain and unused key packages
        let other_account = self.client.get_account_devices(other_acc_id).await?;

        ctx.in_txn(|tx_ctx| {
            let backend = &tx_ctx.backend();
            let mut group_members = HashMap::new();

            // Build a list of new group members
            for (devices, acc_id) in [this_account, other_account]
                .into_iter()
                .zip([this_acc_id, other_acc_id])
            {
                let chain = SignatureChain::decode(
                    devices
                        .chain
                        .ok_or(anyhow!("AccountDevices is missing the chain"))?,
                )?;
                chain.verify(backend)?;
                if chain.root() != acc_id {
                    bail!(
                        "Chain root doesn't match account id ({} != {acc_id})",
                        chain.root()
                    );
                }

                let members = chain.members(backend.crypto())?;
                for message in devices.key_packages {
                    let package = KeyPackage::tls_deserialize(&mut message.data.as_slice())?;
                    let device_id = get_device_id(package.credential())?;
                    if members.find_by_id(&device_id).is_some() && device_id != ctx.device().id {
                        // Include only devices that signature chain knows of
                        group_members.insert(device_id, package);
                    }
                }
            }

            if group_members.is_empty() {
                return Err(anyhow!("Devices list is empty"));
            }

            // Create secret group
            let mut group = self.do_create(
                tx_ctx,
                Some(vec![this_acc_id.to_string(), other_acc_id.to_string()]),
            )?;
            tracing::info!(
                group_id = group.id(),
                chain_hash = group.chain.head(),
                add_members = group_members.len(),
                "Creating new MLS group for contact"
            );

            let commit = self.add(
                tx_ctx,
                &mut group,
                group_members.into_iter().map(|(_, p)| p).collect(),
            )?;
            if let Some(c) = commit {
                mailbox::queue_mls_commit(tx_ctx, c)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Add a device to all known groups.
    pub fn add_to_all_groups<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        mut device_key_packages: Vec<KeyPackage>,
    ) -> Result<()> {
        if device_key_packages.is_empty() {
            bail!("New device has no available KeyPackages");
        }

        self.with_all_groups(ctx, |group| {
            // Keep last key package for later use
            let package = if device_key_packages.len() > 2 {
                device_key_packages.pop().unwrap()
            } else {
                device_key_packages[0].clone()
            };

            let commit = self.add(ctx, group, vec![package])?;
            if let Some(commit) = commit {
                mailbox::queue_mls_commit(ctx, commit)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Remove a device from all known groups.
    pub fn remove_from_all_groups<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        device_id: &str,
    ) -> Result<()> {
        self.with_all_groups(ctx, |group| {
            // Remove device if is a member
            let member_ref = group.find_member_ref(device_id, ctx.backend().crypto());
            if let Some(member_ref) = member_ref {
                let last_counter = ctx.device().get_clock(ctx, device_id)?;
                let commit = self.remove(
                    ctx,
                    group,
                    vec![DeviceRemovedOp {
                        key_ref: member_ref,
                        last_counter,
                    }],
                )?;
                if let Some(commit) = commit {
                    mailbox::queue_mls_commit(ctx, commit)?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Inform all known groups that this device wants to leave them.
    pub fn leave_all_groups<'a>(&self, ctx: &impl SecretGroupCtx<'a>) -> Result<()> {
        self.with_all_groups(ctx, |group| {
            // Send remove suggestion
            let message = self.encrypt_message(
                ctx,
                group,
                &AppMessage {
                    value: Some(app_message::Value::RemoveMe(RemoveMe {})),
                },
            )?;
            mailbox::queue_mls_message(ctx, message)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Iterate over all known groups
    fn with_all_groups<'a>(
        &self,
        ctx: &impl SecretGroupCtx<'a>,
        mut cb: impl FnMut(&mut SecretGroup) -> Result<()>,
    ) -> Result<()> {
        // Go through each known group (select latest epochs)
        let mut stmt = ctx.txn().prepare(
            r#"
SELECT encrypted_state
  FROM mls_groups g
 INNER JOIN (
             SELECT id, MAX(epoch) as epoch
               FROM mls_groups
              GROUP BY id
            ) as sub
    ON g.id = sub.id AND g.epoch = sub.epoch"#,
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            // Read secret group
            let mls = Self::read_row(ctx, row)?;
            let id = Self::group_id_str(mls.group_id());
            let chain =
                SignatureChainStorage::load(ctx.txn(), &id)?.ok_or(anyhow!("Missing chain"))?;
            let mut group = SecretGroup { mls, chain };

            // Process secret group
            cb(&mut group)?;
        }

        Ok(())
    }
}

fn group_state(g: &MlsGroup) -> String {
    format!("{:?}", g.authentication_secret().as_slice().split_at(6).0)
}

pub enum GroupApplyResult {
    /// Secret group received an application message
    AppMessage {
        group_id: String,
        message: ApplicationMessage,
        sender: Sender,
    },
    /// Secret group was modified
    Commit {
        group: SecretGroup,
        messages_out: Vec<request::SecretGroupCommit>,
        stats: GroupCommitStats,
    },
    /// Received message from unknown group. This could happen if we receive a message
    /// for the remote chain that was dropped in favour of local one.
    UnknownGroup,
    /// This is temporary until we start supporting proposals
    Nothing,
}

#[derive(Default, Debug)]
pub struct GroupCommitStats {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

fn build_commit_message(
    mls_commit: MlsMessageOut,
    chain: &SignatureChain,
    welcome: Option<Welcome>,
) -> Result<request::SecretGroupCommit> {
    let epoch = mls_commit.epoch().as_u64();
    Ok(request::SecretGroupCommit {
        mls: mls_commit.tls_serialize_detached()?,
        welcome: match welcome {
            Some(w) => Some(w.tls_serialize_detached()?),
            None => None,
        },
        chain: Some(chain.encode_at_epoch(epoch + 1)?),
    })
}
