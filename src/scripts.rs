use miden_client::{
    ScriptBuilder,
    account::AccountId,
    asset::FungibleAsset,
    note::{NoteAssets, NoteId, NoteInputs},
    transaction::{OutputNote, TransactionRequestBuilder},
};
use miden_crypto::{Felt, Word};
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

use crate::{
    accounts::{create_deployer_account, create_naming_account, safe_account_import},
    client::{create_keystore, initiate_client},
    domain::encode_domain,
    notes::{create_library, create_note_for_naming_with_client},
    transaction::wait_for_tx,
    utils::get_price_by_length,
};

pub async fn deploy(is_network: bool) -> anyhow::Result<()> {
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
    let mut client = initiate_client(keystore.clone()).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client, is_network).await?;

    // Init note
    let script_code = fs::read_to_string(Path::new("./masm/scripts/init_on_chain.masm")).unwrap();

    let account_code = fs::read_to_string(Path::new("./masm/accounts/naming_unsafe.masm")).unwrap();
    let library_path = "miden_name::naming";

    let library = create_library(account_code, library_path)?;

    let tx_script = client
        .script_builder()
        .with_dynamically_linked_library(&library)?
        .compile_tx_script(&script_code)?;

    let tx_init_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .unwrap();

    let tx_id = client
        .submit_new_transaction(naming_account.id(), tx_init_request)
        .await?;

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        tx_id
    );

    // Wait for the transaction to be committed
    wait_for_tx(&mut client, tx_id).await.unwrap();

    // Contract initialzed

    let initialize_inputs = NoteInputs::new(
        [
            Felt::new(deployer_account.id().suffix().into()),
            Felt::new(deployer_account.id().prefix().into()),
            Felt::new(0),
            Felt::new(0),
        ]
        .to_vec(),
    )?;
    let init_note = create_note_for_naming_with_client(
        "initialize_naming".to_string(),
        initialize_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
        &mut client,
    )
    .await?;

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note.clone())])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account.id(), init_req)
        .await?;
    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );

    client.sync_state().await?;

    println!("network init note creation tx submitted, waiting for onchain commitment");

    // Wait for the note transaction to be committed
    wait_for_tx(&mut client, init_tx_id).await.unwrap();

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    if !is_network {
        let init_note_id = init_note.id();

        println!("Consuming init note {:?}", init_note_id);

        let transaction_request = TransactionRequestBuilder::new()
            .build_consume_notes(vec![init_note_id])
            .unwrap();

        let tx_id = client
            .submit_new_transaction(naming_account.id(), transaction_request)
            .await?;

        println!(
            "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
            tx_id
        );

        wait_for_tx(&mut client, tx_id).await.unwrap();
    }

    // Checking updated state
    let new_account_state = client.get_account(naming_account.id()).await.unwrap();

    if let Some(account) = new_account_state.as_ref() {
        let count: Word = account.account().storage().get_item(0).unwrap().into();
        println!("🔢 Final deployer prefix value: {}", count.to_string());
    }

    // SET PRICE
    let payment_token_id = AccountId::from_hex("0x54bf4e12ef20082070758b022456c7")?;

    let set_prices_note_inputs = NoteInputs::new(
        [
            Felt::new(payment_token_id.suffix().into()),
            Felt::new(payment_token_id.prefix().into()),
        ]
        .to_vec(),
    )?;

    let set_prices_note = create_note_for_naming_with_client(
        "set_all_prices_testnet".to_string(),
        set_prices_note_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
        &mut client,
    )
    .await?;

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(set_prices_note.clone())])
        .build()?;

    let set_price_tx_id = client
        .submit_new_transaction(deployer_account.id(), set_price_req)
        .await?;
    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        set_price_tx_id
    );

    client.sync_state().await?;

    println!("network set price note creation tx submitted, waiting for onchain commitment");

    // Wait for the note transaction to be committed
    wait_for_tx(&mut client, set_price_tx_id).await.unwrap();

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    if !is_network {
        let set_prices_note_id = set_prices_note.id();

        println!("Consuming set_prices_note_id {:?}", set_prices_note_id);

        let transaction_request = TransactionRequestBuilder::new()
            .build_consume_notes(vec![set_prices_note_id])
            .unwrap();

        let prices_tx_id = client
            .submit_new_transaction(naming_account.id(), transaction_request)
            .await?;

        println!(
            "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
            prices_tx_id
        );

        wait_for_tx(&mut client, prices_tx_id).await.unwrap();
    }

    let new_account_state = client.get_account(naming_account.id()).await.unwrap();

    if let Some(account) = new_account_state.as_ref() {
        let one_letter_word = Word::new([
            Felt::new(payment_token_id.suffix().as_int()),
            Felt::new(payment_token_id.prefix().as_u64()),
            Felt::new(1),
            Felt::new(0),
        ]);
        let one_letter_price: Word = account
            .account()
            .storage()
            .get_map_item(2, one_letter_word)
            .unwrap()
            .into();
        println!(
            "🔢 one letter price value: {}",
            one_letter_price.to_string()
        );

        let two_letter_word = Word::new([
            Felt::new(payment_token_id.suffix().as_int()),
            Felt::new(payment_token_id.prefix().as_u64()),
            Felt::new(2),
            Felt::new(0),
        ]);
        let two_letter_price: Word = account
            .account()
            .storage()
            .get_map_item(2, two_letter_word)
            .unwrap()
            .into();
        println!(
            "🔢 two letter price value: {}",
            two_letter_price.to_string()
        );

        let three_letter_word = Word::new([
            Felt::new(payment_token_id.suffix().as_int()),
            Felt::new(payment_token_id.prefix().as_u64()),
            Felt::new(3),
            Felt::new(0),
        ]);
        let three_letter_price: Word = account
            .account()
            .storage()
            .get_map_item(2, three_letter_word)
            .unwrap()
            .into();
        println!(
            "🔢 three letter price value: {}",
            three_letter_price.to_string()
        );

        let four_letter_word = Word::new([
            Felt::new(payment_token_id.suffix().as_int()),
            Felt::new(payment_token_id.prefix().as_u64()),
            Felt::new(4),
            Felt::new(0),
        ]);
        let four_letter_price: Word = account
            .account()
            .storage()
            .get_map_item(2, four_letter_word)
            .unwrap()
            .into();
        println!(
            "🔢 four letter price value: {}",
            four_letter_price.to_string()
        );

        let five_letter_word = Word::new([
            Felt::new(payment_token_id.suffix().as_int()),
            Felt::new(payment_token_id.prefix().as_u64()),
            Felt::new(5),
            Felt::new(0),
        ]);
        let five_letter_price: Word = account
            .account()
            .storage()
            .get_map_item(2, five_letter_word)
            .unwrap()
            .into();
        println!(
            "🔢 five letter price value: {}",
            five_letter_price.to_string()
        );
    }

    Ok(())
}

pub async fn consume_single_note(note_id: String, naming_account_id: String) -> anyhow::Result<()> {
    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let note_id = NoteId::try_from_hex(&note_id)?;

    let naming_account = AccountId::from_hex(&naming_account_id)?;

    safe_account_import(&mut client, naming_account).await?;

    println!(
        "consumable notes {:?}",
        client.get_consumable_notes(Some(naming_account)).await?
    );

    client.sync_state().await?;

    let transaction_request = TransactionRequestBuilder::new()
        .build_consume_notes(vec![note_id])
        .unwrap();

    let tx_id = client
        .submit_new_transaction(naming_account, transaction_request)
        .await?;

    wait_for_tx(&mut client, tx_id).await?;

    Ok(())
}

pub async fn send_register_note(
    account: String,
    naming_account: String,
    faucet_id: String,
    name: String,
) -> anyhow::Result<()> {
    println!("\n[Sending register note]");
    println!("=================================================");

    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    client.sync_state().await?;

    let account = AccountId::from_hex(&account)?;
    let naming_account = AccountId::from_hex(&naming_account)?;
    let faucet_id = AccountId::from_hex(&faucet_id)?;

    safe_account_import(&mut client, account).await?;
    safe_account_import(&mut client, naming_account).await?;

    let is_network = naming_account.is_network();

    println!("Checking balance of sender account");

    let balance = client
        .get_account(account)
        .await?
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_id)?;

    let price = get_price_by_length(&name);

    if price > balance {
        return Err(anyhow::anyhow!("Insufficient balance of sender account"));
    }

    println!("Creating register note for account: {:?}", account.to_hex());

    let domain = encode_domain(name);

    let fungible_asset = FungibleAsset::new(faucet_id, price).unwrap();
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

    let register_note = create_note_for_naming_with_client(
        "register_name".to_string(),
        register_note_inputs,
        account,
        naming_account,
        register_asset,
        &mut client,
    )
    .await?;

    let register_note_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(register_note.clone())])
        .build()?;

    let register_note_tx_id = client
        .submit_new_transaction(account, register_note_req)
        .await?;

    let note_id = register_note.id();

    println!("\n");

    println!("Register note id {:?}", note_id.to_hex());
    println!(
        "View note on MidenScan: https://testnet.midenscan.com/note/{}",
        note_id.to_hex()
    );
    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        register_note_tx_id
    );

    println!("\n");

    wait_for_tx(&mut client, register_note_tx_id).await?;

    client.sync_state().await?;

    if !is_network {
        let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm"))?;
        let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

        let consume_request = TransactionRequestBuilder::new()
            .authenticated_input_notes(vec![(note_id, None)])
            .custom_script(transaction_script)
            .build()?;

        let consume_tx_id = client
            .submit_new_transaction(naming_account, consume_request)
            .await?;

        println!("\n");

        println!("📝 Consuming notes via transaction: {:?}", consume_tx_id);

        println!("\n");

        wait_for_tx(&mut client, consume_tx_id).await?;

        println!("\n");

        println!(
            "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
            consume_tx_id
        );
    }

    println!("Registration done");

    Ok(())
}

pub async fn find_consumable_notes(account: String) -> anyhow::Result<()> {
    use tokio::time::{Duration, sleep};

    let keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let account = AccountId::from_hex(&account)?;

    safe_account_import(&mut client, account).await?;

    client.sync_state().await?;

    println!("Finding notes...");

    let max_attempts = 10;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("\n🔍 Attempt {}/{}", attempt, max_attempts);

        let consumable_notes = client.get_consumable_notes(Some(account)).await?;

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
                .submit_new_transaction(account, consume_request)
                .await?;
            println!("📝 Consuming notes via transaction: {:?}", consume_tx_id);

            wait_for_tx(&mut client, consume_tx_id).await?;
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

            println!("⏰ Waiting 3 seconds before retry...");
            sleep(Duration::from_secs(3)).await;
        }
    }
}
