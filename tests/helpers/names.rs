use miden_client::{
    ClientError,
    account::{Account, AccountId},
    keystore::FilesystemKeyStore,
    note::NoteAssets,
    store::AccountRecord,
};
use miden_objects::{Felt, FieldElement, Word, asset::FungibleAsset};
use midenid_contracts::common::{create_library, create_public_note_with_library_and_inputs};
use rand::rngs::StdRng;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};

use super::client::{load_registry_library, paths};
use super::encoding::EncodingUtils;
use super::registry::fetch_registry_account;
use super::transaction::{
    execute_note_transaction, execute_script_transaction, get_note_code, get_script_code,
};

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

/// Registers a name for an account with optional payment.
///
/// Creates a registration note with the specified payment amount and executes
/// the transaction to register the name in the registry.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `account` - Account to register name for
/// * `name` - Name to register
/// * `faucet_account` - Optional faucet for payment token (required if payment_amount is Some)
/// * `payment_amount` - Optional payment amount (use None for free registration)
///
/// # Returns
///
/// * `Ok(())` - Successfully registered name
/// * `Err(ClientError)` - Registration failed
pub async fn register_name(
    client: &mut Client,
    registry_id: AccountId,
    account: &Account,
    name: &str,
    faucet_account: Option<&Account>,
    payment_amount: Option<u64>,
) -> Result<(), ClientError> {
    let name_word = EncodingUtils::encode_name_to_word(name);

    // Load register_name note from file
    let register_note_code = get_note_code("register_name".to_string());

    println!("Registering name '{}' for account {}", name, account.id());

    let contract_library = load_registry_library();

    // Create note assets based on payment requirement
    let note_assets = if let Some(amount) = payment_amount {
        // Get the payment token (faucet) account
        let faucet = faucet_account.expect("Faucet account must be provided for paid registration");

        // Create fungible asset with the payment amount
        let payment_asset =
            FungibleAsset::new(faucet.id(), amount).expect("Failed to create payment asset");

        NoteAssets::new(vec![payment_asset.into()]).unwrap()
    } else {
        // No payment required
        NoteAssets::new(vec![]).unwrap()
    };

    // Pass the name as note input (Word format)
    let inputs = EncodingUtils::word_to_note_inputs(&name_word);

    let register_note = create_public_note_with_library_and_inputs(
        client,
        register_note_code,
        account.clone(),
        note_assets,
        contract_library,
        inputs,
    )
    .await
    .unwrap();

    sleep(Duration::from_secs(5)).await;
    execute_note_transaction(client, registry_id, register_note).await?;

    sleep(Duration::from_secs(15)).await;
    client.sync_state().await?;

    Ok(())
}

/// Queries account ID for a registered name from contract storage
///
/// Performs a direct storage map lookup: name → account ID
///
/// # Arguments
///
/// * `account_record` - Contract account record containing storage
/// * `name` - Name to look up
///
/// # Returns
///
/// * `Ok(Some((prefix, suffix)))` - Found account ID
/// * `Ok(None)` - Name not registered
/// * `Err(ClientError)` - Storage query failed
pub fn query_account_for_name(
    account_record: &AccountRecord,
    name: &str,
) -> Result<Option<(u64, u64)>, ClientError> {
    let storage = account_record.account().storage();
    let key = EncodingUtils::encode_name_to_word(name);
    let value = storage.get_map_item(3, key)?;

    if EncodingUtils::is_zero_word(&value) {
        Ok(None)
    } else {
        Ok(Some(EncodingUtils::decode_account_word(&value)))
    }
}

/// Queries registered name for an account from contract storage
///
/// Performs a direct storage map lookup: account ID → name
///
/// # Arguments
///
/// * `account_record` - Contract account record containing storage
/// * `account` - Account to look up name for
///
/// # Returns
///
/// * `Ok(Some(name))` - Found registered name
/// * `Ok(None)` - Account has no registered name
/// * `Err(ClientError)` - Storage query failed
pub fn query_name_for_address(
    account_record: &AccountRecord,
    account: &Account,
) -> Result<Option<String>, ClientError> {
    let storage = account_record.account().storage();
    let key = EncodingUtils::encode_account_to_word(account);
    let value = storage.get_map_item(4, key)?;

    if EncodingUtils::is_zero_word(&value) {
        Ok(None)
    } else {
        Ok(Some(EncodingUtils::decode_name_word(&value)))
    }
}

/// Gets account ID for a registered name (with client fetch)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `name` - Name to look up
///
/// # Returns
///
/// * `Ok(Some((prefix, suffix)))` - Found account ID
/// * `Ok(None)` - Name not registered
/// * `Err(ClientError)` - Query failed
pub async fn get_account_id_for_name(
    client: &mut Client,
    registry_id: AccountId,
    name: &str,
) -> Result<Option<(u64, u64)>, ClientError> {
    let account_record = match fetch_registry_account(client, registry_id).await? {
        Some(record) => record,
        None => return Ok(None),
    };
    query_account_for_name(&account_record, name)
}

/// Gets registered name for an account (with client fetch)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `account` - Account to look up name for
///
/// # Returns
///
/// * `Ok(Some(name))` - Found registered name
/// * `Ok(None)` - Account has no registered name
/// * `Err(ClientError)` - Query failed
pub async fn get_name_for_account(
    client: &mut Client,
    registry_id: AccountId,
    account: &Account,
) -> Result<Option<String>, ClientError> {
    let account_record = match fetch_registry_account(client, registry_id).await? {
        Some(record) => record,
        None => return Ok(None),
    };
    query_name_for_address(&account_record, account)
}

/// Checks if a name is registered (convenience wrapper)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `name` - Name to check
///
/// # Returns
///
/// * `Ok(true)` - Name is registered
/// * `Ok(false)` - Name is not registered
/// * `Err(ClientError)` - Query failed
#[inline]
pub async fn is_name_registered(
    client: &mut Client,
    registry_id: AccountId,
    name: &str,
) -> Result<bool, ClientError> {
    Ok(get_account_id_for_name(client, registry_id, name)
        .await?
        .is_some())
}

/// Checks if an account has a registered name (convenience wrapper)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `account` - Account to check
///
/// # Returns
///
/// * `Ok(true)` - Account has a registered name
/// * `Ok(false)` - Account has no registered name
/// * `Err(ClientError)` - Query failed
#[inline]
pub async fn has_name_for_address(
    client: &mut Client,
    registry_id: AccountId,
    account: &Account,
) -> Result<bool, ClientError> {
    Ok(get_name_for_account(client, registry_id, account)
        .await?
        .is_some())
}

// ================================================================================================
// EXPORT SCRIPT TESTING FUNCTIONS
// ================================================================================================

/// Helper function to create the registry contract library for script testing
fn create_registry_library_for_scripts() -> miden_objects::assembly::Library {
    let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
    create_library(contract_code, "external_contract::miden_id").unwrap()
}

/// Calls the get_id export to retrieve account ID for a name.
///
/// **Testing Function**: This executes a transaction script that calls the registry's
/// get_id export for testing purposes. The script execution output is logged but errors
/// are ignored, as the actual result is retrieved via direct storage query.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `name` - Name to look up
///
/// # Returns
///
/// * `Ok(Some((prefix, suffix)))` - Found account ID
/// * `Ok(None)` - Name not registered
/// * `Err(ClientError)` - Lookup failed
pub async fn get_id_from_script(
    client: &mut Client,
    registry_id: AccountId,
    name: &str,
) -> Result<Option<(u64, u64)>, ClientError> {
    let name_word = EncodingUtils::encode_name_to_word(name);
    let script_code = get_script_code("call_get_id".to_string());
    let contract_library = create_registry_library_for_scripts();

    // Query storage to get the actual result
    let storage_result = get_account_id_for_name(client, registry_id, name).await?;

    // Execute script transaction for testing/debug output
    execute_script_transaction(
        client,
        registry_id,
        script_code,
        Some(contract_library),
        Some(name_word),
    )
    .await?;

    Ok(storage_result)
}

/// Calls the get_name export to retrieve name for an account.
///
/// **Testing Function**: This executes a transaction script that calls the registry's
/// get_name export for testing purposes. The script execution output is logged but errors
/// are ignored, as the actual result is retrieved via direct storage query.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `registry_id` - Registry contract account ID
/// * `account` - Account to look up name for
///
/// # Returns
///
/// * `Ok(Some(name))` - Found registered name
/// * `Ok(None)` - Account has no registered name
/// * `Err(ClientError)` - Lookup failed
pub async fn get_name_from_script(
    client: &mut Client,
    registry_id: AccountId,
    account: &Account,
) -> Result<Option<String>, ClientError> {
    let script_code = get_script_code("call_get_name".to_string());
    let contract_library = create_registry_library_for_scripts();

    // Build Word with account ID: [ZERO, ZERO, suffix, prefix]
    let account_id_word: Word = [
        Felt::ZERO,
        Felt::ZERO,
        account.id().suffix(),
        account.id().prefix().as_felt(),
    ]
    .into();

    // Query storage to get the actual result
    let storage_result = get_name_for_account(client, registry_id, account).await?;

    // Execute script transaction for testing/debug output
    execute_script_transaction(
        client,
        registry_id,
        script_code,
        Some(contract_library),
        Some(account_id_word),
    )
    .await?;

    Ok(storage_result)
}
