use anyhow::{Context, Result};
use bolik_migrations::rusqlite::params;
use bolik_proto::sync::request;
use prost::Message;

mod mailbox_atom;
use chrono::{Timelike, Utc};
pub use mailbox_atom::{MailboxAtom, MailboxCtx};
use uuid::Uuid;

use crate::registry::WithTxn;

/// Add MLS message to push mailbox queue
pub fn queue_mls_message<'a>(
    ctx: &impl WithTxn<'a>,
    message: request::SecretGroupMessage,
) -> Result<()> {
    queue_mailbox(ctx, request::push_mailbox::Value::Message(message))
}

/// Add MLS commit to push mailbox queue
pub fn queue_mls_commit<'a>(
    ctx: &impl WithTxn<'a>,
    message: request::SecretGroupCommit,
) -> Result<()> {
    queue_mailbox(ctx, request::push_mailbox::Value::Commit(message))
}

/// Add mailbox message to push mailbox queue
pub fn queue_mailbox<'a>(
    ctx: &impl WithTxn<'a>,
    value: request::push_mailbox::Value,
) -> Result<()> {
    let now = Utc::now();
    let message = request::PushMailbox {
        id: Uuid::new_v4().to_string(),
        value: Some(value),
        created_at_sec: now.timestamp(),
        created_at_nano: now.nanosecond(),
    };
    ctx.txn()
        .execute(
            "INSERT INTO push_mailbox_queue (id, message) VALUES (?1, ?2)",
            params![message.id, message.encode_to_vec()],
        )
        .context("Insert push_mailbox_queue")?;
    Ok(())
}
