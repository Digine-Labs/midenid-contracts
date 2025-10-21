use anyhow::Ok;
use miden_client::builder::ClientBuilder;
use miden_client::keystore::FilesystemKeyStore;
use miden_client::rpc::{Endpoint, TonicRpcClient};
use miden_client::transaction::{OutputNote, TransactionRequestBuilder};
use miden_client::{Client, DebugMode};
use miden_crypto::Felt;
use rand::rngs::StdRng;
use crate::config::DeploymentConfig;
use crate::deploy::{
    create_deployer_account, create_network_naming_account, create_network_pricing_account,
    delete_keystore_and_store, instantiate_client,
};
use crate::notes::{create_naming_initialize_note, create_price_set_note, create_pricing_initialize_note};
use crate::utils::create_tx_script;
use std::sync::Arc;
use std::{fs, path::Path};
use tokio::time::{sleep, Duration};

pub async fn initialize_keystore() -> anyhow::Result<()> {
    let keystore = FilesystemKeyStore::new("./keystore".into())?;
    let mut client = create_client().await?;

    client.sync_state().await;
    let last_block =client.get_sync_height().await?;

    println!("Keystore initialized. Client latest block heigh: {}", last_block.as_u64());
    Ok(())
}

async fn create_client() -> anyhow::Result<Client<FilesystemKeyStore<StdRng>>> {
    let rpc_api = Arc::new(TonicRpcClient::new(&Endpoint::testnet(), 10000));

    let client = ClientBuilder::new().rpc(rpc_api.clone()).filesystem_keystore("./keystore").in_debug_mode(DebugMode::Enabled).build().await?;

    Ok(client)
}

/// Deploy the pricing contract to the network
pub async fn deploy_pricing() -> anyhow::Result<()> {
    println!("\n📦 Deploying Pricing Contract\n");

    let config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    let (pricing_account, pricing_seed) = create_network_pricing_account().await;
    client.add_account(&pricing_account, Some(pricing_seed), false).await?;

    println!("✅ Pricing contract deployed: {}", pricing_account.id());
    println!("   Account ID: 0x{}", pricing_account.id().to_hex());

    Ok(())
}

/// Deploy the naming contract to the network
pub async fn deploy_naming() -> anyhow::Result<()> {
    println!("\n📦 Deploying Naming Contract\n");

    let config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    let (naming_account, naming_seed) = create_network_naming_account().await;
    client.add_account(&naming_account, Some(naming_seed), false).await?;

    println!("✅ Naming contract deployed: {}", naming_account.id());
    println!("   Account ID: 0x{}", naming_account.id().to_hex());

    Ok(())
}

/// Initialize the pricing contract
pub async fn init_pricing() -> anyhow::Result<()> {
    println!("\n🔧 Initializing Pricing Contract\n");

    let config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    // TODO: Get pricing contract ID from environment or config
    println!("⚠️  This script needs the pricing contract ID to be configured");

    Ok(())
}

/// Set prices on the pricing contract
pub async fn set_prices() -> anyhow::Result<()> {
    println!("\n💰 Setting Prices\n");

    let config = DeploymentConfig::from_env()?;
    let mut client = instantiate_client(Endpoint::testnet()).await?;
    client.sync_state().await?;

    // TODO: Implement price setting logic
    println!("⚠️  This script needs the deployer and pricing contract IDs");

    Ok(())
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