use miden_assembly::{DefaultSourceManager, Library, LibraryPath, ast::{Module, ModuleKind}};
use miden_client::{
    Client, account::{AccountBuilder, AccountId, AccountStorageMode, AccountType}, auth::NoAuth, keystore::FilesystemKeyStore, note::{NoteAssets, NoteInputs}, transaction::{OutputNote, TransactionKernel, TransactionRequestBuilder}
};
use miden_crypto::Felt;
use miden_objects::account::AccountComponent;
use rand::{RngCore, rngs::StdRng};
use std::{fs, path::Path};
use tokio::time::{sleep, Duration};


use crate::{accounts::{create_deployer_account, create_naming_account}, client::{create_keystore, initiate_client}, notes::create_note_for_naming, storage::naming_storage, transaction::wait_for_tx};

pub async fn deploy() -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment...");
    let mut keystore = create_keystore()?;
    let mut client = initiate_client(keystore.clone()).await?;

    let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
    let naming_account = create_naming_account(&mut client).await?;
    client.sync_state().await?;

    let initialize_inputs = NoteInputs::new([
        Felt::new(deployer_account.id().suffix().into()),
        Felt::new(deployer_account.id().prefix().into()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(5000),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    let init_note = create_note_for_naming("initialize_naming".to_string(), initialize_inputs, deployer_account.id(), naming_account.id(), NoteAssets::new(vec![]).unwrap()).await?;

    let init_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(init_note)])
        .build()?;

    let init_tx_id = client.submit_new_transaction(deployer_account.id(), init_req).await?;

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );
    client.sync_state().await?;

    println!("naming initialize note creation tx submitted, waiting for onchain commitment");

    wait_for_tx(&mut client, init_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    println!("Setting prices");

    let payment_token_id = AccountId::from_hex("0x54bf4e12ef20082070758b022456c7")?;

    let set_prices_note_inputs = NoteInputs::new([
        Felt::new(payment_token_id.suffix().into()),
        Felt::new(payment_token_id.prefix().into()),
    ].to_vec())?;

    let set_prices_note = create_note_for_naming("set_all_prices_testnet".to_string(), set_prices_note_inputs, deployer_account.id(), naming_account.id(), NoteAssets::new(vec![]).unwrap()).await?;

    let set_price_req = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(set_prices_note)])
        .build()?;

    let set_prices_tx_id = client.submit_new_transaction(deployer_account.id(), set_price_req).await?;

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        set_prices_tx_id
    );
    client.sync_state().await?;

    println!("set prices tx submitted, waiting for onchain commitment");

    wait_for_tx(&mut client, set_prices_tx_id).await?;

    sleep(Duration::from_secs(6)).await;

    client.sync_state().await?;

    // Consume notes explicitly (required for NoAuth accounts)
    println!("Consuming initialization notes...");
    let consumable_notes = client.get_consumable_notes(Some(naming_account.id())).await?;

    if !consumable_notes.is_empty() {
        println!("Found {} consumable note(s)", consumable_notes.len());

        let note_ids: Vec<_> = consumable_notes.iter().map(|(record, _)| (record.id(), None)).collect();

        let nop_script_code = std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
        use miden_client::ScriptBuilder;
        let transaction_script = ScriptBuilder::new(false)
            .compile_tx_script(nop_script_code)?;

        let consume_request = TransactionRequestBuilder::new()
            .authenticated_input_notes(note_ids)
            .custom_script(transaction_script)
            .build()?;

        let consume_tx_id = client.submit_new_transaction(naming_account.id(), consume_request).await?;
        println!("Consuming notes via transaction: {:?}", consume_tx_id);

        wait_for_tx(&mut client, consume_tx_id).await?;
        println!("✅ Notes consumed successfully!");
    } else {
        println!("Warning: No consumable notes found");
    }

    Ok(())
}

