use miden_client::ScriptBuilder;
use miden_client::asset::FungibleAsset;
use miden_client::note::{NoteId, NoteType};
use miden_client::{
    Client,
    builder::ClientBuilder,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, GrpcClient},
};
use miden_client::{
    Felt, Word,
    account::{
        Account, AccountBuilder, AccountId, AccountStorageMode, AccountType,
        component::BasicFungibleFaucet,
    },
    address::NetworkId,
    auth::{AuthRpoFalcon512, AuthSecretKey},
    note::{NoteAssets, NoteInputs},
    transaction::{OutputNote, TransactionRequestBuilder},
};
use miden_client_sqlite_store::ClientBuilderSqliteExt;
use miden_lib::note::create_p2id_note;
use miden_objects::asset::TokenSymbol;
use midenname_contracts::accounts::{create_deployer_account, create_naming_account};
use midenname_contracts::domain::encode_domain;
use midenname_contracts::scripts::deploy;
use midenname_contracts::{notes::create_note_for_naming, transaction::wait_for_tx};
use rand::Rng;
use rand::RngCore;
use rand::rngs::StdRng;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// HOW TO RUN
// cargo run --bin local_deploy_and_register 2>&1 | tee output.log
// HOW TO RUN

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment...");
    println!("=================================================");
    println!("Deleting existing store & keystore (store.sqlite3)");
    let _ = std::fs::remove_file("store.sqlite3");
    let _ = std::fs::remove_dir("keystore");
    println!("Deletion complete.");
    println!("=================================================");

    let mut keystore = midenname_contracts::client::create_keystore()?;

    // Initiate local client / we can initiate testnet client by using initiate_client function
    let mut client = initiate_local_client(keystore.clone()).await?;

    // Define all account IDs here
    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client, false).await?;

    // deploy contracts
    deploy(
        &mut client,
        &mut keystore,
        deployer_account.id(),
        naming_account.id(),
    )
    .await?;

    // Create 70 accounts
    let all_accounts = create_multiple_accounts(&mut client, &mut keystore, 2).await?;

    // Deploy a fungible faucet
    let faucet_id = deploy_fungible_faucet(
        &mut client,
        &mut keystore,
        "TRY",
        6,
        100_000_000_000_000_000,
    )
    .await?;

    // Fund all created accounts from the faucet
    fund_account(&mut client, faucet_id, all_accounts.clone()).await?;

    // Consume funding notes for all created accounts
    let total_accounts = all_accounts.len();
    for (index, account) in all_accounts.clone().iter().enumerate() {
        consume_funding_note(&mut client, account.id(), index + 1, total_accounts).await?;
    }

    // Send and consume set price note
    sending_and_consuming_set_price_note(
        &mut client,
        faucet_id,
        deployer_account.id(),
        naming_account.id(),
        false,
    )
    .await?;

    // Send and consume register notes for all created accounts
    send_register_note(
        &mut client,
        naming_account.id(),
        all_accounts.clone(),
        faucet_id,
    )
    .await?;

    // Check if any consumable notes are left for the naming account
    find_consumable_notes(&mut client, naming_account.id()).await?;

    Ok(())
}

pub async fn initiate_local_client(
    keystore: Arc<FilesystemKeyStore<StdRng>>,
) -> anyhow::Result<Client<FilesystemKeyStore<StdRng>>> {
    const TIMEOUT: u64 = 10_000;

    let endpoint = Endpoint::new("http".into(), "0.0.0.0".into(), Some(57291));

    let rpc_client = Arc::new(GrpcClient::new(&endpoint, TIMEOUT));

    let store_path = std::path::PathBuf::from("./store.sqlite3");

    let mut client = ClientBuilder::new()
        .rpc(rpc_client)
        .sqlite_store(store_path)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .build()
        .await?;

    let sync_summary = client.sync_state().await.unwrap();
    println!("Latest block: {}", sync_summary.block_num);
    Ok(client)
}

async fn create_multiple_accounts(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    keystore: &mut Arc<FilesystemKeyStore<StdRng>>,
    count: usize,
) -> anyhow::Result<Vec<Account>> {
    println!("Creating {} accounts...", count);
    println!("=================================================");
    let mut created_accounts = Vec::new();

    for i in 0..count {
        let account = create_deployer_account(client, keystore).await?;
        created_accounts.push(account.clone());
        println!(
            "✅ Created account {}/{}: {:?}",
            i + 1,
            count,
            account.id().to_hex()
        );
    }

    println!("=================================================");
    println!("🎉 Successfully created all {} accounts!", count);
    Ok(created_accounts)
}

async fn deploy_fungible_faucet(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    keystore: &mut Arc<FilesystemKeyStore<StdRng>>,
    symbol: &str,
    decimals: u8,
    max_supply: u64,
) -> anyhow::Result<AccountId> {
    println!("\n[Deploying Fungible Faucet]");
    println!("=================================================");

    // Faucet seed
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    // Faucet parameters
    let token_symbol = TokenSymbol::new(symbol).unwrap();
    let max_supply_felt = Felt::new(max_supply);

    // Generate key pair
    let key_pair = AuthSecretKey::new_rpo_falcon512();

    // Build the faucet account
    let faucet_account = AccountBuilder::new(init_seed)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key().to_commitment()))
        .with_component(BasicFungibleFaucet::new(token_symbol, decimals, max_supply_felt).unwrap())
        .build()
        .unwrap();

    // Add the faucet to the client
    client.add_account(&faucet_account, false).await?;

    // Add the key pair to the keystore
    keystore.add_key(&key_pair).unwrap();

    let faucet_account_id = faucet_account.id();
    let faucet_account_id_bech32 = faucet_account_id.to_bech32(NetworkId::Devnet);
    println!("Faucet account ID: {:?}", faucet_account_id_bech32);
    println!(
        "Symbol: {}, Decimals: {}, Max Supply: {}",
        symbol, decimals, max_supply
    );

    // Resync to show newly deployed faucet
    client.sync_state().await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("=================================================");
    println!("✅ Faucet deployed successfully!");

    Ok(faucet_account_id)
}

async fn fund_account(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    faucet_id: AccountId,
    target_accounts: Vec<Account>,
) -> anyhow::Result<()> {
    println!("\n[Funding Account]");
    println!("=================================================");

    let amount: u64 = 1_000_000;

    let fungible_asset = FungibleAsset::new(faucet_id, amount).unwrap();

    let total_accounts = target_accounts.len();
    for (index, account) in target_accounts.clone().iter().enumerate() {
        sleep(Duration::from_secs(6)).await;
        println!(
            "Funding account {}/{}: {:?}",
            index + 1,
            total_accounts,
            account.id().to_hex()
        );
        let transaction_request = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(
                fungible_asset.clone(),
                account.id(),
                NoteType::Public,
                client.rng(),
            )
            .unwrap();

        println!("Funding TX request built.");

        client.sync_state().await?;

        sleep(Duration::from_secs(6)).await;

        let tx_id = client
            .submit_new_transaction(faucet_id, transaction_request)
            .await?;

        wait_for_tx(client, tx_id).await?;

        client.sync_state().await?;

        println!(
            "Submitted funding transaction: {:?} \nAccount {}/{}: {:?}",
            tx_id,
            index + 1,
            total_accounts,
            account.id().to_hex()
        );
        println!("=================================================");
    }

    println!("\nFunding completed.");
    Ok(())
}

async fn consume_funding_note(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    target_account: AccountId,
    account_index: usize,
    total_accounts: usize,
) -> anyhow::Result<()> {
    println!(
        "\n[Consuming Funding Note For Account {}/{}: {:?}]",
        account_index,
        total_accounts,
        target_account.to_hex()
    );
    println!("=================================================");

    client.sync_state().await?;

    sleep(Duration::from_secs(6)).await;

    loop {
        // Resync to get the latest data
        client.sync_state().await?;

        let consumable_notes = client.get_consumable_notes(Some(target_account)).await?;
        let list_of_note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();

        if list_of_note_ids.len() == 1 {
            println!("Found 1 consumable note. Consuming it now...");
            let transaction_request = TransactionRequestBuilder::new()
                .build_consume_notes(list_of_note_ids)
                .unwrap();

            let tx_id = client
                .submit_new_transaction(target_account, transaction_request)
                .await?;

            wait_for_tx(client, tx_id).await?;

            println!(
                "✅ Account {}/{} - Note consumed successfully. TX: {:?}",
                account_index, total_accounts, tx_id
            );

            return anyhow::Ok(());
        } else {
            println!(
                "Currently, {} number of consumable notes. Waiting...",
                list_of_note_ids.len()
            );
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}

async fn sending_and_consuming_set_price_note(
    mut client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    payment_token_id: AccountId,
    deployer_account: AccountId,
    naming_account: AccountId,
    is_network: bool,
) -> anyhow::Result<()> {
    println!("\n[Setting prices]");
    println!("=================================================");
    println!("Set price note creation started");

    let set_prices_note_inputs = NoteInputs::new(
        [
            Felt::new(payment_token_id.suffix().into()),
            Felt::new(payment_token_id.prefix().into()),
        ]
        .to_vec(),
    )?;

    let set_prices_note = create_note_for_naming(
        "set_all_prices".to_string(),
        set_prices_note_inputs,
        deployer_account,
        naming_account,
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(set_prices_note.clone())])
        .build()?;

    let set_prices_tx_id = client
        .submit_new_transaction(deployer_account, set_price_req)
        .await?;

    println!("Submitted set price note transaction.");

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        set_prices_tx_id
    );
    client.sync_state().await?;

    println!("set prices tx submitted, waiting for onchain commitment");

    println!("=================================================");

    wait_for_tx(&mut client, set_prices_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    if is_network {
        sleep(Duration::from_secs(6)).await;

        println!("Consuming pricing notes...");

        let nop_script_code =
            std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
        let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

        let consume_request = TransactionRequestBuilder::new()
            .authenticated_input_notes(vec![(set_prices_note.id(), None)])
            .custom_script(transaction_script)
            .build()?;

        let consume_tx_id = client
            .submit_new_transaction(naming_account, consume_request)
            .await?;
        println!(
            "Consuming pricing notes via transaction: {:?}",
            consume_tx_id
        );

        wait_for_tx(&mut client, consume_tx_id).await?;
    }

    println!("✅ Notes pricing successfully!");
    Ok(())
}

async fn send_register_note(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    naming_account: AccountId,
    generated_accounts: Vec<Account>,
    faucet_id: AccountId,
) -> anyhow::Result<()> {
    println!("\n[Sending register notes]");
    println!("=================================================");

    let mut rng = rand::rng();

    for account in generated_accounts {
        println!(
            "Creating register note for account: {:?}",
            account.id().to_hex()
        );

        let mut s = String::with_capacity(5);
        for _ in 0..5 {
            let digit = rng.random_range(0..10);
            s.push(char::from(b'0' + digit as u8));
        }

        println!("Generated domain string: {}", s);

        let domain = encode_domain(s);

        println!("Encoded domain: {:?}", domain);

        let fungible_asset = FungibleAsset::new(faucet_id, 1).unwrap();
        let register_note_inputs = NoteInputs::new(
            [
                Felt::new(faucet_id.suffix().as_int()),
                faucet_id.prefix().as_felt(),
                Felt::new(0),
                Felt::new(0),
                domain[0],
                domain[1],
                domain[2],
                domain[3],
            ]
            .to_vec(),
        )?;

        let register_asset = NoteAssets::new(vec![fungible_asset.into()])?;

        let register_note = create_note_for_naming(
            "register_name".to_string(),
            register_note_inputs,
            account.id(),
            naming_account,
            register_asset,
        )
        .await?;

        let register_note_req = TransactionRequestBuilder::new()
            .own_output_notes(vec![OutputNote::Full(register_note.clone())])
            .build()?;

        let register_note_tx_id = client
            .submit_new_transaction(account.id(), register_note_req)
            .await?;

        wait_for_tx(client, register_note_tx_id).await?;

        sleep(Duration::from_secs(6)).await;

        client.sync_state().await?;

        let note_id = register_note.id();

        let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
        let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

        let consume_request = TransactionRequestBuilder::new()
            .authenticated_input_notes(vec![(note_id, None)])
            .custom_script(transaction_script)
            .build()?;

        let consume_tx_id = client
            .submit_new_transaction(naming_account, consume_request)
            .await?;
        println!("📝 Consuming notes via transaction: {:?}", consume_tx_id);

        wait_for_tx(client, consume_tx_id).await?;

        sleep(Duration::from_secs(6)).await;

        client.sync_state().await?;
    }

    // Implementation for sending register note goes here

    Ok(())
}

async fn find_consumable_notes(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    naming_account: AccountId,
) -> anyhow::Result<()> {
    use tokio::time::{Duration, sleep};

    println!("Finding notes...");

    let max_attempts = 50;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("\n🔍 Attempt {}/{}", attempt, max_attempts);

        let consumable_notes = client.get_consumable_notes(Some(naming_account)).await?;

        if !consumable_notes.is_empty() {
            println!("✅ Found {} consumable note(s)", consumable_notes.len());

            let note_ids: Vec<_> = consumable_notes
                .iter()
                .map(|(record, _)| (record.id(), None))
                .collect();

            let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
            let transaction_script =
                ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

            let consume_request = TransactionRequestBuilder::new()
                .authenticated_input_notes(note_ids)
                .custom_script(transaction_script)
                .build()?;

            let consume_tx_id = client
                .submit_new_transaction(naming_account, consume_request)
                .await?;
            println!("📝 Consuming notes via transaction: {:?}", consume_tx_id);

            wait_for_tx(client, consume_tx_id).await?;
            println!("✅ Notes consumed successfully!");
            return Ok(());
        } else {
            println!("⏳ No consumable notes found yet...");

            if attempt >= max_attempts {
                println!(
                    "❌ Max attempts ({}) reached. No consumable notes found.",
                    max_attempts
                );
                return Ok(());
            }

            println!("⏰ Waiting 2 seconds before retry...");
            sleep(Duration::from_secs(2)).await;
        }
    }
}

async fn _storage_check(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    naming_account: AccountId,
    registered_account: AccountId,
) -> anyhow::Result<()> {
    let storage_key = encode_domain("6".to_string());

    println!("storage key: {:?}", storage_key);

    let get_account = client.get_account(naming_account).await?;

    let storage = get_account
        .as_ref()
        .unwrap()
        .account()
        .storage()
        .get_map_item(5, storage_key);

    println!("Storage item: {:?}", storage.unwrap());
    let id_storage_key = Word::new([
        Felt::new(registered_account.suffix().as_int()),
        Felt::new(registered_account.prefix().as_u64()),
        Felt::new(0),
        Felt::new(0),
    ]);

    println!("id_storage_key: {:?}", id_storage_key);

    let id_storage = get_account
        .as_ref()
        .unwrap()
        .account()
        .storage()
        .get_map_item(3, id_storage_key);

    println!("Storage item: {:?}", id_storage);

    return Ok(());
}

async fn _consume_single_note(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    naming_account: AccountId,
) -> anyhow::Result<()> {
    let note =
        NoteId::try_from_hex("0x70ea026678980c2fe5ade933171df2982cf4882a589eee3cb28c386c608613b8")?;

    println!("Consuming note: {:?}", note);

    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(vec![note])
        .unwrap();

    let tx_id = client
        .submit_new_transaction(naming_account, transaction_request)
        .await?;

    println!("Submitted transaction to consume note: {:?}", tx_id);

    //tx with authenticated_input_notes nop script

    // let nop_script_code = std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
    // let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

    // println!("111111111111111111:");

    // let consume_request = TransactionRequestBuilder::new()
    //     .authenticated_input_notes(vec![(note, None)])
    //     .custom_script(transaction_script)
    //     .build()?;

    // println!("2222222222222222222:");

    // let consume_tx_id = client
    //     .submit_new_transaction(naming_account, consume_request)
    //     .await?;

    // println!("Consuming notes via transaction: {:?}", consume_tx_id);

    // wait_for_tx(client, consume_tx_id).await?;
    // println!("✅ Notes consumed successfully!");

    return Ok(());
}

async fn _send_init_note(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    naming_account: AccountId,
    deployer_account: AccountId,
) -> anyhow::Result<()> {
    use tokio::time::{Duration, sleep};

    let initialize_inputs = NoteInputs::new(
        [
            Felt::new(deployer_account.suffix().into()),
            Felt::new(deployer_account.prefix().into()),
            Felt::new(0),
            Felt::new(0),
            Felt::new(5000),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;

    let init_note = create_note_for_naming(
        "initialize_naming".to_string(),
        initialize_inputs,
        deployer_account,
        naming_account,
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note)])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account, init_req)
        .await?;

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );
    client.sync_state().await?;

    println!("naming initialize note creation tx submitted, waiting for onchain commitment");

    wait_for_tx(client, init_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    Ok(())
}

async fn _send_asset_to_generated_accounts(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    faucet_id: AccountId,
    sender: AccountId,
    generated_accounts: Vec<Account>,
) -> anyhow::Result<()> {
    println!("\n[Sending Assets to Generated Accounts]");
    println!("=================================================");

    let amount_per_account: u64 = 100_000;

    let fungible_asset = FungibleAsset::new(faucet_id, amount_per_account).unwrap();

    for account in generated_accounts.clone() {
        println!(
            "Creating P2ID note for account: {:?}",
            account.id().to_hex()
        );

        let p2id_note = create_p2id_note(
            sender,
            account.id(),
            vec![fungible_asset.into()],
            NoteType::Public,
            Felt::new(0),
            client.rng(),
        )?;

        println!(
            "P2ID note created for account: {:?} with note_id: {:?}",
            account.id().to_hex(),
            p2id_note.id()
        );

        let transaction_request = TransactionRequestBuilder::new()
            .build_consume_notes(vec![p2id_note.id()])
            .unwrap();

        let tx_id = client
            .submit_new_transaction(account.id(), transaction_request)
            .await?;

        wait_for_tx(client, tx_id).await?;
    }

    for account in generated_accounts {
        let amount = account.vault().get_balance(faucet_id)?;
        println!(
            "Account {:?} balance for faucet asset: {}",
            account.id().to_hex(),
            amount
        );
    }

    println!("=================================================");
    println!("✅ Assets sent to all generated accounts successfully!");

    Ok(())
}

async fn _safe_account_import(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    account_id: AccountId,
) -> anyhow::Result<()> {
    if client.get_account(account_id).await?.is_none() {
        match client.import_account_by_id(account_id).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Warning: Failed to import account: {:?}", e);
            }
        }
    }
    Ok(())
}
