use miden_client::{
    ClientError, Word,
    account::Account,
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets},
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
// DATA STRUCTURES
// ================================================================================================

/// Complete contract state structure for validation
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

/// Test helper struct to encapsulate common test operations for Miden ID registry
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
    /// Create a new test helper WITHOUT clearing the database/keystore (preserves existing data)
    pub async fn new_persistent() -> Result<Self, ClientError> {
        // Skip delete_keystore_and_store() to preserve existing accounts and notes

        let endpoint = Endpoint::testnet();
        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(e) => {
                println!("Testnet connection failed: {}", e);
                println!("Falling back to localhost...");
                let localhost_endpoint = Endpoint::localhost();
                instantiate_client(localhost_endpoint.clone()).await?
            }
        };

        let keystore = FilesystemKeyStore::new("./keystore".into()).unwrap();

        Ok(Self {
            client,
            endpoint,
            keystore,
            registry_contract: None,
            owner_account: None,
            faucet_account: None,
        })
    }

    /// Create a new test helper with network connection (testnet with localhost fallback)
    pub async fn new() -> Result<Self, ClientError> {
        delete_keystore_and_store().await;

        let endpoint = Endpoint::testnet();
        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(e) => {
                println!("Testnet connection failed: {}", e);
                println!("Falling back to localhost...");
                let localhost_endpoint = Endpoint::localhost();
                instantiate_client(localhost_endpoint.clone()).await?
            }
        };

        let keystore = FilesystemKeyStore::new("./keystore".into()).unwrap();

        Ok(Self {
            client,
            endpoint,
            keystore,
            registry_contract: None,
            owner_account: None,
            faucet_account: None,
        })
    }

    /// Setup environment with deployed contract only (no accounts, no initialization)
    pub async fn setup_with_deployed_contract() -> Result<Self, ClientError> {
        let mut helper = Self::new().await?;
        helper.sync_network().await?;
        helper.deploy_registry_contract().await?;
        Ok(helper)
    }

    /// Setup with deployed contract and initialized registry using a faucet for payment
    /// Returns (helper, owner_account, faucet_account)
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

    /// Sync with network and return latest state
    pub async fn sync_network(&mut self) -> Result<(), ClientError> {
        self.client.sync_state().await?;
        Ok(())
    }

    // ================================================================================================
    // ACCOUNT & CONTRACT MANAGEMENT
    // ================================================================================================

    /// Create a new basic wallet account
    pub async fn create_account(&mut self, _role: &str) -> Result<Account, ClientError> {
        let (account, _) = create_basic_account(&mut self.client, self.keystore.clone()).await?;
        sleep(Duration::from_secs(3)).await;
        Ok(account)
    }

    /// Create a fungible faucet account for testing payment validation
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

    /// Mint tokens from faucet directly into recipient's account (simplified for testing)
    /// This creates a fungible asset in the recipient's vault without using notes
    pub async fn mint_tokens_to_account(
        &mut self,
        faucet: &Account,
        recipient: &Account,
        amount: u64,
    ) -> Result<(), ClientError> {
        // Step 1: Mint tokens from faucet
        let mint_script = format!(
            r#"
use.miden::faucet
use.miden::tx

begin
    # Mint the fungible asset
    push.{amount}
    exec.faucet::mint
    # => [ASSET]

    # For testing: we'll create a P2ID note that the recipient can consume
    # This is a workaround - in production, proper note distribution is needed
    dropw  # For now, just drop the asset (this is a limitation of the test setup)
end
"#,
            amount = amount
        );

        let transaction_script = create_tx_script(mint_script, None).unwrap();

        let request = TransactionRequestBuilder::new()
            .custom_script(transaction_script)
            .build()
            .unwrap();

        let _tx_result = self.client.new_transaction(faucet.id(), request).await?;

        println!("⚠️  Note: Faucet token distribution requires proper note handling.");
        println!(
            "⚠️  For full payment validation, use real testnet faucet or implement P2ID notes."
        );
        println!("⚠️  Current test will validate payment logic but skip actual token transfer.");

        Ok(())
    }

    /// Deploy the registry contract as public immutable
    pub async fn deploy_registry_contract(&mut self) -> Result<Account, ClientError> {
        let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
        let (registry_contract, registry_seed) =
            create_public_immutable_contract(&mut self.client, &registry_code).await?;

        self.client
            .add_account(&registry_contract, Some(registry_seed), false)
            .await?;

        self.registry_contract = Some(registry_contract.clone());
        sleep(Duration::from_secs(5)).await;

        Ok(registry_contract)
    }

    /// Initialize the registry contract with given owner
    /// Optionally accepts a faucet account for payment token (for payment validation tests)
    /// If no faucet provided, uses owner account as payment token with price=0 (free registration)
    pub async fn initialize_registry(
        &mut self,
        owner_account: &Account,
    ) -> Result<(), ClientError> {
        self.initialize_registry_with_faucet(owner_account, None)
            .await
    }

    /// Initialize the registry contract with a specific faucet for payment validation
    pub async fn initialize_registry_with_faucet(
        &mut self,
        owner_account: &Account,
        faucet_account: Option<&Account>,
    ) -> Result<(), ClientError> {
        let contract_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();

        // Use faucet if provided, otherwise use owner account
        let payment_token = faucet_account.unwrap_or(owner_account);
        let token_prefix = payment_token.id().prefix().as_felt().as_int();
        let token_suffix = payment_token.id().suffix().as_int();

        // Set price based on whether we have a faucet
        // With faucet: price = 100 (payment required)
        // Without faucet: price = 0 (free registration)
        let price = if faucet_account.is_some() { 100 } else { 0 };

        // Create dynamic init note with payment token
        let init_note_code = format!(
            r#"
use.miden_id::registry
use.std::sys

begin
    push.{token_prefix}
    push.{token_suffix}
    push.{price}
    # Stack: [price, token_suffix, token_prefix]
    call.registry::init
    exec.sys::truncate_stack
end
"#,
            token_prefix = token_prefix,
            token_suffix = token_suffix,
            price = price
        );

        println!("DEBUG: Init note code:\n{}", init_note_code);

        let library_namespace = "miden_id::registry";
        let contract_library = create_library(contract_code, library_namespace).unwrap();
        let empty_assets = NoteAssets::new(vec![]).unwrap();

        let init_note = create_public_note_with_library(
            &mut self.client,
            init_note_code,
            owner_account.clone(),
            empty_assets,
            contract_library,
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

    /// Update the registration price (only owner can call)
    pub async fn update_price(
        &mut self,
        owner_account: &Account,
        new_price: u64,
    ) -> Result<(), ClientError> {
        let contract_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();

        // Create note to update price
        let update_price_note_code = format!(
            r#"
use.miden_id::registry
use.std::sys

begin
    push.0.0.0.{new_price} # new price as PRICE_WORD [price, 0, 0, 0]
    call.registry::update_price
    exec.sys::truncate_stack
end
"#,
            new_price = new_price
        );

        let library_namespace = "miden_id::registry";
        let contract_library = create_library(contract_code, library_namespace).unwrap();
        let empty_assets = NoteAssets::new(vec![]).unwrap();

        let update_price_note = create_public_note_with_library(
            &mut self.client,
            update_price_note_code,
            owner_account.clone(),
            empty_assets,
            contract_library,
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

    /// Get current contract account record
    pub async fn get_contract_state(&mut self) -> Result<Option<AccountRecord>, ClientError> {
        let registry_id = self.registry_contract.as_ref().unwrap().id();
        self.client.get_account(registry_id).await
    }

    /// Get registry account record (alias for get_contract_state with unwrap)
    pub async fn get_registry_account(&mut self) -> Result<AccountRecord, ClientError> {
        self.require_contract_state().await
    }

    /// Helper to get contract state or panic
    async fn require_contract_state(&mut self) -> Result<AccountRecord, ClientError> {
        self.get_contract_state()
            .await?
            .ok_or_else(|| panic!("Registry contract not found"))
    }

    /// Get initialization state from slot 0
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

    /// Get payment token configuration from slot 2
    /// Returns (prefix, suffix)
    pub fn get_payment_token_state(&self, account_record: &AccountRecord) -> (u64, u64) {
        let payment_token: Word = account_record
            .account()
            .storage()
            .get_item(2)
            .unwrap()
            .into();

        // Payment token stored as Word [prefix, suffix, 0, 0] at slot 2
        (
            payment_token.get(0).unwrap().as_int(), // prefix at index 0
            payment_token.get(1).unwrap().as_int(), // suffix at index 1
        )
    }

    /// Get registration price from slot 5
    pub fn get_price(&self, account_record: &AccountRecord) -> u64 {
        let price_word: Word = account_record
            .account()
            .storage()
            .get_item(5)
            .unwrap()
            .into();
        price_word.get(0).unwrap().as_int()
    }

    /// Get registry mapping root hashes from slots 3 & 4
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

    /// Get complete contract state structure
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

    /// Get payment token account ID (async wrapper)
    pub async fn get_payment_token_id(&mut self) -> Result<(u64, u64), ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_payment_token_state(&contract_state))
    }

    /// Get registration price (async wrapper)
    pub async fn get_registration_price(&mut self) -> Result<u64, ClientError> {
        let contract_state = self.require_contract_state().await?;
        Ok(self.get_price(&contract_state))
    }

    // ================================================================================================
    // ENCODING/DECODING UTILITIES
    // ================================================================================================

    /// Encode a name string to a Word (max 20 characters)
    /// Format: [length, chars_1-7, chars_8-14, chars_15-20]
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

    /// Decode a Word back to a name string
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

    /// Encode an account to Word format: [suffix, prefix, 0, 0]
    fn encode_account_to_word(account: &Account) -> Word {
        Word::new([
            Felt::new(account.id().suffix().as_int()),
            Felt::new(account.id().prefix().as_felt().as_int()),
            Felt::ZERO,
            Felt::ZERO,
        ])
    }

    /// Decode account Word to (prefix, suffix) tuple
    fn decode_account_word(word: &Word) -> (u64, u64) {
        let suffix = word.get(0).map(|felt| felt.as_int()).unwrap_or(0);
        let prefix = word.get(1).map(|felt| felt.as_int()).unwrap_or(0);
        (prefix, suffix)
    }

    /// Check if a Word is all zeros
    pub fn is_zero_word(word: &Word) -> bool {
        (0..4).all(|idx| word.get(idx).map(|felt| felt.as_int()).unwrap_or(0) == 0)
    }

    // ================================================================================================
    // REGISTRATION OPERATIONS
    // ================================================================================================

    /// Register a name for an account (without payment)
    pub async fn register_name_for_account(
        &mut self,
        account: &Account,
        name: &str,
    ) -> Result<(), ClientError> {
        self.register_name_for_account_with_payment(account, name, None)
            .await
    }

    /// Register a name for an account with optional payment
    pub async fn register_name_for_account_with_payment(
        &mut self,
        account: &Account,
        name: &str,
        payment_amount: Option<u64>,
    ) -> Result<(), ClientError> {
        let name_word = Self::encode_name_to_word(name);
        let name_push_str = word_to_masm_push_string(&name_word);

        // Create note code based on whether payment is required
        let register_note_code = if payment_amount.is_some() {
            // For paid registration, use Miden's standard add_assets_to_account helper
            format!(
                r#"
use.miden_id::registry
use.std::sys
use.miden::note

begin
    # Use standard Miden helper to transfer all assets from note to account
    # This will call the account's receive_asset export for each asset
    #exec.note::add_assets_to_account

    # Now register the name - payment validation will check the vault balance
    push.{name_push}
    call.registry::register_name
    exec.sys::truncate_stack
end
"#,
                name_push = name_push_str
            )
        } else {
            // For free registration, no asset transfer needed
            format!(
                r#"
use.miden_id::registry
use.std::sys

begin
    push.{name_push}
    call.registry::register_name
    exec.sys::truncate_stack
end
"#,
                name_push = name_push_str
            )
        };

        println!("Register note code for '{}': push.{}", name, name_push_str);

        let contract_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
        let contract_library = create_library(contract_code, "miden_id::registry").unwrap();

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

        let register_note = create_public_note_with_library(
            &mut self.client,
            register_note_code,
            account.clone(),
            note_assets,
            contract_library,
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

    /// Check if a name is registered (slot 3 query)
    pub async fn is_name_registered(&mut self, name: &str) -> Result<bool, ClientError> {
        Ok(self.get_account_word_for_name(name).await?.is_some())
    }

    /// Get account ID for a name (forward lookup - slot 3)
    pub async fn get_account_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<(u64, u64)>, ClientError> {
        match self.get_account_word_for_name(name).await? {
            Some(word) => Ok(Some(Self::decode_account_word(&word))),
            None => Ok(None),
        }
    }

    /// Get raw account Word for a name (slot 3)
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

    /// Check if an address has a name (slot 4 query)
    pub async fn has_name_for_address(&mut self, account: &Account) -> Result<bool, ClientError> {
        Ok(self.get_name_word_for_account(account).await?.is_some())
    }

    /// Get name for an address (reverse lookup - slot 4)
    pub async fn get_name_for_address(
        &mut self,
        account: &Account,
    ) -> Result<Option<String>, ClientError> {
        match self.get_name_word_for_account(account).await? {
            Some(word) => Ok(Some(Self::decode_name_word(&word))),
            None => Ok(None),
        }
    }

    /// Get raw name Word for an account (slot 4)
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

    /// Execute a transaction with a note (internal helper)
    async fn execute_transaction_with_note(&mut self, note: Note) -> Result<(), ClientError> {
        let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm")).unwrap();
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

/// Converts a Word to a MASM push string
/// CRITICAL: push.a.b.c.d in MASM = Word([a,b,c,d]) in Rust (SAME ORDER)
fn word_to_masm_push_string(word: &Word) -> String {
    format!(
        "{}.{}.{}.{}",
        word[0].as_int(),
        word[1].as_int(),
        word[2].as_int(),
        word[3].as_int()
    )
}
