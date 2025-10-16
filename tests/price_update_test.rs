use miden_client::{
    ClientError,
    transaction::{PaymentNoteDescription, TransactionRequestBuilder},
};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod helpers;
use helpers::{
    RegistryTestHelper, create_basic_wallet, create_faucet, deploy_registry,
    fetch_registry_account, get_price, initialize_registry, update_registry_price,
};

#[tokio::test]
async fn update_price_with_note() -> Result<(), ClientError> {
    // Setup: Deploy contract and initialize registry with price of 100
    let mut helper = RegistryTestHelper::new().await?;
    helper.client.sync_state().await?;

    let owner_account =
        create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Owner").await?;
    let faucet_account = create_faucet(
        &mut helper.client,
        helper.keystore.clone(),
        "REG",
        8,
        1_000_000,
    )
    .await?;
    let registry = deploy_registry(&mut helper.client).await?;

    initialize_registry(
        &mut helper.client,
        registry.id(),
        &owner_account,
        Some(&faucet_account),
        100,
    )
    .await?;

    // Verify initial price is 100
    let initial_state = fetch_registry_account(&mut helper.client, registry.id())
        .await?
        .unwrap();
    let initial_price = get_price(&initial_state);
    assert_eq!(initial_price, 100, "Initial price should be 100");

    // Update price to 200
    update_registry_price(&mut helper.client, registry.id(), &owner_account, 200).await?;

    // Verify price is now 200
    let updated_state = fetch_registry_account(&mut helper.client, registry.id())
        .await?
        .unwrap();
    let updated_price = get_price(&updated_state);
    assert_eq!(updated_price, 200, "Updated price should be 200");

    Ok(())
}
