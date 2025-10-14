use core::slice;
use miden_client::{note::{NoteExecutionMode, NoteTag}, testing::NoteBuilder, transaction::{AccountInterface, OutputNote}, asset::{Asset, FungibleAsset}, testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1};
use miden_crypto::Felt;
use miden_testing::{Auth, MockChain, MockChainBuilder, TransactionContextBuilder};
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use miden_objects::note::{NoteType};
use midenid_contracts::{notes::create_pricing_initialize_note, utils::{create_account, create_naming_account, create_naming_library, create_pricing_account}};
use midenid_contracts::notes::{get_note_code, create_naming_initialize_note, create_pricing_calculate_cost_note};




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

    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone()
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
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

    // Create the calculate_domain_cost note
    let calculate_note = create_pricing_calculate_cost_note(
        tx_sender_account.clone(),
        domain_word,
        pricing_account.clone()
    ).await.unwrap();

    // Add both notes to the builder before building the chain
    builder.add_note(OutputNote::Full(initialize_input_note.clone()));
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