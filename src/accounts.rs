use miden_client::{
    Client,
    account::{Account, AccountBuilder, AccountId, AccountStorageMode, AccountType},
    auth::{AuthScheme, AuthSecretKey, NoAuth},
    keystore::{FilesystemKeyStore, Keystore},
};
use miden_protocol::{
    account::{AccountComponent, AccountComponentMetadata},
    transaction::TransactionKernel,
};
use miden_standards::{account::auth::AuthSingleSig, account::wallets::BasicWallet};
use rand::RngCore;
use std::{fs, path::Path, sync::Arc};

use crate::storage::naming_storage;

pub async fn create_deployer_account(
    client: &mut Client<FilesystemKeyStore>,
    keystore: &mut Arc<FilesystemKeyStore>,
) -> anyhow::Result<Account> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = AuthSecretKey::new_falcon512_poseidon2();

    // Build the account
    let deployer_account = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(AuthSingleSig::new(
            key_pair.public_key().to_commitment(),
            AuthScheme::Falcon512Poseidon2,
        ))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    // Add the account to the client
    client.add_account(&deployer_account, false).await?;

    // Add the key pair to the keystore
    keystore.add_key(&key_pair, deployer_account.id()).await?;

    println!(
        "Deployer account ID: {:?}",
        deployer_account.id().to_string()
    );
    Ok(deployer_account)
}

pub async fn create_naming_account(
    client: &mut Client<FilesystemKeyStore>,
    is_network: bool,
) -> anyhow::Result<Account> {
    let account_code = fs::read_to_string(Path::new("./masm/accounts/naming_unsafe.masm")).unwrap();

    let storage_mode = if is_network {
        AccountStorageMode::Network
    } else {
        AccountStorageMode::Public
    };

    // Compile the account code using the assembler
    let source_manager = Arc::new(miden_assembly::DefaultSourceManager::default());
    let assembler = TransactionKernel::assembler_with_source_manager(source_manager.clone())
        .with_dynamic_library(miden_standards::StandardsLib::default())
        .expect("failed to load standards lib");
    let module = miden_assembly::ast::Module::parser(miden_assembly::ast::ModuleKind::Library)
        .parse_str("naming", account_code, source_manager)
        .unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    let account_component = AccountComponent::new(
        (*library).clone(),
        naming_storage(),
        AccountComponentMetadata::new("midenid-naming", [AccountType::RegularAccountImmutableCode]),
    )?;

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let account = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_auth_component(NoAuth)
        .with_component(account_component.clone())
        .build()?;

    client.add_account(&account, false).await.unwrap();

    println!("Naming account ID: {:?}", account.id().to_string());
    Ok(account)
}

pub async fn safe_account_import(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
) -> anyhow::Result<()> {
    if client.get_account(account_id).await?.is_none() {
        if let Err(e) = client.import_account_by_id(account_id).await {
            eprintln!("Warning: Failed to import account: {:?}", e);
        }
    }
    Ok(())
}
