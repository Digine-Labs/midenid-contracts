/// Export account details including seed phrase for backup
use midenid_contracts::common::instantiate_client;
use miden_client::rpc::Endpoint;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("\n❌ Error: Missing account ID\n");
        eprintln!("Usage: {} <account_id_hex>\n", args[0]);
        eprintln!("Example:");
        eprintln!("  {} 0x1c89546e3b82cd1012a9fe4853bc68\n", args[0]);
        std::process::exit(1);
    }

    let account_id_str = &args[1];
    let network_str = args.get(2).map(|s| s.as_str()).unwrap_or("testnet");

    println!("\n📤 Exporting Account Details\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Parse account ID
    use miden_client::account::AccountId;
    let account_id = if account_id_str.starts_with("0x") {
        AccountId::from_hex(account_id_str)
            .map_err(|e| format!("Invalid hex account ID: {}", e))?
    } else {
        return Err("Account ID must be in hex format (0x...)".into());
    };

    let endpoint = match network_str.to_lowercase().as_str() {
        "devnet" => Endpoint::devnet(),
        "testnet" => Endpoint::testnet(),
        "mainnet" => Endpoint::new("https".into(), "rpc.mainnet.miden.io".into(), None),
        _ => Endpoint::testnet(),
    };

    println!("🌐 Network: {}", network_str.to_uppercase());
    println!("🔍 Looking for account: {}\n", account_id);

    let mut client = instantiate_client(endpoint).await?;
    client.sync_state().await?;

    // Get account from database
    let account_record = client.get_account(account_id).await?;

    if let Some(record) = account_record {
        println!("✅ Account found!\n");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        println!("📋 Account Information:\n");
        println!("  Account ID (hex):     {}", record.account().id().to_hex());
        println!("  Account ID (decimal): {}", record.account().id());
        println!("  Storage Mode:         {:?}", record.account().id().storage_mode());
        println!("  Account Type:         {:?}\n", record.account().id().account_type());

        // Get the seed if available
        if let Some(seed) = record.seed() {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            println!("🔐 ACCOUNT SEED (KEEP THIS SECRET!):\n");
            println!("  {}\n", seed);
            println!("⚠️  WARNING: This seed gives full control of the account!");
            println!("   Store it securely and never share it.\n");
        } else {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            println!("⚠️  Account seed not available (might be imported without seed)\n");
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    } else {
        eprintln!("❌ Account not found in local database!\n");
        eprintln!("Make sure the account exists locally. Run:");
        eprintln!("  ./target/release/list_accounts {}\n", network_str);
        std::process::exit(1);
    }

    Ok(())
}
