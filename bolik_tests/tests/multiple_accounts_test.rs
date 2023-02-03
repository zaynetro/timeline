use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::Path,
};

use bolik_proto::sync::DeviceVectorClock;
use bolik_sdk::{
    account::AccContact,
    client::Client,
    output::OutputEvent,
    timeline::{
        acl_doc::AclRights,
        card::{CardBlock, CardChange, CardText, ContentView},
    },
    MoveToBinScope, BIN_LABEL_ID,
};
use chrono::Utc;

use bolik_tests as common;

#[tokio::test]
async fn test_multiple_accounts_add_contact() {
    // Account 1: Device A
    // Account 2: Device C and D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    sdk_c.link_devices(&mut sdk_d).await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Add second account as contact
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Second account should see a notification
    sdk_c.sync();
    let notification = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    let notification_id = notification.id();
    assert_eq!(notification_id, format!("contact-request/{}", acc_1.id));
    let acc_2 = sdk_c.get_account().unwrap();
    assert_eq!(acc_2.contacts.len(), 0);

    // Second account adds a contact and acks notification
    sdk_c.accept_notification(&notification_id).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // No notification on A
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();

    // No notification on D
    sdk_d.sync();
    sdk_d.expect_acc_updated().await.unwrap();
    sdk_d.expect_notifications().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Verify profiles were shared with both accounts
    let profiles_a = sdk_a.list_profiles().unwrap();
    let names_a: HashSet<_> = profiles_a.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names_a, HashSet::from(["Account 1", "Account 2"]));

    let profiles_c = sdk_c.list_profiles().unwrap();
    let names_c: HashSet<_> = profiles_c.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names_c, HashSet::from(["Account 1", "Account 2"]));

    let profiles_d = sdk_d.list_profiles().unwrap();
    let names_d: HashSet<_> = profiles_d.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names_d, HashSet::from(["Account 1", "Account 2"]));

    // Verify contact names were set
    let acc_1 = sdk_a.get_account().unwrap();
    let acc_2_c = sdk_c.get_account().unwrap();
    let acc_2_d = sdk_d.get_account().unwrap();
    assert_eq!("Custom Account 2", acc_1.contacts[0].name);
    assert_eq!("Account 1", acc_2_c.contacts[0].name);
    assert_eq!("Account 1", acc_2_d.contacts[0].name);
}

#[tokio::test]
async fn test_multiple_accounts_ignore_contact_request() {
    // Account 1: Device A
    // Account 2: Device C and D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    sdk_c.link_devices(&mut sdk_d).await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Add second account as contact
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Second account should see a notification
    sdk_c.sync();
    let notification = sdk_c.expect_notification().await.unwrap();
    let notification_id = notification.id();
    assert_eq!(notification_id, format!("contact-request/{}", acc_1.id));
    sdk_c.expect_synced().await.unwrap();
    let acc_2 = sdk_c.get_account().unwrap();
    assert_eq!(acc_2.contacts.len(), 0);

    // Second account ignores notification
    sdk_c.ignore_notification(&notification_id).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // No notification on A
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();

    // No notification on D
    sdk_d.sync();
    sdk_d.expect_notifications().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Verify second account has no contacts
    let acc_1 = sdk_a.get_account().unwrap();
    let acc_2_c = sdk_c.get_account().unwrap();
    let acc_2_d = sdk_d.get_account().unwrap();
    assert_eq!("Custom Account 2", acc_1.contacts[0].name);
    assert_eq!(0, acc_2_c.contacts.len());
    assert_eq!(0, acc_2_d.contacts.len());
}

#[tokio::test]
async fn test_multiple_accounts_share_card() {
    // Account 1: Device A and B
    // Account 2: Device C
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Share card with second account
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Expect contact request notification and card share notification
    sdk_c.sync();
    let contact_notification = sdk_c.expect_notification().await.unwrap();
    let card_notification = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    let contact_notification_id = contact_notification.id();
    let card_notification_id = card_notification.id();
    assert_eq!(
        contact_notification_id,
        format!("contact-request/{}", acc_1.id)
    );
    assert_eq!(card_notification_id, format!("card-share/{}", card_1.id));

    // Verify timeline is empty on C
    sdk_c.expect_timeline_days(0).unwrap();

    // Accept contact and card share
    sdk_c
        .accept_notification(&contact_notification_id)
        .await
        .unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_c
        .accept_notification(&card_notification_id)
        .await
        .unwrap();
    sdk_c.expect_doc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Verify timeline is not empty on C
    let timeline_days = sdk_c.timeline_days(vec![]).unwrap();
    assert_eq!(1, timeline_days.len());

    sdk_c.get_card(&card_1.id).unwrap();
    let acc_2 = sdk_c.get_account().unwrap();
    assert_eq!(acc_2.contacts.len(), 1);
    assert_eq!(acc_2.contacts[0].account_id, acc_1.id);

    // Link device B
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // All devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    sdk_c.get_card(&card_1.id).unwrap();

    // Share card with first account
    let card_2 = sdk_c.create_sample_card("Hello 2").unwrap();
    let card_2 = sdk_c
        .edit_collaborators(
            &card_2.id,
            HashMap::from([(acc_1.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Sync other devices
    // Both devices should see notification
    sdk_a.sync();
    let n1 = sdk_a.expect_notification().await.unwrap();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_notification().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Accept share
    sdk_a.accept_notification(&n1.id()).await.unwrap();
    sdk_a.expect_doc_updated().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_notifications().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // All devices should see the card 2
    sdk_a.get_card(&card_2.id).unwrap();
    sdk_b.get_card(&card_2.id).unwrap();
    sdk_c.get_card(&card_2.id).unwrap();

    // Verify profiles were shared with both accounts
    let profiles_a = sdk_a.list_profiles().unwrap();
    let names_a: HashSet<_> = profiles_a.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names_a, HashSet::from(["Account 1", "Account 2"]));

    let profiles_c = sdk_c.list_profiles().unwrap();
    let names_c: HashSet<_> = profiles_c.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names_c, HashSet::from(["Account 1", "Account 2"]));

    // Verify contact names were set
    let acc_1 = sdk_a.get_account().unwrap();
    let acc_2 = sdk_c.get_account().unwrap();
    assert_eq!("Custom Account 2", acc_1.contacts[0].name);
    assert_eq!("Account 1", acc_2.contacts[0].name);

    // Verify timeline
    sdk_a.expect_timeline_days(1).unwrap();
    sdk_b.expect_timeline_days(1).unwrap();
    sdk_c.expect_timeline_days(1).unwrap();

    let expect_ids: &[&str] = &[card_1.id.as_ref(), &card_2.id.as_ref()];
    sdk_a.expect_timeline_cards(expect_ids).unwrap();
    sdk_b.expect_timeline_cards(expect_ids).unwrap();
    sdk_c.expect_timeline_cards(expect_ids).unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_ignore_card_share() {
    // Account 1: Device A and B
    // Account 2: Device C
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Share card with second account
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Expect contact request notification and card share notification
    sdk_c.sync();
    let contact_notification = sdk_c.expect_notification().await.unwrap();
    let card_notification = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    let contact_notification_id = contact_notification.id();
    let card_notification_id = card_notification.id();
    assert_eq!(
        contact_notification_id,
        format!("contact-request/{}", acc_1.id)
    );
    assert_eq!(card_notification_id, format!("card-share/{}", card_1.id));

    // Verify timeline is empty on C
    sdk_c.expect_timeline_days(0).unwrap();

    // Accept contact request
    sdk_c
        .accept_notification(&contact_notification_id)
        .await
        .unwrap();
    let acc_2 = sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    assert_eq!(acc_2.contacts.len(), 1);
    assert_eq!(acc_2.contacts[0].account_id, acc_1.id);

    // Ignore card share
    sdk_c.ignore_notification(&card_notification_id).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Verify timeline is empty on C
    sdk_c.expect_timeline_days(0).unwrap();

    // Link device B
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Only account A devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    assert!(sdk_c.get_card(&card_1.id).is_err());

    // Share card with first account
    let card_2 = sdk_c.create_sample_card("Hello 2").unwrap();
    let card_2 = sdk_c
        .edit_collaborators(
            &card_2.id,
            HashMap::from([(acc_1.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Ignore card share on A
    sdk_a.sync();
    let card_notification = sdk_a.expect_notification().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    let card_notification_id = card_notification.id();
    assert_eq!(card_notification_id, format!("card-share/{}", card_2.id));

    // Ignore card share
    sdk_a.ignore_notification(&card_notification_id).unwrap();

    // Sync B (should see no notification)
    sdk_b.sync();
    sdk_b.expect_notifications().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Only account C should see the card 2
    assert!(sdk_a.get_card(&card_2.id).is_err());
    assert!(sdk_b.get_card(&card_2.id).is_err());
    sdk_c.get_card(&card_2.id).unwrap();

    // Verify timeline
    sdk_a.expect_timeline_days(1).unwrap();
    sdk_b.expect_timeline_days(1).unwrap();
    sdk_c.expect_timeline_days(1).unwrap();

    let expect_ids_1: &[&str] = &[card_1.id.as_ref()];
    let expect_ids_2: &[&str] = &[&card_2.id.as_ref()];
    sdk_a.expect_timeline_cards(expect_ids_1).unwrap();
    sdk_b.expect_timeline_cards(expect_ids_1).unwrap();
    sdk_c.expect_timeline_cards(expect_ids_2).unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_multiple_cards() {
    // Account 1: Device A and B
    // Account 2: Device C and D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    sdk_c.link_devices(&mut sdk_d).await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Create some cards for account 1
    const CARDS_NUM: usize = 10;
    for i in 0..CARDS_NUM {
        sdk_a
            .create_sample_card(format!("Account 1: card #{}", i))
            .unwrap();
        sdk_a.expect_synced().await.unwrap();
    }

    {
        sdk_b.sync();
        assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
        sdk_b.expect_synced().await.unwrap();

        // Verify A and B see the same cards
        let days = sdk_a.timeline_days(vec![]).unwrap();
        assert_eq!(days.len(), 1);
        let a_cards = sdk_a.timeline_by_day(&days[0], vec![]).unwrap();
        let b_cards = sdk_b.timeline_by_day(&days[0], vec![]).unwrap();
        assert_eq!(a_cards.cards.len(), CARDS_NUM);
        assert_eq!(b_cards.cards.len(), CARDS_NUM);
    }

    // Create some cards for account 2
    for i in 0..CARDS_NUM {
        sdk_c
            .create_sample_card(format!("Account 2: card #{}", i))
            .unwrap();
        sdk_c.expect_synced().await.unwrap();
    }

    {
        sdk_d.sync();
        assert_eq!(sdk_d.output().await.unwrap(), OutputEvent::TimelineUpdated);
        sdk_d.expect_synced().await.unwrap();

        // Verify C and D see the same cards
        let days = sdk_c.timeline_days(vec![]).unwrap();
        assert_eq!(days.len(), 1);
        let c_cards = sdk_c.timeline_by_day(&days[0], vec![]).unwrap();
        let d_cards = sdk_d.timeline_by_day(&days[0], vec![]).unwrap();
        assert_eq!(c_cards.cards.len(), CARDS_NUM);
        assert_eq!(d_cards.cards.len(), CARDS_NUM);
    }

    // Share card with second account
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    let contact_req = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_c.accept_notification(&contact_req.id()).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_d.sync();
    sdk_d.expect_acc_updated().await.unwrap();
    sdk_d.expect_notifications().await.unwrap();
    sdk_d.expect_notification().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Verify contacts on all devices
    let acc_1a = sdk_a.get_account().unwrap();
    let acc_1b = sdk_b.get_account().unwrap();
    let acc_2c = sdk_c.get_account().unwrap();
    let acc_2d = sdk_d.get_account().unwrap();
    assert_eq!(acc_1a.contacts.len(), 1);
    assert_eq!(acc_1a.contacts[0].account_id, acc_2c.id);
    assert_eq!(acc_1b.contacts.len(), 1);
    assert_eq!(acc_1b.contacts[0].account_id, acc_2c.id);
    assert_eq!(acc_2c.contacts.len(), 1);
    assert_eq!(acc_2c.contacts[0].account_id, acc_1a.id);
    assert_eq!(acc_2d.contacts.len(), 1);
    assert_eq!(acc_2d.contacts[0].account_id, acc_1a.id);

    // All devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    sdk_c.get_card(&card_1.id).unwrap();
    sdk_d.get_card(&card_1.id).unwrap();

    // Share card with first account
    let card_2 = sdk_c.create_sample_card("Hello 2").unwrap();
    let card_2 = sdk_c
        .edit_collaborators(
            &card_2.id,
            HashMap::from([(acc_1.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    let _ = sdk_c.output().await.unwrap(); // AccUpdated
    sdk_c.expect_synced().await.unwrap();

    // Sync other devices
    sdk_a.sync();
    sdk_a.expect_notification().await.unwrap();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_notification().await.unwrap();
    sdk_b.expect_synced().await.unwrap();
    sdk_d.sync();
    assert_eq!(sdk_d.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_d.expect_synced().await.unwrap();

    // All devices should see the card 2
    sdk_a.get_card(&card_2.id).unwrap();
    sdk_b.get_card(&card_2.id).unwrap();
    sdk_c.get_card(&card_2.id).unwrap();
    sdk_d.get_card(&card_2.id).unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_share_file() {
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    let acc_2 = sdk_b.create_account(None).unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create a temporary file
    let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
    // Write enough data to span over several chunks when encrypting/decrypting
    tmp_attachment.write(&[1, 2, 3, 4, 5]).unwrap();
    let tmp_attachment_path = tmp_attachment.into_temp_path();

    // Create a card and attach a file
    let card = sdk_a.create_card().unwrap();
    let card = sdk_a.attach_file(&card.id, &tmp_attachment_path).unwrap();
    let ContentView::File(file) = &card.blocks[0].view else {
        panic!("Expected File but got {:?}", card.blocks[0].view)
    };
    assert_eq!("A23RhdnmvLCVNFNp7zQ4aqBEFhf4vFETWky1jYaeBYr", file.checksum);
    assert_eq!(file.size_bytes, 5);

    // Share a card with B
    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card = sdk_a
        .edit_collaborators(
            &card.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_notification().await.unwrap();
    sdk_b.expect_notification().await.unwrap();
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
        assert_eq!(file.metadata().unwrap().len(), 5);
    } else {
        panic!("Expected DownloadCompleted but received {:?}", event);
    }
}

#[tokio::test]
async fn test_multiple_accounts_concurrent_add() {
    // Account 1: Device A and B
    // Account 2: Device C and D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Share card with second account
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_c.sync();
    sdk_c.expect_notification().await.unwrap(); // Expect contact request
    sdk_c.expect_notification().await.unwrap(); // Expect card share
    sdk_c.expect_synced().await.unwrap();

    // A adds B (concurrent)
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    // C adds D (concurrent)
    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    sdk_c.link_devices(&mut sdk_d).await.unwrap();
    let contact_notification = sdk_d.expect_notification().await.unwrap();
    let card_share = sdk_d.expect_notification().await.unwrap(); // Expect card share
    sdk_d.expect_synced().await.unwrap();

    // Sync all devices
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();
    sdk_c.sync();
    sdk_c.expect_synced().await.unwrap();
    sdk_d.sync();
    sdk_d.expect_synced().await.unwrap();

    // Verify account 1 group
    let info_a = sdk_a.account_group().unwrap();
    let info_b = sdk_b.account_group().unwrap();
    assert_eq!(info_a.authentication_secret, info_b.authentication_secret);
    assert_eq!(info_a.devices.len(), 2);
    assert_eq!(info_b.devices.len(), 2);

    // Verify account 2 group
    let info_c = sdk_c.account_group().unwrap();
    let info_d = sdk_d.account_group().unwrap();
    assert_eq!(info_c.authentication_secret, info_d.authentication_secret);
    assert_eq!(info_c.devices.len(), 2);
    assert_eq!(info_d.devices.len(), 2);

    // Verify account 1
    let acc_a = sdk_a.get_account().unwrap();
    let acc_b = sdk_b.get_account().unwrap();
    assert_eq!(acc_a.devices.len(), 2);
    assert_eq!(acc_b.devices.len(), 2);
    assert_eq!(acc_a.contacts.len(), 1);
    assert_eq!(acc_b.contacts.len(), 1);

    // Verify account 2
    let acc_c = sdk_c.get_account().unwrap();
    let acc_d = sdk_d.get_account().unwrap();
    assert_eq!(acc_c.devices.len(), 2);
    assert_eq!(acc_d.devices.len(), 2);
    assert_eq!(acc_c.contacts.len(), 0);
    assert_eq!(acc_d.contacts.len(), 0);

    // All devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    sdk_c.get_card(&card_1.id).unwrap();
    sdk_d.get_card(&card_1.id).unwrap();

    // Accept contact request
    sdk_d
        .accept_notification(&contact_notification.id())
        .await
        .unwrap();
    sdk_d.expect_acc_updated().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Accept card share
    sdk_d.accept_notification(&card_share.id()).await.unwrap();
    sdk_d.expect_doc_updated().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    sdk_c.sync();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_timeline_updated().await.unwrap();
    sdk_c.expect_notifications().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Verify contact group states
    let info_a = sdk_a.contact_group(&acc_2.id).unwrap();
    let info_b = sdk_b.contact_group(&acc_2.id).unwrap();
    let info_c = sdk_c.contact_group(&acc_1.id).unwrap();
    let info_d = sdk_d.contact_group(&acc_1.id).unwrap();
    assert_eq!(info_a.authentication_secret, info_b.authentication_secret);
    assert_eq!(info_c.authentication_secret, info_d.authentication_secret);
    assert_eq!(info_a.authentication_secret, info_c.authentication_secret);
    assert_eq!(info_a.devices.len(), info_b.devices.len());
    assert_eq!(info_c.devices.len(), info_d.devices.len());
    assert_eq!(info_a.devices.len(), info_c.devices.len());

    // Verify timeline
    let expect_ids: &[&str] = &[card_1.id.as_ref()];
    sdk_a.expect_timeline_cards(expect_ids).unwrap();
    sdk_b.expect_timeline_cards(expect_ids).unwrap();
    sdk_c.expect_timeline_cards(expect_ids).unwrap();
    sdk_d.expect_timeline_cards(expect_ids).unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_concurrent_card_edit() {
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1 (devices A and B)
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 2 (device C)
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Share card with second account
    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello world!").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_c.sync();
    let n1 = sdk_c.expect_notification().await.unwrap();
    let n2 = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Accept contact request
    sdk_c.accept_notification(&n1.id()).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Accept card share
    sdk_c.accept_notification(&n2.id()).await.unwrap();
    sdk_c.expect_doc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

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

    sdk_c
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
    sdk_c.close_card(&card_1.id).unwrap();
    assert_eq!(sdk_c.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_c.expect_synced().await.unwrap();

    // First we sync device B so that it sees two versions, one from each account (also doc is new to it)
    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    sdk_a.sync();
    assert_eq!(sdk_a.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_a.expect_synced().await.unwrap();

    // Verify both changes are visible
    let text_a = &sdk_a.get_card(&card_1.id).unwrap().blocks[0].view;
    let text_b = &sdk_b.get_card(&card_1.id).unwrap().blocks[0].view;
    let text_c = &sdk_c.get_card(&card_1.id).unwrap().blocks[0].view;

    for (i, text) in [text_a, text_b, text_c].iter().enumerate() {
        if let ContentView::Text(t) = text {
            assert_eq!("Hi world and Good luck!", t.value, "index={}", i);
        } else {
            panic!("Expected Text content");
        }
    }

    // Verify timeline
    let days_a = sdk_a.timeline_days(vec![]).unwrap();
    let days_b = sdk_b.timeline_days(vec![]).unwrap();
    let days_c = sdk_c.timeline_days(vec![]).unwrap();
    assert_eq!(days_a, days_c);
    assert_eq!(days_a, days_b);
    assert_eq!(days_a.len(), 1);

    let cards_a = sdk_a.timeline_by_day(&days_a[0], vec![]).unwrap();
    let cards_b = sdk_b.timeline_by_day(&days_b[0], vec![]).unwrap();
    let cards_c = sdk_c.timeline_by_day(&days_c[0], vec![]).unwrap();
    assert_eq!(cards_a.cards.len(), cards_c.cards.len());
    assert_eq!(cards_a.cards.len(), cards_b.cards.len());
    assert_eq!(cards_a.cards.len(), 1);

    // Now we have an account, two concurrent docs on the server side and two profiles
    let docs_a = sdk_a
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    assert_eq!(docs_a.docs.len(), 5);
    let docs_c = sdk_a
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    assert_eq!(docs_c.docs.len(), 5);

    // Edit the card to replace two concurrent versions with one
    sdk_a
        .edit_card(&card_1.id, vec![CardChange::append_text(" Fin.")])
        .unwrap();
    sdk_a.close_card(&card_1.id).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let docs_a = sdk_a
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    assert_eq!(docs_a.docs.len(), 4);

    let docs_c = sdk_a
        .client
        .fetch_docs(&DeviceVectorClock::default())
        .await
        .unwrap();
    assert_eq!(docs_c.docs.len(), 4);
}

#[tokio::test]
async fn test_multiple_accounts_permanent_deletion() {
    common::setup();
    let server = common::start_server().await.unwrap();
    // Create account 1 (devices A and B)
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 2 (device C)
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Part one:
    //   Delete for all accounts

    // Create card on A and move to bin
    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello world!").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_a
        .move_card_to_bin(&card_1.id, MoveToBinScope::All)
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // A, B and C should see card in the bin
    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    let n1 = sdk_c.expect_notification().await.unwrap();
    let n2 = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Accept contact request
    sdk_c.accept_notification(&n1.id()).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Accept card share
    sdk_c.accept_notification(&n2.id()).await.unwrap();
    sdk_c.expect_doc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Verify timeline
    let days_a = sdk_a.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    let cards_a = sdk_a
        .timeline_by_day(&days_a[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(1, cards_a.cards.len());
    assert_eq!(card_1.id, cards_a.cards[0].id);

    let cards_b = sdk_b
        .timeline_by_day(&days_a[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(1, cards_b.cards.len());
    assert_eq!(card_1.id, cards_b.cards[0].id);

    let cards_c = sdk_c
        .timeline_by_day(&days_a[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(1, cards_c.cards.len());
    assert_eq!(card_1.id, cards_c.cards[0].id);

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

    // C should still see card in bin
    sdk_c.sync();
    sdk_c.expect_synced().await.unwrap();

    let days_c = sdk_c.timeline_days(vec![BIN_LABEL_ID.into()]).unwrap();
    assert_eq!(1, days_c.len());

    // Part two:
    //   Delete only for this account

    // Create card on C
    let card_2 = sdk_c.create_sample_card("One two!").unwrap();
    sdk_c.expect_synced().await.unwrap();
    let card_2 = sdk_c
        .edit_collaborators(
            &card_2.id,
            HashMap::from([(acc_1.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_c.expect_synced().await.unwrap();

    // A should not be able to move this card to bin for all accounts
    sdk_a.sync();
    let n = sdk_a.expect_notification().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_a.accept_notification(&n.id()).await.unwrap();
    sdk_a.expect_doc_updated().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    assert!(sdk_a
        .move_card_to_bin(&card_2.id, MoveToBinScope::All)
        .is_err());

    // Move card to bin on C
    sdk_c
        .move_card_to_bin(&card_2.id, MoveToBinScope::ThisAccount)
        .unwrap();
    sdk_c.expect_synced().await.unwrap();

    // A and B should see the card not in bin
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_notifications().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    let days_a = sdk_a.timeline_days(vec![]).unwrap();
    let cards_a = sdk_a.timeline_by_day(&days_a[0], vec![]).unwrap();
    assert_eq!(1, cards_a.cards.len());
    assert_eq!(card_2.id, cards_a.cards[0].id);

    let cards_b = sdk_b.timeline_by_day(&days_a[0], vec![]).unwrap();
    assert_eq!(1, cards_b.cards.len());
    assert_eq!(card_2.id, cards_b.cards[0].id);

    // C should see the card in bin (also the one from first part)
    let cards_c = sdk_c
        .timeline_by_day(&days_a[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(2, cards_c.cards.len());
    assert_eq!(card_2.id, cards_c.cards[0].id);
    assert_eq!(card_1.id, cards_c.cards[1].id);

    // Clear bin on C
    sdk_c.empty_bin().unwrap();
    sdk_c.expect_synced().await.unwrap();

    // C should see no cards in bin
    let cards_c = sdk_c
        .timeline_by_day(&days_a[0], vec![BIN_LABEL_ID.into()])
        .unwrap();
    assert_eq!(0, cards_c.cards.len());

    // Sync successfully without timeline updates on other account's devices
    sdk_a.sync();
    sdk_a.expect_synced().await.unwrap();
    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_blobs_cleanup() {
    common::setup();
    let server = common::start_server().await.unwrap();
    // Create account 1 (device A)
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2 (device C)
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create a temporary file
    let mut tmp_attachment = tempfile::NamedTempFile::new().unwrap();
    tmp_attachment.write(&[1, 2, 3, 4, 5]).unwrap();
    let tmp_attachment_path = tmp_attachment.into_temp_path();

    // Create card on A with file attachment
    let card = sdk_a.create_card().unwrap();
    let card = sdk_a.attach_file(&card.id, &tmp_attachment_path).unwrap();
    let card = sdk_a
        .edit_collaborators(
            &card.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Add text on C
    sdk_c.sync();
    sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_c
        .edit_card(&card.id, vec![CardChange::append_text("Hello world!")])
        .unwrap();
    sdk_c.close_card(&card.id).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Try to clean up blobs
    server.app.mark_unused_blobs().unwrap();
    let info = server.app.cleanup_blobs(Some(Utc::now())).await.unwrap();
    assert_eq!(info.removed, 0);
}

#[tokio::test]
async fn test_multiple_accounts_link_same_device() {
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1 (devices A and B)
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let acc_1 = sdk_a.create_account(None).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();

    // Create account 2 (device C)
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let _acc_2 = sdk_c.create_account(None).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Link device B to account 1
    let share = sdk_b.get_device_share().unwrap();
    sdk_b.expect_synced().await.unwrap();

    sdk_a.link_device(&share).await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_connected_account().await.unwrap();
    sdk_b.expect_acc_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Link device B to account 2 (using the same share)
    sdk_c.link_device(&share).await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_synced().await.unwrap();

    // Verify the state
    assert_eq!(acc_1.id, sdk_b.get_account().unwrap().id);
    // NOTE: Ideally, dev B would self-remove itself from the group
}

// TODO: Implement support
// #[tokio::test]
async fn test_multiple_accounts_remove_collaborator() {
    // Account 1: Device A and B
    // Account 2: Device C
    // Account 3: Device D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Create account 3
    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    let acc_3 = sdk_d.create_account(Some("Account 3".into())).unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Share card with both accounts
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_3.id.clone(),
            name: "Custom Account 3".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([
                (acc_2.id.clone(), Some(AclRights::Read)),
                (acc_3.id.clone(), Some(AclRights::Write)),
            ]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Sync
    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    sdk_c.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_c.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_c.expect_synced().await.unwrap();

    sdk_d.sync();
    sdk_d.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_d.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_d.expect_synced().await.unwrap();

    // All devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    sdk_c.get_card(&card_1.id).unwrap();
    sdk_d.get_card(&card_1.id).unwrap();

    // Remove collaborator
    let card_1 = sdk_a
        .edit_collaborators(&card_1.id, HashMap::from([(acc_2.id.clone(), None)]))
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Sync
    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_b.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    sdk_c.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_c.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_c.expect_synced().await.unwrap();

    sdk_d.sync();
    sdk_d.expect_acc_updated().await.unwrap();
    assert_eq!(sdk_d.output().await.unwrap(), OutputEvent::TimelineUpdated);
    sdk_d.expect_synced().await.unwrap();

    // dev B should see the card
    // dev D should see the card
    // dev C should see card moved to the bin
    todo!();
}

#[tokio::test]
async fn test_multiple_accounts_edit_card_by_non_contact() {
    // Account 1: Device A
    // Account 2: Device B
    // Account 3: Device C
    // Account 4: Device D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    let acc_2 = sdk_b.create_account(Some("Account 2".into())).unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 3
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_3 = sdk_c.create_account(Some("Account 3".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Create account 4
    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    let acc_4 = sdk_d.create_account(Some("Account 4".into())).unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Share card with two accounts
    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let _acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_3.id.clone(),
            name: "Custom Account 3".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([
                (acc_2.id.clone(), Some(AclRights::Admin)),
                (acc_3.id.clone(), Some(AclRights::Write)),
            ]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Sync B
    sdk_b.sync();
    sdk_b.expect_notification().await.unwrap();
    let card_share = sdk_b.expect_notification().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Accept card share on B
    sdk_b.accept_notification(&card_share.id()).await.unwrap();
    sdk_b.expect_doc_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Sync C
    sdk_c.sync();
    sdk_c.expect_notification().await.unwrap();
    let card_share = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Accept card share on C
    sdk_c.accept_notification(&card_share.id()).await.unwrap();
    sdk_c.expect_doc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Sync D
    sdk_d.sync();
    sdk_d.expect_synced().await.unwrap();

    // Devices should see the card 1
    sdk_a.get_card(&card_1.id).unwrap();
    sdk_b.get_card(&card_1.id).unwrap();
    sdk_c.get_card(&card_1.id).unwrap();
    assert!(sdk_d.get_card(&card_1.id).is_err());

    // Add new collaborator from another account
    let _acc_2 = sdk_b
        .add_contact(AccContact {
            account_id: acc_4.id.clone(),
            name: "Custom Account 4".into(),
        })
        .await
        .unwrap();
    sdk_b.expect_synced().await.unwrap();
    let card_1 = sdk_b
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_4.id.clone(), Some(AclRights::Write))]),
        )
        .unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Sync D
    sdk_d.sync();
    sdk_d.expect_notification().await.unwrap();
    let card_share = sdk_d.expect_notification().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Accept card share on D
    sdk_d.accept_notification(&card_share.id()).await.unwrap();
    sdk_d.expect_doc_updated().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Edit card on dev D
    sdk_d
        .edit_card(&card_1.id, vec![CardChange::append_text(" Fin.")])
        .unwrap();
    sdk_d.close_card(&card_1.id).unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Sync
    sdk_a.sync();
    sdk_a.expect_timeline_updated().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    sdk_b.sync();
    sdk_b.expect_timeline_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    sdk_c.expect_timeline_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_d.sync();
    sdk_d.expect_synced().await.unwrap();

    // All devices should see the same changes
    let card_a = sdk_a.get_card(&card_1.id).unwrap();
    let card_b = sdk_b.get_card(&card_1.id).unwrap();
    let card_c = sdk_c.get_card(&card_1.id).unwrap();
    let card_d = sdk_d.get_card(&card_1.id).unwrap();
    assert_eq!(card_a.blocks, card_b.blocks);
    assert_eq!(card_b.blocks, card_c.blocks);
    assert_eq!(card_c.blocks, card_d.blocks);

    // Verify timeline
    let expect_ids: &[&str] = &[card_1.id.as_ref()];
    sdk_a.expect_timeline_cards(expect_ids).unwrap();
    sdk_b.expect_timeline_cards(expect_ids).unwrap();
    sdk_c.expect_timeline_cards(expect_ids).unwrap();
    sdk_d.expect_timeline_cards(expect_ids).unwrap();
}

#[tokio::test]
async fn test_multiple_accounts_logout() {
    // Account 1: Device A and B
    // Account 2: Device C
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Link device B
    let mut sdk_b = common::run_sdk("B", &server.addr).await.unwrap();
    sdk_a.link_devices(&mut sdk_b).await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Create a contact
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Sync
    sdk_b.sync();
    sdk_b.expect_acc_updated().await.unwrap();
    sdk_b.expect_synced().await.unwrap();

    sdk_c.sync();
    let notification = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_c.accept_notification(&notification.id()).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Log out on B
    sdk_b.sdk.logout().await;

    // A and C should remove the device
    sdk_c.sync();
    sdk_c.expect_synced().await.unwrap();

    sdk_a.sync();
    sdk_a.expect_acc_updated().await.unwrap();
    sdk_a.expect_synced().await.unwrap();

    let contact_group_a = sdk_a.contact_group(&acc_2.id).unwrap();
    let contact_group_c = sdk_c.contact_group(&acc_1.id).unwrap();
    assert_eq!(contact_group_a.devices.len(), 2);
    assert_eq!(contact_group_c.devices.len(), 2);
    assert_eq!(contact_group_a.group_id, contact_group_c.group_id);
    assert_eq!(
        contact_group_a.authentication_secret,
        contact_group_c.authentication_secret
    );

    let group_a = sdk_a.account_group().unwrap();
    assert_eq!(group_a.devices.len(), 1);

    let acc_1 = sdk_a.get_account().unwrap();
    assert_eq!(acc_1.devices.len(), 1);
}

#[tokio::test]
async fn test_multiple_accounts_local_notification_deleted() {
    // Account 1: Device A
    // Account 2: Device C and D
    common::setup();
    let server = common::start_server().await.unwrap();

    // Create account 1
    let mut sdk_a = common::run_sdk("A", &server.addr).await.unwrap();
    let _acc_1 = sdk_a.create_account(Some("Account 1".into())).unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Create account 2
    let mut sdk_c = common::run_sdk("C", &server.addr).await.unwrap();
    let acc_2 = sdk_c.create_account(Some("Account 2".into())).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Link D
    let mut sdk_d = common::run_sdk("D", &server.addr).await.unwrap();
    sdk_c.link_devices(&mut sdk_d).await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Share card with second account
    let acc_1 = sdk_a
        .add_contact(AccContact {
            account_id: acc_2.id.clone(),
            name: "Custom Account 2".into(),
        })
        .await
        .unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a.create_sample_card("Hello 1").unwrap();
    sdk_a.expect_synced().await.unwrap();
    let card_1 = sdk_a
        .edit_collaborators(
            &card_1.id,
            HashMap::from([(acc_2.id.clone(), Some(AclRights::Read))]),
        )
        .unwrap();
    sdk_a.expect_synced().await.unwrap();

    // Expect notifications on C
    sdk_c.sync();
    let contact_req = sdk_c.expect_notification().await.unwrap();
    let card_share = sdk_c.expect_notification().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    // Expect notifications on D
    sdk_d.sync();
    sdk_d.expect_notification().await.unwrap();
    sdk_d.expect_notification().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // Accept contact and ignore card share
    sdk_c.accept_notification(&contact_req.id()).await.unwrap();
    sdk_c.expect_acc_updated().await.unwrap();
    sdk_c.expect_synced().await.unwrap();

    sdk_c.ignore_notification(&card_share.id()).unwrap();
    sdk_c.expect_synced().await.unwrap();

    // D should remove local notifications
    sdk_d.sync();
    sdk_d.expect_acc_updated().await.unwrap();
    sdk_d.expect_notifications().await.unwrap();
    sdk_d.expect_synced().await.unwrap();

    // No notifications
    let n_c = sdk_c.list_notification_ids().unwrap();
    let n_d = sdk_d.list_notification_ids().unwrap();
    assert_eq!(n_c.len(), 0);
    assert_eq!(n_d.len(), 0);

    // Verify timeline
    sdk_a.expect_timeline_days(1).unwrap();
    sdk_c.expect_timeline_days(0).unwrap();
    sdk_d.expect_timeline_days(0).unwrap();
}

// TODO: after joining a group verify that all account devices are present (maybe one device was added/removed in the meantime)
// TODO: ACL
// TODO: share an account doc (should not mix different accounts)
// TODO: remove device (see how it is removed from all contact groups)
// TODO: verify doc secrets rotate
