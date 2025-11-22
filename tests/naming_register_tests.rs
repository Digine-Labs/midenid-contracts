mod test_utils;

use miden_client::{asset::FungibleAsset, note::{NoteAssets, NoteInputs}};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, unsafe_encode_domain};
use test_utils::init_naming;

use crate::test_utils::{add_note_to_builder, create_note_for_naming, execute_note, execute_notes_and_build_chain, get_test_prices, create_note_for_naming_with_custom_serial_num};

#[tokio::test]
async fn test_naming_initialize() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let mut chain = ctx.builder.build()?;

    execute_note(&mut chain, ctx.initialize_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, ctx.set_prices_note.id(), &mut ctx.naming).await?;
    
    let init_slot = ctx.naming.storage().get_item(0)?;
    let owner_slot = ctx.naming.storage().get_item(1)?;
    let one_year_slot = ctx.naming.storage().get_item(13)?;

    assert_eq!(init_slot.get(0).unwrap().as_int(), 1);
    assert_eq!(owner_slot.get(1).unwrap().as_int(), ctx.owner.id().prefix().as_u64());
    assert_eq!(owner_slot.get(0).unwrap().as_int(), ctx.owner.id().suffix().as_int());
    assert_eq!(one_year_slot.get(0).unwrap().as_int(), 500);

    // Assert prices
    let mock_prices = get_test_prices();
    for i in 1..=5 { 
        let price_slot = ctx.naming.storage()
            .get_map_item(2, 
                Word::new([
                        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
                        ctx.fungible_asset.faucet_id().prefix().as_felt(),
                        Felt::new(i as u64),
                        Felt::new(0)
                    ]))?;
        assert_eq!(price_slot.get(0).unwrap().as_int(), mock_prices[i as usize].as_int());
    }

    
    Ok(())
}
 
#[tokio::test]
async fn test_naming_register_activate() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    let activate_note = create_note_for_naming("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_1.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, activate_note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id(), register_note.id()], &mut ctx.naming).await?;
    //execute_note(&mut ctx.chain, note, &mut ctx.naming).await?;

    

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);
    
    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);
    
    // Activate domain

    
    execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int()); // Now domain mapping must be matched
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());
    assert_eq!(id_to_domain, domain_word);
    Ok(())
}

#[tokio::test]
async fn test_naming_register_activate_by_not_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    let activate_note = create_note_for_naming("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_2.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, activate_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);
    
    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);
    
    // Activate domain - should fail because registrar_2 is not the owner

    
    let result = execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await;

    // This should fail because registrar_2 is not the domain owner
    assert!(result.is_err(), "Expected activation by non-owner to fail, but it succeeded");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_already_exist_domain() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_2.id(), ctx.naming.id(), register_asset).await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    // Try to register again with different owner
    
    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected domain register fails. But it succeeded");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_already_exist_domain_from_same_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    //execute_note(&mut ctx.chain, note, &mut ctx.naming).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    // Try to register again with different owner
    
    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected domain register fails. But it succeeded");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_two_domains_activate_after() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Notes
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_1 = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, register_note_1.clone())?;
    let activate_note_1 = create_note_for_naming("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_1.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    
    add_note_to_builder(&mut ctx.builder, activate_note_1.clone())?;    
    let second_domain = encode_domain_as_felts("test2".to_string());
    let second_domain_word = encode_domain("test2".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        second_domain[0],
        second_domain[1],
        second_domain[2],
        second_domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;    
    

    let activate_note_2 = create_note_for_naming("activate_domain".to_string(), NoteInputs::new(second_domain_word.to_vec())?, ctx.registrar_1.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    add_note_to_builder(&mut ctx.builder, activate_note_2.clone())?;  

    // Execution
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, register_note_1.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);
    
    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);
    
    // Activate domain

    execute_note(&mut chain, activate_note_1.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int()); // Now domain mapping must be matched
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());
    assert_eq!(id_to_domain, domain_word);

    // Register new domain

    
    execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await?;

    let second_domain_owner_slot = ctx.naming.storage().get_map_item(5, second_domain_word)?;
    let second_domain_expiry_slot = ctx.naming.storage().get_map_item(12, second_domain_word)?;

    assert_eq!(second_domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(second_domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(second_domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    // Now activate second domain

    
    execute_note(&mut chain, activate_note_2.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, second_domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int()); // Now domain mapping must be matched
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());
    assert_eq!(id_to_domain, second_domain_word);

    // Check first domain mapping

    let first_domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    assert_eq!(first_domain_to_id.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int()); // First domain must remain mapping to old address
    assert_eq!(first_domain_to_id.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    // Ensure protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555 + 123);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 2);
    Ok(())
}

#[tokio::test]
async fn test_naming_register_less_amount() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_higher_amount() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 1200)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, note.id(), &mut ctx.naming).await?;

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555); // Protocol only saves actual cost as revenue

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);

    Ok(())
}

#[tokio::test]
async fn test_naming_register_wrong_letter_length() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("testtesttesttest".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        Felt::new(11),
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_too_much_letters() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = unsafe_encode_domain("testtesttesttest123123123123".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_empty_domain() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = unsafe_encode_domain("".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        Felt::new(3),
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 1231234)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_expired_domain_clear() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    let clear_note_inputs = NoteInputs::new([
            domain[0],
            domain[1],
            domain[2],
            domain[3], 
        ].to_vec())?;
    let clear_note = create_note_for_naming("clear_expired_domain".to_string(), clear_note_inputs, ctx.registrar_2.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    add_note_to_builder(&mut ctx.builder, clear_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    chain.prove_until_block(100)?;

    execute_note(&mut chain, clear_note.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(),0);
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), 0);

    assert_eq!(domain_expiry_slot.get(0).unwrap().as_int(), 0);

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);
    Ok(())
}

#[tokio::test]
async fn test_expired_domain_clear_not_expired() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    let clear_note_inputs = NoteInputs::new([
            domain[0],
            domain[1],
            domain[2],
            domain[3], 
        ].to_vec())?;
    let clear_note = create_note_for_naming("clear_expired_domain".to_string(), clear_note_inputs, ctx.registrar_2.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    add_note_to_builder(&mut ctx.builder, clear_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    chain.prove_until_block(10)?;

    let result = execute_note(&mut chain, clear_note.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}
#[tokio::test]
async fn test_expired_domain_rebuy() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    let activate_note_1 = create_note_for_naming_with_custom_serial_num("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_1.id(), ctx.naming.id(), NoteAssets::new(vec![])?, Word::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)])).await?;
    add_note_to_builder(&mut ctx.builder, activate_note_1.clone())?; 

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 556)?; // Pay more to not create same nullifier
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_2.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let activate_note_2 = create_note_for_naming_with_custom_serial_num("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_2.id(), ctx.naming.id(), NoteAssets::new(vec![])?, Word::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(5)])).await?;
    add_note_to_builder(&mut ctx.builder, activate_note_2.clone())?; 

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;

    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, activate_note_1.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    chain.prove_until_block(100)?;

    execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_2.id().suffix().as_int()), Felt::new(ctx.registrar_2.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;

    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_2.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_2.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());
    
    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);

    execute_note(&mut chain, activate_note_2.id(), &mut ctx.naming).await?;

    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_2.id().suffix().as_int()), Felt::new(ctx.registrar_2.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    
    assert_eq!(domain_to_id.get(0).unwrap().as_int(), ctx.registrar_2.id().suffix().as_int()); // Now domain mapping must be matched
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), ctx.registrar_2.id().prefix().as_u64());
    assert_eq!(id_to_domain, domain_word);
    Ok(())
}

#[tokio::test]
async fn test_extend_domain_by_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let extend_note = create_note_for_naming("extend_domain".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, extend_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;

    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    
    let current_expiry = domain_expiry_slot.get(0).unwrap().as_int();
    execute_note(&mut chain, extend_note.id(), &mut ctx.naming).await?;

    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    let updated_expiry = domain_expiry_slot.get(0).unwrap().as_int();

    assert_eq!(updated_expiry, current_expiry + ctx.one_year as u64);
    Ok(())
}

#[tokio::test]
async fn test_extend_expired_domain() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let extend_note = create_note_for_naming("extend_domain".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, extend_note.clone())?;
    
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;

    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    chain.prove_until_block(100)?;
    
    let result = execute_note(&mut chain, extend_note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected fail");

    Ok(())
}

#[tokio::test]
async fn test_extend_domain_by_not_owner() -> anyhow::Result<()> {
        let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(1), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let extend_note = create_note_for_naming("extend_domain".to_string(), register_note_inputs.clone(), ctx.registrar_2.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, extend_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;

    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    let result = execute_note(&mut chain, extend_note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected fail");
    Ok(())
}

#[tokio::test]
async fn test_register_with_discount_5yr() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(5), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let discounted_cost = 1390; // some rounding

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), discounted_cost)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let current_time = chain.latest_block_header().timestamp();
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    
    let expiry = domain_expiry_slot.get(0).unwrap().as_int();
    
    let expected_expiry = current_time + (ctx.one_year * 5);
    assert_eq!(expiry, expected_expiry as u64);

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), discounted_cost);

    Ok(())
}

#[tokio::test]
async fn test_register_with_discount_3yr() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(3), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let discounted_cost = 1167; // some rounding
    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), discounted_cost)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let current_time = chain.latest_block_header().timestamp();
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    
    let expiry = domain_expiry_slot.get(0).unwrap().as_int();
    
    let expected_expiry = current_time + (ctx.one_year * 3);
    assert_eq!(expiry, expected_expiry as u64);

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), discounted_cost);

    Ok(())
}

#[tokio::test]
async fn test_register_with_discount_10yr() -> anyhow::Result<()> {
let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0),
        Felt::new(0),
        domain[0],
        domain[1],
        domain[2],
        domain[3],
        Felt::new(10), // register length
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;

    let discounted_cost = 2780; // some rounding

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), discounted_cost)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs.clone(), ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id()], &mut ctx.naming).await?;
    let current_time = chain.latest_block_header().timestamp();
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    let domain_expiry_slot = ctx.naming.storage().get_map_item(12, domain_word)?;
    
    let expiry = domain_expiry_slot.get(0).unwrap().as_int();
    
    let expected_expiry = current_time + (ctx.one_year * 10);
    assert_eq!(expiry, expected_expiry as u64);

    let total_revenue_slot = ctx.naming.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), discounted_cost);

    Ok(())
}