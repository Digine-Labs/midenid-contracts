mod test_utils;

use miden_client::Felt;
use miden_objects::{asset::FungibleAsset, note::{NoteAssets, NoteInputs}};
use test_utils::*;

/// Test basic name registration with payment
#[tokio::test]
async fn test_register_name_with_payment() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create name to register
    let name = encode_domain_as_felts("alice".to_string());
    
    // Create registration note with payment
    let register_inputs = NoteInputs::new(
        [name[0], name[1], name[2], name[3]].to_vec(),
    )?;
    
    let payment = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets = NoteAssets::new(vec![payment.into()])?;
    
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets,
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Build chain and execute both init and register notes
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    execute_note(&mut chain, register_note.id(), &mut ctx.naming).await?;

    // Verify name is registered in name->ID map (slot 3)
    let name_word = encode_domain("alice".to_string());
    let name_to_id_value = ctx.naming.storage().get_map_item(3, name_word)?;
    
    // Should contain the user's account ID
    let stored_prefix = name_to_id_value.get(1).unwrap().as_int();
    let stored_suffix = name_to_id_value.get(0).unwrap().as_int();
    
    assert_eq!(stored_prefix, ctx.user_1.id().prefix().as_u64());
    assert_eq!(stored_suffix, ctx.user_1.id().suffix().as_int());

    // Verify reverse mapping in ID->name map (slot 4)
    let user_key = miden_client::Word::new([
        Felt::new(ctx.user_1.id().suffix().as_int()),
        Felt::new(ctx.user_1.id().prefix().as_u64()),
        Felt::new(0),
        Felt::new(0),
    ]);
    let id_to_name_value = ctx.naming.storage().get_map_item(4, user_key)?;
    
    // Should contain the name
    assert_eq!(id_to_name_value.get(0).unwrap().as_int(), name[0].as_int());
    assert_eq!(id_to_name_value.get(1).unwrap().as_int(), name[1].as_int());

    Ok(())
}

/// Test that duplicate names are rejected
#[tokio::test]
async fn test_duplicate_name_rejected() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let name = encode_domain_as_felts("alice".to_string());

    // First registration
    let register_inputs_1 = NoteInputs::new([name[0], name[1], name[2], name[3]].to_vec())?;
    let payment_1 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets_1 = NoteAssets::new(vec![payment_1.into()])?;
    
    let register_note_1 = create_note_for_naming(
        "register_name".to_string(),
        register_inputs_1,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets_1,
    )
    .await?;
    
    // Second registration (same name, different user)
    let register_inputs_2 = NoteInputs::new([name[0], name[1], name[2], name[3]].to_vec())?;
    let payment_2 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets_2 = NoteAssets::new(vec![payment_2.into()])?;
    
    let register_note_2 = create_note_for_naming(
        "register_name".to_string(),
        register_inputs_2,
        ctx.user_2.id(),
        ctx.naming.id(),
        register_assets_2,
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, register_note_1.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    // Execute initialization and first registration
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    execute_note(&mut chain, register_note_1.id(), &mut ctx.naming).await?;

    // Second registration should fail
    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Duplicate name registration should fail");

    Ok(())
}

/// Test that one account can only register one name
#[tokio::test]
async fn test_one_name_per_account() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let name1 = encode_domain_as_felts("alice".to_string());
    let name2 = encode_domain_as_felts("bob".to_string());
    
    // First registration
    let register_inputs_1 = NoteInputs::new([name1[0], name1[1], name1[2], name1[3]].to_vec())?;
    let payment_1 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets_1 = NoteAssets::new(vec![payment_1.into()])?;
    
    let register_note_1 = create_note_for_naming(
        "register_name".to_string(),
        register_inputs_1,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets_1,
    )
    .await?;
    
    // Second registration (different name, same user)
    let register_inputs_2 = NoteInputs::new([name2[0], name2[1], name2[2], name2[3]].to_vec())?;
    let payment_2 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets_2 = NoteAssets::new(vec![payment_2.into()])?;
    
    let register_note_2 = create_note_for_naming(
        "register_name".to_string(),
        register_inputs_2,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets_2,
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, register_note_1.clone())?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    // Execute initialization and first registration
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    execute_note(&mut chain, register_note_1.id(), &mut ctx.naming).await?;

    // Second registration should fail (same account)
    let result = execute_note(&mut chain, register_note_2.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Account should only register one name");

    Ok(())
}

/// Test that insufficient payment is rejected
#[tokio::test]
async fn test_insufficient_payment_rejected() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let name = encode_domain_as_felts("alice".to_string());
    
    // Registration with insufficient payment (price is 100, paying only 50)
    let register_inputs = NoteInputs::new([name[0], name[1], name[2], name[3]].to_vec())?;
    let insufficient_payment = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 50)?;
    let register_assets = NoteAssets::new(vec![insufficient_payment.into()])?;
    
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets,
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Registration should fail due to insufficient payment
    let result = execute_note(&mut chain, register_note.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Insufficient payment should be rejected");

    Ok(())
}

/// Test that name length validation works
#[tokio::test]
async fn test_name_length_validation() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create a name that's too long (> 20 characters)
    let long_name = "this_name_is_way_too_long_for_the_registry";
    let name = encode_domain_as_felts(long_name.to_string());
    
    let register_inputs = NoteInputs::new([name[0], name[1], name[2], name[3]].to_vec())?;
    let payment = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 100)?;
    let register_assets = NoteAssets::new(vec![payment.into()])?;
    
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.user_1.id(),
        ctx.naming.id(),
        register_assets,
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Registration should fail due to name being too long
    let result = execute_note(&mut chain, register_note.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Name exceeding max length should be rejected");

    Ok(())
}
