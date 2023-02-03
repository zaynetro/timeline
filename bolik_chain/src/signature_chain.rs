use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ops::Deref,
};

use bolik_proto::{
    prost::Message,
    sync::{self, signature_chain},
};
use multihash::{Blake3_256, Hasher};
use openmls::prelude::{
    CredentialBundle, KeyPackage, KeyPackageBundle, KeyPackageRef, OpenMlsCrypto, Signature,
    SignaturePrivateKey, TlsDeserializeTrait, TlsSerializeTrait,
};
use openmls_traits::OpenMlsCryptoProvider;
use thiserror::Error;

use crate::device::{get_device_id, id_from_key};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct DeviceOps {
    pub add: Vec<KeyPackage>,
    pub remove: Vec<DeviceRemovedOp>,
    pub update: Vec<KeyPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceRemovedOp {
    pub key_ref: KeyPackageRef,
    pub last_counter: u64,
}

#[derive(Clone)]
pub struct ChangeAuthor {
    pub bundle: CredentialBundle,
}

/// Chain of device operations (DAG, Causal Graph, Time DAG).
/// This chain records all device additions and removals in linear history.
/// When two chains are merged they both agree on the same history.
#[derive(Debug, Clone)]
pub struct SignatureChain {
    blocks: Vec<ChainBlock>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChainUsed {
    Local,
    Remote,
}

pub struct MergeAdvice {
    pub chain: SignatureChain,
    pub used: ChainUsed,
    /// Remote blocks that should be applied to the chain
    pub remote_blocks: Vec<ChainBlock>,
}

pub enum ApplyBlock {
    RemoteBlock(ChainBlock),
    LocalOps(DeviceOps),
}

impl MergeAdvice {
    fn local(chain: SignatureChain) -> Self {
        Self::local_with_blocks(chain, vec![])
    }

    fn local_with_blocks(chain: SignatureChain, remote_blocks: Vec<ChainBlock>) -> Self {
        Self {
            chain,
            used: ChainUsed::Local,
            remote_blocks,
        }
    }

    fn remote(chain: SignatureChain) -> Self {
        Self {
            chain,
            used: ChainUsed::Remote,
            remote_blocks: vec![],
        }
    }
}

impl SignatureChain {
    pub fn new(
        author: ChangeAuthor,
        key_bundle: KeyPackageBundle,
        account_ids: Vec<String>,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<Self, ChainError> {
        if !account_ids.is_empty() && account_ids.len() != 2 {
            return Err(ChainError::InvalidRoot);
        }

        let (credential, sig_key) = author.bundle.into_parts();
        let device_id = get_device_id(&credential).map_err(ChainError::ReadDeviceId)?;
        let root_block = Self::build_block(
            ChainBody {
                parent: None,
                authored_by: device_id,
                epoch: 0,
                ops: DeviceOps {
                    add: vec![key_bundle.into_parts().0],
                    remove: vec![],
                    update: vec![],
                },
                account_ids,
            },
            &sig_key,
            backend,
        )?;

        Ok(Self {
            blocks: vec![root_block],
        })
    }

    pub fn root(&self) -> &str {
        &self.blocks[0].hash
    }

    pub fn head(&self) -> &str {
        &self.blocks[self.blocks.len() - 1].hash
    }

    /// List of account ids this chain manages. For single account this returns empty list.
    pub fn account_ids(&self) -> &[String] {
        &self.blocks[0].body.account_ids
    }

    pub fn hash_at(&self, epoch: u64) -> Option<&str> {
        self.blocks
            .iter()
            .find(|b| b.body.epoch == epoch)
            .map(|b| b.hash.as_ref())
    }

    pub fn epoch(&self) -> u64 {
        self.blocks.last().map(|b| b.body.epoch).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn last_block(&self) -> Option<&ChainBlock> {
        self.blocks.last()
    }

    fn build_block(
        body: ChainBody,
        sig_key: &SignaturePrivateKey,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<ChainBlock, ChainError> {
        let hash = Self::get_body_hash(&body)?;
        let signature = sig_key
            .sign(backend, hash.as_bytes())
            .map_err(|_| ChainError::InvalidSignature(body.epoch))?;

        Ok(ChainBlock {
            hash,
            body,
            signature,
            body_version: 0,
        })
    }

    fn get_body_hash(body: &ChainBody) -> Result<String, ChainError> {
        // TODO: consider using a custom encoding struct for hashing (e.g use KeyPackageRef always, even additions)
        // TODO: or set a version and use a different protobuf struct per version
        let encoded = body.encode()?.encode_to_vec();
        let mut hasher = Blake3_256::default();
        hasher.update(encoded.as_slice());
        let hash_bytes = hasher.finalize();
        Ok(id_from_key(hash_bytes))
    }

    fn assert_block_hash(block: &ChainBlock) -> Result<(), ChainError> {
        let hash = Self::get_body_hash(&block.body)?;
        if hash == block.hash {
            Ok(())
        } else {
            Err(ChainError::HashMismatch(block.body.epoch))
        }
    }

    pub fn members(&self, crypto: &impl OpenMlsCrypto) -> Result<SignatureMembers, ChainError> {
        self.members_at(u64::MAX, crypto)
    }

    pub fn members_at(
        &self,
        epoch: u64,
        crypto: &impl OpenMlsCrypto,
    ) -> Result<SignatureMembers, ChainError> {
        let mut members = SignatureMembers::default();

        for block in &self.blocks {
            if block.body.epoch > epoch {
                break;
            }

            for key_package in &block.body.ops.add {
                members.insert(key_package, block.body.epoch, crypto)?;
            }

            for removed in &block.body.ops.remove {
                members.remove(&removed)?;
            }

            for key_package in &block.body.ops.update {
                members.insert(key_package, block.body.epoch, crypto)?;
            }
        }

        Ok(members)
    }

    pub fn append(
        &mut self,
        ops: DeviceOps,
        author: ChangeAuthor,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<(), ChainError> {
        let (credential, sig_key) = author.bundle.into_parts();
        let author_id = get_device_id(&credential).map_err(ChainError::ReadDeviceId)?;
        let block = Self::build_block(
            ChainBody {
                parent: Some(self.head().to_string()),
                authored_by: author_id,
                epoch: self.epoch() + 1,
                ops,
                account_ids: vec![],
            },
            &sig_key,
            backend,
        )?;

        self.blocks.push(block);
        Ok(())
    }

    pub fn verify(&self, backend: &impl OpenMlsCryptoProvider) -> Result<(), ChainError> {
        if self.blocks.is_empty() {
            return Err(ChainError::Empty);
        }

        let mut blocks_iter = self.blocks.iter();
        let root = blocks_iter.next().unwrap();

        if root.body.ops.add.len() != 1 {
            return Err(ChainError::InvalidRootOps);
        }

        if !root.body.account_ids.is_empty() && root.body.account_ids.len() != 2 {
            return Err(ChainError::InvalidRoot);
        }

        let mut members = SignatureMembers::default();

        let key_package = &root.body.ops.add[0];
        members.insert(key_package, root.body.epoch, backend.crypto())?;

        Self::assert_block_hash(root)?;
        key_package
            .credential()
            .verify(backend, root.hash.as_bytes(), &root.signature)
            .map_err(|_| ChainError::InvalidSignature(0))?;

        for block in blocks_iter {
            let author_member = members
                .find_by_id(&block.body.authored_by.deref())
                .ok_or(ChainError::NonMemberEdit)?;

            Self::assert_block_hash(block)?;
            author_member
                .package
                .credential()
                .verify(backend, block.hash.as_bytes(), &block.signature)
                .map_err(|_| ChainError::InvalidSignature(block.body.epoch))?;

            let ops = &block.body.ops;
            if ops.add.is_empty() && ops.remove.is_empty() && ops.update.is_empty() {
                return Err(ChainError::EmptyOps(block.body.epoch));
            }

            for key_package in &ops.add {
                members.insert(key_package, block.body.epoch, backend.crypto())?;
            }

            for removed in &ops.remove {
                members.remove(&removed)?;
            }

            for key_package in &ops.update {
                members.insert(key_package, block.body.epoch, backend.crypto())?;
            }
        }

        Ok(())
    }

    pub fn encode(&self) -> Result<sync::SignatureChain, ChainError> {
        self.encode_at_epoch(u64::MAX)
    }

    pub fn encode_bytes(&self) -> Result<Vec<u8>, ChainError> {
        Ok(self.encode()?.encode_to_vec())
    }

    /// Encode chain at epoch. This method skips all blocks that were added after the epoch.
    pub fn encode_at_epoch(&self, epoch: u64) -> Result<sync::SignatureChain, ChainError> {
        let mut encoded_blocks = Vec::with_capacity(self.blocks.len());
        for block in &self.blocks {
            if block.body.epoch > epoch {
                break;
            }

            encoded_blocks.push(signature_chain::ChainBlock {
                hash: block.hash.clone(),
                signature: block.signature.tls_serialize_detached()?,
                body: Some(block.body.encode()?),
                body_version: block.body_version,
            });
        }

        Ok(sync::SignatureChain {
            blocks: encoded_blocks,
        })
    }

    pub fn decode_bytes(bytes: &[u8]) -> Result<Self, ChainError> {
        Self::decode(sync::SignatureChain::decode(bytes)?)
    }

    pub fn decode(message: sync::SignatureChain) -> Result<Self, ChainError> {
        let mut blocks = Vec::with_capacity(message.blocks.len());
        for encoded_block in message.blocks.into_iter() {
            let encoded_body = encoded_block
                .body
                .ok_or(ChainError::DecodeMissingField("body"))?;
            let encoded_ops = encoded_body
                .ops
                .ok_or(ChainError::DecodeMissingField("ops"))?;

            let mut additions = Vec::with_capacity(encoded_ops.add_packages.len());
            let mut removals = Vec::with_capacity(encoded_ops.remove.len());
            let mut updates = Vec::with_capacity(encoded_ops.update_packages.len());

            for encoded_key_package in encoded_ops.add_packages {
                let key_package = KeyPackage::tls_deserialize(&mut encoded_key_package.as_slice())?;
                additions.push(key_package);
            }
            for removal in encoded_ops.remove {
                let key_ref = KeyPackageRef::tls_deserialize(&mut removal.key_ref.as_slice())?;
                removals.push(DeviceRemovedOp {
                    key_ref,
                    last_counter: removal.last_counter,
                });
            }
            for encoded_key_package in encoded_ops.update_packages {
                let key_package = KeyPackage::tls_deserialize(&mut encoded_key_package.as_slice())?;
                updates.push(key_package);
            }

            blocks.push(ChainBlock {
                hash: encoded_block.hash,
                signature: Signature::tls_deserialize(&mut encoded_block.signature.as_slice())?,
                body: ChainBody {
                    parent: encoded_body.parent,
                    authored_by: encoded_body.authored_by,
                    epoch: encoded_body.epoch,
                    ops: DeviceOps {
                        add: additions,
                        remove: removals,
                        update: updates,
                    },
                    account_ids: encoded_body.account_ids,
                },
                body_version: encoded_block.body_version,
            });
        }

        Ok(Self { blocks })
    }

    pub fn changes_since(&self, epoch: u64) -> impl Iterator<Item = &ChainBlock> {
        self.blocks.iter().filter(move |b| b.body.epoch > epoch)
    }

    pub fn last(&self) -> &ChainBlock {
        &self.blocks[self.blocks.len() - 1]
    }

    fn authored_iter<'a, 'b>(
        &'a self,
        crypto: &'b impl OpenMlsCrypto,
    ) -> impl Iterator<Item = Result<AuthoredBlock, ChainError>> + 'b
    where
        'a: 'b,
    {
        let empty_members = SignatureMembers::default();

        self.blocks.iter().scan(empty_members, |members, block| {
            for key_package in &block.body.ops.add {
                if let Err(err) = members.insert(key_package, block.body.epoch, crypto) {
                    return Some(Err(err.into()));
                }
            }

            for removed in &block.body.ops.remove {
                let _ = members.remove(&removed);
            }

            let author_added_at_epoch = members
                .find_by_id(&block.body.authored_by)
                .map(|m| m.added_at_epoch)
                .unwrap_or(u64::MAX);
            Some(Ok(AuthoredBlock {
                block,
                author_added_at_epoch,
            }))
        })
    }

    fn diff<'a, 'b>(
        &'a self,
        other: &'b Self,
        crypto: &impl OpenMlsCrypto,
    ) -> Result<(Vec<AuthoredBlock<'a>>, Vec<AuthoredBlock<'b>>), ChainError> {
        let mut self_iter = self.authored_iter(crypto);
        let mut other_iter = other.authored_iter(crypto);

        let mut self_changes = vec![];
        let mut other_changes = vec![];

        loop {
            match (self_iter.next(), other_iter.next()) {
                (Some(s), Some(o)) => {
                    let s = s?;
                    let o = o?;
                    if s.block.hash != o.block.hash {
                        self_changes.push(s);
                        other_changes.push(o);
                    }
                }
                (Some(s), None) => {
                    let s = s?;
                    self_changes.push(s);
                }
                (None, Some(o)) => {
                    let o = o?;
                    other_changes.push(o);
                }
                (None, None) => break Ok((self_changes, other_changes)),
            }
        }
    }

    pub fn prepare_merge(
        self,
        remote: Self,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<MergeAdvice, ChainError> {
        remote.verify(backend)?;

        if self == remote {
            // Chains are the same
            return Ok(MergeAdvice::local(self));
        }

        let (local_changes, remote_changes) = self.diff(&remote, backend.crypto())?;

        if local_changes.is_empty() {
            // Remote chain is continuation of ours
            return Ok(MergeAdvice::remote(remote));
        }

        if remote_changes.is_empty() {
            // Local chain is continuation of theirs
            return Ok(MergeAdvice::local(self));
        }

        // Resolve the conflict
        let local_next = &local_changes[0];
        let remote_next = &remote_changes[0];

        // Pick author which has been the longest in the account
        let local_wins = if local_next.author_added_at_epoch == remote_next.author_added_at_epoch {
            local_next.block.body.authored_by < remote_next.block.body.authored_by
        } else {
            local_next.author_added_at_epoch < remote_next.author_added_at_epoch
        };

        if local_wins {
            // Apply remote changes into local chain
            let remote_blocks = remote_changes
                .into_iter()
                .map(|c| c.block.clone())
                .collect();
            Ok(MergeAdvice::local_with_blocks(self, remote_blocks))
        } else {
            Ok(MergeAdvice::remote(remote))
        }
    }

    pub fn modify(
        &mut self,
        apply_block: ApplyBlock,
        author: ChangeAuthor,
        backend: &impl OpenMlsCryptoProvider,
    ) -> Result<bool, ChainError> {
        // We use ordered collections so that changes are reproducible (they generate the same hashes).
        let mut additions = BTreeMap::new();
        let mut removals = BTreeSet::new();
        let mut updates = BTreeMap::new();

        let members = self.members(backend.crypto())?;
        let ops = match apply_block {
            ApplyBlock::RemoteBlock(b) => {
                if members.find_by_id(&b.body.authored_by).is_none() {
                    return Err(ChainError::NonMemberEdit);
                }
                b.body.ops
            }
            ApplyBlock::LocalOps(ops) => ops,
        };

        if !ops.add.is_empty() && !ops.remove.is_empty() {
            return Err(ChainError::DifferentOps);
        }

        for key_package in ops.add {
            let key_ref = key_package.hash_ref(backend.crypto())?;
            let device_id =
                get_device_id(key_package.credential()).map_err(ChainError::ReadDeviceId)?;
            if members.find_by_id(&device_id).is_none() {
                // Keep only new members
                additions.insert(key_ref, key_package);
            }
        }

        for removed in ops.remove {
            if members.find_by_ref(&removed.key_ref).is_some() {
                // Keep only existing members
                removals.insert(removed);
            }
        }

        for key_package in ops.update {
            let key_ref = key_package.hash_ref(backend.crypto())?;
            let device_id =
                get_device_id(key_package.credential()).map_err(ChainError::ReadDeviceId)?;
            if members.find_by_id(&device_id).is_some() {
                // Keep only existing members
                updates.insert(key_ref, key_package);
            }
        }

        // Apply changes to the chain:
        // We apply additions separately from removals because current openmls lib doesn't support
        // additions and removals in the same commit.
        if !additions.is_empty() {
            let ops = DeviceOps {
                add: additions.into_iter().map(|(_, v)| v).collect(),
                remove: vec![],
                update: vec![],
            };
            self.append(ops, author.clone(), backend)?;
        } else if !removals.is_empty() {
            let ops = DeviceOps {
                add: vec![],
                remove: removals.into_iter().collect(),
                update: vec![],
            };
            self.append(ops, author.clone(), backend)?;
        } else if !updates.is_empty() {
            let ops = DeviceOps {
                add: vec![],
                remove: vec![],
                update: updates.into_iter().map(|(_, v)| v).collect(),
            };
            self.append(ops, author.clone(), backend)?;
        } else {
            return Ok(false);
        };

        Ok(true)
    }
}

impl PartialEq for SignatureChain {
    fn eq(&self, other: &Self) -> bool {
        self.blocks.last() == other.blocks.last()
    }
}

#[derive(Default)]
pub struct SignatureMembers<'a> {
    pub key_refs: HashMap<KeyPackageRef, SignatureMember<'a>>,
    pub device_ids: HashMap<String, SignatureMember<'a>>,
    pub removed: HashMap<String, ChainRemovedMember<'a>>,
}

pub struct SignatureMember<'a> {
    pub package: &'a KeyPackage,
    pub added_at_epoch: u64,
}

pub struct ChainRemovedMember<'a> {
    pub package: &'a KeyPackage,
    pub last_counter: u64,
}

impl<'a> SignatureMembers<'a> {
    pub fn insert(
        &mut self,
        key_package: &'a KeyPackage,
        added_at_epoch: u64,
        crypto: &impl OpenMlsCrypto,
    ) -> Result<(), ChainError> {
        let key_ref = key_package.hash_ref(crypto)?;
        let device_id =
            get_device_id(key_package.credential()).map_err(ChainError::ReadDeviceId)?;

        if let Some(member) = self.find_by_id(&device_id) {
            // Delete previous key ref
            let prev_key_ref = member.package.hash_ref(crypto)?;
            self.key_refs.remove(&prev_key_ref);
        }

        self.key_refs.insert(
            key_ref,
            SignatureMember {
                package: key_package,
                added_at_epoch,
            },
        );
        self.removed.remove(&device_id);
        self.device_ids.insert(
            device_id,
            SignatureMember {
                package: key_package,
                added_at_epoch,
            },
        );
        Ok(())
    }

    pub fn remove(&mut self, removed: &DeviceRemovedOp) -> Result<(), ChainError> {
        if let Some(member) = self.key_refs.remove(&removed.key_ref) {
            let device_id =
                get_device_id(member.package.credential()).map_err(ChainError::ReadDeviceId)?;
            if let Some(member) = self.device_ids.remove(&device_id) {
                self.removed.insert(
                    device_id,
                    ChainRemovedMember {
                        package: member.package,
                        last_counter: removed.last_counter,
                    },
                );
            }
        }
        Ok(())
    }

    pub fn find_by_id(&self, device_id: &str) -> Option<&SignatureMember> {
        self.device_ids.get(device_id)
    }

    pub fn find_by_ref(&self, key_ref: &KeyPackageRef) -> Option<&SignatureMember> {
        self.key_refs.get(key_ref)
    }

    pub fn device_ids(&self) -> HashSet<&str> {
        self.device_ids.keys().map(|id| id.as_ref()).collect()
    }

    pub fn len(&self) -> usize {
        self.device_ids.len()
    }
}

struct AuthoredBlock<'a> {
    block: &'a ChainBlock,
    author_added_at_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct ChainBlock {
    pub hash: String,
    signature: Signature,
    pub body: ChainBody,
    body_version: u32,
}

impl std::hash::Hash for ChainBlock {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for ChainBlock {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for ChainBlock {}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainBody {
    pub parent: Option<String>,
    authored_by: String,
    pub epoch: u64,
    pub ops: DeviceOps,
    pub account_ids: Vec<String>,
}

impl ChainBody {
    fn encode(&self) -> Result<signature_chain::ChainBody, ChainError> {
        let mut add_packages = Vec::with_capacity(self.ops.add.len());
        let mut removals = Vec::with_capacity(self.ops.remove.len());
        let mut update_packages = Vec::with_capacity(self.ops.update.len());

        for key_package in &self.ops.add {
            add_packages.push(key_package.tls_serialize_detached()?);
        }

        for removed in &self.ops.remove {
            removals.push(signature_chain::Removed {
                key_ref: removed.key_ref.tls_serialize_detached()?,
                last_counter: removed.last_counter,
            });
        }

        for key_package in &self.ops.update {
            update_packages.push(key_package.tls_serialize_detached()?);
        }

        Ok(signature_chain::ChainBody {
            parent: self.parent.clone(),
            authored_by: self.authored_by.clone(),
            epoch: self.epoch,
            ops: Some(signature_chain::DeviceOps {
                add_packages,
                remove: removals,
                update_packages,
            }),
            account_ids: self.account_ids.clone(),
        })
    }
}

#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Root block must specify zero or two account ids")]
    InvalidRoot,
    #[error("Root block must add single device")]
    InvalidRootOps,
    #[error("Signature (epoch={0})")]
    InvalidSignature(u64),
    #[error("Hash mismatch (epoch={0})")]
    HashMismatch(u64),
    #[error("Add and remove should be in separate operations")]
    DifferentOps,
    #[error("Empty SignatureChain")]
    Empty,
    #[error("Empty operations (epoch={0})")]
    EmptyOps(u64),
    #[error("Authoring device is not part of the chain")]
    NonMemberEdit,
    #[error("Missing field={0} when decoding")]
    DecodeMissingField(&'static str),
    #[error("Read device id: {0}")]
    ReadDeviceId(crate::device::DeviceError),
    #[error("Decode TLS error: {0}")]
    DecodeTls(#[from] tls_codec::Error),
    #[error("Decode protobuf error: {0}")]
    DecodeProtobuf(#[from] bolik_proto::prost::DecodeError),
    #[error("OpenMLS error: {0}")]
    OpenMls(#[from] openmls::error::LibraryError),
}
