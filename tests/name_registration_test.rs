use miden_client::{ClientError, account::Account, transaction::TransactionRequestBuilder};
use miden_objects::{account::AccountId, asset::FungibleAsset, note::NoteType};

mod test_helper;
use test_helper::{
    RegistryTestHelper, get_or_init_shared_contract, mint_and_fund_account,
    setup_helper_with_contract,
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
            helper.sync_network().await?;
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
    helper.sync_network().await?;
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

    let user = helper.create_account("User1").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;

    helper
        .register_name_for_account_with_payment(&user, "alice", Some(100))
        .await?;

    assert!(helper.is_name_registered("alice").await?);
    assert!(helper.has_name_for_address(&user).await?);
    assert_eq!(
        helper.get_name_for_address(&user).await?,
        Some("alice".to_string())
    );

    let (prefix, suffix) = helper.get_account_for_name("alice").await?.unwrap();
    assert_eq!(prefix, user.id().prefix().as_felt().as_int());
    assert_eq!(suffix, user.id().suffix().as_int());

    Ok(())
}

#[tokio::test]
async fn cannot_register_same_name_twice() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = helper.create_account("User2").await?;
    let user2 = helper.create_account("User3").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];

    helper
        .register_name_for_account_with_payment(&user1, "bob", Some(100))
        .await?;

    let result = helper
        .register_name_for_account_with_payment(&user2, "bob", Some(100))
        .await;
    assert!(result.is_err());

    assert_eq!(
        helper.get_name_for_address(&user1).await?,
        Some("bob".to_string())
    );
    assert_eq!(helper.get_name_for_address(&user2).await?, None);

    Ok(())
}

#[tokio::test]
async fn account_can_only_register_one_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("User4").await?;
    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 200).await?;

    helper
        .register_name_for_account_with_payment(&user, "charlie", Some(100))
        .await?;

    let result = helper
        .register_name_for_account_with_payment(&user, "david", Some(100))
        .await;
    assert!(result.is_err());

    assert_eq!(
        helper.get_name_for_address(&user).await?,
        Some("charlie".to_string())
    );
    assert!(!helper.is_name_registered("david").await?);

    Ok(())
}

#[tokio::test]
async fn multiple_accounts_register_different_names() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = helper.create_account("User5").await?;
    let user2 = helper.create_account("User6").await?;
    let user3 = helper.create_account("User7").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2, user3], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];
    let user3 = &users[2];

    helper
        .register_name_for_account_with_payment(&user1, "eve", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&user2, "frank", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&user3, "grace", Some(100))
        .await?;

    assert_eq!(
        helper.get_name_for_address(&user1).await?,
        Some("eve".to_string())
    );
    assert_eq!(
        helper.get_name_for_address(&user2).await?,
        Some("frank".to_string())
    );
    assert_eq!(
        helper.get_name_for_address(&user3).await?,
        Some("grace".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn unregistered_names_return_none() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("User8").await?;

    assert!(!helper.is_name_registered("unknown").await?);
    assert_eq!(helper.get_account_for_name("unknown").await?, None);
    assert_eq!(helper.get_name_for_address(&user).await?, None);

    Ok(())
}

#[tokio::test]
async fn owner_can_register_name() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let owner = helper.create_account("Owner2").await?;
    let owner = mint_and_fund_account(&mut helper, faucet_id, &owner, 100).await?;
    helper
        .register_name_for_account_with_payment(&owner, "admin", Some(100))
        .await?;

    assert!(helper.is_name_registered("admin").await?);
    assert_eq!(
        helper.get_name_for_address(&owner).await?,
        Some("admin".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn bidirectional_mapping_consistency() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user1 = helper.create_account("User9").await?;
    let user2 = helper.create_account("User10").await?;

    let users = mint_and_fund_multiple(&mut helper, faucet_id, &[user1, user2], 100).await?;
    let user1 = &users[0];
    let user2 = &users[1];

    helper
        .register_name_for_account_with_payment(&user1, "henry", Some(100))
        .await?;
    helper
        .register_name_for_account_with_payment(&user2, "iris", Some(100))
        .await?;

    // Test get_id for registered names (forward lookup)
    let (user1_prefix, user1_suffix) = helper.get_account_for_name("henry").await?.unwrap();
    assert_eq!(user1_prefix, user1.id().prefix().as_felt().as_int());
    assert_eq!(user1_suffix, user1.id().suffix().as_int());

    let (user2_prefix, user2_suffix) = helper.get_account_for_name("iris").await?.unwrap();
    assert_eq!(user2_prefix, user2.id().prefix().as_felt().as_int());
    assert_eq!(user2_suffix, user2.id().suffix().as_int());

    // Test get_id for unregistered name returns None
    assert!(helper.get_account_for_name("nonexistent").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn registration_state_changes() -> Result<(), ClientError> {
    let (contract_id, faucet_id, _) = get_or_init_shared_contract().await;
    let mut helper = setup_helper_with_contract(contract_id, faucet_id).await?;

    let user = helper.create_account("User11").await?;

    assert!(!helper.has_name_for_address(&user).await?);
    assert_eq!(helper.get_name_for_address(&user).await?, None);

    let user = mint_and_fund_account(&mut helper, faucet_id, &user, 100).await?;
    helper
        .register_name_for_account_with_payment(&user, "testuser", Some(100))
        .await?;

    assert!(helper.has_name_for_address(&user).await?);
    assert_eq!(
        helper.get_name_for_address(&user).await?,
        Some("testuser".to_string())
    );

    Ok(())
}
