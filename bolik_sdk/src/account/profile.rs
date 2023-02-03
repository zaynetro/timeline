use yrs::{Map, ReadTxn, Transact};

use crate::documents::{DbDocRow, DbDocRowMeta};

/// Profile is a public part of account
pub struct ProfileView {
    pub account_id: String,
    pub name: String,
}

impl ProfileView {
    const FIELDS: &'static str = "fields";
    const NAME: &'static str = "name";

    pub fn init(client_id: yrs::block::ClientID) -> yrs::Doc {
        yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        })
    }

    pub fn from_db(row: DbDocRow) -> (Self, yrs::Doc) {
        let name = Self::get_name(&row);

        (
            Self {
                account_id: row.meta.id.split('/').next().unwrap().into(),
                name,
            },
            row.yrs,
        )
    }

    pub fn set_name(doc: &yrs::Doc, name: String) {
        let fields = doc.get_or_insert_map(Self::FIELDS);
        let txn = &mut doc.transact_mut();
        fields.insert(txn, Self::NAME, name);
    }

    pub fn default_name(account_id: &str) -> String {
        let short_id: String = account_id.chars().take(6).collect();
        format!("Account #{}", short_id.to_lowercase())
    }

    pub fn get_name(row: &DbDocRow) -> String {
        let txn = &row.yrs.transact();
        txn.get_map(Self::FIELDS)
            .and_then(|m| m.get(txn, Self::NAME).map(|v| v.to_string(txn)))
            .unwrap_or_else(|| Self::default_name(&row.meta.id))
    }

    pub fn get_account_id(meta: &DbDocRowMeta) -> Option<String> {
        let mut parts = meta.id.split('/');
        match (parts.next(), parts.next()) {
            (Some(id), Some("profile")) => Some(id.into()),
            _ => None,
        }
    }
}
