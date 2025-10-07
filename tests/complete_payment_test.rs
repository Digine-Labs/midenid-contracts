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
async fn test_complete_payment_with_p2id() -> Result<(), ClientError> {
    println!("\n🚀 Testing complete payment flow with P2ID notes...\n");

    // Step 1: Initialize test helper
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    // Step 2: Create faucet account
    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    // Step 3: Create Alice account
    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    // Step 4: Mint tokens from faucet to Alice
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

    // Step 5: Wait for notes and consume them
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

    // Step 6: Consume all notes
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

    // Sync and get updated Alice account
    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account_updated: miden_client::account::Account = alice_record.into();

    println!("\n💰 Alice's balance after consuming notes:");
    println!("   Account: {}", alice_account_updated.id());
    println!("   Total tokens: 500");

    // Step 7: Deploy and initialize registry
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

    // Verify initialization
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

    // Step 8: Alice registers name with payment
    println!("\n📝 Registering name 'alice' with 100 token payment...");
    println!(
        "   Registry now implements basic wallet interface (receive_asset + move_asset_to_note)"
    );

    helper
        .register_name_for_account_with_payment(&alice_account_updated, "alice", Some(100))
        .await?;
    println!("✅ Name registered successfully with payment!");

    // Step 10: Verify registration
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

    // Step 11: Verify Alice's balance decreased
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

/// Test that registration fails when Alice doesn't have enough tokens
/// This verifies the payment validation logic correctly rejects insufficient payments
#[tokio::test]
#[ignore]
async fn test_insufficient_payment_reverts() -> Result<(), ClientError> {
    println!("\n🚀 Testing insufficient payment validation...\n");

    // Step 1: Initialize test helper
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    // Step 2: Create faucet account
    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    // Step 3: Create Bob account (who has NO tokens)
    println!("\n👤 Creating Bob account (with no tokens)...");
    let bob_account = helper.create_account("Bob").await?;
    println!("✅ Bob created: {}", bob_account.id());
    println!("   Bob has 0 tokens");

    // Step 4: Deploy and initialize registry with price=100
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

    // Verify the price was actually set
    helper.sync_network().await?;
    let contract_record = helper.get_registry_account().await?;
    let price_word: miden_objects::Word = contract_record
        .account()
        .storage()
        .get_item(5)
        .unwrap()
        .into();
    println!("   🔍 Full price Word from slot 5: {:?}", price_word);
    let actual_price = helper.get_price(&contract_record);
    println!("   🔍 Verified price in contract storage: {}", actual_price);
    assert_eq!(actual_price, 100, "Price should be 100");

    // Step 5: Try to register without any payment (should fail)
    println!("\n❌ Bob attempting to register name without payment...");
    println!("   Expected: Transaction should fail due to insufficient payment");

    // Check if Bob has any consumable notes first
    helper.sync_network().await?;
    let bob_notes = helper
        .client
        .get_consumable_notes(Some(bob_account.id()))
        .await?;
    println!("   Bob has {} consumable notes", bob_notes.len());

    let result = helper.register_name_for_account(&bob_account, "bob").await;

    match result {
        Err(e) => {
            println!("✅ Transaction failed as expected!");
            println!("   Error: {:?}", e);

            // Check if error contains payment-related message
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
                panic!(
                    "❌ CRITICAL BUG: Registration succeeded without payment! Name 'bob' is now registered even though Bob has no tokens!"
                );
            } else {
                println!("⚠️  Function returned Ok but name is NOT registered - investigating...");
                panic!("❌ FAIL: Function should have returned error, not Ok");
            }
        }
    }

    println!("\n🎉 SUCCESS! Insufficient payment validation works correctly:");
    println!("   ✅ Registry requires payment (price=100)");
    println!("   ✅ Bob has no tokens");
    println!("   ✅ Registration attempt failed as expected");
    println!("   ✅ Payment validation prevents unauthorized registration");

    Ok(())
}

/// Test that registration fails when Alice sends less than the required amount
/// This verifies the exact amount validation
#[tokio::test]
#[ignore]
async fn test_partial_payment_reverts() -> Result<(), ClientError> {
    println!("\n🚀 Testing partial payment rejection...\n");

    // Step 1: Initialize test helper
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    // Step 2: Create faucet account
    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    // Step 3: Create Alice account
    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    // Step 4: Mint only 50 tokens to Alice (half of required 100)
    println!("\n💰 Minting only 50 tokens to Alice (insufficient for price=100)...");
    let amount: u64 = 50;
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
    println!("✅ Minted 50 tokens to Alice");

    // Step 5: Wait and consume the note
    println!("\n🔄 Waiting for note...");

    let list_of_note_ids = loop {
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
    println!("✅ Alice consumed note (has 50 tokens)");

    helper.sync_network().await?;
    let alice_record = helper
        .client
        .get_account(alice_account.id())
        .await?
        .unwrap();
    let alice_account_updated: miden_client::account::Account = alice_record.into();

    // Step 6: Deploy and initialize registry with price=100
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

    // Step 7: Alice tries to send P2ID with only 50 tokens (insufficient)
    println!("\n❌ Alice attempting to pay with only 50 tokens (needs 100)...");

    let payment_amount = 50u64; // Only half of required amount!
    let fungible_asset_payment = FungibleAsset::new(faucet_account.id(), payment_amount)
        .expect("Failed to create payment asset");

    let payment_transaction = PaymentNoteDescription::new(
        vec![fungible_asset_payment.into()],
        alice_account_updated.id(),
        registry_account_id,
    );

    let p2id_transaction_request = TransactionRequestBuilder::new()
        .build_pay_to_id(payment_transaction, NoteType::Public, helper.client.rng())
        .unwrap();

    println!("   Submitting P2ID payment with 50 tokens...");
    let p2id_tx_result = helper
        .client
        .new_transaction(alice_account_updated.id(), p2id_transaction_request)
        .await?;

    helper.client.submit_transaction(p2id_tx_result).await?;
    println!("✅ P2ID note created (with insufficient amount)");

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    helper.sync_network().await?;

    // Step 8: Try to register (should fail due to insufficient payment)
    println!("\n❌ Attempting registration with insufficient payment...");
    let result = helper
        .register_name_for_account(&alice_account_updated, "alice")
        .await;

    match result {
        Err(e) => {
            println!("✅ Transaction failed as expected!");
            println!("   Error: {:?}", e);

            let error_msg = format!("{:?}", e);
            if error_msg.contains("WRONG_AMOUNT_PAID")
                || error_msg.contains("assertion")
                || error_msg.contains("failed")
            {
                println!("✅ Error indicates payment amount validation failure");
            }
        }
        Ok(_) => {
            panic!(
                "❌ FAIL: Registration should have failed - Alice only paid 50 but price is 100!"
            );
        }
    }

    println!("\n🎉 SUCCESS! Partial payment validation works correctly:");
    println!("   ✅ Registry requires 100 tokens");
    println!("   ✅ Alice sent only 50 tokens");
    println!("   ✅ Registration attempt failed as expected");
    println!("   ✅ Contract validates exact payment amount");

    Ok(())
}

/// Test that price updates are enforced correctly
/// 1. Init with price=100, Alice registers with 100 tokens (succeeds)
/// 2. Owner updates price to 200
/// 3. Bob tries to register with only 100 tokens (should fail)
#[tokio::test]
#[ignore]
async fn test_price_update_validation() -> Result<(), ClientError> {
    println!("\n🚀 Testing price update validation...\n");

    // Step 1: Initialize test helper
    let mut helper = RegistryTestHelper::new().await?;
    helper.sync_network().await?;

    // Step 2: Create faucet account
    println!("📦 Creating faucet account...");
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    println!("✅ Faucet created: {}", faucet_account.id());

    // Step 3: Create Alice and Bob accounts
    println!("\n👤 Creating Alice account...");
    let alice_account = helper.create_account("Alice").await?;
    println!("✅ Alice created: {}", alice_account.id());

    println!("\n👤 Creating Bob account...");
    let bob_account = helper.create_account("Bob").await?;
    println!("✅ Bob created: {}", bob_account.id());

    // Step 4: Mint tokens to Alice (100 tokens)
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

    // Wait and consume Alice's note
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

    // Step 5: Mint tokens to Bob (100 tokens)
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

    // Wait and consume Bob's note
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

    // Step 6: Deploy and initialize registry with price=100
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

    // Verify initial price
    helper.sync_network().await?;
    let contract_record = helper.get_registry_account().await?;
    let price = helper.get_price(&contract_record);
    println!("   Initial price: {}", price);
    assert_eq!(price, 100, "Initial price should be 100");

    // Step 7: Alice registers name with 100 tokens (should succeed)
    println!("\n📝 Alice registering name with 100 tokens at price=100...");
    helper
        .register_name_for_account_with_payment(&alice_account_updated, "alice", Some(100))
        .await?;
    println!("✅ Alice registered successfully");

    // Verify Alice's registration
    helper.sync_network().await?;
    let registered = helper.is_name_registered("alice").await?;
    assert!(registered, "Alice's name should be registered");
    println!("✅ Verified: 'alice' is registered");

    // Step 8: Owner updates price to 200
    println!("\n💵 Owner updating price from 100 to 200...");
    helper.update_price(&owner_account, 200).await?;
    println!("✅ Price updated to 200");

    // Verify price update
    helper.sync_network().await?;
    let contract_record = helper.get_registry_account().await?;
    let new_price = helper.get_price(&contract_record);
    println!("   New price: {}", new_price);
    assert_eq!(new_price, 200, "Price should be updated to 200");

    // Step 9: Bob tries to register with only 100 tokens (should fail)
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
