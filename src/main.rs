use miden_client::rpc::Endpoint;
use midenname_contracts::config::DeploymentConfig;
use midenname_contracts::deploy::{delete_keystore_and_store, instantiate_client};

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
            eprintln!("  - INITIAL_PRICE (registration price as u64)");
            eprintln!("  - CONTRACT_ADDRESS (optional, for existing contracts)");
            eprintln!("\nYou can create a .env file in the project root with these variables.");
            std::process::exit(1);
        }
    };

    // Print configuration
    config.print();

    // Clean up existing data (for now)
    delete_keystore_and_store().await;

    println!("✅ Ready to deploy to {} network", config.network.as_str());
    println!("💰 Initial registration price: {}", config.initial_price);

    let mut client = instantiate_client(Endpoint::testnet()).await?;
    
    client.sync_state().await?;
    println!("✅ Client synced with network\n");
    
    Ok(())
}
