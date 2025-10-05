use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn test_register_name_creates_bidirectional_mapping() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;
    let user_account = helper.create_account("User").await?;
    helper
        .register_name_for_account(&user_account, "alice")
        .await?;

    // Verify name is registered in slot 3 (Name→ID)
    assert!(
        helper.is_name_registered("alice").await?,
        "name 'alice' should be registered in slot 3"
    );

    // Verify reverse mapping exists in slot 4 (ID→Name)
    assert!(
        helper.has_name_for_address(&user_account).await?,
        "user account should have name in slot 4"
    );
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("alice".to_string()),
        "reverse lookup should return 'alice'"
    );

    // Verify forward mapping returns correct account ID
    let (registered_prefix, registered_suffix) = helper
        .get_account_for_name("alice")
        .await?
        .expect("forward lookup should return account ID");
    assert_eq!(
        registered_prefix,
        user_account.id().prefix().as_felt().as_int(),
        "registered prefix should match user account prefix"
    );
    assert_eq!(
        registered_suffix,
        user_account.id().suffix().as_int(),
        "registered suffix should match user account suffix"
    );

    Ok(())
}

#[tokio::test]
async fn test_cannot_register_same_name_twice() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;
    let user1_account = helper.create_account("User1").await?;
    let user2_account = helper.create_account("User2").await?;
    helper
        .register_name_for_account(&user1_account, "alice")
        .await?;

    let duplicate_result = helper
        .register_name_for_account(&user2_account, "alice")
        .await;

    // Should fail (either with our error message or a transaction executor error)
    assert!(
        duplicate_result.is_err(),
        "duplicate registration should fail but succeeded"
    );

    // The error message should contain our constant or be a transaction error
    let error_msg = duplicate_result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("name already registered") || error_msg.contains("transaction"),
        "unexpected error message: {}",
        error_msg
    );

    // Verify only user1 has the name
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    let user_account = helper.create_account("User").await?;

    helper
        .register_name_for_account(&user_account, "alice")
        .await?;

    let error = helper
        .register_name_for_account(&user_account, "bob")
        .await
        .expect_err("same account should not register twice");
    // Error message from MASM assert.err= may not propagate as readable string
    // Just verify that registration failed
    let error_msg = error.to_string().to_lowercase();
    assert!(
        error_msg.contains("account already has name") || error_msg.contains("transaction"),
        "unexpected error message: {error}"
    );

    // Verify user still has alice, not bob
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    let alice_account = helper.create_account("Alice").await?;
    let bob_account = helper.create_account("Bob").await?;
    let charlie_account = helper.create_account("Charlie").await?;

    helper
        .register_name_for_account(&alice_account, "alice")
        .await?;
    helper
        .register_name_for_account(&bob_account, "bob")
        .await?;
    helper
        .register_name_for_account(&charlie_account, "charlie")
        .await?;

    // Verify reverse lookups (ID→Name)
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

    // Verify forward lookups (Name→ID)
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    let user_account = helper.create_account("User").await?;

    // Verify slot 3 (Name→ID) returns empty for unregistered name
    assert!(
        !helper.is_name_registered("unknown").await?,
        "unregistered name should not be found"
    );
    assert_eq!(
        helper.get_account_for_name("unknown").await?,
        None,
        "unregistered name should return None"
    );

    // Verify slot 4 (ID→Name) returns empty for address with no name
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    helper
        .register_name_for_account(&owner_account, "admin")
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    let alice_account = helper.create_account("Alice").await?;
    let bob_account = helper.create_account("Bob").await?;

    // Register names for both accounts
    helper
        .register_name_for_account(&alice_account, "alice")
        .await?;
    helper
        .register_name_for_account(&bob_account, "bob")
        .await?;

    // Test slot 3 (Name→ID) for alice
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

    // Test slot 4 (ID→Name) for alice
    assert!(
        helper.has_name_for_address(&alice_account).await?,
        "alice account should have name in slot 4"
    );
    assert_eq!(
        helper.get_name_for_address(&alice_account).await?,
        Some("alice".to_string()),
        "alice reverse lookup should return 'alice'"
    );

    // Test slot 3 (Name→ID) for bob
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

    // Test slot 4 (ID→Name) for bob
    assert!(
        helper.has_name_for_address(&bob_account).await?,
        "bob account should have name in slot 4"
    );
    assert_eq!(
        helper.get_name_for_address(&bob_account).await?,
        Some("bob".to_string()),
        "bob reverse lookup should return 'bob'"
    );

    // Verify slot 4 doesn't return wrong names (cross-contamination check)
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
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    let user_account = helper.create_account("User").await?;

    // Before registration: slot 4 should be empty
    assert!(
        !helper.has_name_for_address(&user_account).await?,
        "user should not have name before registration"
    );
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        None,
        "reverse lookup should return None before registration"
    );

    // Register name
    helper
        .register_name_for_account(&user_account, "testuser")
        .await?;

    // After registration: slot 4 should contain the name
    assert!(
        helper.has_name_for_address(&user_account).await?,
        "user should have name after registration"
    );
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("testuser".to_string()),
        "reverse lookup should return 'testuser' after registration"
    );

    // Verify slot 3 also works (Name→ID)
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
