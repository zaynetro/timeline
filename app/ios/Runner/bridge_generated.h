#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
typedef struct _Dart_Handle* Dart_Handle;

typedef struct DartCObject DartCObject;

typedef int64_t DartPort;

typedef bool (*DartPostCObjectFnType)(DartPort port_id, void *message);

typedef struct wire_uint_8_list {
  uint8_t *ptr;
  int32_t len;
} wire_uint_8_list;

typedef struct wire_StringList {
  struct wire_uint_8_list **ptr;
  int32_t len;
} wire_StringList;

typedef struct wire_CardTextAttrs {
  bool *bold;
  bool *italic;
  bool *underline;
  bool *strikethrough;
  struct wire_uint_8_list *link;
  bool *checked;
  uint8_t *heading;
  struct wire_uint_8_list *block;
} wire_CardTextAttrs;

typedef struct wire_CardText {
  struct wire_uint_8_list *value;
  struct wire_CardTextAttrs *attrs;
} wire_CardText;

typedef struct wire_ContentView_Text {
  struct wire_CardText *field0;
} wire_ContentView_Text;

typedef struct wire_CardFile {
  struct wire_uint_8_list *blob_id;
  struct wire_uint_8_list *device_id;
  struct wire_uint_8_list *checksum;
  uint32_t size_bytes;
  struct wire_uint_8_list *name;
} wire_CardFile;

typedef struct wire_ContentView_File {
  struct wire_CardFile *field0;
} wire_ContentView_File;

typedef union ContentViewKind {
  struct wire_ContentView_Text *Text;
  struct wire_ContentView_File *File;
} ContentViewKind;

typedef struct wire_ContentView {
  int32_t tag;
  union ContentViewKind *kind;
} wire_ContentView;

typedef struct wire_CardBlock {
  uint32_t position;
  struct wire_ContentView *view;
} wire_CardBlock;

typedef struct wire_CardChange_Insert {
  struct wire_CardBlock *field0;
} wire_CardChange_Insert;

typedef struct wire_CardChange_Remove {
  uint32_t position;
  uint32_t len;
} wire_CardChange_Remove;

typedef struct wire_CardChange_Format {
  uint32_t position;
  uint32_t len;
  struct wire_CardTextAttrs *attributes;
} wire_CardChange_Format;

typedef union CardChangeKind {
  struct wire_CardChange_Insert *Insert;
  struct wire_CardChange_Remove *Remove;
  struct wire_CardChange_Format *Format;
} CardChangeKind;

typedef struct wire_CardChange {
  int32_t tag;
  union CardChangeKind *kind;
} wire_CardChange;

typedef struct wire_list_card_change {
  struct wire_CardChange *ptr;
  int32_t len;
} wire_list_card_change;

typedef struct wire_AccContact {
  struct wire_uint_8_list *account_id;
  struct wire_uint_8_list *name;
} wire_AccContact;

typedef struct wire_CollaboratorChange {
  struct wire_uint_8_list *account_id;
  int32_t *rights;
} wire_CollaboratorChange;

typedef struct wire_list_collaborator_change {
  struct wire_CollaboratorChange *ptr;
  int32_t len;
} wire_list_collaborator_change;

typedef struct DartCObject *WireSyncReturn;

void store_dart_post_cobject(DartPostCObjectFnType ptr);

Dart_Handle get_dart_object(uintptr_t ptr);

void drop_dart_object(uintptr_t ptr);

uintptr_t new_dart_opaque(Dart_Handle handle);

intptr_t init_frb_dart_api_dl(void *obj);

void wire_setup_logs(int64_t port_);

void wire_setup(int64_t port_,
                struct wire_uint_8_list *app_support_dir,
                struct wire_uint_8_list *files_dir,
                struct wire_uint_8_list *device_name);

void wire_timeline_days(int64_t port_, struct wire_StringList *label_ids);

void wire_timeline_by_day(int64_t port_,
                          struct wire_uint_8_list *day,
                          struct wire_StringList *label_ids);

void wire_get_device_share(int64_t port_);

void wire_link_device(int64_t port_, struct wire_uint_8_list *share);

void wire_remove_device(int64_t port_, struct wire_uint_8_list *remove_id);

void wire_sync(int64_t port_);

void wire_create_account(int64_t port_, struct wire_uint_8_list *name);

void wire_save_file(int64_t port_, struct wire_uint_8_list *card_id, struct wire_uint_8_list *path);

void wire_edit_card(int64_t port_,
                    struct wire_uint_8_list *card_id,
                    struct wire_list_card_change *changes);

void wire_close_card(int64_t port_, struct wire_uint_8_list *card_id);

void wire_get_card(int64_t port_, struct wire_uint_8_list *card_id);

void wire_create_card(int64_t port_);

void wire_move_card_to_bin(int64_t port_, struct wire_uint_8_list *card_id);

void wire_restore_from_bin(int64_t port_, struct wire_uint_8_list *card_id);

void wire_empty_bin(int64_t port_);

void wire_move_card_to_bin_all(int64_t port_, struct wire_uint_8_list *card_id);

void wire_add_card_label(int64_t port_,
                         struct wire_uint_8_list *card_id,
                         struct wire_uint_8_list *label_id);

void wire_remove_card_label(int64_t port_,
                            struct wire_uint_8_list *card_id,
                            struct wire_uint_8_list *label_id);

void wire_get_file_path(int64_t port_, struct wire_uint_8_list *blob_id);

void wire_download_file(int64_t port_,
                        struct wire_uint_8_list *card_id,
                        struct wire_uint_8_list *blob_id,
                        struct wire_uint_8_list *device_id);

void wire_get_account(int64_t port_);

void wire_account_group(int64_t port_);

void wire_edit_name(int64_t port_, struct wire_uint_8_list *name);

void wire_add_contact(int64_t port_, struct wire_AccContact *contact);

void wire_edit_contact_name(int64_t port_,
                            struct wire_uint_8_list *account_id,
                            struct wire_uint_8_list *name);

void wire_create_acc_label(int64_t port_, struct wire_uint_8_list *name);

void wire_delete_acc_label(int64_t port_, struct wire_uint_8_list *label_id);

void wire_edit_collaborators(int64_t port_,
                             struct wire_uint_8_list *card_id,
                             struct wire_list_collaborator_change *changes);

void wire_export_data(int64_t port_, struct wire_uint_8_list *out_dir);

void wire_import_data(int64_t port_, struct wire_uint_8_list *in_dir);

WireSyncReturn wire_get_current_device_id(void);

WireSyncReturn wire_get_deleted_label_id(void);

void wire_scan_qr_code(int64_t port_,
                       uint32_t width,
                       uint32_t height,
                       int32_t format,
                       struct wire_uint_8_list *buf);

void wire_list_profiles(int64_t port_);

void wire_accept_notification(int64_t port_, struct wire_uint_8_list *id);

void wire_ignore_notification(int64_t port_, struct wire_uint_8_list *id);

void wire_list_notification_ids(int64_t port_);

void wire_logout(int64_t port_);

struct wire_StringList *new_StringList_0(int32_t len);

struct wire_AccContact *new_box_autoadd_acc_contact_0(void);

int32_t *new_box_autoadd_acl_rights_0(int32_t value);

bool *new_box_autoadd_bool_0(bool value);

struct wire_CardBlock *new_box_autoadd_card_block_0(void);

struct wire_CardFile *new_box_autoadd_card_file_0(void);

struct wire_CardText *new_box_autoadd_card_text_0(void);

struct wire_CardTextAttrs *new_box_autoadd_card_text_attrs_0(void);

uint8_t *new_box_autoadd_u8_0(uint8_t value);

struct wire_ContentView *new_box_content_view_0(void);

struct wire_list_card_change *new_list_card_change_0(int32_t len);

struct wire_list_collaborator_change *new_list_collaborator_change_0(int32_t len);

struct wire_uint_8_list *new_uint_8_list_0(int32_t len);

union CardChangeKind *inflate_CardChange_Insert(void);

union CardChangeKind *inflate_CardChange_Remove(void);

union CardChangeKind *inflate_CardChange_Format(void);

union ContentViewKind *inflate_ContentView_Text(void);

union ContentViewKind *inflate_ContentView_File(void);

void free_WireSyncReturn(WireSyncReturn ptr);

static int64_t dummy_method_to_enforce_bundling(void) {
    int64_t dummy_var = 0;
    dummy_var ^= ((int64_t) (void*) wire_setup_logs);
    dummy_var ^= ((int64_t) (void*) wire_setup);
    dummy_var ^= ((int64_t) (void*) wire_timeline_days);
    dummy_var ^= ((int64_t) (void*) wire_timeline_by_day);
    dummy_var ^= ((int64_t) (void*) wire_get_device_share);
    dummy_var ^= ((int64_t) (void*) wire_link_device);
    dummy_var ^= ((int64_t) (void*) wire_remove_device);
    dummy_var ^= ((int64_t) (void*) wire_sync);
    dummy_var ^= ((int64_t) (void*) wire_create_account);
    dummy_var ^= ((int64_t) (void*) wire_save_file);
    dummy_var ^= ((int64_t) (void*) wire_edit_card);
    dummy_var ^= ((int64_t) (void*) wire_close_card);
    dummy_var ^= ((int64_t) (void*) wire_get_card);
    dummy_var ^= ((int64_t) (void*) wire_create_card);
    dummy_var ^= ((int64_t) (void*) wire_move_card_to_bin);
    dummy_var ^= ((int64_t) (void*) wire_restore_from_bin);
    dummy_var ^= ((int64_t) (void*) wire_empty_bin);
    dummy_var ^= ((int64_t) (void*) wire_move_card_to_bin_all);
    dummy_var ^= ((int64_t) (void*) wire_add_card_label);
    dummy_var ^= ((int64_t) (void*) wire_remove_card_label);
    dummy_var ^= ((int64_t) (void*) wire_get_file_path);
    dummy_var ^= ((int64_t) (void*) wire_download_file);
    dummy_var ^= ((int64_t) (void*) wire_get_account);
    dummy_var ^= ((int64_t) (void*) wire_account_group);
    dummy_var ^= ((int64_t) (void*) wire_edit_name);
    dummy_var ^= ((int64_t) (void*) wire_add_contact);
    dummy_var ^= ((int64_t) (void*) wire_edit_contact_name);
    dummy_var ^= ((int64_t) (void*) wire_create_acc_label);
    dummy_var ^= ((int64_t) (void*) wire_delete_acc_label);
    dummy_var ^= ((int64_t) (void*) wire_edit_collaborators);
    dummy_var ^= ((int64_t) (void*) wire_export_data);
    dummy_var ^= ((int64_t) (void*) wire_import_data);
    dummy_var ^= ((int64_t) (void*) wire_get_current_device_id);
    dummy_var ^= ((int64_t) (void*) wire_get_deleted_label_id);
    dummy_var ^= ((int64_t) (void*) wire_scan_qr_code);
    dummy_var ^= ((int64_t) (void*) wire_list_profiles);
    dummy_var ^= ((int64_t) (void*) wire_accept_notification);
    dummy_var ^= ((int64_t) (void*) wire_ignore_notification);
    dummy_var ^= ((int64_t) (void*) wire_list_notification_ids);
    dummy_var ^= ((int64_t) (void*) wire_logout);
    dummy_var ^= ((int64_t) (void*) new_StringList_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_acc_contact_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_acl_rights_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_bool_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_card_block_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_card_file_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_card_text_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_card_text_attrs_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_u8_0);
    dummy_var ^= ((int64_t) (void*) new_box_content_view_0);
    dummy_var ^= ((int64_t) (void*) new_list_card_change_0);
    dummy_var ^= ((int64_t) (void*) new_list_collaborator_change_0);
    dummy_var ^= ((int64_t) (void*) new_uint_8_list_0);
    dummy_var ^= ((int64_t) (void*) inflate_CardChange_Insert);
    dummy_var ^= ((int64_t) (void*) inflate_CardChange_Remove);
    dummy_var ^= ((int64_t) (void*) inflate_CardChange_Format);
    dummy_var ^= ((int64_t) (void*) inflate_ContentView_Text);
    dummy_var ^= ((int64_t) (void*) inflate_ContentView_File);
    dummy_var ^= ((int64_t) (void*) free_WireSyncReturn);
    dummy_var ^= ((int64_t) (void*) store_dart_post_cobject);
    dummy_var ^= ((int64_t) (void*) get_dart_object);
    dummy_var ^= ((int64_t) (void*) drop_dart_object);
    dummy_var ^= ((int64_t) (void*) new_dart_opaque);
    return dummy_var;
}