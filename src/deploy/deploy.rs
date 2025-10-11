use miden_client::{account::AccountId, rpc::Endpoint, transaction::TransactionRequestBuilder};
use miden_objects::{
    Felt,
    address::Address,
    note::{NoteAssets, NoteInputs},
};
use midenid_contracts::common::{
    create_library, create_public_immutable_contract, create_public_note_with_library_and_inputs,
    create_tx_script, instantiate_client,
};
use std::{env, fs, path::Path};
use tokio::time::{Duration, sleep};

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
            Address::AccountId(account_id_address) => Ok(account_id_address.id()),
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
    println!("\nMiden ID Registry Deployment\n");

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Error: Missing required parameters\n");
        eprintln!(
            "Usage: {} <network> <payment_token_id> <price> [owner_account_id]\n",
            args[0]
        );
        eprintln!("Examples:");
        eprintln!("  {} testnet 0x97598f759deab5201e93e1aac55997 10", args[0]);
        eprintln!(
            "  {} testnet 0x97598f759deab5201e93e1aac55997 10 0x1c89546e3b82cd1012a9fe4853bc68\n",
            args[0]
        );
        eprintln!("Parameters:");
        eprintln!("  network           - devnet, testnet, or mainnet");
        eprintln!("  payment_token_id  - Faucet/token ID (hex or bech32)");
        eprintln!("  price             - Registration price in tokens");
        eprintln!("  owner_account_id  - (Optional) Existing account ID\n");
        std::process::exit(1);
    }

    let network_str = &args[1];
    let payment_token_id_str = &args[2];
    let price: u64 = args[3]
        .parse()
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
        }
        _ => {
            return Err(format!(
                "Invalid network: '{}'. Must be one of: devnet, testnet, mainnet",
                network_str
            )
            .into());
        }
    };

    println!("Network: {}", network_str);

    let payment_token_id = parse_account_id(payment_token_id_str)?;
    println!("Payment Token: {}", payment_token_id);

    let mut client = instantiate_client(endpoint.clone()).await?;
    client.sync_state().await?;
    println!("Client synced\n");

    // Handle owner account - either use existing or create new
    let (owner_id, owner_account, owner_seed) = if let Some(existing_id) = owner_id_str {
        let owner_id = parse_account_id(existing_id)?;
        println!("Using existing owner: {}", owner_id);

        let owner_record = client.get_account(owner_id).await?;
        let owner_account = if let Some(record) = owner_record {
            record.account().clone()
        } else {
            return Err(format!("Owner account {} not found in local database", owner_id).into());
        };

        (owner_id, owner_account, None)
    } else {
        println!("Creating new owner account...");

        use miden_client::keystore::FilesystemKeyStore;
        use midenid_contracts::common::create_basic_account;

        let keystore = FilesystemKeyStore::new("./keystore".into())?;
        let (account, _key_pair) = create_basic_account(&mut client, keystore).await?;

        let account_record = client.get_account(account.id()).await?;
        let seed = if let Some(record) = account_record {
            record.seed().cloned()
        } else {
            None
        };

        println!("Owner created: {}", account.id());

        if let Some(ref seed_value) = seed {
            println!("\n⚠️  ACCOUNT SEED (BACKUP THIS!):");
            println!("{}", seed_value);
            println!("WARNING: This seed gives full control of the account!\n");
        }

        (account.id(), account, seed)
    };

    println!("\n[1/3] Deploying contract...");

    let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm"))?;

    let (registry_contract, registry_seed) =
        create_public_immutable_contract(&mut client, &registry_code).await?;

    client
        .add_account(&registry_contract, Some(registry_seed), false)
        .await?;

    println!("Registry deployed: {}\n", registry_contract.id());

    sleep(Duration::from_secs(5)).await;

    println!("[2/3] Initializing registry...");

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

    let tx_result = client
        .new_transaction(registry_contract.id(), request)
        .await?;
    client.submit_transaction(tx_result).await?;

    println!("Registry initialized\n");

    sleep(Duration::from_secs(15)).await;
    client.sync_state().await?;

    println!("[3/3] Verifying deployment...");

    let contract_record = client.get_account(registry_contract.id()).await?;
    if let Some(record) = contract_record {
        let storage = record.account().storage();
        let initialized = storage.get_item(0).unwrap().get(3).unwrap().as_int();
        let price_stored = storage.get_item(5).unwrap().get(0).unwrap().as_int();

        if initialized == 1 && price_stored == price {
            println!("Deployment verified\n");
        } else {
            eprintln!(
                "Warning: State mismatch (initialized: {}, price: {})",
                initialized, price_stored
            );
        }
    }

    println!("Deployment Complete\n");
    println!("Registry:  {}", registry_contract.id());
    println!("Owner:     {}", owner_id);
    println!("Token:     {}", payment_token_id);
    println!("Price:     {} tokens\n", price);

    if let Some(ref seed_value) = owner_seed {
        println!("⚠️  OWNER SEED (BACKUP!):");
        println!("{}\n", seed_value);
    }

    // Export deployment info to file
    use std::io::Write;
    let deployment_dir = Path::new("./deployments");
    fs::create_dir_all(deployment_dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let registry_id_short = &registry_contract.id().to_hex()[2..12];
    let filename = format!("{}_{}.txt", timestamp, registry_id_short);
    let filepath = deployment_dir.join(&filename);

    let mut file = fs::File::create(&filepath)?;

    writeln!(file, "Miden ID Registry Deployment\n")?;
    writeln!(
        file,
        "Date: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )?;
    writeln!(file, "Network: {}\n", network_str)?;
    writeln!(file, "Registry ID: {}", registry_contract.id().to_hex())?;
    writeln!(file, "Owner ID: {}", owner_id.to_hex())?;
    writeln!(file, "Payment Token: {}", payment_token_id.to_hex())?;
    writeln!(file, "Price: {} tokens\n", price)?;

    if let Some(ref seed_value) = owner_seed {
        writeln!(file, "OWNER SEED (KEEP SECRET!):\n{}\n", seed_value)?;
        writeln!(
            file,
            "WARNING: This seed gives full control of the owner account!\n"
        )?;
    }

    writeln!(file, "Frontend Config:\n")?;
    writeln!(
        file,
        "REGISTRY_CONTRACT_ID=\"{}\"",
        registry_contract.id().to_hex()
    )?;
    writeln!(file, "PAYMENT_TOKEN_ID=\"{}\"", payment_token_id.to_hex())?;
    writeln!(file, "REGISTRATION_PRICE={}", price)?;

    println!("Saved to: {}\n", filepath.display());

    Ok(())
}
