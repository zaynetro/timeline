// use crate::{
//     account::{AccLabel, DeviceView},
//     timeline_item::{MediaContentView, TimelineItem, TimelineItemPreview},
// };

// #[derive(Debug)]
// pub enum InputEvent {
//     CreateTimelineItem {
//         respond: tokio::sync::oneshot::Sender<TimelineItem>,
//     },
//     GetTimelineItem {
//         item_id: String,
//         respond: tokio::sync::oneshot::Sender<Option<TimelineItem>>,
//     },
//     CreateAccountLabel {
//         label: AccLabel,
//     },
//     AddItemLabel {
//         item_id: String,
//         label_id: String,
//     },
//     RemoveItemLabel {
//         item_id: String,
//         label_id: String,
//     },
//     AddTextBlock {
//         item_id: String,
//         content_id: String,
//         text: Option<String>,
//     },
//     EditText {
//         item_id: String,
//         content_id: String,
//         new_value: String,
//     },
//     AddMediaBlock {
//         item_id: String,
//         content_id: String,
//         media: MediaContentView,
//     },
//     RemoveBlock {
//         item_id: String,
//         index: u32,
//     },
//     CloseTimelineItem {
//         item_id: String,
//     },
//     TimelineDays {
//         respond: tokio::sync::oneshot::Sender<Vec<String>>,
//     },
//     TimelineByPrevDay {
//         prev_day: Option<String>,
//         respond: tokio::sync::oneshot::Sender<Vec<TimelineItemPreview>>,
//     },
//     DeviceShare {
//         respond: tokio::sync::oneshot::Sender<String>,
//     },
//     LinkToDevice {
//         ed25519_key: Ed25519PublicKey,
//         curve25519_key: Curve25519PublicKey,
//         one_time_key: Curve25519PublicKey,
//     },
//     CreateAccount,
// }
