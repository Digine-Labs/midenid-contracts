mod test_utils;

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
    let mut rng = RpoRandomCoin::new(Word::new([Felt::new(0), Felt::new(2), Felt::new(3), Felt::new(112)]));
    let p2id_note = create_p2id_note(updated_account.id(), ctx.owner.id(), vec![], NoteType::Public, Felt::new(0), &mut rng)?;
    let p2id_recipient = p2id_note.recipient().digest().to_vec();
    let claim_inputs = NoteInputs::new([
        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
        Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()),
        Felt::new(0),
        Felt::new(0),
        p2id_recipient[0],
        p2id_recipient[1],
        p2id_recipient[2],
        p2id_recipient[3],
    ].to_vec())?;
    let claim_note = create_note_for_naming("claim_protocol_revenue".to_string(), claim_inputs, ctx.owner.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    let updated_naming_account = execute_note(&mut ctx.chain, claim_note, updated_account).await?;

    // check balances
    Ok(())
}