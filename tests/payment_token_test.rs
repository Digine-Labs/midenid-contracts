use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn test_payment_token_initialization() -> Result<(), ClientError> {
    // Setup environment and create accounts
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;

    // Initialize registry
    helper.initialize_registry(&owner_account).await?;

    // Validate the contract state
    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);
        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, 1234, "Payment token prefix should be 1234");
        assert_eq!(token_suffix, 6789, "Payment token suffix should be 6789");
    }

    Ok(())
}
