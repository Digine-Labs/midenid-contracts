mod test_helper;

use miden_client::ClientError;
use test_helper::{
    RegistryTestHelper, get_or_init_shared_contract, mint_and_fund_account,
    setup_helper_with_contract,
};

#[tokio::test]
async fn test_deployment_flow() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

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

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
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

        let price = helper.get_price(&contract_state);
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

    let contract_state = helper.get_contract_state().await?.unwrap();
    let (initialized, owner_prefix, owner_suffix) =
        helper.get_initialization_state(&contract_state);

    assert_eq!(initialized, 1, "Registry should be initialized");
    assert_eq!(owner_prefix, owner_id.prefix().as_felt().as_int());
    assert_eq!(owner_suffix, owner_id.suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn test_registry_ready_for_registration_after_deployment() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("DeployUser").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 200).await?;

    helper
        .register_name_for_account_with_payment(&user, "deploytest", Some(100))
        .await?;

    let is_registered = helper.is_name_registered("deploytest").await?;
    assert!(is_registered, "Name 'deploytest' should be registered");

    Ok(())
}

#[tokio::test]
async fn test_deployment_with_zero_price() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new_persistent().await?;
    helper.sync_network().await?;

    let owner = helper.create_account("OwnerZeroPrice").await?;
    let registry = helper.deploy_registry_contract().await?;

    helper.sync_network().await?;
    helper.initialize_registry(&owner).await?;
    helper.sync_network().await?;

    let contract_state = helper.client.get_account(registry.id()).await?.unwrap();
    let price = helper.get_price(&contract_state);

    assert_eq!(price, 0, "Price should be 0");

    Ok(())
}

// Note: Storage layout validation is covered by test_deployment_flow and test_deployment_verification_checks
// which verify initialization, owner, payment token, and price through the helper methods
