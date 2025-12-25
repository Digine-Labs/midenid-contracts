use miden_client::{
    ScriptBuilder,
    account::AccountId,
    note::{NoteAssets, NoteInputs},
    transaction::{OutputNote, TransactionRequestBuilder},
};
use miden_crypto::{Felt, Word};
use tokio::time::{Duration, sleep};
use std::{fs, path::Path, sync::Arc};

use crate::{
    accounts::{create_deployer_account, create_naming_account, create_network_naming_account},
    client::{create_keystore, initiate_client},
    notes::{create_library, create_note_for_naming},
    transaction::wait_for_tx,
};

pub async fn deploy_as_network_account() -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment ( Using network account )");
    println!("=================================================");
    println!("Deleting existing store & keystore (store.sqlite3)");
    let _ = std::fs::remove_file("store.sqlite3");
    let _ = std::fs::remove_dir("keystore");
    println!("Deletion complete.");
    println!("=================================================");

    let mut keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_network_naming_account(&mut client).await?;

    // Init note
    let script_code = fs::read_to_string(Path::new("./masm/scripts/init_on_chain.masm")).unwrap();

    let account_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library_path = "miden_name::naming";

    let library = create_library(account_code, library_path)?;

    let tx_script = client.script_builder().with_dynamically_linked_library(&library)?.compile_tx_script(&script_code)?;

    let tx_init_request = TransactionRequestBuilder::new().custom_script(tx_script).build().unwrap();

    let tx_id = client.submit_new_transaction(naming_account.id(), tx_init_request).await?;

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
    let init_note = create_note_for_naming(
        "initialize_naming".to_string(),
        initialize_inputs,
        deployer_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    ).await?;

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note)])
        .build()?;

    let init_tx_id = client.submit_new_transaction(deployer_account.id(), init_req).await?;
    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );

    client.sync_state().await?;

    println!("network init note creation tx submitted, waiting for onchain commitment");

    // Wait for the note transaction to be committed
    wait_for_tx(&mut client, init_tx_id).await.unwrap();

    sleep(Duration::from_secs(12)).await;

    client.sync_state().await?;

    // Checking updated state
    let new_account_state = client.get_account(naming_account.id()).await.unwrap();

    if let Some(account) = new_account_state.as_ref() {
        let count: Word = account.account().storage().get_item(0).unwrap().into();
        println!("🔢 Final deployer prefix value: {}", count.to_string());
    }
    Ok(())
}

pub async fn deploy() -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment...");
    println!("=================================================");
    println!("Deleting existing store & keystore (store.sqlite3)");
    let _ = std::fs::remove_file("store.sqlite3");
    let _ = std::fs::remove_dir("keystore");
    println!("Deletion complete.");
    println!("=================================================");

    let mut keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client).await?;
    client.sync_state().await?;

    println!("=================================================");

    println!("Init note creation started");

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

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note.clone())])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account.id(), init_req)
        .await?;

    println!("Submitted initialization note transaction.");

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );
    client.sync_state().await?;

    println!("naming initialize note creation tx submitted, waiting for onchain commitment");

    wait_for_tx(&mut client, init_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    println!("=================================================");

    // Consume notes explicitly (required for NoAuth accounts)
    println!("Consuming initialization notes...");

    let nop_script_code = std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
    let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

    let consume_request = TransactionRequestBuilder::new()
        .authenticated_input_notes(vec![(init_note.id(), None)])
        .custom_script(transaction_script)
        .build()?;

    let consume_tx_id = client
        .submit_new_transaction(naming_account.id(), consume_request)
        .await?;
    println!(
        "Consuming initialization notes via transaction: {:?}",
        consume_tx_id
    );

    wait_for_tx(&mut client, consume_tx_id).await?;

    println!("✅ Initialization Notes consumed successfully!");

    println!("=================================================");

    println!("Setting prices");

    println!("Set price note creation started");

    let payment_token_id = AccountId::from_hex("0x54bf4e12ef20082070758b022456c7")?;

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

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(set_prices_note.clone())])
        .build()?;

    let set_prices_tx_id = client
        .submit_new_transaction(deployer_account.id(), set_price_req)
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

    sleep(Duration::from_secs(6)).await;

    println!("Consuming pricing notes...");

    let nop_script_code = std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
    let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

    let consume_request = TransactionRequestBuilder::new()
        .authenticated_input_notes(vec![(set_prices_note.id(), None)])
        .custom_script(transaction_script)
        .build()?;

    let consume_tx_id = client
        .submit_new_transaction(naming_account.id(), consume_request)
        .await?;
    println!(
        "Consuming pricing notes via transaction: {:?}",
        consume_tx_id
    );

    wait_for_tx(&mut client, consume_tx_id).await?;

    println!("✅ Notes pricing successfully!");

    println!("=================================================");

    println!("✅ Miden Name Registry deployed successfully! ✅");

    Ok(())
}
