use crate::account::{AccNotification, AccView};

#[derive(Debug, Clone, PartialEq)]
pub enum OutputEvent {
    Synced,
    SyncFailed,
    TimelineUpdated,
    DeviceAdded {
        device_name: String,
    },
    ConnectedToAccount {
        view: AccView,
    },
    AccUpdated {
        view: AccView,
    },
    DocUpdated {
        doc_id: String,
    },
    DownloadCompleted {
        blob_id: String,
        device_id: String,
        path: String,
    },
    DownloadFailed {
        blob_id: String,
    },
    Notification(AccNotification),
    NotificationsUpdated,
    LogOut,
}
