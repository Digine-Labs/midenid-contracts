//! Regression tests for the u32 overflow that bricked registration once the registry vault
//! balance crossed `u32::MAX`. `_receive_payment` and `_get_remaining_revenue` used
//! `u32overflowing_sub` on values that are felts by protocol construction (asset amounts are
//! capped at 2^63 - 2^31), so the VM's u32 range check trapped before any arithmetic ran.
//!
//! These tests use a price table where every price already exceeds `u32::MAX`, so a single
//! registration pushes the vault past the old limit.

mod test_utils;

use miden_client::{
    asset::FungibleAsset,
    note::{NoteAssets, NoteStorage, NoteTag, NoteType},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, reverse_word};
use midenname_contracts::storage::slot_name;
use miden_protocol::transaction::RawOutputNote;

use crate::test_utils::{
    add_note_to_builder, create_note_for_naming, create_p2id_note_exact, execute_note,
    execute_note_with_expected_output, execute_notes_and_build_chain, init_naming_with,
};

/// Matches `masm/notes/set_all_prices_high.masm`.
const FOUR_LETTER_PRICE: u64 = 6_000_000_000;
const FIVE_LETTER_PRICE: u64 = 5_000_000_000;
const U32_MAX: u64 = u32::MAX as u64;

/// Enough to pay for both registrations in the multi-registration test.
const REGISTRAR_FUNDING: u64 = 20_000_000_000;

fn register_inputs(
    faucet: miden_client::account::AccountId,
    domain: &[Felt],
) -> anyhow::Result<NoteStorage> {
    Ok(NoteStorage::new(
        [
            faucet.suffix(),
            faucet.prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?)
}

fn revenue_key(faucet: miden_client::account::AccountId) -> anyhow::Result<Word> {
    Ok(Word::new([
        faucet.suffix(),
        faucet.prefix().as_felt(),
        Felt::new(0)?,
        Felt::new(0)?,
    ]))
}

/// A single registration takes the vault from 0 to above `u32::MAX`. Under the old
/// `u32overflowing_sub` this trapped on `after_bal`.
#[tokio::test]
async fn test_register_when_balance_crosses_u32_max() -> anyhow::Result<()> {
    let mut ctx = init_naming_with(REGISTRAR_FUNDING, "set_all_prices_high").await?;
    let faucet = ctx.fungible_asset.faucet_id();

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());

    let cost = FungibleAsset::new(faucet, FOUR_LETTER_PRICE)?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs(faucet, &domain)?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            register_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;

    assert!(
        FOUR_LETTER_PRICE > U32_MAX,
        "test is meaningless unless the price alone exceeds u32::MAX"
    );

    // Domain ownership was recorded, so _receive_payment accepted the payment.
    let domain_owner = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::domain_to_owner"), reverse_word(domain_word))?,
    );
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );

    // Revenue accumulated at full felt width.
    let total_revenue = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(revenue_key(faucet)?),
    )?);
    assert_eq!(
        total_revenue.get(0).unwrap().as_canonical_u64(),
        FOUR_LETTER_PRICE
    );

    Ok(())
}

/// The second registration runs with `before_bal` already above `u32::MAX`, which is the state
/// the live registry was stuck in — both operands of the old subtraction were out of range.
#[tokio::test]
async fn test_register_when_balance_already_above_u32_max() -> anyhow::Result<()> {
    let mut ctx = init_naming_with(REGISTRAR_FUNDING, "set_all_prices_high").await?;
    let faucet = ctx.fungible_asset.faucet_id();

    let first_domain = encode_domain_as_felts("test".to_string());
    let first_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs(faucet, &first_domain)?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![FungibleAsset::new(faucet, FOUR_LETTER_PRICE)?.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, first_note.clone())?;

    let second_domain = encode_domain_as_felts("abcde".to_string());
    let second_domain_word = encode_domain("abcde".to_string());
    let second_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs(faucet, &second_domain)?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![FungibleAsset::new(faucet, FIVE_LETTER_PRICE)?.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, second_note.clone())?;

    let mut chain = execute_notes_and_build_chain(
        ctx.builder,
        &[
            ctx.initialize_note.id(),
            ctx.set_prices_note.id(),
            first_note.id(),
        ],
        &mut ctx.naming,
    )
    .await?;

    // Vault now holds FOUR_LETTER_PRICE (> u32::MAX); this second payment reads it as before_bal.
    execute_note(&mut chain, second_note.id(), &mut ctx.naming).await?;

    let domain_owner = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::domain_to_owner"),
        reverse_word(second_domain_word),
    )?);
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar_1.id().suffix().as_canonical_u64()
    );

    let total_revenue = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(revenue_key(faucet)?),
    )?);
    assert_eq!(
        total_revenue.get(0).unwrap().as_canonical_u64(),
        FOUR_LETTER_PRICE + FIVE_LETTER_PRICE
    );

    Ok(())
}

/// `_get_remaining_revenue` subtracts claimed from total revenue. Both accumulate as felts, so
/// the old u32 subtraction bricked revenue claims permanently once cumulative revenue crossed
/// `u32::MAX` (unlike the vault balance, total_revenue can never be brought back down).
#[tokio::test]
async fn test_claim_protocol_revenue_above_u32_max() -> anyhow::Result<()> {
    let mut ctx = init_naming_with(REGISTRAR_FUNDING, "set_all_prices_high").await?;
    let faucet = ctx.fungible_asset.faucet_id();

    let domain = encode_domain_as_felts("test".to_string());
    let cost = FungibleAsset::new(faucet, FOUR_LETTER_PRICE)?;
    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_inputs(faucet, &domain)?,
        ctx.registrar_1.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![cost.into()])?,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    // The claim pays the whole accumulated revenue out to the owner via P2ID.
    let p2id_note = create_p2id_note_exact(
        ctx.naming.id(),
        ctx.owner.id(),
        vec![cost.into()],
        NoteType::Public,
        Word::default(),
    )?;
    let p2id_recipient = p2id_note.recipient().digest();
    let tag = NoteTag::with_account_target(ctx.owner.id());

    let claim_note_inputs = NoteStorage::new(
        [
            // RECIPIENT (0-3), passed reversed for the contract's mem_loadw_be.
            p2id_recipient[3],
            p2id_recipient[2],
            p2id_recipient[1],
            p2id_recipient[0],
            // NOTE_DETAILS (4-7): note_type, tag, padding, padding
            NoteType::Public.into(),
            tag.into(),
            Felt::new(0)?,
            Felt::new(0)?,
            // TOKEN (8-11)
            faucet.suffix(),
            faucet.prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
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

    let key = revenue_key(faucet)?;
    let total_revenue = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::total_revenue"), reverse_word(key))?,
    );
    let claimed_revenue = reverse_word(
        ctx.naming
            .storage()
            .get_map_item(&slot_name("naming::claimed_revenue"), reverse_word(key))?,
    );
    assert_eq!(
        total_revenue.get(0).unwrap().as_canonical_u64(),
        FOUR_LETTER_PRICE
    );
    assert_eq!(
        claimed_revenue.get(0).unwrap().as_canonical_u64(),
        FOUR_LETTER_PRICE
    );

    Ok(())
}
