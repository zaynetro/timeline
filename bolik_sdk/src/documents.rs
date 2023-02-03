use anyhow::Result;
use bolik_migrations::rusqlite::{params, Connection};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{DateTime, Utc};
use yrs::{updates::decoder::Decode, ReadTxn, StateVector, Transact, Update};

mod docs_atom;
mod sync_docs_atom;
pub mod yrs_util;

pub use docs_atom::{DocsAtom, DocsCtx};
pub use sync_docs_atom::SyncDocsAtom;

/// Mark moved to bin cards
pub const BIN_LABEL_ID: &str = "bolik-bin";
/// A helper label to allow us to find not deleted cards.
pub(crate) const ALL_LABEL_ID: &str = "bolik-all";

pub struct DbDocRow {
    pub meta: DbDocRowMeta,
    pub yrs: yrs::Doc,
    pub acl: yrs::Doc,
}

#[derive(Debug, Clone)]
pub struct DbDocRowMeta {
    pub id: String,
    pub author_device_id: String,
    pub counter: u64,
    /// [bolik_proto::sync::doc_payload::DocSchema]
    pub schema: i32,
    pub created_at: DateTime<Utc>,
    pub edited_at: DateTime<Utc>,
}

pub fn merge_yrs_docs(source: &yrs::Doc, updates: &[u8]) -> Result<()> {
    let mut txn = source.transact_mut();
    let u = Update::decode_v2(updates)?;
    txn.apply_update(u);
    Ok(())
}

pub fn build_yrs_doc(yrs_client_id: yrs::block::ClientID, data: &[u8]) -> Result<yrs::Doc> {
    let doc = yrs::Doc::with_options(yrs::Options {
        client_id: yrs_client_id,
        offset_kind: yrs::OffsetKind::Utf32,
        ..Default::default()
    });
    merge_yrs_docs(&doc, data)?;
    Ok(doc)
}

pub fn encode_yrs_doc(doc: &yrs::Doc) -> Vec<u8> {
    let txn = doc.transact();
    txn.encode_state_as_update_v2(&StateVector::default())
}

fn save(conn: &Connection, row: &DbDocRow) -> Result<()> {
    let data = encode_yrs_doc(&row.yrs);
    let acl_data = encode_yrs_doc(&row.acl);

    conn.execute(
        r#"
INSERT INTO documents (id, data, acl_data, author_device_id, counter, created_at, edited_at, schema)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
  ON CONFLICT (id) DO UPDATE
     SET data = excluded.data,
         acl_data = excluded.acl_data,
         author_device_id = excluded.author_device_id,
         counter = excluded.counter,
         edited_at = excluded.edited_at"#,
        params![
            row.meta.id,
            data,
            acl_data,
            row.meta.author_device_id,
            row.meta.counter,
            row.meta.created_at,
            row.meta.edited_at,
            row.meta.schema,
        ],
    )?;

    Ok(())
}

pub(crate) fn delete_row(conn: &Connection, doc_id: &str) -> Result<()> {
    conn.execute("DELETE FROM documents WHERE id = ?", [doc_id])?;
    conn.execute("DELETE FROM card_index WHERE id = ?", [doc_id])?;
    Ok(())
}

#[derive(Clone)]
pub struct DocSecretRow {
    pub id: String,
    /// Secret value
    pub key: Vec<u8>,
    pub account_ids: Vec<String>,
    pub doc_id: Option<String>,
    pub algorithm: i32,
    pub created_at: DateTime<Utc>,
    /// Time after which this secret should no longer be used.
    pub obsolete_at: DateTime<Utc>,
}

pub struct DocSecret {
    pub id: String,
    pub cipher: ChaCha20Poly1305,
}

impl DocSecret {
    pub fn new(id: &str, bytes: &[u8]) -> Self {
        let key = chacha20poly1305::Key::from_slice(bytes);
        Self {
            id: id.to_string(),
            cipher: ChaCha20Poly1305::new(key),
        }
    }
}

impl From<DocSecretRow> for DocSecret {
    fn from(row: DocSecretRow) -> Self {
        let key = chacha20poly1305::Key::from_slice(row.key.as_ref());
        Self {
            id: row.id,
            cipher: ChaCha20Poly1305::new(key),
        }
    }
}
