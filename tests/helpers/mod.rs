// Module declarations
pub mod client;
pub mod encoding;
pub mod faucet;
pub mod names;
pub mod registry;
pub mod transaction;
pub mod types;

// Re-exports
pub use client::{
    get_or_init_shared_contract, load_registry_library, load_registry_library_with_namespace,
    paths, setup_helper_with_contract,
};
pub use encoding::EncodingUtils;
pub use faucet::{create_basic_wallet, create_faucet, mint_and_fund_account};
pub use names::{
    // Name query async functions (with client fetch)
    get_account_id_for_name,
    get_account_id_for_name as get_account_for_name,
    // Export function wrappers
    get_id_from_script,
    get_id_from_script as call_get_id_export,
    get_name_for_account,
    get_name_from_script,
    get_name_from_script as call_get_name_export,
    has_name_for_address,
    is_name_registered,
    // Name query sync functions (require AccountRecord)
    query_account_for_name,
    query_name_for_address,
    // Name registration
    register_name,
};
pub use registry::{
    // Registry operations
    deploy_registry,
    // Registry state async functions (with client fetch)
    fetch_registry_account,
    // Registry state sync functions (require AccountRecord)
    get_initialization_state,
    get_owner,
    get_payment_token,
    get_payment_token_id_from_registry,
    get_payment_token_state,
    get_price,
    get_registration_price_from_registry,
    get_registry_mapping_state,
    initialize_registry,
    parse_registry_state,
    transfer_registry_ownership,
    update_registry_price,
};
pub use transaction::{
    execute_note_transaction, execute_script_transaction, get_note_code, get_script_code,
};
pub use types::{ContractState, RegistryTestHelper};
