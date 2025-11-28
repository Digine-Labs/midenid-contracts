mod test_utils;

use std::any::Any;

use miden_client::{asset::FungibleAsset, note::{NoteAssets, NoteExecutionHint, NoteInputs, NoteTag, NoteType}, transaction::OutputNote};
use miden_crypto::{Felt, Word, rand::RpoRandomCoin};
use miden_lib::note::create_p2id_note;
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, unsafe_encode_domain};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use test_utils::init_naming;

use crate::test_utils::{add_note_to_builder, create_note_for_naming, create_p2id_note_exact, execute_note, execute_notes_and_build_chain};

#[tokio::test]
#[ignore = "not implemented"]
async fn test_claim_protocol_revenue() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;
    println!("\nOwner prefix: {}, suffix: {}", ctx.owner.id().prefix().to_string(), ctx.owner.id().suffix().to_string());
    println!("Naming prefix: {}, suffix: {}", ctx.naming.id().prefix().to_string(), ctx.naming.id().suffix().to_string());
    // Register domain to increase protocol revenue
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
    let register_note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset.clone()).await?;
    
    add_note_to_builder(&mut ctx.builder, register_note.clone())?;

    let p2id_note = create_p2id_note_exact(ctx.naming.id(), ctx.owner.id(), vec![cost.into()], NoteType::Public, Felt::new(27), Word::default())?;
    let p2id_recipient = p2id_note.recipient().digest();
    
    let withdraw_note_inputs = NoteInputs::new([
        p2id_recipient[0],
        p2id_recipient[1],
        p2id_recipient[2],
        p2id_recipient[3],
        NoteExecutionHint::Always.into(),
        NoteType::Public.into(),
        Felt::new(27),
        NoteTag::from_account_id(ctx.naming.id()).into(),
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    let withdraw_note = create_note_for_naming("claim_protocol_revenue".to_string(), withdraw_note_inputs.clone(), ctx.owner.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    add_note_to_builder(&mut ctx.builder, withdraw_note.clone())?;
    
    let mut chain = execute_notes_and_build_chain(ctx.builder, &[ctx.initialize_note.id(), ctx.set_prices_note.id(), register_note.id()], &mut ctx.naming).await?;

    execute_note(&mut chain, withdraw_note.id(), &mut ctx.naming).await?;
    // check balances
    Ok(())
}