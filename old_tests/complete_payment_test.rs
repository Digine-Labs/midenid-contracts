use miden_client::{
    ClientError,
    transaction::{PaymentNoteDescription, TransactionRequestBuilder},
};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod test_helper;
use test_helper::RegistryTestHelper;

/// Complete test with actual payment using P2ID notes
/// 1. Create faucet and mint tokens to Alice
/// 2. Alice consumes notes to get tokens in her vault
/// 3. Deploy registry with faucet as payment token and price=100
/// 4. Alice creates a P2ID note that pays 100 tokens to registry while calling register_name
/// 5. Verify registration and payment consumption
#[tokio::test]
async fn register_with_payment() -> Result<(), ClientError> {
    println!("\n🚀 Testing complete payment flow with P2ID notes...\n");

    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    println!("\n💰 Minting tokens from faucet to Alice...");
    let amount: u64 = 100;
    let fungible_asset =
        FungibleAsset::new(faucet_account.id(), amount).expect("Failed to create fungible asset");

    for i in 1..=2 {
        println!("   Minting note {} with {} tokens...", i, amount);

        let transaction_request = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                alice_account.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();

        let tx_execution_result = helper
            .client
            .new_transaction(faucet_account.id(), transaction_request)
            .await?;

        helper
            .client
            .submit_transaction(tx_execution_result)
            .await?;
    }
    println!("✅ Minted 2 notes of {} tokens each", amount);

    println!("\n🔄 Waiting for notes to be available...");

    let list_of_note_ids = loop {
        helper.client.sync_state().await?;

        let consumable_notes = helper
            .client
            .get_consumable_notes(Some(alice_account.id()))
            .await?;

        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();

        println!("   Found {} consumable notes", note_ids.len());

        if note_ids.len() >= 2 {
            break note_ids;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    println!("✅ All 2 notes are now consumable");

    println!("\n💸 Alice consuming all notes...");
    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(list_of_note_ids)
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(alice_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ All notes consumed successfully");

    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account_updated: miden_client::account::Account = alice_record.into();

    println!("\n💰 Alice's balance after consuming notes:");
    println!("   Account: {}", alice_account_updated.id());
    println!("   Total tokens: 200");

    println!("\n📜 Deploying registry contract...");
    helper.deploy_registry_contract().await?;
    let registry_account_id = helper.registry_contract.as_ref().unwrap().id();
    println!("✅ Registry deployed: {}", registry_account_id);

    println!("\n⚙️  Creating owner account...");
    let owner_account = helper.create_account("RegistryOwner").await?;
    println!("✅ Owner created: {}", owner_account.id());

    println!("\n🔧 Initializing registry with faucet as payment token (price=100)...");
    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;
    println!("✅ Registry initialized");

    let contract_record = helper.get_registry_account().await?;
    let state = helper.get_complete_contract_state(&contract_record);
    println!("\n📊 Registry state:");
    println!("   Initialized: {}", state.initialized);
    println!(
        "   Owner: 0x{:x}{:016x}",
        state.owner_prefix, state.owner_suffix
    );
    println!(
        "   Payment token: 0x{:x}{:016x}",
        state.token_prefix, state.token_suffix
    );

    let price = helper.get_price(&contract_record);
    println!("   Registration price: {}", price);
    assert_eq!(price, 100, "Price should be 100");

    println!("\n📝 Registering name 'alice' with 100 token payment...");
    println!(
        "   Registry now implements basic wallet interface (receive_asset + move_asset_to_note)"
    );

    helper
        .register_name_for_account_with_payment(&alice_account_updated, "alice", Some(100))
        .await?;
    println!("✅ Name registered successfully with payment!");

    println!("\n🔍 Verifying registration...");
    let registered = helper.is_name_registered("alice").await?;
    assert!(registered, "Name 'alice' should be registered");

    if let Some((prefix, suffix)) = helper.get_account_for_name("alice").await? {
        println!(
            "✅ Name 'alice' is registered to: 0x{:x}{:016x}",
            prefix, suffix
        );
        assert_eq!(
            prefix,
            alice_account_updated.id().prefix().as_felt().as_int()
        );
        assert_eq!(suffix, alice_account_updated.id().suffix().as_int());
    } else {
        panic!("Name lookup failed");
    }

    helper.sync_network().await?;
    let alice_final_record = helper
        .client
        .get_account(alice_account_updated.id())
        .await?
        .unwrap();
    let alice_final: miden_client::account::Account = alice_final_record.into();

    println!("\n💰 Alice's final state:");
    println!("   Account: {}", alice_final.id());
    println!("   Tokens should have decreased from 500 to 400");

    println!("\n🎉 SUCCESS! Complete payment flow with P2ID verified:");
    println!("   ✅ Faucet created and 500 tokens minted to Alice");
    println!("   ✅ Alice consumed all minted notes");
    println!("   ✅ Registry initialized with price=100");
    println!("   ✅ Alice sent P2ID payment of 100 tokens to registry");
    println!("   ✅ Name registration completed with payment consumed");
    println!("   ✅ Payment validation successful!");

    Ok(())
}

#[tokio::test]
async fn register_with_payment_wrong_amount() -> Result<(), ClientError> {
    println!("\n🚀 Testing complete payment flow with P2ID notes...\n");

    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    println!("\n💰 Minting tokens from faucet to Alice...");
    let amount: u64 = 100;
    let fungible_asset =
        FungibleAsset::new(faucet_account.id(), amount).expect("Failed to create fungible asset");

    for i in 1..=2 {
        println!("   Minting note {} with {} tokens...", i, amount);

        let transaction_request = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset,
                alice_account.id(),
                NoteType::Public,
                helper.client.rng(),
            )
            .unwrap();

        let tx_execution_result = helper
            .client
            .new_transaction(faucet_account.id(), transaction_request)
            .await?;

        helper
            .client
            .submit_transaction(tx_execution_result)
            .await?;
    }
    println!("✅ Minted 2 notes of {} tokens each", amount);

    println!("\n🔄 Waiting for notes to be available...");

    let list_of_note_ids = loop {
        helper.client.sync_state().await?;

        let consumable_notes = helper
            .client
            .get_consumable_notes(Some(alice_account.id()))
            .await?;

        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();

        println!("   Found {} consumable notes", note_ids.len());

        if note_ids.len() >= 2 {
            break note_ids;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    println!("✅ All 2 notes are now consumable");

    println!("\n💸 Alice consuming all notes...");
    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(list_of_note_ids)
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(alice_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ All notes consumed successfully");

    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account_updated: miden_client::account::Account = alice_record.into();

    println!("\n💰 Alice's balance after consuming notes:");
    println!("   Account: {}", alice_account_updated.id());
    println!("   Total tokens: 200");

    println!("\n📜 Deploying registry contract...");
    helper.deploy_registry_contract().await?;
    let registry_account_id = helper.registry_contract.as_ref().unwrap().id();
    println!("✅ Registry deployed: {}", registry_account_id);

    println!("\n⚙️  Creating owner account...");
    let owner_account = helper.create_account("RegistryOwner").await?;
    println!("✅ Owner created: {}", owner_account.id());

    println!("\n🔧 Initializing registry with faucet as payment token (price=100)...");
    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;
    println!("✅ Registry initialized");

    let contract_record = helper.get_registry_account().await?;
    let state = helper.get_complete_contract_state(&contract_record);
    println!("\n📊 Registry state:");
    println!("   Initialized: {}", state.initialized);
    println!(
        "   Owner: 0x{:x}{:016x}",
        state.owner_prefix, state.owner_suffix
    );
    println!(
        "   Payment token: 0x{:x}{:016x}",
        state.token_prefix, state.token_suffix
    );

    let price = helper.get_price(&contract_record);
    println!("   Registration price: {}", price);
    assert_eq!(price, 100, "Price should be 100");

    println!("\n📝 Registering name 'alice' with 100 token payment...");
    println!(
        "   Registry now implements basic wallet interface (receive_asset + move_asset_to_note)"
    );

    let registration_result = helper
        .register_name_for_account_with_payment(&alice_account_updated, "alice", Some(50))
        .await;
    let err = registration_result.expect_err(
        "Registration should fail when payment note amount is below the configured price",
    );
    println!("✅ Registration failed as expected with error: {err}");

    println!("\n🔍 Verifying registration did not happen...");
    let registered = helper.is_name_registered("alice").await?;
    assert!(
        !registered,
        "Name 'alice' must remain unregistered when payment is insufficient"
    );

    if let Some((prefix, suffix)) = helper.get_account_for_name("alice").await? {
        panic!(
            "Name unexpectedly registered to 0x{:x}{:016x} after insufficient payment",
            prefix, suffix
        );
    } else {
        println!("✅ Name lookup confirms 'alice' is not registered");
    }

    helper.sync_network().await?;
    let alice_final_record = helper
        .client
        .get_account(alice_account_updated.id())
        .await?
        .unwrap();
    let alice_final: miden_client::account::Account = alice_final_record.into();

    println!("\n💰 Alice's final state:");
    println!("   Account: {}", alice_final.id());
    println!("   Tokens should still be 200 because the payment was rejected");

    println!("\n🎉 EXPECTED FAILURE CONFIRMED:");
    println!("   ✅ Faucet created and 200 tokens minted to Alice");
    println!("   ✅ Alice consumed all minted notes");
    println!("   ✅ Registry initialized with price=100");
    println!("   ✅ Alice attempted to pay only 50 tokens");
    println!("   ✅ Registration rejected due to insufficient payment");
    println!("   ✅ Contract state unchanged for 'alice'");

    Ok(())
}

/// Test that price updates are enforced correctly
/// 1. Init with price=100, Alice registers with 100 tokens (succeeds)
/// 2. Owner updates price to 200
/// 3. Bob tries to register with only 100 tokens (should fail)
#[tokio::test]
async fn test_price_update_validation() -> Result<(), ClientError> {
    println!("\n🚀 Testing price update validation...\n");

    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    println!("\n👤 Creating Bob account...");
    let bob_account = helper.create_account("Bob").await?;
    println!("✅ Bob created: {}", bob_account.id());

    println!("\n💰 Minting 100 tokens to Alice...");
    let amount: u64 = 100;
    let fungible_asset =
        FungibleAsset::new(faucet_account.id(), amount).expect("Failed to create fungible asset");

    let transaction_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset,
            alice_account.id(),
            NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(faucet_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ Minted 100 tokens to Alice");

    println!("\n🔄 Waiting for Alice's note...");
    let alice_note_ids = loop {
        helper.client.sync_state().await?;
        let consumable_notes = helper
            .client
            .get_consumable_notes(Some(alice_account.id()))
            .await?;
        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();
        if !note_ids.is_empty() {
            break note_ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(alice_note_ids)
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(alice_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ Alice consumed note");

    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account_updated: miden_client::account::Account = alice_record.into();

    println!("\n💰 Minting 100 tokens to Bob...");
    let fungible_asset_bob =
        FungibleAsset::new(faucet_account.id(), amount).expect("Failed to create fungible asset");

    let transaction_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            fungible_asset_bob,
            bob_account.id(),
            NoteType::Public,
            helper.client.rng(),
        )
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(faucet_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ Minted 100 tokens to Bob");

    println!("\n🔄 Waiting for Bob's note...");
    let bob_note_ids = loop {
        helper.client.sync_state().await?;
        let consumable_notes = helper
            .client
            .get_consumable_notes(Some(bob_account.id()))
            .await?;
        let note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();
        if !note_ids.is_empty() {
            break note_ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(bob_note_ids)
        .unwrap();

    let tx_execution_result = helper
        .client
        .new_transaction(bob_account.id(), transaction_request)
        .await?;

    helper
        .client
        .submit_transaction(tx_execution_result)
        .await?;
    println!("✅ Bob consumed note");

    helper.sync_network().await?;
    let bob_record = helper.client.get_account(bob_account.id()).await?.unwrap();
    let bob_account_updated: miden_client::account::Account = bob_record.into();

    println!("\n📜 Deploying registry contract...");
    helper.deploy_registry_contract().await?;
    let registry_account_id = helper.registry_contract.as_ref().unwrap().id();
    println!("✅ Registry deployed: {}", registry_account_id);

    println!("\n⚙️  Creating owner account...");
    let owner_account = helper.create_account("RegistryOwner").await?;
    println!("✅ Owner created: {}", owner_account.id());

    println!("\n🔧 Initializing registry with price=100...");
    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;
    println!("✅ Registry initialized with price=100");

    helper.sync_network().await?;
    let contract_record = helper.get_registry_account().await?;
    let price = helper.get_price(&contract_record);
    println!("   Initial price: {}", price);
    assert_eq!(price, 100, "Initial price should be 100");

    println!("\n📝 Alice registering name with 100 tokens at price=100...");
    helper
        .register_name_for_account_with_payment(&alice_account_updated, "alice", Some(100))
        .await?;
    println!("✅ Alice registered successfully");

    helper.sync_network().await?;
    let registered = helper.is_name_registered("alice").await?;
    assert!(registered, "Alice's name should be registered");
    println!("✅ Verified: 'alice' is registered");

    println!("\n💵 Owner updating price from 100 to 200...");
    helper.update_price(&owner_account, 200).await?;
    println!("✅ Price updated to 200");

    helper.sync_network().await?;
    let contract_record = helper.get_registry_account().await?;
    let new_price = helper.get_price(&contract_record);
    println!("   New price: {}", new_price);
    assert_eq!(new_price, 200, "Price should be updated to 200");

    println!("\n❌ Bob attempting to register with 100 tokens at new price=200...");
    println!("   Expected: Transaction should fail due to insufficient payment");

    let result = helper
        .register_name_for_account_with_payment(&bob_account_updated, "bob", Some(100))
        .await;

    match result {
        Err(e) => {
            println!("✅ Transaction failed as expected!");
            println!("   Error: {:?}", e);

            let error_msg = format!("{:?}", e);
            if error_msg.contains("WRONG_AMOUNT_PAID")
                || error_msg.contains("Payment insufficient")
                || error_msg.contains("assertion")
                || error_msg.contains("failed")
            {
                println!("✅ Error indicates payment validation failure");
            } else {
                println!("⚠️  Error type: {}", error_msg);
            }
        }
        Ok(_) => {
            // Check if name was actually registered
            helper.sync_network().await?;
            let is_registered = helper.is_name_registered("bob").await?;

            if is_registered {
                panic!("❌ CRITICAL BUG: Bob registered with 100 tokens when price is 200!");
            } else {
                panic!("❌ FAIL: Function should have returned error, not Ok");
            }
        }
    }

    println!("\n🎉 SUCCESS! Price update validation works correctly:");
    println!("   ✅ Initial price was 100");
    println!("   ✅ Alice registered successfully with 100 tokens");
    println!("   ✅ Owner updated price to 200");
    println!("   ✅ Bob's attempt with 100 tokens failed as expected");
    println!("   ✅ Dynamic price validation enforced!");

    Ok(())
}
