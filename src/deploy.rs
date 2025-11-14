use miden_client::{
    account::{Account, AccountBuilder, AccountId, AccountStorageMode, AccountType}, auth::AuthSecretKey, builder::ClientBuilder, crypto::SecretKey, keystore::FilesystemKeyStore, rpc::{Endpoint, TonicRpcClient}, store::TransactionFilter, transaction::{OutputNote, TransactionId, TransactionRequestBuilder, TransactionStatus}, Client, ClientError, DebugMode};
use miden_crypto::{Felt, Word};
use miden_lib::{account::auth::{self, AuthRpoFalcon512}, account::wallets::BasicWallet, transaction::TransactionKernel};
use miden_objects::account::AccountComponent;
use rand::{RngCore, rngs::StdRng};
use std::{sync::Arc, time::Duration};
use crate::{notes::{create_naming_initialize_note, create_naming_set_payment_token_contract, create_naming_set_pricing_root, create_naming_transfer_owner_note, create_pricing_initialize_note}, utils::{get_naming_account_code, get_price_set_notes, get_pricing_account_code, naming_storage, pricing_storage}};
use tokio::time::sleep;
type ClientType = Client<FilesystemKeyStore<rand::prelude::StdRng>>;

pub async fn delete_keystore_and_store() {
    let store_path = "./store.sqlite3";
    if tokio::fs::metadata(store_path).await.is_ok() {
        if let Err(e) = tokio::fs::remove_file(store_path).await {
            eprintln!("failed to remove {}: {}", store_path, e);
        } else {
            println!("cleared sqlite store: {}", store_path);
        }
    } else {
        println!("store not found: {}", store_path);
    }

    let keystore_dir = "./keystore";
    match tokio::fs::read_dir(keystore_dir).await {
        Ok(mut dir) => {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let file_path = entry.path();
                if let Err(e) = tokio::fs::remove_file(&file_path).await {
                    eprintln!("failed to remove {}: {}", file_path.display(), e);
                } else {
                    println!("removed file: {}", file_path.display());
                }
            }
        }
        Err(e) => eprintln!("failed to read directory {}: {}", keystore_dir, e),
    }
}

pub async fn instantiate_client(endpoint: Endpoint) -> Result<ClientType, ClientError> {
    let timeout_ms = 30_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let client = ClientBuilder::new()
        .rpc(rpc_api.clone())
        .filesystem_keystore("./keystore")
        .in_debug_mode(DebugMode::Enabled)
        .build()
        .await?;

    Ok(client)
}

pub async fn create_deployer_account(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
     keystore: FilesystemKeyStore<StdRng>
) -> Result<(miden_client::account::Account, SecretKey), ClientError> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = SecretKey::with_rng(client.rng());
    let builder = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Network)
        .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key().clone()))
        .with_component(BasicWallet);
    let (account, seed) = builder.build().unwrap();
    client.add_account(&account, Some(seed), false).await?;
    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(key_pair.clone()))
        .unwrap();

    Ok((account, key_pair))
}

pub async fn create_network_naming_account(client: &mut Client<FilesystemKeyStore<StdRng>>) -> (Account, Word) {
    let storage_slots = naming_storage();
    let account_code = get_naming_account_code();

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let (account, word) = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Network)
        .with_component(account_component)
        .with_auth_component(auth::NoAuth)
        .build()
        .unwrap();
    return (account, word);
}

pub async fn create_network_pricing_account(client: &mut Client<FilesystemKeyStore<StdRng>>) -> (Account, Word) {
    let storage_slots = pricing_storage();
    let account_code = get_pricing_account_code();

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let (account, word) = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Network)
        .with_component(account_component)
        .with_auth_component(auth::NoAuth)
        .build()
        .unwrap();
    return (account, word);
}

pub async fn initialize_pricing_contract(client: &mut Client<FilesystemKeyStore<StdRng>>, initializer_account: AccountId, token: AccountId, setter: AccountId, contract: Account) -> anyhow::Result<()> {
    let initialize_note = create_pricing_initialize_note(initializer_account, token, setter, contract.clone()).await?;

    //let tx_request = TransactionRequestBuilder::new().own_output_notes(vec![OutputNote::Full(initialize_note.clone())]).build()?;
    let tx_request = TransactionRequestBuilder::new()
    
        .own_output_notes(vec![OutputNote::Full(initialize_note)])
        .build()?;
    
    let tx_result = client.new_transaction(initializer_account, tx_request).await?;

    let _ = client.submit_transaction(tx_result).await?;
    client.sync_state().await?;
    Ok(())
}

pub async fn initialize_naming_contract(client: &mut Client<FilesystemKeyStore<StdRng>>,initializer_account: AccountId, owner: AccountId, treasury: AccountId, contract: Account) -> anyhow::Result<()> {
    let initialize_note = create_naming_initialize_note(initializer_account, owner, contract.clone()).await?;

    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(initialize_note)])
        .build()?;
    let tx_result = client.new_transaction(initializer_account, tx_request).await?;
    let _ = client.submit_transaction(tx_result).await?;
    client.sync_state().await?;
    Ok(())
}

async fn wait_for_tx(
    client: &mut Client<FilesystemKeyStore<rand::prelude::StdRng>>,
    tx_id: TransactionId,
) -> Result<(), ClientError> {
    loop {
        client.sync_state().await?;

        // Check transaction status
        let txs = client
            .get_transactions(TransactionFilter::Ids(vec![tx_id]))
            .await?;
        let tx_committed = if !txs.is_empty() {
            matches!(txs[0].status, TransactionStatus::Committed { .. })
        } else {
            false
        };

        if tx_committed {
            println!("✅ transaction {} committed", tx_id.to_hex());
            break;
        }

        println!(
            "Transaction {} not yet committed. Waiting...",
            tx_id.to_hex()
        );
        sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

pub async fn initialize_all(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    initializer_account: AccountId, // tx_sender_1
    updated_account: AccountId, // tx_sender_2
    owner: AccountId, 
    treasury: AccountId, 
    token: AccountId, 
    _setter: AccountId, 
    naming_contract: Account, 
    pricing_contract: Account,
    prices: Vec<Felt>
) -> anyhow::Result<()> {
    let init_naming_note = create_naming_initialize_note(initializer_account, initializer_account, naming_contract.clone()).await?;
    // Initially set setter as initializer_account
    let init_pricing_note = create_pricing_initialize_note(initializer_account, token, initializer_account, pricing_contract.clone()).await?;

    // price set notes
    let set_price_notes = get_price_set_notes(initializer_account, pricing_contract.id(), prices).await;

    // naming set payment token note
    let set_payment_token = create_naming_set_payment_token_contract(initializer_account, token, pricing_contract.id(), naming_contract.id()).await?;

    // naming transfer ownership
    let transfer_ownership = create_naming_transfer_owner_note(initializer_account, updated_account, naming_contract.id()).await?;

    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![
            OutputNote::Full(init_naming_note.clone()), 
            OutputNote::Full(init_pricing_note.clone()),
            OutputNote::Full(set_price_notes[0].clone()),
            OutputNote::Full(set_price_notes[1].clone()),
            OutputNote::Full(set_price_notes[2].clone()),
            OutputNote::Full(set_price_notes[3].clone()),
            OutputNote::Full(set_price_notes[4].clone()),
            OutputNote::Full(set_payment_token.clone()),
            OutputNote::Full(transfer_ownership.clone()),
        ])
        .build()?;
    let tx_result = client.new_transaction(initializer_account, tx_request).await?;
    let _ = client.submit_transaction(tx_result.clone()).await?;
    client.sync_state().await?;
    println!("Submitted notes");

    // Consume notes


    let consume_naming_notes_request = TransactionRequestBuilder::new()
            .unauthenticated_input_notes([
                    (init_naming_note, None), 
                    (set_payment_token, None),
                    (transfer_ownership, None),
                ])
            .build()
            .unwrap();
    

    let consume_naming_notes_tx_result = client
        .new_transaction(naming_contract.id(), consume_naming_notes_request)
        .await
        .unwrap();

    let _ = client.submit_transaction(consume_naming_notes_tx_result.clone()).await;
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;

    let naming_notes_consume_tx_id = consume_naming_notes_tx_result.executed_transaction().id();

    wait_for_tx(client, naming_notes_consume_tx_id).await.unwrap();
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;

    println!("Naming notes consumed.");

    let consume_pricing_notes_request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([
                (init_pricing_note, None),
                (set_price_notes[0].clone(), None),
                (set_price_notes[1].clone(), None),
                (set_price_notes[2].clone(), None),
                (set_price_notes[3].clone(), None),
                (set_price_notes[4].clone(), None),
            ])
        .build()
        .unwrap();
    
    let consume_pricing_notes_tx_result = client
        .new_transaction(pricing_contract.id(), consume_pricing_notes_request)
        .await
        .unwrap();
    
    let _ = client.submit_transaction(consume_pricing_notes_tx_result.clone()).await;
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;
    
    
    let pricing_notes_consume_tx_id = consume_pricing_notes_tx_result.executed_transaction().id();

    wait_for_tx(client, pricing_notes_consume_tx_id).await.unwrap();
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;

    println!("Consumed all noted. Initialization success. Ensuring initializations");

    let pricing_account = client.get_account(pricing_contract.id()).await?;
    let pricing_account_data = pricing_account.unwrap().account().clone();

    let naming_account = client.get_account(naming_contract.id()).await?;
    let naming_account_data = naming_account.unwrap().account().clone();

    assert_eq!(pricing_account_data.storage().get_item(0).unwrap().get(0).unwrap().as_int(), 1);
    assert_eq!(naming_account_data.storage().get_item(0).unwrap().get(0).unwrap().as_int(), 1);

    println!("Naming current owner: {:?}", naming_account_data.storage().get_item(1));

    // Now we should update procedure root
    
    let pricing_root = pricing_account_data.storage().get_item(4).unwrap();
    
    let naming_root_set_note = create_naming_set_pricing_root(updated_account, pricing_root, pricing_contract.id(), naming_contract.id()).await?;
        
    let transfer_ownership = create_naming_transfer_owner_note(updated_account, owner, naming_contract.id()).await?;

    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![
            OutputNote::Full(naming_root_set_note.clone()), 
            OutputNote::Full(transfer_ownership.clone()),
        ])
        .build()?;
    let tx_result = client.new_transaction(updated_account, tx_request).await?;
    let _ = client.submit_transaction(tx_result.clone()).await?;
    client.sync_state().await?;
    println!("Submitted update notes");

    let consume_naming_notes_request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([
                (naming_root_set_note, None), 
                (transfer_ownership, None),
            ])
        .build()
        .unwrap();

    let consume_naming_notes_tx_result = client
        .new_transaction(naming_contract.id(), consume_naming_notes_request)
        .await
        .unwrap();

    let _ = client.submit_transaction(consume_naming_notes_tx_result.clone()).await;
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;

    let update_notes_tx_id = consume_naming_notes_tx_result.executed_transaction().id();

    wait_for_tx(client, update_notes_tx_id).await.unwrap();
    sleep(Duration::from_secs(2)).await;
    client.sync_state().await?;
    
    println!("Everything is done. Now ensuring contract states are correct.");

    let pricing_account = client.get_account(pricing_contract.id()).await?;
    let pricing_account_data = pricing_account.unwrap().account().clone();

    let naming_account = client.get_account(naming_contract.id()).await?;
    let naming_account_data = naming_account.unwrap().account().clone();

    println!("Naming owner: {:?}", naming_account_data.storage().get_item(1).unwrap());
    println!("Naming treasury: {:?}", naming_account_data.storage().get_item(2).unwrap());
    println!("Naming token to pricing: {:?}", naming_account_data.storage().get_map_item(3, Word::new([Felt::new(token.suffix().as_int()),token.prefix().as_felt(), Felt::new(0), Felt::new(0) ])).unwrap());
    println!("Naming pricing root: {:?}", naming_account_data.storage().get_map_item(7, Word::new([Felt::new(pricing_contract.id().suffix().as_int()),pricing_contract.id().prefix().as_felt(), Felt::new(0), Felt::new(0) ])).unwrap());
    
    println!("Pricing setter: {:?}", pricing_account_data.storage().get_item(1).unwrap());
    println!("Pricing token: {:?}", pricing_account_data.storage().get_item(2).unwrap());
    println!("Pricing Letter(1) price: {:?}", pricing_account_data.storage().get_map_item(3, Word::new([Felt::new(1),Felt::new(0),Felt::new(0),Felt::new(0)])).unwrap());
    println!("Pricing Letter(2) price: {:?}", pricing_account_data.storage().get_map_item(3, Word::new([Felt::new(2),Felt::new(0),Felt::new(0),Felt::new(0)])).unwrap());
    println!("Pricing Letter(3) price: {:?}", pricing_account_data.storage().get_map_item(3, Word::new([Felt::new(3),Felt::new(0),Felt::new(0),Felt::new(0)])).unwrap());
    println!("Pricing Letter(4) price: {:?}", pricing_account_data.storage().get_map_item(3, Word::new([Felt::new(4),Felt::new(0),Felt::new(0),Felt::new(0)])).unwrap());
    println!("Pricing Letter(5) price: {:?}", pricing_account_data.storage().get_map_item(3, Word::new([Felt::new(5),Felt::new(0),Felt::new(0),Felt::new(0)])).unwrap());
    println!("Pricing calculate root: {:?}", pricing_account_data.storage().get_item(4).unwrap());


    Ok(())
}