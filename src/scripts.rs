
use miden_client::{
    account::{AccountId}, note::{NoteAssets, NoteInputs}, transaction::{OutputNote, TransactionRequestBuilder}
};
use miden_crypto::Felt;
use tokio::time::{sleep, Duration};


use crate::{accounts::{create_deployer_account, create_naming_account}, client::{create_keystore, initiate_client}, notes::create_note_for_naming, transaction::wait_for_tx};

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
    Ok(())
}

