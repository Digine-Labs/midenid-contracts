use midenid_contracts::common::instantiate_client;
use miden_client::rpc::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Checking Local Miden Client Accounts\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Check what network to use
    let args: Vec<String> = std::env::args().collect();
    let network_str = args.get(1).map(|s| s.as_str()).unwrap_or("testnet");

    let endpoint = match network_str.to_lowercase().as_str() {
        "devnet" => Endpoint::devnet(),
        "testnet" => Endpoint::testnet(),
        "mainnet" => Endpoint::new("https".into(), "rpc.mainnet.miden.io".into(), None),
        _ => Endpoint::testnet(),
    };

    println!("🌐 Network: {}\n", network_str.to_uppercase());
    println!("💾 Database: ./store.sqlite3");
    println!("🔑 Keystore: ./keystore\n");

    let mut client = instantiate_client(endpoint).await?;

    println!("Syncing with network...");
    client.sync_state().await?;
    println!("✅ Synced\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Try to query accounts from the database
    println!("Checking local database for accounts...\n");

    // Check using sqlite3 directly
    use std::process::Command;

    let output = Command::new("sqlite3")
        .arg("./store.sqlite3")
        .arg("SELECT id FROM accounts;")
        .output()?;

    if output.status.success() {
        let accounts_output = String::from_utf8_lossy(&output.stdout);
        if accounts_output.trim().is_empty() {
            println!("❌ No accounts found in local database\n");
        } else {
            println!("✅ Found accounts in database:\n");
            for account_id in accounts_output.lines() {
                if !account_id.trim().is_empty() {
                    // Try to parse as u64 and convert to hex
                    if let Ok(id_num) = account_id.trim().parse::<u64>() {
                        println!("  Account ID (decimal): {}", id_num);
                        println!("  Account ID (hex):     0x{:x}\n", id_num);
                    } else {
                        println!("  Account ID: {}\n", account_id);
                    }
                }
            }
        }
    } else {
        println!("⚠️  Could not query database directly\n");
    }

    Ok(())
}
