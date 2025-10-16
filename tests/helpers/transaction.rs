use miden_client::{
    ClientError, Word, account::AccountId, note::Note, transaction::TransactionRequestBuilder,
};
use miden_objects::assembly::Library;
use midenid_contracts::common::create_tx_script;
use rand::rngs::StdRng;
use std::{fs, path::Path};

type Client = miden_client::Client<miden_client::keystore::FilesystemKeyStore<StdRng>>;

use super::client::paths;

/// Executes a transaction with a note on the specified account
///
/// Creates a transaction with:
/// - An unauthenticated input note
/// - A NOP script (no-operation, just processes the note)
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `account_id` - Account ID to execute transaction on (usually registry contract)
/// * `note` - Note to process in the transaction
///
/// # Returns
///
/// * `Ok(())` - Transaction submitted successfully
/// * `Err(ClientError)` - Transaction failed
pub async fn execute_note_transaction(
    client: &mut Client,
    account_id: AccountId,
    note: Note,
) -> Result<(), ClientError> {
    let nop_script_code = fs::read_to_string(Path::new(paths::NOP_SCRIPT)).unwrap();
    let transaction_script = create_tx_script(nop_script_code, None).unwrap();

    let request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(note, None)])
        .custom_script(transaction_script)
        .build()
        .unwrap();

    let tx_result = client.new_transaction(account_id, request).await?;
    client.submit_transaction(tx_result).await?;

    Ok(())
}

/// Executes a custom script transaction on the specified account
///
/// Creates and executes a transaction with:
/// - A custom script with optional library
/// - Optional script arguments
///
/// **Note**: This is primarily for testing export functions. The transaction
/// may fail (which is expected for read-only operations), and the caller
/// should handle errors accordingly.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `account_id` - Account ID to execute transaction on (usually registry contract)
/// * `script_code` - MASM script code to execute
/// * `library` - Optional library to link with the script
/// * `script_arg` - Optional Word argument to pass to the script
///
/// # Returns
///
/// * `Ok(())` - Transaction submitted successfully (or failed as expected)
/// * `Err(ClientError)` - Unexpected transaction error
pub async fn execute_script_transaction(
    client: &mut Client,
    account_id: AccountId,
    script_code: String,
    library: Option<Library>,
    script_arg: Option<Word>,
) -> Result<(), ClientError> {
    let transaction_script = create_tx_script(script_code, library).unwrap();

    let mut request_builder = TransactionRequestBuilder::new().custom_script(transaction_script);

    if let Some(arg) = script_arg {
        request_builder = request_builder.script_arg(arg);
    }

    let request = request_builder.build().unwrap();

    // Execute transaction - errors are ignored as they're expected for read-only operations
    let _ = client.new_transaction(account_id, request).await;

    Ok(())
}

/// Loads script code from file
///
/// # Arguments
///
/// * `script_name` - Name of the script (without .masm extension)
///
/// # Returns
///
/// Script code as a string
pub fn get_script_code(script_name: String) -> String {
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::SCRIPTS_DIR,
        script_name
    )))
    .unwrap()
}

/// Loads note code from file
///
/// # Arguments
///
/// * `note_name` - Name of the note (without .masm extension)
///
/// # Returns
///
/// Note code as a string
pub fn get_note_code(note_name: String) -> String {
    fs::read_to_string(Path::new(&format!(
        "{}/{}.masm",
        paths::NOTES_DIR,
        note_name
    )))
    .unwrap()
}
