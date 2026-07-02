use miden_client::{
    Client,
    account::AccountId,
    asset::{AssetCallbackFlag, AssetVaultKey, FungibleAsset},
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets, NoteFile, NoteId, NoteStorage},
    store::NoteFilter,
    transaction::TransactionRequestBuilder,
};
use miden_crypto::{Felt, Word};
use miden_standards::code_builder::CodeBuilder;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

use crate::{
    accounts::{create_deployer_account, create_naming_account, safe_account_import},
    client::{create_keystore, initiate_client},
    domain::{encode_domain, reverse_word},
    notes::create_note_for_naming_with_client,
    storage::slot_name,
    transaction::wait_for_tx,
    utils::get_price_by_length,
};

fn midenscan_base_url(use_testnet: bool) -> &'static str {
    if use_testnet {
        "https://testnet.midenscan.com"
    } else {
        "https://devnet.midenscan.com"
    }
}

fn default_faucet_id(use_testnet: bool) -> &'static str {
    if use_testnet {
        "0x2458e5446128e6b150b75b8ebd9ce1"
    } else {
        "0x16f6c85d5652c9200879145bfdda93"
    }
}

pub async fn deploy(is_network: bool, use_testnet: bool) -> anyhow::Result<()> {
    println!(
        "Starting Miden Name Registry deployment ( network: {} )",
        is_network
    );
    println!("=================================================");
    println!("Deleting existing store & keystore (store.sqlite3)");
    let _ = std::fs::remove_file("store.sqlite3");
    let _ = std::fs::remove_dir("keystore");
    println!("Deletion complete.");
    println!("=================================================");

    let mut keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone(), use_testnet).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client, is_network).await?;

    // Init note — compiled via the shared helper so its root matches the network account's
    // tx-script allowlist.
    let tx_script = crate::notes::compile_init_on_chain_tx_script(is_network)?;

    let tx_init_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .unwrap();

    let tx_id = client
        .submit_new_transaction(naming_account.id(), tx_init_request)
        .await?;

    let base_url = midenscan_base_url(use_testnet);
    println!("View transaction on MidenScan: {}/tx/{:?}", base_url, tx_id);

    // Wait for the transaction to be committed
    wait_for_tx(&mut client, tx_id).await.unwrap();

    // Contract initialized

    let initialize_inputs = NoteStorage::new(
        [
            deployer_account.id().suffix(),
            deployer_account.id().prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
        ]
        .to_vec(),
    )?;
    let init_note = create_note_for_naming_with_client(
        "initialize_naming".to_string(),
        initialize_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
        is_network,
        &mut client,
    )
    .await?;

    let init_note_id = init_note.id();

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![init_note])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account.id(), init_req)
        .await?;
    println!(
        "View transaction on MidenScan: {}/tx/{:?}",
        base_url, init_tx_id
    );

    client.sync_state().await?;

    println!("init note creation tx submitted, waiting for onchain commitment");

    // Wait for the note transaction to be committed
    wait_for_tx(&mut client, init_tx_id).await.unwrap();

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    if !is_network {
        consume_note_by_id(&mut client, init_note_id, naming_account.id(), use_testnet).await?;
    }

    // Checking updated state
    let new_account_state = client.get_account(naming_account.id()).await.unwrap();

    if let Some(record) = new_account_state {
        let account: miden_protocol::account::Account = record.try_into().unwrap();
        let count: Word = account
            .storage()
            .get_item(&slot_name("naming::init_flag"))
            .unwrap()
            .into();
        println!("Final deployer prefix value: {}", count.to_string());
    }

    // SET PRICE
    let payment_token_id = AccountId::from_hex(default_faucet_id(use_testnet))?;

    let set_prices_note_inputs = NoteStorage::new(
        [
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
        ]
        .to_vec(),
    )?;

    let set_prices_note = create_note_for_naming_with_client(
        "set_all_prices_testnet".to_string(),
        set_prices_note_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
        is_network,
        &mut client,
    )
    .await?;

    let set_prices_note_id = set_prices_note.id();

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![set_prices_note])
        .build()?;

    let set_price_tx_id = client
        .submit_new_transaction(deployer_account.id(), set_price_req)
        .await?;
    println!(
        "View transaction on MidenScan: {}/tx/{:?}",
        base_url, set_price_tx_id
    );

    client.sync_state().await?;

    println!("set price note creation tx submitted, waiting for onchain commitment");

    // Wait for the note transaction to be committed
    wait_for_tx(&mut client, set_price_tx_id).await.unwrap();

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    if !is_network {
        consume_note_by_id(
            &mut client,
            set_prices_note_id,
            naming_account.id(),
            use_testnet,
        )
        .await?;
    }

    let new_account_state = client.get_account(naming_account.id()).await.unwrap();

    if let Some(record) = new_account_state {
        let account: miden_protocol::account::Account = record.try_into().unwrap();
        let prices_slot = slot_name("naming::prices");

        let one_letter_word = Word::new([
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
            Felt::new(1)?,
            Felt::new(0)?,
        ]);
        let one_letter_price: Word = account
            .storage()
            .get_map_item(&prices_slot, reverse_word(one_letter_word))
            .unwrap()
            .into();
        println!("one letter price value: {}", one_letter_price.to_string());

        let two_letter_word = Word::new([
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
            Felt::new(2)?,
            Felt::new(0)?,
        ]);
        let two_letter_price: Word = account
            .storage()
            .get_map_item(&prices_slot, reverse_word(two_letter_word))
            .unwrap()
            .into();
        println!("two letter price value: {}", two_letter_price.to_string());

        let three_letter_word = Word::new([
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
            Felt::new(3)?,
            Felt::new(0)?,
        ]);
        let three_letter_price: Word = account
            .storage()
            .get_map_item(&prices_slot, reverse_word(three_letter_word))
            .unwrap()
            .into();
        println!(
            "three letter price value: {}",
            three_letter_price.to_string()
        );

        let four_letter_word = Word::new([
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
            Felt::new(4)?,
            Felt::new(0)?,
        ]);
        let four_letter_price: Word = account
            .storage()
            .get_map_item(&prices_slot, reverse_word(four_letter_word))
            .unwrap()
            .into();
        println!("four letter price value: {}", four_letter_price.to_string());

        let five_letter_word = Word::new([
            payment_token_id.suffix(),
            payment_token_id.prefix().as_felt(),
            Felt::new(5)?,
            Felt::new(0)?,
        ]);
        let five_letter_price: Word = account
            .storage()
            .get_map_item(&prices_slot, reverse_word(five_letter_word))
            .unwrap()
            .into();
        println!("five letter price value: {}", five_letter_price.to_string());
    }

    Ok(())
}

/// Helper to import and consume a note by ID against a target account.
/// Uses explicit import to bypass NoteScreener which may fail for custom note scripts.
async fn consume_note_by_id(
    client: &mut Client<FilesystemKeyStore>,
    note_id: NoteId,
    target_account_id: AccountId,
    use_testnet: bool,
) -> anyhow::Result<()> {
    println!("Importing and consuming note {:?}", note_id);
    client.import_notes(&[NoteFile::NoteId(note_id)]).await?;
    client.sync_state().await?;

    let input_notes = client
        .get_input_notes(NoteFilter::List(vec![note_id]))
        .await?;
    let notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = input_notes
        .into_iter()
        .map(|record| {
            let note: Note = record.try_into().unwrap();
            (note, None)
        })
        .collect();

    let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
    let transaction_request = TransactionRequestBuilder::new()
        .input_notes(notes)
        .custom_script(CodeBuilder::default().compile_tx_script(nop_script_code)?)
        .build()?;

    let tx_id = client
        .submit_new_transaction(target_account_id, transaction_request)
        .await?;

    let base_url = midenscan_base_url(use_testnet);
    println!("View transaction on MidenScan: {}/tx/{:?}", base_url, tx_id);

    wait_for_tx(client, tx_id).await?;

    sleep(Duration::from_secs(6)).await;
    client.sync_state().await?;

    Ok(())
}

pub async fn consume_single_note(
    note_id: String,
    naming_account_id: String,
    use_testnet: bool,
) -> anyhow::Result<()> {
    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone(), use_testnet).await?;

    let note_id = NoteId::try_from_hex(&note_id)?;

    let naming_account = AccountId::from_hex(&naming_account_id)?;

    safe_account_import(&mut client, naming_account).await?;

    client.sync_state().await?;

    consume_note_by_id(&mut client, note_id, naming_account, use_testnet).await?;

    Ok(())
}

pub async fn send_register_note(
    account: String,
    naming_account: String,
    faucet_id: String,
    name: String,
    is_network: bool,
    use_testnet: bool,
) -> anyhow::Result<()> {
    println!("\n[Sending register note]");
    println!("=================================================");

    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone(), use_testnet).await?;

    client.sync_state().await?;

    let account = AccountId::from_hex(&account)?;
    let naming_account = AccountId::from_hex(&naming_account)?;
    let faucet_id = AccountId::from_hex(&faucet_id)?;

    safe_account_import(&mut client, account).await?;
    safe_account_import(&mut client, naming_account).await?;

    // 0.15: network status is no longer encoded in the AccountId; it is supplied explicitly
    // (matching the deploy-time `--as-network` flag).

    println!("Checking balance of sender account");

    let account_record = client.get_account(account).await?.unwrap();
    let full_account: miden_protocol::account::Account = account_record.try_into()?;
    // 0.15: get_balance takes an AssetVaultKey and returns AssetAmount.
    let balance: u64 = full_account
        .vault()
        .get_balance(AssetVaultKey::new_fungible(faucet_id, AssetCallbackFlag::Disabled))?
        .into();

    let price = get_price_by_length(&name);

    if price > balance {
        for asset in full_account.vault().assets() {
            if let miden_protocol::asset::Asset::Fungible(fa) = asset {
                eprintln!("  Vault faucet: {} amount: {}", fa.faucet_id().to_hex(), fa.amount());
            }
        }
        return Err(anyhow::anyhow!(
            "Insufficient balance: {} < price {} (faucet: {})",
            balance,
            price,
            faucet_id.to_hex()
        ));
    }

    println!("Creating register note for account: {:?}", account.to_hex());

    let domain = encode_domain(name);

    let fungible_asset = FungibleAsset::new(faucet_id, price).unwrap();
    let register_note_inputs = NoteStorage::new(
        [
            faucet_id.suffix(),
            faucet_id.prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
            domain[0],
            domain[1],
            domain[2],
            domain[3],
        ]
        .to_vec(),
    )?;

    let register_asset = NoteAssets::new(vec![fungible_asset.into()])?;

    let register_note = create_note_for_naming_with_client(
        "register_name".to_string(),
        register_note_inputs,
        account,
        naming_account,
        register_asset,
        is_network,
        &mut client,
    )
    .await?;

    let note_id = register_note.id();

    let register_note_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![register_note])
        .build()?;

    let register_note_tx_id = client
        .submit_new_transaction(account, register_note_req)
        .await?;

    println!("\n");

    let base_url = midenscan_base_url(use_testnet);
    println!("Register note id {:?}", note_id.to_hex());
    println!(
        "View note on MidenScan: {}/note/{}",
        base_url,
        note_id.to_hex()
    );
    println!(
        "View transaction on MidenScan: {}/tx/{:?}",
        base_url, register_note_tx_id
    );

    println!("\n");

    wait_for_tx(&mut client, register_note_tx_id).await?;

    client.sync_state().await?;

    if !is_network {
        consume_note_by_id(&mut client, note_id, naming_account, use_testnet).await?;
    }

    println!("Registration done");

    Ok(())
}

pub async fn find_consumable_notes(account: String, use_testnet: bool) -> anyhow::Result<()> {
    use tokio::time::{Duration, sleep};

    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone(), use_testnet).await?;

    let account = AccountId::from_hex(&account)?;

    safe_account_import(&mut client, account).await?;

    client.sync_state().await?;

    println!("Finding notes...");

    let max_attempts = 10;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("\nAttempt {}/{}", attempt, max_attempts);

        let consumable_notes = client.get_consumable_notes(Some(account)).await?;

        if !consumable_notes.is_empty() {
            println!("Found {} consumable note(s)", consumable_notes.len());

            let notes: Vec<(Note, Option<miden_client::transaction::NoteArgs>)> = consumable_notes
                .into_iter()
                .map(|(record, _)| {
                    let note: Note = record.try_into().unwrap();
                    (note, None)
                })
                .collect();

            let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
            let transaction_script = CodeBuilder::default().compile_tx_script(nop_script_code)?;

            let consume_request = TransactionRequestBuilder::new()
                .input_notes(notes)
                .custom_script(transaction_script)
                .build()?;

            let consume_tx_id = client
                .submit_new_transaction(account, consume_request)
                .await?;
            println!("Consuming notes via transaction: {:?}", consume_tx_id);

            wait_for_tx(&mut client, consume_tx_id).await?;
            println!("Notes consumed successfully!");
            return Ok(());
        } else {
            println!("No consumable notes found yet...");

            if attempt >= max_attempts {
                println!(
                    "Max attempts ({}) reached. No consumable notes found.",
                    max_attempts
                );
                return Ok(());
            }

            println!("Waiting 3 seconds before retry...");
            sleep(Duration::from_secs(3)).await;
        }
    }
}
