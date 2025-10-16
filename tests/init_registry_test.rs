use miden_client::ClientError;

mod helpers;
use helpers::{
    RegistryTestHelper, create_basic_wallet, deploy_registry, fetch_registry_account,
    get_initialization_state, get_or_init_shared_contract, get_payment_token_state, get_price,
    initialize_registry, setup_helper_with_contract,
};

#[tokio::test]
async fn init_registry_with_faucet() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    if let Some(contract_state) = fetch_registry_account(&mut helper.client, contract_id).await? {
        let (initialized, owner_prefix, owner_suffix) = get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Contract should be initialized");
        assert_eq!(owner_prefix, owner_id.prefix().as_felt().as_int());
        assert_eq!(owner_suffix, owner_id.suffix().as_int());

        let (token_prefix, token_suffix) = get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, faucet_id.prefix().as_felt().as_int());
        assert_eq!(token_suffix, faucet_id.suffix().as_int());

        let price = get_price(&contract_state);
        assert_eq!(price, 100);
    } else {
        panic!("Contract state should be available after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn init_registry_without_faucet() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new_persistent().await?;
    helper.client.sync_state().await?;

    let owner =
        create_basic_wallet(&mut helper.client, helper.keystore.clone(), "OwnerNoFaucet").await?;
    let registry = deploy_registry(&mut helper.client).await?;

    helper.client.sync_state().await?;
    initialize_registry(&mut helper.client, registry.id(), &owner, None, 0).await?;
    helper.client.sync_state().await?;

    let contract_state = helper.client.get_account(registry.id()).await?.unwrap();
    let (initialized, owner_prefix, owner_suffix) = get_initialization_state(&contract_state);

    assert_eq!(initialized, 1, "Contract should be initialized");
    assert_eq!(owner_prefix, owner.id().prefix().as_felt().as_int());
    assert_eq!(owner_suffix, owner.id().suffix().as_int());

    let price = get_price(&contract_state);
    assert_eq!(price, 0, "Price should be 0 when no faucet");

    Ok(())
}
