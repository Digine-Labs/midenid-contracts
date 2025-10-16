use miden_client::{
    ClientError, account::Account, keystore::FilesystemKeyStore,
    transaction::TransactionRequestBuilder,
};
use miden_objects::{account::AccountId, asset::FungibleAsset, note::NoteType};
use midenid_contracts::common::{create_basic_account, create_faucet_account};
use rand::rngs::StdRng;
use tokio::time::{Duration, sleep};

use super::types::RegistryTestHelper;

type Client = miden_client::Client<FilesystemKeyStore<StdRng>>;

/// Creates a new basic wallet account.
///
/// Creates a wallet account with BasicAuth authentication component.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `keystore` - Keystore for account authentication
/// * `_role` - Role hint (currently unused, for future role-based accounts)
///
/// # Returns
///
/// * `Ok(Account)` - Successfully created account
/// * `Err(ClientError)` - Account creation failed
pub async fn create_basic_wallet(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
    _role: &str,
) -> Result<Account, ClientError> {
    let (account, _) = create_basic_account(client, keystore).await?;
    sleep(Duration::from_secs(3)).await;
    Ok(account)
}

/// Creates a new fungible asset faucet.
///
/// The faucet can mint fungible tokens up to the specified max supply.
///
/// # Arguments
///
/// * `client` - Miden client instance
/// * `keystore` - Keystore for faucet authentication
/// * `symbol` - Token symbol (e.g., "REG", "POL")
/// * `decimals` - Number of decimal places
/// * `max_supply` - Maximum token supply
///
/// # Returns
///
/// * `Ok(Account)` - Successfully created faucet
/// * `Err(ClientError)` - Faucet creation failed
pub async fn create_faucet(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
    symbol: &str,
    decimals: u8,
    max_supply: u64,
) -> Result<Account, ClientError> {
    let (faucet, _) = create_faucet_account(client, keystore, symbol, decimals, max_supply).await?;
    sleep(Duration::from_secs(3)).await;
    Ok(faucet)
}

/// Mints tokens from faucet and funds an account.
///
/// This is a helper function that:
/// 1. Mints tokens from the faucet to create a note
/// 2. Waits for the note to be available
/// 3. Consumes the note with the target account
/// 4. Returns the updated account with funds
///
/// # Arguments
///
/// * `helper` - Test helper instance
/// * `faucet_id` - Faucet account ID to mint from
/// * `account` - Account to fund
/// * `amount` - Amount of tokens to mint
///
/// # Returns
///
/// Updated account with funds
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
        helper.client.sync_state().await?;
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

    helper.client.sync_state().await?;
    let record = helper.client.get_account(account.id()).await?.unwrap();
    Ok(record.into())
}
