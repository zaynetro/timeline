use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use yrs::{Map, ReadTxn, Transact};

use crate::documents::{
    yrs_util::{self, int64_from_yrs},
    BIN_LABEL_ID,
};

#[derive(Debug)]
pub struct AclDoc {
    /// Account ID to rights mapping
    pub accounts: HashMap<String, AclRights>,
    /// Present when doc was moved to bin
    pub bolik_bin: Option<DateTime<Utc>>,
    /// Define how the doc should be sent to accounts it is shared with
    pub mode: AclOperationMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AclRights {
    Read,
    Write,
    Admin,
}

impl AclRights {
    fn to_ordinal(&self) -> u8 {
        match self {
            Self::Read => 1,
            Self::Write => 2,
            Self::Admin => 8,
        }
    }

    fn from_ordinal(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Read),
            2 => Some(Self::Write),
            8 => Some(Self::Admin),
            _ => None,
        }
    }
}

impl PartialOrd for AclRights {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_ordinal().partial_cmp(&other.to_ordinal())
    }
}

#[derive(Debug)]
pub enum AclOperationMode {
    /// Build list of participants from accounts map.
    Normal,

    /// Build list of participants outside of this doc.
    /// Use case: we want to send Profile doc to all account contacts.
    Custom,
}

impl AclOperationMode {
    fn to_ordinal(&self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Custom => 1,
        }
    }

    fn from_ordinal(n: u8) -> Self {
        match n {
            1 => Self::Custom,
            _ => Self::Normal,
        }
    }
}

impl AclDoc {
    const FIELDS: &'static str = "fields";
    const ACCOUNTS: &'static str = "accounts";
    const MODE: &'static str = "mode";

    pub fn new(admin_id: impl Into<String>) -> Self {
        let mut accounts = HashMap::new();
        accounts.insert(admin_id.into(), AclRights::Admin);
        Self {
            accounts,
            bolik_bin: None,
            mode: AclOperationMode::Normal,
        }
    }

    pub fn init(client_id: yrs::block::ClientID, admin_id: &str) -> yrs::Doc {
        Self::init_with_mode(client_id, admin_id, AclOperationMode::Normal)
    }

    pub fn init_with_mode(
        client_id: yrs::block::ClientID,
        admin_id: &str,
        mode: AclOperationMode,
    ) -> yrs::Doc {
        let doc = yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        });
        Self::add(&doc, admin_id.to_string(), AclRights::Admin);
        Self::set_mode(&doc, mode);
        doc
    }

    pub fn from_doc(doc: &yrs::Doc) -> Self {
        let accounts = Self::read_accounts(doc);

        let mut acl = Self {
            accounts,
            bolik_bin: None,
            mode: AclOperationMode::Normal,
        };

        let txn = &doc.transact();
        if let Some(fields) = txn.get_map(Self::FIELDS) {
            acl.bolik_bin = fields
                .get(txn, BIN_LABEL_ID)
                .and_then(|v| int64_from_yrs(v))
                .and_then(|secs| Utc.timestamp_opt(secs, 0).earliest());

            if let Some(yrs::types::Value::Any(lib0::any::Any::Number(n))) =
                fields.get(txn, Self::MODE)
            {
                acl.mode = AclOperationMode::from_ordinal(n as u8);
            }
        }

        acl
    }

    /// Check if given account id has rights to edit
    pub fn allowed_to_edit(&self, account_id: &str) -> bool {
        self.accounts
            .get(account_id)
            .map(|p| p >= &AclRights::Write)
            .unwrap_or(false)
    }

    /// Check if given account id has rights to admin
    pub fn allowed_to_admin(&self, account_id: &str) -> bool {
        self.accounts
            .get(account_id)
            .map(|p| p >= &AclRights::Admin)
            .unwrap_or(false)
    }

    pub fn add(doc: &yrs::Doc, account_id: String, rights: AclRights) {
        let accounts = doc.get_or_insert_map(Self::ACCOUNTS);
        let txn = &mut doc.transact_mut();
        accounts.insert(txn, account_id, rights.to_ordinal() as u32);
    }

    pub fn remove(doc: &yrs::Doc, account_id: &str) {
        let accounts = doc.get_or_insert_map(Self::ACCOUNTS);
        let txn = &mut doc.transact_mut();
        accounts.remove(txn, account_id);
    }

    pub fn set_mode(doc: &yrs::Doc, mode: AclOperationMode) {
        let fields = doc.get_or_insert_map(Self::FIELDS);
        let txn = &mut doc.transact_mut();
        fields.insert(txn, Self::MODE, mode.to_ordinal() as u32);
    }

    pub fn move_to_bin(doc: &yrs::Doc) {
        let fields = doc.get_or_insert_map(Self::FIELDS);
        let txn = &mut doc.transact_mut();
        fields.insert(txn, BIN_LABEL_ID, Utc::now().timestamp());
    }

    /// Return when this doc was added to bin if it was.
    pub fn in_bin_since(doc: &yrs::Doc) -> Option<DateTime<Utc>> {
        let txn = &doc.transact();
        txn.get_map(Self::FIELDS)
            .and_then(|fields| fields.get(txn, BIN_LABEL_ID))
            .and_then(|v| yrs_util::int64_from_yrs(v))
            .and_then(|secs| Utc.timestamp_opt(secs, 0).earliest())
    }

    pub fn read_accounts(doc: &yrs::Doc) -> HashMap<String, AclRights> {
        let txn = &doc.transact();
        let mut accounts = HashMap::new();
        if let Some(acc_map) = txn.get_map(Self::ACCOUNTS) {
            for (account_id, v) in acc_map.iter(txn) {
                if let yrs::types::Value::Any(lib0::any::Any::Number(n)) = v {
                    if let Some(rights) = AclRights::from_ordinal(n as u8) {
                        accounts.insert(account_id.to_string(), rights);
                    }
                }
            }
        }
        accounts
    }

    /// Build a list of participants (account ids the doc should be delivered to).
    pub fn participants(self) -> Vec<String> {
        self.accounts.into_iter().map(|(k, _)| k).collect()
    }
}

#[derive(Debug)]
pub enum AclChange {
    Add {
        account_id: String,
        rights: AclRights,
    },
    Remove {
        account_id: String,
    },
    MoveToBin,
}
