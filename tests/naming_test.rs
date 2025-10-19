
use anyhow::Ok;
use miden_client::{asset::FungibleAsset, testing::{account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1}, transaction::{OutputNote}};
use miden_crypto::{Felt, Word};
use miden_testing::{Auth, MockChain, TransactionContextBuilder};
use miden_objects::{account::Account, transaction::AccountInputs};
use midenname_contracts::{notes::{create_naming_register_name_note, create_naming_set_payment_token_contract, create_naming_set_pricing_root}, utils::{create_naming_account, create_pricing_account, encode_domain, get_price_set_notes, get_test_prices}};
use midenname_contracts::notes::{create_naming_initialize_note, create_pricing_initialize_note};

// Develop test like that
// https://github.com/0xMiden/miden-base/blob/719ff03d1482e6ce2ad4e986f59ec7b9a8ddf962/crates/miden-testing/src/kernel_tests/tx/test_fpi.rs#L515

pub struct InitializedNamingAndPricing {
    pub mock_chain: MockChain,
    pub owner_account: Account,
    pub domain_registrar_account: Account,
    pub treasury_account: Account,
    pub pricing_tx_sender_account: Account,
    pub pricing_setter_account: Account,
    pub naming_account: Account,
    pub pricing_account: Account,
    //pub faucet: Account,
    pub fungible_asset: FungibleAsset,
}

async fn initiate_pricing_and_naming() -> anyhow::Result<InitializedNamingAndPricing>{
    
    let mut builder = MockChain::builder();
    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();
    //let faucet = builder.create_new_faucet(Auth::Noop, "TEST", 1_000_000)?;

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let domain_registrar_account = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset.into()])?;
    
    let amount = domain_registrar_account.vault().get_balance(fungible_asset.faucet_id())?;
    println!("Registrar initial balance: {}", amount);

    let treasury_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_setter_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let naming_account = create_naming_account();
    let pricing_account = create_pricing_account();

    println!(r"Account Addresses:
    owner_account: {}
    domain_registrar_account: {}
    treasury_account: {}
    pricing_tx_sender_account: {}
    pricing_setter_account: {}
    naming_account: {}
    pricing_account: {}
    faucet_account: {}",
        owner_account.id(),
        domain_registrar_account.id(),
        treasury_account.id(),
        pricing_tx_sender_account.id(),
        pricing_setter_account.id(),
        naming_account.id(),
        pricing_account.id(),
        fungible_asset.faucet_id()
    );

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
    //builder.add_note(OutputNote::Full(set_pricing_root_note.clone()));
    //builder.add_note(OutputNote::Full(register_name_note.clone()));

    builder.add_account(naming_account.clone())?;
    builder.add_account(pricing_account.clone())?;
    //builder.add_existing_faucet(auth_method, token_symbol, max_supply, total_issuance)

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

    mock_chain.prove_next_block()?;
    let root_on_contract = updated_pricing_account.storage().get_item(4).unwrap();

    let set_pricing_root_note = create_naming_set_pricing_root(
        owner_account.clone(), 
        root_on_contract, 
        naming_account.clone()
    ).await?;

    println!("{}", root_on_contract.to_string());
    println!("NoteId: {}", set_pricing_root_note.id());

    mock_chain.add_pending_note(OutputNote::Full(set_pricing_root_note.clone()));
    mock_chain.prove_next_block()?;
    let set_pricing_root_inputs= mock_chain.get_transaction_inputs(updated_naming_account.clone(),
     None,
     &[set_pricing_root_note.id()],
     &[])?;

    let set_pricing_root_tx_context = TransactionContextBuilder::new(updated_naming_account.clone())
        .account_seed(None)
        .tx_inputs(set_pricing_root_inputs)
        .build()?;

    let executed_tx = set_pricing_root_tx_context.execute().await?;

    let updated_naming_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

    Ok(InitializedNamingAndPricing {
        mock_chain,
        owner_account,
        domain_registrar_account,
        treasury_account,
        pricing_tx_sender_account,
        pricing_setter_account,
        naming_account: updated_naming_account,
        pricing_account: updated_pricing_account,
        fungible_asset,
    })
}

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
async fn test_naming_register() -> anyhow::Result<()> {

    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        encode_domain("test".to_string()), 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let _register_name_inputs= setup.mock_chain.get_transaction_inputs(setup.naming_account.clone(),
     None,
     &[register_name_note.id()],
     &[])?;
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

    let pricing_account_witnesses = setup.mock_chain.account_witnesses(vec![pricing_account_id]);
    
    let _pricing_account_witness = pricing_account_witnesses
        .get(&pricing_account_id)
        .unwrap()
        .clone();
    let foreign_pricing_account_input = AccountInputs::new(
        setup.pricing_account.clone().into(),
        pricing_account_from_chain
    );
    let register_name_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(register_name_inputs)
        .foreign_accounts(vec![foreign_pricing_account_input])
        .build()?;

    let executed_tx = register_name_tx_context.execute().await?;

    let _updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;
    
    Ok(())
}
