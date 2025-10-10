use std::{env, fs, path::Path};
use midenid_contracts::common::{
    create_library, create_public_immutable_contract, create_public_note_with_library_and_inputs,
    create_tx_script, instantiate_client,
};
use miden_client::{
    account::AccountId,
    rpc::Endpoint,
    transaction::TransactionRequestBuilder,
};
use miden_objects::{address::Address, note::{NoteAssets, NoteInputs}, Felt, FieldElement};
use tokio::time::{sleep, Duration};

/// Parses an account ID from either hexadecimal or Bech32 format.
///
/// This function supports two input formats for Miden account IDs:
/// - **Hexadecimal**: Prefixed with `0x` (e.g., `0x1c89546e3b82cd1012a9fe4853bc68`)
/// - **Bech32**: Network-specific prefixes (e.g., `mm1...`, `mtst1...`, `mdev1...`)
///
/// # Arguments
///
/// * `id_str` - A string slice containing the account ID in either hex or bech32 format
///
/// # Returns
///
/// * `Ok(AccountId)` - Successfully parsed account ID
/// * `Err(String)` - Error message if parsing fails or format is unsupported
///
/// # Errors
///
/// Returns an error if:
/// - The input string doesn't start with `0x`, `mm1`, `mtst1`, or `mdev1`
/// - The hex format is invalid
/// - The bech32 address cannot be decoded
/// - The bech32 address doesn't contain an AccountId type
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Miden ID Registry Deployment Script\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("\n❌ Error: Missing required parameters\n");
        eprintln!("Usage: {} <network> <payment_token_id> <price> [owner_account_id]\n", args[0]);
        eprintln!("Examples:");
        eprintln!("  # Auto-create owner account:");
        eprintln!("  {} testnet 0x97598f759deab5201e93e1aac55997 10\n", args[0]);
        eprintln!("  # Use existing owner account:");
        eprintln!("  {} testnet 0x97598f759deab5201e93e1aac55997 10 0x1c89546e3b82cd1012a9fe4853bc68\n", args[0]);
        eprintln!("  # With bech32 addresses:");
        eprintln!("  {} testnet mtst1... 10 mtst1qqwgj4rw8wpv6yqj48lys5audpcqqykld75\n", args[0]);
        eprintln!("Parameters:");
        eprintln!("  network           - Network to deploy to: devnet, testnet, or mainnet");
        eprintln!("  payment_token_id  - Faucet/token ID (hex: 0x... OR bech32: mm1.../mtst.../mdev...)");
        eprintln!("  price             - Registration price in tokens (e.g., 10, 100)");
        eprintln!("  owner_account_id  - (Optional) Use existing account, or omit to auto-create\n");
        std::process::exit(1);
    }

    let network_str = &args[1];
    let payment_token_id_str = &args[2];
    let price: u64 = args[3].parse()
        .map_err(|_| format!("Invalid price: '{}'. Must be a number.", args[3]))?;
    let owner_id_str = args.get(4).map(|s| s.as_str());

    // Parse network
    let endpoint = match network_str.to_lowercase().as_str() {
        "devnet" => Endpoint::devnet(),
        "testnet" => Endpoint::testnet(),
        "mainnet" => {
            // Mainnet endpoint - using the expected mainnet RPC URL
            // Note: Update this URL when Miden mainnet is officially launched
            Endpoint::new("https".into(), "rpc.mainnet.miden.io".into(), None)
        },
        _ => {
            return Err(format!(
                "Invalid network: '{}'. Must be one of: devnet, testnet, mainnet",
                network_str
            ).into());
        }
    };

    println!("🌐 Target Network: {}\n", network_str.to_uppercase());

    // Parse payment token ID (supports both hex and bech32)
    println!("🔍 Parsing payment token ID...");
    if payment_token_id_str.starts_with("mm1") || payment_token_id_str.starts_with("mtst1") || payment_token_id_str.starts_with("mdev1") {
        println!("   Detected bech32 format, converting to AccountId...");
    }
    let payment_token_id = parse_account_id(payment_token_id_str)?;
    println!("   ✓ Payment Token AccountId: {}\n", payment_token_id);

    let mut client = instantiate_client(endpoint.clone()).await?;

    println!("📡 Connected to {}", network_str);
    println!("💾 Database: ./store.sqlite3");
    println!("🔑 Keystore: ./keystore\n");

    client.sync_state().await?;
    println!("✅ Client synced with network\n");

    // Handle owner account - either use existing or create new
    let (owner_id, owner_account, owner_seed) = if let Some(existing_id) = owner_id_str {
        println!("🔍 Using existing owner account...");
        if existing_id.starts_with("mm1") || existing_id.starts_with("mtst1") || existing_id.starts_with("mdev1") {
            println!("   Detected bech32 format, converting to AccountId...");
        }
        let owner_id = parse_account_id(existing_id)?;
        println!("   ✓ Owner AccountId: {}\n", owner_id);

        let owner_record = client.get_account(owner_id).await?;
        let owner_account = if let Some(record) = owner_record {
            println!("✅ Owner account found in local database\n");
            record.account().clone()
        } else {
            return Err(format!(
                "❌ Owner account {} not found in local database!\n\
                 The owner account must be imported locally to sign the initialization note.",
                owner_id
            ).into());
        };

        (owner_id, owner_account, None)
    } else {
        println!("🔑 Creating new owner account...\n");

        use midenid_contracts::common::create_basic_account;
        use miden_client::keystore::FilesystemKeyStore;

        let keystore = FilesystemKeyStore::new("./keystore".into())?;
        let (account, _key_pair) = create_basic_account(&mut client, keystore).await?;

        println!("✅ New owner account created!\n");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📋 Owner Account Details:\n");
        println!("   Account ID (hex):     {}", account.id().to_hex());
        println!("   Account ID (decimal): {}", account.id());
        println!("   Storage Mode:         {:?}", account.id().storage_mode());
        println!("   Account Type:         {:?}\n", account.id().account_type());

        // Get the seed for backup
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
            println!("⚠️  WARNING: This seed gives full control of the account!");
            println!("   Store it securely - you'll need it to recover the account.\n");
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        (account.id(), account, seed)
    };

    println!("📋 Final Configuration:");
    println!("   Network:         {}", network_str.to_uppercase());
    println!("   Owner Account:   {}", owner_id);
    println!("   Payment Token:   {}", payment_token_id);
    println!("   Price:           {} tokens\n", price);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 1: Deploy Registry Contract");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm"))?;
    println!("📄 Loading contract: ./masm/accounts/miden_id.masm");

    let (registry_contract, registry_seed) =
        create_public_immutable_contract(&mut client, &registry_code).await?;

    client
        .add_account(&registry_contract, Some(registry_seed), false)
        .await?;

    println!("✅ Registry contract deployed!");
    println!("   ID: {}", registry_contract.id());
    println!("   Type: Public, Immutable\n");

    sleep(Duration::from_secs(5)).await;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Initialize Registry");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("⚙️  Initialization:");
    println!("   Owner: {}", owner_id);
    println!("   Payment Token: {}", payment_token_id);
    println!("   Price: {} tokens\n", price);

    // Read init note code from file
    let init_note_code = fs::read_to_string(Path::new("./masm/notes/init.masm"))?;

    let library_namespace = "miden_id::registry";
    let contract_library = create_library(registry_code.clone(), library_namespace)?;
    let empty_assets = NoteAssets::new(vec![])?;

    // Pass initialization parameters as note inputs
    let token_prefix = payment_token_id.prefix().as_felt();
    let token_suffix = payment_token_id.suffix();
    let inputs = NoteInputs::new(vec![token_prefix, token_suffix, Felt::new(price)])?;

    let init_note = create_public_note_with_library_and_inputs(
        &mut client,
        init_note_code,
        owner_account.clone(),
        empty_assets,
        contract_library,
        inputs,
    )
    .await?;

    sleep(Duration::from_secs(5)).await;

    let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
    let transaction_script = create_tx_script(nop_script_code, None)?;

    let request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(init_note, None)])
        .custom_script(transaction_script)
        .build()?;

    let tx_result = client.new_transaction(registry_contract.id(), request).await?;
    client.submit_transaction(tx_result).await?;

    println!("✅ Registry initialized!\n");

    sleep(Duration::from_secs(15)).await;
    client.sync_state().await?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 4: Verify Deployment");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let contract_record = client.get_account(registry_contract.id()).await?;
    if let Some(record) = contract_record {
        let storage = record.account().storage();
       
        let initialized = storage.get_item(0).unwrap().get(3).unwrap().as_int();
        let owner_word = storage.get_item(1).unwrap();
        let payment_token_word = storage.get_item(2).unwrap();
        let price_word = storage.get_item(5).unwrap();

        println!("✅ Registry State:");
        println!("   Initialized: {} (raw value: {})", if initialized == 1 { "✓" } else { "✗" }, initialized);
        println!("   Owner Prefix: {} (expected: {})",
            owner_word.get(0).unwrap().as_int(),
            owner_id.prefix().as_felt().as_int());
        println!("   Owner Suffix: {} (expected: {})",
            owner_word.get(1).unwrap().as_int(),
            owner_id.suffix().as_int());
        println!("   Payment Token Prefix: {} (expected: {})",
            payment_token_word.get(1).unwrap().as_int(),
            payment_token_id.prefix().as_felt().as_int());
        println!("   Payment Token Suffix: {} (expected: {})",
            payment_token_word.get(0).unwrap().as_int(),
            payment_token_id.suffix().as_int());
        println!("   Price: {} tokens (expected: {})\n",
            price_word.get(0).unwrap().as_int(),
            price);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 Deployment Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📋 Summary:\n");
    println!("   Registry Contract:  {}", registry_contract.id());
    println!("   Owner Account:      {}", owner_id);
    println!("   Payment Token:      {}", payment_token_id);
    println!("   Registration Price: {} tokens\n", price);

    if let Some(ref seed_value) = owner_seed {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🔐 OWNER ACCOUNT SEED (BACKUP THIS!):\n");
        println!("   {}\n", seed_value);
        println!("⚠️  IMPORTANT: Store this seed securely!");
        println!("   You need it to recover the owner account.\n");
    }

    // Export deployment info to file
    use std::io::Write;
    let deployment_dir = Path::new("./deployments");
    fs::create_dir_all(deployment_dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let registry_id_short = &registry_contract.id().to_hex()[2..12]; // First 10 hex chars after 0x
    let filename = format!("{}_{}.txt", timestamp, registry_id_short);
    let filepath = deployment_dir.join(&filename);

    let mut file = fs::File::create(&filepath)?;

    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
    writeln!(file, "MIDEN ID REGISTRY DEPLOYMENT RECORD")?;
    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n")?;

    writeln!(file, "Deployment Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(file, "Network:         {}\n", network_str.to_uppercase())?;

    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
    writeln!(file, "REGISTRY CONTRACT")?;
    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n")?;

    writeln!(file, "Registry ID (hex):     {}", registry_contract.id().to_hex())?;
    writeln!(file, "Registry ID (decimal): {}\n", registry_contract.id())?;

    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
    writeln!(file, "CONFIGURATION")?;
    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n")?;

    writeln!(file, "Owner Account (hex):     {}", owner_id.to_hex())?;
    writeln!(file, "Owner Account (decimal): {}", owner_id)?;
    writeln!(file, "Payment Token (hex):     {}", payment_token_id.to_hex())?;
    writeln!(file, "Payment Token (decimal): {}", payment_token_id)?;
    writeln!(file, "Registration Price:      {} tokens\n", price)?;

    if let Some(ref seed_value) = owner_seed {
        writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(file, "OWNER ACCOUNT SEED (KEEP SECRET!)")?;
        writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n")?;
        writeln!(file, "{}\n", seed_value)?;
        writeln!(file, "⚠️  WARNING: This seed gives full control of the owner account!")?;
        writeln!(file, "   Store it securely and never share it.\n")?;
    }

    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
    writeln!(file, "FRONTEND INTEGRATION")?;
    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n")?;
    writeln!(file, "Use these values in your frontend configuration:\n")?;
    writeln!(file, "REGISTRY_CONTRACT_ID=\"{}\"", registry_contract.id().to_hex())?;
    writeln!(file, "PAYMENT_TOKEN_ID=\"{}\"", payment_token_id.to_hex())?;
    writeln!(file, "REGISTRATION_PRICE={}", price)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💾 Deployment info saved to: {}\n", filepath.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}
