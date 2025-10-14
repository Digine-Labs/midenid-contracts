use miden_client::ClientError;

mod test_helper;
use test_helper::{
    RegistryTestHelper, get_or_init_shared_contract, mint_and_fund_account,
    setup_helper_with_contract,
};

#[tokio::test]
async fn register_with_payment() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _owner_id) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let alice = helper.create_account("Alice1").await?;
    let alice = mint_and_fund_account(&mut helper, faucet_id, &alice, 200).await?;

    helper
        .register_name_for_account_with_payment(&alice, "alice", Some(100))
        .await?;

    assert!(helper.is_name_registered("alice").await?);
    if let Some((prefix, suffix)) = helper.get_account_for_name("alice").await? {
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

    let alice = helper.create_account("Alice2").await?;
    let alice = mint_and_fund_account(&mut helper, faucet_id, &alice, 200).await?;

    let result = helper
        .register_name_for_account_with_payment(&alice, "alice2", Some(50))
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

    let user1 = helper.create_account("User1").await?;
    let user2 = helper.create_account("User2").await?;

    let user1 = mint_and_fund_account(&mut helper, faucet_id, &user1, 200).await?;
    let user2 = mint_and_fund_account(&mut helper, faucet_id, &user2, 300).await?;

    helper
        .register_name_for_account_with_payment(&user1, "user1", Some(100))
        .await?;

    let owner = helper.client.get_account(owner_id).await?.unwrap().into();
    helper.update_price(&owner, 200).await?;

    helper
        .register_name_for_account_with_payment(&user2, "user2", Some(200))
        .await?;

    Ok(())
}
