use miden_assembly::{DefaultSourceManager, Library, LibraryPath, ast::{Module, ModuleKind}};
use miden_client::{
    Client, account::{AccountBuilder, AccountStorageMode, AccountType}, auth::NoAuth, keystore::FilesystemKeyStore, note::{NoteAssets, NoteInputs}, transaction::{OutputNote, TransactionKernel, TransactionRequestBuilder}
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
    Ok(())
}

