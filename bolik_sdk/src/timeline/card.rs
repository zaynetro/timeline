use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use bolik_proto::sync::SecretAlgorithm;
use chrono::{DateTime, TimeZone, Utc};
use lib0::any::Any;
use uuid::Uuid;
use yrs::{
    types::{text::YChange, Attrs},
    Map, MapPrelim, ReadTxn, Text, Transact,
};

use crate::documents::{
    yrs_util::{bytes_from_yrs, int64_from_yrs, uint_from_yrs, uint_from_yrs_any},
    DbDocRow, BIN_LABEL_ID,
};

use super::acl_doc::AclDoc;

/// Card is a bare minimum block that user can create.
pub struct CardView {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: DateTime<Utc>,
    pub acl: AclDoc,

    pub blocks: Vec<CardBlock>,
    pub labels: Vec<CardLabel>,
    pub thumbnail: Option<FileThumbnail>,
    /// Mapping from blob id to file's secret.
    pub secrets: HashMap<String, CardSecret>,
}

impl CardView {
    const CONTENT: &'static str = "content";
    const THUMBNAIL: &'static str = "thumbnail";
    const SECRETS: &'static str = "secrets";

    pub fn empty(account_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            edited_at: now,
            acl: AclDoc::new(account_id),
            blocks: vec![],
            labels: vec![],
            thumbnail: None,
            secrets: HashMap::new(),
        }
    }

    pub fn init(client_id: yrs::block::ClientID) -> yrs::Doc {
        yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        })
    }

    pub fn from_db(row: DbDocRow, labels_row: Option<DbDocRow>) -> (Self, yrs::Doc) {
        let mut blocks = vec![];
        let mut thumbnail = None;
        let mut secrets = HashMap::new();

        {
            let txn = &row.yrs.transact();
            if let Some(text) = txn.get_text(Self::CONTENT) {
                let mut position = 0u32;
                for diff in text.diff(txn, YChange::identity) {
                    match diff.insert {
                        yrs::types::Value::Any(lib0::any::Any::String(s)) => {
                            let attrs = diff.attributes.map(|a| a.into());
                            blocks.push(CardBlock {
                                position,
                                view: ContentView::Text(CardText {
                                    value: s.to_string(),
                                    attrs,
                                }),
                            });
                            position += s.chars().count() as u32;
                        }
                        yrs::types::Value::Any(lib0::any::Any::Map(map)) => {
                            let content_type = map
                                .get("_type")
                                .map(|t| t.to_string())
                                .unwrap_or(String::new());

                            if content_type == CardFile::TYPE {
                                let file = CardFile::from_map(&map);
                                if let Some(file) = file {
                                    blocks.push(CardBlock {
                                        position,
                                        view: ContentView::File(file),
                                    });
                                }
                            } else {
                                tracing::warn!("Unknown content type={}", content_type);
                                blocks.push(CardBlock::unsupported(position));
                            }

                            position += 1;
                        }
                        _ => {
                            blocks.push(CardBlock::unsupported(position));
                            position += 1;
                        }
                    }
                }
            }

            if let Some(thumb_map) = txn.get_map(Self::THUMBNAIL) {
                thumbnail = FileThumbnail::from_map(txn, thumb_map);
            }

            if let Some(secrets_map) = txn.get_map(Self::SECRETS) {
                for (k, v) in secrets_map.iter(txn) {
                    if let Some(secret) = CardSecret::from_map_entry(txn, v) {
                        secrets.insert(k.into(), secret);
                    }
                }
            }
        }

        let acl = AclDoc::from_doc(&row.acl);

        let mut labels: Option<Vec<CardLabel>> = labels_row.as_ref().and_then(|row| {
            let txn = &row.yrs.transact();
            txn.get_map(CardLabels::LABELS).and_then(|map| {
                map.iter(txn)
                    .map(|(label_id, v)| CardLabel::from_map_entry(txn, label_id.to_string(), v))
                    .into_iter()
                    .collect()
            })
        });

        // If card was moved to bin for everyone then the label will be present in ACL doc.
        if let Some(added_at) = &acl.bolik_bin {
            let label = CardLabel {
                id: BIN_LABEL_ID.to_string(),
                added_at: *added_at,
            };

            if let Some(l) = labels.as_mut() {
                l.push(label);
            } else {
                labels = Some(vec![label]);
            }
        }
        let edited_at = labels_row
            .filter(|labels| labels.meta.edited_at > row.meta.edited_at)
            .map(|r| r.meta.edited_at)
            .unwrap_or(row.meta.edited_at);

        let view = Self {
            id: row.meta.id,
            created_at: row.meta.created_at,
            edited_at,
            acl,

            blocks,
            labels: labels.unwrap_or_default(),
            thumbnail,
            secrets,
        };
        (view, row.yrs)
    }

    pub fn edit(doc: &yrs::Doc, changes: Vec<CardChange>) {
        let text = doc.get_or_insert_text(Self::CONTENT);
        let thumb_map = doc.get_or_insert_map(Self::THUMBNAIL);
        let secrets = doc.get_or_insert_map(Self::SECRETS);
        let txn = &mut doc.transact_mut();

        for change in changes {
            tracing::debug!(%change);
            match change {
                CardChange::Insert(CardBlock { position, view }) => {
                    let index = min(position, text.len(txn));
                    match view {
                        ContentView::Text(t) => {
                            let attrs = t.attrs.map(|a| a.into()).unwrap_or_default();
                            text.insert_with_attributes(txn, index, &t.value, attrs);
                        }
                        ContentView::File(file) => {
                            let embed = file.embed();
                            // We insert embed with empty attributes so that attributes
                            // would not be inherited from previous chunk.
                            text.insert_embed_with_attributes(
                                txn,
                                index,
                                embed.into(),
                                Attrs::new(),
                            );
                        }
                    }
                }
                CardChange::Remove { position, len } => {
                    let index = min(position, text.len(txn));
                    let len = min(len, text.len(txn) - index);
                    text.remove_range(txn, index, len);
                }
                CardChange::SetThumbnail(thumb) => {
                    thumb_map.clear(txn);
                    if let Some(thumb) = thumb {
                        thumb_map.insert(txn, FileThumbnail::MIME_TYPE, thumb.mime_type);
                        thumb_map.insert(txn, FileThumbnail::WIDTH, thumb.width);
                        thumb_map.insert(txn, FileThumbnail::HEIGHT, thumb.height);
                        thumb_map.insert(txn, FileThumbnail::DATA, thumb.data);
                        thumb_map.insert(txn, FileThumbnail::FROM_CHECKSUM, thumb.from_checksum);
                    }
                }
                CardChange::Format {
                    position,
                    len,
                    attributes,
                } => {
                    let index = min(position, text.len(txn));
                    let len = min(len, text.len(txn) - index);
                    text.format(txn, index, len, attributes.into());
                }
                CardChange::AddFileSecret { blob_id, value } => {
                    secrets.insert(
                        txn,
                        blob_id,
                        MapPrelim::from(CardSecret::new(value).embed()),
                    );
                }
            }
        }
    }

    pub fn cleanup(&self) -> CleanupResult {
        let changes = vec![];
        let mut blob_ids = HashSet::new();

        for block in &self.blocks {
            match &block.view {
                ContentView::File(f) => {
                    blob_ids.insert(&f.blob_id);
                }
                _ => {}
            }
        }

        // TODO: remove unused file secrets

        CleanupResult {
            changes,
            files_changed: false,
        }
    }

    pub fn get_file(self, blob_id: &str) -> Option<CardFile> {
        for block in self.blocks.into_iter() {
            match block.view {
                ContentView::File(f) if f.blob_id == blob_id => {
                    return Some(f);
                }
                _ => {}
            }
        }

        None
    }
}

pub struct CleanupResult {
    pub changes: Vec<CardChange>,
    pub files_changed: bool,
}

#[derive(Debug, PartialEq)]
pub struct CardBlock {
    /// Block offset from the beginning of the text.
    /// If block is text then position is incremented by character count.
    /// If block is an embed then position is incremented by one.
    pub position: u32,
    pub view: ContentView,
}

impl CardBlock {
    fn unsupported(position: u32) -> Self {
        Self {
            position,
            view: ContentView::Text(CardText {
                value: "?".into(),
                attrs: None,
            }),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ContentView {
    Text(CardText),
    File(CardFile),
}

pub struct CardLabels {}

impl CardLabels {
    const LABELS: &'static str = "labels";

    pub fn init(client_id: yrs::block::ClientID) -> yrs::Doc {
        yrs::Doc::with_options(yrs::Options {
            client_id,
            offset_kind: yrs::OffsetKind::Utf32,
            ..Default::default()
        })
    }

    pub fn add_label(doc: &yrs::Doc, label_id: String) {
        let now = Utc::now();
        let label_prelim: MapPrelim<Any> = MapPrelim::from(HashMap::from([(
            CardLabel::ADDED_AT.to_string(),
            now.timestamp().into(),
        )]));

        let labels = doc.get_or_insert_map(Self::LABELS);
        let txn = &mut doc.transact_mut();
        labels.insert(txn, label_id, label_prelim);
    }

    pub fn remove_label(doc: &yrs::Doc, label_id: &str) {
        let labels = doc.get_or_insert_map(Self::LABELS);
        let txn = &mut doc.transact_mut();
        labels.remove(txn, label_id);
    }

    /// Return when this doc was added to bin if it was.
    pub fn in_bin_since(doc: &yrs::Doc) -> Option<DateTime<Utc>> {
        let txn = &doc.transact();
        txn.get_map(Self::LABELS)
            .and_then(|labels| labels.get(txn, BIN_LABEL_ID))
            .and_then(|v| v.to_ymap())
            .and_then(|map| map.get(txn, CardLabel::ADDED_AT))
            .and_then(|v| int64_from_yrs(v))
            .and_then(|secs| Utc.timestamp_opt(secs, 0).earliest())
    }
}

#[derive(Debug, Clone)]
pub struct CardLabel {
    pub id: String,
    pub added_at: DateTime<Utc>,
}

impl CardLabel {
    const ADDED_AT: &'static str = "added_at";

    fn from_map_entry(
        txn: &yrs::Transaction,
        id: String,
        value: yrs::types::Value,
    ) -> Option<Self> {
        value.to_ymap().and_then(|ymap| {
            let secs = int64_from_yrs(ymap.get(txn, Self::ADDED_AT)?)?;
            Some(Self {
                id,
                added_at: Utc.timestamp_opt(secs, 0).earliest()?,
            })
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct CardText {
    pub value: String,
    pub attrs: Option<CardTextAttrs>,
}

impl CardText {
    pub fn new(s: impl Into<String>, attrs: Option<CardTextAttrs>) -> Self {
        Self {
            value: s.into(),
            attrs,
        }
    }

    pub fn text(s: impl Into<String>) -> Self {
        Self::new(s, None)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CardTextAttrs {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub link: Option<String>,
    pub checked: Option<bool>,
    pub heading: Option<u8>,

    // Example values: quote, check list (cl), ordered list (ol), unordered list (ul), code
    pub block: Option<String>,
}

impl CardTextAttrs {
    const BOLD: &'static str = "b";
    const ITALIC: &'static str = "i";
    const UNDERLINE: &'static str = "u";
    const STRIKETHROUGH: &'static str = "s";
    const LINK: &'static str = "a";
    const CHECKED: &'static str = "checked";
    const BLOCK: &'static str = "block";
    const HEADING: &'static str = "heading";

    fn read_bool(attrs: &mut Box<Attrs>, key: &str) -> Option<bool> {
        attrs
            .remove(key)
            .and_then(|v| if let Any::Bool(b) = v { Some(b) } else { None })
    }

    fn read_num(attrs: &mut Box<Attrs>, key: &str) -> Option<f64> {
        attrs.remove(key).and_then(|v| {
            if let Any::Number(n) = v {
                Some(n)
            } else {
                None
            }
        })
    }

    fn read_str(attrs: &mut Box<Attrs>, key: &str) -> Option<String> {
        attrs
            .remove(key)
            .map(|v| v.to_string())
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
    }

    fn insert_bool(attrs: &mut Attrs, key: &str, b: bool) {
        let v = if b { true.into() } else { Any::Null };
        attrs.insert(key.into(), v);
    }

    fn insert_num(attrs: &mut Attrs, key: &str, n: f64) {
        let v = if n > 0.0 { n.into() } else { Any::Null };
        attrs.insert(key.into(), v);
    }

    fn insert_str(attrs: &mut Attrs, key: &str, s: String) {
        let v = if !s.is_empty() { s.into() } else { Any::Null };
        attrs.insert(key.into(), v);
    }
}

impl From<Box<Attrs>> for CardTextAttrs {
    fn from(mut attrs: Box<Attrs>) -> Self {
        Self {
            bold: Self::read_bool(&mut attrs, Self::BOLD),
            italic: Self::read_bool(&mut attrs, Self::ITALIC),
            underline: Self::read_bool(&mut attrs, Self::UNDERLINE),
            strikethrough: Self::read_bool(&mut attrs, Self::STRIKETHROUGH),
            link: Self::read_str(&mut attrs, Self::LINK),
            checked: Self::read_bool(&mut attrs, Self::CHECKED),
            heading: Self::read_num(&mut attrs, Self::HEADING).map(|n| n as u8),
            block: Self::read_str(&mut attrs, Self::BLOCK),
        }
    }
}

impl Into<Attrs> for CardTextAttrs {
    fn into(self) -> Attrs {
        let mut attrs = Attrs::new();
        if let Some(b) = self.bold {
            Self::insert_bool(&mut attrs, Self::BOLD, b);
        }
        if let Some(i) = self.italic {
            Self::insert_bool(&mut attrs, Self::ITALIC, i);
        }
        if let Some(u) = self.underline {
            Self::insert_bool(&mut attrs, Self::UNDERLINE, u);
        }
        if let Some(s) = self.strikethrough {
            Self::insert_bool(&mut attrs, Self::STRIKETHROUGH, s);
        }
        if let Some(checked) = self.checked {
            Self::insert_bool(&mut attrs, Self::CHECKED, checked);
        }
        if let Some(link) = self.link {
            Self::insert_str(&mut attrs, Self::LINK, link);
        }
        if let Some(block) = self.block {
            Self::insert_str(&mut attrs, Self::BLOCK, block);
        }
        if let Some(level) = self.heading {
            Self::insert_num(&mut attrs, Self::HEADING, level as f64);
        }
        attrs
    }
}

impl std::fmt::Display for CardTextAttrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(_) = self.bold {
            f.write_str(Self::BOLD)?;
            f.write_str(",")?;
        }
        if let Some(_) = self.italic {
            f.write_str(Self::ITALIC)?;
            f.write_str(",")?;
        }
        if let Some(_) = self.underline {
            f.write_str(Self::UNDERLINE)?;
            f.write_str(",")?;
        }
        if let Some(_) = self.strikethrough {
            f.write_str(Self::STRIKETHROUGH)?;
            f.write_str(",")?;
        }
        if let Some(_) = self.checked {
            f.write_str(Self::CHECKED)?;
            f.write_str(",")?;
        }
        if let Some(link) = &self.link {
            f.write_fmt(format_args!("{}(len={}),", Self::LINK, link.len()))?;
        }
        if let Some(block) = &self.block {
            f.write_fmt(format_args!("{}={},", Self::BLOCK, block))?;
        }
        if let Some(level) = self.heading {
            f.write_fmt(format_args!("{}={},", Self::HEADING, level))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardFile {
    pub blob_id: String,
    pub device_id: String,
    pub checksum: String,
    pub size_bytes: u32,
    pub name: Option<String>,

    // In case file is an image it will have width and height
    pub dimensions: Option<CardFileDimensions>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardFileDimensions {
    pub width: u32,
    pub height: u32,
}

impl CardFile {
    const TYPE: &'static str = "card_file";
    const BLOB_ID: &'static str = "blob_id";
    const DEVICE_ID: &'static str = "device_id";
    const CHECKSUM: &'static str = "checksum";
    const SIZE_BYTES: &'static str = "size_bytes";
    const NAME: &'static str = "name";
    const WIDTH: &'static str = "width";
    const HEIGHT: &'static str = "height";

    fn from_map(map: &HashMap<String, lib0::any::Any>) -> Option<Self> {
        Some(Self {
            blob_id: map.get(Self::BLOB_ID)?.to_string(),
            device_id: map.get(Self::DEVICE_ID)?.to_string(),
            checksum: map.get(Self::CHECKSUM)?.to_string(),
            size_bytes: uint_from_yrs_any(map.get(Self::SIZE_BYTES)?)?,
            name: map.get(Self::NAME).map(|n| n.to_string()),
            // TODO:
            dimensions: None,
        })
    }

    fn embed(self) -> HashMap<String, lib0::any::Any> {
        let mut map = HashMap::from([
            ("_type".into(), Self::TYPE.into()),
            (Self::BLOB_ID.into(), self.blob_id.into()),
            (Self::DEVICE_ID.into(), self.device_id.into()),
            (Self::CHECKSUM.into(), self.checksum.into()),
            (Self::NAME.into(), self.name.into()),
            (Self::SIZE_BYTES.into(), self.size_bytes.into()),
        ]);

        if let Some(d) = self.dimensions {
            map.insert(Self::WIDTH.into(), d.width.into());
            map.insert(Self::HEIGHT.into(), d.height.into());
        }

        map
    }
}

#[derive(Clone)]
pub struct FileThumbnail {
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub from_checksum: String,
}

impl FileThumbnail {
    const MIME_TYPE: &'static str = "mime_type";
    const WIDTH: &'static str = "width";
    const HEIGHT: &'static str = "height";
    const DATA: &'static str = "data";
    const FROM_CHECKSUM: &'static str = "from_checksum";

    fn from_map(txn: &yrs::Transaction, map: impl yrs::Map) -> Option<Self> {
        Some(Self {
            mime_type: map.get(txn, Self::MIME_TYPE)?.to_string(txn),
            width: uint_from_yrs(map.get(txn, Self::WIDTH)?)?,
            height: uint_from_yrs(map.get(txn, Self::HEIGHT)?)?,
            data: bytes_from_yrs(map.get(txn, Self::DATA)?)?,
            from_checksum: map.get(txn, Self::FROM_CHECKSUM)?.to_string(txn),
        })
    }
}

pub struct CardSecret {
    pub secret: Vec<u8>,
    pub algorithm: SecretAlgorithm,
}

impl CardSecret {
    const SECRET: &'static str = "secret";
    const ALGORITHM: &'static str = "alg";

    fn new(secret: Vec<u8>) -> Self {
        Self {
            secret,
            algorithm: SecretAlgorithm::ChaCha20Poly1305,
        }
    }

    fn from_map_entry(txn: &yrs::Transaction, value: yrs::types::Value) -> Option<Self> {
        value.to_ymap().and_then(|ymap| {
            let alg = uint_from_yrs(ymap.get(txn, Self::ALGORITHM)?)?;
            Some(Self {
                secret: bytes_from_yrs(ymap.get(txn, Self::SECRET)?)?,
                algorithm: SecretAlgorithm::from_i32(alg as i32)?,
            })
        })
    }

    fn embed(self) -> HashMap<String, lib0::any::Any> {
        HashMap::from([
            (Self::SECRET.into(), self.secret.into()),
            (Self::ALGORITHM.into(), (self.algorithm as i32).into()),
        ])
    }
}

pub enum CardChange {
    Insert(CardBlock),
    Remove {
        position: u32,
        len: u32,
    },
    Format {
        position: u32,
        len: u32,
        attributes: CardTextAttrs,
    },
    SetThumbnail(Option<FileThumbnail>),
    AddFileSecret {
        blob_id: String,
        value: Vec<u8>,
    },
}

impl CardChange {
    pub fn append(view: ContentView) -> Self {
        Self::Insert(CardBlock {
            position: u32::MAX,
            view,
        })
    }

    pub fn append_text(s: impl Into<String>) -> Self {
        Self::append(ContentView::Text(CardText::text(s)))
    }

    /// Append a task (single checklist item)
    pub fn append_task(s: impl Into<String>, checked: bool) -> Vec<Self> {
        vec![
            Self::append_text(s),
            // Each task ends attributed newline
            Self::append(ContentView::Text(CardText {
                value: "\n".into(),
                attrs: Some(CardTextAttrs {
                    block: Some("cl".into()),
                    checked: if checked { Some(true) } else { None },
                    ..Default::default()
                }),
            })),
        ]
    }

    /// Append a text with block attribute
    pub fn append_text_block(s: impl Into<String>, block: impl Into<String>) -> Vec<Self> {
        vec![
            Self::append_text(s),
            // Block attribute is added only on the newline
            Self::append(ContentView::Text(CardText {
                value: "\n".into(),
                attrs: Some(CardTextAttrs {
                    block: Some(block.into()),
                    ..Default::default()
                }),
            })),
        ]
    }

    /// Append a text with heading attribute
    pub fn append_text_heading(s: impl Into<String>, level: u8) -> Vec<Self> {
        vec![
            Self::append_text(s),
            // Heading attribute is added only on the newline
            Self::append(ContentView::Text(CardText {
                value: "\n".into(),
                attrs: Some(CardTextAttrs {
                    heading: Some(level),
                    ..Default::default()
                }),
            })),
        ]
    }
}

impl std::fmt::Display for CardChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CardChange::")?;
        match self {
            Self::Insert(CardBlock { position, view }) => {
                f.write_fmt(format_args!("Insert(pos={} ", position))?;
                match view {
                    ContentView::Text(text) => {
                        if let Some(a) = &text.attrs {
                            f.write_fmt(format_args!(
                                "text_bytes={} attrs={}",
                                text.value.len(),
                                a
                            ))?;
                        } else {
                            f.write_fmt(format_args!(
                                "text_bytes={} attrs=None",
                                text.value.len()
                            ))?;
                        }
                    }
                    ContentView::File(file) => {
                        f.write_fmt(format_args!("blob_id={}", file.blob_id))?;
                    }
                }
                f.write_str(")")?;
            }
            Self::Remove { position, len } => {
                f.write_fmt(format_args!("Remove(from={} len={})", position, len))?
            }
            CardChange::Format { position, len, .. } => {
                f.write_fmt(format_args!("Format(from={} len={})", position, len))?
            }
            Self::SetThumbnail(thumb) => {
                f.write_fmt(format_args!("SetThumbnail({})", thumb.is_some()))?
            }
            Self::AddFileSecret { blob_id, .. } => {
                f.write_fmt(format_args!("AddFileSecret(blob_id={})", blob_id))?
            }
        };
        Ok(())
    }
}

pub enum CardLabelsChange {
    AddLabel { label_id: String },
    RemoveLabel { label_id: String },
}

impl std::fmt::Display for CardLabelsChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CardLabelsChange::")?;
        match self {
            Self::AddLabel { label_id } => {
                f.write_fmt(format_args!("AddLabel(id={})", label_id))?
            }
            Self::RemoveLabel { label_id } => {
                f.write_fmt(format_args!("RemoveLabel(id={})", label_id))?
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bolik_proto::sync::doc_payload::DocSchema;
    use chrono::Utc;
    use yrs::{
        types::text::YChange, updates::decoder::Decode, GetString, ReadTxn, StateVector, Text,
        Transact, Update,
    };

    use crate::{
        documents::{DbDocRow, DbDocRowMeta},
        timeline::card::{CardBlock, CardChange, CardText, CardTextAttrs, ContentView},
    };

    use super::CardView;

    fn dummy_db_meta() -> DbDocRowMeta {
        DbDocRowMeta {
            id: "doc-1".to_string(),
            author_device_id: "dev-A".to_string(),
            counter: 0,
            created_at: Utc::now(),
            edited_at: Utc::now(),
            schema: DocSchema::CardV1 as i32,
        }
    }

    fn get_texts(view: &CardView) -> Vec<&str> {
        view.blocks
            .iter()
            .map(|b| {
                if let ContentView::Text(t) = &b.view {
                    t.value.as_ref()
                } else {
                    panic!("Expected Text but got {:?}", b.view);
                }
            })
            .collect()
    }

    fn build_row(doc: yrs::Doc) -> DbDocRow {
        DbDocRow {
            meta: dummy_db_meta(),
            yrs: doc,
            acl: yrs::Doc::new(),
        }
    }

    fn append_text(doc: &yrs::Doc, t: &str) {
        CardView::edit(&doc, vec![CardChange::append_text(t)]);
    }

    #[test]
    fn test_card_text_edit() {
        // Set original text
        let doc = CardView::init(1);
        append_text(&doc, "Hello");
        let (view, doc) = CardView::from_db(build_row(doc), None);
        assert_eq!(1, view.blocks.len());
        assert_eq!(vec!["Hello"], get_texts(&view));

        // Add to text
        append_text(&doc, " world!");
        let (view, doc) = CardView::from_db(build_row(doc), None);
        assert_eq!(vec!["Hello world!"], get_texts(&view));

        // Remove and add to text
        CardView::edit(
            &doc,
            vec![
                CardChange::Remove {
                    position: 1,
                    len: 4,
                },
                CardChange::Insert(CardBlock {
                    position: 1,
                    view: ContentView::Text(CardText {
                        value: "i".into(),
                        attrs: None,
                    }),
                }),
            ],
        );
        let (view, _) = CardView::from_db(build_row(doc), None);
        assert_eq!(vec!["Hi world!"], get_texts(&view));
    }

    #[test]
    fn test_card_text_edit_ru() {
        // Verify doc works with Russian and Emojis
        let doc = CardView::init(1);
        append_text(&doc, "Привет\n");
        CardView::edit(&doc, CardChange::append_text_block("Один", "ul"));

        // Insert text
        CardView::edit(
            &doc,
            vec![CardChange::Insert(CardBlock {
                position: 3,
                view: ContentView::Text(CardText {
                    value: " звезда ★ ".into(),
                    attrs: None,
                }),
            })],
        );

        let (view, doc) = CardView::from_db(build_row(doc), None);

        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "При звезда ★ вет\nОдин");
            assert_eq!(t.value.chars().count(), 21);
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        assert_eq!(21, view.blocks[1].position);
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.block, Some("ul".to_string()));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }

        // Remove text
        CardView::edit(
            &doc,
            vec![CardChange::Remove {
                position: 10,
                len: 6,
            }],
        );
        let (view, doc) = CardView::from_db(build_row(doc), None);

        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "При звезда\nОдин");
            assert_eq!(t.value.chars().count(), 15);
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        assert_eq!(15, view.blocks[1].position);
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.block, Some("ul".to_string()));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }

        // Format text
        CardView::edit(
            &doc,
            vec![CardChange::Format {
                position: 4,
                len: 6,
                attributes: CardTextAttrs {
                    bold: Some(true),
                    ..Default::default()
                },
            }],
        );
        let (view, _doc) = CardView::from_db(build_row(doc), None);

        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "При ");
            assert_eq!(t.value.chars().count(), 4);
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        assert_eq!(4, view.blocks[1].position);
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "звезда");
            assert_eq!(t.value.chars().count(), 6);
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.bold, Some(true));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }
        assert_eq!(10, view.blocks[2].position);
        if let ContentView::Text(t) = &view.blocks[2].view {
            assert_eq!(t.value, "\nОдин");
            assert_eq!(t.value.chars().count(), 5);
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[2]);
        }
        assert_eq!(15, view.blocks[3].position);
        if let ContentView::Text(t) = &view.blocks[3].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.block, Some("ul".to_string()));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[3]);
        }
    }

    #[test]
    fn test_card_text_format() {
        // Set original text
        let doc = CardView::init(1);
        append_text(&doc, "One\nTwo\n");

        // Apply formatting
        CardView::edit(
            &doc,
            vec![
                CardChange::Format {
                    position: 0,
                    len: 3,
                    attributes: CardTextAttrs {
                        bold: Some(true),
                        ..Default::default()
                    },
                },
                CardChange::Format {
                    position: 7,
                    len: 1,
                    attributes: CardTextAttrs {
                        checked: Some(true),
                        block: Some("cl".into()),
                        ..Default::default()
                    },
                },
            ],
        );

        let (view, doc) = CardView::from_db(build_row(doc), None);
        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "One");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.bold, Some(true));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "\nTwo");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }
        if let ContentView::Text(t) = &view.blocks[2].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.block, Some("cl".into()));
            assert_eq!(attrs.checked, Some(true));
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[2]);
        }

        // Undo formatting partially
        CardView::edit(
            &doc,
            vec![
                CardChange::Format {
                    position: 0,
                    len: 3,
                    attributes: CardTextAttrs {
                        bold: Some(false),
                        ..Default::default()
                    },
                },
                CardChange::Format {
                    position: 7,
                    len: 1,
                    attributes: CardTextAttrs {
                        checked: Some(false),
                        ..Default::default()
                    },
                },
            ],
        );

        let (view, doc) = CardView::from_db(build_row(doc), None);
        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "One");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "\nTwo");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }
        if let ContentView::Text(t) = &view.blocks[2].view {
            assert_eq!(t.value, "\n");
            let attrs = t.attrs.clone().unwrap();
            assert_eq!(attrs.block, Some("cl".into()));
            assert_eq!(attrs.checked, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[2]);
        }

        // Undo formatting fully
        CardView::edit(
            &doc,
            vec![CardChange::Format {
                position: 7,
                len: 1,
                attributes: CardTextAttrs {
                    block: Some("".into()),
                    ..Default::default()
                },
            }],
        );

        let (view, _doc) = CardView::from_db(build_row(doc), None);
        if let ContentView::Text(t) = &view.blocks[0].view {
            assert_eq!(t.value, "One");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[0]);
        }
        if let ContentView::Text(t) = &view.blocks[1].view {
            assert_eq!(t.value, "\nTwo\n");
            assert_eq!(t.attrs, None);
        } else {
            panic!("Expected ContentView::Text but got {:?}", view.blocks[1]);
        }
    }

    #[test]
    fn test_yrs_text_embeds() {
        use std::collections::HashMap;
        use std::rc::Rc;

        use yrs::types::Attrs;

        let doc = &yrs::Doc::new();

        let bold_key: Rc<str> = "bold".into();
        let italic_key: Rc<str> = "italic".into();

        let ytext = doc.get_or_insert_text("text");
        let txn = &mut doc.transact_mut();

        // Append normal text
        ytext.push(txn, "Hello ");
        // Append formatted text
        ytext.insert_with_attributes(
            txn,
            ytext.len(txn),
            "world!",
            Attrs::from([(bold_key, true.into())]),
        );
        // We need to insert without attributes when appending to the text
        // that has attributes. Otherwise bold attribute will remain.
        ytext.insert_with_attributes(txn, ytext.len(txn), " (Tom) ", Attrs::from([]));
        // Format existing text
        ytext.format(txn, 8, 2, Attrs::from([(italic_key, true.into())]));

        // Append an embedded block
        let embed_map: HashMap<String, lib0::any::Any> = HashMap::from([
            ("name".into(), "hello.txt".into()),
            ("size".into(), (10u32).into()),
        ]);
        ytext.insert_embed_with_attributes(
            txn,
            ytext.len(txn),
            embed_map.clone().into(),
            Attrs::new(),
        );

        ytext.push(txn, "Fin.");

        let update = txn.encode_state_as_update_v2(&StateVector::default());
        let doc2 = yrs::Doc::new();
        let _ytext2 = doc2.get_or_insert_text("text");
        let txn2 = &mut doc2.transact_mut();
        let u = Update::decode_v2(&update).unwrap();
        txn2.apply_update(u);

        for diff in ytext.diff(txn, YChange::identity) {
            println!("Diff {:?}", diff);
        }

        // Move embeds around
        ytext.remove_range(txn, 19, 1);
        println!("\nAfter removal:");
        for diff in ytext.diff(txn, YChange::identity) {
            println!("Diff {:?}", diff);
        }
        ytext.insert_embed_with_attributes(txn, 12, embed_map.into(), Attrs::new());
        let mut count = 0;
        println!("\nAfter insertion:");
        for diff in ytext.diff(txn, YChange::identity) {
            println!("Diff {:?}", diff);
            match diff.insert {
                yrs::types::Value::Any(lib0::any::Any::String(s)) => {
                    count += s.len();
                }
                _ => {
                    count += 1;
                }
            }
        }

        println!("{}", ytext.get_string(txn));
        println!("count={} len={}", count, ytext.len(txn));

        // assert!(false);
    }
}

// fn print_text(text: &impl Text, txn: &impl ReadTxn) {
//     println!("##################################################");
//     println!("Doc:");
//     let mut debug_text = String::new();
//     let mut count = 0;
//     for diff in text.diff(txn, YChange::identity) {
//         match diff.insert {
//             yrs::types::Value::Any(lib0::any::Any::String(s)) => {
//                 let attrs: CardTextAttrs = diff.attributes.map(|a| a.into()).unwrap_or_default();
//                 for c in s.chars() {
//                     if c == '\n' {
//                         debug_text.push_str(&format!("{}: ⮰ {}\n", count, attrs));
//                     } else {
//                         debug_text.push_str(&format!("{}: {} ", count, c));
//                     }
//                     count += 1;
//                 }
//             }
//             _ => {
//                 debug_text.push_str(&format!("{}: ?", count));
//                 count += 1;
//             }
//         }
//     }
//     println!("{}", debug_text);
//     println!("##################################################");
// }
