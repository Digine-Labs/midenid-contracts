use miden_client::{
    ClientError,
    transaction::{PaymentNoteDescription, TransactionRequestBuilder},
};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn update_owner() -> Result<(), ClientError> {
    // Setup: Deploy contract and initialize registry with price of 100
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;
    let new_owner = helper.create_account("Owner2").await?;

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    // Verify initial owner
    let initial_state = helper.get_contract_state().await?.unwrap();
    //assert_eq!(initial_price, 100, "Initial price should be 100");

    // Update owner
    helper.update_owner(&owner_account, &new_owner).await?;

    // Verify price is now 200
    let updated_state = helper.get_contract_state().await?.unwrap();
    let (prefix, suffix) = helper.get_owner(&updated_state);
    assert_eq!(
        prefix,
        new_owner.id().prefix().as_felt().as_int(),
        "New owner prefix wrong"
    );
    assert_eq!(
        suffix,
        new_owner.id().suffix().as_int(),
        "New owner suffix wrong"
    );

    Ok(())
}
