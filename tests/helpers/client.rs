use miden_client::ClientError;
use miden_objects::{account::AccountId, assembly::Library};
use midenid_contracts::common::create_library;
use std::{fs, path::Path, sync::OnceLock};

use super::types::RegistryTestHelper;
use super::{create_basic_wallet, create_faucet, deploy_registry, initialize_registry};

static SHARED_REGISTRY_CONTRACT_ID: OnceLock<AccountId> = OnceLock::new();
static SHARED_FAUCET_ID: OnceLock<AccountId> = OnceLock::new();
static SHARED_OWNER_ID: OnceLock<AccountId> = OnceLock::new();

/// Path constants for test resources
///
/// Contains standard file paths used across the test suite for locating
/// MASM source files, scripts, notes, and data directories.
pub mod paths {
    /// Path to the main registry contract MASM source file
    pub const REGISTRY_CONTRACT: &str = "./masm/accounts/miden_id.masm";

    /// Path to the no-operation (NOP) transaction script
    pub const NOP_SCRIPT: &str = "./masm/scripts/nop.masm";

    /// Directory containing MASM note templates
    pub const NOTES_DIR: &str = "./masm/notes";

    /// Directory containing MASM transaction scripts
    pub const SCRIPTS_DIR: &str = "./masm/scripts";

    /// Directory for keystore data (account authentication keys)
    pub const KEYSTORE_DIR: &str = "./keystore";
}

/// Loads the registry contract library with the standard namespace
///
/// Reads the registry MASM contract file and compiles it into a library
/// using the standard namespace `"miden_id::registry"`.
///
/// # Returns
///
/// Compiled MASM library ready to be linked with transaction scripts
///
/// # Panics
///
/// Panics if the contract file cannot be read or compiled
pub fn load_registry_library() -> Library {
    let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
    create_library(contract_code, "miden_id::registry").unwrap()
}

/// Loads the registry contract library with a custom namespace
///
/// Reads the registry MASM contract file and compiles it into a library
/// using a caller-specified namespace.
///
/// # Arguments
///
/// * `namespace` - Custom namespace for the library (e.g., `"external_contract::miden_id"`)
///
/// # Returns
///
/// Compiled MASM library ready to be linked with transaction scripts
///
/// # Panics
///
/// Panics if the contract file cannot be read or compiled
pub fn load_registry_library_with_namespace(namespace: &str) -> Library {
    let contract_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT)).unwrap();
    create_library(contract_code, namespace).unwrap()
}

/// Sets up a test helper connected to existing contract and faucet
///
/// Creates a new persistent test helper instance (preserves database/keystore)
/// and fetches the specified registry contract and faucet accounts from the
/// client, attaching them to the helper for convenient access in tests.
///
/// **Use Case**: Connect to already-deployed shared test infrastructure
///
/// # Arguments
///
/// * `contract_id` - Registry contract account ID to fetch
/// * `faucet_id` - Faucet account ID to fetch
///
/// # Returns
///
/// * `Ok(RegistryTestHelper)` - Configured test helper with attached accounts
/// * `Err(ClientError)` - Failed to create helper or fetch accounts
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

/// Gets or initializes shared registry contract for tests
///
/// Implements a singleton pattern for test infrastructure to improve performance
/// by reusing deployed contracts across multiple tests.
///
/// **First call**: Deploys a new registry contract, creates faucet and owner accounts,
/// initializes the registry with default settings (price=100), and caches the IDs.
///
/// **Subsequent calls**: Returns cached account IDs instantly without redeployment.
///
/// # Configuration
///
/// Default initialization settings:
/// - Faucet: "REG" token, 8 decimals, 10B max supply
/// - Registration price: 100 tokens
/// - Owner: Basic wallet account
///
/// # Returns
///
/// Tuple of `(registry_contract_id, faucet_id, owner_id)`
///
/// # Notes
///
/// - Uses persistent storage (does not clear database between tests)
/// - Prints initialization messages on first deployment
/// - Thread-safe via `OnceLock` for concurrent test execution
pub async fn get_or_init_shared_contract() -> (AccountId, AccountId, AccountId) {
    if let (Some(&contract_id), Some(&faucet_id), Some(&owner_id)) = (
        SHARED_REGISTRY_CONTRACT_ID.get(),
        SHARED_FAUCET_ID.get(),
        SHARED_OWNER_ID.get(),
    ) {
        return (contract_id, faucet_id, owner_id);
    }

    let mut helper = RegistryTestHelper::new().await.unwrap();
    let registry = deploy_registry(&mut helper.client).await.unwrap();
    let faucet = create_faucet(
        &mut helper.client,
        helper.keystore.clone(),
        "REG",
        8,
        10_000_000_000,
    )
    .await
    .unwrap();
    let owner = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Owner")
        .await
        .unwrap();

    initialize_registry(
        &mut helper.client,
        registry.id(),
        &owner,
        Some(&faucet),
        100,
    )
    .await
    .unwrap();

    helper.registry_contract = Some(registry);

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
