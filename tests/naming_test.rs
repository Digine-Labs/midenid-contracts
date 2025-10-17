use core::slice;
use anyhow::Ok;
use miden_client::{account::AccountId, asset::FungibleAsset, note::{NoteExecutionMode, NoteTag}, testing::{account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1, NoteBuilder}, transaction::{AccountInterface, OutputNote}};
use miden_crypto::{Felt, Word};
use miden_testing::{Auth, MockChain, MockChainBuilder, TransactionContext, TransactionContextBuilder};
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use miden_objects::{account::Account, note::NoteType};
use midenid_contracts::{notes::{create_naming_set_payment_token_contract, create_naming_set_pricing_root, create_price_set_note}, utils::{create_account, create_naming_account, create_naming_library, create_pricing_account, get_price_set_notes, get_test_prices}};
use midenid_contracts::notes::{get_note_code, create_naming_initialize_note, create_pricing_initialize_note};


pub struct InitializedPricingAccount {
    pub tx_sender_account: Account,
    pub setter_account: Account,
    pub pricing_account: Account,
    pub mock_chain: MockChain,
    pub txs: Vec<TransactionContext>
}

async fn init_pricing_account(mut builder: MockChainBuilder, fungible_asset: FungibleAsset) -> anyhow::Result<InitializedPricingAccount> {
    //let mut builder = MockChain::builder();

    //let fungible_asset = FungibleAsset::new(token_id, 100000).unwrap();

    let tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let setter_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let pricing_account = create_pricing_account();

    let initialize_input_note = create_pricing_initialize_note(tx_sender_account.clone(), fungible_asset.faucet_id(), setter_account.clone(), pricing_account.clone()).await.unwrap();

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

    let tx_inputs = mock_chain.get_transaction_inputs(pricing_account.clone(), None, &[initialize_input_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

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

    let executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;
    Ok(InitializedPricingAccount {
        tx_sender_account,
        setter_account,
        pricing_account: updated_pricing_account,
        mock_chain,
        txs: vec![]
    })
}


// Develop test like that
// https://github.com/0xMiden/miden-base/blob/719ff03d1482e6ce2ad4e986f59ec7b9a8ddf962/crates/miden-testing/src/kernel_tests/tx/test_fpi.rs#L515

#[tokio::test]
async fn test_naming_init() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let treasury_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let naming_account = create_naming_account();

    let initialize_input_note = create_naming_initialize_note(owner_account.clone(), treasury_account.clone(), naming_account.clone()).await.unwrap();

    builder.add_note(OutputNote::Full(initialize_input_note.clone()));

    let mut mock_chain = builder.build()?;

    let tx_inputs = mock_chain.get_transaction_inputs(naming_account.clone(), None, &[initialize_input_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(naming_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_naming_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;
    
    let init_flag = updated_naming_account.storage().get_item(0)?.get(0).unwrap().as_int();
    let owner_slot = updated_naming_account.storage().get_item(1)?;
    let trasury_slot = updated_naming_account.storage().get_item(2)?;

    assert_eq!(init_flag, 1);
    assert_eq!(owner_account.id().prefix().as_u64(), owner_slot.get(1).unwrap().as_int());
    assert_eq!(owner_account.id().suffix().as_int(), owner_slot.get(0).unwrap().as_int());
    assert_eq!(treasury_account.id().prefix().as_u64(), trasury_slot.get(1).unwrap().as_int());
    assert_eq!(treasury_account.id().suffix().as_int(), trasury_slot.get(0).unwrap().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_set_pricing_contract_and_root() -> anyhow::Result<()> {

    let mut builder = MockChain::builder();
    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let treasury_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_setter_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let naming_account = create_naming_account();
    let pricing_account = create_pricing_account();

    let initialize_naming_note = create_naming_initialize_note(
        owner_account.clone(),
        treasury_account.clone(),
        naming_account.clone()
    ).await.unwrap();

    let initialize_pricing_note = create_pricing_initialize_note(
        pricing_tx_sender_account.clone(), 
        fungible_asset.faucet_id(), 
        pricing_setter_account.clone(), 
        pricing_account.clone()
    ).await.unwrap();

    let set_payment_token_note = create_naming_set_payment_token_contract(
        owner_account.clone(), 
        fungible_asset.faucet_id(), 
        pricing_account.id(), 
        naming_account.clone()
    ).await?;

    // TODO: set pricing root too

    let test_prices = get_test_prices();
    let set_notes = get_price_set_notes(pricing_setter_account.clone(), pricing_account.clone(), test_prices).await;

    builder.add_note(OutputNote::Full(initialize_naming_note.clone()));
    builder.add_note(OutputNote::Full(initialize_pricing_note.clone()));
    
    builder.add_note(OutputNote::Full(set_notes[0].clone()));
    builder.add_note(OutputNote::Full(set_notes[1].clone()));
    builder.add_note(OutputNote::Full(set_notes[2].clone()));
    builder.add_note(OutputNote::Full(set_notes[3].clone()));
    builder.add_note(OutputNote::Full(set_notes[4].clone()));
    builder.add_note(OutputNote::Full(set_payment_token_note.clone()));


    let mut mock_chain = builder.build()?;

    //builder.add_note(OutputNote::Full(set_payment_token_note.clone()));
    //mock_chain = builder.build()?;

    // Init naming
    let tx_inputs = mock_chain.get_transaction_inputs(naming_account.clone(), None, &[initialize_naming_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(naming_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_naming_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Init pricing
    let tx_inputs = mock_chain.get_transaction_inputs(pricing_account.clone(), None, &[initialize_pricing_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;

    // Set prices
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

    let executed_tx = setter_tx_context.execute().await?;

    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;
    
    // set pricing contract on naming

    let set_pricing_contract_inputs= mock_chain.get_transaction_inputs(updated_naming_account.clone(),
     None,
     &[set_payment_token_note.id()],
     &[])?;

    let set_pricing_contract_tx_context = TransactionContextBuilder::new(updated_naming_account.clone())
        .account_seed(None)
        .tx_inputs(set_pricing_contract_inputs)
        .build()?;

    let executed_tx = set_pricing_contract_tx_context.execute().await?;

    let updated_naming_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let token_to_price_map_value = updated_naming_account.storage()
        .get_map_item(3, Word::new([fungible_asset.faucet_id().suffix(),fungible_asset.faucet_id().prefix().into(),Felt::new(0),Felt::new(0)]))?;

    assert_eq!(token_to_price_map_value.get(0).unwrap().as_int(), pricing_account.id().suffix().as_int());
    assert_eq!(token_to_price_map_value.get(1).unwrap().as_int(), pricing_account.id().prefix().as_felt().as_int());

    Ok(())
}