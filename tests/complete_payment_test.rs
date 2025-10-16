use miden_client::ClientError;

mod helpers;
use helpers::{
    RegistryTestHelper, create_basic_wallet, get_account_for_name, get_or_init_shared_contract,
    is_name_registered, mint_and_fund_account, register_name, setup_helper_with_contract,
    update_registry_price,
};

#[tokio::test]
async fn register_with_payment() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let alice = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Alice1").await?;
    let alice = mint_and_fund_account(&mut helper, faucet_id, &alice, 200).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &alice,
        "alice",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    assert!(is_name_registered(&mut helper.client, contract_id, "alice").await?);
    if let Some((prefix, suffix)) =
        get_account_for_name(&mut helper.client, contract_id, "alice").await?
    {
        assert_eq!(prefix, alice.id().prefix().as_felt().as_int());
        assert_eq!(suffix, alice.id().suffix().as_int());
    } else {
        panic!("Name lookup failed");
    }

    Ok(())
}

#[tokio::test]
async fn register_with_payment_wrong_amount() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let alice = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Alice2").await?;
    let alice = mint_and_fund_account(&mut helper, faucet_id, &alice, 200).await?;

    let result = register_name(
        &mut helper.client,
        contract_id,
        &alice,
        "alice2",
        helper.faucet_account.as_ref(),
        Some(50),
    )
    .await;

    assert!(
        result.is_err(),
        "Registration should fail with insufficient payment"
    );

    Ok(())
}

#[tokio::test]
async fn test_price_update_validation() -> Result<(), ClientError> {
    let (contract_id, faucet_id, owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User1").await?;
    let user2 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User2").await?;

    let user1 = mint_and_fund_account(&mut helper, faucet_id, &user1, 200).await?;
    let user2 = mint_and_fund_account(&mut helper, faucet_id, &user2, 300).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user1,
        "user1",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let owner = helper.client.get_account(owner_id).await?.unwrap().into();
    update_registry_price(&mut helper.client, contract_id, &owner, 200).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user2,
        "user2",
        helper.faucet_account.as_ref(),
        Some(200),
    )
    .await?;

    Ok(())
}
