mod test_utils;

use miden_client::{
    asset::FungibleAsset,
    note::{NoteAssets, NoteInputs},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, unsafe_encode_domain};
use test_utils::init_naming;

use crate::test_utils::{
    add_note_to_builder, create_note_for_naming, create_note_for_naming_with_custom_serial_num,
    execute_note, execute_notes_and_build_chain, get_test_prices,
};

#[tokio::test]
async fn test_naming_initialize() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let mut chain = ctx.builder.build()?;

    execute_note(&mut chain, ctx.initialize_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, ctx.set_prices_note.id(), &mut ctx.naming).await?;

    let init_slot = ctx.naming.storage().get_item(0)?;
    let owner_slot = ctx.naming.storage().get_item(1)?;

    assert_eq!(init_slot.get(0).unwrap().as_int(), 1);
    assert_eq!(
        owner_slot.get(1).unwrap().as_int(),
        ctx.owner.id().prefix().as_u64()
    );
    assert_eq!(
        owner_slot.get(0).unwrap().as_int(),
        ctx.owner.id().suffix().as_int()
    );

    // Assert prices
    let mock_prices = get_test_prices();
    for i in 1..=5 {
        let price_slot = ctx.naming.storage().get_map_item(
            2,
            Word::new([
                Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
                ctx.fungible_asset.faucet_id().prefix().as_felt(),
                Felt::new(i as u64),
                Felt::new(0),
            ]),
        )?;
        assert_eq!(
            price_slot.get(0).unwrap().as_int(),
            mock_prices[i as usize].as_int()
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_naming_register_activate() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    let activate_note = create_note_for_naming(
        "activate_domain".to_string(),
        NoteInputs::new(domain_word.to_vec())?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, activate_note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            register_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;
    //execute_note(&mut ctx.chain, note, &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_owner_slot.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    );
    assert_eq!(
        domain_owner_slot.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);

    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(
        10,
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);

    // Activate domain

    execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_to_id.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    ); // Now domain mapping must be matched
    assert_eq!(
        domain_to_id.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );
    assert_eq!(id_to_domain, domain_word);
    Ok(())
}

#[tokio::test]
async fn test_naming_register_activate_by_not_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    let activate_note = create_note_for_naming(
        "activate_domain".to_string(),
        NoteInputs::new(domain_word.to_vec())?,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;

    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, activate_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_owner_slot.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    );
    assert_eq!(
        domain_owner_slot.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);

    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(
        10,
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);

    // Activate domain - should fail because registrar_2 is not the owner

    let result = execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await;

    // This should fail because registrar_2 is not the domain owner
    assert!(
        result.is_err(),
        "Expected activation by non-owner to fail, but it succeeded"
    );
    Ok(())
}

#[tokio::test]
async fn test_naming_register_already_exist_domain() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs.clone(),
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;

    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    // Try to register again with different owner

    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;

    assert!(
        result.is_err(),
        "Expected domain register fails. But it succeeded"
    );
    Ok(())
}

#[tokio::test]
async fn test_naming_register_already_exist_domain_from_same_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs.clone(),
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    //execute_note(&mut ctx.chain, note, &mut ctx.naming).await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;
    // Try to register again with different owner

    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;

    assert!(
        result.is_err(),
        "Expected domain register fails. But it succeeded"
    );
    Ok(())
}

#[tokio::test]
async fn test_naming_register_two_domains_activate_after() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Notes
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_1 = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note_1.clone())?;
    let activate_note_1 = create_note_for_naming(
        "activate_domain".to_string(),
        NoteInputs::new(domain_word.to_vec())?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;

    add_note_to_builder(&mut ctx.builder, activate_note_1.clone())?;
    let second_domain = encode_domain_as_felts("test2".to_string());
    let second_domain_word = encode_domain("test2".to_string());
    let register_note_inputs = NoteInputs::new(
        [
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
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let activate_note_2 = create_note_for_naming(
        "activate_domain".to_string(),
        NoteInputs::new(second_domain_word.to_vec())?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, activate_note_2.clone())?;

    // Execution
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, register_note_1.id(), &mut ctx.naming).await?;

    let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;
    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_owner_slot.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    );
    assert_eq!(
        domain_owner_slot.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);

    // Protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(
        10,
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);

    // Activate domain

    execute_note(&mut chain, activate_note_1.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_to_id.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    ); // Now domain mapping must be matched
    assert_eq!(
        domain_to_id.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );
    assert_eq!(id_to_domain, domain_word);

    // Register new domain

    execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await?;

    let second_domain_owner_slot = ctx.naming.storage().get_map_item(5, second_domain_word)?;

    assert_eq!(
        second_domain_owner_slot.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    );
    assert_eq!(
        second_domain_owner_slot.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );

    // Now activate second domain

    execute_note(&mut chain, activate_note_2.id(), &mut ctx.naming).await?; // Use always updated account as target

    let domain_to_id = ctx.naming.storage().get_map_item(4, second_domain_word)?;
    let id_to_domain = ctx.naming.storage().get_map_item(
        3,
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_int()),
            Felt::new(ctx.registrar_1.id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;

    assert_eq!(
        domain_to_id.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    ); // Now domain mapping must be matched
    assert_eq!(
        domain_to_id.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );
    assert_eq!(id_to_domain, second_domain_word);

    // Check first domain mapping

    let first_domain_to_id = ctx.naming.storage().get_map_item(4, domain_word)?;
    assert_eq!(
        first_domain_to_id.get(0).unwrap().as_int(),
        ctx.registrar_1.id().suffix().as_int()
    ); // First domain must remain mapping to old address
    assert_eq!(
        first_domain_to_id.get(1).unwrap().as_int(),
        ctx.registrar_1.id().prefix().as_u64()
    );

    // Ensure protocol values

    let total_revenue_slot = ctx.naming.storage().get_map_item(
        10,
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555 + 123);

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 2);
    Ok(())
}

#[tokio::test]
async fn test_naming_register_less_amount() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_higher_amount() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 1200)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, note.id(), &mut ctx.naming).await?;

    let total_revenue_slot = ctx.naming.storage().get_map_item(
        10,
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555); // Protocol only saves actual cost as revenue

    let total_domain_count = ctx.naming.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);

    Ok(())
}

#[tokio::test]
async fn test_naming_register_wrong_letter_length() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("testtesttesttest".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            Felt::new(11),
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_too_much_letters() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = unsafe_encode_domain("testtesttesttest123123123123".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 554)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_empty_domain() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = unsafe_encode_domain("".to_string());
    let register_note_inputs = NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            Felt::new(3),
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 1231234)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    let result = execute_note(&mut chain, note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Expected revert but succeeded.");
    Ok(())
}

#[tokio::test]
async fn test_naming_register_70_domains() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create all 70 registration notes upfront
    let mut register_notes = Vec::new();
    for i in 0..70 {
        let domain_name = format!("domain{}", i);

        let domain = encode_domain_as_felts(domain_name.clone());
        let register_note_inputs = NoteInputs::new(
            [
                Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
                ctx.fungible_asset.faucet_id().prefix().as_felt(),
                Felt::new(0),
                Felt::new(0),
                domain[0],
                domain[1],
                domain[2],
                domain[3],
            ]
            .to_vec(),
        )?;

        // Cost depends on domain length, using higher amount to cover various lengths
        let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123123)?; // Price for 1 letter
        let register_asset = NoteAssets::new(vec![cost.into()])?;

        // Create unique serial number for each note to avoid conflicts
        let serial_num = Word::new([
            Felt::new(i as u64),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]);

        let register_note = create_note_for_naming_with_custom_serial_num(
            "register_name".to_string(),
            register_note_inputs,
            ctx.registrar_1.id(),
            ctx.naming.id(),
            register_asset,
            serial_num,
        )
        .await?;

        add_note_to_builder(&mut ctx.builder, register_note.clone())?;
        register_notes.push((register_note, domain_name));
    }

    // Build the chain with initialization notes
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Execute each registration note and verify
    for (i, (register_note, domain_name)) in register_notes.iter().enumerate() {
        println!("Registering domain #{}: {}", i + 1, domain_name);

        let result = execute_note(&mut chain, register_note.id(), &mut ctx.naming).await;

        // Check each registration individually
        if result.is_err() {
            println!(
                "ERROR: Failed to register domain #{}: {}",
                i + 1,
                domain_name
            );
            if i >= 64 {
                println!(
                    "CRITICAL: Error occurred after 64th registration at index {}",
                    i
                );
            } else {
                println!("Error occurred at index {}", i);
            }
            return result;
        } else {
            if i >= 64 {
                println!(
                    "SUCCESS: Domain #{} registered successfully (after 64th threshold)",
                    i + 1
                );
            } else {
                println!("SUCCESS: Domain #{} registered", i + 1);
            }
        }

        // Verify domain was registered
        let domain_word = encode_domain(domain_name.clone());
        let domain_owner_slot = ctx.naming.storage().get_map_item(5, domain_word)?;

        assert_eq!(
            domain_owner_slot.get(0).unwrap().as_int(),
            ctx.registrar_1.id().suffix().as_int(),
            "Domain #{} owner suffix mismatch",
            i + 1
        );
        assert_eq!(
            domain_owner_slot.get(1).unwrap().as_int(),
            ctx.registrar_1.id().prefix().as_u64(),
            "Domain #{} owner prefix mismatch",
            i + 1
        );

        // Check domain count (slot 9) and revenue after each registration
        if (i + 1) % 10 == 0 || i >= 63 {
            // Check domain count in slot 9
            let domain_count_slot = ctx.naming.storage().get_item(9)?;
            let current_domain_count = domain_count_slot.get(0).unwrap().as_int();

            // Check revenue in slot 10
            let total_revenue_slot = ctx.naming.storage().get_map_item(
                10,
                Word::new([
                    Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
                    Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
                    Felt::new(0),
                    Felt::new(0),
                ]),
            )?;
            let current_revenue = total_revenue_slot.get(0).unwrap().as_int();

            println!(
                "After domain #{}: Domain count in slot 9 = {} (expected: {}), Revenue = {} (expected: {})",
                i + 1,
                current_domain_count,
                i + 1,
                current_revenue,
                123123 * (i + 1)
            );

            // Assert domain count matches
            assert_eq!(
                current_domain_count,
                (i + 1) as u64,
                "Domain count mismatch after registration #{}",
                i + 1
            );
        }
    }

    // Verify total domain count
    let total_domain_count = ctx.naming.storage().get_item(9)?;
    println!(
        "Total domain count: {}",
        total_domain_count.get(0).unwrap().as_int()
    );
    assert_eq!(
        total_domain_count.get(0).unwrap().as_int(),
        70,
        "Expected 70 domains registered"
    );

    println!("All 70 domains registered successfully!");
    Ok(())
}
