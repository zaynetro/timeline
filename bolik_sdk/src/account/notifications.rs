use anyhow::Result;
use lib0::any::Any;
use yrs::{Map, ReadTxn, Transact};

pub struct AccNotifications {}

impl AccNotifications {
    const IDS: &'static str = "ids";

    pub fn init(client_id: yrs::block::ClientID) -> yrs::Doc {
        yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        })
    }

    /// Accept notification.
    pub fn accept(doc: &yrs::Doc, id: String) {
        let ids = doc.get_or_insert_map(Self::IDS);
        let txn = &mut doc.transact_mut();
        ids.insert(txn, id, true);
    }

    /// Ignore notification.
    pub fn ignore(doc: &yrs::Doc, id: String) {
        let ids = doc.get_or_insert_map(Self::IDS);
        let txn = &mut doc.transact_mut();
        ids.insert(txn, id, false);
    }

    pub fn status(doc: &yrs::Doc, id: &str) -> NotificationStatus {
        let txn = &doc.transact();
        let v = txn
            .get_map(Self::IDS)
            .and_then(|m| m.get(txn, id))
            .and_then(|v| {
                if let yrs::types::Value::Any(Any::Bool(b)) = v {
                    Some(b)
                } else {
                    None
                }
            });
        match v {
            Some(true) => NotificationStatus::Accepted,
            Some(false) => NotificationStatus::Ignored,
            None => NotificationStatus::Missing,
        }
    }

    /// Iterate over notification ids and apply a function to each entry.
    pub fn iter_ids(
        doc: &yrs::Doc,
        f: impl Fn(&str, NotificationStatus) -> Result<()>,
    ) -> Result<()> {
        let txn = &doc.transact();
        if let Some(map) = txn.get_map(Self::IDS) {
            for (notification_id, v) in map.iter(txn) {
                let status = match v {
                    yrs::types::Value::Any(Any::Bool(true)) => NotificationStatus::Accepted,
                    yrs::types::Value::Any(Any::Bool(false)) => NotificationStatus::Ignored,
                    _ => {
                        continue;
                    }
                };

                f(notification_id, status)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum NotificationStatus {
    Missing,
    Accepted,
    Ignored,
}
