//! Runtime coverage for `naming_discount.masm`. Nothing else instantiates this contract, so
//! before these tests the only guarantee was that it assembled.
//!
//! The focus is the multi-year discount fee math, which is where the u32 arithmetic was most
//! badly wrong: a 6e9 base price times a 5000bp discount is 3e13, roughly 7000x u32::MAX, so the
//! old `u32overflowing_mul` would have trapped at any realistic price even after the opcode was
//! valid.

mod test_utils;

use miden_client::{
    account::AccountId,
    asset::FungibleAsset,
    note::{NoteAssets, NoteStorage},
};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, reverse_word};
use midenname_contracts::storage::slot_name;
use miden_testing::{Auth, MockChain};

use crate::test_utils::{
    NAMING_DISCOUNT_CONTRACT, add_note_to_builder, create_note_for_naming_with_contract,
    create_test_account_from, execute_note, execute_notes_and_build_chain,
};

/// Matches `masm/notes/set_all_prices_high.masm`.
const FOUR_LETTER_PRICE: u64 = 6_000_000_000;
const FIVE_YR_DISCOUNT_BP: u64 = 5000;
const THREE_YR_DISCOUNT_BP: u64 = 3000;
const BASIS_POINTS: u64 = 10_000;
const ONE_YEAR: u64 = 500;
const REGISTRAR_FUNDING: u64 = 200_000_000_000;

/// What `_calculate_domain_price` should charge: apply the multi-year discount to the per-year
/// price, then multiply by the number of years.
fn expected_price(base: u64, reg_len: u64) -> u64 {
    let discount_bp = if reg_len >= 5 {
        FIVE_YR_DISCOUNT_BP
    } else if reg_len >= 3 {
        THREE_YR_DISCOUNT_BP
    } else {
        0
    };
    let discounted = base - (base * discount_bp) / BASIS_POINTS;
    discounted * reg_len
}

struct DiscountCtx {
    builder: miden_testing::MockChainBuilder,
    owner: miden_client::account::Account,
    registrar: miden_client::account::Account,
    naming: miden_client::account::Account,
    faucet: AccountId,
    init_note_id: miden_client::note::NoteId,
    prices_note_id: miden_client::note::NoteId,
}

async fn init_discount() -> anyhow::Result<DiscountCtx> {
    let mut builder = MockChain::builder();
    let funding = FungibleAsset::new(
        miden_client::testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1
            .try_into()
            .unwrap(),
        REGISTRAR_FUNDING,
    )?;
    let faucet = funding.faucet_id();

    let owner = builder.add_existing_wallet(Auth::BasicAuth {
        auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
    })?;
    let registrar = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth {
            auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
        },
        vec![funding.into()],
    )?;
    let naming = create_test_account_from(NAMING_DISCOUNT_CONTRACT);
    builder.add_account(naming.clone())?;

    // initialize_naming.masm reads OWNER from inputs 0-3 and the one-year timestamp from 4-7.
    // Both words are loaded with mem_loadw_be, so the meaningful felt sits at the low index.
    let init_note = create_note_for_naming_with_contract(
        "initialize_naming".to_string(),
        NoteStorage::new(
            [
                owner.id().suffix(),
                owner.id().prefix().as_felt(),
                Felt::new(0)?,
                Felt::new(0)?,
                Felt::new(ONE_YEAR)?,
                Felt::new(0)?,
                Felt::new(0)?,
                Felt::new(0)?,
            ]
            .to_vec(),
        )?,
        owner.id(),
        naming.id(),
        NoteAssets::new(vec![])?,
        NAMING_DISCOUNT_CONTRACT,
    )
    .await?;
    add_note_to_builder(&mut builder, init_note.clone())?;

    let prices_note = create_note_for_naming_with_contract(
        "set_all_prices_high".to_string(),
        NoteStorage::new([faucet.suffix(), faucet.prefix().as_felt()].to_vec())?,
        owner.id(),
        naming.id(),
        NoteAssets::new(vec![])?,
        NAMING_DISCOUNT_CONTRACT,
    )
    .await?;
    add_note_to_builder(&mut builder, prices_note.clone())?;

    Ok(DiscountCtx {
        builder,
        owner,
        registrar,
        naming,
        faucet,
        init_note_id: init_note.id(),
        prices_note_id: prices_note.id(),
    })
}

/// register_name.masm inputs: TOKEN at 0-3, DOMAIN at 4-7, REG_LEN at 8-11.
fn register_inputs(faucet: AccountId, domain: &[Felt], reg_len: u64) -> anyhow::Result<NoteStorage> {
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
            Felt::new(reg_len)?,
            Felt::new(0)?,
            Felt::new(0)?,
            Felt::new(0)?,
        ]
        .to_vec(),
    )?)
}

async fn register_for(reg_len: u64, pay: u64) -> anyhow::Result<(DiscountCtx, bool)> {
    let mut ctx = init_discount().await?;
    let domain = encode_domain_as_felts("test".to_string());

    let note = create_note_for_naming_with_contract(
        "register_name".to_string(),
        register_inputs(ctx.faucet, &domain, reg_len)?,
        ctx.registrar.id(),
        ctx.naming.id(),
        NoteAssets::new(vec![FungibleAsset::new(ctx.faucet, pay)?.into()])?,
        NAMING_DISCOUNT_CONTRACT,
    )
    .await?;
    add_note_to_builder(&mut ctx.builder, note.clone())?;

    let builder = std::mem::replace(&mut ctx.builder, MockChain::builder());
    let mut naming = ctx.naming.clone();
    let mut chain = execute_notes_and_build_chain(
        builder,
        &[ctx.init_note_id, ctx.prices_note_id],
        &mut naming,
    )
    .await?;
    let ok = execute_note(&mut chain, note.id(), &mut naming).await.is_ok();
    ctx.naming = naming;
    Ok((ctx, ok))
}

/// 5-year registration: 50% off the per-year price, times 5 years. The intermediate
/// `6e9 * 5000` is ~7000x u32::MAX.
#[tokio::test]
async fn test_five_year_discount_price() -> anyhow::Result<()> {
    let expected = expected_price(FOUR_LETTER_PRICE, 5);
    assert_eq!(expected, 15_000_000_000);

    let (ctx, ok) = register_for(5, expected).await?;
    assert!(ok, "registration paying exactly the discounted price should succeed");

    let total_revenue = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(Word::new([
            ctx.faucet.suffix(),
            ctx.faucet.prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
        ])),
    )?);
    assert_eq!(total_revenue.get(0).unwrap().as_canonical_u64(), expected);

    let domain_owner = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::domain_to_owner"),
        reverse_word(encode_domain("test".to_string())),
    )?);
    assert_eq!(
        domain_owner.get(1).unwrap().as_canonical_u64(),
        ctx.registrar.id().suffix().as_canonical_u64()
    );
    Ok(())
}

/// 3-year registration: 30% off, times 3 years.
#[tokio::test]
async fn test_three_year_discount_price() -> anyhow::Result<()> {
    let expected = expected_price(FOUR_LETTER_PRICE, 3);
    assert_eq!(expected, 12_600_000_000);

    let (ctx, ok) = register_for(3, expected).await?;
    assert!(ok, "registration paying exactly the discounted price should succeed");

    let total_revenue = reverse_word(ctx.naming.storage().get_map_item(
        &slot_name("naming::total_revenue"),
        reverse_word(Word::new([
            ctx.faucet.suffix(),
            ctx.faucet.prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
        ])),
    )?);
    assert_eq!(total_revenue.get(0).unwrap().as_canonical_u64(), expected);
    Ok(())
}

/// 1-year registration gets no discount, so the price is the raw per-year price.
#[tokio::test]
async fn test_single_year_has_no_discount() -> anyhow::Result<()> {
    let expected = expected_price(FOUR_LETTER_PRICE, 1);
    assert_eq!(expected, FOUR_LETTER_PRICE);

    let (_ctx, ok) = register_for(1, expected).await?;
    assert!(ok, "registration paying exactly the undiscounted price should succeed");
    Ok(())
}

/// Underpaying by one unit must still be rejected — the felt subtraction in `_receive_payment`
/// must not have loosened the check.
#[tokio::test]
async fn test_underpayment_is_rejected() -> anyhow::Result<()> {
    let expected = expected_price(FOUR_LETTER_PRICE, 5);
    let (_ctx, ok) = register_for(5, expected - 1).await?;
    assert!(!ok, "paying one unit under the discounted price must fail");
    Ok(())
}
