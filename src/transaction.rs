use miden_assembly::Library;
use miden_client::{Client, ClientError, ScriptBuilder, keystore::FilesystemKeyStore, store::TransactionFilter, transaction::{TransactionId, TransactionScript, TransactionStatus}};
use rand::rngs::StdRng;
use tokio::time::{sleep, Duration};

pub async fn wait_for_tx(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    tx_id: TransactionId,
) -> Result<(), ClientError> {
    loop {
        client.sync_state().await?;

        // Check transaction status
        let txs = client
            .get_transactions(TransactionFilter::Ids(vec![tx_id]))
            .await?;

        let tx_committed = if !txs.is_empty() {
            matches!(txs[0].status, TransactionStatus::Committed { .. })
        } else {
            false
        };

        if tx_committed {
            println!("✅ transaction {} committed", tx_id.to_hex());
            break;
        }

        println!(
            "Transaction {} not yet committed. Waiting...",
            tx_id.to_hex()
        );
        sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

pub fn create_tx_script(
    script_code: String,
    library: Option<Library>,
) -> anyhow::Result<TransactionScript> {
    if let Some(lib) = library {
        return Ok(ScriptBuilder::new(true)
            .with_dynamically_linked_library(&lib)
            .unwrap()
            .compile_tx_script(script_code)
            .unwrap());
    };

    Ok(ScriptBuilder::new(true)
        .compile_tx_script(script_code)
        .unwrap())
}