use std::collections::HashMap;

use anyhow::{bail, Result};
use chrono::{DateTime, TimeZone, Utc};
use lib0::any::Any;
use uuid::Uuid;
use yrs::{Map, MapPrelim, ReadTxn, Transact};

use crate::documents::{yrs_util::int64_from_yrs, DbDocRow};

use super::ProfileView;

#[derive(Debug, Clone, PartialEq)]
pub struct AccView {
    pub id: String,
    pub created_at: DateTime<Utc>,

    pub name: String,
    pub contacts: Vec<AccContact>,
    pub labels: Vec<AccLabel>,
    pub devices: Vec<AccDevice>,
}

impl AccView {
    const CONTACTS: &'static str = "contacts";
    const LABELS: &'static str = "labels";
    const DEVICES: &'static str = "devices";

    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            created_at: Utc::now(),
            name: "(Pending)".into(),
            contacts: vec![],
            labels: vec![],
            devices: vec![],
        }
    }

    pub fn init(client_id: yrs::block::ClientID) -> yrs::Doc {
        yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        })
    }

    pub fn from_db(row: DbDocRow, profile_row: Option<DbDocRow>) -> (Self, yrs::Doc) {
        let meta = row.meta;
        let doc = row.yrs;

        let name = if let Some(profile) = profile_row {
            let profile = ProfileView::from_db(profile).0;
            profile.name
        } else {
            ProfileView::default_name(&meta.id)
        };

        let contacts = Self::read_contacts(&doc);
        let labels = Self::read_labels(&doc);
        let devices = Self::read_devices(&doc);

        (
            Self {
                id: meta.id,
                created_at: meta.created_at,
                name,
                contacts: contacts.unwrap_or_default(),
                labels: labels.unwrap_or_default(),
                devices: devices.unwrap_or_default(),
            },
            doc,
        )
    }

    pub fn with_profile(&mut self, profile: ProfileView) {
        self.name = profile.name;
    }

    pub fn add_contact(doc: &yrs::Doc, contact: AccContact) {
        let contact_prelim: MapPrelim<Any> = MapPrelim::from(HashMap::from([(
            AccContact::NAME.to_string(),
            contact.name.into(),
        )]));
        let contacts = doc.get_or_insert_map(Self::CONTACTS);
        let txn = &mut doc.transact_mut();
        contacts.insert(txn, contact.account_id, contact_prelim);
    }

    pub fn edit_contact_name(doc: &yrs::Doc, account_id: &str, name: &str) -> Result<()> {
        let contacts = doc.get_or_insert_map(Self::CONTACTS);
        let txn = &mut doc.transact_mut();
        if let Some(contact) = contacts.get(txn, account_id).and_then(|v| v.to_ymap()) {
            contact.insert(txn, AccContact::NAME, name);
            Ok(())
        } else {
            bail!("Unknown contact")
        }
    }

    pub fn create_label(doc: &yrs::Doc, label: AccLabel) {
        let label_prelim: MapPrelim<Any> = MapPrelim::from(HashMap::from([(
            AccLabel::NAME.to_string(),
            label.name.into(),
        )]));
        let labels = doc.get_or_insert_map(Self::LABELS);
        let txn = &mut doc.transact_mut();
        labels.insert(txn, label.id, label_prelim);
    }

    pub fn delete_label(doc: &yrs::Doc, label_id: &str) {
        let labels = doc.get_or_insert_map(Self::LABELS);
        let txn = &mut doc.transact_mut();
        labels.remove(txn, label_id);
    }

    pub fn add_device(doc: &yrs::Doc, device: AccDevice) {
        let device_prelim: MapPrelim<Any> = MapPrelim::from(HashMap::from([
            (AccDevice::NAME.to_string(), device.name.into()),
            (
                AccDevice::ADDED_AT.to_string(),
                device.added_at.timestamp().into(),
            ),
        ]));
        let devices = doc.get_or_insert_map(Self::DEVICES);
        let txn = &mut doc.transact_mut();
        devices.insert(txn, device.id, device_prelim);
    }

    pub fn remove_device(doc: &yrs::Doc, device_id: &str) {
        let devices = doc.get_or_insert_map(Self::DEVICES);
        let txn = &mut doc.transact_mut();
        devices.remove(txn, device_id);
    }

    pub fn read_contacts(doc: &yrs::Doc) -> Option<Vec<AccContact>> {
        let txn = &doc.transact();
        txn.get_map(Self::CONTACTS).and_then(|m| {
            m.iter(txn)
                .map(|(account_id, v)| AccContact::from_map_entry(txn, account_id.to_string(), v))
                .into_iter()
                .collect()
        })
    }

    fn read_labels(doc: &yrs::Doc) -> Option<Vec<AccLabel>> {
        let txn = &doc.transact();
        txn.get_map(Self::LABELS).and_then(|m| {
            m.iter(txn)
                .map(|(id, v)| AccLabel::from_map_entry(txn, id.to_string(), v))
                .into_iter()
                .collect()
        })
    }

    fn read_devices(doc: &yrs::Doc) -> Option<Vec<AccDevice>> {
        let txn = &doc.transact();
        txn.get_map(Self::DEVICES).and_then(|m| {
            m.iter(txn)
                .map(|(id, v)| AccDevice::from_map_entry(txn, id.to_string(), v))
                .into_iter()
                .collect()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccContact {
    pub account_id: String,
    pub name: String,
}

impl AccContact {
    const NAME: &'static str = "name";

    fn from_map_entry(
        txn: &impl ReadTxn,
        account_id: String,
        value: yrs::types::Value,
    ) -> Option<Self> {
        value.to_ymap().and_then(|ymap| {
            Some(Self {
                name: ymap
                    .get(txn, Self::NAME)
                    .map(|v| v.to_string(txn))
                    .unwrap_or_else(|| ProfileView::default_name(&account_id)),
                account_id,
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccLabel {
    pub id: String,
    pub name: String,
}

impl AccLabel {
    const NAME: &'static str = "name";

    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
        }
    }

    fn from_map_entry(txn: &impl ReadTxn, id: String, value: yrs::types::Value) -> Option<Self> {
        value.to_ymap().and_then(|ymap| {
            Some(Self {
                id,
                name: ymap.get(txn, Self::NAME)?.to_string(txn),
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccDevice {
    pub id: String,
    pub name: String,
    pub added_at: DateTime<Utc>,
}

impl AccDevice {
    const NAME: &'static str = "name";
    const ADDED_AT: &'static str = "added_at";

    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            added_at: Utc::now(),
        }
    }

    fn from_map_entry(txn: &impl ReadTxn, id: String, value: yrs::types::Value) -> Option<Self> {
        value.to_ymap().and_then(|ymap| {
            let added_at = ymap
                .get(txn, Self::ADDED_AT)
                .and_then(int64_from_yrs)
                .and_then(|secs| Utc.timestamp_opt(secs, 0).earliest())
                .unwrap_or(Utc::now());
            Some(Self {
                id,
                name: ymap.get(txn, Self::NAME)?.to_string(txn),
                added_at,
            })
        })
    }
}
