use miden_client::{
    account::{Account, AccountBuilder, AccountStorageMode, AccountType}, builder::ClientBuilder, keystore::FilesystemKeyStore, rpc::{Endpoint, TonicRpcClient},
    crypto::SecretKey, Client, ClientError, DebugMode, auth::AuthSecretKey,
};
use miden_crypto::Word;
use miden_lib::{account::auth::{self, AuthRpoFalcon512}, account::wallets::BasicWallet, transaction::TransactionKernel};
use miden_objects::account::AccountComponent;
use rand::{Rng, SeedableRng, RngCore, rngs::StdRng};
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

pub async fn create_deployer_account(
    client: &mut miden_client::Client<miden_client::keystore::FilesystemKeyStore<rand::prelude::StdRng>>,
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