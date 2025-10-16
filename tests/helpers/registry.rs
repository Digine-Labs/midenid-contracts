use miden_client::{
    ClientError, Felt, Word,
    account::{Account, AccountId},
    keystore::FilesystemKeyStore,
    note::NoteAssets,
    note::NoteInputs,
    store::AccountRecord,
};
use midenid_contracts::common::{
    create_public_immutable_contract, create_public_note_with_library_and_inputs,
};
use rand::rngs::StdRng;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

use super::client::{load_registry_library, paths};
use super::transaction::{execute_note_transaction, get_note_code};
use super::types::ContractState;

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

/// Deploys the Miden ID registry contract.
///
/// Creates a public immutable contract from the MASM code in miden_id.masm.
/// The contract must be initialized before use via initialization methods.
///
/// # Arguments
///
/// * `client` - Miden client instance
///
/// # Returns
///
/// * `Ok(Account)` - Successfully deployed contract
/// * `Err(ClientError)` - Deployment failed
pub async fn deploy_registry(client: &mut Client) -> Result<Account, ClientError> {
    let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
    let (registry_contract, registry_seed) =
        create_public_immutable_contract(client, &registry_code).await?;

    client
        .add_account(&registry_contract, Some(registry_seed), false)
        .await?;

    sleep(Duration::from_secs(5)).await;

    Ok(registry_contract)
}

/// Initializes the registry with owner and payment token configuration.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `owner_account` - Account that will own the registry
/// * `faucet_account` - Optional faucet for payment token (if None, uses owner)
/// * `price` - Registration price (typically 100 with faucet, 0 without)
///
/// # Returns
///
/// * `Ok(())` - Successfully initialized
/// * `Err(ClientError)` - Initialization failed
pub async fn initialize_registry(
    client: &mut Client,
    registry_id: AccountId,
    owner_account: &Account,
    faucet_account: Option<&Account>,
    price: u64,
) -> Result<(), ClientError> {
    let payment_token = faucet_account.unwrap_or(owner_account);
    let token_prefix = payment_token.id().prefix().as_felt();
    let token_suffix = payment_token.id().suffix();

    // Create dynamic init note with payment token
    let init_note_code = get_note_code("init".to_string());
    let contract_library = load_registry_library();
    let empty_assets = NoteAssets::new(vec![]).unwrap();
    let inputs = NoteInputs::new(vec![token_prefix, token_suffix, Felt::new(price)]).unwrap();

    let init_note = create_public_note_with_library_and_inputs(
        client,
        init_note_code,
        owner_account.clone(),
        empty_assets,
        contract_library,
        inputs,
    )
    .await
    .unwrap();

    sleep(Duration::from_secs(5)).await;
    execute_note_transaction(client, registry_id, init_note).await?;
    sleep(Duration::from_secs(8)).await;
    client.sync_state().await?;

    Ok(())
}

/// Returns initialization state from storage
///
/// Extracts the initialization flag and owner information from the registry storage.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Tuple of `(initialized, owner_prefix, owner_suffix)`:
/// * `initialized` - 1 if initialized, 0 if not
/// * `owner_prefix` - Owner account ID prefix
/// * `owner_suffix` - Owner account ID suffix
pub fn get_initialization_state(account_record: &AccountRecord) -> (u64, u64, u64) {
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

/// Updates the name registration price.
///
/// Only the registry owner can update the price.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `owner_account` - Registry owner account
/// * `new_price` - New registration price
///
/// # Returns
///
/// * `Ok(())` - Successfully updated price
/// * `Err(ClientError)` - Update failed
pub async fn update_registry_price(
    client: &mut Client,
    registry_id: AccountId,
    owner_account: &Account,
    new_price: u64,
) -> Result<(), ClientError> {
    let update_price_note_code = get_note_code("update_price".to_string());
    let contract_library = load_registry_library();
    let empty_assets = NoteAssets::new(vec![]).unwrap();
    let inputs = NoteInputs::new(vec![
        Felt::new(new_price),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ])
    .unwrap();

    let update_price_note = create_public_note_with_library_and_inputs(
        client,
        update_price_note_code,
        owner_account.clone(),
        empty_assets,
        contract_library,
        inputs,
    )
    .await
    .unwrap();

    execute_note_transaction(client, registry_id, update_price_note).await?;
    sleep(Duration::from_secs(8)).await;
    client.sync_state().await?;

    Ok(())
}

/// Transfers registry ownership to a new account.
///
/// Only the current owner can transfer ownership.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `owner_account` - Current registry owner
/// * `new_owner` - New owner account
///
/// # Returns
///
/// * `Ok(())` - Successfully transferred ownership
/// * `Err(ClientError)` - Transfer failed
pub async fn transfer_registry_ownership(
    client: &mut Client,
    registry_id: AccountId,
    owner_account: &Account,
    new_owner: &Account,
) -> Result<(), ClientError> {
    let update_owner_note_code = get_note_code("update_owner".to_string());
    let contract_library = load_registry_library();
    let empty_assets = NoteAssets::new(vec![]).unwrap();
    let inputs = NoteInputs::new(vec![
        new_owner.id().suffix(),
        new_owner.id().prefix().as_felt(),
    ])
    .unwrap();

    let update_owner_note = create_public_note_with_library_and_inputs(
        client,
        update_owner_note_code,
        owner_account.clone(),
        empty_assets,
        contract_library,
        inputs,
    )
    .await
    .unwrap();

    execute_note_transaction(client, registry_id, update_owner_note).await?;
    sleep(Duration::from_secs(8)).await;
    client.sync_state().await?;

    Ok(())
}

/// Returns owner ID from storage
///
/// Extracts the owner account ID from registry storage.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Tuple of `(owner_prefix, owner_suffix)` representing the owner account ID
pub fn get_owner(account_record: &AccountRecord) -> (u64, u64) {
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

/// Returns payment token ID from storage
///
/// Extracts the payment token account ID from registry storage.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Tuple of `(token_prefix, token_suffix)` representing the payment token account ID
pub fn get_payment_token(account_record: &AccountRecord) -> (u64, u64) {
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

/// Legacy alias for `get_payment_token`
#[inline]
pub fn get_payment_token_state(account_record: &AccountRecord) -> (u64, u64) {
    get_payment_token(account_record)
}

/// Returns registration price from storage
///
/// Extracts the current name registration price from registry storage.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Current registration price as u64
pub fn get_price(account_record: &AccountRecord) -> u64 {
    let price_word: Word = account_record
        .account()
        .storage()
        .get_item(5)
        .unwrap()
        .into();
    price_word.get(0).unwrap().as_int()
}

/// Returns registry mapping roots from storage
///
/// Extracts the root nodes of the two mapping trees (name→ID and ID→name)
/// from registry storage.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Tuple of `(name_to_id, id_to_name)`:
/// * `name_to_id` - Root of name→account_id mapping tree (Option<Word>)
/// * `id_to_name` - Root of account_id→name mapping tree (Option<Word>)
pub fn get_registry_mapping_state(account_record: &AccountRecord) -> (Option<Word>, Option<Word>) {
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

/// Parses registry contract state from an account record
///
/// Extracts and organizes all registry state data into a ContractState struct.
///
/// # Arguments
///
/// * `account_record` - Registry account record containing storage
///
/// # Returns
///
/// Parsed ContractState with all registry data
pub fn parse_registry_state(account_record: &AccountRecord) -> ContractState {
    let (initialized, owner_prefix, owner_suffix) = get_initialization_state(account_record);
    let (token_prefix, token_suffix) = get_payment_token(account_record);
    let (name_to_id, id_to_name) = get_registry_mapping_state(account_record);

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

/// Fetches the registry account record from the client
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
///
/// # Returns
///
/// * `Ok(Some(AccountRecord))` - Registry account found
/// * `Ok(None)` - Registry account not found
/// * `Err(ClientError)` - Query failed
pub async fn fetch_registry_account(
    client: &mut Client,
    registry_id: AccountId,
) -> Result<Option<AccountRecord>, ClientError> {
    client.get_account(registry_id).await
}

/// Gets payment token ID from registry (with client fetch)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
///
/// # Returns
///
/// * `Ok((prefix, suffix))` - Payment token account ID
/// * `Err(ClientError)` - Query failed
pub async fn get_payment_token_id_from_registry(
    client: &mut Client,
    registry_id: AccountId,
) -> Result<(u64, u64), ClientError> {
    let account_record = fetch_registry_account(client, registry_id)
        .await?
        .expect("Registry contract not found");
    Ok(get_payment_token(&account_record))
}

/// Gets registration price from registry (with client fetch)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
///
/// # Returns
///
/// * `Ok(price)` - Registration price
/// * `Err(ClientError)` - Query failed
pub async fn get_registration_price_from_registry(
    client: &mut Client,
    registry_id: AccountId,
) -> Result<u64, ClientError> {
    let account_record = fetch_registry_account(client, registry_id)
        .await?
        .expect("Registry contract not found");
    Ok(get_price(&account_record))
}
