use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn test_basic_name_registration() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    helper.initialize_registry(&owner_account).await?;
    // Test with a simple name (within 28 char limit)
    helper
        .register_name_for_account(&user_account, "alice")
        .await?;

    assert!(helper.is_name_registered("alice").await?);
    // Reverse mapping not implemented - should return None
    assert!(!helper.has_name_for_address(&user_account).await?);
    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        None // Returns None since reverse mapping is not implemented
    );

    let (registered_prefix, registered_suffix) = helper
        .get_account_for_name("alice")
        .await?
        .expect("mapping missing");
    assert_eq!(
        registered_prefix,
        user_account.id().prefix().as_felt().as_int()
    );
    assert_eq!(registered_suffix, user_account.id().suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn test_duplicate_name_registration_fails() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let user1_account = helper.create_account("User1").await?;
    let user2_account = helper.create_account("User2").await?;

    helper.initialize_registry(&owner_account).await?;
    helper
        .register_name_for_account(&user1_account, "alice")
        .await?;

    let duplicate_error = helper
        .register_name_for_account(&user2_account, "alice")
        .await
        .expect_err("duplicate registration should fail");
    assert!(
        duplicate_error
            .to_string()
            .to_lowercase()
            .contains("name already registered"),
        "unexpected error message: {duplicate_error}"
    );

    assert!(helper.is_name_registered("alice").await?);
    assert_eq!(
        helper.get_name_for_address(&user1_account).await?,
        Some("alice".to_string())
    );
    assert_eq!(helper.get_name_for_address(&user2_account).await?, None);
    assert!(!helper.has_name_for_address(&user2_account).await?);

    Ok(())
}

#[tokio::test]
async fn test_multiple_names_per_account_fails() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    helper.initialize_registry(&owner_account).await?;
    helper
        .register_name_for_account(&user_account, "alice")
        .await?;

    let error = helper
        .register_name_for_account(&user_account, "bob")
        .await
        .expect_err("same account should not register twice");
    assert!(
        error
            .to_string()
            .to_lowercase()
            .contains("account already has name"),
        "unexpected error message: {error}"
    );

    assert_eq!(
        helper.get_name_for_address(&user_account).await?,
        Some("alice".to_string())
    );
    assert!(!helper.is_name_registered("bob").await?);

    Ok(())
}

#[tokio::test]
async fn test_name_registration_for_multiple_users() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let alice_account = helper.create_account("Alice").await?;
    let bob_account = helper.create_account("Bob").await?;
    let charlie_account = helper.create_account("Charlie").await?;

    helper.initialize_registry(&owner_account).await?;
    helper
        .register_name_for_account(&alice_account, "alice")
        .await?;
    helper
        .register_name_for_account(&bob_account, "bob")
        .await?;
    helper
        .register_name_for_account(&charlie_account, "charlie")
        .await?;

    assert_eq!(
        helper.get_name_for_address(&alice_account).await?,
        Some("alice".to_string())
    );
    assert_eq!(
        helper.get_name_for_address(&bob_account).await?,
        Some("bob".to_string())
    );
    assert_eq!(
        helper.get_name_for_address(&charlie_account).await?,
        Some("charlie".to_string())
    );

    let alice_lookup = helper.get_account_for_name("alice").await?.unwrap();
    assert_eq!(
        alice_lookup.0,
        alice_account.id().prefix().as_felt().as_int()
    );
    assert_eq!(alice_lookup.1, alice_account.id().suffix().as_int());

    let bob_lookup = helper.get_account_for_name("bob").await?.unwrap();
    assert_eq!(bob_lookup.0, bob_account.id().prefix().as_felt().as_int());
    assert_eq!(bob_lookup.1, bob_account.id().suffix().as_int());

    let charlie_lookup = helper.get_account_for_name("charlie").await?.unwrap();
    assert_eq!(
        charlie_lookup.0,
        charlie_account.id().prefix().as_felt().as_int()
    );
    assert_eq!(charlie_lookup.1, charlie_account.id().suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn test_unregistered_queries_are_empty() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let user_account = helper.create_account("User").await?;

    helper.initialize_registry(&owner_account).await?;

    assert!(!helper.is_name_registered("unknown").await?);
    assert_eq!(helper.get_name_for_address(&user_account).await?, None);
    assert!(!helper.has_name_for_address(&user_account).await?);
    assert_eq!(helper.get_account_for_name("unknown").await?, None);

    Ok(())
}

#[tokio::test]
async fn test_owner_can_register_name() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    helper.initialize_registry(&owner_account).await?;
    helper
        .register_name_for_account(&owner_account, "admin")
        .await?;

    assert!(helper.is_name_registered("admin").await?);
    assert_eq!(
        helper.get_name_for_address(&owner_account).await?,
        Some("admin".to_string())
    );

    let (prefix, suffix) = helper.get_account_for_name("admin").await?.unwrap();
    assert_eq!(prefix, owner_account.id().prefix().as_felt().as_int());
    assert_eq!(suffix, owner_account.id().suffix().as_int());

    Ok(())
}
