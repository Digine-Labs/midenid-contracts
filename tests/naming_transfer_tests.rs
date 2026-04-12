mod test_utils;

use miden_client::{
    asset::FungibleAsset,
    note::{NoteAssets, NoteStorage},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts};
use midenname_contracts::storage::slot_name;
use test_utils::init_naming;

use crate::test_utils::{
    add_note_to_builder, create_note_for_naming, create_note_for_naming_with_custom_serial_num,
    execute_note, execute_notes_and_build_chain,
};

// ─── Domain transfer tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_transfer_domain_happy_path() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Transfer "test" from registrar_1 to registrar_2
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

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
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;

    // Assert: domain_to_owner = registrar_2
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: domain_to_account = 0 (cleared by transfer)
    let domain_to_account = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(domain_to_account.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(domain_to_account.get(1).unwrap().as_canonical_u64(), 0);

    // Assert: account_to_domain[registrar_1] = 0 (cleared)
    let r1_to_domain = ctx.naming.storage().get_map_item(
        &slot_name("naming::account_to_domain"),
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(r1_to_domain.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(r1_to_domain.get(1).unwrap().as_canonical_u64(), 0);

    Ok(())
}

#[tokio::test]
async fn test_transfer_domain_then_reactivate_by_new_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Transfer "test" from registrar_1 to registrar_2
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    // Activate domain as new owner (registrar_2)
    let activate_note = create_note_for_naming(
        "activate_domain".to_string(),
        NoteStorage::new(domain_word.to_vec())?,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
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
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await?;

    // Assert: domain_to_owner = registrar_2
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: domain_to_account = registrar_2
    let domain_to_account = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(
        domain_to_account.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_to_account.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: account_to_domain[registrar_2] = domain_word
    let r2_to_domain = ctx.naming.storage().get_map_item(
        &slot_name("naming::account_to_domain"),
        Word::new([
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(r2_to_domain, domain_word);

    Ok(())
}

#[tokio::test]
async fn test_transfer_domain_by_non_owner_fails() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // registrar_2 tries to transfer "test" (not the owner)
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_3.id().suffix().as_canonical_u64()),
            ctx.registrar_3.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

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

    let result = execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await;
    assert!(
        result.is_err(),
        "Expected transfer by non-owner to fail, but it succeeded"
    );

    // Owner is still registrar_1
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().prefix().as_felt().as_canonical_u64()
    );

    Ok(())
}

#[tokio::test]
async fn test_transfer_nonexistent_domain_fails() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("ghost".to_string());

    // Try to transfer unregistered domain
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    let result = execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await;
    assert!(
        result.is_err(),
        "Expected transfer of nonexistent domain to fail, but it succeeded"
    );

    Ok(())
}

#[tokio::test]
async fn test_transfer_domain_preserves_other_domains() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let alpha = encode_domain_as_felts("alpha".to_string());
    let alpha_word = encode_domain("alpha".to_string());
    let beta = encode_domain_as_felts("beta".to_string());
    let beta_word = encode_domain("beta".to_string());

    // Register "alpha" (5 letters, cost=123) as registrar_1
    let register_alpha_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            alpha[0],
            alpha[1],
            alpha[2],
            alpha[3],
        ]
        .to_vec(),
    )?;
    let cost_alpha = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123)?;
    let register_alpha_note = create_note_for_naming(
        "register_name".to_string(),
        register_alpha_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost_alpha.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_alpha_note.clone())?;

    // Register "beta" (4 letters, cost=555) as registrar_1
    let register_beta_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            beta[0],
            beta[1],
            beta[2],
            beta[3],
        ]
        .to_vec(),
    )?;
    let cost_beta = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_beta_note = create_note_for_naming(
        "register_name".to_string(),
        register_beta_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost_beta.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_beta_note.clone())?;

    // Transfer only "alpha" to registrar_2
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            alpha[0],
            alpha[1],
            alpha[2],
            alpha[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            register_alpha_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, register_beta_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;

    // Assert: "alpha" owner = registrar_2
    let alpha_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), alpha_word)?;
    assert_eq!(
        alpha_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        alpha_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: "beta" owner still = registrar_1
    let beta_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), beta_word)?;
    assert_eq!(
        beta_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        beta_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: domain_count = 2
    let domain_count = ctx
        .naming
        .storage()
        .get_item(&slot_name("naming::domain_count"))?;
    assert_eq!(domain_count.get(0).unwrap().as_canonical_u64(), 2);

    Ok(())
}

// ─── Registry ownership transfer tests ───────────────────────────────────────

#[tokio::test]
async fn test_transfer_ownership_happy_path() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Owner transfers registry ownership to registrar_1
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_ownership".to_string(),
        transfer_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;

    // Assert: owner = registrar_1
    let owner_slot = ctx.naming.storage().get_item(&slot_name("naming::owner"))?;
    assert_eq!(
        owner_slot.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        owner_slot.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().prefix().as_felt().as_canonical_u64()
    );

    Ok(())
}

#[tokio::test]
async fn test_transfer_ownership_by_non_owner_fails() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // registrar_1 (not owner) tries to transfer ownership
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_ownership".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    let result = execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await;
    assert!(
        result.is_err(),
        "Expected ownership transfer by non-owner to fail, but it succeeded"
    );

    // Owner is still the original owner
    let owner_slot = ctx.naming.storage().get_item(&slot_name("naming::owner"))?;
    assert_eq!(
        owner_slot.get(0).unwrap().as_canonical_u64(),
        ctx.owner.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        owner_slot.get(1).unwrap().as_canonical_u64(),
        ctx.owner.id().prefix().as_felt().as_canonical_u64()
    );

    Ok(())
}

#[tokio::test]
async fn test_new_owner_can_set_prices_after_transfer() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Owner transfers registry ownership to registrar_1
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_ownership".to_string(),
        transfer_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    // New owner (registrar_1) sends set_all_prices — use custom serial_num to avoid NoteId collision
    let price_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
        ]
        .to_vec(),
    )?;
    let set_prices_note = create_note_for_naming_with_custom_serial_num(
        "set_all_prices".to_string(),
        price_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
        Word::new([Felt::new(1), Felt::new(0), Felt::new(0), Felt::new(0)]),
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, set_prices_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, set_prices_note.id(), &mut ctx.naming).await?;

    // Assert: owner = registrar_1
    let owner_slot = ctx.naming.storage().get_item(&slot_name("naming::owner"))?;
    assert_eq!(
        owner_slot.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        owner_slot.get(1).unwrap().as_canonical_u64(),
        u64::from(ctx.registrar_1.id().prefix())
    );

    // Assert: 4-letter price = 555
    let price_slot = ctx.naming.storage().get_map_item(
        &slot_name("naming::prices"),
        Word::new([
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(4),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(price_slot.get(0).unwrap().as_canonical_u64(), 555);

    Ok(())
}

#[tokio::test]
async fn test_old_owner_loses_admin_after_transfer() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Owner transfers registry ownership to registrar_1
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_ownership".to_string(),
        transfer_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

    // Old owner tries to set prices after transfer — use custom serial_num to avoid NoteId collision
    let price_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
        ]
        .to_vec(),
    )?;
    let set_prices_note = create_note_for_naming_with_custom_serial_num(
        "set_all_prices".to_string(),
        price_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
        Word::new([Felt::new(2), Felt::new(0), Felt::new(0), Felt::new(0)]),
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, set_prices_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;

    let result = execute_note(&mut chain, set_prices_note.id(), &mut ctx.naming).await;
    assert!(
        result.is_err(),
        "Expected old owner to lose admin after transfer, but set_prices succeeded"
    );

    // Owner is still registrar_1
    let owner_slot = ctx.naming.storage().get_item(&slot_name("naming::owner"))?;
    assert_eq!(
        owner_slot.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        owner_slot.get(1).unwrap().as_canonical_u64(),
        u64::from(ctx.registrar_1.id().prefix())
    );

    Ok(())
}

// ─── Activate then transfer tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_activate_then_transfer_clears_activation() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Activate domain as registrar_1
    let activate_note = create_note_for_naming(
        "activate_domain".to_string(),
        NoteStorage::new(domain_word.to_vec())?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, activate_note.clone())?;

    // Transfer domain to registrar_2
    let transfer_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer_note.clone())?;

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
    execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await?;

    // Verify activation worked before transfer
    let domain_to_account_before = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(
        domain_to_account_before.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );

    execute_note(&mut chain, transfer_note.id(), &mut ctx.naming).await?;

    // Assert: domain_to_owner = registrar_2
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_2.id().prefix().as_felt().as_canonical_u64()
    );

    // Assert: domain_to_account = 0 (activation cleared by transfer)
    let domain_to_account = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(domain_to_account.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(domain_to_account.get(1).unwrap().as_canonical_u64(), 0);

    // Assert: account_to_domain[registrar_1] = 0 (cleared)
    let r1_to_domain = ctx.naming.storage().get_map_item(
        &slot_name("naming::account_to_domain"),
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(r1_to_domain.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(r1_to_domain.get(1).unwrap().as_canonical_u64(), 0);

    Ok(())
}

// ─── Additional domain transfer tests ────────────────────────────────────────

#[tokio::test]
async fn test_double_domain_transfer() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Transfer registrar_1 → registrar_2
    let transfer1_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer1_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer1_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer1_note.clone())?;

    // Transfer registrar_2 → registrar_3
    let transfer2_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_3.id().suffix().as_canonical_u64()),
            ctx.registrar_3.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer2_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer2_inputs,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer2_note.clone())?;

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
    execute_note(&mut chain, transfer1_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, transfer2_note.id(), &mut ctx.naming).await?;

    // Assert: final owner = registrar_3
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_3.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        u64::from(ctx.registrar_3.id().prefix())
    );

    // Assert: domain_to_account cleared
    let domain_to_account = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(domain_to_account.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(domain_to_account.get(1).unwrap().as_canonical_u64(), 0);

    // Assert: domain_count unchanged (still 1, transfers don't add domains)
    let domain_count = ctx
        .naming
        .storage()
        .get_item(&slot_name("naming::domain_count"))?;
    assert_eq!(domain_count.get(0).unwrap().as_canonical_u64(), 1);

    Ok(())
}

#[tokio::test]
async fn test_transfer_domain_back_to_original_owner() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    // Register "test" as registrar_1
    let register_inputs = NoteStorage::new(
        [
            Felt::new(ctx.fungible_asset.faucet_id().suffix().as_canonical_u64()),
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
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Transfer registrar_1 → registrar_2
    let transfer1_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_2.id().suffix().as_canonical_u64()),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer1_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer1_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer1_note.clone())?;

    // Transfer registrar_2 → registrar_1 (back)
    let transfer2_inputs = NoteStorage::new(
        [
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;
    let transfer2_note = create_note_for_naming(
        "transfer_domain".to_string(),
        transfer2_inputs,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, transfer2_note.clone())?;

    // registrar_1 re-activates domain
    let activate_note = create_note_for_naming(
        "activate_domain".to_string(),
        NoteStorage::new(domain_word.to_vec())?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
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
    execute_note(&mut chain, transfer1_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, transfer2_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, activate_note.id(), &mut ctx.naming).await?;

    // Assert: owner = registrar_1 again
    let domain_owner = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_owner"), domain_word)?;
    assert_eq!(
        domain_owner.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        u64::from(ctx.registrar_1.id().prefix())
    );

    // Assert: domain_to_account = registrar_1 (re-activated)
    let domain_to_account = ctx
        .naming
        .storage()
        .get_map_item(&slot_name("naming::domain_to_account"), domain_word)?;
    assert_eq!(
        domain_to_account.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_to_account.get(1).unwrap().as_canonical_u64(),
        u64::from(ctx.registrar_1.id().prefix())
    );

    // Assert: account_to_domain[registrar_1] = domain_word
    let r1_to_domain = ctx.naming.storage().get_map_item(
        &slot_name("naming::account_to_domain"),
        Word::new([
            Felt::new(ctx.registrar_1.id().suffix().as_canonical_u64()),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0),
            Felt::new(0),
        ]),
    )?;
    assert_eq!(r1_to_domain, domain_word);

    Ok(())
}
