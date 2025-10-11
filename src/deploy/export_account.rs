/// Export account details including seed phrase for backup
use miden_client::{account::AccountId, rpc::Endpoint};
use midenid_contracts::common::instantiate_client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Error: Missing account ID");
        eprintln!("Usage: {} <account_id_hex> [network]", args[0]);
        eprintln!("Example: {} 0x1c89546e3b82cd1012a9fe4853bc68", args[0]);
        std::process::exit(1);
    }

    let account_id_str = &args[1];
    let network_str = args.get(2).map(|s| s.as_str()).unwrap_or("testnet");

    // Parse account ID
    let account_id = if account_id_str.starts_with("0x") {
        AccountId::from_hex(account_id_str).map_err(|e| format!("Invalid hex account ID: {}", e))?
    } else {
        return Err("Account ID must be in hex format (0x...)".into());
    };

    let endpoint = match network_str.to_lowercase().as_str() {
        "devnet" => Endpoint::devnet(),
        "testnet" => Endpoint::testnet(),
        "mainnet" => Endpoint::new("https".into(), "rpc.mainnet.miden.io".into(), None),
        _ => Endpoint::testnet(),
    };

    println!("\nExporting Account");
    println!("Network: {}", network_str);
    println!("Account: {}\n", account_id);

    let mut client = instantiate_client(endpoint).await?;
    client.sync_state().await?;

    let account_record = client.get_account(account_id).await?;

    if let Some(record) = account_record {
        println!("Account Information:");
        println!("  ID (hex):     {}", record.account().id().to_hex());
        println!("  ID (decimal): {}", record.account().id());
        println!("  Storage:      {:?}", record.account().id().storage_mode());
        println!("  Type:         {:?}", record.account().id().account_type());

        if let Some(seed) = record.seed() {
            println!("\nACCOUNT SEED (KEEP SECRET):");
            println!("{}", seed);
            println!("\nWARNING: This seed gives full control of the account.");
            println!("Store it securely and never share it.");
        } else {
            println!("\nAccount seed not available (might be imported without seed)");
        }
    } else {
        eprintln!("Account not found in local database");
        eprintln!("Run: ./target/release/list_accounts {}", network_str);
        std::process::exit(1);
    }

    Ok(())
}
