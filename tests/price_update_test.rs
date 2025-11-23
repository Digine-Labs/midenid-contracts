use miden_client::{
    ClientError,
    transaction::{PaymentNoteDescription, TransactionRequestBuilder},
};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod test_helper;
use test_helper::RegistryTestHelper;

#[tokio::test]
async fn update_price_with_note() -> Result<(), ClientError> {
    // Setup: Deploy contract and initialize registry with price of 100
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner_account = helper.create_account("Owner").await?;
    let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;

    helper
        .initialize_registry_with_faucet(&owner_account, Some(&faucet_account))
        .await?;

    // Verify initial price is 100
    let initial_state = helper.get_contract_state().await?.unwrap();
    let initial_price = helper.get_price(&initial_state);
    assert_eq!(initial_price, 100, "Initial price should be 100");

    // Update price to 200
    helper.update_price(&owner_account, 200).await?;

    // Verify price is now 200
    let updated_state = helper.get_contract_state().await?.unwrap();
    let updated_price = helper.get_price(&updated_state);
    assert_eq!(updated_price, 200, "Updated price should be 200");

    Ok(())
}
