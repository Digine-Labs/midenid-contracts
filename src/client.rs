use std::sync::Arc;

use miden_client::{builder::ClientBuilder, keystore::FilesystemKeyStore, rpc::{Endpoint, GrpcClient}, Client};
use miden_client_sqlite_store::ClientBuilderSqliteExt;
use rand::rngs::StdRng;

const TIMEOUT: u64 = 10_000;

pub async fn initiate_client(keystore: Arc<FilesystemKeyStore<StdRng>>) -> anyhow::Result<Client<FilesystemKeyStore<StdRng>>> {
    let endpoint = Endpoint::testnet();

    let rpc_client = Arc::new(GrpcClient::new(&endpoint, TIMEOUT));

    let store_path = std::path::PathBuf::from("./store.sqlite3");

    let mut client= ClientBuilder::new()
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

pub fn create_keystore() -> anyhow::Result<Arc<FilesystemKeyStore<StdRng>>> {
    let keystore_path = std::path::PathBuf::from("./keystore");
    let keystore: Arc<FilesystemKeyStore<StdRng>> = Arc::new(FilesystemKeyStore::<StdRng>::new(keystore_path)?);
    
    Ok(keystore)
}