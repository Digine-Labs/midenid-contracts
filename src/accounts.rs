use miden_assembly::{DefaultSourceManager, Library, LibraryPath, ast::{Module, ModuleKind}};
use miden_client::{
    Client, ClientError, DebugMode, account::{Account, AccountBuilder, AccountId, AccountStorageMode, AccountType}, auth::{AuthSecretKey, NoAuth}, builder::ClientBuilder, keystore::FilesystemKeyStore, rpc::Endpoint, store::TransactionFilter, transaction::{OutputNote, TransactionId, TransactionRequestBuilder, TransactionStatus}};
use miden_crypto::{Felt, Word, dsa::rpo_falcon512::SecretKey};
use miden_lib::{account::auth::{self, AuthRpoFalcon512}, account::wallets::BasicWallet, transaction::TransactionKernel};
use miden_objects::account::AccountComponent;
use rand::{RngCore, rngs::StdRng};
use std::{fs, path::Path, sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::storage::naming_storage;
type ClientType = Client<FilesystemKeyStore<rand::prelude::StdRng>>;

pub async fn create_deployer_account(client: &mut Client<FilesystemKeyStore<StdRng>>, keystore: &mut Arc<FilesystemKeyStore<StdRng>>) -> anyhow::Result<Account> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = AuthSecretKey::new_rpo_falcon512();

    // Build the account
    let deployer_account = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key().to_commitment()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    // Add the account to the client
    client.add_account(&deployer_account, false).await?;

    // Add the key pair to the keystore
    keystore.add_key(&key_pair).unwrap();

    println!(
        "Deployer account ID: {:?}",
        deployer_account.id().to_string()
    );
    Ok(deployer_account) 
}

pub async fn create_naming_account(client: &mut Client<FilesystemKeyStore<StdRng>>) -> anyhow::Result<Account> {
    let account_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();

    let account_component = AccountComponent::compile(
            &account_code,
            TransactionKernel::assembler(),
            naming_storage()
        )?
        .with_supports_all_types();

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let account = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_component(account_component.clone())
        .with_auth_component(NoAuth)
        .build()?;

    client.add_account(&account, false).await?;

    println!(
        "Naming account ID: {:?}",
        account.id().to_string()
    );
    Ok(account)
}
