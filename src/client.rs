use std::sync::Arc;

use miden_client::{
    Client,
    builder::ClientBuilder,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, GrpcClient},
};
use miden_client_sqlite_store::ClientBuilderSqliteExt;

const TIMEOUT: u64 = 10_000;

pub async fn initiate_client(
    keystore: Arc<FilesystemKeyStore>,
    use_testnet: bool,
) -> anyhow::Result<Client<FilesystemKeyStore>> {
    let endpoint = if use_testnet {
        Endpoint::testnet()
    } else {
        Endpoint::devnet()
    };

    let rpc_client = Arc::new(GrpcClient::new(&endpoint, TIMEOUT));

    let store_path = std::path::PathBuf::from("./store.sqlite3");

    let mut client = ClientBuilder::new()
        .rpc(rpc_client)
        .sqlite_store(store_path)
        .authenticator(keystore.clone())
        .build()
        .await?;

    let sync_summary = client.sync_state().await.unwrap();
    println!("Latest block: {}", sync_summary.block_num);
    Ok(client)
}

pub fn create_keystore() -> anyhow::Result<Arc<FilesystemKeyStore>> {
    let keystore_path = std::path::PathBuf::from("./keystore");
    let keystore: Arc<FilesystemKeyStore> = Arc::new(FilesystemKeyStore::new(keystore_path)?);

    Ok(keystore)
}
