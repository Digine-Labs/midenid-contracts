use midenid_contracts::common::{
    create_basic_account, create_public_immutable_contract, create_public_note, wait_for_note,
};

use miden_client_tools::{
    create_library, create_tx_script, delete_keystore_and_store, instantiate_client,
};

use miden_client::{
    ClientError, Word, keystore::FilesystemKeyStore, note::NoteAssets, rpc::Endpoint,
    transaction::TransactionRequestBuilder, Felt, account::{StorageSlot}
};
use miden_objects::{
    account::{AccountComponent, StorageMap, NetworkId},
    assembly::Assembler,
    assembly::DefaultSourceManager,
};
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn register_with_script() -> Result<(), ClientError> {
    delete_keystore_and_store(None).await;

    let endpoint = Endpoint::localhost();
    let mut client = instantiate_client(endpoint.clone(), None).await.unwrap();

    let sync_summary = client.sync_state().await.unwrap();
    println!("Latest block: {}", sync_summary.block_num);

    // -------------------------------------------------------------------------
    // STEP 1: Create counter smart contract
    // -------------------------------------------------------------------------
    let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id_registry.masm")).unwrap();
    // Provision storage slots so indices 10 and 11 exist as maps
    let mut storage_slots = Vec::with_capacity(12);
    for _ in 0..12 { storage_slots.push(StorageSlot::empty_value()); }
    storage_slots[10] = StorageSlot::Map(StorageMap::new()); // name -> addr
    storage_slots[11] = StorageSlot::Map(StorageMap::new()); // addr -> name

    let (registry_contract, registry_seed) =
        create_public_immutable_contract(&mut client, &registry_code, storage_slots)
            .await
            .unwrap();
    println!("contract id: {:?}", registry_contract.id().to_hex());

    client
        .add_account(&registry_contract, Some(registry_seed), false)
        .await
        .unwrap();
    println!("Contract created");
    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let script_code =
        fs::read_to_string(Path::new("./masm/scripts/registry_set_script.masm")).unwrap();

    let library_path = "external_contract::miden_id_registry_contract";

    let library = create_library(registry_code, library_path).unwrap();

    let tx_script = create_tx_script(script_code, Some(library)).unwrap();
    println!("Script prepared");

    // -------------------------------------------------------------------------
    // STEP 3: Build & Submit Transaction
    // -------------------------------------------------------------------------
    let tx_set_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .unwrap();

    let tx_result = client
        .new_transaction(registry_contract.id(), tx_set_request)
        .await
        .unwrap();

    let _ = client.submit_transaction(tx_result).await;
    println!("Tx submitted");
    // -------------------------------------------------------------------------
    // STEP 4: Validate Updated State
    // -------------------------------------------------------------------------
    sleep(Duration::from_secs(7)).await;

    delete_keystore_and_store(None).await;

    let mut client = instantiate_client(endpoint, None).await.unwrap();

    client
        .import_account_by_id(registry_contract.id())
        .await
        .unwrap();

    let new_account_state = client
        .get_account(registry_contract.id())
        .await
        .unwrap()
        .unwrap();

    // Validate mapping at slot 10: name -> addr
    let index = 10u8;
    let key = [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(0)].into();
    let expected: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)].into();

    let got = new_account_state
        .account()
        .storage()
        .get_map_item(index, key);

    assert!(got.is_ok(), "map item not found for provided key");
    assert_eq!(got.unwrap(), expected);

    Ok(())
}
