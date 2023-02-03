use anyhow::Context;
use bolik_chain::{
    ApplyBlock, ChainBlock, ChainError, ChainUsed, ChangeAuthor, DeviceOps, DeviceRemovedOp,
    MergeAdvice, SignatureChain,
};
use bolik_migrations::rusqlite::{params, Connection, OptionalExtension};
use bolik_proto::sync;
use openmls::prelude::{KeyPackage, KeyPackageBundle, TlsSerializeTrait};
use openmls_traits::OpenMlsCryptoProvider;

use crate::{db::StringListWriteColumn, secrets::SqliteCryptoProvider};

pub(crate) struct SignatureChainStorage<'a> {
    pub(crate) author: ChangeAuthor,
    backend: &'a SqliteCryptoProvider<'a>,
}

impl<'a> SignatureChainStorage<'a> {
    pub fn new(author: ChangeAuthor, backend: &'a SqliteCryptoProvider) -> Self {
        Self { author, backend }
    }

    pub fn create_with_key(
        &self,
        key_bundle: KeyPackageBundle,
        account_ids: Option<Vec<String>>,
    ) -> anyhow::Result<SignatureChain> {
        let chain = SignatureChain::new(
            self.author.clone(),
            key_bundle,
            account_ids.unwrap_or_default(),
            self.backend,
        )?;
        self.save(&chain)?;
        Ok(chain)
    }

    pub fn merge_remote(
        &self,
        remote_chain_msg: sync::SignatureChain,
    ) -> anyhow::Result<(SignatureChain, ChainUsed)> {
        let remote_chain = SignatureChain::decode(remote_chain_msg)?;
        let local_chain = match Self::load(self.backend.conn, remote_chain.root())? {
            Some(c) => c,
            None => {
                // New chain --> just store it
                self.save(&remote_chain)?;
                return Ok((remote_chain, ChainUsed::Remote));
            }
        };

        let MergeAdvice {
            mut chain,
            used,
            remote_blocks,
        } = local_chain.prepare_merge(remote_chain, self.backend)?;

        if remote_blocks.is_empty() {
            self.save(&chain)?;
        } else {
            for block in remote_blocks {
                match chain.modify(
                    ApplyBlock::RemoteBlock(block),
                    self.author.clone(),
                    self.backend,
                ) {
                    Ok(_) => {}
                    Err(ChainError::NonMemberEdit) => {
                        tracing::warn!("Stopping chain modification: edit by non-member");
                        break;
                    }
                    Err(err) => {
                        return Err(err.into());
                    }
                }
            }
            self.save(&chain)?;
        }

        Ok((chain, used))
    }

    pub fn add<'b>(
        &self,
        chain: &'b mut SignatureChain,
        additions: Vec<KeyPackage>,
    ) -> anyhow::Result<Option<&'b ChainBlock>> {
        let ops = DeviceOps {
            add: additions,
            remove: vec![],
            update: vec![],
        };
        let block = self.modify(chain, ops)?;
        Ok(block)
    }

    pub fn remove<'b>(
        &self,
        chain: &'b mut SignatureChain,
        removals: Vec<DeviceRemovedOp>,
    ) -> anyhow::Result<Option<&'b ChainBlock>> {
        let ops = DeviceOps {
            add: vec![],
            remove: removals,
            update: vec![],
        };
        let block = self.modify(chain, ops)?;
        Ok(block)
    }

    pub fn update<'b>(
        &self,
        chain: &'b mut SignatureChain,
        updates: Vec<KeyPackage>,
    ) -> anyhow::Result<Option<&'b ChainBlock>> {
        let ops = DeviceOps {
            add: vec![],
            remove: vec![],
            update: updates,
        };
        let block = self.modify(chain, ops)?;
        Ok(block)
    }

    fn modify<'b>(
        &self,
        chain: &'b mut SignatureChain,
        ops: DeviceOps,
    ) -> anyhow::Result<Option<&'b ChainBlock>> {
        let modified =
            chain.modify(ApplyBlock::LocalOps(ops), self.author.clone(), self.backend)?;
        self.save(chain)?;
        if modified {
            Ok(chain.last_block())
        } else {
            Ok(None)
        }
    }

    fn save(&self, chain: &SignatureChain) -> anyhow::Result<()> {
        let chain_bytes = chain.encode_bytes()?;
        let conn = self.backend.conn;
        let account_ids = chain.account_ids();

        let query = r#"
INSERT INTO signature_chains (id, chain, account_ids) VALUES (?1, ?2, ?3)
  ON CONFLICT (id) DO UPDATE SET chain = excluded.chain"#;

        let chain_id = chain.root();
        conn.execute(
            &query,
            params![chain_id, chain_bytes, StringListWriteColumn(account_ids)],
        )
        .context("Insert signature_chain")?;

        // Update the list of signature devices
        conn.execute(
            "DELETE FROM signature_chain_devices WHERE chain_id = ?",
            [chain_id],
        )
        .context("Clear signature_chain_devices")?;

        let members = chain.members(self.backend.crypto())?;
        for (device_id, member) in members.device_ids {
            conn.execute(
                "INSERT INTO signature_chain_devices (device_id, chain_id, credential) VALUES (?1, ?2, ?3)", params![
                device_id,
                chain_id,
                member.package.credential().tls_serialize_detached()?,
            ])?;
        }
        for (device_id, member) in members.removed {
            conn.execute("INSERT INTO signature_chain_devices (device_id, chain_id, credential, last_counter) VALUES (?1, ?2, ?3, ?4)", params![
                device_id,
                chain_id,
                member.package.credential().tls_serialize_detached()?,
                member.last_counter
            ])?;
        }

        Ok(())
    }

    pub fn load(conn: &Connection, id: &str) -> anyhow::Result<Option<SignatureChain>> {
        let chain_bytes: Option<Vec<u8>> = conn
            .query_row(
                "SELECT chain FROM signature_chains WHERE id = ?",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(bytes) = chain_bytes {
            let chain = SignatureChain::decode_bytes(&bytes)?;
            Ok(Some(chain))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        fs::File,
        io::{BufRead, Write},
    };

    use anyhow::{anyhow, ensure, Result};
    use bolik_migrations::rusqlite::Connection;
    use openmls::prelude::{
        Credential, CredentialBundle, CredentialType, KeyPackage, KeyPackageBundle, KeyPackageRef,
        OpenMlsKeyStore, TlsSerializeTrait,
    };
    use openmls_rust_crypto::{OpenMlsRustCrypto, RustCrypto};
    use openmls_traits::{
        key_store::{FromKeyStoreValue, ToKeyStoreValue},
        OpenMlsCryptoProvider,
    };

    use crate::{
        db::migrations,
        device::get_device_id,
        generate_db_key,
        secrets::{DbCipher, SqliteCryptoProvider, CIPHERSUITES, DEFAULT_CIPHERSUITE},
        signature_chain::{ChainUsed, DeviceRemovedOp},
    };

    use super::{ChangeAuthor, SignatureChain, SignatureChainStorage};

    #[test]
    fn test_chain_linear_history() {
        //
        //   add A ─ add B ─ add C
        //

        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, used) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        assert_eq!(used, ChainUsed::Remote);
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("HMKQ9v", &chain_b).unwrap();

        // B adds C
        storage_b.add(&mut chain_b, vec![c.package()]).unwrap();
        compare_devs(&[&a, &b, &c], &chain_b).unwrap();
        assert_chain_head("9rYpG8", &chain_b).unwrap();

        // A integrates B's changes
        let chain_b_msg = chain_b.encode().unwrap();
        let (chain_a, used) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_eq!(used, ChainUsed::Remote);
        compare_devs(&[&a, &b, &c], &chain_a).unwrap();
        assert_chain_head("9rYpG8", &chain_a).unwrap();
    }

    #[test]
    fn test_chain_add_multiple() {
        // A adds C and D, while B adds E:
        //
        //                  ┌─ add [C, D]
        //   add A ─ add B ─┤
        //                  └─ add E
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ add [C, D] ─ add E
    }

    #[test]
    fn test_chain_remove_multiple() {
        // A removes C, while B removes C and D:
        //
        //                          ┌─ rm C
        //   add A ─ add [B, C, D] ─┤
        //                          └─ rm [C, D]
        //
        // We expect chain to be:
        //
        //   add A ─ add [B, C, D] ─ rm C ─ rm D

        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();
        let d = get_device_d();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B, C, D
        storage_a
            .add(&mut chain_a, vec![b.package(), c.package(), d.package()])
            .unwrap();
        compare_devs(&[&a, &b, &c, &d], &chain_a).unwrap();
        assert_chain_head("2RxrXc", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        compare_devs(&[&a, &b, &c, &d], &chain_b).unwrap();
        assert_chain_head("2RxrXc", &chain_b).unwrap();

        // A removes C
        storage_a
            .remove(
                &mut chain_a,
                vec![DeviceRemovedOp {
                    key_ref: c.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap();
        compare_devs(&[&a, &b, &d], &chain_a).unwrap();
        assert_chain_head("Bb54sF", &chain_a).unwrap();

        // B removes C, D
        storage_b
            .remove(
                &mut chain_b,
                vec![
                    DeviceRemovedOp {
                        key_ref: c.key_ref(),
                        last_counter: 1,
                    },
                    DeviceRemovedOp {
                        key_ref: d.key_ref(),
                        last_counter: 2,
                    },
                ],
            )
            .unwrap();
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("8hcQYB", &chain_b).unwrap();

        // Prepare messages
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        // A integrates B's changes
        let (chain_a, used) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_eq!(used, ChainUsed::Local);
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("7in9s5", &chain_a).unwrap();

        // B integrates A's changes
        let (chain_b, used) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(used, ChainUsed::Remote);
        compare_devs(&[&a, &b, &d], &chain_b).unwrap();
        assert_chain_head("Bb54sF", &chain_b).unwrap();

        // We need to do a merge again so that chains converge.
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_chain_head("7in9s5", &chain_a).unwrap();
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_chain_head("7in9s5", &chain_b).unwrap();

        // Chain remembers all devices
        let members = chain_a.members(backend_a.crypto()).unwrap();
        assert_eq!(2, members.device_ids.len());
        assert_eq!(2, members.removed.len());
        assert_eq!(members.removed.get(&c.id).unwrap().last_counter, 1);
        assert_eq!(members.removed.get(&d.id).unwrap().last_counter, 2);
    }

    #[test]
    fn test_chain_diverged_multiple_blocks() {
        // A adds C and D, while B adds E:
        //
        //                  ┌─ add C ─ add D
        //   add A ─ add B ─┤
        //                  └─ add E
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ add C ─ add D ─ add E

        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();
        let d = get_device_d();
        let e = get_device_e();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("HMKQ9v", &chain_b).unwrap();

        // A adds C
        storage_a.add(&mut chain_a, vec![c.package()]).unwrap();
        compare_devs(&[&a, &b, &c], &chain_a).unwrap();
        assert_chain_head("xctJFo", &chain_a).unwrap();

        // A adds D
        storage_a.add(&mut chain_a, vec![d.package()]).unwrap();
        compare_devs(&[&a, &b, &c, &d], &chain_a).unwrap();
        assert_chain_head("Cf8Mwv", &chain_a).unwrap();

        // B adds E
        storage_b.add(&mut chain_b, vec![e.package()]).unwrap();
        compare_devs(&[&a, &b, &e], &chain_b).unwrap();
        assert_chain_head("8QHLyH", &chain_b).unwrap();

        // Prepare messages
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        // A integrates B's changes
        let (chain_a, used) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_eq!(used, ChainUsed::Local);
        compare_devs(&[&a, &b, &c, &d, &e], &chain_a).unwrap();
        assert_chain_head("3t94eS", &chain_a).unwrap();

        // B integrates A's changes
        let (chain_b, used) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(used, ChainUsed::Remote);
        compare_devs(&[&a, &b, &c, &d], &chain_b).unwrap();
        assert_chain_head("Cf8Mwv", &chain_b).unwrap();

        // We need to do a merge again so that chains converge.
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_chain_head("3t94eS", &chain_a).unwrap();
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_chain_head("3t94eS", &chain_b).unwrap();
    }

    #[test]
    fn test_chain_diverged_multiple_blocks_reverse() {
        // A adds E, while B adds C and D:
        //
        //                  ┌─ add E
        //   add A ─ add B ─┤
        //                  └─ add C ─ add D
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ add E ─ add C ─ add D

        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();
        let d = get_device_d();
        let e = get_device_e();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("HMKQ9v", &chain_b).unwrap();

        // A adds E
        storage_a.add(&mut chain_a, vec![e.package()]).unwrap();
        compare_devs(&[&a, &b, &e], &chain_a).unwrap();
        assert_chain_head("3o9CcQ", &chain_a).unwrap();

        // B adds C
        storage_b.add(&mut chain_b, vec![c.package()]).unwrap();
        compare_devs(&[&a, &b, &c], &chain_b).unwrap();
        assert_chain_head("9rYpG8", &chain_b).unwrap();

        // B adds D
        storage_b.add(&mut chain_b, vec![d.package()]).unwrap();
        compare_devs(&[&a, &b, &c, &d], &chain_b).unwrap();
        assert_chain_head("8BnBw2", &chain_b).unwrap();

        // Prepare messages
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        // A integrates B's changes
        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        compare_devs(&[&a, &b, &c, &d, &e], &chain_a).unwrap();
        assert_eq!(5, chain_a.len());
        assert_chain_head("GCoDwz", &chain_a).unwrap();

        // B integrates A's changes
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        compare_devs(&[&a, &b, &e], &chain_b).unwrap();
        assert_eq!(3, chain_b.len());
        assert_chain_head("3o9CcQ", &chain_b).unwrap();

        // We need to do a merge again so that chains converge.
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        assert_chain_head("GCoDwz", &chain_a).unwrap();
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_chain_head("GCoDwz", &chain_b).unwrap();
    }

    #[test]
    fn test_chain_mutual_remove() {
        // A removes B, while B removes A:
        //
        //                  ┌─ rm B
        //   add A ─ add B ─┤
        //                  └─ rm A
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ rm B

        let a = get_device_a();
        let b = get_device_b();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("HMKQ9v", &chain_b).unwrap();

        // A removes B
        storage_a
            .remove(
                &mut chain_a,
                vec![DeviceRemovedOp {
                    key_ref: b.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap();
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("GApEKZ", &chain_a).unwrap();

        // B removes A
        storage_b
            .remove(
                &mut chain_b,
                vec![DeviceRemovedOp {
                    key_ref: a.key_ref(),
                    last_counter: 2,
                }],
            )
            .unwrap();
        compare_devs(&[&b], &chain_b).unwrap();
        assert_chain_head("5ybeKB", &chain_b).unwrap();

        // Prepare messages
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        // A integrates B's changes (B change should be excluded)
        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        compare_devs(&[&a], &chain_a).unwrap();
        assert_eq!(3, chain_a.len());
        assert_chain_head("GApEKZ", &chain_a).unwrap();

        // B integrates A's changes (B change should be excluded)
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        compare_devs(&[&a], &chain_b).unwrap();
        assert_eq!(3, chain_b.len());
        assert_chain_head("GApEKZ", &chain_b).unwrap();
    }

    #[test]
    fn test_chain_add_by_removed() {
        // A removes B, while B adds C and D:
        //
        //                  ┌─ rm B
        //   add A ─ add B ─┤
        //                  └─ add C ─ add D
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ rm B

        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();
        let d = get_device_d();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);
        let backend_b = backend_of(&b);
        let storage_b = storage_of(&b, &backend_b);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // B merges chain
        let chain_a_msg = chain_a.encode().unwrap();
        let (mut chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        assert_eq!(chain_b.root(), chain_a.root());
        compare_devs(&[&a, &b], &chain_b).unwrap();
        assert_chain_head("HMKQ9v", &chain_b).unwrap();

        // A removes B
        storage_a
            .remove(
                &mut chain_a,
                vec![DeviceRemovedOp {
                    key_ref: b.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap();
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("GApEKZ", &chain_a).unwrap();

        // B adds C
        storage_b.add(&mut chain_b, vec![c.package()]).unwrap();
        compare_devs(&[&a, &b, &c], &chain_b).unwrap();
        assert_chain_head("9rYpG8", &chain_b).unwrap();

        // B adds D
        storage_b.add(&mut chain_b, vec![d.package()]).unwrap();
        compare_devs(&[&a, &b, &c, &d], &chain_b).unwrap();
        assert_chain_head("8BnBw2", &chain_b).unwrap();

        // Prepare messages
        let chain_a_msg = chain_a.encode().unwrap();
        let chain_b_msg = chain_b.encode().unwrap();

        // A integrates B's changes (B change should be excluded)
        let (chain_a, _) = storage_a.merge_remote(chain_b_msg).unwrap();
        compare_devs(&[&a], &chain_a).unwrap();
        assert_eq!(3, chain_a.len());
        assert_chain_head("GApEKZ", &chain_a).unwrap();

        // B integrates A's changes (B change should be excluded)
        let (chain_b, _) = storage_b.merge_remote(chain_a_msg).unwrap();
        compare_devs(&[&a], &chain_b).unwrap();
        assert_eq!(3, chain_b.len());
        assert_chain_head("GApEKZ", &chain_b).unwrap();
    }

    #[test]
    fn test_chain_duplicate_members() {
        let a = get_device_a();
        let b = get_device_b();
        let c = get_device_c();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // Try to add the same devices --> chain should not change
        storage_a
            .add(&mut chain_a, vec![a.new_package(), b.new_package()])
            .unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // Try to remove non-members --> chain should not change
        storage_a
            .remove(
                &mut chain_a,
                vec![DeviceRemovedOp {
                    key_ref: c.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();
    }

    #[test]
    fn test_chain_add_removed() {
        let a = get_device_a();
        let b = get_device_b();

        let backend_a = backend_of(&a);
        let storage_a = storage_of(&a, &backend_a);

        // A creates a chain
        let mut chain_a = storage_a
            .create_with_key(a.key_bundle.clone(), None)
            .unwrap();
        assert_eq!(chain_a.head(), chain_a.root());
        compare_devs(&[&a], &chain_a).unwrap();
        assert_chain_head("2eRxMm", &chain_a).unwrap();

        // A adds B
        storage_a.add(&mut chain_a, vec![b.package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_chain_head("HMKQ9v", &chain_a).unwrap();

        // A removes B
        storage_a
            .remove(
                &mut chain_a,
                vec![DeviceRemovedOp {
                    key_ref: b.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap();
        assert_chain_head("GApEKZ", &chain_a).unwrap();
        compare_devs(&[&a], &chain_a).unwrap();

        // A adds B again
        storage_a.add(&mut chain_a, vec![b.new_package()]).unwrap();
        compare_devs(&[&a, &b], &chain_a).unwrap();
        assert_eq!(
            0,
            chain_a.members(backend_a.crypto()).unwrap().removed.len()
        );
    }

    fn db_conn() -> Result<(Connection, DbCipher)> {
        let conn_path = format!("file:mem{}?mode=memory", rand::random::<u16>());
        let conn = Connection::open(conn_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations::apply(&conn)?;
        let db_key = generate_db_key();
        let db_cipher = DbCipher::new(&db_key);
        Ok((conn, db_cipher))
    }

    struct TestDevice {
        conn: Connection,
        db_cipher: DbCipher,
        id: String,
        bundle: CredentialBundle,
        key_bundle: KeyPackageBundle,
        crypto: RustCrypto,
    }

    impl TestDevice {
        fn from_reader(reader: impl BufRead) -> Result<Self> {
            let mut lines = reader.lines();
            let first = lines.next().unwrap().unwrap();
            let second = lines.next().unwrap().unwrap();

            let cred_bytes = first.as_bytes();
            let key_bytes = second.as_bytes();
            Self::from_lines(cred_bytes, key_bytes)
        }

        fn from_lines(cred_bytes: &[u8], key_bytes: &[u8]) -> Result<Self> {
            let (conn, db_cipher) = db_conn()?;
            let crypto = RustCrypto::default();
            let bundle = CredentialBundle::from_key_store_value(cred_bytes)?;
            let id = get_device_id(bundle.credential())?;
            let credential_id_bytes: Vec<u8> = bundle
                .credential()
                .signature_key()
                .tls_serialize_detached()?;
            SqliteCryptoProvider::new(&db_cipher, &conn, &crypto)
                .key_store()
                .store(&credential_id_bytes, &bundle)
                .map_err(|err| anyhow!("{:?}", err))?;

            let key_bundle = KeyPackageBundle::from_key_store_value(key_bytes)?;

            Ok(Self {
                bundle,
                id,
                conn,
                db_cipher,
                key_bundle,
                crypto,
            })
        }

        fn author(&self) -> ChangeAuthor {
            ChangeAuthor {
                bundle: self.bundle.clone(),
            }
        }

        #[allow(unused)]
        fn credential(&self) -> Credential {
            self.bundle.credential().clone()
        }

        fn package(&self) -> KeyPackage {
            self.key_bundle.clone().into_parts().0
        }

        fn new_package(&self) -> KeyPackage {
            let backend = backend_of(self);
            let key_bundle =
                KeyPackageBundle::new(&CIPHERSUITES, &self.bundle, &backend, vec![]).unwrap();
            key_bundle.into_parts().0
        }

        fn key_ref(&self) -> KeyPackageRef {
            self.key_bundle
                .key_package()
                .hash_ref(&RustCrypto::default())
                .unwrap()
        }
    }

    fn backend_of(d: &TestDevice) -> SqliteCryptoProvider {
        SqliteCryptoProvider::new(&d.db_cipher, &d.conn, &d.crypto)
    }

    fn storage_of<'a>(
        d: &TestDevice,
        backend: &'a SqliteCryptoProvider,
    ) -> SignatureChainStorage<'a> {
        SignatureChainStorage::new(d.author(), backend)
    }

    fn compare_devs(expected: &[&TestDevice], chain: &SignatureChain) -> Result<()> {
        let expected_ids: HashSet<_> = expected.iter().map(|d| d.id.as_ref()).collect();
        let members = chain.members(&RustCrypto::default())?;
        let chain_ids = members.device_ids();
        if expected_ids == chain_ids {
            Ok(())
        } else {
            Err(anyhow!(
                "expected != chain.devices\nexpected(len={}): {:?},\nchain(len={}):    {:?}",
                expected_ids.len(),
                expected_ids,
                chain_ids.len(),
                chain_ids
            ))
        }
    }

    fn assert_chain_head(begin: &str, chain: &SignatureChain) -> Result<()> {
        let head = chain.head();
        ensure!(
            head.starts_with(begin),
            "assert_chain_head:\n{:?} doesn't start with '{}'",
            head,
            begin
        );
        Ok(())
    }

    fn get_device_a() -> TestDevice {
        // Generated with:
        // gen_packages("A");
        let bytes = include_bytes!("../../test_data/device-A.bundles");
        TestDevice::from_reader(bytes.as_ref()).unwrap()
    }

    fn get_device_b() -> TestDevice {
        let bytes = include_bytes!("../../test_data/device-B.bundles");
        TestDevice::from_reader(bytes.as_ref()).unwrap()
    }

    fn get_device_c() -> TestDevice {
        let bytes = include_bytes!("../../test_data/device-C.bundles");
        TestDevice::from_reader(bytes.as_ref()).unwrap()
    }

    fn get_device_d() -> TestDevice {
        let bytes = include_bytes!("../../test_data/device-D.bundles");
        TestDevice::from_reader(bytes.as_ref()).unwrap()
    }

    fn get_device_e() -> TestDevice {
        let bytes = include_bytes!("../../test_data/device-E.bundles");
        TestDevice::from_reader(bytes.as_ref()).unwrap()
    }

    // #[allow(unused)]
    // fn print_chain(chain: &SignatureChain) {
    //     println!("SignatureChain (len={}):", chain.blocks.len());
    //     for block in &chain.blocks {
    //         let hash: String = block.hash.chars().take(6).collect();
    //         let author: String = block.body.authored_by.chars().take(6).collect();
    //         println!(
    //             "epoch={}: {} by={} added={} removed={}",
    //             block.body.epoch,
    //             hash,
    //             author,
    //             block.body.ops.add.len(),
    //             block.body.ops.remove.len(),
    //         );
    //     }
    // }

    #[allow(unused)]
    fn gen_packages(identity: &str) {
        let backend = &OpenMlsRustCrypto::default();
        let cred_bundle = CredentialBundle::new(
            identity.as_bytes().to_vec(),
            CredentialType::Basic,
            DEFAULT_CIPHERSUITE.signature_algorithm(),
            backend,
        )
        .unwrap();
        let key_bundle =
            KeyPackageBundle::new(&CIPHERSUITES, &cred_bundle, backend, vec![]).unwrap();

        let cred_bytes = cred_bundle.to_key_store_value().unwrap();
        let key_bytes = key_bundle.to_key_store_value().unwrap();

        let file_name = format!("../test_data/device-{}.bundles", identity);
        println!("Writing bundles to {}...", file_name);
        let mut file = File::create(&file_name).unwrap();
        file.write_all(&cred_bytes).unwrap();
        file.write(b"\n").unwrap();
        file.write_all(&key_bytes).unwrap();
        file.write(b"\n").unwrap();
        file.flush().unwrap();
    }
}
