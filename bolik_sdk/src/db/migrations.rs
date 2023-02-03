use anyhow::Result;
use bolik_migrations::rusqlite::Connection;

const _FUTURE_CHANGELOG: &[(&str, &str)] = &[(
    "20230127",
    r#"
CREATE TABLE device_settings (
  device_id TEXT REFERENCES mls_keys(id) PRIMARY KEY,
  device_name TEXT NOT NULL,
  account_id TEXT
) WITHOUT ROWID;

CREATE TABLE device_vector_clock (
  device_id TEXT PRIMARY KEY,
  counter INT NOT NULL
) WITHOUT ROWID;

CREATE TABLE mls_keys (
  id TEXT PRIMARY KEY,
  encrypted_value BLOB NOT NULL,
  deleted_at TEXT
) WITHOUT ROWID;

CREATE TABLE signature_chains (
  id TEXT PRIMARY KEY,
  chain BLOB NOT NULL,
  account_ids TEXT
) WITHOUT ROWID;

CREATE TABLE signature_chain_devices (
  device_id TEXT NOT NULL,
  chain_id TEXT NOT NULL REFERENCES signature_chains(id),
  credential BLOB NOT NULL,
  last_counter INT,
  PRIMARY KEY (device_id, chain_id)
) WITHOUT ROWID;

CREATE TABLE mls_groups (
  id TEXT NOT NULL REFERENCES signature_chains(id),
  chain_hash TEXT NOT NULL,
  epoch INT NOT NULL,
  encrypted_state BLOB NOT NULL,
  accounts_hash TEXT,
  PRIMARY KEY (id, chain_hash)
) WITHOUT ROWID;

CREATE TABLE doc_secrets (
  id TEXT PRIMARY KEY,
  encrypted_secret BLOB NOT NULL,
  accounts_hash TEXT,
  account_ids TEXT NOT NULL,
  doc_id TEXT,
  algorithm INT NOT NULL,
  created_at TEXT NOT NULL,
  obsolete_at TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  schema INT NOT NULL,
  data BLOB,
  acl_data BLOB NOT NULL,
  author_device_id TEXT NOT NULL,
  counter INT NOT NULL,
  created_at TEXT NOT NULL,
  edited_at TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE blobs (
  id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  checksum TEXT NOT NULL,
  path TEXT NOT NULL,
  synced INT,
  PRIMARY KEY (id, device_id)
) WITHOUT ROWID;

CREATE TABLE key_packages_queue (
  message BLOB NOT NULL
);

CREATE TABLE push_mailbox_queue (
  id TEXT NOT NULL PRIMARY KEY,
  message BLOB NOT NULL
);

CREATE TABLE ack_mailbox_queue (
  message_id TEXT PRIMARY KEY,
  error TEXT
) WITHOUT ROWID;

CREATE VIRTUAL TABLE card_index USING fts5 (id UNINDEXED, text, label_ids);

CREATE TABLE local_notifications (
  id TEXT PRIMARY KEY,
  body BLOB,
  created_at TEXT NOT NULL
) WITHOUT ROWID;

-- Contains a complete doc message to be sent
CREATE TABLE push_docs_queue (
  message BLOB,
  queued_at TEXT DEFAULT_TIMESTAMP
);

CREATE TABLE failed_docs (
  doc_id TEXT NOT NULL,
  author_device_id TEXT NOT NULL,
  tries INT DEFAULT 1,
  retry_after TEXT NOT NULL,
  PRIMARY KEY (doc_id, author_device_id)
);

CREATE TABLE process_fetched_docs_queue (
  doc_id TEXT NOT NULL REFERENCES documents (id) ON DELETE CASCADE PRIMARY KEY,
  is_new INT NOT NULL,
  from_account_id TEXT NOT NULL,
  priority INT NOT NULL
);
"#,
)];

const CHANGELOG: &[(&str, &str)] = &[
    (
        "20230107",
        r#"
CREATE TABLE device_settings (
  device_id TEXT REFERENCES mls_keys(id) PRIMARY KEY,
  device_name TEXT NOT NULL,
  account_id TEXT
) WITHOUT ROWID;

CREATE TABLE device_vector_clock (
  device_id TEXT PRIMARY KEY,
  counter INT NOT NULL
) WITHOUT ROWID;

CREATE TABLE mls_keys (
  id TEXT PRIMARY KEY,
  encrypted_value BLOB NOT NULL,
  deleted_at TEXT
) WITHOUT ROWID;

CREATE TABLE signature_chains (
  id TEXT PRIMARY KEY,
  chain BLOB NOT NULL,
  account_ids TEXT
) WITHOUT ROWID;

CREATE TABLE signature_chain_devices (
  device_id TEXT NOT NULL,
  chain_id TEXT NOT NULL REFERENCES signature_chains(id),
  credential BLOB NOT NULL,
  last_counter INT,
  PRIMARY KEY (device_id, chain_id)
) WITHOUT ROWID;

CREATE TABLE mls_groups (
  id TEXT NOT NULL REFERENCES signature_chains(id),
  chain_hash TEXT NOT NULL,
  epoch INT NOT NULL,
  encrypted_state BLOB NOT NULL,
  accounts_hash TEXT,
  PRIMARY KEY (id, chain_hash)
) WITHOUT ROWID;

CREATE TABLE doc_secrets (
  id TEXT PRIMARY KEY,
  encrypted_secret BLOB NOT NULL,
  accounts_hash TEXT,
  account_ids TEXT NOT NULL,
  algorithm INT NOT NULL,
  created_at TEXT NOT NULL,
  obsolete_at TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  schema INT NOT NULL,
  data BLOB,
  acl_data BLOB NOT NULL,
  author_device_id TEXT NOT NULL,
  counter INT NOT NULL,
  created_at TEXT NOT NULL,
  edited_at TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE deleted_docs_queue (
  doc_id TEXT PRIMARY KEY,
  orig_created_at TEXT NOT NULL,
  deleted_at TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE blobs (
  id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  checksum TEXT NOT NULL,
  path TEXT NOT NULL,
  synced INT,
  PRIMARY KEY (id, device_id)
) WITHOUT ROWID;

CREATE TABLE key_packages_queue (
  message BLOB NOT NULL
);

CREATE TABLE push_mailbox_queue (
  id TEXT NOT NULL PRIMARY KEY,
  message BLOB NOT NULL
);

CREATE TABLE ack_mailbox_queue (
  message_id TEXT PRIMARY KEY,
  error TEXT
) WITHOUT ROWID;

CREATE VIRTUAL TABLE card_index USING fts5 (id UNINDEXED, text, label_ids);
"#,
    ),
    (
        "20230127",
        r#"
ALTER TABLE doc_secrets ADD COLUMN doc_id TEXT;

CREATE TABLE local_notifications (
  id TEXT PRIMARY KEY,
  body BLOB,
  created_at TEXT NOT NULL
) WITHOUT ROWID;

-- Contains a complete doc message to be sent
CREATE TABLE push_docs_queue (
  message BLOB,
  queued_at TEXT DEFAULT_TIMESTAMP
);
DROP TABLE deleted_docs_queue;

CREATE TABLE failed_docs (
  doc_id TEXT NOT NULL,
  author_device_id TEXT NOT NULL,
  tries INT DEFAULT 1,
  retry_after TEXT NOT NULL,
  PRIMARY KEY (doc_id, author_device_id)
);
CREATE TABLE process_fetched_docs_queue (
  doc_id TEXT NOT NULL REFERENCES documents (id) ON DELETE CASCADE PRIMARY KEY,
  is_new INT NOT NULL,
  from_account_id TEXT NOT NULL,
  priority INT NOT NULL
);
"#,
    ),
];

// Search cards examples:
// Data:
//   INSERT INTO card_index (id, text, labels) VALUES (1, 'Hello', 'one,two,three,');
// Query to filter labels:
//   SELECT * FROM card_index WHERE labels MATCH 'two';
//   SELECT * FROM card_index WHERE labels MATCH 'one AND two';
// Query to search:
//   SELECT * FROM card_index WHERE text MATCH 'NEAR(hel*)';

pub fn apply(conn: &Connection) -> Result<()> {
    bolik_migrations::apply(conn, CHANGELOG)?;
    bolik_migrations::add_seahash(conn)?;
    bolik_migrations::add_group_seahash(conn)?;
    Ok(())
}
