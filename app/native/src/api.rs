use std::collections::HashMap;
use std::{io::Write, sync::Mutex};

use anyhow::{bail, Result};

pub use bolik_sdk::account::{AccContact, AccDevice, AccLabel};
pub use bolik_sdk::timeline::acl_doc::AclRights;
pub use bolik_sdk::timeline::card::CardLabel;
use bolik_sdk::timeline::card::CardLabelsChange;
pub use bolik_sdk::timeline::card::CardTextAttrs;
pub use bolik_sdk::ImportResult;
use bolik_sdk::{account, key_from_slice, output, start_runtime, timeline, DefaultSdk};
use bolik_sdk::{MoveToBinScope, BIN_LABEL_ID};
use chrono::{DateTime, Utc};
use flutter_rust_bridge::handler::{self, ErrorHandler, ReportDartErrorHandler};
use flutter_rust_bridge::support::WireSyncReturn;
use flutter_rust_bridge::{frb, StreamSink, SyncReturn, ZeroCopyBuffer};
use tokio::{runtime::Runtime, sync::oneshot};
use tracing_subscriber::{fmt::MakeWriter, EnvFilter};

use crate::qr;

static BOLIK_SDK: Mutex<Option<DefaultSdk>> = Mutex::new(None);
static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

#[derive(Copy, Clone)]
pub struct MyErrorHandler(ReportDartErrorHandler);

impl ErrorHandler for MyErrorHandler {
    fn handle_error(&self, port: i64, error: handler::Error) {
        // Here I can handle the error
        self.0.handle_error(port, error)
    }

    fn handle_error_sync(&self, error: handler::Error) -> WireSyncReturn {
        self.0.handle_error_sync(error)
    }
}

struct LogSink {
    sink: StreamSink<String>,
}

impl<'a> Write for &'a LogSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(buf).to_string();
        self.sink.add(line);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogSink {
    type Writer = &'a LogSink;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}

pub fn setup_logs(sink: StreamSink<String>) -> Result<()> {
    let log_sink = LogSink { sink };

    // Subscribe to tracing events and publish them to the UI
    if let Err(err) = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(EnvFilter::new("info,bolik_sdk=trace"))
        .with_writer(log_sink)
        .try_init()
    {
        bail!("{}", err);
    }
    Ok(())
}

/// Initialize native code and subscribe to the events that native module dispatches.
pub fn setup(
    sink: StreamSink<OutputEvent>,
    app_support_dir: String,
    files_dir: String,
    device_name: String,
) -> Result<()> {
    // let db_encryption_key = generate_db_key();
    let db_encryption_key = key_from_slice(b"an example very very secret key.")?;
    let rt = start_runtime()?;

    let sdk = rt.block_on(bolik_sdk::run(
        app_support_dir,
        files_dir,
        device_name,
        db_encryption_key,
    ))?;

    let mut events = sdk.broadcast_subscribe();

    match sdk.get_account() {
        Some(view) => {
            sink.add(OutputEvent::PostAccount(PostAccountPhase {
                acc_view: view.into(),
            }));
        }
        None => {
            sink.add(OutputEvent::PreAccount);
        }
    }

    let handle = rt.handle().clone();
    {
        *BOLIK_SDK.lock().expect("Set sdk") = Some(sdk);
        *RUNTIME.lock().expect("Set runtime") = Some(rt);
    }

    let (tx, rx) = oneshot::channel();
    // We are spawning an async task from a thread that is not managed by
    // Tokio runtime. For this to work we need to enter the handle.
    // Ref: https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html#method.current
    let _guard = handle.enter();
    tokio::spawn(async move {
        while let Ok(e) = events.recv().await {
            sink.add(e.into());
        }
        let _ = tx.send(());
    });

    let _ = rx.blocking_recv();
    Ok(())
}

fn with_runtime<R>(cb: impl FnOnce(&Runtime, &mut DefaultSdk) -> Result<R>) -> Result<R> {
    let mut sdk_guard = BOLIK_SDK.lock().expect("Get sdk");
    let sdk = sdk_guard.as_mut().expect("Sdk present");

    // We are calling async sdk methods from a thread that is not managed by
    // Tokio runtime. For this to work we need to enter the handle.
    // Ref: https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html#method.current
    let mut rt_guard = RUNTIME.lock().expect("Get runtime");
    let rt = rt_guard.as_mut().expect("Runtime present");
    let _guard = rt.enter();
    cb(rt, sdk)
}

fn with_sdk<R>(cb: impl FnOnce(&mut DefaultSdk) -> Result<R>) -> Result<R> {
    with_runtime(|_rt, sdk| cb(sdk))
}

pub fn timeline_days(label_ids: Vec<String>) -> Result<Vec<String>> {
    with_sdk(|sdk| sdk.timeline_days(label_ids))
}

pub fn timeline_by_day(day: String, label_ids: Vec<String>) -> Result<TimelineDay> {
    let timeline_day = with_sdk(|sdk| sdk.timeline_by_day(&day, label_ids))?;
    Ok(timeline_day.into())
}

pub fn get_device_share() -> Result<String> {
    let share = with_sdk(|sdk| sdk.get_device_share())?;
    Ok(share)
}

pub fn link_device(share: String) -> Result<String> {
    let linked_device_name = with_runtime(|rt, sdk| rt.block_on(sdk.link_device(&share)))?;
    Ok(linked_device_name)
}

pub fn remove_device(remove_id: String) -> Result<AccView> {
    let view = with_sdk(|sdk| sdk.remove_device(&remove_id))?;
    Ok(view.into())
}

pub fn sync() {
    let _ = with_sdk(|sdk| Ok(sdk.sync()));
}

pub fn create_account(name: Option<String>) -> Result<AccView> {
    let view = with_sdk(|sdk| sdk.create_account(name))?;
    Ok(view.into())
}

pub fn save_file(card_id: String, path: String) -> Result<CardFile> {
    let card_file = with_sdk(|sdk| sdk.save_file(&card_id, path))?;
    Ok(card_file.into())
}

pub fn edit_card(card_id: String, changes: Vec<CardChange>) -> Result<CardView> {
    let card_changes: Vec<timeline::card::CardChange> = changes
        .into_iter()
        .map(|change| match change {
            CardChange::Insert(b) => timeline::card::CardChange::Insert(b.into()),
            CardChange::Remove { position, len } => {
                timeline::card::CardChange::Remove { position, len }
            }
            CardChange::Format {
                position,
                len,
                attributes,
            } => timeline::card::CardChange::Format {
                position,
                len,
                attributes,
            },
        })
        .collect();
    let card = with_sdk(|sdk| sdk.edit_card(&card_id, card_changes))?;
    Ok(card.into())
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
}

pub fn close_card(card_id: String) -> Result<()> {
    with_sdk(|sdk| sdk.close_card(&card_id))
}

pub fn get_card(card_id: String) -> Result<CardView> {
    let card = with_sdk(|sdk| sdk.get_card(&card_id))?;
    Ok(card.into())
}

pub fn create_card() -> Result<CardView> {
    let card = with_sdk(|sdk| sdk.create_card())?;
    Ok(card.into())
}

pub fn move_card_to_bin(card_id: String) -> Result<()> {
    with_sdk(|sdk| sdk.move_card_to_bin(&card_id, MoveToBinScope::ThisAccount))?;
    Ok(())
}

pub fn restore_from_bin(card_id: String) -> Result<CardView> {
    let card = with_sdk(|sdk| sdk.restore_from_bin(&card_id))?;
    Ok(card.into())
}

pub fn empty_bin() -> Result<()> {
    with_sdk(|sdk| sdk.empty_bin())
}

pub fn move_card_to_bin_all(card_id: String) -> Result<()> {
    with_sdk(|sdk| sdk.move_card_to_bin(&card_id, MoveToBinScope::All))?;
    Ok(())
}

pub fn add_card_label(card_id: String, label_id: String) -> Result<CardView> {
    let card = with_sdk(|sdk| {
        sdk.edit_card_labels(&card_id, vec![CardLabelsChange::AddLabel { label_id }])
    })?;
    Ok(card.into())
}

pub fn remove_card_label(card_id: String, label_id: String) -> Result<CardView> {
    let card = with_sdk(|sdk| {
        sdk.edit_card_labels(&card_id, vec![CardLabelsChange::RemoveLabel { label_id }])
    })?;
    Ok(card.into())
}

pub fn get_file_path(blob_id: String) -> Result<Option<String>> {
    let path = with_sdk(|sdk| sdk.get_file_path(&blob_id))?;
    Ok(path)
}

pub fn download_file(
    card_id: String,
    blob_id: String,
    device_id: String,
) -> Result<DownloadResult> {
    let res = with_sdk(|sdk| sdk.download_blob(&card_id, &blob_id, &device_id))?;
    Ok(res.into())
}

pub fn get_account() -> Result<Option<AccView>> {
    let res = with_sdk(|sdk| Ok(sdk.get_account()))?;
    Ok(res.map(|v| v.into()))
}

pub fn account_group() -> Result<SecretGroupStatus> {
    let g = with_sdk(|sdk| sdk.account_group())?;
    Ok(g.into())
}

pub fn edit_name(name: String) -> Result<AccView> {
    let res = with_sdk(|sdk| sdk.edit_name(name))?;
    Ok(res.into())
}

pub fn add_contact(contact: AccContact) -> Result<AccView> {
    let view = with_runtime(|rt, sdk| rt.block_on(sdk.add_contact(contact)))?;
    Ok(view.into())
}

pub fn edit_contact_name(account_id: String, name: String) -> Result<AccView> {
    let res = with_sdk(|sdk| sdk.edit_contact_name(&account_id, &name))?;
    Ok(res.into())
}

pub fn create_acc_label(name: String) -> Result<CreateAccLabelResult> {
    let res = with_sdk(|sdk| sdk.create_acc_label(name))?;
    Ok(res.into())
}

pub fn delete_acc_label(label_id: String) -> Result<AccView> {
    let view = with_sdk(|sdk| sdk.delete_acc_label(&label_id))?;
    Ok(view.into())
}

pub fn edit_collaborators(card_id: String, changes: Vec<CollaboratorChange>) -> Result<CardView> {
    let changed: HashMap<_, _> = changes
        .into_iter()
        .map(|c| {
            if c.removed {
                (c.account_id, None)
            } else {
                (c.account_id, Some(c.rights))
            }
        })
        .collect();
    let card = with_sdk(|sdk| sdk.edit_collaborators(&card_id, changed))?;
    Ok(card.into())
}

pub fn export_data(out_dir: String) -> Result<()> {
    with_runtime(|rt, sdk| rt.block_on(sdk.export_cards_to_dir(out_dir)))?;
    Ok(())
}

pub fn import_data(in_dir: String) -> Result<ImportResult> {
    let res = with_sdk(|sdk| sdk.import_data(in_dir))?;
    Ok(res)
}

pub fn get_current_device_id() -> Result<SyncReturn<String>> {
    let id = with_sdk(|sdk| Ok(sdk.get_device_id().to_string()))?;
    Ok(SyncReturn(id))
}

pub fn get_deleted_label_id() -> SyncReturn<String> {
    SyncReturn(BIN_LABEL_ID.to_string())
}

pub fn scan_qr_code(
    width: u32,
    height: u32,
    format: PixelFormat,
    buf: Vec<u8>,
) -> Result<Option<String>> {
    qr::scan_qr_code(width, height, format, buf)
}

pub fn list_profiles() -> Result<Vec<ProfileView>> {
    let profiles = with_sdk(|sdk| sdk.list_profiles())?;
    Ok(profiles.into_iter().map(|p| p.into()).collect())
}

pub fn accept_notification(id: String) -> Result<()> {
    with_runtime(|rt, sdk| rt.block_on(sdk.accept_notification(&id)))?;
    Ok(())
}

pub fn ignore_notification(id: String) -> Result<()> {
    with_sdk(|sdk| sdk.ignore_notification(&id))?;
    Ok(())
}

pub fn list_notification_ids() -> Result<Vec<String>> {
    let ids = with_sdk(|sdk| sdk.list_notification_ids())?;
    Ok(ids)
}

/// Log out from the account. You must re-initialize SDK after calling this function.
pub fn logout() {
    let mut sdk_guard = BOLIK_SDK.lock().expect("Get sdk");
    let sdk = sdk_guard.take().expect("SDK present");
    let mut rt_guard = RUNTIME.lock().expect("Get runtime");
    let rt = rt_guard.take().expect("Runtime present");
    let _guard = rt.enter();

    rt.block_on(sdk.logout());
}

// Flutter Rust bridge mirror seems to have quite a few limitations
// or I don't fully understand how to use it.
// Hence for now I just duplicate the types below.

pub enum OutputEvent {
    Synced,
    SyncFailed,
    TimelineUpdated,
    PreAccount,
    PostAccount(PostAccountPhase),
    DeviceAdded(DeviceAddedEvent),
    DocUpdated(DocUpdatedEvent),
    DownloadCompleted { blob_id: String, path: String },
    DownloadFailed { blob_id: String },
    AccUpdated(AccView),
    Notification { id: String },
    NotificationsUpdated,
    LogOut,
}

impl From<output::OutputEvent> for OutputEvent {
    fn from(event: output::OutputEvent) -> Self {
        match event {
            output::OutputEvent::Synced => Self::Synced,
            output::OutputEvent::SyncFailed => Self::SyncFailed,
            output::OutputEvent::TimelineUpdated => Self::TimelineUpdated,
            output::OutputEvent::DeviceAdded { device_name } => {
                Self::DeviceAdded(DeviceAddedEvent { device_name })
            }
            output::OutputEvent::ConnectedToAccount { view } => {
                Self::PostAccount(PostAccountPhase {
                    acc_view: view.into(),
                })
            }
            output::OutputEvent::DocUpdated { doc_id } => {
                Self::DocUpdated(DocUpdatedEvent { doc_id })
            }
            output::OutputEvent::DownloadCompleted { blob_id, path, .. } => {
                Self::DownloadCompleted { blob_id, path }
            }
            output::OutputEvent::DownloadFailed { blob_id } => Self::DownloadFailed { blob_id },
            output::OutputEvent::AccUpdated { view } => Self::AccUpdated(view.into()),
            output::OutputEvent::LogOut => Self::LogOut,
            output::OutputEvent::Notification(n) => Self::Notification { id: n.id() },
            output::OutputEvent::NotificationsUpdated => Self::NotificationsUpdated,
        }
    }
}

pub struct PostAccountPhase {
    pub acc_view: AccView,
}

pub struct DeviceAddedEvent {
    pub device_name: String,
}

pub struct TimelineDay {
    pub day: String,
    pub cards: Vec<CardView>,
}

impl From<timeline::TimelineDay> for TimelineDay {
    fn from(item: timeline::TimelineDay) -> Self {
        Self {
            day: item.day,
            cards: item.cards.into_iter().map(|c| c.into()).collect(),
        }
    }
}

pub struct AccView {
    pub id: String,
    pub created_at_sec: i64,
    pub name: String,
    pub contacts: Vec<AccContact>,
    pub labels: Vec<AccLabel>,
    pub devices: Vec<AccDevice>,
}

impl From<account::AccView> for AccView {
    fn from(view: account::AccView) -> Self {
        Self {
            id: view.id,
            created_at_sec: view.created_at.timestamp(),
            name: view.name,
            contacts: view.contacts,
            labels: view.labels,
            devices: view.devices,
        }
    }
}

pub struct CardView {
    pub id: String,
    pub created_at_sec: i64,
    pub edited_at_sec: i64,
    pub acl: AclDoc,
    pub blocks: Vec<CardBlock>,
    pub labels: Vec<CardLabel>,
    pub thumbnail: Option<FileThumbnail>,
}

impl From<timeline::card::CardView> for CardView {
    fn from(c: timeline::card::CardView) -> Self {
        Self {
            id: c.id,
            created_at_sec: c.created_at.timestamp(),
            edited_at_sec: c.edited_at.timestamp(),
            acl: c.acl.into(),
            blocks: c.blocks.into_iter().map(|c| c.into()).collect(),
            labels: c.labels.into_iter().collect(),
            thumbnail: c.thumbnail.map(|t| t.into()),
        }
    }
}

pub struct CardBlock {
    pub position: u32,
    pub view: Box<ContentView>,
}

impl From<timeline::card::CardBlock> for CardBlock {
    fn from(c: timeline::card::CardBlock) -> Self {
        Self {
            position: c.position,
            view: Box::new(c.view.into()),
        }
    }
}

impl Into<timeline::card::CardBlock> for CardBlock {
    fn into(self) -> timeline::card::CardBlock {
        timeline::card::CardBlock {
            position: self.position,
            view: match *self.view {
                ContentView::Text(t) => {
                    timeline::card::ContentView::Text(timeline::card::CardText {
                        value: t.value,
                        attrs: t.attrs,
                    })
                }
                ContentView::File(f) => {
                    timeline::card::ContentView::File(timeline::card::CardFile {
                        blob_id: f.blob_id,
                        device_id: f.device_id,
                        checksum: f.checksum,
                        size_bytes: f.size_bytes,
                        name: f.name,
                        dimensions: None,
                    })
                }
            },
        }
    }
}

pub enum ContentView {
    Text(CardText),
    File(CardFile),
}

impl From<timeline::card::ContentView> for ContentView {
    fn from(v: timeline::card::ContentView) -> Self {
        match v {
            timeline::card::ContentView::Text(t) => Self::Text(t.into()),
            timeline::card::ContentView::File(f) => Self::File(f.into()),
        }
    }
}

#[frb(mirror(CardLabel))]
pub struct _CardLabel {
    pub id: String,
    pub added_at: DateTime<Utc>,
}

pub struct CardText {
    pub value: String,
    pub attrs: Option<CardTextAttrs>,
}

impl From<timeline::card::CardText> for CardText {
    fn from(t: timeline::card::CardText) -> Self {
        Self {
            value: t.value,
            attrs: t.attrs,
        }
    }
}

#[frb(mirror(CardTextAttrs))]
pub struct _CardTextAttrs {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
    link: Option<String>,
    checked: Option<bool>,
    heading: Option<u8>,
    block: Option<String>,
}

pub struct CardFile {
    pub blob_id: String,
    pub device_id: String,
    pub checksum: String,
    pub size_bytes: u32,
    pub name: Option<String>,
}

impl From<timeline::card::CardFile> for CardFile {
    fn from(f: timeline::card::CardFile) -> Self {
        Self {
            blob_id: f.blob_id,
            device_id: f.device_id,
            checksum: f.checksum,
            size_bytes: f.size_bytes,
            name: f.name,
        }
    }
}

pub struct FileThumbnail {
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub data: ZeroCopyBuffer<Vec<u8>>,
}

impl From<timeline::card::FileThumbnail> for FileThumbnail {
    fn from(thumb: timeline::card::FileThumbnail) -> Self {
        Self {
            mime_type: thumb.mime_type,
            width: thumb.width,
            height: thumb.height,
            data: ZeroCopyBuffer(thumb.data),
        }
    }
}

pub struct DocUpdatedEvent {
    pub doc_id: String,
}

pub struct DownloadResult {
    // Will be present if file is already downloaded
    pub path: Option<String>,
    // Will be true when client started downloading the file
    pub download_started: bool,
}

impl From<bolik_sdk::DownloadResult> for DownloadResult {
    fn from(r: bolik_sdk::DownloadResult) -> Self {
        Self {
            path: r.path,
            download_started: r.download_started,
        }
    }
}

#[frb(mirror(AccContact))]
pub struct _AccContact {
    pub account_id: String,
    pub name: String,
}

#[frb(mirror(AccLabel))]
pub struct _AccLabel {
    pub id: String,
    pub name: String,
}

#[frb(mirror(AccDevice))]
pub struct _AccDevice {
    pub id: String,
    pub name: String,
    pub added_at: DateTime<Utc>,
}

pub struct AclDoc {
    pub accounts: Vec<AclEntry>,
}

impl From<timeline::acl_doc::AclDoc> for AclDoc {
    fn from(a: timeline::acl_doc::AclDoc) -> Self {
        Self {
            accounts: a
                .accounts
                .into_iter()
                .map(|(account_id, rights)| AclEntry { account_id, rights })
                .collect(),
        }
    }
}

pub struct AclEntry {
    pub account_id: String,
    pub rights: AclRights,
}

#[frb(mirror(AclRights))]
pub enum _AclRights {
    Read,
    Write,
    Admin,
}

pub struct CreateAccLabelResult {
    pub view: AccView,
    pub label: AccLabel,
}

impl From<bolik_sdk::CreateAccLabelResult> for CreateAccLabelResult {
    fn from(value: bolik_sdk::CreateAccLabelResult) -> Self {
        Self {
            view: value.view.into(),
            label: value.label,
        }
    }
}

pub struct SecretGroupStatus {
    pub authentication_secret: Vec<u8>,
    pub devices: Vec<String>,
}

impl From<bolik_sdk::SecretGroupStatus> for SecretGroupStatus {
    fn from(s: bolik_sdk::SecretGroupStatus) -> Self {
        Self {
            authentication_secret: s.authentication_secret,
            devices: s.devices,
        }
    }
}

pub struct ProfileView {
    pub account_id: String,
    pub name: String,
}

impl From<bolik_sdk::account::ProfileView> for ProfileView {
    fn from(s: bolik_sdk::account::ProfileView) -> Self {
        Self {
            account_id: s.account_id,
            name: s.name,
        }
    }
}

pub struct CollaboratorChange {
    pub account_id: String,
    pub rights: AclRights,
    // rights: Option<AclRights>,
    // TODO:
    // Once https://github.com/fzyzcjy/flutter_rust_bridge/pull/949 lands, I can remove this field
    // and make rights field optional. Atm, flutter rust bridge doesn't like Options.
    pub removed: bool,
}

pub enum PixelFormat {
    /// Blue-Green-Red-Alpha (8bit for each): https://developer.apple.com/documentation/corevideo/kcvpixelformattype_32bgra
    BGRA8888,
    /// Compressed JPEG: https://developer.android.com/reference/android/graphics/ImageFormat#JPEG
    JPEG,
}

#[frb(mirror(ImportResult))]
pub struct _ImportResult {
    pub imported: u32,
    pub duplicates: Vec<String>,
    pub failed: Vec<String>,
}
