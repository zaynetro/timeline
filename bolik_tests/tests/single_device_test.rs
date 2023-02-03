use std::{io::Write, path::Path};

use bolik_proto::sync::{request, DeviceVectorClock};
use bolik_sdk::{
    client::Client,
    output::OutputEvent,
    timeline::card::{CardChange, ContentView},
    MoveToBinScope, BIN_LABEL_ID,
};
use chrono::Utc;

use bolik_tests as common;

#[tokio::test]
async fn test_single_device_create_account() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();
    let device_id = sdk.get_device_id().to_string();

    // Create account
    let acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();

    // Verify account devices
    assert_eq!(acc.devices.len(), 1);
    assert_eq!(acc.devices[0].id, device_id);
    assert_eq!(sdk.account_group().unwrap().devices.len(), 1);

    // Verify key packages on the server
    let server_packages = sdk.client.get_device_packages(&device_id).await.unwrap();
    assert!(server_packages.key_packages.len() > 2);

    // Verify account devices on the server
    let server_devices = sdk.client.get_account_devices(&acc.id).await.unwrap();
    assert_eq!(
        server_devices.key_packages.len(),
        server_packages.key_packages.len()
    );

    // Verify that account document is on the server
    sdk.sync();
    sdk.expect_synced().await.unwrap();

    let docs = sdk
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    // Account and profile docs
    assert_eq!(docs.docs.len(), 2);
}

#[tokio::test]
async fn test_single_device_edit_account_name() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();

    // Create account
    let acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();
    assert!(acc.name.starts_with("Account #"));

    // Edit account name
    let acc = sdk.edit_name("My Test".into()).unwrap();
    assert_eq!("My Test", acc.name);

    // Edit account name to empty string
    let acc = sdk.edit_name("".into()).unwrap();
    assert_eq!("", acc.name);
}

#[tokio::test]
async fn test_single_device_push_docs() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();

    // Create a doc and sync it
    let card = sdk.create_card().unwrap();
    let card = sdk
        .edit_card(&card.id, vec![CardChange::append_text("Hello")])
        .unwrap();
    sdk.close_card(&card.id).unwrap();
    sdk.expect_synced().await.unwrap();

    // Verify device from another account cannot access the card
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let _acc = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();
    let res = sdk_c.get_card(&card.id);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_single_device_upload_and_download_blob() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();

    // Create a temporary file
    let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
    // Write enough data to span over several chunks when encrypting/decrypting
    for _ in 0..50000 {
        tmp_attachment.write(&[1]).unwrap();
    }
    let tmp_attachment_path = tmp_attachment.into_temp_path();

    // Create a card and attach a file
    let card = sdk.create_card().unwrap();
    let card = sdk.attach_file(&card.id, &tmp_attachment_path).unwrap();
    let ContentView::File(file) = &card.blocks[0].view else {
        panic!("Expected File but got {:?}", card.blocks[0].view)
    };
    assert_eq!(
        "Chpo8EQoL6C91RWQhJPU18gcLn25GUQJWMLB6przUCrT",
        file.checksum
    );
    assert_eq!(file.size_bytes, 50000);

    // Upload a file
    sdk.close_card(&card.id).unwrap();
    sdk.expect_synced().await.unwrap();

    // Pretend that file was removed from disk and download it
    let saved_path = sdk.get_file_path(&file.blob_id).unwrap().unwrap();
    std::fs::remove_file(&saved_path).unwrap();
    let res = sdk
        .download_blob(&card.id, &file.blob_id, &file.device_id)
        .unwrap();
    assert!(res.path.is_none());
    assert!(res.download_started);

    let event = sdk.output().await.unwrap();
    if let OutputEvent::DownloadCompleted { path, .. } = event {
        assert!(Path::new(&path).exists());
        // Verify file size
        let file = std::fs::File::open(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 50000);
    } else {
        panic!("Expected DownloadCompleted but received {:?}", event);
    }

    // Verify device from another account cannot access the blob
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let _acc = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();
    let res = sdk_c
        .client
        .download_blob(&request::PresignDownload {
            blob_id: file.blob_id.clone(),
            device_id: file.device_id.clone(),
            doc_id: card.id.clone(),
        })
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_single_device_blob_server_cleanup() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();

    // Create a temporary file
    let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
    tmp_attachment.write(&[1, 2, 3, 4, 5]).unwrap();
    let tmp_attachment_path = tmp_attachment.into_temp_path();

    // Create a card and attach a file
    let card = sdk.create_card().unwrap();
    let card = sdk.attach_file(&card.id, &tmp_attachment_path).unwrap();
    // Upload a file
    sdk.close_card(&card.id).unwrap();
    sdk.expect_synced().await.unwrap();

    // Trigger blob clean up job
    server.app.mark_unused_blobs().unwrap();
    let info = server.app.cleanup_blobs(Some(Utc::now())).await.unwrap();
    assert_eq!(info.removed, 0);

    // Remove file from card
    sdk.edit_card(
        &card.id,
        vec![CardChange::Remove {
            position: 0,
            len: 1,
        }],
    )
    .unwrap();
    sdk.close_card(&card.id).unwrap();
    sdk.expect_synced().await.unwrap();

    // Trigger blob clean up job
    server.app.mark_unused_blobs().unwrap();
    let info = server.app.cleanup_blobs(Some(Utc::now())).await.unwrap();
    // Now file should be cleaned
    assert_eq!(info.removed, 1);
}

#[tokio::test]
async fn test_single_device_restore_from_bin() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc = sdk.create_account(None).unwrap();
    sdk.expect_synced().await.unwrap();

    // Create a doc and sync it
    let card = sdk.create_card().unwrap();
    let card = sdk
        .edit_card(&card.id, vec![CardChange::append_text("Hello")])
        .unwrap();
    sdk.move_card_to_bin(&card.id, MoveToBinScope::ThisAccount)
        .unwrap();
    sdk.expect_synced().await.unwrap();

    // Restore card
    let restored_card = sdk.restore_from_bin(&card.id).unwrap();
    assert_ne!(card.id, restored_card.id);
    sdk.expect_synced().await.unwrap();

    // Verify timeline
    let days = sdk.timeline_days(vec![]).unwrap();
    assert_eq!(days.len(), 1);
    let cards = sdk.timeline_by_day(&days[0], vec![]).unwrap();
    assert_eq!(cards.cards.len(), 1);
    assert_eq!(cards.cards[0].id, restored_card.id);

    // Verify bin
    let bin_days = sdk.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(bin_days.len(), 0);
}

// TODO: Test blobs ACL: I fail to download some blobs due to `Error Db: 'Find doc_payload_blob': Query returned no rows`
