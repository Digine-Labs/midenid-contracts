use miden_client::{ClientError, transaction::TransactionRequestBuilder};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn test_register_name_creates_bidirectional_mapping() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    let fungible_asset = FungibleAsset::new(faucet_account.id(), 100).unwrap();
    let tx_req = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset,
            user_account.id(),
            NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();
    helper
        .client
        .submit_new_transaction(faucet_account.id(), tx_req)
        .await?;

    let list_of_note_ids = loop {
        helper.client.sync_state().await?;
        let consumable_notes = helper
            .client
            .get_consumable_notes(Some(user_account.id()))
            .await?;
        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();
        if !note_ids.is_empty() {
            break note_ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(list_of_note_ids)
        .unwrap();
    helper
        .client
        .submit_new_transaction(user_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let user_record = helper.client.get_account(user_account.id()).await?.unwrap();
    let user_account = user_record.into();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;
    helper
        .register_name_for_account_with_payment(&user_account, "alice", Some(100))
        .await?;

    assert!(helper.is_name_registered("alice").await?);
    assert!(helper.has_name_for_address(&user_account).await?);
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("alice".to_string())
    );

    let (registered_prefix, registered_suffix) =
        helper.get_account_for_name("alice").await?.unwrap();
    assert_eq!(
        registered_prefix,
        user_account.id().prefix().as_felt().as_int()
    );
    assert_eq!(registered_suffix, user_account.id().suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn test_cannot_register_same_name_twice() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let user1_account = helper.create_account("User1").await?;
    let user2_account = helper.create_account("User2").await?;

    for user in [&user1_account, &user2_account] {
        let fungible_asset =
            FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
        let tx_req = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                user.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();
        helper
            .client
            .submit_new_transaction(faucet_account.id(), tx_req)
            .await?;
    }

    let user1_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(user1_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(user1_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(user1_account.id(), tx_req)
        .await?;

    let user2_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(user2_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(user2_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(user2_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let user1_record = helper
        .client
        .get_account(user1_account.id())
        .await?
        .unwrap();
    let user1_account: miden_client::account::Account = user1_record.into();
    let user2_record = helper
        .client
        .get_account(user2_account.id())
        .await?
        .unwrap();
    let user2_account: miden_client::account::Account = user2_record.into();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    helper
        .register_name_for_account_with_payment(&user1_account, "alice", Some(100))
        .await?;

    let duplicate_result = helper
        .register_name_for_account_with_payment(&user2_account, "alice", Some(100))
        .await;

    assert!(
        duplicate_result.is_err(),
        "duplicate registration should fail but succeeded"
    );

    let error_msg = duplicate_result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("name already registered") || error_msg.contains("transaction"),
        "unexpected error message: {}",
        error_msg
    );

    assert!(
        helper.is_name_registered("alice").await?,
        "alice should remain registered"
    );
    assert_eq!(
        helper.get_name_for_address(&user1_account).await?,
        Some("alice".to_string()),
        "user1 should still have alice"
    );
    assert_eq!(
        helper.get_name_for_address(&user2_account).await?,
        None,
        "user2 should not have any name"
    );
    assert!(
        !helper.has_name_for_address(&user2_account).await?,
        "user2 should not have name registered"
    );

    Ok(())
}

#[tokio::test]
async fn test_account_can_only_register_one_name() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    for _ in 0..2 {
        let fungible_asset =
            FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
        let tx_req = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                user_account.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();
        helper
            .client
            .submit_new_transaction(faucet_account.id(), tx_req)
            .await?;
    }

    let user_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(user_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if ids.len() >= 2 {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(user_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(user_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let user_record = helper.client.get_account(user_account.id()).await?.unwrap();
    let user_account: miden_client::account::Account = user_record.into();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    helper
        .register_name_for_account_with_payment(&user_account, "alice", Some(100))
        .await?;

    let error = helper
        .register_name_for_account_with_payment(&user_account, "bob", Some(100))
        .await
        .expect_err("same account should not register twice");
    let error_msg = error.to_string().to_lowercase();
    assert!(
        error_msg.contains("account already has name") || error_msg.contains("transaction"),
        "unexpected error message: {error}"
    );

    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("alice".to_string()),
        "user should still have 'alice' registered"
    );
    assert!(
        !helper.is_name_registered("bob").await?,
        "bob should not be registered"
    );

    Ok(())
}

#[tokio::test]
async fn test_multiple_accounts_can_register_different_names() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let alice_account = helper.create_account("Alice").await?;
    let bob_account = helper.create_account("Bob").await?;
    let charlie_account = helper.create_account("Charlie").await?;

    for user in [&alice_account, &bob_account, &charlie_account] {
        let fungible_asset =
            FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
        let tx_req = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                user.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();
        helper
            .client
            .submit_new_transaction(faucet_account.id(), tx_req)
            .await?;
    }

    let mut updated_accounts: Vec<miden_client::account::Account> = Vec::new();
    for user in [alice_account, bob_account, charlie_account] {
        let notes = loop {
            helper.client.sync_state().await?;
            let consumable = helper.client.get_consumable_notes(Some(user.id())).await?;
            let ids: Vec<_> = consumable.iter().map(|(n, _)| n.id()).collect();
            if !ids.is_empty() {
                break ids;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        };
        let tx_req = TransactionRequestBuilder::new()
            .build_consume_notes(notes)
            .unwrap();
        helper
            .client
            .submit_new_transaction(user.id(), tx_req)
            .await?;

        helper.sync_network().await?;
        let record = helper.client.get_account(user.id()).await?.unwrap();
        updated_accounts.push(record.into());
    }

    let alice_account: miden_client::account::Account = updated_accounts[0].clone();
    let bob_account: miden_client::account::Account = updated_accounts[1].clone();
    let charlie_account: miden_client::account::Account = updated_accounts[2].clone();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    helper
        .register_name_for_account_with_payment(&alice_account, "alice", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&bob_account, "bob", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&charlie_account, "charlie", Some(100))
        .await?;

    assert_eq!(
        helper.get_name_for_address(&alice_account).await?,
        Some("alice".to_string()),
        "alice account should return 'alice'"
    );
    assert_eq!(
        helper.get_name_for_address(&bob_account).await?,
        Some("bob".to_string()),
        "bob account should return 'bob'"
    );
    assert_eq!(
        helper.get_name_for_address(&charlie_account).await?,
        Some("charlie".to_string()),
        "charlie account should return 'charlie'"
    );

    let alice_lookup = helper
        .get_account_for_name("alice")
        .await?
        .expect("alice should be registered");
    assert_eq!(
        alice_lookup.0,
        alice_account.id().prefix().as_felt().as_int(),
        "alice prefix mismatch"
    );
    assert_eq!(
        alice_lookup.1,
        alice_account.id().suffix().as_int(),
        "alice suffix mismatch"
    );

    let bob_lookup = helper
        .get_account_for_name("bob")
        .await?
        .expect("bob should be registered");
    assert_eq!(
        bob_lookup.0,
        bob_account.id().prefix().as_felt().as_int(),
        "bob prefix mismatch"
    );
    assert_eq!(
        bob_lookup.1,
        bob_account.id().suffix().as_int(),
        "bob suffix mismatch"
    );

    let charlie_lookup = helper
        .get_account_for_name("charlie")
        .await?
        .expect("charlie should be registered");
    assert_eq!(
        charlie_lookup.0,
        charlie_account.id().prefix().as_felt().as_int(),
        "charlie prefix mismatch"
    );
    assert_eq!(
        charlie_lookup.1,
        charlie_account.id().suffix().as_int(),
        "charlie suffix mismatch"
    );

    Ok(())
}

#[tokio::test]
async fn test_unregistered_names_and_addresses_return_none() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    assert!(
        !helper.is_name_registered("unknown").await?,
        "unregistered name should not be found"
    );
    assert_eq!(
        helper.get_account_for_name("unknown").await?,
        None,
        "unregistered name should return None"
    );

    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        None,
        "address with no name should return None"
    );
    assert!(
        !helper.has_name_for_address(&user_account).await?,
        "address should not have name"
    );

    Ok(())
}

#[tokio::test]
async fn test_registry_owner_can_register_name_for_themselves() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;

    let fungible_asset =
        FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
    let tx_req = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset,
            owner_account.id(),
            NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();
    helper
        .client
        .submit_new_transaction(faucet_account.id(), tx_req)
        .await?;

    let owner_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(owner_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(owner_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(owner_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let owner_record = helper
        .client
        .get_account(owner_account.id())
        .await?
        .unwrap();
    let owner_account: miden_client::account::Account = owner_record.into();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    helper
        .register_name_for_account_with_payment(&owner_account, "admin", Some(100))
        .await?;

    assert!(
        helper.is_name_registered("admin").await?,
        "admin should be registered"
    );
    assert_eq!(
        helper.get_name_for_address(&owner_account).await?,
        Some("admin".to_string()),
        "owner should have 'admin' name"
    );

    let (prefix, suffix) = helper
        .get_account_for_name("admin")
        .await?
        .expect("admin should resolve to owner account");
    assert_eq!(
        prefix,
        owner_account.id().prefix().as_felt().as_int(),
        "prefix should match owner"
    );
    assert_eq!(
        suffix,
        owner_account.id().suffix().as_int(),
        "suffix should match owner"
    );

    Ok(())
}

#[tokio::test]
async fn test_name_to_id_and_id_to_name_mappings_stay_consistent() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let alice_account = helper.create_account("Alice").await?;
    let bob_account = helper.create_account("Bob").await?;

    for user in [&alice_account, &bob_account] {
        let fungible_asset =
            FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
        let tx_req = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                user.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();
        helper
            .client
            .submit_new_transaction(faucet_account.id(), tx_req)
            .await?;
    }

    let alice_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(alice_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(alice_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(alice_account.id(), tx_req)
        .await?;

    let bob_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(bob_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(bob_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(bob_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account: miden_client::account::Account = alice_record.into();
    let bob_record = helper.client.get_account(bob_account.id()).await?.unwrap();
    let bob_account: miden_client::account::Account = bob_record.into();

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    helper
        .register_name_for_account_with_payment(&alice_account, "alice", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&bob_account, "bob", Some(100))
        .await?;

    assert!(
        helper.is_name_registered("alice").await?,
        "alice should be registered in slot 3"
    );
    let alice_lookup = helper
        .get_account_for_name("alice")
        .await?
        .expect("alice should resolve to account");
    assert_eq!(
        alice_lookup.0,
        alice_account.id().prefix().as_felt().as_int(),
        "alice forward lookup prefix mismatch"
    );
    assert_eq!(
        alice_lookup.1,
        alice_account.id().suffix().as_int(),
        "alice forward lookup suffix mismatch"
    );

    assert!(
        helper.has_name_for_address(&alice_account).await?,
        "alice account should have name in slot 4"
    );
    assert_eq!(
        helper.get_name_for_address(&alice_account).await?,
        Some("alice".to_string()),
        "alice reverse lookup should return 'alice'"
    );

    assert!(
        helper.is_name_registered("bob").await?,
        "bob should be registered in slot 3"
    );
    let bob_lookup = helper
        .get_account_for_name("bob")
        .await?
        .expect("bob should resolve to account");
    assert_eq!(
        bob_lookup.0,
        bob_account.id().prefix().as_felt().as_int(),
        "bob forward lookup prefix mismatch"
    );
    assert_eq!(
        bob_lookup.1,
        bob_account.id().suffix().as_int(),
        "bob forward lookup suffix mismatch"
    );

    assert!(
        helper.has_name_for_address(&bob_account).await?,
        "bob account should have name in slot 4"
    );
    assert_eq!(
        helper.get_name_for_address(&bob_account).await?,
        Some("bob".to_string()),
        "bob reverse lookup should return 'bob'"
    );

    let alice_name = helper.get_name_for_address(&alice_account).await?;
    let bob_name = helper.get_name_for_address(&bob_account).await?;
    assert_ne!(
        alice_name, bob_name,
        "alice and bob should have different names"
    );
    assert_eq!(
        alice_name,
        Some("alice".to_string()),
        "alice should still be 'alice'"
    );
    assert_eq!(
        bob_name,
        Some("bob".to_string()),
        "bob should still be 'bob'"
    );

    Ok(())
}

#[tokio::test]
async fn test_address_has_no_name_before_registration_and_has_name_after() -> Result<(), ClientError>
{
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    assert!(
        !helper.has_name_for_address(&user_account).await?,
        "user should not have name before registration"
    );
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        None,
        "reverse lookup should return None before registration"
    );

    let fungible_asset =
        FungibleAsset::new(faucet_account.id(), 100).expect("Failed to create fungible asset");
    let tx_req = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset,
            user_account.id(),
            NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();
    helper
        .client
        .submit_new_transaction(faucet_account.id(), tx_req)
        .await?;

    let user_notes = loop {
        helper.client.sync_state().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(user_account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };
    let tx_req = TransactionRequestBuilder::new()
        .build_consume_notes(user_notes)
        .unwrap();
    helper
        .client
        .submit_new_transaction(user_account.id(), tx_req)
        .await?;

    helper.sync_network().await?;
    let user_record = helper.client.get_account(user_account.id()).await?.unwrap();
    let user_account: miden_client::account::Account = user_record.into();

    helper
        .register_name_for_account_with_payment(&user_account, "testuser", Some(100))
        .await?;

    assert!(
        helper.has_name_for_address(&user_account).await?,
        "user should have name after registration"
    );
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("testuser".to_string()),
        "reverse lookup should return 'testuser' after registration"
    );

    assert!(
        helper.is_name_registered("testuser").await?,
        "testuser should be registered in slot 3"
    );
    let (prefix, suffix) = helper
        .get_account_for_name("testuser")
        .await?
        .expect("testuser should resolve to user account");
    assert_eq!(
        prefix,
        user_account.id().prefix().as_felt().as_int(),
        "forward lookup prefix should match"
    );
    assert_eq!(
        suffix,
        user_account.id().suffix().as_int(),
        "forward lookup suffix should match"
    );

    Ok(())
}
