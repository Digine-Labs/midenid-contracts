use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn init_registry_with_note() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(token_suffix, owner_account.id().suffix().as_int());

        let price_word = contract_state.account().storage().get_item(5).unwrap();
        let price = price_word.get(0).unwrap().as_int();
        assert_eq!(price, 0);
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn init_registry_complete_flow() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let complete_state = helper.get_complete_contract_state(&contract_state);

        assert_eq!(complete_state.initialized, 1);
        assert_eq!(complete_state.owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(complete_state.owner_suffix, owner_account.id().suffix().as_int());
        assert_eq!(complete_state.token_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(complete_state.token_suffix, owner_account.id().suffix().as_int());
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn test_double_initialization_different_accounts() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let unauthorized_account = helper.create_account("Unauthorized User").await?;

    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1);
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    let double_init_result = helper.initialize_registry(&unauthorized_account).await;

    assert!(double_init_result.is_err(), "Second initialization should fail");

    Ok(())
}

#[tokio::test]
async fn test_double_initialization_same_account() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1);
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    let double_init_result = helper.initialize_registry(&owner_account).await;

    assert!(double_init_result.is_err(), "Second initialization should fail");

    Ok(())
}

#[tokio::test]
async fn test_owner_validation() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1);
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    Ok(())
}

#[tokio::test]
#[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
async fn test_initialization_fails_without_deployed_contract() {
    let mut helper = RegistryTestHelper::new().await.unwrap();
    helper.sync_network().await.unwrap();
    let owner = helper.create_account("Owner").await.unwrap();
    helper.initialize_registry(&owner).await.unwrap();
}

#[tokio::test]
async fn test_complete_contract_state_validation() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let state = helper.get_complete_contract_state(&contract_state);

        assert_eq!(state.initialized, 1);
        assert_eq!(state.owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(state.owner_suffix, owner_account.id().suffix().as_int());
        assert_eq!(state.token_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(state.token_suffix, owner_account.id().suffix().as_int());
    } else {
        panic!("Failed to get contract state");
    }

    Ok(())
}
