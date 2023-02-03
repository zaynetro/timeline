use std::cell::RefCell;

use anyhow::Result;
use bolik_migrations::rusqlite::{Connection, Transaction};
use openmls_rust_crypto::RustCrypto;

use crate::{
    account::AccountAtom,
    blobs::BlobsAtom,
    client::Client,
    db::Db,
    device::DeviceAtom,
    documents::{DocsAtom, SyncDocsAtom},
    export::ExportAtom,
    import::ImportAtom,
    mailbox::MailboxAtom,
    output::OutputEvent,
    secret_group::SecretGroupAtom,
    secrets::{DbCipher, SqliteCryptoProvider},
    timeline::TimelineAtom,
};

#[derive(Default)]
struct CryptoProvider(RustCrypto);

impl Clone for CryptoProvider {
    fn clone(&self) -> Self {
        Self(RustCrypto::default())
    }
}

#[derive(Clone)]
pub struct Registry<C: Clone> {
    pub db: Db,
    pub device: DeviceAtom,
    pub docs: DocsAtom,
    pub sync_docs: SyncDocsAtom<C>,
    pub secret_group: SecretGroupAtom<C>,
    pub account: AccountAtom,
    pub mailbox: MailboxAtom<C>,
    pub timeline: TimelineAtom,
    pub export: ExportAtom,
    pub import: ImportAtom,
    pub blobs: BlobsAtom<C>,
    crypto: CryptoProvider,
    pub broadcast: tokio::sync::broadcast::Sender<OutputEvent>,
}

impl<C> Registry<C>
where
    C: Client,
{
    pub fn new(db: Db, device: DeviceAtom, client: C) -> Self {
        let (broadcaster, _) = tokio::sync::broadcast::channel(20);
        let docs = DocsAtom::new(&device);
        let sync_docs = SyncDocsAtom::new(client.clone());
        let account = AccountAtom::new();
        let secret_group = SecretGroupAtom::new(client.clone());
        let mailbox = MailboxAtom::new(client.clone());
        let timeline = TimelineAtom::new();
        let export = ExportAtom::new();
        let import = ImportAtom::new();
        let blobs = BlobsAtom::new(client);

        Self {
            db,
            device,
            docs,
            sync_docs,
            secret_group,
            account,
            mailbox,
            timeline,
            export,
            import,
            blobs,
            broadcast: broadcaster,
            crypto: CryptoProvider::default(),
        }
    }

    pub fn in_txn<T>(&self, f: impl FnOnce(&TxnCtx<C>, &Self) -> Result<T>) -> Result<T> {
        let mut conn = self.db.conn.lock().unwrap();
        let ctx = TxnCtx::new(&mut conn, self)?;
        let res = f(&ctx, &self)?;
        ctx.commit()?;
        Ok(res)
    }

    pub fn db_ctx(&self) -> DbCtx<C> {
        DbCtx::new(self)
    }
}

// Future (Alias type is not in stable yet):
// trait DeviceCtx<'a> = WithTxn<'a> + WithBackend;
//
// Now:
// trait DeviceCtx<'a>: WithTxn<'a> + WithBackend {}
// impl<'a, T> DeviceCtx<'a> for T where T: WithTxn<'a> + WithBackend {}
//
// Thanks to https://github.com/rust-lang/rust/issues/41517#issuecomment-1100644808

pub trait WithEvents {
    /// Queue the event. Duplicate events are skipped.
    fn queue_event(&self, event: OutputEvent);
}

pub trait WithDeviceAtom {
    fn device(&self) -> &DeviceAtom;
}

pub trait WithAccountAtom {
    fn account(&self) -> &AccountAtom;
}

pub trait WithDocsAtom {
    fn docs(&self) -> &DocsAtom;
}

pub trait WithSecretGroupAtom<C: Clone> {
    fn secret_group(&self) -> &SecretGroupAtom<C>;
}

pub trait WithMailboxAtom<C: Clone> {
    fn mailbox(&self) -> &MailboxAtom<C>;
}

pub trait WithTimelineAtom {
    fn timeline(&self) -> &TimelineAtom;
}

pub trait WithBlobsAtom<C: Clone> {
    fn blobs(&self) -> &BlobsAtom<C>;
}

pub trait WithTxn<'a> {
    fn txn(&self) -> &Transaction<'a>;
    fn db_cipher(&self) -> &DbCipher;
}

pub trait WithDb {
    fn db(&self) -> &Db;
}

pub trait WithBackend {
    fn backend(&self) -> SqliteCryptoProvider;
}

pub trait WithBackendConn<'a> {
    fn backend(&self, conn: &'a Connection) -> SqliteCryptoProvider<'a>;
}

pub trait WithBroadcast {
    fn broadcast(&self, event: OutputEvent);
}

pub struct DbCtx<'a, C: Clone> {
    registry: &'a Registry<C>,
}

impl<'a, C: Clone> DbCtx<'a, C> {
    pub fn new(registry: &'a Registry<C>) -> Self {
        Self { registry }
    }

    pub fn orig_in_txn<T>(&self, f: impl FnOnce(&TxnCtx<C>) -> Result<T>) -> Result<T> {
        let mut conn = self.registry.db.conn.lock().unwrap();
        let ctx = TxnCtx::new(&mut conn, &self.registry)?;
        let res = f(&ctx)?;
        ctx.commit()?;
        Ok(res)
    }
}

pub trait WithInTxn<C: Clone> {
    fn in_txn<T>(&self, f: impl FnOnce(&mut TxnCtx<C>) -> Result<T>) -> Result<T>;
}

impl<'a, C: Clone> WithInTxn<C> for DbCtx<'a, C> {
    fn in_txn<T>(&self, f: impl FnOnce(&mut TxnCtx<C>) -> Result<T>) -> Result<T> {
        let mut conn = self.registry.db.conn.lock().unwrap();
        let mut ctx = TxnCtx::new(&mut conn, &self.registry)?;
        let res = f(&mut ctx)?;
        ctx.commit()?;
        Ok(res)
    }
}

impl<'a, C: Clone> WithDb for DbCtx<'a, C> {
    fn db(&self) -> &Db {
        &self.registry.db
    }
}

impl<'a, C: Clone> WithBackendConn<'a> for DbCtx<'a, C> {
    fn backend(&self, conn: &'a Connection) -> SqliteCryptoProvider<'a> {
        SqliteCryptoProvider::new(&self.registry.db.db_cipher, conn, &self.registry.crypto.0)
    }
}

impl<'a, C: Clone> WithDeviceAtom for DbCtx<'a, C> {
    fn device(&self) -> &DeviceAtom {
        &self.registry.device
    }
}

impl<'a, C: Clone> WithMailboxAtom<C> for DbCtx<'a, C> {
    fn mailbox(&self) -> &MailboxAtom<C> {
        &self.registry.mailbox
    }
}

impl<'a, C: Clone> WithSecretGroupAtom<C> for DbCtx<'a, C> {
    fn secret_group(&self) -> &SecretGroupAtom<C> {
        &self.registry.secret_group
    }
}

impl<'a, C: Clone> WithBlobsAtom<C> for DbCtx<'a, C> {
    fn blobs(&self) -> &BlobsAtom<C> {
        &self.registry.blobs
    }
}

impl<'a, C: Clone> WithBroadcast for DbCtx<'a, C> {
    fn broadcast(&self, event: OutputEvent) {
        if let Err(err) = self.registry.broadcast.send(event) {
            tracing::warn!("Failed to broadcast OutputEvent: {}", err);
        }
    }
}

pub struct TxnCtx<'a, C: Clone> {
    registry: &'a Registry<C>,
    txn: Transaction<'a>,
    events: RefCell<Vec<OutputEvent>>,
}

impl<'a, C: Clone> TxnCtx<'a, C> {
    pub fn new(conn: &'a mut Connection, registry: &'a Registry<C>) -> Result<Self> {
        let txn = conn.transaction()?;
        Ok(Self {
            registry,
            txn,
            events: RefCell::default(),
        })
    }

    pub fn commit(self) -> Result<()> {
        self.txn.commit()?;

        for event in self.events.take() {
            if let Err(err) = self.registry.broadcast.send(event) {
                tracing::warn!("Failed to broadcast OutputEvent: {}", err);
            }
        }
        Ok(())
    }
}

impl<'a, C: Clone> WithTxn<'a> for TxnCtx<'a, C> {
    fn txn(&self) -> &Transaction<'a> {
        &self.txn
    }

    fn db_cipher(&self) -> &DbCipher {
        &self.registry.db.db_cipher
    }
}

impl<'a, C: Clone> WithBackend for TxnCtx<'a, C> {
    fn backend(&self) -> SqliteCryptoProvider {
        SqliteCryptoProvider::new(
            &self.registry.db.db_cipher,
            &self.txn,
            &self.registry.crypto.0,
        )
    }
}

impl<'a, C: Clone> WithDeviceAtom for TxnCtx<'a, C> {
    fn device(&self) -> &DeviceAtom {
        &self.registry.device
    }
}

impl<'a, C: Clone> WithAccountAtom for TxnCtx<'a, C> {
    fn account(&self) -> &AccountAtom {
        &self.registry.account
    }
}

impl<'a, C: Clone> WithDocsAtom for TxnCtx<'a, C> {
    fn docs(&self) -> &DocsAtom {
        &self.registry.docs
    }
}

impl<'a, C: Clone> WithSecretGroupAtom<C> for TxnCtx<'a, C> {
    fn secret_group(&self) -> &SecretGroupAtom<C> {
        &self.registry.secret_group
    }
}

impl<'a, C: Clone> WithTimelineAtom for TxnCtx<'a, C> {
    fn timeline(&self) -> &TimelineAtom {
        &self.registry.timeline
    }
}

impl<'a, C: Clone> WithEvents for TxnCtx<'a, C> {
    fn queue_event(&self, event: OutputEvent) {
        let mut events = self.events.borrow_mut();
        if !events.contains(&event) {
            events.push(event);
        }
    }
}

pub struct SetupTxnCtx<'a> {
    db: &'a Db,
    txn: Transaction<'a>,
    crypto: RustCrypto,
}

impl<'a> SetupTxnCtx<'a> {
    pub fn new(conn: &'a mut Connection, db: &'a Db) -> Result<Self> {
        let txn = conn.transaction()?;
        Ok(Self {
            db,
            txn,
            crypto: RustCrypto::default(),
        })
    }

    pub fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

impl<'a> WithTxn<'a> for SetupTxnCtx<'a> {
    fn txn(&self) -> &Transaction<'a> {
        &self.txn
    }

    fn db_cipher(&self) -> &DbCipher {
        &self.db.db_cipher
    }
}

impl<'a> WithBackend for SetupTxnCtx<'a> {
    fn backend(&self) -> SqliteCryptoProvider {
        SqliteCryptoProvider::new(&self.db.db_cipher, &self.txn, &self.crypto)
    }
}
