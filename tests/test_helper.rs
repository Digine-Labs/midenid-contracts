use miden_client::{
    ClientError, Word,
    account::Account,
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets, NoteInputs},
    rpc::Endpoint,
    store::AccountRecord,
    transaction::TransactionRequestBuilder,
};
use miden_objects::{Felt, FieldElement, account::AccountId, asset::FungibleAsset};
use midenid_contracts::common::*;
use rand::rngs::StdRng;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

// ================================================================================================
// CONSTANTS
// ================================================================================================

/// File path constants for registry operations
mod paths {
    pub const REGISTRY_CONTRACT: &str = "./masm/accounts/miden_id.masm";
    pub const NOP_SCRIPT: &str = "./masm/scripts/nop.masm";
    pub const NOTES_DIR: &str = "./masm/notes";
    pub const SCRIPTS_DIR: &str = "./masm/scripts";
    pub const KEYSTORE_DIR: &str = "./keystore";
    pub const STORE_DB: &str = "./store.sqlite3";
}

// ================================================================================================
// DATA STRUCTURES
// ================================================================================================

/// Complete contract state structure for validation and testing.
///
/// This struct encapsulates all storage data from the Miden ID registry contract,
/// making it easy to validate contract state in tests.
///
/// # Fields
///
/// * `initialized` - Flag indicating if registry is initialized (0 = no, 1 = yes)
/// * `owner_prefix` - First part of the owner account ID
/// * `owner_suffix` - Second part of the owner account ID
/// * `token_prefix` - First part of the payment token account ID
/// * `token_suffix` - Second part of the payment token account ID
/// * `name_to_id_mapping` - SMT root for name → account ID lookups (slot 3)
/// * `id_to_name_mapping` - SMT root for account ID → name lookups (slot 4)
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

/// Test helper struct to encapsulate common test operations for Miden ID registry.
///
/// This helper provides high-level methods for testing the registry contract,
/// including account creation, contract deployment, initialization, name registration,
/// and state queries.
///
/// # Fields
///
/// * `client` - Miden client instance for blockchain operations
/// * `endpoint` - Network endpoint (testnet, localhost, etc.)
/// * `keystore` - Filesystem-based keystore for account management
/// * `registry_contract` - The deployed registry contract account (if deployed)
/// * `owner_account` - The registry owner account (if initialized)
/// * `faucet_account` - The payment token faucet (if created)
pub struct RegistryTestHelper {
    pub client: Client,
    pub endpoint: Endpoint,
    pub keystore: FilesystemKeyStore<StdRng>,
    pub registry_contract: Option<Account>,
    pub owner_account: Option<Account>,
    pub faucet_account: Option<Account>,
}

// ================================================================================================
// SETUP & LIFECYCLE
// ================================================================================================

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
        // Skip delete_keystore_and_store() to preserve existing accounts and notes

        let endpoint = Endpoint::testnet();
        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(e) => {
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
        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(e) => {
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

    /// Sets up a test environment with a deployed registry contract.
    ///
    /// This convenience method combines clean state initialization, network sync,
    /// and contract deployment into a single step. The contract is NOT initialized yet.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Helper with deployed registry contract
    /// * `Err(ClientError)` - Network or deployment failure
    ///
    /// # Process
    ///
    /// 1. Creates clean test environment (via `new()`)
    /// 2. Syncs with network
    /// 3. Deploys registry contract as public, immutable
    pub async fn setup_with_deployed_contract() -> Result<Self, ClientError> {
        let mut helper = Self::new().await?;
        helper.sync_network().await?;
        helper.deploy_registry_contract().await?;
        Ok(helper)
    }

    /// Sets up a fully initialized registry with payment token faucet.
    ///
    /// This is the most complete setup method, creating a fully functional registry
    /// ready for name registration testing with payment validation.
    ///
    /// # Arguments
    ///
    /// * `owner_name` - Descriptive name for the owner account (used for debugging)
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * `Self` - Helper instance with deployed and initialized registry
    /// * `Account` - Owner account that controls the registry
    /// * `Account` - Faucet account for payment token (REG token)
    ///
    /// # Process
    ///
    /// 1. Deploys registry contract
    /// 2. Creates owner account
    /// 3. Creates fungible faucet (symbol: "REG", decimals: 8, max: 1B)
    /// 4. Initializes registry with owner, faucet, and price of 100
    pub async fn setup_initialized_with_faucet(
        owner_name: &str,
    ) -> Result<(Self, Account, Account), ClientError> {
        let mut helper = Self::setup_with_deployed_contract().await?;
        let owner = helper.create_account(owner_name).await?;
        let faucet = helper.create_faucet("REG", 8, 1_000_000_000).await?;
        helper
            .initialize_registry_with_faucet(&owner, Some(&faucet))
            .await?;
        Ok((helper, owner, faucet))
    }

    /// Synchronizes the local client state with the Miden network.
    ///
    /// This method fetches the latest blockchain state and updates the local client's
    /// view of accounts, notes, and transactions. Call this after transactions to see
    /// the updated state.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully synced
    /// * `Err(ClientError)` - Network communication error
    pub async fn sync_network(&mut self) -> Result<(), ClientError> {
        self.client.sync_state().await?;
        Ok(())
    }

    // ================================================================================================
    // HELPER UTILITIES
    // ================================================================================================

    /// Loads the registry contract code and creates a library.
    ///
    /// This helper consolidates the repeated pattern of loading the contract code
    /// and creating a library namespace.
    ///
    /// # Returns
    ///
    /// The compiled contract library for "miden_id::registry" namespace
    ///
    /// # Panics
    ///
    /// Panics if the contract file cannot be read or library creation fails
    fn load_registry_library(&self) -> miden_objects::assembly::Library {
        let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        create_library(contract_code, "miden_id::registry").unwrap()
    }

    /// Converts a Word to NoteInputs by extracting all 4 Felts.
    ///
    /// This helper consolidates the repeated pattern of extracting Felts from a Word
    /// and creating NoteInputs.
    ///
    /// # Arguments
    ///
    /// * `word` - The Word to convert
    ///
    /// # Returns
    ///
    /// NoteInputs containing the 4 Felts from the Word
    ///
    /// # Panics
    ///
    /// Panics if the Word doesn't have 4 elements or NoteInputs creation fails
    fn word_to_note_inputs(word: &Word) -> NoteInputs {
        let felts: Vec<Felt> = (0..4).map(|i| *word.get(i).unwrap()).collect();
        NoteInputs::new(felts).unwrap()
    }

    // ================================================================================================
    // ACCOUNT & CONTRACT MANAGEMENT
    // ================================================================================================

    /// Creates a new basic wallet account.
    ///
    /// This method creates a standard updatable account on the Miden network
    /// and waits for it to be confirmed.
    ///
    /// # Arguments
    ///
    /// * `_role` - Descriptive label for the account (currently unused, for debugging)
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - The newly created account
    /// * `Err(ClientError)` - Account creation failed
    pub async fn create_account(&mut self, _role: &str) -> Result<Account, ClientError> {
        let (account, _) = create_basic_account(&mut self.client, self.keystore.clone()).await?;
        sleep(Duration::from_secs(3)).await;
        Ok(account)
    }

    /// Creates a fungible faucet account for testing payment validation.
    ///
    /// A faucet is a special account type that can mint fungible tokens.
    /// This is useful for testing payment flows in the registry.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Token symbol (e.g., "REG", "TEST")
    /// * `decimals` - Number of decimal places (typically 8)
    /// * `max_supply` - Maximum number of tokens that can be minted
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - The newly created faucet account
    /// * `Err(ClientError)` - Faucet creation failed
    pub async fn create_faucet(
        &mut self,
        symbol: &str,
        decimals: u8,
        max_supply: u64,
    ) -> Result<Account, ClientError> {
        let (faucet, _) = create_faucet_account(
            &mut self.client,
            self.keystore.clone(),
            symbol,
            decimals,
            max_supply,
        )
        .await?;
        sleep(Duration::from_secs(3)).await;
        Ok(faucet)
    }

    /// Deploys the Miden ID registry contract as public and immutable.
    ///
    /// Reads the contract code from `./masm/accounts/miden_id.masm` and deploys it
    /// to the network. The contract is stored in `self.registry_contract` for later use.
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - The deployed registry contract account
    /// * `Err(ClientError)` - Deployment failed
    ///
    /// # Contract Properties
    ///
    /// - **Public**: Anyone can interact with the contract
    /// - **Immutable**: Contract code cannot be changed after deployment
    pub async fn deploy_registry_contract(&mut self) -> Result<Account, ClientError> {
        let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        let registry_contract =
            create_public_immutable_contract(&mut self.client, &registry_code).await?;

        self.client.add_account(&registry_contract, false).await?;

        self.registry_contract = Some(registry_contract.clone());
        sleep(Duration::from_secs(5)).await;

        Ok(registry_contract)
    }

    /// Initializes the registry without payment validation (free registration).
    ///
    /// This is a convenience wrapper that initializes the registry with price = 0,
    /// allowing free name registrations without payment tokens.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - The account that will own and control the registry
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Registry initialized successfully
    /// * `Err(ClientError)` - Initialization failed
    pub async fn initialize_registry(
        &mut self,
        owner_account: &Account,
    ) -> Result<(), ClientError> {
        self.initialize_registry_with_faucet(owner_account, None)
            .await
    }

    /// Initializes the registry contract with owner and optional payment token.
    ///
    /// This method sets up the registry with the owner account and configures payment
    /// validation. If a faucet is provided, registrations require payment; otherwise,
    /// registrations are free.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - The account that will own and control the registry
    /// * `faucet_account` - Optional payment token faucet for paid registrations
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Registry initialized successfully
    /// * `Err(ClientError)` - Initialization failed
    ///
    /// # Behavior
    ///
    /// - **With faucet**: Price set to 100 tokens, payment required for registration
    /// - **Without faucet**: Price set to 0, free registration
    ///
    /// # Process
    ///
    /// 1. Loads the registry contract code and init note template
    /// 2. Creates note inputs with payment token ID and price
    /// 3. Executes initialization transaction
    /// 4. Stores owner and faucet accounts in helper state
    /// 5. Syncs with network to reflect changes
    pub async fn initialize_registry_with_faucet(
        &mut self,
        owner_account: &Account,
        faucet_account: Option<&Account>,
    ) -> Result<(), ClientError> {
        // Use faucet if provided, otherwise use owner account
        let payment_token = faucet_account.unwrap_or(owner_account);
        let token_prefix = payment_token.id().prefix().as_felt();
        let token_suffix = payment_token.id().suffix();

        // Set price based on whether we have a faucet
        // With faucet: price = 100 (payment required)
        // Without faucet: price = 0 (free registration)
        let price = if faucet_account.is_some() { 100 } else { 0 };

        // Create dynamic init note with payment token
        let init_note_code = get_note_code("init".to_string());
        let contract_library = self.load_registry_library();
        let empty_assets = NoteAssets::new(vec![]).unwrap();
        let inputs = NoteInputs::new(vec![token_prefix, token_suffix, Felt::new(price)]).unwrap();

        let init_note = create_public_note_with_library_and_inputs(
            &mut self.client,
            init_note_code,
            owner_account.clone(),
            empty_assets,
            contract_library,
            inputs,
        )
        .await
        .unwrap();

        sleep(Duration::from_secs(5)).await;

        self.execute_transaction_with_note(init_note).await?;
        self.owner_account = Some(owner_account.clone());
        if let Some(faucet) = faucet_account {
            self.faucet_account = Some(faucet.clone());
        }

        sleep(Duration::from_secs(8)).await;
        self.sync_network().await?;

        Ok(())
    }

    /// Updates the registration price for the Miden ID registry.
    ///
    /// This method allows the registry owner to update the price that users must pay
    /// to register a new name. The update is performed by creating a public note
    /// containing the new price and executing it as a transaction on the registry contract.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - The account authorized to update the price (must be the registry owner)
    /// * `new_price` - The new registration price in token units (e.g., 100 for 100 tokens)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Price updated successfully and changes synced with the network
    /// * `Err(ClientError)` - Transaction failed or network sync error
    ///
    /// # Process
    ///
    /// 1. Loads the registry contract code and creates a library namespace
    /// 2. Reads the `update_price.masm` note template
    /// 3. Creates a public note with the new price as input
    /// 4. Executes the note as a transaction on the registry contract
    /// 5. Waits for transaction finalization and syncs state
    ///
    /// # Security
    ///
    /// - Only the registry owner can successfully execute this operation
    /// - The contract validates ownership before updating the price
    /// - Non-owner attempts will fail during transaction execution
    pub async fn update_price(
        &mut self,
        owner_account: &Account,
        new_price: u64,
    ) -> Result<(), ClientError> {
        // Create note to update price
        let update_price_note_code = get_note_code("update_price".to_string());
        let contract_library = self.load_registry_library();
        let empty_assets = NoteAssets::new(vec![]).unwrap();
        let inputs = NoteInputs::new(vec![
            Felt::new(new_price),
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
        ])
        .unwrap();

        let update_price_note = create_public_note_with_library_and_inputs(
            &mut self.client,
            update_price_note_code,
            owner_account.clone(),
            empty_assets,
            contract_library,
            inputs,
        )
        .await
        .unwrap();

        self.execute_transaction_with_note(update_price_note)
            .await?;

        sleep(Duration::from_secs(8)).await;
        self.sync_network().await?;

        Ok(())
    }

    /// Transfers ownership of the registry to a new owner account.
    ///
    /// This method allows the current registry owner to transfer ownership to another account.
    /// The transfer is performed by creating a public note containing the new owner's account ID
    /// and executing it as a transaction on the registry contract.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - The current owner account (must be the registry owner)
    /// * `new_owner` - The account that will become the new registry owner
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Ownership transferred successfully and changes synced
    /// * `Err(ClientError)` - Transaction failed or network sync error
    ///
    /// # Security
    ///
    /// - Only the current registry owner can transfer ownership
    /// - The contract validates ownership before executing the transfer
    /// - After transfer, only the new owner can perform owner-only operations
    pub async fn update_owner(
        &mut self,
        owner_account: &Account,
        new_owner: &Account,
    ) -> Result<(), ClientError> {
        let update_owner_note_code = get_note_code("update_owner".to_string());
        let contract_library = self.load_registry_library();
        let empty_assets = NoteAssets::new(vec![]).unwrap();
        let inputs = NoteInputs::new(vec![
            new_owner.id().suffix(),
            new_owner.id().prefix().as_felt(),
        ])
        .unwrap();

        let update_price_note = create_public_note_with_library_and_inputs(
            &mut self.client,
            update_owner_note_code,
            owner_account.clone(),
            empty_assets,
            contract_library,
            inputs,
        )
        .await
        .unwrap();

        self.execute_transaction_with_note(update_price_note)
            .await?;

        sleep(Duration::from_secs(8)).await;
        self.sync_network().await?;

        Ok(())
    }

    // ================================================================================================
    // STATE QUERY METHODS
    // ================================================================================================

    /// Retrieves the current registry contract account record from the client.
    ///
    /// This method fetches the latest account state from the local client database,
    /// which should be synced with the network using `sync_network()`.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(AccountRecord))` - Registry contract account record found
    /// * `Ok(None)` - Registry contract not found in local database
    /// * `Err(ClientError)` - Error querying the client database
    ///
    /// # Panics
    ///
    /// Panics if `registry_contract` is None (contract not deployed)
    pub async fn get_contract_state(&mut self) -> Result<Option<AccountRecord>, ClientError> {
        let registry_id = self.registry_contract.as_ref().unwrap().id();
        self.client.get_account(registry_id).await
    }

    /// Retrieves the registry account record, guaranteed to exist.
    ///
    /// This is a convenience method that wraps `require_contract_state()`, providing
    /// a more descriptive name for getting the registry account.
    ///
    /// # Returns
    ///
    /// * `Ok(AccountRecord)` - The registry contract account record
    /// * `Err(ClientError)` - Error or registry not found (panics)
    ///
    /// # Panics
    ///
    /// Panics if the registry contract is not found in the local database
    pub async fn get_registry_account(&mut self) -> Result<AccountRecord, ClientError> {
        self.require_contract_state().await
    }

    /// Internal helper to get contract state or panic if not found.
    ///
    /// This method ensures the registry contract exists in the local database,
    /// panicking if it's not found (indicating an error in test setup).
    ///
    /// # Returns
    ///
    /// * `Ok(AccountRecord)` - The registry contract account record
    /// * `Err(ClientError)` - Error querying the client database
    ///
    /// # Panics
    ///
    /// Panics with message "Registry contract not found" if the contract doesn't exist
    async fn require_contract_state(&mut self) -> Result<AccountRecord, ClientError> {
        self.get_contract_state()
            .await?
            .ok_or_else(|| panic!("Registry contract not found"))
    }

    /// Retrieves the initialization state and owner information from registry storage.
    ///
    /// This method reads both storage slot 0 (initialization flag) and slot 1 (owner info)
    /// to provide a complete picture of the registry's initialization state.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * `u64` - Initialization flag (0 = not initialized, 1 = initialized)
    /// * `u64` - Owner account ID prefix
    /// * `u64` - Owner account ID suffix
    ///
    /// # Storage Layout
    ///
    /// - **Slot 0**: `[0, 0, 0, initialized_flag]`
    /// - **Slot 1**: `[owner_prefix, owner_suffix, 0, 0]`
    pub fn get_initialization_state(&self, account_record: &AccountRecord) -> (u64, u64, u64) {
        let init_flag: Word = account_record
            .account()
            .storage()
            .get_item(0)
            .unwrap()
            .into();
        let initialized = init_flag.get(3).unwrap().as_int();

        let owner: Word = account_record
            .account()
            .storage()
            .get_item(1)
            .unwrap()
            .into();
        // Owner stored as Word [prefix, suffix, 0, 0] at slot 1
        let (owner_prefix, owner_suffix) = (
            owner.get(0).unwrap().as_int(), // prefix at index 0
            owner.get(1).unwrap().as_int(), // suffix at index 1
        );

        (initialized, owner_prefix, owner_suffix)
    }

    /// Retrieves the owner account ID from the registry contract storage.
    ///
    /// Reads storage slot 1 which contains the owner account information stored as
    /// a Word in the format `[prefix, suffix, 0, 0]`.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// A tuple `(owner_prefix, owner_suffix)` representing the owner's account ID components
    pub fn get_owner(&self, account_record: &AccountRecord) -> (u64, u64) {
        let owner: Word = account_record
            .account()
            .storage()
            .get_item(1)
            .unwrap()
            .into();
        let (owner_prefix, owner_suffix) = (
            owner.get(0).unwrap().as_int(), // prefix at index 0
            owner.get(1).unwrap().as_int(), // suffix at index 1
        );

        (owner_prefix, owner_suffix)
    }

    /// Retrieves the payment token account ID from registry storage slot 2.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// A tuple `(token_prefix, token_suffix)` representing the payment token's account ID components
    ///
    /// # Storage Layout
    ///
    /// - **Slot 2**: `[suffix, prefix, 0, 0]` (note the reversed order in storage)
    /// - Returns normalized as `(prefix, suffix)` for consistency
    pub fn get_payment_token_state(&self, account_record: &AccountRecord) -> (u64, u64) {
        let payment_token: Word = account_record
            .account()
            .storage()
            .get_item(2)
            .unwrap()
            .into();

        // Payment token stored as Word [suffix, prefix, 0, 0] at slot 2 due to Word reversal
        // But we return (prefix, suffix) tuple for consistency
        (
            payment_token.get(1).unwrap().as_int(), // prefix at index 1
            payment_token.get(0).unwrap().as_int(), // suffix at index 0
        )
    }

    /// Retrieves the registration price from registry storage slot 5.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// The registration price in token units (e.g., 100 for 100 tokens)
    ///
    /// # Storage Layout
    ///
    /// - **Slot 5**: `[price, 0, 0, 0]`
    pub fn get_price(&self, account_record: &AccountRecord) -> u64 {
        let price_word: Word = account_record
            .account()
            .storage()
            .get_item(5)
            .unwrap()
            .into();
        price_word.get(0).unwrap().as_int()
    }

    /// Retrieves the Sparse Merkle Tree (SMT) root hashes for name mappings.
    ///
    /// The registry uses two SMTs to maintain bidirectional name-to-ID mappings.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * `Option<Word>` - Name-to-ID mapping root (slot 3), None if not initialized
    /// * `Option<Word>` - ID-to-Name mapping root (slot 4), None if not initialized
    ///
    /// # Storage Layout
    ///
    /// - **Slot 3**: SMT root for name → account ID lookups
    /// - **Slot 4**: SMT root for account ID → name lookups
    pub fn get_registry_mapping_state(
        &self,
        account_record: &AccountRecord,
    ) -> (Option<Word>, Option<Word>) {
        let name_to_id = account_record
            .account()
            .storage()
            .get_item(3)
            .map(|item| item.into())
            .ok();

        let id_to_name = account_record
            .account()
            .storage()
            .get_item(4)
            .map(|item| item.into())
            .ok();

        (name_to_id, id_to_name)
    }

    /// Retrieves the complete contract state by aggregating all storage slots.
    ///
    /// This is a convenience method that calls all the individual state getter methods
    /// and combines them into a single `ContractState` struct for easy validation.
    ///
    /// # Arguments
    ///
    /// * `account_record` - The registry contract's account record
    ///
    /// # Returns
    ///
    /// A `ContractState` struct containing all registry storage data
    pub fn get_complete_contract_state(&self, account_record: &AccountRecord) -> ContractState {
        let (initialized, owner_prefix, owner_suffix) =
            self.get_initialization_state(account_record);
        let (token_prefix, token_suffix) = self.get_payment_token_state(account_record);
        let (name_to_id, id_to_name) = self.get_registry_mapping_state(account_record);

        ContractState {
            initialized,
            owner_prefix,
            owner_suffix,
            token_prefix,
            token_suffix,
            name_to_id_mapping: name_to_id,
            id_to_name_mapping: id_to_name,
        }
    }

    /// Retrieves the payment token account ID asynchronously.
    ///
    /// This is a convenience async wrapper that syncs the contract state and returns
    /// the payment token ID.
    ///
    /// # Returns
    ///
    /// * `Ok((prefix, suffix))` - Payment token account ID components
    /// * `Err(ClientError)` - Failed to fetch contract state
    pub async fn get_payment_token_id(&mut self) -> Result<(u64, u64), ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_payment_token_state(&contract_state))
    }

    /// Retrieves the registration price asynchronously.
    ///
    /// This is a convenience async wrapper that syncs the contract state and returns
    /// the current registration price.
    ///
    /// # Returns
    ///
    /// * `Ok(price)` - Current registration price in token units
    /// * `Err(ClientError)` - Failed to fetch contract state
    pub async fn get_registration_price(&mut self) -> Result<u64, ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_price(&contract_state))
    }

    // ================================================================================================
    // ENCODING/DECODING UTILITIES
    // ================================================================================================

    /// Encodes a name string into a Word for storage in the registry.
    ///
    /// Names are packed into a single Word (4 Felts) with the following layout:
    /// - Felt[0]: Name length
    /// - Felt[1-3]: ASCII characters, 7 characters per Felt (56 bits used per Felt)
    ///
    /// # Arguments
    ///
    /// * `name` - The name string to encode (max 20 characters, ASCII only)
    ///
    /// # Returns
    ///
    /// A Word containing the encoded name
    ///
    /// # Panics
    ///
    /// Panics if the name exceeds 20 characters
    ///
    /// # Format
    ///
    /// Word: `[length, chars_1-7, chars_8-14, chars_15-20]`
    pub fn encode_name_to_word(name: &str) -> Word {
        assert!(name.len() <= 20, "Name must not exceed 20 characters");

        let bytes = name.as_bytes();
        let mut felts = [Felt::ZERO; 4];

        // Felt[0]: Store name length
        felts[0] = Felt::new(bytes.len() as u64);

        // Felt[1-3]: Pack 7 ASCII characters per felt (56 bits used)
        for (i, chunk) in bytes.chunks(7).enumerate() {
            if i >= 3 {
                break;
            }

            let mut value = 0u64;
            for (j, &byte) in chunk.iter().enumerate() {
                value |= (byte as u64) << (j * 8);
            }
            felts[i + 1] = Felt::new(value);
        }

        Word::new(felts)
    }

    /// Decodes a Word back into a name string.
    ///
    /// This is the inverse operation of `encode_name_to_word`, extracting the ASCII
    /// characters from the packed Word representation.
    ///
    /// # Arguments
    ///
    /// * `word` - The Word containing the encoded name
    ///
    /// # Returns
    ///
    /// The decoded name string, or empty string if the Word is empty
    ///
    /// # Format
    ///
    /// Expects Word in format: `[length, chars_1-7, chars_8-14, chars_15-20]`
    pub fn decode_name_word(word: &Word) -> String {
        let length = word.get(0).map(|f| f.as_int() as usize).unwrap_or(0);
        if length == 0 {
            return String::new();
        }

        let mut bytes = Vec::new();

        // Extract ASCII characters from felts 1-3
        for idx in 1..4 {
            if let Some(felt) = word.get(idx) {
                let mut value = felt.as_int();
                for _ in 0..7 {
                    if bytes.len() >= length {
                        break;
                    }
                    let byte = (value & 0xFF) as u8;
                    if byte == 0 {
                        break;
                    }
                    bytes.push(byte);
                    value >>= 8;
                }
            }
            if bytes.len() >= length {
                break;
            }
        }

        String::from_utf8(bytes).unwrap_or_default()
    }

    /// Encodes an account ID into Word format for storage queries.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to encode
    ///
    /// # Returns
    ///
    /// A Word in format: `[suffix, prefix, 0, 0]`
    fn encode_account_to_word(account: &Account) -> Word {
        Word::new([
            Felt::new(account.id().suffix().as_int()),
            Felt::new(account.id().prefix().as_felt().as_int()),
            Felt::ZERO,
            Felt::ZERO,
        ])
    }

    /// Decodes an account Word back to (prefix, suffix) tuple.
    ///
    /// # Arguments
    ///
    /// * `word` - The Word containing account data in format `[suffix, prefix, 0, 0]`
    ///
    /// # Returns
    ///
    /// A tuple `(prefix, suffix)` representing the account ID components
    fn decode_account_word(word: &Word) -> (u64, u64) {
        let suffix = word.get(0).map(|felt| felt.as_int()).unwrap_or(0);
        let prefix = word.get(1).map(|felt| felt.as_int()).unwrap_or(0);
        (prefix, suffix)
    }

    /// Checks if a Word contains all zeros.
    ///
    /// Used to determine if a storage value is empty/uninitialized.
    ///
    /// # Arguments
    ///
    /// * `word` - The Word to check
    ///
    /// # Returns
    ///
    /// `true` if all 4 Felts in the Word are zero, `false` otherwise
    pub fn is_zero_word(word: &Word) -> bool {
        (0..4).all(|idx| word.get(idx).map(|felt| felt.as_int()).unwrap_or(0) == 0)
    }

    // ================================================================================================
    // REGISTRATION OPERATIONS
    // ================================================================================================

    /// Registers a name for an account without payment (free registration).
    ///
    /// This is a convenience wrapper for free name registration, typically used
    /// when the registry is initialized without a payment faucet.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to register the name for
    /// * `name` - The name to register (max 20 characters, ASCII)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Name registered successfully
    /// * `Err(ClientError)` - Registration failed
    pub async fn register_name_for_account(
        &mut self,
        account: &Account,
        name: &str,
    ) -> Result<(), ClientError> {
        self.register_name_for_account_with_payment(account, name, None)
            .await
    }

    /// Registers a name for an account with optional payment validation.
    ///
    /// This method creates a registration note with the encoded name and optional
    /// payment assets, then executes it on the registry contract.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to register the name for
    /// * `name` - The name to register (max 20 characters, ASCII)
    /// * `payment_amount` - Optional payment amount in tokens (e.g., Some(100) for 100 tokens)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Name registered successfully
    /// * `Err(ClientError)` - Registration failed
    ///
    /// # Process
    ///
    /// 1. Encodes the name to Word format
    /// 2. Loads register_name.masm note template
    /// 3. Creates note assets (payment tokens if amount provided)
    /// 4. Passes name as note inputs (4 Felts)
    /// 5. Executes registration transaction
    /// 6. Waits for finalization and syncs network
    ///
    /// # Panics
    ///
    /// Panics if `payment_amount` is provided but no faucet account is set in the helper
    pub async fn register_name_for_account_with_payment(
        &mut self,
        account: &Account,
        name: &str,
        payment_amount: Option<u64>,
    ) -> Result<(), ClientError> {
        let name_word = Self::encode_name_to_word(name);

        // Load register_name note from file instead of hardcoding
        let register_note_code = get_note_code("register_name".to_string());

        println!("Registering name '{}' for account {}", name, account.id());

        let contract_library = self.load_registry_library();

        // Create note assets based on payment requirement
        let note_assets = if let Some(amount) = payment_amount {
            // Get the payment token (faucet) account
            let faucet_account = self
                .faucet_account
                .as_ref()
                .expect("Faucet account must be set for paid registration");

            // Create fungible asset with the payment amount
            let payment_asset = FungibleAsset::new(faucet_account.id(), amount)
                .expect("Failed to create payment asset");

            NoteAssets::new(vec![payment_asset.into()]).unwrap()
        } else {
            // No payment required
            NoteAssets::new(vec![]).unwrap()
        };

        // Pass the name as note input (Word format)
        let inputs = Self::word_to_note_inputs(&name_word);

        let register_note = create_public_note_with_library_and_inputs(
            &mut self.client,
            register_note_code,
            account.clone(),
            note_assets,
            contract_library,
            inputs,
        )
        .await
        .unwrap();

        sleep(Duration::from_secs(5)).await;
        self.execute_transaction_with_note(register_note).await?;

        sleep(Duration::from_secs(15)).await;
        self.sync_network().await?;

        Ok(())
    }

    // ================================================================================================
    // LOOKUP OPERATIONS
    // ================================================================================================

    /// Checks if a name is registered in the registry.
    ///
    /// Performs a forward lookup (name → account ID) by querying storage slot 3.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to check
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Name is registered
    /// * `Ok(false)` - Name is not registered
    /// * `Err(ClientError)` - Query failed
    pub async fn is_name_registered(&mut self, name: &str) -> Result<bool, ClientError> {
        Ok(self.get_account_word_for_name(name).await?.is_some())
    }

    /// Retrieves the account ID associated with a name (forward lookup).
    ///
    /// Queries the name-to-ID mapping in storage slot 3 (SMT).
    ///
    /// # Arguments
    ///
    /// * `name` - The name to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some((prefix, suffix)))` - Name is registered, returns account ID components
    /// * `Ok(None)` - Name is not registered
    /// * `Err(ClientError)` - Query failed
    pub async fn get_account_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<(u64, u64)>, ClientError> {
        match self.get_account_word_for_name(name).await? {
            Some(word) => Ok(Some(Self::decode_account_word(&word))),
            None => Ok(None),
        }
    }

    /// Retrieves the raw account Word for a name from storage slot 3.
    ///
    /// This is a low-level method that queries the SMT directly without decoding.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Word))` - Raw account data Word
    /// * `Ok(None)` - Name not found or Word is all zeros
    /// * `Err(ClientError)` - Query failed
    pub async fn get_account_word_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<Word>, ClientError> {
        let contract_state = match self.get_contract_state().await? {
            Some(state) => state,
            None => return Ok(None),
        };

        let storage = contract_state.account().storage();
        let key = Self::encode_name_to_word(name);
        let value = storage.get_map_item(3, key)?;

        if Self::is_zero_word(&value) {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    /// Checks if an account has a registered name (reverse lookup).
    ///
    /// Queries the ID-to-name mapping in storage slot 4.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to check
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Account has a registered name
    /// * `Ok(false)` - Account has no registered name
    /// * `Err(ClientError)` - Query failed
    pub async fn has_name_for_address(&mut self, account: &Account) -> Result<bool, ClientError> {
        Ok(self.get_name_word_for_account(account).await?.is_some())
    }

    /// Retrieves the name associated with an account (reverse lookup).
    ///
    /// Queries the ID-to-name mapping in storage slot 4 (SMT).
    ///
    /// # Arguments
    ///
    /// * `account` - The account to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some(name))` - Account has a registered name
    /// * `Ok(None)` - Account has no registered name
    /// * `Err(ClientError)` - Query failed
    pub async fn get_name_for_address(
        &mut self,
        account: &Account,
    ) -> Result<Option<String>, ClientError> {
        match self.get_name_word_for_account(account).await? {
            Some(word) => Ok(Some(Self::decode_name_word(&word))),
            None => Ok(None),
        }
    }

    /// Retrieves the raw name Word for an account from storage slot 4.
    ///
    /// This is a low-level method that queries the SMT directly without decoding.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Word))` - Raw name data Word
    /// * `Ok(None)` - Account has no name or Word is all zeros
    /// * `Err(ClientError)` - Query failed
    pub async fn get_name_word_for_account(
        &mut self,
        account: &Account,
    ) -> Result<Option<Word>, ClientError> {
        let contract_state = match self.get_contract_state().await? {
            Some(state) => state,
            None => return Ok(None),
        };

        let storage = contract_state.account().storage();
        let key = Self::encode_account_to_word(account);
        let value = storage.get_map_item(4, key)?;

        if Self::is_zero_word(&value) {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    // ================================================================================================
    // INTERNAL HELPERS
    // ================================================================================================

    /// Executes a transaction with a note on the registry contract.
    ///
    /// This internal helper creates and submits a transaction that includes the provided
    /// note. It uses the nop.masm script as the transaction script.
    ///
    /// # Arguments
    ///
    /// * `note` - The note to include in the transaction
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Transaction submitted successfully
    /// * `Err(ClientError)` - Transaction creation or submission failed
    ///
    /// # Process
    ///
    /// 1. Loads nop.masm transaction script
    /// 2. Creates transaction request with unauthenticated note
    /// 3. Submits transaction to the registry contract
    async fn execute_transaction_with_note(&mut self, note: Note) -> Result<(), ClientError> {
        let nop_script_code = fs::read_to_string(Path::new(paths::NOP_SCRIPT)).unwrap();
        let transaction_script = create_tx_script(nop_script_code, None).unwrap();

        let request = TransactionRequestBuilder::new()
            .unauthenticated_input_notes([(note, None)])
            .custom_script(transaction_script)
            .build()
            .unwrap();

        let registry_id = self.registry_contract.as_ref().unwrap().id();
        self.client
            .submit_new_transaction(registry_id, request)
            .await?;

        Ok(())
    }
}

// ================================================================================================
// FREE FUNCTIONS
// ================================================================================================

/// Reads a MASM script file from the scripts directory.
///
/// # Arguments
///
/// * `script_name` - Name of the script file without the `.masm` extension
///
/// # Returns
///
/// The script content as a String
///
/// # Panics
///
/// Panics if the script file cannot be read or does not exist
pub fn get_script_code(script_name: String) -> String {
    // Construct the file path and read the MASM script content
    // Used to load transaction scripts like "nop.masm" for executing transactions
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::SCRIPTS_DIR,
        script_name
    )))
    .unwrap()
}

/// Reads a MASM note file from the notes directory.
///
/// # Arguments
///
/// * `note_name` - Name of the note file without the `.masm` extension
///
/// # Returns
///
/// The note content as a String
///
/// # Panics
///
/// Panics if the note file cannot be read or does not exist
pub fn get_note_code(note_name: String) -> String {
    // Construct the file path and read the MASM note content
    // Used to load note templates like "init.masm", "register_name.masm", "update_price.masm", etc.
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::NOTES_DIR,
        note_name
    )))
    .unwrap()
}
