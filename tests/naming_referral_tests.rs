mod test_utils;

use miden_client::{
    asset::FungibleAsset,
    note::{NoteAssets, NoteStorage},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{
    encode_domain, encode_domain_as_felts, reverse_word, unsafe_encode_domain,
};
use midenname_contracts::storage::slot_name;
use test_utils::init_naming;

use crate::test_utils::{
    add_note_to_builder, create_note_for_naming, create_note_for_naming_with_custom_serial_num,
    execute_note, execute_notes_and_build_chain, get_test_prices,
};
#[tokio::test]
#[ignore = "tbd"]
async fn test_naming_register_under_referrer() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let set_ref_rate_inputs = NoteStorage::new(
        [
            Felt::new(2000).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;

    let set_ref_rate_note = create_note_for_naming(
        "set_referrer_rate".to_string(),
        set_ref_rate_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, set_ref_rate_note.clone())?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteStorage::new(
        [
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
            Felt::new(1).unwrap(), // register length
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_with_referrer".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            set_ref_rate_note.id(),
            register_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;

    let domain_owner_slot = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::domain_to_owner"), reverse_word(domain_word))?,
    );
    let domain_expiry_slot = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::domain_expiry"), reverse_word(domain_word))?,
    );
    let domain_to_id = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::domain_to_account"), reverse_word(domain_word))?,
    );
    let id_to_domain = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::account_to_domain"),
        reverse_word(Word::new([
            ctx.registrar_1.id().suffix(),
            ctx.registrar_1.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);

    assert_eq!(
        domain_owner_slot.get(0).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );
    assert_eq!(
        domain_owner_slot.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().prefix().as_felt().as_canonical_u64()
    );

    assert!(
        domain_expiry_slot.get(0).unwrap().as_canonical_u64() >= (1700000000 + ctx.one_year).into()
    );

    assert_eq!(domain_to_id.get(0).unwrap().as_canonical_u64(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_canonical_u64(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_canonical_u64(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_canonical_u64(), 0);

    // Protocol values

    let total_revenue_slot = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(Word::new([
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(total_revenue_slot.get(0).unwrap().as_canonical_u64(), 444);

    // Referrer values

    let referrer_slot = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::ref_total_revenue"),
        reverse_word(Word::new([
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(referrer_slot.get(0).unwrap().as_canonical_u64(), 111);
    Ok(())
}

#[tokio::test]
#[ignore = "tbd"]
async fn test_naming_referrer_revenue_accumulation() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let set_ref_rate_inputs = NoteStorage::new(
        [
            Felt::new(2000).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;

    let set_ref_rate_note = create_note_for_naming(
        "set_referrer_rate".to_string(),
        set_ref_rate_inputs,
        ctx.owner.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, set_ref_rate_note.clone())?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
    let register_note_inputs = NoteStorage::new(
        [
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain[0],
            domain[1],
            domain[2],
            domain[3],
            Felt::new(1).unwrap(), // register length
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 555)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note = create_note_for_naming(
        "register_with_referrer".to_string(),
        register_note_inputs,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let domain_2 = encode_domain_as_felts("test2".to_string());
    let domain_word_2 = encode_domain("test2".to_string());
    let register_note_inputs_2 = NoteStorage::new(
        [
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            domain_2[0],
            domain_2[1],
            domain_2[2],
            domain_2[3],
            Felt::new(1).unwrap(), // register length
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ]
        .to_vec(),
    )?;

    let cost = FungibleAsset::new(ctx.fungible_asset.faucet_id(), 123)?;
    let register_asset = NoteAssets::new(vec![cost.into()])?;
    let register_note_2 = create_note_for_naming_with_custom_serial_num(
        "register_with_referrer".to_string(),
        register_note_inputs_2,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        register_asset,
        Word::new([Felt::new(1).unwrap(), Felt::new(2).unwrap(), Felt::new(3).unwrap(), Felt::new(1).unwrap()]),
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note_2.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            set_ref_rate_note.id(),
            register_note.id(),
            register_note_2.id(),
        ],
        &mut ctx.naming,
    )
    .await?;

    // Protocol values

    let total_revenue_slot = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(Word::new([
            ctx.fungible_asset.faucet_id().suffix(),
            ctx.fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(total_revenue_slot.get(0).unwrap().as_canonical_u64(), 543);

    // Referrer values

    let referrer_slot = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::ref_total_revenue"),
        reverse_word(Word::new([
            ctx.registrar_2.id().suffix(),
            ctx.registrar_2.id().prefix().as_felt(),
            Felt::new(0).unwrap(),
            Felt::new(0).unwrap(),
        ])),
    )?);
    assert_eq!(referrer_slot.get(0).unwrap().as_canonical_u64(), 135);
    Ok(())
}
