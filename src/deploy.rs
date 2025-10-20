use miden_client::{
    account::{Account, AccountBuilder, AccountStorageMode, AccountType}, builder::ClientBuilder, keystore::FilesystemKeyStore, rpc::{Endpoint, TonicRpcClient}, Client, ClientError, DebugMode
};
use miden_crypto::Word;
use miden_lib::{account::auth, transaction::TransactionKernel};
use miden_objects::account::AccountComponent;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;

use crate::utils::{get_naming_account_code, get_pricing_account_code, naming_storage, pricing_storage};

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
    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let client = ClientBuilder::new()
        .rpc(rpc_api.clone())
        .filesystem_keystore("./keystore")
        .in_debug_mode(DebugMode::Enabled)
        .build()
        .await?;

    Ok(client)
}

pub async fn create_network_naming_account() -> (Account, Word) {
    let storage_slots = naming_storage();
    let account_code = get_naming_account_code();

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let (account, word) = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .account_type(AccountType::RegularAccountImmutableCode)
        .with_component(account_component)
        .storage_mode(AccountStorageMode::Network)
        .build().unwrap();
    return (account, word);
}

pub async fn create_network_pricing_account() -> (Account, Word) {
    let storage_slots = pricing_storage();
    let account_code = get_pricing_account_code();

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let (account, word) = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .account_type(AccountType::RegularAccountImmutableCode)
        .with_component(account_component)
        .storage_mode(AccountStorageMode::Network)
        .build().unwrap();
    return (account, word);
}