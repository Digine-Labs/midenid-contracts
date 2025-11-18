mod test_utils;

use std::any::Any;

use miden_client::{asset::FungibleAsset, note::{NoteAssets, NoteInputs, NoteType}, transaction::OutputNote};
use miden_crypto::{Felt, Word, rand::RpoRandomCoin};
use miden_lib::note::create_p2id_note;
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts, unsafe_encode_domain};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use test_utils::init_naming;

use crate::test_utils::{create_note_for_naming, execute_note};
#[tokio::test]
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
    let note = create_note_for_naming("register_name".to_string(), register_note_inputs, ctx.registrar_1.id(), ctx.naming.id(), register_asset).await?;
    let updated_account = execute_note(&mut ctx.chain, note, ctx.naming.clone()).await?;
    
    // Protocol values

    let total_revenue_slot = updated_account.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = updated_account.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);
    
    // Withdraw
    // 1,2,3,4 => 4,3,2,1
    let p2id_note = ctx.chain.add_pending_p2id_note(ctx.naming.id(), ctx.owner.id(), &[], NoteType::Public)?;
    ctx.chain.prove_next_block()?;
    let recipient_hash = p2id_note.recipient().digest();
    let claim_inputs = NoteInputs::new([
        // TOKEN
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
        Felt::new(0),
        Felt::new(0),
        // NOTE_DETAILS
        Felt::new(p2id_note.metadata().execution_hint().into()), //exec_hint
        Felt::new(1), //note_type
        p2id_note.header().metadata().aux(), // aux
        Felt::new(p2id_note.header().metadata().tag().as_u32().into()), //tag
        // RECIPIENT
        recipient_hash[0],
        recipient_hash[1],
        recipient_hash[2],
        recipient_hash[3],
    ].to_vec())?;
    let claim_note = create_note_for_naming("claim_protocol_revenue".to_string(), claim_inputs, ctx.owner.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    let updated_naming_account = execute_note(&mut ctx.chain, claim_note, updated_account).await?;

    // check balances
    Ok(())
}