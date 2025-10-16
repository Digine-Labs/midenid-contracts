mod helpers;

use helpers::{
    RegistryTestHelper, create_basic_wallet, deploy_registry, fetch_registry_account,
    get_initialization_state, get_or_init_shared_contract, get_payment_token_state, get_price,
    initialize_registry, is_name_registered, mint_and_fund_account, register_name,
    setup_helper_with_contract,
};
use miden_client::ClientError;

#[tokio::test]
async fn test_deployment_flow() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    if let Some(contract_state) = fetch_registry_account(&mut helper.client, contract_id).await? {
        let (initialized, owner_prefix, owner_suffix) = get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Registry should be initialized");
        assert_eq!(
            owner_prefix,
            owner_id.prefix().as_felt().as_int(),
            "Owner prefix mismatch"
        );
        assert_eq!(
            owner_suffix,
            owner_id.suffix().as_int(),
            "Owner suffix mismatch"
        );

        let (token_prefix, token_suffix) = get_payment_token_state(&contract_state);
        assert_eq!(
            token_prefix,
            faucet_id.prefix().as_felt().as_int(),
            "Payment token prefix mismatch"
        );
        assert_eq!(
            token_suffix,
            faucet_id.suffix().as_int(),
            "Payment token suffix mismatch"
        );

        let price = get_price(&contract_state);
        assert_eq!(price, 100, "Price should be 100");
    } else {
        panic!("Contract state not found after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn test_deployment_verification_checks() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let owner_exists = helper.client.get_account(owner_id).await?.is_some();
    let faucet_exists = helper.client.get_account(faucet_id).await?.is_some();
    let registry_exists = helper.client.get_account(contract_id).await?.is_some();

    assert!(owner_exists, "Owner account should exist");
    assert!(faucet_exists, "Faucet account should exist");
    assert!(registry_exists, "Registry contract should exist");

    let contract_state = fetch_registry_account(&mut helper.client, contract_id)
        .await?
        .unwrap();
    let (initialized, owner_prefix, owner_suffix) = get_initialization_state(&contract_state);

    assert_eq!(initialized, 1, "Registry should be initialized");
    assert_eq!(owner_prefix, owner_id.prefix().as_felt().as_int());
    assert_eq!(owner_suffix, owner_id.suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn test_registry_ready_for_registration_after_deployment() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user =
        create_basic_wallet(&mut helper.client, helper.keystore.clone(), "DeployUser").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 200).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "deploytest",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let is_registered = is_name_registered(&mut helper.client, contract_id, "deploytest").await?;
    assert!(is_registered, "Name 'deploytest' should be registered");

    Ok(())
}

#[tokio::test]
async fn test_deployment_with_zero_price() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new_persistent().await?;
    helper.client.sync_state().await?;

    let owner = create_basic_wallet(
        &mut helper.client,
        helper.keystore.clone(),
        "OwnerZeroPrice",
    )
    .await?;
    let registry = deploy_registry(&mut helper.client).await?;

    helper.client.sync_state().await?;
    initialize_registry(&mut helper.client, registry.id(), &owner, None, 0).await?;
    helper.client.sync_state().await?;

    let contract_state = helper.client.get_account(registry.id()).await?.unwrap();
    let price = get_price(&contract_state);

    assert_eq!(price, 0, "Price should be 0");

    Ok(())
}
