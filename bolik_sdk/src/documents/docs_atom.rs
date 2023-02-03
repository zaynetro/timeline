use anyhow::Result;
use bolik_migrations::rusqlite::{params, OptionalExtension, Params, Row};
use bolik_proto::sync::{doc_payload::DocSchema, request, SecretAlgorithm};
use chrono::{DateTime, Days, Utc};
use prost::Message;
use uuid::Uuid;

use crate::{
    db::{StringListReadColumn, StringListWriteColumn},
    device::DeviceAtom,
    registry::{WithBackend, WithDeviceAtom, WithTxn},
    secrets::{build_accounts_hash, generate_key},
};

use super::{save, DbDocRow, DbDocRowMeta, DocSecretRow, ALL_LABEL_ID};

pub trait DocsCtx<'a>: WithTxn<'a> {}
impl<'a, T> DocsCtx<'a> for T where T: WithTxn<'a> {}

#[derive(Clone)]
pub struct DocsAtom {
    client_id: yrs::block::ClientID,
    device_id: String,
}

impl DocsAtom {
    pub fn new(device: &DeviceAtom) -> Self {
        Self {
            client_id: device.yrs_client_id,
            device_id: device.id.clone(),
        }
    }

    /// Find a document by ID. Never returns a deleted document
    pub fn find<'a>(&self, ctx: &impl DocsCtx<'a>, doc_id: &str) -> Result<Option<DbDocRow>> {
        self.query_row(
            ctx,
            r#"
SELECT id, author_device_id, counter, data, acl_data, created_at, edited_at, schema
  FROM documents
 WHERE id = ?"#,
            [doc_id],
        )
    }

    /// Find first document after offset
    pub fn find_first<'a>(
        &self,
        ctx: &impl DocsCtx<'a>,
        schema: DocSchema,
        offset: u64,
    ) -> Result<Option<DbDocRow>> {
        let not_deleted = format!(r#""{}""#, ALL_LABEL_ID);
        self.query_row(
            ctx,
            r#"
SELECT d.id, author_device_id, counter, data, acl_data, created_at, edited_at, schema
  FROM documents d
  JOIN card_index i ON d.id = i.id
 WHERE schema = ?1 AND i.label_ids MATCH ?2
 ORDER BY created_at DESC
 LIMIT ?, 1"#,
            params![schema as i32, not_deleted, offset],
        )
    }

    /// Find first locally modified documents with clock higher than provided or zero counter.
    pub fn find_local_after<'a>(
        &self,
        ctx: &impl DocsCtx<'a>,
        counter: u64,
    ) -> Result<Option<DbDocRow>> {
        self.query_row(
            ctx,
            r#"
SELECT id, author_device_id, counter, data, acl_data, created_at, edited_at, schema
  FROM documents
 WHERE author_device_id = ?1 AND counter > ?2
 ORDER BY counter
 LIMIT 1"#,
            params![self.device_id, counter],
        )
    }

    pub fn save<'a>(&self, ctx: &impl DocsCtx<'a>, row: &DbDocRow) -> Result<()> {
        save(ctx.txn(), row)
    }

    pub fn queue_doc_push<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        message: &request::DocMessage,
    ) -> Result<()> {
        let data = message.encode_to_vec();
        ctx.txn()
            .execute("INSERT INTO push_docs_queue (message) VALUES (?)", [&data])?;
        Ok(())
    }

    /// Add new entry to deleted docs queue table.
    pub fn add_to_deleted_queue<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDeviceAtom + WithBackend),
        acc_id: &str,
        doc_id: &str,
        orig_created_at: DateTime<Utc>,
        deleted_at: DateTime<Utc>,
    ) -> Result<()> {
        let deleted_at_sec = deleted_at.timestamp();
        let payload = format!("{},{}", doc_id, deleted_at_sec);
        let payload_signature = ctx.device().sign(ctx, payload.as_bytes())?;
        let message = request::DocMessage {
            id: doc_id.into(),
            to_account_ids: vec![acc_id.into()],
            current_clock: None,
            counter: 0,
            created_at_sec: orig_created_at.timestamp(),
            payload_signature,
            body: Some(request::doc_message::Body::Deleted(
                request::doc_message::DeletionBody { deleted_at_sec },
            )),
        };

        self.queue_doc_push(ctx, &message)
    }

    pub fn find_queued_doc<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDeviceAtom),
    ) -> Result<Option<(u32, request::DocMessage)>> {
        let row = ctx
            .txn()
            .query_row(
                "SELECT rowid, message FROM push_docs_queue LIMIT 1",
                [],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()?;
        if let Some((rowid, bytes)) = row {
            let mut message = request::DocMessage::decode(bytes.as_slice())?;
            message.counter = ctx.device().increment_clock(ctx)?;
            Ok(Some((rowid, message)))
        } else {
            Ok(None)
        }
    }

    pub fn remove_queued_doc<'a>(&self, ctx: &impl WithTxn<'a>, rowid: u32) -> Result<()> {
        ctx.txn()
            .execute("DELETE FROM push_docs_queue WHERE rowid = ?", [rowid])?;
        Ok(())
    }

    /// List all docs filtered by schema
    pub fn list_by_schema<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        schema: DocSchema,
    ) -> Result<Vec<DbDocRow>> {
        let mut stmt = ctx.txn().prepare(
            r#"
    SELECT id, author_device_id, counter, data, acl_data, created_at, edited_at, schema
      FROM documents
     WHERE schema = ?"#,
        )?;
        let mut rows = stmt.query([schema as i32])?;
        let mut docs = vec![];
        while let Some(row) = rows.next()? {
            let doc = self.read_row(row)?;
            docs.push(doc);
        }
        Ok(docs)
    }

    fn query_row<'a, P>(
        &self,
        ctx: &impl WithTxn<'a>,
        query: &str,
        params: P,
    ) -> Result<Option<DbDocRow>>
    where
        P: Params,
    {
        let mut stmt = ctx.txn().prepare(query)?;
        let mut rows = stmt.query(params)?;

        if let Some(row) = rows.next()? {
            let read = self.read_row(row)?;
            Ok(Some(read))
        } else {
            Ok(None)
        }
    }

    fn read_row(&self, row: &Row) -> Result<DbDocRow> {
        let data: Vec<u8> = row.get(3)?;
        let acl_data: Vec<u8> = row.get(4)?;

        let meta = DbDocRowMeta {
            id: row.get(0)?,
            author_device_id: row.get(1)?,
            counter: row.get(2)?,
            created_at: row.get(5)?,
            edited_at: row.get(6)?,
            schema: row.get(7)?,
        };

        let doc = super::build_yrs_doc(self.client_id, &data)?;
        let acl = super::build_yrs_doc(self.client_id, &acl_data)?;

        Ok(DbDocRow {
            meta,
            yrs: doc,
            acl,
        })
    }

    /// Save new doc secret.
    pub fn save_secret<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        mut row: DocSecretRow,
    ) -> Result<DocSecretRow> {
        let nonce_ciphertext = ctx.db_cipher().encrypt(&row.key)?;
        let accounts_hash = build_accounts_hash(&mut row.account_ids);
        let query = r#"
INSERT INTO doc_secrets (id, encrypted_secret, accounts_hash, account_ids, doc_id, algorithm, created_at, obsolete_at)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT (id) DO NOTHING"#;

        ctx.txn().execute(
            &query,
            params![
                row.id,
                nonce_ciphertext,
                accounts_hash,
                StringListWriteColumn(&row.account_ids),
                row.doc_id,
                row.algorithm,
                row.created_at,
                row.obsolete_at,
            ],
        )?;
        Ok(row)
    }

    /// List all known doc secrets.
    pub fn list_secrets<'a>(&self, ctx: &impl WithTxn<'a>) -> Result<Vec<DocSecretRow>> {
        let query = r#"
SELECT id, encrypted_secret, account_ids, doc_id, algorithm, created_at, obsolete_at
  FROM doc_secrets"#;
        let mut stmt = ctx.txn().prepare(query)?;
        let mut rows = stmt.query([])?;
        let mut secrets = vec![];

        while let Some(row) = rows.next()? {
            let secret = self.read_secret_row(ctx, row)?;
            secrets.push(secret);
        }

        Ok(secrets)
    }

    /// Find a secret or create if not found. Returns secret row and a boolean indiciating if secret was just created.
    pub fn get_secret_row_for_accounts<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        account_ids: &mut [String],
    ) -> Result<(DocSecretRow, bool)> {
        let accounts_hash = build_accounts_hash(account_ids);
        let query = r#"
SELECT id, encrypted_secret, account_ids, doc_id, algorithm, created_at, obsolete_at
  FROM doc_secrets
 WHERE accounts_hash = ?1 AND obsolete_at > ?2
 ORDER BY created_at
 LIMIT 1"#;
        let now = Utc::now();
        let mut stmt = ctx.txn().prepare(query)?;
        let mut rows = stmt.query(params![accounts_hash, now])?;

        if let Some(row) = rows.next()? {
            let secret = self.read_secret_row(ctx, row)?;
            Ok((secret, false))
        } else {
            let secret = self.create_secret(ctx, None, account_ids)?;
            Ok((secret, true))
        }
    }

    fn create_secret<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        doc_id: Option<String>,
        account_ids: &[String],
    ) -> Result<DocSecretRow> {
        let key = generate_key();
        let now = Utc::now();

        let secret = DocSecretRow {
            id: Uuid::new_v4().to_string(),
            key: key.to_vec(),
            account_ids: account_ids.to_vec(),
            doc_id,
            algorithm: SecretAlgorithm::ChaCha20Poly1305.into(),
            created_at: now,
            obsolete_at: now.checked_add_days(Days::new(30)).unwrap_or(now),
        };
        self.save_secret(ctx, secret)
    }

    pub fn find_secret_by_id<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        secret_id: &str,
    ) -> Result<Option<DocSecretRow>> {
        let query = r#"
SELECT id, encrypted_secret, account_ids, doc_id, algorithm, created_at, obsolete_at
  FROM doc_secrets
 WHERE id = ?"#;
        let mut stmt = ctx.txn().prepare(query)?;
        let mut rows = stmt.query([secret_id])?;

        if let Some(row) = rows.next()? {
            let secret = self.read_secret_row(ctx, row)?;
            Ok(Some(secret))
        } else {
            Ok(None)
        }
    }

    /// Mark all non-obsolete secrets that are shared with given account id as obsolete.
    pub fn mark_secrets_obsolete<'a>(
        &self,
        ctx: &impl WithTxn<'a>,
        account_id: &str,
    ) -> Result<()> {
        let now = Utc::now();
        ctx.txn().execute(
            r#"
UPDATE doc_secrets
   SET obsolete_at = ?1
 WHERE id IN (
              SELECT DISTINCT s.id
                FROM doc_secrets s, json_each(account_ids)
               WHERE json_each.value = ?2 AND obsolete_at > ?1
             )"#,
            params![now, account_id],
        )?;
        Ok(())
    }

    fn read_secret_row<'a>(&self, ctx: &impl WithTxn<'a>, row: &Row) -> Result<DocSecretRow> {
        let nonce_ciphertext: Vec<u8> = row.get(1)?;
        let bytes = ctx.db_cipher().decrypt(&nonce_ciphertext)?;
        Ok(DocSecretRow {
            id: row.get(0)?,
            key: bytes,
            account_ids: row.get::<_, StringListReadColumn>(2)?.0,
            doc_id: row.get(3)?,
            algorithm: row.get(4)?,
            created_at: row.get(5)?,
            obsolete_at: row.get(6)?,
        })
    }

    /// Remove external doc from local database.
    pub fn remove_external<'a>(&self, ctx: &impl WithTxn<'a>, id: &str) -> Result<()> {
        ctx.txn()
            .execute("DELETE FROM documents WHERE id = ?", [id])?;
        Ok(())
    }
}
