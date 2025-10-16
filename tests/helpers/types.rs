use miden_client::{
    ClientError, Word, account::Account, keystore::FilesystemKeyStore, rpc::Endpoint,
};
use midenid_contracts::common::*;
use rand::rngs::StdRng;

use super::client::paths;

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

/// Registry test helper for managing test state
///
/// Provides a stateful wrapper around the Miden client with references to:
/// - The deployed registry contract
/// - The registry owner account
/// - The payment token faucet (if used)
pub struct RegistryTestHelper {
    pub client: Client,
    pub endpoint: Endpoint,
    pub keystore: FilesystemKeyStore<StdRng>,
    pub registry_contract: Option<Account>,
    pub owner_account: Option<Account>,
    pub faucet_account: Option<Account>,
}

impl RegistryTestHelper {
    /// Creates a new test helper WITHOUT clearing the database/keystore.
    ///
    /// This constructor preserves existing data in `./store.sqlite3` and `./keystore/`,
    /// making it useful for tests that need to access previously created accounts or contracts.
    ///
    /// # Network Connection
    ///
    /// Attempts to connect to testnet first, falling back to localhost if unavailable.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully created helper instance
    /// * `Err(ClientError)` - Failed to connect to any endpoint
    pub async fn new_persistent() -> Result<Self, ClientError> {
        let endpoint = Endpoint::testnet();
        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(_) => {
                let localhost_endpoint = Endpoint::localhost();
                instantiate_client(localhost_endpoint.clone()).await?
            }
        };

        let keystore = FilesystemKeyStore::new(paths::KEYSTORE_DIR.into()).unwrap();

        Ok(Self {
            client,
            endpoint,
            keystore,
            registry_contract: None,
            owner_account: None,
            faucet_account: None,
        })
    }

    /// Creates a new test helper with network connection and CLEAN state.
    ///
    /// This constructor deletes existing database and keystore data, providing a
    /// fresh environment for tests. Use this for isolated test cases.
    ///
    /// # Network Connection
    ///
    /// Attempts to connect to testnet first, falling back to localhost if unavailable.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully created helper with clean state
    /// * `Err(ClientError)` - Failed to connect to any endpoint
    ///
    /// # Side Effects
    ///
    /// - Deletes `./store.sqlite3` database
    /// - Clears `./keystore/` directory
    pub async fn new() -> Result<Self, ClientError> {
        delete_keystore_and_store().await;

        let endpoint = Endpoint::testnet();
        let mut client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(_) => {
                let localhost_endpoint = Endpoint::localhost();
                instantiate_client(localhost_endpoint.clone()).await?
            }
        };

        // Sync with node to fetch initial blockchain state
        client.sync_state().await?;

        let keystore = FilesystemKeyStore::new(paths::KEYSTORE_DIR.into()).unwrap();

        Ok(Self {
            client,
            endpoint,
            keystore,
            registry_contract: None,
            owner_account: None,
            faucet_account: None,
        })
    }
}

/// Parsed registry contract state
///
/// Represents the current state of a deployed registry contract,
/// extracted from the contract's storage slots.
#[derive(Debug)]
pub struct ContractState {
    pub initialized: u64,
    pub owner_prefix: u64,
    pub owner_suffix: u64,
    pub token_prefix: u64,
    pub token_suffix: u64,
    pub name_to_id_mapping: Option<Word>,
    pub id_to_name_mapping: Option<Word>,
}
