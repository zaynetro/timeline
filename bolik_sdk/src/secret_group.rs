use std::collections::HashMap;

use crate::{client::HttpClient, device::get_device_id};
use anyhow::Result;
use bolik_chain::SignatureChain;
use openmls::prelude::{Credential, KeyPackageRef, MlsGroup, OpenMlsCrypto};

mod group_atom;
pub use group_atom::{GroupApplyResult, SecretGroupAtom, SecretGroupCtx};

pub struct SecretGroup {
    pub mls: MlsGroup,
    pub chain: SignatureChain,
}

impl SecretGroup {
    pub fn device_ids(&self) -> Result<HashMap<String, Credential>> {
        self.mls
            .members()
            .iter()
            .map(|m| get_device_id(m.credential()).map(|id| (id, m.credential().clone())))
            .into_iter()
            .collect()
    }

    pub fn id(&self) -> String {
        SecretGroupAtom::<HttpClient>::group_id_str(self.mls.group_id())
    }

    pub fn find_id_by_ref(
        &self,
        key_ref: KeyPackageRef,
        crypto: &impl OpenMlsCrypto,
    ) -> Option<String> {
        let expect_key_ref = Some(key_ref);
        self.mls
            .members()
            .iter()
            .find(|m| m.hash_ref(crypto).ok() == expect_key_ref)
            .and_then(|m| get_device_id(m.credential()).ok())
    }

    pub fn find_member_ref(
        &self,
        device_id: &str,
        crypto: &impl OpenMlsCrypto,
    ) -> Option<KeyPackageRef> {
        let expect_id = Some(device_id.to_string());
        self.mls
            .members()
            .iter()
            .find(|m| get_device_id(m.credential()).ok() == expect_id)
            .and_then(|m| m.hash_ref(crypto).ok())
    }

    pub fn status(&self) -> Result<SecretGroupStatus> {
        let ids = self.device_ids()?.into_iter().map(|(id, _)| id).collect();
        Ok(SecretGroupStatus {
            group_id: self.id(),
            authentication_secret: self.mls.authentication_secret().as_slice().to_vec(),
            devices: ids,
        })
    }
}

/// Status info about account's MlsGroup
pub struct SecretGroupStatus {
    pub group_id: String,
    /// MlsGroup::authentication_secret to assert that group states are the same
    pub authentication_secret: Vec<u8>,
    /// List of device ids that are group members
    pub devices: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::PathBuf};

    use anyhow::{anyhow, Result};
    use bolik_chain::{DeviceRemovedOp, SignatureChain};
    use bolik_migrations::rusqlite::{Connection, Transaction};
    use bolik_proto::sync::{request, response, AppMessage};
    use openmls::prelude::{
        CredentialBundle, CredentialType, KeyPackage, KeyPackageBundle, KeyPackageRef,
        MlsMessageIn, OpenMlsKeyStore, TlsDeserializeTrait, TlsSerializeTrait,
    };
    use openmls_rust_crypto::RustCrypto;
    use openmls_traits::OpenMlsCryptoProvider;

    use crate::{
        client::mock::MockClient,
        db::migrations,
        device::{get_device_id, DeviceAtom},
        generate_db_key,
        registry::{WithBackend, WithDeviceAtom, WithTxn},
        secret_group::{GroupApplyResult, SecretGroupAtom},
        secrets::{DbCipher, SqliteCryptoProvider, CIPHERSUITES, DEFAULT_CIPHERSUITE},
    };

    use super::SecretGroup;

    #[test]
    fn test_group_local_changes() {
        let (a, mut conn_a) = get_device_a();
        let (b, mut conn_b) = get_device_b();
        let (c, _conn_c) = get_device_c();

        let ctx_a = &a.ctx(&mut conn_a);
        let ctx_b = &b.ctx(&mut conn_b);
        let atom = SecretGroupAtom::new(MockClient::default());

        // A creates a group
        let mut group_a = atom.create(ctx_a).unwrap();

        // Add new members
        let message = atom
            .add(ctx_a, &mut group_a, vec![b.package()])
            .unwrap()
            .unwrap();
        compare_devs(&[&a, &b], &group_a).unwrap();
        assert!(message.chain.is_some());
        assert!(message.welcome.is_some());

        // Add existing members --> noop
        let message = atom
            .add(
                ctx_a,
                &mut group_a,
                vec![a.new_package(ctx_a), b.new_package(ctx_b)],
            )
            .unwrap();
        compare_devs(&[&a, &b], &group_a).unwrap();
        assert!(message.is_none());

        // Remove members
        let message = atom
            .remove(
                ctx_a,
                &mut group_a,
                vec![DeviceRemovedOp {
                    key_ref: b.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap()
            .unwrap();
        compare_devs(&[&a], &group_a).unwrap();
        assert!(message.chain.is_some());
        assert!(message.welcome.is_none());

        // Remove unknown members --> noop
        let message = atom
            .remove(
                ctx_a,
                &mut group_a,
                vec![
                    DeviceRemovedOp {
                        key_ref: b.key_ref(),
                        last_counter: 3,
                    },
                    DeviceRemovedOp {
                        key_ref: c.key_ref(),
                        last_counter: 4,
                    },
                ],
            )
            .unwrap();
        compare_devs(&[&a], &group_a).unwrap();
        assert!(message.is_none());
    }

    #[test]
    fn test_group_follows_local_chain() {
        // A adds C, while B adds C:
        //
        //                  ┌─ add C
        //   add A ─ add B ─┤
        //                  └─ add D
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ add C ─ add D

        let (a, mut conn_a) = get_device_a();
        let (b, mut conn_b) = get_device_b();
        let (c, mut conn_c) = get_device_c();
        let (d, mut conn_d) = get_device_d();

        let ctx_a = &a.ctx(&mut conn_a);
        let ctx_b = &b.ctx(&mut conn_b);
        let ctx_c = &c.ctx(&mut conn_c);
        let ctx_d = &d.ctx(&mut conn_d);
        let atom = SecretGroupAtom::new(MockClient::default());

        // A creates a group
        let mut group_a = atom.create(ctx_a).unwrap();

        // A adds B
        let message = atom
            .add(ctx_a, &mut group_a, vec![b.package()])
            .unwrap()
            .unwrap();

        {
            // Verify chain_hash on sent commit message
            let msg_in = MlsMessageIn::tls_deserialize(&mut message.mls.as_slice()).unwrap();
            let chain = SignatureChain::decode(message.chain.clone().unwrap()).unwrap();
            assert_eq!(chain.head(), group_a.chain.head());
            assert!(msg_in.is_handshake_message());

            // Verify chain_hash on non-commit message
            let msg = atom
                .encrypt_message(ctx_a, &mut group_a, &AppMessage { value: None })
                .unwrap();
            let msg_in = MlsMessageIn::tls_deserialize(&mut msg.mls.as_slice()).unwrap();
            assert_eq!(msg.chain_hash, group_a.chain.head());
            assert!(!msg_in.is_handshake_message());
        }

        // B joins group
        let mut group_b = atom
            .join(
                ctx_b,
                response::SecretGroupWelcome {
                    welcome: message.welcome.unwrap(),
                    chain: message.chain,
                },
            )
            .unwrap()
            .unwrap();

        // Now we have two MLS groups in the same state and with same members.
        assert_eq!(
            group_a.mls.authentication_secret().as_slice(),
            group_b.mls.authentication_secret().as_slice()
        );
        compare_devs(&[&a, &b], &group_a).unwrap();
        compare_devs(&[&a, &b], &group_b).unwrap();

        // A adds C
        let message_ac = atom
            .add(ctx_a, &mut group_a, vec![c.package()])
            .unwrap()
            .unwrap();
        assert_eq!(2, group_a.mls.epoch().as_u64());
        assert_eq!(2, group_a.chain.epoch());

        // B adds D
        let message_bd = atom
            .add(ctx_b, &mut group_b, vec![d.package()])
            .unwrap()
            .unwrap();
        assert_eq!(2, group_b.mls.epoch().as_u64());
        assert_eq!(2, group_b.chain.epoch());

        // C joins group
        let _group_c = atom
            .join(
                ctx_c,
                response::SecretGroupWelcome {
                    welcome: message_ac.welcome.clone().unwrap(),
                    chain: message_ac.chain.clone(),
                },
            )
            .unwrap()
            .unwrap();

        // D joins group
        let _group_d = atom
            .join(
                ctx_d,
                response::SecretGroupWelcome {
                    welcome: message_bd.welcome.clone().unwrap(),
                    chain: message_bd.chain.clone(),
                },
            )
            .unwrap()
            .unwrap();

        // B integrates A's changes (switches to remote chain and discards local changes)
        let res_b = atom.apply(ctx_b, to_group_res(message_ac.clone())).unwrap();
        let (group_b, out) = expect_commit(res_b).unwrap();
        assert_eq!(out.len(), 0);
        compare_devs(&[&a, &b, &c], &group_b).unwrap();

        // A integrates B's changes
        let res_a = atom.apply(ctx_a, to_group_res(message_bd.clone())).unwrap();
        let (group_a, out) = expect_commit(res_a).unwrap();
        assert_eq!(out.len(), 1);
        let message_a = out[0].clone();

        assert_eq!(3, group_a.mls.epoch().as_u64());
        assert_eq!(3, group_a.chain.epoch());
        compare_devs(&[&a, &b, &c, &d], &group_a).unwrap();

        // B integrates A's changes
        let res_b = atom.apply(ctx_b, to_group_res(message_a.clone())).unwrap();
        let (group_b, out) = expect_commit(res_b).unwrap();
        assert_eq!(3, group_b.mls.epoch().as_u64());
        assert_eq!(out.len(), 0);

        // C integrates A's changes
        let res_c = atom.apply(ctx_c, to_group_res(message_a.clone())).unwrap();
        let (group_c, out) = expect_commit(res_c).unwrap();
        assert_eq!(3, group_c.mls.epoch().as_u64());
        assert_eq!(out.len(), 0);

        // D integrates A's changes (by rejoining the group)
        let group_d = atom
            .join(
                ctx_d,
                response::SecretGroupWelcome {
                    welcome: message_a.welcome.unwrap(),
                    chain: message_a.chain,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(3, group_d.mls.epoch().as_u64());
        assert_eq!(out.len(), 0);

        // Now we have four MLS groups in the same state and with same members.
        assert_eq!(
            group_a.mls.authentication_secret().as_slice(),
            group_b.mls.authentication_secret().as_slice()
        );
        assert_eq!(
            group_c.mls.authentication_secret().as_slice(),
            group_d.mls.authentication_secret().as_slice()
        );
        assert_eq!(
            group_a.mls.authentication_secret().as_slice(),
            group_d.mls.authentication_secret().as_slice()
        );

        assert_eq!(group_a.chain.head(), group_b.chain.head());
        assert_eq!(group_c.chain.head(), group_d.chain.head());
        assert_eq!(group_a.chain.head(), group_d.chain.head());

        compare_devs(&[&a, &b, &c, &d], &group_a).unwrap();
        compare_devs(&[&a, &b, &c, &d], &group_b).unwrap();
        compare_devs(&[&a, &b, &c, &d], &group_c).unwrap();
        compare_devs(&[&a, &b, &c, &d], &group_d).unwrap();
    }

    #[test]
    fn test_group_can_decrypt_messages() {
        // Verify can decrypt app messages from previous epoch
        // Verify can decrypt app messages from current epoch
    }

    #[test]
    fn test_group_follows_local_chain_multiple_blocks() {
        // A adds C, while B adds C:
        //
        //                  ┌─ add C ─ add E
        //   add A ─ add B ─┤
        //                  └─ add D
        //
        // We expect chain to be:
        //
        //   add A ─ add B ─ add C ─ add E ─ add D
    }

    #[test]
    fn test_group_remove_member() {
        // A adds D, while B removes C:
        //
        //                     ┌─ add D
        //   add A ─ add B, C ─┤
        //                     └─ rm C
        //
        // We expect chain to be:
        //
        //   add A ─ add B, C ─ add D ─ rm C

        let (a, mut conn_a) = get_device_a();
        let (b, mut conn_b) = get_device_b();
        let (c, mut conn_c) = get_device_c();
        let (d, mut conn_d) = get_device_d();

        let ctx_a = &a.ctx(&mut conn_a);
        let ctx_b = &b.ctx(&mut conn_b);
        let ctx_c = &c.ctx(&mut conn_c);
        let ctx_d = &d.ctx(&mut conn_d);
        let atom = SecretGroupAtom::new(MockClient::default());

        // A creates a group
        let mut group_a = atom.create(ctx_a).unwrap();

        // A adds B, C
        let message = atom
            .add(ctx_a, &mut group_a, vec![b.package(), c.package()])
            .unwrap()
            .unwrap();

        // B joins group
        let mut group_b = atom
            .join(
                ctx_b,
                response::SecretGroupWelcome {
                    welcome: message.welcome.clone().unwrap(),
                    chain: message.chain.clone(),
                },
            )
            .unwrap()
            .unwrap();

        // C joins group
        let _group_c = atom
            .join(
                ctx_c,
                response::SecretGroupWelcome {
                    welcome: message.welcome.unwrap(),
                    chain: message.chain,
                },
            )
            .unwrap()
            .unwrap();

        // A adds D
        let message_ad = atom
            .add(ctx_a, &mut group_a, vec![d.package()])
            .unwrap()
            .unwrap();

        // B removes C
        let message_bc = atom
            .remove(
                ctx_b,
                &mut group_b,
                vec![DeviceRemovedOp {
                    key_ref: c.key_ref(),
                    last_counter: 1,
                }],
            )
            .unwrap()
            .unwrap();

        // D joins group
        let _group_d = atom
            .join(
                ctx_d,
                response::SecretGroupWelcome {
                    welcome: message_ad.welcome.clone().unwrap(),
                    chain: message_ad.chain.clone(),
                },
            )
            .unwrap()
            .unwrap();

        // C integrates B's changes
        let res_c = atom.apply(ctx_c, to_group_res(message_bc.clone())).unwrap();
        let (group_c, _out) = expect_commit(res_c).unwrap();
        // Verify C no longer in the group
        assert!(!group_c.mls.is_active());

        // B integrates A's changes
        let res_b = atom.apply(ctx_b, to_group_res(message_ad.clone())).unwrap();
        let (group_b, out) = expect_commit(res_b).unwrap();
        assert_eq!(out.len(), 0);
        compare_devs(&[&a, &b, &c, &d], &group_b).unwrap();

        // C integrates A's changes
        let res_c = atom.apply(ctx_c, to_group_res(message_ad.clone())).unwrap();
        let (group_c, _out) = expect_commit(res_c).unwrap();
        // C is in the group again
        compare_devs(&[&a, &b, &c, &d], &group_c).unwrap();

        // A integrates B's changes
        let res_a = atom.apply(ctx_a, to_group_res(message_bc.clone())).unwrap();
        let (group_a, out) = expect_commit(res_a).unwrap();
        assert_eq!(out.len(), 1);
        let message_a = out[0].clone();
        compare_devs(&[&a, &b, &d], &group_a).unwrap();

        // B integrates A's changes
        let res_b = atom.apply(ctx_b, to_group_res(message_a.clone())).unwrap();
        let (group_b, _out) = expect_commit(res_b).unwrap();

        // C integrates A's changes
        let res_c = atom.apply(ctx_c, to_group_res(message_a.clone())).unwrap();
        let (group_c, _out) = expect_commit(res_c).unwrap();

        // D integrates A's changes
        let res_d = atom.apply(ctx_d, to_group_res(message_a.clone())).unwrap();
        let (group_d, _out) = expect_commit(res_d).unwrap();

        // Now we have three MLS groups in the same state and with same members.
        assert_eq!(
            group_a.mls.authentication_secret().as_slice(),
            group_b.mls.authentication_secret().as_slice()
        );
        assert_eq!(
            group_a.mls.authentication_secret().as_slice(),
            group_d.mls.authentication_secret().as_slice()
        );

        assert_eq!(group_a.chain.head(), group_b.chain.head());
        assert_eq!(group_a.chain.head(), group_d.chain.head());

        compare_devs(&[&a, &b, &d], &group_a).unwrap();
        compare_devs(&[&a, &b, &d], &group_b).unwrap();
        compare_devs(&[&a, &b, &d], &group_d).unwrap();

        // C's group is different
        assert_ne!(
            group_a.mls.authentication_secret().as_slice(),
            group_c.mls.authentication_secret().as_slice()
        );
        assert!(!group_c.mls.is_active());
    }

    #[test]
    fn test_group_key_package_update() {
        let (a, mut conn_a) = get_device_a();
        let (b, mut conn_b) = get_device_b();

        let ctx_a = &a.ctx(&mut conn_a);
        let ctx_b = &b.ctx(&mut conn_b);
        let atom = SecretGroupAtom::new(MockClient::default());

        // A creates a group
        let mut group_a = atom.create(ctx_a).unwrap();

        // A adds B
        let message = atom
            .add(ctx_a, &mut group_a, vec![b.package()])
            .unwrap()
            .unwrap();

        // B joins group
        let mut group_b = atom
            .join(
                ctx_b,
                response::SecretGroupWelcome {
                    welcome: message.welcome.unwrap(),
                    chain: message.chain,
                },
            )
            .unwrap()
            .unwrap();

        // B updates key package
        let message_b = atom
            .self_update(ctx_b, &mut group_b, b.key_bundle_alt.clone().unwrap())
            .unwrap()
            .unwrap();

        // A merges changes
        let res_a = atom.apply(ctx_a, to_group_res(message_b)).unwrap();
        let (group_a, _out) = expect_commit(res_a).unwrap();

        // Verify groups and chains
        assert_eq!(2, group_a.chain.epoch());
        assert_eq!(2, group_a.mls.epoch().as_u64());
        assert_eq!(2, group_b.chain.epoch());
        assert_eq!(2, group_b.mls.epoch().as_u64());

        let crypto = &RustCrypto::default();
        let chain_members = group_a.chain.members(crypto).unwrap();
        assert!(chain_members.find_by_ref(&b.key_ref()).is_none());
        assert!(chain_members
            .find_by_ref(&b.key_ref_alt().unwrap())
            .is_some());
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
        db_cipher: DbCipher,
        id: String,
        bundle: CredentialBundle,
        key_bundle: KeyPackageBundle,
        key_bundle_alt: Option<KeyPackageBundle>,
        crypto: RustCrypto,
    }

    impl TestDevice {
        fn random(identity: &str) -> Result<(Self, Connection)> {
            let (conn, db_cipher) = db_conn()?;
            let crypto = RustCrypto::default();
            let backend = &SqliteCryptoProvider::new(&db_cipher, &conn, &crypto);
            let bundle = CredentialBundle::new(
                identity.as_bytes().to_vec(),
                CredentialType::Basic,
                DEFAULT_CIPHERSUITE.signature_algorithm(),
                backend,
            )
            .unwrap();
            let id = get_device_id(bundle.credential())?;
            let credential_id_bytes: Vec<u8> = bundle
                .credential()
                .signature_key()
                .tls_serialize_detached()?;
            backend
                .key_store()
                .store(&credential_id_bytes, &bundle)
                .map_err(|err| anyhow!("{:?}", err))?;

            let key_bundle =
                KeyPackageBundle::new(&CIPHERSUITES, &bundle, backend, vec![]).unwrap();
            let key_ref = key_bundle.key_package().hash_ref(backend.crypto())?;
            backend
                .key_store()
                .store(key_ref.value(), &key_bundle)
                .map_err(|err| anyhow!("{:?}", err))?;

            let key_bundle_alt =
                KeyPackageBundle::new(&CIPHERSUITES, &bundle, backend, vec![]).unwrap();
            let key_ref_alt = key_bundle_alt.key_package().hash_ref(backend.crypto())?;
            backend
                .key_store()
                .store(key_ref_alt.value(), &key_bundle_alt)
                .map_err(|err| anyhow!("{:?}", err))?;

            Ok((
                Self {
                    bundle,
                    id,
                    db_cipher,
                    key_bundle,
                    key_bundle_alt: Some(key_bundle_alt),
                    crypto,
                },
                conn,
            ))
        }

        fn package(&self) -> KeyPackage {
            self.key_bundle.clone().into_parts().0
        }

        fn new_package(&self, ctx: &TestTxnCtx) -> KeyPackage {
            let key_bundle =
                KeyPackageBundle::new(&CIPHERSUITES, &self.bundle, &ctx.backend(), vec![]).unwrap();
            key_bundle.into_parts().0
        }

        fn key_ref(&self) -> KeyPackageRef {
            self.key_bundle
                .key_package()
                .hash_ref(&RustCrypto::default())
                .unwrap()
        }

        fn key_ref_alt(&self) -> Option<KeyPackageRef> {
            self.key_bundle_alt
                .clone()
                .map(|b| b.key_package().hash_ref(&RustCrypto::default()).unwrap())
        }

        fn ctx<'a>(&'a self, conn: &'a mut Connection) -> TestTxnCtx<'a> {
            let device_atom = DeviceAtom {
                id: self.id.clone(),
                name: "dev".into(),
                yrs_client_id: 1,
                blobs_dir: PathBuf::new(),
            };
            let txn = conn.transaction().unwrap();
            TestTxnCtx {
                device_atom,
                device: &self,
                txn,
            }
        }
    }

    struct TestTxnCtx<'a> {
        device_atom: DeviceAtom,
        device: &'a TestDevice,
        txn: Transaction<'a>,
    }

    impl<'a> WithDeviceAtom for TestTxnCtx<'a> {
        fn device(&self) -> &DeviceAtom {
            &self.device_atom
        }
    }

    impl<'a> WithTxn<'a> for TestTxnCtx<'a> {
        fn txn(&self) -> &Transaction<'a> {
            &self.txn
        }

        fn db_cipher(&self) -> &DbCipher {
            &self.device.db_cipher
        }
    }

    impl<'a> WithBackend for TestTxnCtx<'a> {
        fn backend(&self) -> SqliteCryptoProvider {
            SqliteCryptoProvider::new(&self.device.db_cipher, &self.txn, &self.device.crypto)
        }
    }

    fn get_device_a() -> (TestDevice, Connection) {
        TestDevice::random("A").unwrap()
    }

    fn get_device_b() -> (TestDevice, Connection) {
        TestDevice::random("B").unwrap()
    }

    fn get_device_c() -> (TestDevice, Connection) {
        TestDevice::random("C").unwrap()
    }

    fn get_device_d() -> (TestDevice, Connection) {
        TestDevice::random("D").unwrap()
    }

    fn compare_devs(expected: &[&TestDevice], group: &SecretGroup) -> Result<()> {
        let expected_ids: HashSet<&str> = expected.iter().map(|d| d.id.as_ref()).collect();
        let chain_members = group.chain.members(&RustCrypto::default())?;
        let chain_ids = chain_members.device_ids();
        if expected_ids == chain_ids {
            let mls_ids_owned = group.device_ids().unwrap();
            let mls_ids: HashSet<&str> = mls_ids_owned.iter().map(|(id, _)| id.as_ref()).collect();
            if expected_ids == mls_ids {
                Ok(())
            } else {
                Err(anyhow!(
                    "expected != mls.devices\nexpected(len={}): {:?},\nmls(len={}):    {:?}",
                    expected_ids.len(),
                    expected_ids,
                    mls_ids.len(),
                    mls_ids
                ))
            }
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

    fn expect_commit(
        res: GroupApplyResult,
    ) -> Result<(SecretGroup, Vec<request::SecretGroupCommit>)> {
        if let GroupApplyResult::Commit {
            group,
            messages_out,
            ..
        } = res
        {
            Ok((group, messages_out))
        } else {
            Err(anyhow!("Expected Commit"))
        }
    }

    fn to_group_res(msg: request::SecretGroupCommit) -> response::SecretGroupMessage {
        let chain = SignatureChain::decode(msg.chain.clone().unwrap()).unwrap();
        let mls = MlsMessageIn::tls_deserialize(&mut msg.mls.as_slice()).unwrap();
        let epoch = mls.epoch().as_u64();

        response::SecretGroupMessage {
            mls: msg.mls,
            chain_hash: chain.hash_at(epoch).unwrap().to_string(),
            chain: msg.chain,
        }
    }
}
