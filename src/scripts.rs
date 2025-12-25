use miden_client::{
    ScriptBuilder,
    account::AccountId,
    keystore::FilesystemKeyStore,
    note::{NoteAssets, NoteInputs},
    transaction::{OutputNote, TransactionRequestBuilder},
};
use miden_crypto::Felt;
use rand::rngs::StdRng;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use crate::{notes::create_note_for_naming, transaction::wait_for_tx};

pub async fn deploy(
    client: &mut miden_client::Client<
        miden_client::keystore::FilesystemKeyStore<rand::rngs::StdRng>,
    >,
    keystore: &mut Arc<FilesystemKeyStore<StdRng>>,
    deployer_account: AccountId,
    naming_account: AccountId,
) -> anyhow::Result<()> {
    println!("Starting Miden Name Registry deployment...");

    client.sync_state().await?;

    println!("=================================================");

    println!("Init note creation started");

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
        .own_output_notes(vec![OutputNote::Full(init_note.clone())])
        .build()?;

    let init_tx_id = client
        .submit_new_transaction(deployer_account, init_req)
        .await?;

    println!("Submitted initialization note transaction.");

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
        init_tx_id
    );
    client.sync_state().await?;

    println!("naming initialize note creation tx submitted, waiting for onchain commitment");

    wait_for_tx(client, init_tx_id).await?;

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
        .submit_new_transaction(naming_account, consume_request)
        .await?;
    println!(
        "Consuming initialization notes via transaction: {:?}",
        consume_tx_id
    );

    wait_for_tx(client, consume_tx_id).await?;

    println!("✅ Initialization Notes consumed successfully!");

    println!("=================================================");

    // println!("Setting prices");

    // println!("Set price note creation started");

    // let payment_token_id = AccountId::from_hex("0x54bf4e12ef20082070758b022456c7")?;

    // let set_prices_note_inputs = NoteInputs::new(
    //     [
    //         Felt::new(payment_token_id.suffix().into()),
    //         Felt::new(payment_token_id.prefix().into()),
    //     ]
    //     .to_vec(),
    // )?;

    // let set_prices_note = create_note_for_naming(
    //     "set_all_prices_testnet".to_string(),
    //     set_prices_note_inputs,
    //     deployer_account.id(),
    //     naming_account.id(),
    //     NoteAssets::new(vec![]).unwrap(),
    // )
    // .await?;

    // let set_price_req = TransactionRequestBuilder::new()
    //     .own_output_notes(vec![OutputNote::Full(set_prices_note.clone())])
    //     .build()?;

    // let set_prices_tx_id = client
    //     .submit_new_transaction(deployer_account.id(), set_price_req)
    //     .await?;

    // println!("Submitted set price note transaction.");

    // println!(
    //     "View transaction on MidenScan: https://testnet.midenscan.com/tx/{:?}",
    //     set_prices_tx_id
    // );
    // client.sync_state().await?;

    // println!("set prices tx submitted, waiting for onchain commitment");

    // println!("=================================================");

    // wait_for_tx(&mut client, set_prices_tx_id).await?;

    // sleep(Duration::from_secs(6)).await;

    // client.sync_state().await?;

    // sleep(Duration::from_secs(6)).await;

    // println!("Consuming pricing notes...");

    // let nop_script_code = std::fs::read_to_string(std::path::Path::new("./masm/scripts/nop.masm"))?;
    // let transaction_script = ScriptBuilder::new(false).compile_tx_script(nop_script_code)?;

    // let consume_request = TransactionRequestBuilder::new()
    //     .authenticated_input_notes(vec![(set_prices_note.id(), None)])
    //     .custom_script(transaction_script)
    //     .build()?;

    // let consume_tx_id = client
    //     .submit_new_transaction(naming_account.id(), consume_request)
    //     .await?;
    // println!(
    //     "Consuming pricing notes via transaction: {:?}",
    //     consume_tx_id
    // );

    // wait_for_tx(&mut client, consume_tx_id).await?;

    // println!("✅ Notes pricing successfully!");

    println!("=================================================");

    println!("✅ Miden Name Registry deployed successfully! ✅");

    Ok(())
}
