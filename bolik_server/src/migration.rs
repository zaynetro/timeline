use bolik_migrations::{rusqlite::Connection, MigrationError};

const CHANGELOG: [(&str, &str); 1] = [(
    "20220807",
    r#"
CREATE TABLE credentials (
  device_id TEXT PRIMARY KEY,
  data BLOB NOT NULL
) WITHOUT ROWID;

CREATE TABLE unused_key_packages (
  ref TEXT PRIMARY KEY,
  data BLOB NOT NULL,
  device_id TEXT NOT NULL REFERENCES credentials(device_id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE signature_chains (
  id TEXT PRIMARY KEY,
  chain BLOB NOT NULL,
  is_account INT
) WITHOUT ROWID;

CREATE TABLE signature_chain_members (
  chain_id TEXT NOT NULL REFERENCES signature_chains(id) ON DELETE CASCADE,
  device_id TEXT NOT NULL REFERENCES credentials(device_id),
  PRIMARY KEY (chain_id, device_id)
) WITHOUT ROWID;

CREATE TABLE device_mailbox (
  id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  data BLOB NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (id, device_id)
);

CREATE TABLE blobs (
  id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  bucket TEXT NOT NULL,
  path TEXT NOT NULL,
  uploaded INT,
  size_bytes INT,
  unused_since TEXT,
  PRIMARY KEY (id, device_id)
) WITHOUT ROWID;

CREATE TABLE account_docs (
  account_id TEXT NOT NULL,
  doc_id TEXT NOT NULL,
  author_device_id TEXT NOT NULL,
  counter INT NOT NULL,
  secret_id TEXT,
  payload BLOB,
  payload_signature TEXT NOT NULL,
  created_at TEXT NOT NULL,
  deleted_at TEXT,
  PRIMARY KEY (account_id, doc_id, author_device_id)
) WITHOUT ROWID;

-- blob_ref (blob_id and device_id) point to blobs
-- doc_ref (account_id, doc_id, author_device_id) point to docs
CREATE TABLE doc_blobs (
  blob_id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  account_id TEXT NOT NULL,
  doc_id TEXT NOT NULL,
  author_device_id TEXT NOT NULL,
  PRIMARY KEY (blob_id, device_id, account_id, doc_id, author_device_id),
  FOREIGN KEY(blob_id, device_id) REFERENCES blobs(id, device_id),
  FOREIGN KEY(account_id, doc_id, author_device_id)
      REFERENCES account_docs(account_id, doc_id, author_device_id)
      ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE mailbox_ack_stats (
  day TEXT PRIMARY KEY,
  processed INT DEFAULT 0,
  failed INT DEFAULT 0
) WITHOUT ROWID;
"#,
)];

pub fn apply(conn: &Connection) -> Result<(), MigrationError> {
    bolik_migrations::apply(conn, &CHANGELOG)?;
    bolik_migrations::add_seahash(conn)?;
    bolik_migrations::add_group_seahash(conn)?;

    Ok(())
}
