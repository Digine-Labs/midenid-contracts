use miden_client::rpc::Endpoint;
use midenid_contracts::common::instantiate_client;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let network_str = args.get(1).map(|s| s.as_str()).unwrap_or("testnet");

    let endpoint = match network_str.to_lowercase().as_str() {
        "devnet" => Endpoint::devnet(),
        "testnet" => Endpoint::testnet(),
        "mainnet" => Endpoint::new("https".into(), "rpc.mainnet.miden.io".into(), None),
        _ => Endpoint::testnet(),
    };

    println!("\nListing Local Accounts");
    println!("Network: {}", network_str);
    println!("Database: ./store.sqlite3");
    println!("Keystore: ./keystore\n");

    let mut client = instantiate_client(endpoint).await?;

    println!("Syncing...");
    client.sync_state().await?;
    println!("Synced\n");

    let output = Command::new("sqlite3")
        .arg("./store.sqlite3")
        .arg("SELECT id FROM accounts;")
        .output()?;

    if output.status.success() {
        let accounts_output = String::from_utf8_lossy(&output.stdout);
        if accounts_output.trim().is_empty() {
            println!("No accounts found in local database");
        } else {
            println!("Found accounts:\n");
            for account_id in accounts_output.lines() {
                if !account_id.trim().is_empty() {
                    if let Ok(id_num) = account_id.trim().parse::<u64>() {
                        println!("  {} (0x{:x})", id_num, id_num);
                    } else {
                        println!("  {}", account_id);
                    }
                }
            }
        }
    } else {
        println!("Could not query database directly");
    }

    Ok(())
}
