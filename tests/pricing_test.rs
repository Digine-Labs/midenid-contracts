use core::slice;
use miden_client::{note::{NoteExecutionMode, NoteTag}, testing::NoteBuilder, transaction::{AccountInterface, OutputNote}, asset::{Asset, FungibleAsset}, testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1};
use miden_crypto::{Felt, Word};
use miden_testing::{Auth, MockChain, MockChainBuilder, TransactionContextBuilder};
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use miden_objects::note::{NoteType};
use midenid_contracts::{notes::{create_price_set_note, create_pricing_initialize_note}, utils::{create_account, create_naming_account, create_naming_library, create_pricing_account, get_price_set_notes, get_test_prices}};
use midenid_contracts::notes::{get_note_code, create_naming_initialize_note, create_pricing_calculate_cost_note};


// Current todo: call set price before calculate domain costs

// Develop test like that
// https://github.com/0xMiden/miden-base/blob/719ff03d1482e6ce2ad4e986f59ec7b9a8ddf962/crates/miden-testing/src/kernel_tests/tx/test_fpi.rs#L515

#[tokio::test]
async fn test_pricing_init() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    let initialize_input_note = create_pricing_initialize_note(tx_sender_account.clone(), fungible_asset.faucet_id(), setter_account.clone(), pricing_account.clone()).await.unwrap();

    builder.add_note(OutputNote::Full(initialize_input_note.clone()));

    let mut mock_chain = builder.build()?;

    let tx_inputs = mock_chain.get_transaction_inputs(pricing_account.clone(), None, &[initialize_input_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    let owner_slot = updated_pricing_account.storage().get_item(1)?;
    let trasury_slot = updated_pricing_account.storage().get_item(2)?;

    assert_eq!(init_flag, 1);
    assert_eq!(setter_account.id().prefix().as_u64(), owner_slot.get(1).unwrap().as_int());
    assert_eq!(setter_account.id().suffix().as_int(), owner_slot.get(0).unwrap().as_int());
    assert_eq!(fungible_asset.faucet_id().prefix().as_u64(), trasury_slot.get(1).unwrap().as_int());
    assert_eq!(fungible_asset.faucet_id().suffix().as_int(), trasury_slot.get(0).unwrap().as_int());
    Ok(())
}

#[tokio::test]
async fn test_pricing_set_price() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    let set_price_note = create_price_set_note(
        setter_account.clone(),
        vec![Felt::new(123), Felt::new(1)],
        pricing_account.clone()
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    builder.add_note(OutputNote::Full(set_price_note.clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    // Now execute the calculate_domain_cost transaction
    let tx_inputs_calc = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_price_note.id()],
        &[]
    )?;

    let tx_context_calc = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs_calc)
        .build()?;

    // This should succeed if the domain validation passes
    let executed_tx = tx_context_calc.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let price_map = updated_pricing_account.storage().get_map_item(3, Word::new([Felt::new(1),Felt::new(0),Felt::new(0),Felt::new(0)]))?;

    assert_eq!(price_map.get(0).unwrap().as_int(), 123);

    Ok(())
}

#[tokio::test]
async fn test_pricing_set_price_all_letters() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(setter_account.clone(), pricing_account.clone(), test_prices).await;

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    // Now execute the calculate_domain_cost transaction
    let setter_tx_inputs = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_notes[0].id(), set_notes[1].id(), set_notes[2].id(), set_notes[3].id(), set_notes[4].id()],
        &[]
    )?;

    let setter_tx_context = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(setter_tx_inputs)
        .build()?;

    // This should succeed if the domain validation passes
    let executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let price_1_letter = updated_pricing_account.storage()
        .get_map_item(3, Word::new([Felt::new(1),Felt::new(0),Felt::new(0),Felt::new(0)]))?
        .get(0).unwrap().as_int();

    let price_2_letter = updated_pricing_account.storage()
        .get_map_item(3, Word::new([Felt::new(2),Felt::new(0),Felt::new(0),Felt::new(0)]))?
        .get(0).unwrap().as_int();

    let price_3_letter = updated_pricing_account.storage()
        .get_map_item(3, Word::new([Felt::new(3),Felt::new(0),Felt::new(0),Felt::new(0)]))?
        .get(0).unwrap().as_int();

    let price_4_letter = updated_pricing_account.storage()
        .get_map_item(3, Word::new([Felt::new(4),Felt::new(0),Felt::new(0),Felt::new(0)]))?
        .get(0).unwrap().as_int();

    let price_5_letter = updated_pricing_account.storage()
        .get_map_item(3, Word::new([Felt::new(5),Felt::new(0),Felt::new(0),Felt::new(0)]))?
        .get(0).unwrap().as_int();

    assert_eq!(price_1_letter, 123123);
    assert_eq!(price_2_letter, 45645);
    assert_eq!(price_3_letter, 789);
    assert_eq!(price_4_letter, 555);
    assert_eq!(price_5_letter, 123);

    Ok(())
}

#[tokio::test]
async fn test_pricing_calculate_domain_cost() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // Sending reversed word so stack will init however we want
    let domain_word = [
        Felt::new(0),                          // 3rd part
        Felt::new(0),                          // 2nd felt
        Felt::new(0x656369_6C_61_62_63),             // 1st felt
        Felt::new(7),            // length
    ].into();

    // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    // price setter notes

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(setter_account.clone(), pricing_account.clone(), test_prices).await;

    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone(),
        123
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));
    builder.add_note(OutputNote::Full(calculate_note.clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    // Execute setter notes

    let setter_tx_inputs = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_notes[0].id(), set_notes[1].id(), set_notes[2].id(), set_notes[3].id(), set_notes[4].id()],
        &[]
    )?;

    let setter_tx_context = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(setter_tx_inputs)
        .build()?;

    let _executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_executed_tx)?;

    // Now execute the calculate_domain_cost transaction
    let tx_inputs_calc = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[calculate_note.id()],
        &[]
    )?;


    let tx_context_calc = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs_calc)
        .build()?;

    // This should succeed if the domain validation passes
    let result = tx_context_calc.execute().await;

    // Assert that the transaction executed successfully
    assert!(result.is_ok(), "calculate_domain_cost should validate the domain successfully: {:?}", result.err());

    Ok(())
}

#[tokio::test]
async fn test_pricing_calculate_domain_cost_multiple_words() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // Sending reversed word so stack will init however we want
    let domain_word = [
        Felt::new(0x666766676869),                          // 3rd part
        Felt::new(0x65636667686361),                          // 2nd felt
        Felt::new(0x656369_6C_61_62_63),             // 1st felt
        Felt::new(20),            // length
    ].into();

    // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(setter_account.clone(), pricing_account.clone(), test_prices).await;


    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone(),
        123
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));
    builder.add_note(OutputNote::Full(calculate_note.clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    let setter_tx_inputs = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_notes[0].id(), set_notes[1].id(), set_notes[2].id(), set_notes[3].id(), set_notes[4].id()],
        &[]
    )?;

    let setter_tx_context = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(setter_tx_inputs)
        .build()?;

    let _executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_executed_tx)?;

    // Now execute the calculate_domain_cost transaction
    let tx_inputs_calc = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[calculate_note.id()],
        &[]
    )?;

    let tx_context_calc = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs_calc)
        .build()?;

    // This should succeed if the domain validation passes
    let result = tx_context_calc.execute().await;

    // Assert that the transaction executed successfully
    assert!(result.is_ok(), "calculate_domain_cost should validate the domain successfully: {:?}", result.err());

    Ok(())
}

#[tokio::test]
async fn test_pricing_calculate_domain_cost_one_letter() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // Sending reversed word so stack will init however we want
    let domain_word = [
        Felt::new(0),                          // 3rd part
        Felt::new(0),                          // 2nd felt
        Felt::new(0x65),             // 1st felt
        Felt::new(1),            // length
    ].into();

        // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(setter_account.clone(), pricing_account.clone(), test_prices).await;


    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone(),
        123123
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));
    builder.add_note(OutputNote::Full(calculate_note.clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    let setter_tx_inputs = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_notes[0].id(), set_notes[1].id(), set_notes[2].id(), set_notes[3].id(), set_notes[4].id()],
        &[]
    )?;

    let setter_tx_context = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(setter_tx_inputs)
        .build()?;

    let _executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_executed_tx)?;

    // Now execute the calculate_domain_cost transaction
    let tx_inputs_calc = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[calculate_note.id()],
        &[]
    )?;

    let tx_context_calc = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs_calc)
        .build()?;

    // This should succeed if the domain validation passes
    let result = tx_context_calc.execute().await;

    // Assert that the transaction executed successfully
    assert!(result.is_ok(), "calculate_domain_cost should validate the domain successfully: {:?}", result.err());

    Ok(())
}

#[tokio::test]
async fn test_pricing_calculate_empty_domain_cost() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    // Sending reversed word so stack will init however we want
    let domain_word = [
        Felt::new(0),                          // 3rd part
        Felt::new(0),                          // 2nd felt
        Felt::new(0),             // 1st felt
        Felt::new(0),            // length
    ].into();

        // First, initialize the pricing contract
    let initialize_input_note = create_pricing_initialize_note(
        tx_sender_account.clone(),
        fungible_asset.faucet_id(),
        setter_account.clone(),
        pricing_account.clone()
    ).await.unwrap();

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(setter_account.clone(), pricing_account.clone(), test_prices).await;


    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone(),
        123123
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));
    builder.add_note(OutputNote::Full(calculate_note.clone()));

    let mut mock_chain = builder.build()?;

    // Execute initialization transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        pricing_account.clone(),
        None,
        &[initialize_input_note.id()],
        &[]
    )?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Verify initialization succeeded
    let init_flag = updated_pricing_account.storage().get_item(0)?.get(0).unwrap().as_int();
    assert_eq!(init_flag, 1, "Contract should be initialized");

    let setter_tx_inputs = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[set_notes[0].id(), set_notes[1].id(), set_notes[2].id(), set_notes[3].id(), set_notes[4].id()],
        &[]
    )?;

    let setter_tx_context = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(setter_tx_inputs)
        .build()?;

    let _executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_executed_tx)?;

    // Now execute the calculate_domain_cost transaction
    let tx_inputs_calc = mock_chain.get_transaction_inputs(
        updated_pricing_account.clone(),
        None,
        &[calculate_note.id()],
        &[]
    )?;

    let tx_context_calc = TransactionContextBuilder::new(updated_pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs_calc)
        .build()?;

    // This should succeed if the domain validation passes
    let result = tx_context_calc.execute().await;

    // Assert that the transaction executed successfully
    assert!(result.is_err(), "calculate_domain_cost should revert on empty domain");

    Ok(())
}