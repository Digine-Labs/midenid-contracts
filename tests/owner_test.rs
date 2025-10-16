use miden_client::{
    ClientError,
    transaction::{PaymentNoteDescription, TransactionRequestBuilder},
};
use miden_objects::{asset::FungibleAsset, note::NoteType};

mod helpers;
use helpers::{
    RegistryTestHelper, create_basic_wallet, create_faucet, deploy_registry,
    fetch_registry_account, get_owner, initialize_registry, transfer_registry_ownership,
};

#[tokio::test]
async fn update_owner() -> Result<(), ClientError> {
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
    let new_owner =
        create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Owner2").await?;
    let registry = deploy_registry(&mut helper.client).await?;

    initialize_registry(
        &mut helper.client,
        registry.id(),
        &owner_account,
        Some(&faucet_account),
        100,
    )
    .await?;

    // Verify initial owner
    let initial_state = fetch_registry_account(&mut helper.client, registry.id())
        .await?
        .unwrap();
    //assert_eq!(initial_price, 100, "Initial price should be 100");

    // Update owner
    transfer_registry_ownership(
        &mut helper.client,
        registry.id(),
        &owner_account,
        &new_owner,
    )
    .await?;

    // Verify owner has changed
    let updated_state = fetch_registry_account(&mut helper.client, registry.id())
        .await?
        .unwrap();
    let (prefix, suffix) = get_owner(&updated_state);
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
