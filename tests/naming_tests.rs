mod test_utils;

use miden_crypto::{Felt, Word};
use test_utils::init_naming;

use crate::test_utils::get_test_prices;

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
    Ok(())
}