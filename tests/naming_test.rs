
use anyhow::Ok;
use miden_client::{asset::FungibleAsset, testing::{account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1}, transaction::{OutputNote}};
use miden_crypto::{Felt, Word};
use miden_testing::{Auth, MockChain, TransactionContextBuilder};
use miden_objects::{account::Account, transaction::AccountInputs};
use midenname_contracts::{notes::{create_naming_register_name_note, create_naming_set_payment_token_contract, create_naming_set_pricing_root, create_naming_transfer_note, create_naming_transfer_owner_note}, utils::{create_naming_account, create_pricing_account, encode_domain, get_price_set_notes, get_test_prices, unsafe_encode_domain}};
use midenname_contracts::notes::{create_naming_initialize_note, create_pricing_initialize_note};

pub struct InitializedNamingAndPricing {
    pub mock_chain: MockChain,
    pub owner_account: Account,
    pub domain_registrar_account: Account,
    pub domain_registrar_account_2: Account,
    pub domain_registrar_account_3: Account,
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
    let fungible_asset_1 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();
    let fungible_asset_2 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 50000).unwrap();
    let fungible_asset_3 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 20000).unwrap();
    //let faucet = builder.create_new_faucet(Auth::Noop, "TEST", 1_000_000)?;

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let domain_registrar_account = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_1.into()])?;
    let domain_registrar_account_2 = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_2.into()])?;
    let domain_registrar_account_3 = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_3.into()])?;

    let treasury_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_tx_sender_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let pricing_setter_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let naming_account = create_naming_account();
    let pricing_account = create_pricing_account();

    /*println!(r"Account Addresses:
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
        fungible_asset_1.faucet_id()
    );*/

    let initialize_naming_note = create_naming_initialize_note(
        owner_account.clone(),
        treasury_account.clone(),
        naming_account.clone()
    ).await.unwrap();

    let initialize_pricing_note = create_pricing_initialize_note(
        pricing_tx_sender_account.clone(), 
        fungible_asset_1.faucet_id(), 
        pricing_setter_account.clone(), 
        pricing_account.clone()
    ).await.unwrap();

    let set_payment_token_note = create_naming_set_payment_token_contract(
        owner_account.clone(), 
        fungible_asset_1.faucet_id(), 
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

    let executed_tx = tx_context.execute().await?;
    let updated_naming_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

    // Init pricing
    let tx_inputs = mock_chain.get_transaction_inputs(pricing_account.clone(), None, &[initialize_pricing_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(pricing_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let executed_tx = tx_context.execute().await?;
    let updated_pricing_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;

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
        .get_map_item(3, Word::new([fungible_asset_1.faucet_id().suffix(),fungible_asset_1.faucet_id().prefix().into(),Felt::new(0),Felt::new(0)]))?;

    assert_eq!(token_to_price_map_value.get(0).unwrap().as_int(), pricing_account.id().suffix().as_int());
    assert_eq!(token_to_price_map_value.get(1).unwrap().as_int(), pricing_account.id().prefix().as_felt().as_int());

    mock_chain.prove_next_block()?;
    let root_on_contract = updated_pricing_account.storage().get_item(4).unwrap();

    let set_pricing_root_note = create_naming_set_pricing_root(
        owner_account.clone(), 
        root_on_contract, 
        naming_account.clone()
    ).await?;

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
        domain_registrar_account_2,
        domain_registrar_account_3,
        treasury_account,
        pricing_tx_sender_account,
        pricing_setter_account,
        naming_account: updated_naming_account,
        pricing_account: updated_pricing_account,
        fungible_asset: fungible_asset_1,
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

    let executed_tx = tx_context.execute().await?;
    let updated_naming_account = mock_chain.add_pending_executed_transaction(&executed_tx)?;
    
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
    let domain = encode_domain("test".to_string());
    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let domain_to_id_map = updated_naming_account.storage().get_map_item(5, domain).unwrap();
    let id_to_domain_map =updated_naming_account.storage().get_map_item(4, Word::new([Felt::new(setup.domain_registrar_account.id().suffix().as_int()), Felt::new(setup.domain_registrar_account.id().prefix().as_felt().as_int()), Felt::new(0), Felt::new(0)])).unwrap();
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_id_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_id_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    assert_eq!(id_to_domain_map, domain);

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());
    
    Ok(())
}

#[tokio::test]
async fn test_naming_register_exist_name() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = encode_domain("test".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name

    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;
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
    setup.mock_chain.prove_next_block()?;
    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;
    
    // Try to register test domain again

        let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account_2.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        updated_naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;


    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        updated_naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

    let foreign_pricing_account_input = AccountInputs::new(
        setup.pricing_account.clone().into(),
        pricing_account_from_chain
    );
    let register_name_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(register_name_inputs)
        .foreign_accounts(vec![foreign_pricing_account_input])
        .build()?;

    let _executed_tx = register_name_tx_context.execute().await;
    assert!(_executed_tx.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_naming_register_wrong_payment() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 350)?;
    let domain = encode_domain("test".to_string());
    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

    let foreign_pricing_account_input = AccountInputs::new(
        setup.pricing_account.clone().into(),
        pricing_account_from_chain
    );
    let register_name_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(register_name_inputs)
        .foreign_accounts(vec![foreign_pricing_account_input])
        .build()?;

    let executed_tx = register_name_tx_context.execute().await;

    assert!(executed_tx.is_err());
    Ok(())
}

#[tokio::test]
async fn test_naming_transfer_domain() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = encode_domain("test".to_string());
    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name

    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    // Verify domain owner is registrar_1
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    // Transfer domain

    let transfer_domain_note = create_naming_transfer_note(setup.domain_registrar_account, setup.domain_registrar_account_2.id(), domain, updated_naming_account.clone()).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(transfer_domain_note.clone()));
    setup.mock_chain.prove_next_block()?;

    let transfer_domain_inputs = setup.mock_chain.get_transaction_inputs(
        updated_naming_account.clone(),
        None,
        &[transfer_domain_note.id()],
        &[]
    )?;

    let transfer_domain_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(transfer_domain_inputs)
        .build()?;

    let executed_tx = transfer_domain_tx_context.execute().await?;

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let updated_domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(updated_domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account_2.id().suffix().as_int());
    assert_eq!(updated_domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account_2.id().prefix().as_felt().as_int());
    Ok(())

}

#[tokio::test]
async fn test_naming_transfer_domain_not_from_owner() -> anyhow::Result<()> {
        let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = encode_domain("test".to_string());
    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    // Verify domain owner is registrar_1
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    // Transfer domain

    let transfer_domain_note = create_naming_transfer_note(setup.domain_registrar_account_2, setup.domain_registrar_account_3.id(), domain, updated_naming_account.clone()).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(transfer_domain_note.clone()));
    setup.mock_chain.prove_next_block()?;

    let transfer_domain_inputs = setup.mock_chain.get_transaction_inputs(
        updated_naming_account.clone(),
        None,
        &[transfer_domain_note.id()],
        &[]
    )?;

    let transfer_domain_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(transfer_domain_inputs)
        .build()?;

    let executed_tx = transfer_domain_tx_context.execute().await;

    assert!(executed_tx.is_err());

    Ok(())
}

#[tokio::test]
async fn test_naming_register_empty_domain() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = unsafe_encode_domain("".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;
    let foreign_pricing_account_input = AccountInputs::new(
        setup.pricing_account.clone().into(),
        pricing_account_from_chain
    );
    let register_name_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(register_name_inputs)
        .foreign_accounts(vec![foreign_pricing_account_input])
        .build()?;

    let executed_tx = register_name_tx_context.execute().await;

    assert!(executed_tx.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_naming_register_two_felts_domain() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = unsafe_encode_domain("testtest".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let domain_to_id_map = updated_naming_account.storage().get_map_item(5, domain).unwrap();
    let id_to_domain_map =updated_naming_account.storage().get_map_item(4, Word::new([Felt::new(setup.domain_registrar_account.id().suffix().as_int()), Felt::new(setup.domain_registrar_account.id().prefix().as_felt().as_int()), Felt::new(0), Felt::new(0)])).unwrap();
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_id_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_id_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    assert_eq!(id_to_domain_map, domain);

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_register_three_felts_domain() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = unsafe_encode_domain("testtesttesttesttest".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let domain_to_id_map = updated_naming_account.storage().get_map_item(5, domain).unwrap();
    let id_to_domain_map =updated_naming_account.storage().get_map_item(4, Word::new([Felt::new(setup.domain_registrar_account.id().suffix().as_int()), Felt::new(setup.domain_registrar_account.id().prefix().as_felt().as_int()), Felt::new(0), Felt::new(0)])).unwrap();
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_id_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_id_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    assert_eq!(id_to_domain_map, domain);

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_register_max_length_domain() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = unsafe_encode_domain("testmaxlendomain12345".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name

    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;
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

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let domain_to_id_map = updated_naming_account.storage().get_map_item(5, domain).unwrap();
    let id_to_domain_map =updated_naming_account.storage().get_map_item(4, Word::new([Felt::new(setup.domain_registrar_account.id().suffix().as_int()), Felt::new(setup.domain_registrar_account.id().prefix().as_felt().as_int()), Felt::new(0), Felt::new(0)])).unwrap();
    let domain_to_owner_map = updated_naming_account.storage().get_map_item(6, domain).unwrap();

    assert_eq!(domain_to_id_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_id_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());

    assert_eq!(id_to_domain_map, domain);

    assert_eq!(domain_to_owner_map.get(0).unwrap().as_int(), setup.domain_registrar_account.id().suffix().as_int());
    assert_eq!(domain_to_owner_map.get(1).unwrap().as_int(), setup.domain_registrar_account.id().prefix().as_felt().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_register_domain_length_too_high() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let asset = FungibleAsset::new(setup.fungible_asset.faucet_id(), 555)?;
    let domain = unsafe_encode_domain("testmaxlendomain12345123".to_string());

    let register_name_note = create_naming_register_name_note(
        setup.domain_registrar_account.clone(), 
        setup.fungible_asset.faucet_id(), 
        domain, 
        asset,
        setup.naming_account.clone()
        
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(register_name_note.clone()));
    setup.mock_chain.prove_next_block()?;

    // Register name
    let pricing_account_id = setup.pricing_account.id();

    let pricing_account_from_chain = setup.mock_chain.account_tree().open(pricing_account_id);

    let register_name_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(),
        None,
        &[register_name_note.id()],
        &[]
    )?;

    let foreign_pricing_account_input = AccountInputs::new(
        setup.pricing_account.clone().into(),
        pricing_account_from_chain
    );
    let register_name_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(register_name_inputs)
        .foreign_accounts(vec![foreign_pricing_account_input])
        .build()?;

    let executed_tx = register_name_tx_context.execute().await;

    assert!(executed_tx.is_err());
    Ok(())
}

#[tokio::test]
async fn test_naming_ownership_transfer_owner() -> anyhow::Result<()> {
    let mut setup = initiate_pricing_and_naming().await?;

    let transfer_owner_note = create_naming_transfer_owner_note(
        setup.owner_account.clone(),
        setup.domain_registrar_account_3.id(), 
        setup.naming_account.clone()
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(transfer_owner_note.clone()));
    setup.mock_chain.prove_next_block()?;

    let transfer_owner_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(), 
        None, 
        &[transfer_owner_note.id()], 
        &[]
    )?;

    let transfer_owner_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(transfer_owner_inputs)
        .build()?;

    let executed_tx = transfer_owner_tx_context.execute().await?;

    let updated_naming_account = setup.mock_chain.add_pending_executed_transaction(&executed_tx)?;

    let owner_slot = updated_naming_account.storage().get_item(1)?;

    assert_eq!(setup.domain_registrar_account_3.id().prefix().as_u64(), owner_slot.get(1).unwrap().as_int());
    assert_eq!(setup.domain_registrar_account_3.id().suffix().as_int(), owner_slot.get(0).unwrap().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_ownership_transfer_not_owner() -> anyhow::Result<()> {
        let mut setup = initiate_pricing_and_naming().await?;

    let transfer_owner_note = create_naming_transfer_owner_note(
        setup.domain_registrar_account_2.clone(),
        setup.domain_registrar_account_3.id(), 
        setup.naming_account.clone()
    ).await?;

    setup.mock_chain.add_pending_note(OutputNote::Full(transfer_owner_note.clone()));
    setup.mock_chain.prove_next_block()?;

    let transfer_owner_inputs = setup.mock_chain.get_transaction_inputs(
        setup.naming_account.clone(), 
        None, 
        &[transfer_owner_note.id()], 
        &[]
    )?;

    let transfer_owner_tx_context = TransactionContextBuilder::new(setup.naming_account.clone())
        .account_seed(None)
        .tx_inputs(transfer_owner_inputs)
        .build()?;

    let executed_tx = transfer_owner_tx_context.execute().await;
    assert!(executed_tx.is_err());
    Ok(())
}

#[tokio::test]
#[ignore = "not implemented"]
async fn test_naming_ownership_update_treasury() {}

#[tokio::test]
#[ignore = "not implemented"]
async fn test_naming_ownership_withdraw_assets() {}

#[tokio::test]
#[ignore = "not implemented"]
async fn test_naming_update_default_domain() -> anyhow::Result<()> {
    // Register 2 domains from same registrar. Try to set second one as default.
    // Ensure the mapping matches last domain with id. However owner of both domain must be same

    Ok(())
}

#[tokio::test]
#[ignore = "not implemented"]
async fn test_naming_update_default_domain_to_not_owner_domain() {
    // Register 2 domains from same registrar. Try to set second one as default but from another account.
}