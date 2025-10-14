mod test_helper;

use miden_client::ClientError;
use test_helper::{get_or_init_shared_contract, mint_and_fund_account, setup_helper_with_contract};

#[tokio::test]
async fn export_get_id() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("david").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    helper
        .register_name_for_account_with_payment(&user, "david", Some(100))
        .await?;

    let result = helper.call_get_id_export("david").await?;
    assert!(result.is_some());

    let (prefix, suffix) = result.unwrap();
    assert_eq!(prefix, user.id().prefix().as_felt().as_int());
    assert_eq!(suffix, user.id().suffix().as_int());

    let result = helper.call_get_id_export("nonexistent").await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn export_get_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("eve").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    helper
        .register_name_for_account_with_payment(&user, "eve", Some(100))
        .await?;

    let result = helper.call_get_name_export(&user).await?;
    assert_eq!(result, Some("eve".to_string()));

    let user2 = helper.create_account("frank").await?;
    let result = helper.call_get_name_export(&user2).await?;
    assert!(result.is_none());

    Ok(())
}
