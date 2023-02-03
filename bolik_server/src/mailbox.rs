use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension,
};
use bolik_chain::{ChainUsed, MergeAdvice, SignatureChain};
use bolik_migrations::rusqlite::{params, Connection, OptionalExtension};
use bolik_proto::{
    prost::Message,
    sync::{request, response},
};
use chrono::{DateTime, TimeZone, Utc};
use hyper::StatusCode;
use openmls::prelude::{MlsMessageIn, TlsDeserializeTrait};
use openmls_traits::OpenMlsCryptoProvider;
use tracing::instrument;

use crate::{
    error::{AppError, DbContext, ServerError, UserError},
    mls::{get_device_id, get_key_package_ref, CryptoProvider, VoidCryptoProvider},
    router::CurrentDevice,
    state::{AppState, Protobuf},
};

#[axum::debug_handler]
#[instrument(skip_all, fields(group_id, chain_hash, msg_id = message.id))]
pub async fn push(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Protobuf(message): Protobuf<request::PushMailbox>,
) -> Result<impl IntoResponse, AppError> {
    let created_at = Utc
        .timestamp_opt(message.created_at_sec, message.created_at_nano)
        .earliest()
        .ok_or(UserError::InvalidCreatedAt(
            message.created_at_sec,
            message.created_at_nano,
        ))?;

    match message.value {
        Some(request::push_mailbox::Value::Account(a)) => {
            let chain_msg = a.chain.ok_or(UserError::MissingField {
                field: "chain".into(),
            })?;
            let chain = SignatureChain::decode(chain_msg.clone())
                .map_err(UserError::SignatureChainDecode)?;
            let group_id = chain.root().to_string();
            tracing::Span::current().record("group_id", &group_id);
            tracing::debug!("Received Account");

            if chain.head() != chain.root() {
                return Err(UserError::WrongSignatureChainLen.into());
            }
            chain
                .verify(&VoidCryptoProvider::default())
                .map_err(UserError::SignatureChainVerify)?;

            // Save signature chain
            let mut conn = app.conn.lock().unwrap();
            let txn = conn.transaction().db_txn()?;

            let existing_row = txn
                .query_row(
                    "SELECT 1 FROM signature_chains WHERE id = ?",
                    [&group_id],
                    |_row| Ok(()),
                )
                .optional()
                .db_context("Query signature_chain")?;

            // Save signature chain for account message only once
            if existing_row.is_none() {
                txn.execute(
                    "INSERT INTO signature_chains (id, chain, is_account) VALUES (?, ?, ?)",
                    params![group_id, chain_msg.encode_to_vec(), true],
                )
                .db_context("Insert signature_chain")?;
                txn.execute(
                    "DELETE FROM signature_chain_members WHERE chain_id = ?",
                    params![group_id],
                )
                .db_context("Delete signature_chain_members")?;

                let members = chain
                    .members(&CryptoProvider::default())
                    .map_err(ServerError::SignatureChain)?;
                for device_id in members.device_ids() {
                    txn.execute(
                        "INSERT INTO signature_chain_members (chain_id, device_id) VALUES (?, ?)",
                        params![group_id, device_id],
                    )
                    .db_context("Insert signature_chain_member")?;
                }
            }
            txn.commit().db_commit()?;
        }
        Some(request::push_mailbox::Value::Commit(c)) => {
            let chain_msg = c.chain.clone().ok_or(UserError::MissingField {
                field: "chain".into(),
            })?;
            let chain = SignatureChain::decode(chain_msg.clone())
                .map_err(UserError::SignatureChainDecode)?;
            let group_id = chain.root().to_string();
            tracing::Span::current().record("group_id", &group_id);
            tracing::Span::current().record("chain_hash", chain.head());

            let members = chain
                .members(&CryptoProvider::default())
                .map_err(ServerError::SignatureChain)?;
            tracing::debug!(
                chain_members = members.len(),
                invitees = chain.last().body.ops.add.len(),
                "Received Commit"
            );

            let mls_message = MlsMessageIn::tls_deserialize(&mut c.mls.as_slice())
                .map_err(UserError::MlsMessageInDecode)?;
            let epoch = mls_message.epoch().as_u64();

            if !mls_message.is_handshake_message() {
                return Err(UserError::ExpectedHandshakeMsg.into());
            }

            let mls_group_id =
                String::from_utf8_lossy(mls_message.group_id().as_slice()).to_string();
            if mls_group_id != group_id {
                return Err(UserError::GroupChainMismatch.into());
            }

            let backend = &VoidCryptoProvider::default();
            chain
                .verify(backend)
                .map_err(UserError::SignatureChainVerify)?;

            let mut conn = app.conn.lock().unwrap();
            let txn = conn.transaction().db_txn()?;
            let existing_row: Option<Vec<u8>> = txn
                .query_row(
                    "SELECT chain FROM signature_chains WHERE id = ? and is_account = 1",
                    [&group_id],
                    |row| row.get(0),
                )
                .optional()
                .db_context("Find signature_chain")?;

            let mailbox_entry = response::mailbox::Entry {
                id: message.id.clone(),
                value: Some(response::mailbox::entry::Value::Message(
                    response::SecretGroupMessage {
                        mls: c.mls,
                        chain: Some(chain_msg.clone()),
                        // We need to specify the hash of a chain before the commit was applied.
                        // This field is a pointer to the old version of the group
                        // (the version that should be fetch to apply the commit message to).
                        chain_hash: chain
                            .hash_at(epoch)
                            .ok_or(UserError::SignatureChainMissingEpoch(epoch))?
                            .to_string(),
                    },
                )),
            };
            let mailbox_entry = EncodedMailboxEntry::new(mailbox_entry, created_at);

            // Forward message to all devices
            let members = chain
                .members_at(epoch, backend.crypto())
                .map_err(ServerError::SignatureChain)?;
            for device_id in members.device_ids() {
                if device_id != &current_device.device_id {
                    db_add_mailbox(&txn, &device_id, &mailbox_entry)?;
                }
            }

            if let Some(welcome) = c.welcome {
                let welcome_entry = response::mailbox::Entry {
                    id: message.id,
                    value: Some(response::mailbox::entry::Value::Welcome(
                        response::SecretGroupWelcome {
                            welcome,
                            chain: c.chain,
                        },
                    )),
                };
                let welcome_entry = EncodedMailboxEntry::new(welcome_entry, created_at);

                // Forward welcome to all invitees (devices that were added in the last block)
                for key_package in &chain.last().body.ops.add {
                    let device_id = get_device_id(key_package.credential())?;
                    let key_ref = get_key_package_ref(key_package)?;

                    db_add_mailbox(&txn, &device_id, &welcome_entry)?;

                    let remaining_packages: usize = txn
                        .query_row(
                            "SELECT count(*) FROM unused_key_packages WHERE device_id = ?",
                            [&device_id],
                            |row| row.get(0),
                        )
                        .db_context("Find key_packages")?;

                    if remaining_packages > 1 {
                        txn.execute("DELETE FROM unused_key_packages WHERE ref = ?", [key_ref])
                            .db_context("Delete key_package")?;
                    } else {
                        tracing::debug!(
                            device_id,
                            "Skipping deleting key package. Only one remaining."
                        );
                    }
                }
            }

            // Update account chain
            if let Some(existing_bytes) = existing_row {
                let existing_chain = SignatureChain::decode_bytes(&existing_bytes)
                    .map_err(ServerError::SignatureChain)?;

                let MergeAdvice { chain, used, .. } = existing_chain
                    .prepare_merge(chain, backend)
                    .map_err(UserError::SignatureChainVerify)?;

                // Update chain only when remote (received) chain won.
                if used == ChainUsed::Remote {
                    txn.execute(
                        "UPDATE signature_chains SET chain = ? WHERE id = ?",
                        params![chain_msg.encode_to_vec(), group_id],
                    )
                    .db_context("Update signature_chain")?;

                    txn.execute(
                        "DELETE FROM signature_chain_members WHERE chain_id = ?",
                        params![group_id],
                    )
                    .db_context("Delete signature_chain_members")?;

                    let members = chain
                        .members(backend.crypto())
                        .map_err(ServerError::SignatureChain)?;
                    for device_id in members.device_ids() {
                        txn.execute(
                        "INSERT INTO signature_chain_members (chain_id, device_id) VALUES (?, ?)",
                        params![group_id, device_id],
                    )
                    .db_context("Insert signature_chain_member")?;
                    }
                }
            }

            txn.commit().db_commit()?;
        }
        Some(request::push_mailbox::Value::Message(m)) => {
            let mls_in = MlsMessageIn::tls_deserialize(&mut m.mls.as_slice())
                .map_err(UserError::MlsMessageInDecode)?;

            if mls_in.is_handshake_message() {
                return Err(UserError::ExpectedNonHandshakeMsg.into());
            }

            let group_id = String::from_utf8_lossy(mls_in.group_id().as_slice()).to_string();
            tracing::Span::current().record("group_id", &group_id);
            tracing::Span::current().record("chain_hash", &m.chain_hash);
            tracing::debug!("Received Message");

            let mut conn = app.conn.lock().unwrap();
            let txn = conn.transaction().db_txn()?;

            let mailbox_entry = response::mailbox::Entry {
                id: message.id,
                value: Some(response::mailbox::entry::Value::Message(
                    response::SecretGroupMessage {
                        mls: m.mls,
                        chain: None,
                        chain_hash: m.chain_hash,
                    },
                )),
            };
            let mailbox_entry = EncodedMailboxEntry::new(mailbox_entry, created_at);

            // Forward message to all devices
            for device_id in &m.to_device_ids {
                if device_id != &current_device.device_id {
                    db_add_mailbox(&txn, device_id, &mailbox_entry)?;
                }
            }

            txn.commit().db_txn()?;
        }
        None => {
            // Ignore
            return Ok(StatusCode::OK);
        }
    }

    Ok(StatusCode::CREATED)
}

fn db_add_mailbox(
    conn: &Connection,
    to_device_id: &str,
    entry: &EncodedMailboxEntry,
) -> Result<(), AppError> {
    conn.execute(
        r#"
INSERT INTO device_mailbox (id, device_id, data, created_at) VALUES (?1, ?2, ?3, ?4)
    ON CONFLICT (id, device_id) DO NOTHING"#,
        params![entry.id, to_device_id, entry.bytes, entry.created_at],
    )
    .db_context("Insert mailbox msg")?;
    Ok(())
}

#[axum::debug_handler]
pub async fn fetch(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;

    let mut mailbox = vec![];
    {
        let mut stmt = txn
            .prepare(&format!(
                r#"
SELECT data
  FROM device_mailbox
 WHERE device_id = ?
 ORDER BY rowid"#,
            ))
            .db_context("Find mailbox (prepare)")?;

        let mut rows = stmt
            .query(params![current_device.device_id])
            .db_context("Find mailbox")?;
        while let Some(row) = rows.next().db_context("Read row")? {
            let data: Vec<u8> = row.get(0).db_context("Read message data")?;
            let entry = response::mailbox::Entry::decode(data.as_ref())
                .map_err(ServerError::ProtoDecode)?;
            mailbox.push(entry);
        }
    }
    txn.commit().db_txn()?;

    tracing::trace!(mailbox_len = mailbox.len(), "Fetched mailbox");
    let res = response::Mailbox { entries: mailbox };
    Ok((StatusCode::OK, Protobuf(res)))
}

#[axum::debug_handler]
#[instrument(skip_all, fields(message_id))]
pub async fn ack_message(
    State(app): State<AppState>,
    Extension(current_device): Extension<CurrentDevice>,
    Path(message_id): Path<String>,
    Protobuf(info): Protobuf<request::AckMailboxInfo>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(ref err) = info.error {
        tracing::warn!("Client ack mailbox message with error: {}", err);
    }

    let mut conn = app.conn.lock().unwrap();
    let txn = conn.transaction().db_txn()?;

    // Delete mailbox message from queue
    txn.execute(
        "DELETE FROM device_mailbox WHERE id = ? and device_id = ?",
        params![message_id, current_device.device_id],
    )
    .db_context("Delete mailbox msg")?;

    // Save stats
    let day = Utc::now().format("%Y%m%d").to_string();
    let (processed, failed) = if info.error.is_none() { (1, 0) } else { (0, 1) };
    txn.execute(
        r#"
INSERT INTO mailbox_ack_stats (day, processed, failed) VALUES (?1, ?2, ?3)
    ON CONFLICT (day) DO UPDATE
       SET processed = processed + excluded.processed,
           failed = failed + excluded.failed"#,
        params![day, processed, failed],
    )
    .db_context("Insert mailbox stat")?;
    txn.commit().db_commit()?;

    Ok(StatusCode::OK)
}

struct EncodedMailboxEntry {
    id: String,
    bytes: Vec<u8>,
    created_at: DateTime<Utc>,
}

impl EncodedMailboxEntry {
    fn new(entry: response::mailbox::Entry, created_at: DateTime<Utc>) -> Self {
        Self {
            bytes: entry.encode_to_vec(),
            id: entry.id,
            created_at,
        }
    }
}
