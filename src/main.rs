use miden_client::keystore::{self, FilesystemKeyStore};
use miden_client::rpc::Endpoint;
use miden_client::transaction::{OutputNote, TransactionRequestBuilder};
use miden_client::{account::AccountId};
use midenname_contracts::config::DeploymentConfig;
use midenname_contracts::deploy::{create_deployer_account, create_network_pricing_account, delete_keystore_and_store, instantiate_client};
use midenname_contracts::notes::create_pricing_initialize_note;
use miden_objects::{address::Address};
use midenname_contracts::utils::create_tx_script;
use std::{env, fs, path::Path};

use tokio::time::{sleep, Duration};
fn parse_account_id(id_str: &str) -> Result<AccountId, String> {
    // Check if it's a bech32 address (starts with known prefixes)
    if id_str.starts_with("mm1") || id_str.starts_with("mtst1") || id_str.starts_with("mdev1") {
        // Parse bech32 and extract AccountId
        let (_network_id, address) = Address::from_bech32(id_str)
            .map_err(|e| format!("Failed to parse bech32 address '{}': {}", id_str, e))?;

        match address {
            Address::AccountId(account_id_address) => {
                Ok(account_id_address.id())
            }
            _ => Err(format!("Unsupported address type in: {}", id_str)),
        }
    } else if id_str.starts_with("0x") {
        // Parse hex format
        AccountId::from_hex(id_str)
            .map_err(|e| format!("Invalid hex account ID '{}': {}", id_str, e))
    } else {
        Err(format!(
            "Account ID must be in hex (0x...) or bech32 (mm1.../mtst.../mdev...) format.\n\
             Provided: '{}'",
            id_str
        ))
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🚀 Miden Name Registry Deployment\n");

    // Load configuration from environment variables
    let config = match DeploymentConfig::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("❌ Configuration Error: {}", e);
            eprintln!("\nPlease ensure the following environment variables are set:");
            eprintln!("  - MIDEN_NETWORK (testnet or mainnet)");
            eprintln!("\nYou can create a .env file in the project root with these variables.");
            std::process::exit(1);
        }
    };

    // Print configuration
    config.print();

    // Clean up existing data (for now)
    delete_keystore_and_store().await;
    let naming_owner_account = parse_account_id(config.naming_owner_account()).unwrap();
    let naming_treasury_account = parse_account_id(config.naming_treasury_account()).unwrap();
    let pricing_setter_account = parse_account_id(config.pricing_setter_account()).unwrap();
    let pricing_token_address = parse_account_id(config.pricing_token_address()).unwrap();
    

    println!("✅ Ready to deploy to {} network", config.network.as_str());
    println!("✅ Naming owner {}", naming_owner_account.to_hex());
    //println!("💰 Initial registration price: {}", config.initial_price);

    let mut client: miden_client::Client<miden_client::keystore::FilesystemKeyStore<rand::prelude::StdRng>> = instantiate_client(Endpoint::testnet()).await?;
    
    client.sync_state().await?;
    println!("✅ Client synced with network\n");
    
    let keystore = FilesystemKeyStore::new("./keystore".into())?;
    let (deployer_account, deployer_account_key_pair) = create_deployer_account(&mut client, keystore).await?;

    let account_record = client.get_account(deployer_account.id()).await?;
    let deployer_seed = if let Some(record) = account_record {
            record.seed().cloned()
        } else {
            None
        };
    
    let (deployer_id, deployer_account, deployer_seed) = (deployer_account.id(), deployer_account, deployer_seed.unwrap());

    println!("Deployer seed: {}", deployer_seed.to_string());
    
    let (pricing_account, pricing_seed) = create_network_pricing_account().await;
    client.add_account(&pricing_account, Some(pricing_seed), false).await?;

    println!("Pricing contract deployed: {}", pricing_account.id());

    sleep(Duration::from_secs(5)).await;

    // Init pricing

    let pricing_initialize_note = create_pricing_initialize_note(deployer_account.clone(), pricing_token_address, pricing_setter_account, pricing_account.clone()).await?;
    
    let tx_request = TransactionRequestBuilder::new().own_output_notes(vec![OutputNote::Full(pricing_initialize_note.clone())]).build().unwrap();
    let tx_result = client.new_transaction(deployer_account.id(), tx_request).await?;

    let _ = client.submit_transaction(tx_result).await?;
    client.sync_state().await.unwrap();

    let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
    let transaction_script = create_tx_script(nop_script_code, None)?;

    let request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(pricing_initialize_note, None)])
        .custom_script(transaction_script)
        .build()?;

    let tx_result = client.new_transaction(pricing_account.id(), request).await?;
    client.submit_transaction(tx_result).await?;
    println!("✅ Pricing contract initialized!\n");
    Ok(())
}
