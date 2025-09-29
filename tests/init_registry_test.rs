use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn init_registry_with_note() -> Result<(), ClientError> {
    // Setup environment with deployed contract
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    // Initialize the registry with the owner account
    helper.initialize_registry(&owner_account).await?;

    // Validate the registry was initialized correctly
    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

        // Verify initialization flag is set
        assert_eq!(initialized, 1, "Contract should be initialized");

        // Verify owner is set correctly
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());

        // Verify payment token configuration from init note (1234, 6789)
        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, 1234, "Payment token prefix should be 1234");
        assert_eq!(token_suffix, 6789, "Payment token suffix should be 6789");

        // Verify price is set (500 from init note)
        let price_word = contract_state.account().storage().get_item(5).unwrap();
        let price = price_word.get(0).unwrap().as_int();
        assert_eq!(price, 500, "Price should be 500");
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn init_registry_complete_flow() -> Result<(), ClientError> {
    // Setup environment and create accounts
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    // Initialize registry
    helper.initialize_registry(&owner_account).await?;

    // Validate complete contract state
    if let Some(contract_state) = helper.get_contract_state().await? {
        let complete_state = helper.get_complete_contract_state(&contract_state);

        // Check all storage slots are properly configured
        assert_eq!(complete_state.initialized, 1, "Contract should be initialized");
        assert_eq!(complete_state.owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(complete_state.owner_suffix, owner_account.id().suffix().as_int());
        assert_eq!(complete_state.token_prefix, 1234, "Payment token prefix should be 1234");
        assert_eq!(complete_state.token_suffix, 6789, "Payment token suffix should be 6789");

        // Note: Map slots (3, 4) contain initial map root hashes, which is normal
        // The actual name mappings will be stored within these maps when names are registered

        println!("✅ Registry initialization complete with all storage slots properly configured");
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn test_double_initialization_different_accounts() -> Result<(), ClientError> {
    // Setup environment with deployed contract
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let unauthorized_account = helper.create_account("Unauthorized User").await?;

    // Initialize with owner
    helper.initialize_registry(&owner_account).await?;

    // Verify initialization
    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    // Attempt second initialization with different account
    let double_init_result = helper.initialize_registry(&unauthorized_account).await;

    match double_init_result {
        Ok(_) => {
            panic!("Second initialization unexpectedly succeeded - should have been prevented!");
        }
        Err(e) => {
            println!("Double initialization correctly prevented: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_double_initialization_same_account() -> Result<(), ClientError> {
    // Setup environment with deployed contract
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    // Initialize with owner
    helper.initialize_registry(&owner_account).await?;

    // Verify initialization
    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    // Attempt second initialization with same account
    let double_init_result = helper.initialize_registry(&owner_account).await;

    match double_init_result {
        Ok(_) => {
            panic!(
                "Second initialization by same account unexpectedly succeeded - should have been prevented!"
            );
        }
        Err(e) => {
            println!("Double initialization correctly prevented: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_owner_validation() -> Result<(), ClientError> {
    // Setup environment and create accounts
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    // Initialize registry
    helper.initialize_registry(&owner_account).await?;

    // Verify owner is correctly set
    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());
    }

    // TODO: Add tests for owner-only function calls when note scripts are ready

    Ok(())
}

#[tokio::test]
#[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
async fn test_initialization_fails_without_deployed_contract() {
    // Create helper but don't deploy contract
    let mut helper = RegistryTestHelper::new().await.unwrap();
    helper.sync_network().await.unwrap();

    let owner = helper.create_account("Owner").await.unwrap();

    // This should panic because no contract is deployed
    // The panic happens in initialize_registry when it tries to unwrap registry_contract
    helper.initialize_registry(&owner).await.unwrap();
}

#[tokio::test]
async fn test_complete_contract_state_validation() -> Result<(), ClientError> {
    // Setup environment and initialize contract
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    helper.initialize_registry(&owner_account).await?;

    // Get complete contract state
    if let Some(contract_state) = helper.get_contract_state().await? {
        let state = helper.get_complete_contract_state(&contract_state);

        // Validate initialization state (slot 0)
        assert_eq!(state.initialized, 1, "Contract should be initialized");

        // Validate owner state (slot 1)
        assert_eq!(
            state.owner_prefix,
            owner_account.id().prefix().as_felt().as_int(),
            "Owner prefix should match"
        );
        assert_eq!(
            state.owner_suffix,
            owner_account.id().suffix().as_int(),
            "Owner suffix should match"
        );

        // Validate payment token state (slot 2)
        assert_eq!(
            state.token_prefix, 1234,
            "Payment token prefix should be 1234"
        );
        assert_eq!(
            state.token_suffix, 6789,
            "Payment token suffix should be 6789"
        );

        // Note: Slots 3-4 use map storage (set_map_item/get_map_item) which stores
        // Merkle tree metadata, not simple values. Even empty maps have non-zero
        // internal structure, so we can't check for "empty" using get_item.
        // Map emptiness should be verified by querying specific keys, not reading the slot directly.
    } else {
        panic!(
            "Failed to get contract state - this should not happen after successful initialization"
        );
    }

    Ok(())
}