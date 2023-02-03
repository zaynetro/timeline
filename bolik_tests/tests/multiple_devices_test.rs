use std::{collections::HashSet, io::Write, path::Path};

use bolik_proto::sync::DeviceVectorClock;
use bolik_sdk::{
    client::Client,
    output::OutputEvent,
    timeline::card::{CardBlock, CardChange, CardText, ContentView},
    MoveToBinScope, BIN_LABEL_ID,
};
use bolik_server::get_device_id;
use openmls::prelude::{KeyPackage, TlsDeserializeTrait};

use bolik_tests as common;

#[tokio::test]
async fn test_multiple_devices_link_device() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    let share = sdk_b.get_device_share().unwrap();
    let _ = sdk_b.output().await.unwrap();

    // Link B to A
    let added_device = sdk_a.link_device(&share).await.unwrap();
    assert_eq!(added_device, "B");
    sdk_a.expect_synced().await.unwrap();

    // Verify account info on A and B
    sdk_b.sync();
    sdk_b.expect_connected_account().await.unwrap();
    let acc_b = sdk_b.expect_acc_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();
    assert_eq!(acc_a.id, acc_b.id);

    // Push docs on A
    let card_a = sdk_a.create_sample_card("Hello").unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Push docs on B
    let card_b = sdk_b.create_sample_card("Another").unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    // Fetch docs on A
    sdk_a.sync();
    assert_eq!(sdk_a.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_a.expect_synced().await.unwrap();

    // Verify both cards are present on both devices
    sdk_a.get_card(&card_a.id).unwrap();
    sdk_a.get_card(&card_b.id).unwrap();
    sdk_b.get_card(&card_a.id).unwrap();
    sdk_b.get_card(&card_b.id).unwrap();

    // Verify devices
    let info_a = sdk_a.account_group().unwrap();
    let info_b = sdk_b.account_group().unwrap();
    assert_eq!(info_a.authentication_secret, info_b.authentication_secret);
    assert_eq!(info_a.devices.len(), 2);
    assert_eq!(info_b.devices.len(), 2);

    let acc_a = sdk_a.get_account().unwrap();
    let acc_b = sdk_b.get_account().unwrap();
    assert_eq!(acc_a.devices.len(), 2);
    assert_eq!(acc_b.devices.len(), 2);

    // Verify account devices on the server
    let server_devices = sdk_a.client.get_account_devices(&acc_a.id).await.unwrap();
    let mut device_ids = HashSet::new();
    for message in server_devices.key_packages {
        let package = KeyPackage::tls_deserialize(&mut message.data.as_slice()).unwrap();
        let device_id = get_device_id(package.credential()).unwrap();
        device_ids.insert(device_id);
    }

    assert_eq!(device_ids.len(), 2);
}

#[tokio::test]
async fn test_multiple_devices_concurrent_add() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // A adds B
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // A adds C (concurrent)
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_c).await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // B adds D (concurrent)
    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    let share_bd = sdk_d.get_device_share().unwrap();
    let _ = sdk_d.output().await.unwrap();
    sdk_b.link_device(&share_bd).await.unwrap();
    sdk_b.expect_acc_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    sdk_d.sync();
    sdk_d.expect_connected_account().await.unwrap();
    // Fails to sync because backend didn't recognize this chain due to conflict
    sdk_d.expect_sync_failed().await.unwrap();

    // Sync all devices
    sdk_a.sync();
    let _ = sdk_a.output().await.unwrap(); // AccUpdated
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();
    sdk_c.sync();
    let _ = sdk_c.output().await.unwrap(); // AccUpdated
    sdk_c.expect_synced().await.unwrap();
    sdk_d.sync();
    // Now device is operational
    sdk_d.expect_acc_updated().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Verify account group
    let info_a = sdk_a.account_group().unwrap();
    let info_b = sdk_b.account_group().unwrap();
    let info_c = sdk_c.account_group().unwrap();
    let info_d = sdk_d.account_group().unwrap();
    assert_eq!(info_a.authentication_secret, info_b.authentication_secret);
    assert_eq!(info_c.authentication_secret, info_d.authentication_secret);
    assert_eq!(info_a.authentication_secret, info_c.authentication_secret);
    assert_eq!(info_a.devices.len(), 4);
    assert_eq!(info_b.devices.len(), 4);
    assert_eq!(info_c.devices.len(), 4);
    assert_eq!(info_d.devices.len(), 4);

    // Verify account
    let acc_a = sdk_a.get_account().unwrap();
    let acc_b = sdk_b.get_account().unwrap();
    let acc_c = sdk_c.get_account().unwrap();
    let acc_d = sdk_d.get_account().unwrap();
    assert_eq!(acc_a.devices.len(), 4);
    assert_eq!(acc_b.devices.len(), 4);
    assert_eq!(acc_c.devices.len(), 4);
    assert_eq!(acc_d.devices.len(), 4);
}

#[tokio::test]
async fn test_multiple_devices_access_file() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Link B
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create a temporary file
    let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
    // Write enough data to span over several chunks when encrypting/decrypting
    for _ in 0..50_000 {
        tmp_attachment.write(&[1]).unwrap();
    }
    let tmp_attachment_path = tmp_attachment.into_temp_path();

    // Create a card and attach a file
    let card = sdk_a.create_card().unwrap();
    let card = sdk_a.attach_file(&card.id, &tmp_attachment_path).unwrap();
    let ContentView::File(file) = &card.blocks[0].view else {
        panic!("Expected File but got {:?}", card.blocks[0].view)
    };
    assert_eq!(
        "Chpo8EQoL6C91RWQhJPU18gcLn25GUQJWMLB6przUCrT",
        file.checksum
    );
    assert_eq!(file.size_bytes, 50_000);

    sdk_a.close_card(&card.id).unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    // Download the file from B
    let res = sdk_b
        .download_blob(&card.id, &file.blob_id, &file.device_id)
        .unwrap();
    assert!(res.path.is_none());
    assert!(res.download_started);

    let event = sdk_b.output().await.unwrap();
    if let OutputEvent::DownloadCompleted { path, .. } = event {
        assert!(Path::new(&path).exists());
        // Verify file size
        let file = std::fs::File::open(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 50_000);
    } else {
        panic!("Expected DownloadCompleted but received {:?}", event);
    }
}

#[tokio::test]
async fn test_multiple_devices_concurrent_card_edit() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    let card_1 = sdk_a.create_sample_card("Hello world!").unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    // Concurrently edit the card
    sdk_a
        .edit_card(
            &card_1.id,
            vec![CardChange::Insert(CardBlock {
                position: 11,
                view: ContentView::Text(CardText {
                    value: " and Good luck".into(),
                    attrs: None,
                }),
            })],
        )
        .unwrap();
    sdk_a.close_card(&card_1.id).unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b
        .edit_card(
            &card_1.id,
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
        )
        .unwrap();
    sdk_b.close_card(&card_1.id).unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    sdk_a.sync();
    assert_eq!(sdk_a.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_a.expect_synced().await.unwrap();

    // Verify both changes are visible
    let text_a = &sdk_a.get_card(&card_1.id).unwrap().blocks[0].view;
    let text_b = &sdk_b.get_card(&card_1.id).unwrap().blocks[0].view;

    for (i, text) in [text_a, text_b].iter().enumerate() {
        if let ContentView::Text(t) = text {
            assert_eq!("Hi world and Good luck!", t.value, "index={}", i);
        } else {
            panic!("Expected Text content");
        }
    }

    // Verify timeline
    let days_a = sdk_a.timeline_days(vec![]).unwrap();
    let days_b = sdk_a.timeline_days(vec![]).unwrap();
    assert_eq!(days_a, days_b);
    assert_eq!(days_a.len(), 1);

    let cards_a = sdk_a.timeline_by_day(&days_a[0], vec![]).unwrap();
    let cards_b = sdk_b.timeline_by_day(&days_b[0], vec![]).unwrap();
    assert_eq!(cards_a.cards.len(), cards_b.cards.len());
    assert_eq!(cards_a.cards.len(), 1);

    // Now we have an account, a profile and a doc on the server side
    let docs_a = sdk_a
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    assert_eq!(docs_a.docs.len(), 3);
}

#[tokio::test]
async fn test_multiple_devices_permanent_deletion() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Part one:
    //   Sync in between.

    // Create card on A and move to bin
    let card_1 = sdk_a.create_sample_card("Hello world!").unwrap();
    sdk_a.expect_synced().await.unwrap();
    sdk_a
        .move_card_to_bin(&card_1.id, MoveToBinScope::ThisAccount)
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // B should see card in the bin
    sdk_b.sync();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    let days_b = sdk_b.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    let cards_b = sdk_b
        .timeline_by_day(&days_b[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(1, cards_b.cards.len());
    assert_eq!(card_1.id, cards_b.cards[0].id);

    // Clear bin on A
    sdk_a.empty_bin().unwrap();
    sdk_a.expect_synced().await.unwrap();

    // A and B should not see the card
    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();

    let days_a = sdk_a.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_a.len());
    let days_b = sdk_b.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_b.len());

    // Part two:
    //   Sync once.

    // Create card on A and move to bin
    let card_2 = sdk_a.create_sample_card("One two!").unwrap();
    sdk_a.expect_synced().await.unwrap();
    sdk_a
        .move_card_to_bin(&card_2.id, MoveToBinScope::ThisAccount)
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Clear bin on A
    sdk_a.empty_bin().unwrap();
    sdk_a.expect_synced().await.unwrap();

    // A and B should not see the card
    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();

    let days_a = sdk_a.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_a.len());
    let days_b = sdk_b.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_b.len());
}

#[tokio::test]
async fn test_multiple_devices_concurrent_permanent_deletion() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    sdk_b.link_devices(&mut sdk_c).await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Create card on A
    let card_1 = sdk_a.create_sample_card("Hello world!").unwrap();
    sdk_a.expect_acc_updated().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    // B should see card
    sdk_b.sync();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    // Move card to bin and clear bin on A
    sdk_a
        .move_card_to_bin(&card_1.id, MoveToBinScope::ThisAccount)
        .unwrap();
    sdk_a.empty_bin().unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Edit card on B
    sdk_b
        .edit_card(&card_1.id, vec![CardChange::append_text(" from B")])
        .unwrap();
    sdk_b.close_card(&card_1.id).unwrap();
    sdk_b.expect_synced().await.unwrap();

    // A, B and C should not see the card
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();
    sdk_c.sync();
    sdk_c.expect_synced().await.unwrap();

    let days_a = sdk_a.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_a.len());
    let days_b = sdk_b.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_b.len());
    let days_c = sdk_c.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(0, days_c.len());

    assert!(sdk_a.get_card(&card_1.id).is_err());
    assert!(sdk_b.get_card(&card_1.id).is_err());
    assert!(sdk_c.get_card(&card_1.id).is_err());
}

#[tokio::test]
async fn test_multiple_devices_remove_device() {
    common::setup();
    let server = common::start_server().await.unwrap();
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_a = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Remove B
    sdk_a.remove_device(sdk_b.get_device_id()).unwrap();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    assert_eq!(OutputEvent::LogOut, sdk_b.output().await.unwrap());
    sdk_b.expect_synced().await.unwrap();
    assert_eq!(None, sdk_b.get_account());
}

// TODO: concurrent file modification
// TODO: test removing files on single device but not on the other (should keep blob refs to docs)
// TODO: link a device, create a card, remove a device that created a card, link new device (new device should be able to decrypt the card)
// TODO: self update package
