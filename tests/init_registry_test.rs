use miden_client::ClientError;

mod test_helper;
use test_helper::{RegistryTestHelper, get_or_init_shared_contract, setup_helper_with_contract};

#[tokio::test]
async fn init_registry_with_faucet() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_id.prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_id.suffix().as_int());

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, faucet_id.prefix().as_felt().as_int());
        assert_eq!(token_suffix, faucet_id.suffix().as_int());

        let price = helper.get_price(&contract_state);
        assert_eq!(price, 100);
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn init_registry_without_faucet() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new_persistent().await?;
    helper.sync_network().await?;

    let owner = helper.create_account("OwnerNoFaucet").await?;
    let registry = helper.deploy_registry_contract().await?;

    helper.sync_network().await?;
    helper.initialize_registry(&owner).await?;
    helper.sync_network().await?;

    let contract_state = helper.client.get_account(registry.id()).await?.unwrap();
    let (initialized, owner_prefix, owner_suffix) =
        helper.get_initialization_state(&contract_state);

    assert_eq!(initialized, 1, "Contract should be initialized");
    assert_eq!(owner_prefix, owner.id().prefix().as_felt().as_int());
    assert_eq!(owner_suffix, owner.id().suffix().as_int());

    let price = helper.get_price(&contract_state);
    assert_eq!(price, 0, "Price should be 0 when no faucet");

    Ok(())
}
