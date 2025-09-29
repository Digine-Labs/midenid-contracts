use miden_client::{
    ClientError, Word,
    account::{Account, AccountIdAddress, AddressInterface},
    keystore::FilesystemKeyStore,
    note::NoteAssets,
    rpc::Endpoint,
    store::AccountRecord,
    transaction::TransactionRequestBuilder,
};
use miden_objects::{Felt, FieldElement};
use midenid_contracts::common::*;
use rand::rngs::StdRng;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

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
}

impl RegistryTestHelper {
    /// Create a new test helper with network connection (testnet with localhost fallback)
    pub async fn new() -> Result<Self, ClientError> {
        delete_keystore_and_store().await;

        let endpoint = Endpoint::testnet();

        let client = match instantiate_client(endpoint.clone()).await {
            Ok(client) => client,
            Err(e) => {
                println!("⚠️  Testnet connection failed: {}", e);
                println!("🔗 Falling back to localhost...");
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
        })
    }

    /// Sync with network and return latest block number
    pub async fn sync_network(&mut self) -> Result<(), ClientError> {
        let _sync_summary = self.client.sync_state().await?;
        Ok(())
    }

    /// Create a new user account
    pub async fn create_account(&mut self, _role: &str) -> Result<Account, ClientError> {
        let (account, _) = create_basic_account(&mut self.client, self.keystore.clone()).await?;
        sleep(Duration::from_secs(3)).await;

        let _account_address = AccountIdAddress::new(account.id(), AddressInterface::BasicWallet);

        Ok(account)
    }

    /// Deploy the registry contract
    pub async fn deploy_registry_contract(&mut self) -> Result<Account, ClientError> {
        let registry_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
        let (registry_contract, registry_seed) =
            create_public_immutable_contract(&mut self.client, &registry_code).await?;
        self.client
            .add_account(&registry_contract, Some(registry_seed), false)
            .await?;

        let _contract_address =
            AccountIdAddress::new(registry_contract.id(), AddressInterface::Unspecified);

        self.registry_contract = Some(registry_contract.clone());
        sleep(Duration::from_secs(5)).await;

        Ok(registry_contract)
    }

    /// Initialize the registry contract with given owner
    pub async fn initialize_registry(
        &mut self,
        owner_account: &Account,
    ) -> Result<(), ClientError> {
        let init_note_code =
            fs::read_to_string(Path::new("./masm/notes/init_miden_id.masm")).unwrap();
        let contract_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();

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

        let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm")).unwrap();
        let transaction_script = create_tx_script(nop_script_code, None).unwrap();

        let init_request = TransactionRequestBuilder::new()
            .unauthenticated_input_notes([(init_note, None)])
            .custom_script(transaction_script)
            .build()
            .unwrap();

        let registry_id = self.registry_contract.as_ref().unwrap().id();
        let tx_result = self
            .client
            .new_transaction(registry_id, init_request)
            .await?;
        self.client.submit_transaction(tx_result).await?;

        self.owner_account = Some(owner_account.clone());

        // Wait for transaction confirmation
        sleep(Duration::from_secs(8)).await;
        self.sync_network().await?;

        Ok(())
    }

    /// Get contract state
    pub async fn get_contract_state(&mut self) -> Result<Option<AccountRecord>, ClientError> {
        let registry_id = self.registry_contract.as_ref().unwrap().id();
        self.client.get_account(registry_id).await
    }

    /// Get contract initialization state data
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
        let (owner_prefix, owner_suffix) = (
            owner.get(1).unwrap().as_int(),
            owner.get(0).unwrap().as_int(),
        );

        (initialized, owner_prefix, owner_suffix)
    }

    /// Get payment token configuration data
    pub fn get_payment_token_state(&self, account_record: &AccountRecord) -> (u64, u64) {
        let payment_token: Word = account_record
            .account()
            .storage()
            .get_item(2)
            .unwrap()
            .into();
        let (token_prefix, token_suffix) = (
            payment_token.get(0).unwrap().as_int(), // Fixed: prefix is at index 0
            payment_token.get(1).unwrap().as_int(), // Fixed: suffix is at index 1
        );

        (token_prefix, token_suffix)
    }

    /// Get registry mapping state data (for name-to-address and address-to-name mappings)
    pub fn get_registry_mapping_state(
        &self,
        account_record: &AccountRecord,
    ) -> (Option<Word>, Option<Word>) {
        // Storage slot 3: Name -> ID mapping
        let name_to_id_result = account_record.account().storage().get_item(3);

        // Storage slot 4: ID -> Name mapping
        let id_to_name_result = account_record.account().storage().get_item(4);

        let name_to_id = name_to_id_result.map(|item| item.into()).ok();
        let id_to_name = id_to_name_result.map(|item| item.into()).ok();

        (name_to_id, id_to_name)
    }

    /// Get complete contract state for validation
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

    /// Setup environment with deployed contract only (no accounts, no initialization)
    pub async fn setup_with_deployed_contract() -> Result<Self, ClientError> {
        let mut helper = Self::new().await?;
        helper.sync_network().await?;
        helper.deploy_registry_contract().await?;

        Ok(helper)
    }

    /// Encode a name string to a Word (4 felts) - direct string encoding, max 28 chars
    pub fn encode_name_to_word(name: &str) -> Word {
        assert!(name.len() <= 28, "Name must not exceed 28 characters");

        let bytes = name.as_bytes();
        let mut felts = [Felt::ZERO; 4];

        // Pack bytes directly into felts (7 bytes per felt to stay within field limits)
        for (i, chunk) in bytes.chunks(7).enumerate() {
            if i >= 4 {
                break;
            }

            let mut value = 0u64;
            for (j, &byte) in chunk.iter().enumerate() {
                value |= (byte as u64) << (j * 8);
            }
            felts[i] = Felt::new(value);
        }

        Word::new(felts)
    }

    fn encode_account_to_word(account: &Account) -> Word {
        Self::encode_account_from_parts(
            account.id().prefix().as_felt().as_int(),
            account.id().suffix().as_int(),
        )
    }

    fn encode_account_from_parts(prefix: u64, suffix: u64) -> Word {
        Word::new([Felt::new(suffix), Felt::new(prefix), Felt::ZERO, Felt::ZERO])
    }

    fn decode_account_word(word: &Word) -> (u64, u64) {
        let suffix = word.get(0).map(|felt| felt.as_int()).unwrap_or(0);
        let prefix = word.get(1).map(|felt| felt.as_int()).unwrap_or(0);
        (prefix, suffix)
    }

    pub fn decode_name_word(word: &Word) -> String {
        let mut bytes = Vec::new();

        for idx in 0..4 {
            if let Some(felt) = word.get(idx) {
                let mut value = felt.as_int();
                for _ in 0..7 {
                    // Changed from 8 to 7 to match encoding
                    let byte = (value & 0xFF) as u8;
                    if byte == 0 {
                        break;
                    }
                    bytes.push(byte);
                    value >>= 8;
                }
            }
        }

        String::from_utf8(bytes).unwrap_or_default()
    }

    pub fn is_zero_word(word: &Word) -> bool {
        (0..4)
            .map(|idx| word.get(idx).map(|felt| felt.as_int()).unwrap_or(0))
            .all(|value| value == 0)
    }

    /// Register a name for an account
    pub async fn register_name_for_account(
        &mut self,
        account: &Account,
        name: &str,
    ) -> Result<(), ClientError> {
        // Create a custom note script for this specific name
        let name_word = Self::encode_name_to_word(name);

        let register_note_code = format!(
            r#"
use.miden_id::registry
use.std::sys

begin
    push.{}.{}.{}.{}
    call.registry::register_name
    exec.sys::truncate_stack
end
"#,
            name_word[3].as_int(),
            name_word[2].as_int(),
            name_word[1].as_int(),
            name_word[0].as_int()
        );

        let contract_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
        let library_namespace = "miden_id::registry";
        let contract_library = create_library(contract_code, library_namespace).unwrap();
        let empty_assets = NoteAssets::new(vec![]).unwrap();

        let register_note = create_public_note_with_library(
            &mut self.client,
            register_note_code,
            account.clone(),
            empty_assets,
            contract_library,
        )
        .await
        .unwrap();

        sleep(Duration::from_secs(5)).await;

        let nop_script_code = fs::read_to_string(Path::new("./masm/scripts/nop.masm")).unwrap();
        let transaction_script = create_tx_script(nop_script_code, None).unwrap();

        let register_request = TransactionRequestBuilder::new()
            .unauthenticated_input_notes([(register_note, None)])
            .custom_script(transaction_script)
            .build()
            .unwrap();

        let registry_id = self.registry_contract.as_ref().unwrap().id();
        let tx_result = self
            .client
            .new_transaction(registry_id, register_request)
            .await?;
        self.client.submit_transaction(tx_result).await?;

        // Wait for transaction confirmation
        sleep(Duration::from_secs(8)).await;
        self.sync_network().await?;

        Ok(())
    }

    /// Check if a name is registered using the contract function
    pub async fn is_name_registered(&mut self, name: &str) -> Result<bool, ClientError> {
        match self.get_account_word_for_name(name).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    /// Get name for an address using the contract function
    pub async fn get_name_for_address(
        &mut self,
        account: &Account,
    ) -> Result<Option<String>, ClientError> {
        match self.get_name_word_for_account(account).await? {
            Some(word) => Ok(Some(Self::decode_name_word(&word))),
            None => Ok(None),
        }
    }

    /// Check if an address has a name using the contract function
    pub async fn has_name_for_address(&mut self, account: &Account) -> Result<bool, ClientError> {
        Ok(self.get_name_word_for_account(account).await?.is_some())
    }

    pub async fn get_account_word_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<Word>, ClientError> {
        if let Some(contract_state) = self.get_contract_state().await? {
            let storage = contract_state.account().storage();
            let key = Self::encode_name_to_word(name);
            let value = storage.get_map_item(3, key)?; // Use slot 3 where names are stored
            if Self::is_zero_word(&value) {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_name_word_for_account(
        &mut self,
        account: &Account,
    ) -> Result<Option<Word>, ClientError> {
        if let Some(contract_state) = self.get_contract_state().await? {
            let storage = contract_state.account().storage();
            let key = Self::encode_account_to_word(account);
            let value = storage.get_map_item(4, key)?; // Use slot 4 for account->name mapping
            if Self::is_zero_word(&value) {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_account_for_name(
        &mut self,
        name: &str,
    ) -> Result<Option<(u64, u64)>, ClientError> {
        match self.get_account_word_for_name(name).await? {
            Some(word) => Ok(Some(Self::decode_account_word(&word))),
            None => Ok(None),
        }
    }
}
