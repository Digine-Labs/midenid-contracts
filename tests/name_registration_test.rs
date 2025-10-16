use miden_client::{ClientError, account::Account, transaction::TransactionRequestBuilder};
use miden_objects::{account::AccountId, asset::FungibleAsset, note::NoteType};

mod helpers;
use helpers::{
    RegistryTestHelper, create_basic_wallet, get_account_for_name, get_name_for_account,
    get_or_init_shared_contract, has_name_for_address, is_name_registered, mint_and_fund_account,
    register_name, setup_helper_with_contract,
};

async fn mint_and_fund_multiple(
    helper: &mut RegistryTestHelper,
    faucet_id: AccountId,
    accounts: &[Account],
    amount: u64,
) -> Result<Vec<Account>, ClientError> {
    // Submit all mint transactions first
    for account in accounts {
        let asset = FungibleAsset::new(faucet_id, amount)?;
        let mint_tx = TransactionRequestBuilder::new().build_mint_fungible_asset(
            asset,
            account.id(),
            NoteType::Public,
            helper.client.rng(),
        )?;
        let result = helper.client.new_transaction(faucet_id, mint_tx).await?;
        helper.client.submit_transaction(result).await?;
    }

    // Wait for all notes to arrive and consume them
    let mut funded_accounts = Vec::new();
    for account in accounts {
        let note_ids = loop {
            helper.client.sync_state().await?;
            let notes = helper
                .client
                .get_consumable_notes(Some(account.id()))
                .await?;
            let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
            if !ids.is_empty() {
                break ids;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        };

        let consume_tx = TransactionRequestBuilder::new().build_consume_notes(note_ids)?;
        let result = helper
            .client
            .new_transaction(account.id(), consume_tx)
            .await?;
        helper.client.submit_transaction(result).await?;
    }

    // Get updated accounts
    helper.client.sync_state().await?;
    for account in accounts {
        let record = helper.client.get_account(account.id()).await?.unwrap();
        funded_accounts.push(record.into());
    }

    Ok(funded_accounts)
}

#[tokio::test]
async fn register_name_creates_bidirectional_mapping() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User1").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "alice",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    assert!(is_name_registered(&mut helper.client, contract_id, "alice").await?);
    assert!(has_name_for_address(&mut helper.client, contract_id, &user).await?);
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user).await?,
        Some("alice".to_string())
    );

    let (prefix, suffix) = get_account_for_name(&mut helper.client, contract_id, "alice")
        .await?
        .unwrap();
    assert_eq!(prefix, user.id().prefix().as_felt().as_int());
    assert_eq!(suffix, user.id().suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn cannot_register_same_name_twice() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User2").await?;
    let user2 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User3").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];

    register_name(
        &mut helper.client,
        contract_id,
        &user1,
        "bob",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let result = register_name(
        &mut helper.client,
        contract_id,
        &user2,
        "bob",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await;
    assert!(result.is_err());

    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user1).await?,
        Some("bob".to_string())
    );
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user2).await?,
        None
    );

    Ok(())
}

#[tokio::test]
async fn account_can_only_register_one_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User4").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 200).await?;

    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "charlie",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    let result = register_name(
        &mut helper.client,
        contract_id,
        &user,
        "david",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await;
    assert!(result.is_err());

    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user).await?,
        Some("charlie".to_string())
    );
    assert!(!is_name_registered(&mut helper.client, contract_id, "david").await?);

    Ok(())
}

#[tokio::test]
async fn multiple_accounts_register_different_names() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User5").await?;
    let user2 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User6").await?;
    let user3 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User7").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2, user3], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];
    let user3 = &users[2];

    register_name(
        &mut helper.client,
        contract_id,
        &user1,
        "eve",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;
    register_name(
        &mut helper.client,
        contract_id,
        &user2,
        "frank",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;
    register_name(
        &mut helper.client,
        contract_id,
        &user3,
        "grace",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user1).await?,
        Some("eve".to_string())
    );
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user2).await?,
        Some("frank".to_string())
    );
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user3).await?,
        Some("grace".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn unregistered_names_return_none() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User8").await?;

    assert!(!is_name_registered(&mut helper.client, contract_id, "unknown").await?);
    assert_eq!(
        get_account_for_name(&mut helper.client, contract_id, "unknown").await?,
        None
    );
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user).await?,
        None
    );

    Ok(())
}

#[tokio::test]
async fn owner_can_register_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let owner = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Owner2").await?;
    let owner = mint_and_fund_account(&mut helper, faucet_id, &owner, 100).await?;
    register_name(
        &mut helper.client,
        contract_id,
        &owner,
        "admin",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    assert!(is_name_registered(&mut helper.client, contract_id, "admin").await?);
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &owner).await?,
        Some("admin".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn bidirectional_mapping_consistency() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User9").await?;
    let user2 = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User10").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];

    register_name(
        &mut helper.client,
        contract_id,
        &user1,
        "henry",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;
    register_name(
        &mut helper.client,
        contract_id,
        &user2,
        "iris",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    // Test get_id for registered names (forward lookup)
    let (user1_prefix, user1_suffix) =
        get_account_for_name(&mut helper.client, contract_id, "henry")
            .await?
            .unwrap();
    assert_eq!(user1_prefix, user1.id().prefix().as_felt().as_int());
    assert_eq!(user1_suffix, user1.id().suffix().as_int());

    let (user2_prefix, user2_suffix) =
        get_account_for_name(&mut helper.client, contract_id, "iris")
            .await?
            .unwrap();
    assert_eq!(user2_prefix, user2.id().prefix().as_felt().as_int());
    assert_eq!(user2_suffix, user2.id().suffix().as_int());

    // Test get_id for unregistered name returns None
    assert!(
        get_account_for_name(&mut helper.client, contract_id, "nonexistent")
            .await?
            .is_none()
    );

    Ok(())
}

#[tokio::test]
async fn registration_state_changes() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "User11").await?;

    assert!(!has_name_for_address(&mut helper.client, contract_id, &user).await?);
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user).await?,
        None
    );

    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;
    register_name(
        &mut helper.client,
        contract_id,
        &user,
        "testuser",
        helper.faucet_account.as_ref(),
        Some(100),
    )
    .await?;

    assert!(has_name_for_address(&mut helper.client, contract_id, &user).await?);
    assert_eq!(
        get_name_for_account(&mut helper.client, contract_id, &user).await?,
        Some("testuser".to_string())
    );

    Ok(())
}
