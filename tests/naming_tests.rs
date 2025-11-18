mod test_utils;

use miden_client::{asset::FungibleAsset, note::{NoteAssets, NoteInputs}};
use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{encode_domain, encode_domain_as_felts};
use test_utils::init_naming;

use crate::test_utils::{create_note_for_naming, execute_note, get_test_prices};

#[tokio::test]
async fn test_naming_initialize() -> anyhow::Result<()> {
    let ctx = init_naming().await?;

    let init_slot = ctx.naming.storage().get_item(0)?;
    let owner_slot = ctx.naming.storage().get_item(1)?;
    let one_year_slot = ctx.naming.storage().get_item(13)?;

    assert_eq!(init_slot.get(0).unwrap().as_int(), 1);
    assert_eq!(owner_slot.get(1).unwrap().as_int(), ctx.owner.id().prefix().as_u64());
    assert_eq!(owner_slot.get(0).unwrap().as_int(), ctx.owner.id().suffix().as_int());
    assert_eq!(one_year_slot.get(0).unwrap().as_int(), 500);

    // Assert prices
    let mock_prices = get_test_prices();
    for i in 1..=5 { 
        let price_slot = ctx.naming.storage()
            .get_map_item(2, 
                Word::new([
                        Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()),
                        ctx.fungible_asset.faucet_id().prefix().as_felt(),
                        Felt::new(i as u64),
                        Felt::new(0)
                    ]))?;
        assert_eq!(price_slot.get(0).unwrap().as_int(), mock_prices[i as usize].as_int());
    }

    
    Ok(())
}

#[tokio::test]
async fn test_naming_register() -> anyhow::Result<()> {
    let mut ctx = init_naming().await?;

    let domain = encode_domain_as_felts("test".to_string());
    let domain_word = encode_domain("test".to_string());
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

    let domain_owner_slot = updated_account.storage().get_map_item(5, domain_word)?;
    let domain_expiry_slot = updated_account.storage().get_map_item(12, domain_word)?;
    let domain_to_id = updated_account.storage().get_map_item(4, domain_word)?;
    let id_to_domain = updated_account.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;


    assert_eq!(domain_owner_slot.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int());
    assert_eq!(domain_owner_slot.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());

    assert!(domain_expiry_slot.get(0).unwrap().as_int() >= (1700000000 + ctx.one_year).into());

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), 0); // Domain must be clean after register
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(0).unwrap().as_int(), 0);
    assert_eq!(id_to_domain.get(1).unwrap().as_int(), 0);
    
    // Protocol values

    let total_revenue_slot = updated_account.storage().get_map_item(10, Word::new([Felt::new(ctx.fungible_asset.faucet_id().suffix().as_int()), Felt::new(ctx.fungible_asset.faucet_id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;
    assert_eq!(total_revenue_slot.get(0).unwrap().as_int(), 555);

    let total_domain_count = updated_account.storage().get_item(9)?;
    assert_eq!(total_domain_count.get(0).unwrap().as_int(), 1);
    
    // Activate domain

    let activate_note = create_note_for_naming("activate_domain".to_string(), NoteInputs::new(domain_word.to_vec())?, ctx.registrar_1.id(), ctx.naming.id(), NoteAssets::new(vec![])?).await?;
    let updated_account = execute_note(&mut ctx.chain, activate_note, updated_account.clone()).await?; // Use always updated account as target

    let domain_to_id = updated_account.storage().get_map_item(4, domain_word)?;
    let id_to_domain = updated_account.storage().get_map_item(3, Word::new([Felt::new(ctx.registrar_1.id().suffix().as_int()), Felt::new(ctx.registrar_1.id().prefix().as_u64()), Felt::new(0), Felt::new(0)]))?;

    assert_eq!(domain_to_id.get(0).unwrap().as_int(), ctx.registrar_1.id().suffix().as_int()); // Now domain mapping must be matched
    assert_eq!(domain_to_id.get(1).unwrap().as_int(), ctx.registrar_1.id().prefix().as_u64());
    assert_eq!(id_to_domain, domain_word);
    Ok(())
}