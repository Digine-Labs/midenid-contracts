use midenid_contracts::common::{
    create_basic_account, create_public_immutable_contract, create_public_note, wait_for_note, instantiate_client, create_library
};

use miden_client_tools::{
    create_tx_script, delete_keystore_and_store,
};

use miden_client::{
    account::{Address, AddressInterface, AccountIdAddress}, keystore::FilesystemKeyStore, note::NoteAssets, rpc::Endpoint, transaction::TransactionRequestBuilder, ClientError, Word
};
use miden_objects::account::NetworkId;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn increment_counter_with_note() -> Result<(), ClientError> {
    delete_keystore_and_store(None).await;

    let endpoint = Endpoint::localhost();
    let mut client = instantiate_client().await.unwrap();

    let keystore = FilesystemKeyStore::new("./keystore".into()).unwrap();

    let sync_summary = client.sync_state().await.unwrap();
    println!("Latest block: {}", sync_summary.block_num);

    // -------------------------------------------------------------------------
    // STEP 1: Create Basic User Account
    // -------------------------------------------------------------------------
    let alice_account = create_basic_account(&mut client, keystore.clone())
        .await
        .unwrap();
    println!(
        "alice account id: {:?}",
        Address::from(AccountIdAddress::new(
            alice_account.id(),
            AddressInterface::Unspecified
        ))
        .to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 2: Create Counter Smart Contract
    // -------------------------------------------------------------------------
    let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id_registry.masm")).unwrap();

    let (counter_contract, counter_seed) =
        create_public_immutable_contract(&mut client, &registry_code)
            .await
            .unwrap();
    println!(
        "contract id: {:?}",
        Address::from(AccountIdAddress::new(
            counter_contract.id(),
            AddressInterface::Unspecified
        ))
        .to_bech32(NetworkId::Testnet)
    );

    client
        .add_account(&counter_contract, Some(counter_seed), false)
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare & Create the Note
    // -------------------------------------------------------------------------
    let note_code = fs::read_to_string(Path::new("./masm/notes/init_registry_note.masm")).unwrap();
    //let account_code = fs::read_to_string(Path::new("./masm/accounts/miden_id_registry.masm")).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let ext_lib = create_library(registry_code, "external_contract::miden_id_registry_contract").unwrap();
    let increment_note =
        create_public_note(&mut client, &ext_lib, note_code, alice_account, note_assets)
            .await
            .unwrap();

    println!("increment note created, waiting for onchain commitment");

    // -------------------------------------------------------------------------
    // STEP 4: Consume the Note
    // -------------------------------------------------------------------------
    wait_for_note(&mut client, &counter_contract, &increment_note)
        .await
        .unwrap();


    let consume_custom_req = TransactionRequestBuilder::new()
        .authenticated_input_notes([(increment_note.id(), None)])
        .build()
        .unwrap();

    let tx_result = client
        .new_transaction(counter_contract.id(), consume_custom_req)
        .await
        .unwrap();
    let _ = client.submit_transaction(tx_result).await;

    // -------------------------------------------------------------------------
    // STEP 5: Validate Updated State
    // -------------------------------------------------------------------------
    sleep(Duration::from_secs(5)).await;

    delete_keystore_and_store(None).await;

    let mut client = instantiate_client().await.unwrap();

    client
        .import_account_by_id(counter_contract.id())
        .await
        .unwrap();

    let new_account_state = client.get_account(counter_contract.id()).await.unwrap();

    if let Some(account) = new_account_state.as_ref() {
        let count: Word = account.account().storage().get_item(0).unwrap().into();
        let val = count.get(3).unwrap().as_int();
        assert_eq!(val, 1);
    }

    Ok(())
}