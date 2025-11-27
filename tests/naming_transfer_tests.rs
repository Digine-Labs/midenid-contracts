mod test_utils;

use miden_client::{Felt, Word};
use miden_objects::note::{NoteAssets, NoteInputs};
use test_utils::*;

/// Test that owner can update the registration price
#[tokio::test]
async fn test_owner_can_update_price() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create price update note (new price: 200)
    let new_price = 200u64;
    let update_price_inputs = NoteInputs::new(
        [
            Felt::new(new_price),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    
    let update_price_note = create_note_for_naming(
        "update_price".to_string(),
        update_price_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, update_price_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    // Verify initial price is 100
    let initial_price_slot = ctx.naming.storage().get_item(5)?;
    let initial_price_word: Word = initial_price_slot.into();
    assert_eq!(initial_price_word.get(0).unwrap().as_int(), 100);

    // Execute price update
    execute_note(&mut chain, update_price_note.id(), &mut ctx.naming).await?;

    // Verify price is updated to 200
    let updated_price_slot = ctx.naming.storage().get_item(5)?;
    let updated_price_word: Word = updated_price_slot.into();

    assert_eq!(updated_price_word.get(0).unwrap().as_int(), new_price);

    Ok(())
}

/// Test that non-owner cannot update price
#[tokio::test]
async fn test_non_owner_cannot_update_price() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create price update note from non-owner (user_1)
    let new_price = 200u64;
    let update_price_inputs = NoteInputs::new(
        [
            Felt::new(new_price),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    
    let update_price_note = create_note_for_naming(
        "update_price".to_string(),
        update_price_inputs,
        ctx.user_1.id(), // Non-owner trying to update
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, update_price_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Price update should fail (non-owner)
    let result = execute_note(&mut chain, update_price_note.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Non-owner should not be able to update price");

    Ok(())
}

/// Test that owner can transfer ownership
#[tokio::test]
async fn test_owner_can_transfer_ownership() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create ownership transfer note (transfer to user_1)
    let update_owner_inputs = NoteInputs::new(
        [
            Felt::new(ctx.user_1.id().suffix().as_int()),
            Felt::new(ctx.user_1.id().prefix().as_u64()),
        ]
        .to_vec(),
    )?;
    
    let update_owner_note = create_note_for_naming(
        "update_owner".to_string(),
        update_owner_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, update_owner_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    // Verify initial owner
    let initial_owner_slot = ctx.naming.storage().get_item(1)?;
    let initial_owner_word: Word = initial_owner_slot.into();
    assert_eq!(
        initial_owner_word.get(0).unwrap().as_int(),
        ctx.owner.id().prefix().as_u64()
    );
    assert_eq!(
        initial_owner_word.get(1).unwrap().as_int(),
        ctx.owner.id().suffix().as_int()
    );

    // Execute ownership transfer
    execute_note(&mut chain, update_owner_note.id(), &mut ctx.naming).await?;

    // Verify new owner
    let updated_owner_slot = ctx.naming.storage().get_item(1)?;
    let updated_owner_word: Word = updated_owner_slot.into();
    assert_eq!(
        updated_owner_word.get(0).unwrap().as_int(),
        ctx.user_1.id().prefix().as_u64()
    );
    assert_eq!(
        updated_owner_word.get(1).unwrap().as_int(),
        ctx.user_1.id().suffix().as_int()
    );

    Ok(())
}

/// Test that non-owner cannot transfer ownership
#[tokio::test]
async fn test_non_owner_cannot_transfer_ownership() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create ownership transfer note from non-owner (user_1 trying to transfer to user_2)
    let update_owner_inputs = NoteInputs::new(
        [
            Felt::new(ctx.user_2.id().suffix().as_int()),
            Felt::new(ctx.user_2.id().prefix().as_u64()),
        ]
        .to_vec(),
    )?;
    
    let update_owner_note = create_note_for_naming(
        "update_owner".to_string(),
        update_owner_inputs,
        ctx.user_1.id(), // Non-owner trying to transfer
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, update_owner_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Ownership transfer should fail (non-owner)
    let result = execute_note(&mut chain, update_owner_note.id(), &mut ctx.naming).await;
    assert!(result.is_err(), "Non-owner should not be able to transfer ownership");

    Ok(())
}

/// Test that new owner can update price after ownership transfer
#[tokio::test]
async fn test_new_owner_can_update_price() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create ownership transfer note
    let update_owner_inputs = NoteInputs::new(
        [
            Felt::new(ctx.user_1.id().suffix().as_int()),
            Felt::new(ctx.user_1.id().prefix().as_u64()),
        ]
        .to_vec(),
    )?;
    
    let update_owner_note = create_note_for_naming(
        "update_owner".to_string(),
        update_owner_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    // Create price update note from new owner (user_1)
    let new_price = 300u64;
    let update_price_inputs = NoteInputs::new(
        [
            Felt::new(new_price),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    
    let update_price_note = create_note_for_naming(
        "update_price".to_string(),
        update_price_inputs,
        ctx.user_1.id(), // New owner
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;
    
    add_note_to_builder(&mut ctx.builder, update_owner_note.clone())?;
    add_note_to_builder(&mut ctx.builder, update_price_note.clone())?;

    // Execute initialization
    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id()],
        &mut ctx.naming,
    )
    .await?;
    
    // Transfer ownership
    execute_note(&mut chain, update_owner_note.id(), &mut ctx.naming).await?;
    
    // New owner should be able to update price
    execute_note(&mut chain, update_price_note.id(), &mut ctx.naming).await?;

    // Verify price is updated
    let updated_price_slot = ctx.naming.storage().get_item(5)?;
    let updated_price_word: Word = updated_price_slot.into();
    assert_eq!(updated_price_word.get(0).unwrap().as_int(), new_price);

    Ok(())
}
