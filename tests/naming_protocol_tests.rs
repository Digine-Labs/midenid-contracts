mod test_utils;

use miden_client::{Felt, Word};
use miden_objects::note::NoteAssets;
use test_utils::*;

/// Test that the registry can be initialized successfully
#[tokio::test]
async fn test_initialize_registry() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Build chain and execute initialization note
    let mut chain =
        execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id()], &mut ctx.naming)
            .await?;

    // Verify initialization flag is set (slot 0 should be [1, 0, 0, 0])
    let init_flag = ctx.naming.storage().get_item(0)?;
    let init_word: Word = init_flag.into();
    assert_eq!(
        init_word.get(0).unwrap().as_int(),
        1,
        "Registry should be initialized"
    );

    // Verify owner is set (slot 1)
    let owner_slot = ctx.naming.storage().get_item(1)?;
    let owner_word: Word = owner_slot.into();
    let stored_prefix = owner_word.get(1).unwrap().as_int();
    let stored_suffix = owner_word.get(0).unwrap().as_int();

    assert_eq!(stored_prefix, ctx.owner.id().prefix().as_u64());
    assert_eq!(stored_suffix, ctx.owner.id().suffix().as_int());

    // Verify payment token is set (slot 2)
    let token_slot = ctx.naming.storage().get_item(2)?;
    let token_word: Word = token_slot.into();
    let token_prefix = token_word.get(1).unwrap().as_int();
    let token_suffix = token_word.get(0).unwrap().as_int();

    assert_eq!(
        token_prefix,
        ctx.fungible_asset.faucet_id().prefix().as_u64()
    );
    assert_eq!(
        token_suffix,
        ctx.fungible_asset.faucet_id().suffix().as_int()
    );

    // Verify price is set (slot 5)
    let price_slot = ctx.naming.storage().get_item(5)?;
    let price_word: Word = price_slot.into();
    assert_eq!(
        price_word.get(0).unwrap().as_int(),
        100,
        "Price should be 100"
    );

    Ok(())
}

/// Test that double initialization fails
#[tokio::test]
async fn test_double_initialization_fails() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Build chain and execute first initialization
    let mut chain =
        execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id()], &mut ctx.naming)
            .await?;

    // Try to initialize again - create a second init note
    let second_init_inputs = miden_objects::note::NoteInputs::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().into()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(200), // different price
        ]
        .to_vec(),
    )?;

    let second_init_note = create_note_for_naming(
        "init".to_string(),
        second_init_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    // This should fail
    let result = execute_note(&mut chain, second_init_note.id(), &mut ctx.naming).await;

    assert!(result.is_err(), "Double initialization should fail");

    Ok(())
}

/// Test reading storage slots
#[tokio::test]
async fn test_storage_layout() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Build chain and execute initialization
    let mut chain =
        execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id()], &mut ctx.naming)
            .await?;

    // Verify all storage slots are accessible
    for slot_idx in 0..=5 {
        let slot = ctx.naming.storage().get_item(slot_idx);
        assert!(slot.is_ok(), "Slot {} should be accessible", slot_idx);
    }

    // Verify map slots (3 and 4) are accessible
    // The fact that we can get the items without error means they're properly initialized
    let _name_to_id_slot = ctx.naming.storage().get_item(3)?;
    let _id_to_name_slot = ctx.naming.storage().get_item(4)?;

    Ok(())
}
