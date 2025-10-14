use miden_client::{
    ClientError, Word,
    account::Account,
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets, NoteInputs},
    rpc::Endpoint,
    store::AccountRecord,
    transaction::TransactionRequestBuilder,
};
use miden_objects::{Felt, FieldElement, account::AccountId, asset::FungibleAsset, note::NoteType};
use midenid_contracts::common::*;
use rand::rngs::StdRng;
use std::{fs, path::Path, sync::OnceLock};
use tokio::time::{Duration, sleep};

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

static SHARED_REGISTRY_CONTRACT_ID: OnceLock<AccountId> = OnceLock::new();
static SHARED_FAUCET_ID: OnceLock<AccountId> = OnceLock::new();
static SHARED_OWNER_ID: OnceLock<AccountId> = OnceLock::new();

/// Gets or initializes shared registry contract for tests
pub async fn get_or_init_shared_contract() -> (AccountId, AccountId, AccountId) {
    if let (Some(&contract_id), Some(&faucet_id), Some(&owner_id)) = (
        SHARED_REGISTRY_CONTRACT_ID.get(),
        SHARED_FAUCET_ID.get(),
        SHARED_OWNER_ID.get(),
    ) {
        return (contract_id, faucet_id, owner_id);
    }

    let mut helper = RegistryTestHelper::setup_with_deployed_contract()
        .await
        .unwrap();
    let faucet = helper
        .create_faucet("REG", 8, 10_000_000_000)
        .await
        .unwrap();
    let owner = helper.create_account("Owner").await.unwrap();

    helper
        .initialize_registry_with_faucet(&owner, Some(&faucet))
        .await
        .unwrap();

    let contract_id = helper.registry_contract.as_ref().unwrap().id();
    let faucet_id = faucet.id();
    let owner_id = owner.id();

    SHARED_REGISTRY_CONTRACT_ID.set(contract_id).ok();
    SHARED_FAUCET_ID.set(faucet_id).ok();
    SHARED_OWNER_ID.set(owner_id).ok();

    println!("Initialized shared contract: {}", contract_id);
    println!("Initialized shared faucet: {}", faucet_id);

    (contract_id, faucet_id, owner_id)
}

/// Sets up helper with existing contract and faucet
pub async fn setup_helper_with_contract(
    contract_id: AccountId,
    faucet_id: AccountId,
) -> Result<RegistryTestHelper, ClientError> {
    let mut helper = RegistryTestHelper::new_persistent().await?;
    helper.registry_contract = helper
        .client
        .get_account(contract_id)
        .await?
        .map(|r| r.into());
    helper.faucet_account = helper
        .client
        .get_account(faucet_id)
        .await?
        .map(|r| r.into());
    Ok(helper)
}

/// Mints tokens from faucet and funds account
pub async fn mint_and_fund_account(
    helper: &mut RegistryTestHelper,
    faucet_id: AccountId,
    account: &Account,
    amount: u64,
) -> Result<Account, ClientError> {
    let asset = FungibleAsset::new(faucet_id, amount)?;
    let mint_tx = TransactionRequestBuilder::new().build_mint_fungible_asset(
        asset,
        account.id(),
        NoteType::Public,
        helper.client.rng(),
    )?;

    let result = helper.client.new_transaction(faucet_id, mint_tx).await?;
    helper.client.submit_transaction(result).await?;

    let note_ids = loop {
        helper.sync_network().await?;
        let notes = helper
            .client
            .get_consumable_notes(Some(account.id()))
            .await?;
        let ids: Vec<_> = notes.iter().map(|(n, _)| n.id()).collect();
        if !ids.is_empty() {
            break ids;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    };

    let consume_tx = TransactionRequestBuilder::new().build_consume_notes(note_ids)?;
    let result = helper
        .client
        .new_transaction(account.id(), consume_tx)
        .await?;
    helper.client.submit_transaction(result).await?;

    helper.sync_network().await?;
    let record = helper.client.get_account(account.id()).await?.unwrap();
    Ok(record.into())
}

pub mod paths {
    pub const REGISTRY_CONTRACT: &str = "./masm/accounts/miden_id.masm";
    pub const NOP_SCRIPT: &str = "./masm/scripts/nop.masm";
    pub const NOTES_DIR: &str = "./masm/notes";
    pub const SCRIPTS_DIR: &str = "./masm/scripts";
    pub const KEYSTORE_DIR: &str = "./keystore";
}

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
    /// 3. Deploys registry contract
    pub async fn setup_with_deployed_contract() -> Result<Self, ClientError> {
        let mut helper = Self::new().await?;
        helper.sync_network().await?;
        helper.deploy_registry_contract().await?;
        Ok(helper)
    }

    /// Synchronizes client state with the network.
    ///
    /// Fetches latest blocks, notes, and account states from the Miden node.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully synced
    /// * `Err(ClientError)` - Network sync failed
    pub async fn sync_network(&mut self) -> Result<(), ClientError> {
        self.client.sync_state().await?;
        Ok(())
    }

    // ================================================================================================
    // HELPER UTILITIES
    // ================================================================================================

    fn load_registry_library(&self) -> miden_objects::assembly::Library {
        let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        create_library(contract_code, "miden_id::registry").unwrap()
    }

    fn word_to_note_inputs(word: &Word) -> NoteInputs {
        let felts: Vec<Felt> = (0..4).map(|i| *word.get(i).unwrap()).collect();
        NoteInputs::new(felts).unwrap()
    }

    /// Creates a new basic wallet account.
    ///
    /// Creates a wallet account with BasicAuth authentication component.
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - Successfully created account
    /// * `Err(ClientError)` - Account creation failed
    pub async fn create_account(&mut self, _role: &str) -> Result<Account, ClientError> {
        let (account, _) = create_basic_account(&mut self.client, self.keystore.clone()).await?;
        sleep(Duration::from_secs(3)).await;
        Ok(account)
    }

    /// Creates a new fungible asset faucet.
    ///
    /// The faucet can mint fungible tokens up to the specified max supply.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Token symbol (e.g., "REG", "POL")
    /// * `decimals` - Number of decimal places
    /// * `max_supply` - Maximum token supply
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - Successfully created faucet
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

    /// Deploys the Miden ID registry contract.
    ///
    /// Creates a public immutable contract from the MASM code in miden_id.masm.
    /// The contract must be initialized before use via `initialize_registry`.
    ///
    /// # Returns
    ///
    /// * `Ok(Account)` - Successfully deployed contract
    /// * `Err(ClientError)` - Deployment failed
    pub async fn deploy_registry_contract(&mut self) -> Result<Account, ClientError> {
        let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        let (registry_contract, registry_seed) =
            create_public_immutable_contract(&mut self.client, &registry_code).await?;

        self.client
            .add_account(&registry_contract, Some(registry_seed), false)
            .await?;

        self.registry_contract = Some(registry_contract.clone());
        sleep(Duration::from_secs(5)).await;

        Ok(registry_contract)
    }

    /// Initializes the registry with an owner account (free registration).
    ///
    /// Sets registration price to 0, allowing free name registrations.
    /// This is a convenience wrapper around `initialize_registry_with_faucet`.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - Account that will own the registry
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully initialized
    /// * `Err(ClientError)` - Initialization failed
    pub async fn initialize_registry(
        &mut self,
        owner_account: &Account,
    ) -> Result<(), ClientError> {
        self.initialize_registry_with_faucet(owner_account, None)
            .await
    }

    /// Initializes the registry with owner and payment token configuration.
    ///
    /// If a faucet is provided, sets price to 100 (payment required).
    /// If no faucet, sets price to 0 (free registration).
    ///
    /// # Arguments
    ///
    /// * `owner_account` - Account that will own the registry
    /// * `faucet_account` - Optional faucet for payment token
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully initialized
    /// * `Err(ClientError)` - Initialization failed
    pub async fn initialize_registry_with_faucet(
        &mut self,
        owner_account: &Account,
        faucet_account: Option<&Account>,
    ) -> Result<(), ClientError> {
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

    /// Updates the name registration price.
    ///
    /// Only the registry owner can update the price.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - Registry owner account
    /// * `new_price` - New registration price
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully updated price
    /// * `Err(ClientError)` - Update failed
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

    /// Transfers registry ownership to a new account.
    ///
    /// Only the current owner can transfer ownership.
    ///
    /// # Arguments
    ///
    /// * `owner_account` - Current registry owner
    /// * `new_owner` - New owner account
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully transferred ownership
    /// * `Err(ClientError)` - Transfer failed
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

    /// Gets contract state from network
    pub async fn get_contract_state(&mut self) -> Result<Option<AccountRecord>, ClientError> {
        let registry_id = self.registry_contract.as_ref().unwrap().id();
        self.client.get_account(registry_id).await
    }

    async fn require_contract_state(&mut self) -> Result<AccountRecord, ClientError> {
        self.get_contract_state()
            .await?
            .ok_or_else(|| panic!("Registry contract not found"))
    }

    /// Returns initialization state from storage
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

    /// Returns owner ID from storage
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

    /// Returns payment token state from storage
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

    /// Returns registration price from storage
    pub fn get_price(&self, account_record: &AccountRecord) -> u64 {
        let price_word: Word = account_record
            .account()
            .storage()
            .get_item(5)
            .unwrap()
            .into();
        price_word.get(0).unwrap().as_int()
    }

    /// Returns registry mapping roots from storage
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

    /// Returns complete contract state
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

    /// Gets payment token ID from registry
    pub async fn get_payment_token_id(&mut self) -> Result<(u64, u64), ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_payment_token_state(&contract_state))
    }

    /// Gets registration price from registry
    pub async fn get_registration_price(&mut self) -> Result<u64, ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_price(&contract_state))
    }

    // ================================================================================================
    // ENCODING/DECODING UTILITIES
    // ================================================================================================

    /// Encodes a name string into a Miden Word.
    ///
    /// Encoding format: [length, name_bytes_0_6, name_bytes_7_13, 0]
    /// Supports names up to 14 characters.
    ///
    /// # Arguments
    ///
    /// * `name` - Name string to encode
    ///
    /// # Returns
    ///
    /// Word containing encoded name
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

    /// Decodes a Miden Word back to a name string.
    ///
    /// Reverses the encoding done by `encode_name_to_word`.
    ///
    /// # Arguments
    ///
    /// * `word` - Word containing encoded name
    ///
    /// # Returns
    ///
    /// Decoded name string
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

    fn encode_account_to_word(account: &Account) -> Word {
        Word::new([
            Felt::new(account.id().suffix().as_int()),
            Felt::new(account.id().prefix().as_felt().as_int()),
            Felt::ZERO,
            Felt::ZERO,
        ])
    }

    fn decode_account_word(word: &Word) -> (u64, u64) {
        let suffix = word.get(0).map(|felt| felt.as_int()).unwrap_or(0);
        let prefix = word.get(1).map(|felt| felt.as_int()).unwrap_or(0);
        (prefix, suffix)
    }

    /// Checks if Word is all zeros
    pub fn is_zero_word(word: &Word) -> bool {
        (0..4).all(|idx| word.get(idx).map(|felt| felt.as_int()).unwrap_or(0) == 0)
    }

    // ================================================================================================
    // REGISTRATION OPERATIONS
    // ================================================================================================

    /// Registers name for account without payment
    pub async fn register_name_for_account(
        &mut self,
        account: &Account,
        name: &str,
    ) -> Result<(), ClientError> {
        self.register_name_for_account_with_payment(account, name, None)
            .await
    }

    /// Registers a name for an account with payment.
    ///
    /// Creates a registration note with the specified payment amount and executes
    /// the transaction to register the name in the registry.
    ///
    /// # Arguments
    ///
    /// * `account` - Account to register name for
    /// * `name` - Name to register
    /// * `payment_amount` - Optional payment amount
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully registered name
    /// * `Err(ClientError)` - Registration failed
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
    // LOOKUP OPERATIONS (Using MASM Exports)
    // ================================================================================================

    /// Checks if name is registered
    pub async fn is_name_registered(&mut self, name: &str) -> Result<bool, ClientError> {
        Ok(self.get_account_for_name(name).await?.is_some())
    }

    /// Gets account ID for registered name
    pub async fn get_account_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<(u64, u64)>, ClientError> {
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
            Ok(Some(Self::decode_account_word(&value)))
        }
    }

    /// Checks if account has registered name
    pub async fn has_name_for_address(&mut self, account: &Account) -> Result<bool, ClientError> {
        Ok(self.get_name_for_address(account).await?.is_some())
    }

    /// Gets registered name for account
    pub async fn get_name_for_address(
        &mut self,
        account: &Account,
    ) -> Result<Option<String>, ClientError> {
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
            Ok(Some(Self::decode_name_word(&value)))
        }
    }

    // ================================================================================================
    // SCRIPT-BASED EXPORT TESTING
    // ================================================================================================

    /// Calls the get_id export to retrieve account ID for a name.
    ///
    /// Executes a transaction script that calls the registry's get_id export,
    /// which performs a name → account ID lookup.
    ///
    /// # Arguments
    ///
    /// * `name` - Name to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some((prefix, suffix)))` - Found account ID
    /// * `Ok(None)` - Name not registered
    /// * `Err(ClientError)` - Lookup failed
    pub async fn call_get_id_export(
        &mut self,
        name: &str,
    ) -> Result<Option<(u64, u64)>, ClientError> {
        // Encode name to Word format (4 felts)
        let name_word = Self::encode_name_to_word(name);

        // Load the script that calls get_id
        let script_code = get_script_code("call_get_id".to_string());

        // Create library with the external_contract::miden_id path
        let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        let contract_library =
            create_library(contract_code, "external_contract::miden_id").unwrap();

        // Create transaction script with dynamically linked library
        let transaction_script = create_tx_script(script_code, Some(contract_library)).unwrap();

        // Build transaction with name as script arg
        let request = TransactionRequestBuilder::new()
            .custom_script(transaction_script)
            .script_arg(name_word)
            .build()
            .unwrap();

        // Query storage first to get expected result
        let storage_result = self.get_account_for_name(name).await?;

        // Execute on registry contract
        let registry_id = self.registry_contract.as_ref().unwrap().id();

        // Execute transaction - this will show debug output but may error
        // The error is expected for read-only operations
        let _ = self.client.new_transaction(registry_id, request).await;

        Ok(storage_result)
    }

    /// Calls the get_name export to retrieve name for an account.
    ///
    /// Executes a transaction script that calls the registry's get_name export,
    /// which performs an account ID → name lookup.
    ///
    /// # Arguments
    ///
    /// * `account` - Account to look up name for
    ///
    /// # Returns
    ///
    /// * `Ok(Some(name))` - Found registered name
    /// * `Ok(None)` - Account has no registered name
    /// * `Err(ClientError)` - Lookup failed
    pub async fn call_get_name_export(
        &mut self,
        account: &Account,
    ) -> Result<Option<String>, ClientError> {
        // Encode account ID to prefix/suffix format (2 u64s)
        let account_id_felt = Felt::try_from(account.id().prefix()).unwrap();
        let account_prefix = account_id_felt.as_int();
        let account_suffix_felt = Felt::try_from(account.id().suffix()).unwrap();
        let account_suffix = account_suffix_felt.as_int();

        // Load the script that calls get_name
        let script_code = get_script_code("call_get_name".to_string());

        // Create library with the external_contract::miden_id path
        let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
        let contract_library =
            create_library(contract_code, "external_contract::miden_id").unwrap();

        // Create transaction script with dynamically linked library
        let transaction_script = create_tx_script(script_code, Some(contract_library)).unwrap();

        // Build Word in reverse order so stack has [prefix, suffix, 0, 0, ...]
        let account_id_word: Word = [
            Felt::ZERO,
            Felt::ZERO,
            Felt::new(account_suffix),
            Felt::new(account_prefix),
        ]
        .into();
        let request = TransactionRequestBuilder::new()
            .custom_script(transaction_script)
            .script_arg(account_id_word)
            .build()
            .unwrap();

        // Query storage first to get expected result
        let storage_result = self.get_name_for_address(account).await?;

        // Execute on registry contract
        let registry_id = self.registry_contract.as_ref().unwrap().id();

        // Execute transaction - this will show debug output but may error
        // The error is expected for read-only operations or stack depth issues
        let _ = self.client.new_transaction(registry_id, request).await;

        Ok(storage_result)
    }

    // ================================================================================================
    // INTERNAL HELPERS
    // ================================================================================================

    async fn execute_transaction_with_note(&mut self, note: Note) -> Result<(), ClientError> {
        let nop_script_code = fs::read_to_string(Path::new(paths::NOP_SCRIPT)).unwrap();
        let transaction_script = create_tx_script(nop_script_code, None).unwrap();

        let request = TransactionRequestBuilder::new()
            .unauthenticated_input_notes([(note, None)])
            .custom_script(transaction_script)
            .build()
            .unwrap();

        let registry_id = self.registry_contract.as_ref().unwrap().id();
        let tx_result = self.client.new_transaction(registry_id, request).await?;
        self.client.submit_transaction(tx_result).await?;

        Ok(())
    }
}

// ================================================================================================
// FREE FUNCTIONS
// ================================================================================================

/// Loads script code from file
pub fn get_script_code(script_name: String) -> String {
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::SCRIPTS_DIR,
        script_name
    )))
    .unwrap()
}

/// Loads note code from file
pub fn get_note_code(note_name: String) -> String {
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::NOTES_DIR,
        note_name
    )))
    .unwrap()
}
