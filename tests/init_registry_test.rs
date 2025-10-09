use miden_client::ClientError;

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn init_registry_with_note() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;

    helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int());

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, faucet_account.id().prefix().as_felt().as_int());
        assert_eq!(token_suffix, faucet_account.id().suffix().as_int());

        let price_word = contract_state.account().storage().get_item(5).unwrap();
        let price = price_word.get(0).unwrap().as_int();
        assert_eq!(price, 100);
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}