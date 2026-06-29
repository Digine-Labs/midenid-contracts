mod test_utils;

use miden_client::{
    asset::FungibleAsset,
    note::{NoteAssets, NoteStorage, NoteTag, NoteType},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain_as_felts, reverse_word};
use midenname_contracts::storage::slot_name;
use test_utils::init_naming;

use crate::test_utils::{
    add_note_to_builder, create_note_for_naming, create_note_for_naming_with_custom_serial_num,
    create_p2id_note_exact, execute_note, execute_note_with_expected_output,
    execute_notes_and_build_chain,
};
use miden_protocol::transaction::RawOutputNote;

#[tokio::test]
async fn test_double_init_fails() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Create a second init note with custom serial to avoid NoteId collision
    let init_inputs = NoteStorage::new(
        [
            ctx.owner.id().suffix(),
            ctx.owner.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;
    let second_init_note = create_note_for_naming_with_custom_serial_num(
        "initialize_naming".to_string(),
        init_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
        Word::new([Felt::new(99).unwrap(), Felt::new(0).unwrap(), Felt::new(0).unwrap(), Felt::new(0).unwrap()]),
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, second_init_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Second init should fail
    let result = execute_note(&mut chain, second_init_note.id(), &mut ctx.naming).await;
    assert!(
        result.is_err(),
        "Expected double initialization to fail, but it succeeded"
    );

    // Init flag is still 1
    let init_flag = ctx
        .naming
        .storage()
        .get_item(&slot_name("naming::init_flag"))?;
    assert_eq!(init_flag.get(3).unwrap().as_canonical_u64(), 1);

    Ok(())
}

#[tokio::test]
async fn test_prices_updated_with_testnet_script() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Send testnet prices (different from default) — use custom serial_num to avoid collision
    let price_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
        ]
        .to_vec(),
    )?;
    let testnet_prices_note = create_note_for_naming_with_custom_serial_num(
        "set_all_prices_testnet".to_string(),
        price_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
        Word::new([Felt::new(10).unwrap(), Felt::new(0).unwrap(), Felt::new(0).unwrap(), Felt::new(0).unwrap()]),
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, testnet_prices_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[ctx.initialize_note.id(), ctx.set_prices_note.id()],
        &mut ctx.naming,
    )
    .await?;

    // Verify original prices (4-letter = 555)
    let price_key = Word::new([
        ctx.fungible_asset.faucet_id().suffix(),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(4).unwrap(),
        Felt::new(0).unwrap(),
    ]);
    let price_before = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::prices"), reverse_word(price_key))?,
    );
    assert_eq!(price_before.get(0).unwrap().as_canonical_u64(), 555);

    // Apply testnet prices
    execute_note(&mut chain, testnet_prices_note.id(), &mut ctx.naming).await?;

    // Verify testnet prices: 4-letter = 55_000_000
    let price_after = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::prices"), reverse_word(price_key))?,
    );
    assert_eq!(price_after.get(0).unwrap().as_canonical_u64(), 55_000_000);

    // Verify other testnet prices
    let price_1 = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::prices"),
        reverse_word(Word::new([
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(1).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(price_1.get(0).unwrap().as_canonical_u64(), 375_000_000);

    let price_5 = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::prices"),
        reverse_word(Word::new([
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(5).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(price_5.get(0).unwrap().as_canonical_u64(), 20_000_000);

    Ok(())
}

#[tokio::test]
async fn test_revenue_accumulates_across_registrations() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Register "test" (4 letters, cost=555) as registrar_1
    let domain1 = encode_domain_as_felts("test".to_string());
    let register1_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain1[0],
            domain1[1],
            domain1[2],
            domain1[3],
        ]
        .to_vec(),
    )?;
    let cost1 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register1_note = create_note_for_naming(
        "register_name".to_string(),
        register1_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost1.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register1_note.clone())?;

    // Register "alpha" (5 letters, cost=123) as registrar_2
    let domain2 = encode_domain_as_felts("alpha".to_string());
    let register2_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain2[0],
            domain2[1],
            domain2[2],
            domain2[3],
        ]
        .to_vec(),
    )?;
    let cost2 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123)?;
    let register2_note = create_note_for_naming(
        "register_name".to_string(),
        register2_inputs,
        ctx.registrar_2.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost2.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register2_note.clone())?;

    // Register "hey" (3 letters, cost=789) as registrar_3
    let domain3 = encode_domain_as_felts("hey".to_string());
    let register3_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain3[0],
            domain3[1],
            domain3[2],
            domain3[3],
        ]
        .to_vec(),
    )?;
    let cost3 = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 789)?;
    let register3_note = create_note_for_naming(
        "register_name".to_string(),
        register3_inputs,
        ctx.registrar_3.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost3.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register3_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            register1_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;
    execute_note(&mut chain, register2_note.id(), &mut ctx.naming).await?;
    execute_note(&mut chain, register3_note.id(), &mut ctx.naming).await?;

    // Assert: total_revenue = 555 + 123 + 789 = 1467
    let revenue_key = Word::new([
        ctx.fungible_asset.faucet_id().suffix(),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0).unwrap(),
        Felt::new(0).unwrap(),
    ]);
    let total_revenue = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::total_revenue"), reverse_word(revenue_key))?,
    );
    assert_eq!(
        total_revenue.get(0).unwrap().as_canonical_u64(),
        555 + 123 + 789
    );

    // Assert: domain_count = 3
    let domain_count = reverse_word(
        ctx.naming
            .storage()
            .get_item(&slot_name("naming::domain_count"))?,
    );
    assert_eq!(domain_count.get(0).unwrap().as_canonical_u64(), 3);

    Ok(())
}

#[tokio::test]
async fn test_claim_protocol_revenue() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Register "test" (4 letters, cost=555) to generate revenue
    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
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
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Build P2ID recipient targeting the owner
    let p2id_note = create_p2id_note_exact(
        ctx.naming.id(),
        ctx.owner.id(),
        vec![cost.into()],
        NoteType::Public,
        Word::default(),
    )?;
    let p2id_recipient = p2id_note.recipient().digest();
    let tag = NoteTag::with_account_target(ctx.owner.id());

    // Note inputs: RECIPIENT (pos 0-3), NOTE_DETAILS (pos 4-7), TOKEN (pos 8-11)
    // After mem_loadw_be, NOTE_DETAILS = [pos7, pos6, pos5, pos4]
    // Contract drops bottom 2, leaving [pos7, pos6] = [tag, note_type]
    let claim_note_inputs = NoteStorage::new(
        [
            // RECIPIENT (positions 0-3): passed reversed so the contract's mem_loadw_be
            // (which reverses the word in 0.15) reconstructs the correct recipient digest.
            p2id_recipient[3],
            p2id_recipient[2],
            p2id_recipient[1],
            p2id_recipient[0],
            // NOTE_DETAILS (positions 4-7): note_type, tag, padding, padding
            // After mem_loadw_be: [pos7, pos6, pos5, pos4] = [0, 0, tag, note_type]
            // After drop drop: [tag, note_type] which is what output_note::create needs
            NoteType::Public.into(),
            tag.into(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            // TOKEN (positions 8-11): suffix, prefix, 0, 0
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;
    let claim_note = create_note_for_naming(
        "claim_protocol_revenue".to_string(),
        claim_note_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, claim_note.clone())?;

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
    execute_note_with_expected_output(
        &mut chain,
        claim_note.id(),
        &mut ctx.naming,
        vec![RawOutputNote::Full(p2id_note)],
    )
    .await?;

    // Assert: claimed_revenue = total_revenue = 555
    let revenue_key = Word::new([
        ctx.fungible_asset.faucet_id().suffix(),
        ctx.fungible_asset.faucet_id().prefix().as_felt(),
        Felt::new(0).unwrap(),
        Felt::new(0).unwrap(),
    ]);
    let total_revenue = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::total_revenue"), reverse_word(revenue_key))?,
    );
    assert_eq!(total_revenue.get(0).unwrap().as_canonical_u64(), 555);
    let claimed_revenue = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::claimed_revenue"), reverse_word(revenue_key))?,
    );
    assert_eq!(claimed_revenue.get(0).unwrap().as_canonical_u64(), 555);

    Ok(())
}

#[tokio::test]
async fn test_withdraw_assets() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    // Register "test" (4 letters, cost=555) to add assets to the naming vault
    let domain = encode_domain_as_felts("test".to_string());
    let register_note_inputs = NoteStorage::new(
        [
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
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
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // Build P2ID recipient targeting the owner for the full balance withdrawal
    let p2id_note = create_p2id_note_exact(
        ctx.naming.id(),
        ctx.owner.id(),
        vec![cost.into()],
        NoteType::Public,
        Word::default(),
    )?;
    let p2id_recipient = p2id_note.recipient().digest();
    let tag = NoteTag::with_account_target(ctx.owner.id());

    // Note inputs: RECIPIENT (pos 0-3), NOTE_DETAILS (pos 4-7), TOKEN (pos 8-11)
    let withdraw_note_inputs = NoteStorage::new(
        [
            // RECIPIENT (positions 0-3): passed reversed so the contract's mem_loadw_be
            // (which reverses the word in 0.15) reconstructs the correct recipient digest.
            p2id_recipient[3],
            p2id_recipient[2],
            p2id_recipient[1],
            p2id_recipient[0],
            // NOTE_DETAILS (positions 4-7): note_type, tag, padding, padding
            NoteType::Public.into(),
            tag.into(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            // TOKEN (positions 8-11): suffix, prefix, 0, 0
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;
    let withdraw_note = create_note_for_naming(
        "withdraw_assets".to_string(),
        withdraw_note_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, withdraw_note.clone())?;

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
    execute_note_with_expected_output(
        &mut chain,
        withdraw_note.id(),
        &mut ctx.naming,
        vec![RawOutputNote::Full(p2id_note)],
    )
    .await?;

    Ok(())
}
