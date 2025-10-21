use anyhow::Ok;
use miden_client::account::{AccountBuilder, AccountId, AccountStorageMode, AccountType};
use miden_client::auth::AuthSecretKey;
use miden_client::builder::ClientBuilder;
use miden_client::crypto::SecretKey;
use miden_client::keystore::FilesystemKeyStore;
use miden_client::rpc::{Endpoint, TonicRpcClient};
use miden_client::{Client, DebugMode};
use miden_lib::account::auth::AuthRpoFalcon512;
use miden_lib::account::wallets::BasicWallet;
use rand::{RngCore, rngs::StdRng};
use crate::config::DeploymentConfig;
use crate::deploy::{
    create_network_naming_account, create_network_pricing_account, delete_keystore_and_store, initialize_naming_contract, initialize_pricing_contract, instantiate_client
};
use std::sync::Arc;


pub async fn initialize_keystore() -> anyhow::Result<()> {
    let keystore = FilesystemKeyStore::new("./keystore".into())?;
    
    let mut client = create_client().await?;
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

    client.sync_state().await?;
    let last_block = client.get_sync_height().await?;
    
    println!("Keystore initialized. Client latest block heigh: {}", last_block.as_u64());

    let account_record = client.get_account(account.id()).await?;

    let seed = if let Some(record) = account_record {
        record.seed().cloned()
    } else {
        None
    };

    if let Some(ref seed_value) = seed {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🔐 ACCOUNT SEED (BACKUP THIS!):\n");
        println!("   {}\n", seed_value);
        println!("   Deployer Address: {}\n", account.id());
    }
    Ok(())
}

async fn create_client() -> anyhow::Result<Client<FilesystemKeyStore<StdRng>>> {
    let rpc_api = Arc::new(TonicRpcClient::new(&Endpoint::testnet(), 10000));

    let client = ClientBuilder::new().rpc(rpc_api.clone()).filesystem_keystore("./keystore").in_debug_mode(DebugMode::Enabled).build().await?;

    Ok(client)
}

/// Clean keystore and database
pub async fn clean() -> anyhow::Result<()> {
    println!("\n🧹 Cleaning Keystore and Database\n");

    delete_keystore_and_store().await;

    println!("✅ Cleanup complete");

    Ok(())
}

/// Show current configuration
pub async fn show_config() -> anyhow::Result<()> {
    println!("\n⚙️  Current Configuration\n");

    let config = DeploymentConfig::from_env()?;
    config.print();

    Ok(())
}

/// Deploy the pricing contract to the network
pub async fn deploy_pricing() -> anyhow::Result<()> {
    println!("\n📦 Deploying Pricing Contract\n");

    let config = DeploymentConfig::from_env()?;
    let payment_token_address = AccountId::from_hex(config.pricing_token_address())?;
    let setter_address = AccountId::from_hex(config.pricing_setter_account())?;
    let deployer_address = AccountId::from_hex(config.deployer_account())?;

    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    let (pricing_account, pricing_seed) = create_network_pricing_account().await;
    client.add_account(&pricing_account, Some(pricing_seed), false).await?;

    println!("✅ Pricing contract deployed: {}", pricing_account.id());

    client.sync_state().await?;

    initialize_pricing_contract(&mut client, deployer_address, payment_token_address, setter_address, pricing_account.clone()).await?;
    client.sync_state().await?;
    println!("✅ Pricing contract initialized");
    Ok(())
}

// TODOS

/// Deploy the naming contract to the network
pub async fn deploy_naming() -> anyhow::Result<()> {
    println!("\n📦 Deploying Naming Contract\n");

    let config = DeploymentConfig::from_env()?;
    let owner_address = AccountId::from_hex(config.naming_owner_account())?;
    let treasury_address = AccountId::from_hex(config.naming_treasury_account())?;
    let deployer_address = AccountId::from_hex(config.deployer_account())?;

    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    let (naming_account, naming_seed) = create_network_naming_account().await;
    client.add_account(&naming_account, Some(naming_seed), false).await?;

    println!("✅ Naming contract deployed: {}", naming_account.id());

    client.sync_state().await?;

    initialize_naming_contract(&mut client, deployer_address, owner_address, treasury_address, naming_account.clone()).await?;
    client.sync_state().await?;
    println!("✅ Naming contract initialized");
    Ok(())
}

/// Initialize the pricing contract
pub async fn init_pricing() -> anyhow::Result<()> {
    println!("\n🔧 Initializing Pricing Contract\n");

    let _config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    // TODO: Get pricing contract ID from environment or config
    println!("⚠️  This script needs the pricing contract ID to be configured");

    Ok(())
}

/// Set prices on the pricing contract
pub async fn set_prices() -> anyhow::Result<()> {
    println!("\n💰 Setting Prices\n");

    let _config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    // TODO: Implement price setting logic
    println!("⚠️  This script needs the deployer and pricing contract IDs");

    Ok(())
}