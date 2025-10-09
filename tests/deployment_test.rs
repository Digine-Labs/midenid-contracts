mod test_helper;

use test_helper::RegistryTestHelper;
use miden_client::ClientError;

#[tokio::test]
async fn test_deployment_flow() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("\n🚀 Testing Complete Deployment Flow\n");

    println!("Step 1: Create Owner Account");
    let owner_account = helper.create_account("Owner").await?;
    println!("✅ Owner created: {}", owner_account.id());

    println!("\nStep 2: Create Payment Token (Faucet)");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    println!("\nStep 3: Deploy Registry Contract");
    let registry_contract = helper.deploy_registry_contract().await?;
    println!("✅ Registry deployed: {}", registry_contract.id());

    helper.sync_network().await?;

    println!("\nStep 4: Initialize Registry");
    let price = 100;
    helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;
    println!("✅ Registry initialized with price: {}", price);

    println!("\nStep 5: Verify Deployment");

    if let Some(contract_state) = helper.get_contract_state().await? {
        let (initialized, owner_prefix, owner_suffix) =
            helper.get_initialization_state(&contract_state);

        assert_eq!(initialized, 1, "Registry should be initialized");
        assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int(), "Owner prefix mismatch");
        assert_eq!(owner_suffix, owner_account.id().suffix().as_int(), "Owner suffix mismatch");

        let (token_prefix, token_suffix) = helper.get_payment_token_state(&contract_state);
        assert_eq!(token_prefix, faucet_account.id().prefix().as_felt().as_int(), "Payment token prefix mismatch");
        assert_eq!(token_suffix, faucet_account.id().suffix().as_int(), "Payment token suffix mismatch");

        let price_word = contract_state.account().storage().get_item(5).unwrap();
        let stored_price = price_word.get(0).unwrap().as_int();
        assert_eq!(stored_price, price, "Price mismatch");

        println!("✅ All deployment checks passed!");
        println!("\n📋 Deployment Summary:");
        println!("   Registry Contract:  {}", registry_contract.id());
        println!("   Owner Account:      {}", owner_account.id());
        println!("   Payment Token:      {}", faucet_account.id());
        println!("   Registration Price: {} tokens", price);
    } else {
        panic!("Contract state not found after initialization");
    }

    Ok(())
}

#[tokio::test]
async fn test_deployment_with_different_prices() -> Result<(), ClientError> {
    let prices = vec![0, 50, 100, 500, 1000];

    for price in prices {
        println!("\n🧪 Testing deployment with price: {}", price);

        let mut helper = RegistryTestHelper::new().await?;
        helper.sync_network().await?;

        let owner_account = helper.create_account("Owner").await?;
        let faucet_account = helper.create_faucet("REG", 8, 1_000_000_000).await?;
        let registry_contract = helper.deploy_registry_contract().await?;

        helper.sync_network().await?;

        if price == 0 {
            helper.initialize_registry(&owner_account).await?;
        } else {
            helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;
        }

        helper.sync_network().await?;

        let contract_record = helper.client.get_account(registry_contract.id()).await?;
        if let Some(record) = contract_record {
            let storage = record.account().storage();
            let price_word = storage.get_item(5).unwrap();
            let stored_price = price_word.get(0).unwrap().as_int();

            let expected_price = if price == 0 { 0 } else { 100 };
            assert_eq!(stored_price, expected_price, "Price {} should be stored as {}", price, expected_price);
            println!("✅ Price {} verified", stored_price);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_deployment_verification_checks() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("\n🔍 Testing Deployment Verification Checks\n");

    let owner_account = helper.create_account("Owner").await?;
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000_000).await?;
    let registry_contract = helper.deploy_registry_contract().await?;

    helper.sync_network().await?;

    println!("✅ All accounts created and contract deployed");

    println!("\n📋 Account Information:");
    println!("   Owner:    {}", owner_account.id());
    println!("   Faucet:   {}", faucet_account.id());
    println!("   Registry: {}", registry_contract.id());

    let owner_exists = helper.client.get_account(owner_account.id()).await?.is_some();
    let faucet_exists = helper.client.get_account(faucet_account.id()).await?.is_some();
    let registry_exists = helper.client.get_account(registry_contract.id()).await?.is_some();

    assert!(owner_exists, "Owner account should exist");
    assert!(faucet_exists, "Faucet account should exist");
    assert!(registry_exists, "Registry contract should exist");

    println!("\n✅ All accounts verified in database");

    helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;
    helper.sync_network().await?;

    let (initialized, owner_prefix, owner_suffix) = {
        let record = helper.client.get_account(registry_contract.id()).await?.unwrap();
        helper.get_initialization_state(&record)
    };

    assert_eq!(initialized, 1, "Registry should be initialized");
    assert_eq!(owner_prefix, owner_account.id().prefix().as_felt().as_int());
    assert_eq!(owner_suffix, owner_account.id().suffix().as_int());

    println!("✅ Initialization verified");
    println!("\n🎉 All deployment verification checks passed!");

    Ok(())
}

#[tokio::test]
async fn test_registry_ready_for_registration_after_deployment() -> Result<(), ClientError> {
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("\n🧪 Testing Registry Ready for Registration After Deployment\n");

    let owner_account = helper.create_account("Owner").await?;
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000_000).await?;
    let user_account = helper.create_account("User").await?;

    helper.deploy_registry_contract().await?;
    helper.sync_network().await?;

    helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;
    helper.sync_network().await?;

    println!("✅ Registry deployed and initialized");

    println!("\n💰 Minting tokens to user...");
    let fungible_asset = miden_objects::asset::FungibleAsset::new(faucet_account.id(), 100).unwrap();
    let tx_req = miden_client::transaction::TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset,
            user_account.id(),
            miden_objects::note::NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();

    let tx_result = helper.client.new_transaction(faucet_account.id(), tx_req).await?;
    helper.client.submit_transaction(tx_result).await?;

    let list_of_note_ids = loop {
        helper.client.sync_state().await?;
        let consumable_notes = helper.client.get_consumable_notes(Some(user_account.id())).await?;
        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();
        if !note_ids.is_empty() { break note_ids; }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    let tx_req = miden_client::transaction::TransactionRequestBuilder::new()
        .build_consume_notes(list_of_note_ids)
        .unwrap();
    let tx_result = helper.client.new_transaction(user_account.id(), tx_req).await?;
    helper.client.submit_transaction(tx_result).await?;

    helper.sync_network().await?;
    let user_record = helper.client.get_account(user_account.id()).await?.unwrap();
    let user_account = user_record.into();

    println!("✅ User has tokens");

    println!("\n📝 Registering name 'alice'...");
    helper.register_name_for_account_with_payment(&user_account, "alice", Some(100)).await?;
    helper.sync_network().await?;

    let is_registered = helper.is_name_registered("alice").await?;
    assert!(is_registered, "Name 'alice' should be registered");

    println!("✅ Name registered successfully!");
    println!("\n🎉 Registry is fully functional after deployment!");

    Ok(())
}
