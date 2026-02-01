use miden_client::{
    account::AccountId,
    asset::FungibleAsset,
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets, NoteFile, NoteInputs},
    store::NoteFilter,
    transaction::{OutputNote, TransactionRequestBuilder},
    Client,
};
use miden_crypto::Felt;
use miden_standards::code_builder::CodeBuilder;
use tokio::time::{Duration, sleep};

use crate::{
    accounts::{create_deployer_account, create_naming_account},
    client::{create_keystore, initiate_client},
    domain::encode_domain_as_felts,
    notes::create_note_for_naming,
    transaction::wait_for_tx,
};

pub async fn deploy() -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment...");
    let mut keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client).await?;
    client.sync_state().await?;

    let initialize_inputs = NoteInputs::new(
        [
            Felt::new(deployer_account.id().suffix().into()),
            Felt::new(deployer_account.id().prefix().into()),
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
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    let init_note_id = init_note.id();

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note)])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account.id(), init_req)
        .await?;

    println!(
        "View transaction on MidenScan: https://devnet.midenscan.com/tx/{:?}",
        init_tx_id
    );
    client.sync_state().await?;

    println!("naming initialize note creation tx submitted, waiting for onchain commitment");

    wait_for_tx(&mut client, init_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    println!("Setting prices");

    let payment_token_id = AccountId::from_hex("0x8b043136e8426720729a83b33f95a6")?;

    let set_prices_note_inputs = NoteInputs::new(
        [
            Felt::new(payment_token_id.suffix().into()),
            Felt::new(payment_token_id.prefix().into()),
        ]
        .to_vec(),
    )?;

    let set_prices_note = create_note_for_naming(
        "set_all_prices_testnet".to_string(),
        set_prices_note_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    let set_prices_note_id = set_prices_note.id();

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(set_prices_note)])
        .build()?;

    let set_prices_tx_id = client
        .submit_new_transaction(deployer_account.id(), set_price_req)
        .await?;

    println!(
        "View transaction on MidenScan: https://devnet.midenscan.com/tx/{:?}",
        set_prices_tx_id
    );
    client.sync_state().await?;

    println!("set prices tx submitted, waiting for onchain commitment");

    wait_for_tx(&mut client, set_prices_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    // Import and consume notes in separate transactions (init must run first to set owner,
    // then set_prices checks owner). We import by ID to bypass the NoteScreener which may
    // not handle custom note scripts with dynamic library linking.
    let nop_script_code =
        std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;

    // Step 1: Consume init note (sets owner)
    println!("Importing and consuming init note...");
    client.import_notes(&[NoteFile::NoteId(init_note_id)]).await?;
    client.sync_state().await?;

    let init_input = client
        .get_input_notes(NoteFilter::List(vec![init_note_id]))
        .await?;
    let init_notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = init_input
        .into_iter()
        .map(|record| {
            let note: Note = record.try_into().unwrap();
            (note, None)
        })
        .collect();

    let init_consume_req = TransactionRequestBuilder::new()
        .input_notes(init_notes)
        .custom_script(CodeBuilder::default().compile_tx_script(nop_script_code.clone())?)
        .build()?;

    let init_consume_tx = client
        .submit_new_transaction(naming_account.id(), init_consume_req)
        .await?;
    println!("Init note consume tx: {:?}", init_consume_tx);
    wait_for_tx(&mut client, init_consume_tx).await?;
    println!("Init note consumed!");

    sleep(Duration::from_secs(6)).await;
    client.sync_state().await?;

    // Step 2: Consume set_prices note (requires owner to be set)
    println!("Importing and consuming set_prices note...");
    client.import_notes(&[NoteFile::NoteId(set_prices_note_id)]).await?;
    client.sync_state().await?;

    let prices_input = client
        .get_input_notes(NoteFilter::List(vec![set_prices_note_id]))
        .await?;
    let prices_notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = prices_input
        .into_iter()
        .map(|record| {
            let note: Note = record.try_into().unwrap();
            (note, None)
        })
        .collect();

    let prices_consume_req = TransactionRequestBuilder::new()
        .input_notes(prices_notes)
        .custom_script(CodeBuilder::default().compile_tx_script(nop_script_code)?)
        .build()?;

    let prices_consume_tx = client
        .submit_new_transaction(naming_account.id(), prices_consume_req)
        .await?;
    println!("Set prices note consume tx: {:?}", prices_consume_tx);
    wait_for_tx(&mut client, prices_consume_tx).await?;
    println!("Prices set successfully!");

    Ok(())
}

async fn mint_from_faucet(
    client: &mut Client<FilesystemKeyStore>,
    target_account_id: AccountId,
    amount: u64,
) -> anyhow::Result<()> {
    let faucet_api = "https://faucet-api.devnet.miden.io";
    let account_hex = target_account_id.to_hex();

    println!("Requesting {} tokens from devnet faucet for {}...", amount, account_hex);

    let http_client = reqwest::Client::new();

    // Step 1: Get PoW challenge
    let pow_resp: serde_json::Value = http_client
        .get(format!("{}/pow?account_id={}&amount={}", faucet_api, account_hex, amount))
        .send()
        .await?
        .json()
        .await?;

    let challenge = pow_resp["challenge"].as_str().unwrap().to_string();
    let target = pow_resp["target"].as_u64().unwrap();

    println!("PoW challenge received, target: {}, solving...", target);

    // Step 2: Solve PoW - SHA-256(challenge || nonce_be) with big-endian comparison
    use sha2::{Sha256, Digest};
    use rand::Rng;
    let challenge_bytes = hex::decode(&challenge)?;
    let mut rng = rand::rng();
    let nonce: u64 = loop {
        let n: u64 = rng.random();
        let mut hasher = Sha256::new();
        hasher.update(&challenge_bytes);
        hasher.update(&n.to_be_bytes());
        let hash = hasher.finalize();
        let digest = u64::from_be_bytes(hash[0..8].try_into().unwrap());
        if digest < target {
            break n;
        }
    };
    println!("PoW solved! Nonce: {}", nonce);

    // Step 3: Request tokens
    let tokens_resp = http_client
        .get(format!(
            "{}/get_tokens?account_id={}&asset_amount={}&challenge={}&nonce={}&is_private_note=false",
            faucet_api, account_hex, amount, challenge, nonce
        ))
        .send()
        .await?;

    let tokens_text = tokens_resp.text().await?;
    println!("Faucet response: {}", tokens_text);

    // Step 4: Wait for the faucet note to appear and consume it
    println!("Waiting for faucet note to be committed...");
    sleep(Duration::from_secs(10)).await;

    for _ in 0..10 {
        client.sync_state().await?;
        let consumable = client
            .get_consumable_notes(Some(target_account_id))
            .await?;
        if !consumable.is_empty() {
            println!("Found {} faucet note(s), consuming...", consumable.len());

            let notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = consumable
                .into_iter()
                .map(|(record, _)| {
                    let note: Note = record.try_into().unwrap();
                    (note, None)
                })
                .collect();

            let consume_req = TransactionRequestBuilder::new()
                .input_notes(notes)
                .build()?;

            let consume_tx = client
                .submit_new_transaction(target_account_id, consume_req)
                .await?;

            wait_for_tx(client, consume_tx).await?;
            println!("Faucet tokens received!");
            return Ok(());
        }
        println!("No faucet notes yet, waiting...");
        sleep(Duration::from_secs(5)).await;
    }

    anyhow::bail!("Timed out waiting for faucet note")
}

pub async fn register_name(
    name: String,
    naming_account_hex: String,
    deployer_account_hex: String,
) -> anyhow::Result<()> {
    println!("Registering name '{}' on devnet...", name);
    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let naming_account_id = AccountId::from_hex(&naming_account_hex)?;
    let deployer_account_id = AccountId::from_hex(&deployer_account_hex)?;

    // Devnet faucet: mdev1az9sgvfkappxwgrjn2pmx0u45cs34cfk
    let faucet_id = AccountId::from_hex("0x8b043136e8426720729a83b33f95a6")?;

    // Determine base price by name length (from set_all_prices_testnet.masm constants).
    // Values are in the token's smallest unit (e.g. 20_000_000 = 20 tokens).
    let name_len = name.len();
    let base_price: u64 = match name_len {
        1 => 375_000_000,
        2 => 200_000_000,
        3 => 120_000_000,
        4 => 55_000_000,
        _ => 20_000_000, // 5+ characters
    };

    // Apply discount based on registration length (matching _calculate_discount in naming.masm)
    let reg_len: u64 = 1; // 1 year registration
    let discounted_price = if reg_len >= 5 {
        base_price - (base_price * 5000 / 10000) // 50% discount
    } else if reg_len >= 3 {
        base_price - (base_price * 3000 / 10000) // 30% discount
    } else {
        base_price // no discount
    };
    let price = discounted_price * reg_len;

    // Step 1: Mint tokens from faucet to deployer
    mint_from_faucet(&mut client, deployer_account_id, price).await?;

    // Encode the domain name
    let domain = encode_domain_as_felts(name.clone());

    // Register note inputs: [TOKEN (4 felts), DOMAIN (4 felts), REG_LEN (4 felts)]
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
            Felt::new(1), // 1 year registration
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;

    println!("Domain: {}, Length: {}, Price: {} tokens", name, name_len, price);

    let cost = FungibleAsset::new(faucet_id, price)?;
    let register_assets = NoteAssets::new(vec![cost.into()])?;

    let register_note = create_note_for_naming(
        "register_name".to_string(),
        register_note_inputs,
        deployer_account_id,
        naming_account_id,
        register_assets,
    )
    .await?;

    let register_note_id = register_note.id();

    let register_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(register_note)])
        .build()?;

    let register_tx_id = client
        .submit_new_transaction(deployer_account_id, register_req)
        .await?;

    println!(
        "View transaction on MidenScan: https://devnet.midenscan.com/tx/{:?}",
        register_tx_id
    );
    client.sync_state().await?;

    println!("Register note tx submitted, waiting for onchain commitment...");
    wait_for_tx(&mut client, register_tx_id).await?;

    sleep(Duration::from_secs(6)).await;
    client.sync_state().await?;

    // Import the register note explicitly so the naming account can consume it.
    // The automatic NoteScreener may fail to detect custom note scripts as consumable,
    // so we import by ID which fetches the committed note directly from the node.
    println!("Importing register note for naming account...");
    client.import_notes(&[NoteFile::NoteId(register_note_id)]).await?;
    client.sync_state().await?;

    // Get the imported note from the input notes store
    let input_notes = client
        .get_input_notes(NoteFilter::List(vec![register_note_id]))
        .await?;

    if !input_notes.is_empty() {
        println!("Found {} imported note(s), consuming...", input_notes.len());

        let notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = input_notes
            .into_iter()
            .map(|record| {
                let note: Note = record.try_into().unwrap();
                (note, None)
            })
            .collect();

        let nop_script_code =
            std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
        let transaction_script = CodeBuilder::default().compile_tx_script(nop_script_code)?;

        let consume_request = TransactionRequestBuilder::new()
            .input_notes(notes)
            .custom_script(transaction_script)
            .build()?;

        let consume_tx_id = client
            .submit_new_transaction(naming_account_id, consume_request)
            .await?;
        println!(
            "Consuming register note tx: https://devnet.midenscan.com/tx/{:?}",
            consume_tx_id
        );

        wait_for_tx(&mut client, consume_tx_id).await?;
        println!("Name '{}' registered successfully!", name);
    } else {
        anyhow::bail!("Failed to import register note - note not found on chain");
    }

    Ok(())
}
