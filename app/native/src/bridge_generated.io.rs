use super::*;
// Section: wire functions

#[no_mangle]
pub extern "C" fn wire_setup_logs(port_: i64) {
    wire_setup_logs_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_setup(
    port_: i64,
    app_support_dir: *mut wire_uint_8_list,
    files_dir: *mut wire_uint_8_list,
    device_name: *mut wire_uint_8_list,
) {
    wire_setup_impl(port_, app_support_dir, files_dir, device_name)
}

#[no_mangle]
pub extern "C" fn wire_timeline_days(port_: i64, label_ids: *mut wire_StringList) {
    wire_timeline_days_impl(port_, label_ids)
}

#[no_mangle]
pub extern "C" fn wire_timeline_by_day(
    port_: i64,
    day: *mut wire_uint_8_list,
    label_ids: *mut wire_StringList,
) {
    wire_timeline_by_day_impl(port_, day, label_ids)
}

#[no_mangle]
pub extern "C" fn wire_get_device_share(port_: i64) {
    wire_get_device_share_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_link_device(port_: i64, share: *mut wire_uint_8_list) {
    wire_link_device_impl(port_, share)
}

#[no_mangle]
pub extern "C" fn wire_remove_device(port_: i64, remove_id: *mut wire_uint_8_list) {
    wire_remove_device_impl(port_, remove_id)
}

#[no_mangle]
pub extern "C" fn wire_sync(port_: i64) {
    wire_sync_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_create_account(port_: i64, name: *mut wire_uint_8_list) {
    wire_create_account_impl(port_, name)
}

#[no_mangle]
pub extern "C" fn wire_save_file(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    path: *mut wire_uint_8_list,
) {
    wire_save_file_impl(port_, card_id, path)
}

#[no_mangle]
pub extern "C" fn wire_edit_card(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    changes: *mut wire_list_card_change,
) {
    wire_edit_card_impl(port_, card_id, changes)
}

#[no_mangle]
pub extern "C" fn wire_close_card(port_: i64, card_id: *mut wire_uint_8_list) {
    wire_close_card_impl(port_, card_id)
}

#[no_mangle]
pub extern "C" fn wire_get_card(port_: i64, card_id: *mut wire_uint_8_list) {
    wire_get_card_impl(port_, card_id)
}

#[no_mangle]
pub extern "C" fn wire_create_card(port_: i64) {
    wire_create_card_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_move_card_to_bin(port_: i64, card_id: *mut wire_uint_8_list) {
    wire_move_card_to_bin_impl(port_, card_id)
}

#[no_mangle]
pub extern "C" fn wire_restore_from_bin(port_: i64, card_id: *mut wire_uint_8_list) {
    wire_restore_from_bin_impl(port_, card_id)
}

#[no_mangle]
pub extern "C" fn wire_empty_bin(port_: i64) {
    wire_empty_bin_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_move_card_to_bin_all(port_: i64, card_id: *mut wire_uint_8_list) {
    wire_move_card_to_bin_all_impl(port_, card_id)
}

#[no_mangle]
pub extern "C" fn wire_add_card_label(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    label_id: *mut wire_uint_8_list,
) {
    wire_add_card_label_impl(port_, card_id, label_id)
}

#[no_mangle]
pub extern "C" fn wire_remove_card_label(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    label_id: *mut wire_uint_8_list,
) {
    wire_remove_card_label_impl(port_, card_id, label_id)
}

#[no_mangle]
pub extern "C" fn wire_get_file_path(port_: i64, blob_id: *mut wire_uint_8_list) {
    wire_get_file_path_impl(port_, blob_id)
}

#[no_mangle]
pub extern "C" fn wire_download_file(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    blob_id: *mut wire_uint_8_list,
    device_id: *mut wire_uint_8_list,
) {
    wire_download_file_impl(port_, card_id, blob_id, device_id)
}

#[no_mangle]
pub extern "C" fn wire_get_account(port_: i64) {
    wire_get_account_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_account_group(port_: i64) {
    wire_account_group_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_edit_name(port_: i64, name: *mut wire_uint_8_list) {
    wire_edit_name_impl(port_, name)
}

#[no_mangle]
pub extern "C" fn wire_add_contact(port_: i64, contact: *mut wire_AccContact) {
    wire_add_contact_impl(port_, contact)
}

#[no_mangle]
pub extern "C" fn wire_edit_contact_name(
    port_: i64,
    account_id: *mut wire_uint_8_list,
    name: *mut wire_uint_8_list,
) {
    wire_edit_contact_name_impl(port_, account_id, name)
}

#[no_mangle]
pub extern "C" fn wire_create_acc_label(port_: i64, name: *mut wire_uint_8_list) {
    wire_create_acc_label_impl(port_, name)
}

#[no_mangle]
pub extern "C" fn wire_delete_acc_label(port_: i64, label_id: *mut wire_uint_8_list) {
    wire_delete_acc_label_impl(port_, label_id)
}

#[no_mangle]
pub extern "C" fn wire_edit_collaborators(
    port_: i64,
    card_id: *mut wire_uint_8_list,
    changes: *mut wire_list_collaborator_change,
) {
    wire_edit_collaborators_impl(port_, card_id, changes)
}

#[no_mangle]
pub extern "C" fn wire_export_data(port_: i64, out_dir: *mut wire_uint_8_list) {
    wire_export_data_impl(port_, out_dir)
}

#[no_mangle]
pub extern "C" fn wire_import_data(port_: i64, in_dir: *mut wire_uint_8_list) {
    wire_import_data_impl(port_, in_dir)
}

#[no_mangle]
pub extern "C" fn wire_get_current_device_id() -> support::WireSyncReturn {
    wire_get_current_device_id_impl()
}

#[no_mangle]
pub extern "C" fn wire_get_deleted_label_id() -> support::WireSyncReturn {
    wire_get_deleted_label_id_impl()
}

#[no_mangle]
pub extern "C" fn wire_scan_qr_code(
    port_: i64,
    width: u32,
    height: u32,
    format: i32,
    buf: *mut wire_uint_8_list,
) {
    wire_scan_qr_code_impl(port_, width, height, format, buf)
}

#[no_mangle]
pub extern "C" fn wire_list_profiles(port_: i64) {
    wire_list_profiles_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_accept_notification(port_: i64, id: *mut wire_uint_8_list) {
    wire_accept_notification_impl(port_, id)
}

#[no_mangle]
pub extern "C" fn wire_ignore_notification(port_: i64, id: *mut wire_uint_8_list) {
    wire_ignore_notification_impl(port_, id)
}

#[no_mangle]
pub extern "C" fn wire_list_notification_ids(port_: i64) {
    wire_list_notification_ids_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_logout(port_: i64) {
    wire_logout_impl(port_)
}

// Section: allocate functions

#[no_mangle]
pub extern "C" fn new_StringList_0(len: i32) -> *mut wire_StringList {
    let wrap = wire_StringList {
        ptr: support::new_leak_vec_ptr(<*mut wire_uint_8_list>::new_with_null_ptr(), len),
        len,
    };
    support::new_leak_box_ptr(wrap)
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_acc_contact_0() -> *mut wire_AccContact {
    support::new_leak_box_ptr(wire_AccContact::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_bool_0(value: bool) -> *mut bool {
    support::new_leak_box_ptr(value)
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_card_block_0() -> *mut wire_CardBlock {
    support::new_leak_box_ptr(wire_CardBlock::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_card_file_0() -> *mut wire_CardFile {
    support::new_leak_box_ptr(wire_CardFile::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_card_text_0() -> *mut wire_CardText {
    support::new_leak_box_ptr(wire_CardText::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_card_text_attrs_0() -> *mut wire_CardTextAttrs {
    support::new_leak_box_ptr(wire_CardTextAttrs::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_u8_0(value: u8) -> *mut u8 {
    support::new_leak_box_ptr(value)
}

#[no_mangle]
pub extern "C" fn new_box_content_view_0() -> *mut wire_ContentView {
    support::new_leak_box_ptr(wire_ContentView::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_list_card_change_0(len: i32) -> *mut wire_list_card_change {
    let wrap = wire_list_card_change {
        ptr: support::new_leak_vec_ptr(<wire_CardChange>::new_with_null_ptr(), len),
        len,
    };
    support::new_leak_box_ptr(wrap)
}

#[no_mangle]
pub extern "C" fn new_list_collaborator_change_0(len: i32) -> *mut wire_list_collaborator_change {
    let wrap = wire_list_collaborator_change {
        ptr: support::new_leak_vec_ptr(<wire_CollaboratorChange>::new_with_null_ptr(), len),
        len,
    };
    support::new_leak_box_ptr(wrap)
}

#[no_mangle]
pub extern "C" fn new_uint_8_list_0(len: i32) -> *mut wire_uint_8_list {
    let ans = wire_uint_8_list {
        ptr: support::new_leak_vec_ptr(Default::default(), len),
        len,
    };
    support::new_leak_box_ptr(ans)
}

// Section: related functions

// Section: impl Wire2Api

impl Wire2Api<String> for *mut wire_uint_8_list {
    fn wire2api(self) -> String {
        let vec: Vec<u8> = self.wire2api();
        String::from_utf8_lossy(&vec).into_owned()
    }
}
impl Wire2Api<Vec<String>> for *mut wire_StringList {
    fn wire2api(self) -> Vec<String> {
        let vec = unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        };
        vec.into_iter().map(Wire2Api::wire2api).collect()
    }
}
impl Wire2Api<AccContact> for wire_AccContact {
    fn wire2api(self) -> AccContact {
        AccContact {
            account_id: self.account_id.wire2api(),
            name: self.name.wire2api(),
        }
    }
}

impl Wire2Api<AccContact> for *mut wire_AccContact {
    fn wire2api(self) -> AccContact {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<AccContact>::wire2api(*wrap).into()
    }
}

impl Wire2Api<CardBlock> for *mut wire_CardBlock {
    fn wire2api(self) -> CardBlock {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<CardBlock>::wire2api(*wrap).into()
    }
}
impl Wire2Api<CardFile> for *mut wire_CardFile {
    fn wire2api(self) -> CardFile {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<CardFile>::wire2api(*wrap).into()
    }
}
impl Wire2Api<CardText> for *mut wire_CardText {
    fn wire2api(self) -> CardText {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<CardText>::wire2api(*wrap).into()
    }
}
impl Wire2Api<CardTextAttrs> for *mut wire_CardTextAttrs {
    fn wire2api(self) -> CardTextAttrs {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<CardTextAttrs>::wire2api(*wrap).into()
    }
}

impl Wire2Api<Box<ContentView>> for *mut wire_ContentView {
    fn wire2api(self) -> Box<ContentView> {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<ContentView>::wire2api(*wrap).into()
    }
}
impl Wire2Api<CardBlock> for wire_CardBlock {
    fn wire2api(self) -> CardBlock {
        CardBlock {
            position: self.position.wire2api(),
            view: self.view.wire2api(),
        }
    }
}
impl Wire2Api<CardChange> for wire_CardChange {
    fn wire2api(self) -> CardChange {
        match self.tag {
            0 => unsafe {
                let ans = support::box_from_leak_ptr(self.kind);
                let ans = support::box_from_leak_ptr(ans.Insert);
                CardChange::Insert(ans.field0.wire2api())
            },
            1 => unsafe {
                let ans = support::box_from_leak_ptr(self.kind);
                let ans = support::box_from_leak_ptr(ans.Remove);
                CardChange::Remove {
                    position: ans.position.wire2api(),
                    len: ans.len.wire2api(),
                }
            },
            2 => unsafe {
                let ans = support::box_from_leak_ptr(self.kind);
                let ans = support::box_from_leak_ptr(ans.Format);
                CardChange::Format {
                    position: ans.position.wire2api(),
                    len: ans.len.wire2api(),
                    attributes: ans.attributes.wire2api(),
                }
            },
            _ => unreachable!(),
        }
    }
}
impl Wire2Api<CardFile> for wire_CardFile {
    fn wire2api(self) -> CardFile {
        CardFile {
            blob_id: self.blob_id.wire2api(),
            device_id: self.device_id.wire2api(),
            checksum: self.checksum.wire2api(),
            size_bytes: self.size_bytes.wire2api(),
            name: self.name.wire2api(),
        }
    }
}
impl Wire2Api<CardText> for wire_CardText {
    fn wire2api(self) -> CardText {
        CardText {
            value: self.value.wire2api(),
            attrs: self.attrs.wire2api(),
        }
    }
}
impl Wire2Api<CardTextAttrs> for wire_CardTextAttrs {
    fn wire2api(self) -> CardTextAttrs {
        CardTextAttrs {
            bold: self.bold.wire2api(),
            italic: self.italic.wire2api(),
            underline: self.underline.wire2api(),
            strikethrough: self.strikethrough.wire2api(),
            link: self.link.wire2api(),
            checked: self.checked.wire2api(),
            heading: self.heading.wire2api(),
            block: self.block.wire2api(),
        }
    }
}
impl Wire2Api<CollaboratorChange> for wire_CollaboratorChange {
    fn wire2api(self) -> CollaboratorChange {
        CollaboratorChange {
            account_id: self.account_id.wire2api(),
            rights: self.rights.wire2api(),
            removed: self.removed.wire2api(),
        }
    }
}
impl Wire2Api<ContentView> for wire_ContentView {
    fn wire2api(self) -> ContentView {
        match self.tag {
            0 => unsafe {
                let ans = support::box_from_leak_ptr(self.kind);
                let ans = support::box_from_leak_ptr(ans.Text);
                ContentView::Text(ans.field0.wire2api())
            },
            1 => unsafe {
                let ans = support::box_from_leak_ptr(self.kind);
                let ans = support::box_from_leak_ptr(ans.File);
                ContentView::File(ans.field0.wire2api())
            },
            _ => unreachable!(),
        }
    }
}

impl Wire2Api<Vec<CardChange>> for *mut wire_list_card_change {
    fn wire2api(self) -> Vec<CardChange> {
        let vec = unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        };
        vec.into_iter().map(Wire2Api::wire2api).collect()
    }
}
impl Wire2Api<Vec<CollaboratorChange>> for *mut wire_list_collaborator_change {
    fn wire2api(self) -> Vec<CollaboratorChange> {
        let vec = unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        };
        vec.into_iter().map(Wire2Api::wire2api).collect()
    }
}

impl Wire2Api<Vec<u8>> for *mut wire_uint_8_list {
    fn wire2api(self) -> Vec<u8> {
        unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        }
    }
}
// Section: wire structs

#[repr(C)]
#[derive(Clone)]
pub struct wire_StringList {
    ptr: *mut *mut wire_uint_8_list,
    len: i32,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_AccContact {
    account_id: *mut wire_uint_8_list,
    name: *mut wire_uint_8_list,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardBlock {
    position: u32,
    view: *mut wire_ContentView,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardFile {
    blob_id: *mut wire_uint_8_list,
    device_id: *mut wire_uint_8_list,
    checksum: *mut wire_uint_8_list,
    size_bytes: u32,
    name: *mut wire_uint_8_list,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardText {
    value: *mut wire_uint_8_list,
    attrs: *mut wire_CardTextAttrs,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardTextAttrs {
    bold: *mut bool,
    italic: *mut bool,
    underline: *mut bool,
    strikethrough: *mut bool,
    link: *mut wire_uint_8_list,
    checked: *mut bool,
    heading: *mut u8,
    block: *mut wire_uint_8_list,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CollaboratorChange {
    account_id: *mut wire_uint_8_list,
    rights: i32,
    removed: bool,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_list_card_change {
    ptr: *mut wire_CardChange,
    len: i32,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_list_collaborator_change {
    ptr: *mut wire_CollaboratorChange,
    len: i32,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_uint_8_list {
    ptr: *mut u8,
    len: i32,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardChange {
    tag: i32,
    kind: *mut CardChangeKind,
}

#[repr(C)]
pub union CardChangeKind {
    Insert: *mut wire_CardChange_Insert,
    Remove: *mut wire_CardChange_Remove,
    Format: *mut wire_CardChange_Format,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardChange_Insert {
    field0: *mut wire_CardBlock,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardChange_Remove {
    position: u32,
    len: u32,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_CardChange_Format {
    position: u32,
    len: u32,
    attributes: *mut wire_CardTextAttrs,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_ContentView {
    tag: i32,
    kind: *mut ContentViewKind,
}

#[repr(C)]
pub union ContentViewKind {
    Text: *mut wire_ContentView_Text,
    File: *mut wire_ContentView_File,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_ContentView_Text {
    field0: *mut wire_CardText,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_ContentView_File {
    field0: *mut wire_CardFile,
}

// Section: impl NewWithNullPtr

pub trait NewWithNullPtr {
    fn new_with_null_ptr() -> Self;
}

impl<T> NewWithNullPtr for *mut T {
    fn new_with_null_ptr() -> Self {
        std::ptr::null_mut()
    }
}

impl NewWithNullPtr for wire_AccContact {
    fn new_with_null_ptr() -> Self {
        Self {
            account_id: core::ptr::null_mut(),
            name: core::ptr::null_mut(),
        }
    }
}

impl NewWithNullPtr for wire_CardBlock {
    fn new_with_null_ptr() -> Self {
        Self {
            position: Default::default(),
            view: core::ptr::null_mut(),
        }
    }
}

impl NewWithNullPtr for wire_CardChange {
    fn new_with_null_ptr() -> Self {
        Self {
            tag: -1,
            kind: core::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn inflate_CardChange_Insert() -> *mut CardChangeKind {
    support::new_leak_box_ptr(CardChangeKind {
        Insert: support::new_leak_box_ptr(wire_CardChange_Insert {
            field0: core::ptr::null_mut(),
        }),
    })
}

#[no_mangle]
pub extern "C" fn inflate_CardChange_Remove() -> *mut CardChangeKind {
    support::new_leak_box_ptr(CardChangeKind {
        Remove: support::new_leak_box_ptr(wire_CardChange_Remove {
            position: Default::default(),
            len: Default::default(),
        }),
    })
}

#[no_mangle]
pub extern "C" fn inflate_CardChange_Format() -> *mut CardChangeKind {
    support::new_leak_box_ptr(CardChangeKind {
        Format: support::new_leak_box_ptr(wire_CardChange_Format {
            position: Default::default(),
            len: Default::default(),
            attributes: core::ptr::null_mut(),
        }),
    })
}

impl NewWithNullPtr for wire_CardFile {
    fn new_with_null_ptr() -> Self {
        Self {
            blob_id: core::ptr::null_mut(),
            device_id: core::ptr::null_mut(),
            checksum: core::ptr::null_mut(),
            size_bytes: Default::default(),
            name: core::ptr::null_mut(),
        }
    }
}

impl NewWithNullPtr for wire_CardText {
    fn new_with_null_ptr() -> Self {
        Self {
            value: core::ptr::null_mut(),
            attrs: core::ptr::null_mut(),
        }
    }
}

impl NewWithNullPtr for wire_CardTextAttrs {
    fn new_with_null_ptr() -> Self {
        Self {
            bold: core::ptr::null_mut(),
            italic: core::ptr::null_mut(),
            underline: core::ptr::null_mut(),
            strikethrough: core::ptr::null_mut(),
            link: core::ptr::null_mut(),
            checked: core::ptr::null_mut(),
            heading: core::ptr::null_mut(),
            block: core::ptr::null_mut(),
        }
    }
}

impl NewWithNullPtr for wire_CollaboratorChange {
    fn new_with_null_ptr() -> Self {
        Self {
            account_id: core::ptr::null_mut(),
            rights: Default::default(),
            removed: Default::default(),
        }
    }
}

impl NewWithNullPtr for wire_ContentView {
    fn new_with_null_ptr() -> Self {
        Self {
            tag: -1,
            kind: core::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn inflate_ContentView_Text() -> *mut ContentViewKind {
    support::new_leak_box_ptr(ContentViewKind {
        Text: support::new_leak_box_ptr(wire_ContentView_Text {
            field0: core::ptr::null_mut(),
        }),
    })
}

#[no_mangle]
pub extern "C" fn inflate_ContentView_File() -> *mut ContentViewKind {
    support::new_leak_box_ptr(ContentViewKind {
        File: support::new_leak_box_ptr(wire_ContentView_File {
            field0: core::ptr::null_mut(),
        }),
    })
}

// Section: sync execution mode utility

#[no_mangle]
pub extern "C" fn free_WireSyncReturn(ptr: support::WireSyncReturn) {
    unsafe {
        let _ = support::box_from_leak_ptr(ptr);
    };
}
