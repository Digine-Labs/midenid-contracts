mod helpers;

use helpers::{
    call_get_id_export, call_get_name_export, create_basic_wallet, get_or_init_shared_contract,
    mint_and_fund_account, register_name, setup_helper_with_contract,
};
use miden_client::ClientError;

#[tokio::test]
async fn export_get_id() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "david").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "david",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let result = call_get_id_export(&mut helper.client, contract_id, "david").await?;
    assert!(result.is_some());

    let (prefix, suffix) = result.unwrap();
    assert_eq!(prefix, user.id().prefix().as_felt().as_int());
    assert_eq!(suffix, user.id().suffix().as_int());

    let result = call_get_id_export(&mut helper.client, contract_id, "nonexistent").await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn export_get_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "eve").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "eve",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let result = call_get_name_export(&mut helper.client, contract_id, &user).await?;
    assert_eq!(result, Some("eve".to_string()));

    let user2 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "frank").await?;
    let result = call_get_name_export(&mut helper.client, contract_id, &user2).await?;
    assert!(result.is_none());

    Ok(())
}
