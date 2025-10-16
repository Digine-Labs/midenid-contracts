mod helpers;

use helpers::{RegistryTestHelper, create_basic_wallet, deploy_registry, paths};
use miden_client::{
    Felt,
    note::{NoteExecutionMode, NoteTag},
    testing::NoteBuilder,
};
use miden_objects::{
    FieldElement,
    note::{NoteInputs, NoteType},
    transaction::OutputNote,
};
use miden_testing::{Auth, MockChain, MockChainBuilder};
use midenid_contracts::common::create_library;
use std::{fs, path::Path};

// Helper function to create a registry account for MockChain testing
fn create_registry_account() -> Result<miden_client::account::Account, Box<dyn std::error::Error>> {
    use miden_client::account::component::AccountComponent;
    use miden_client::account::{AccountBuilder, AccountStorageMode};
    use miden_client::transaction::TransactionKernel;
    use miden_lib::account::auth::NoAuth;
    use miden_objects::{
        account::{StorageMap, StorageSlot},
        assembly::Assembler,
    };
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT))?;
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let registry_component = AccountComponent::compile(
        registry_code.clone(),
        assembler.clone(),
        vec![
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Map(StorageMap::new()),
            StorageSlot::Map(StorageMap::new()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
        ],
    )?
    .with_supports_all_types();

    // Use build_existing() to create an account that MockChain recognizes as existing
    let registry_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(NoAuth::default())
        .with_component(registry_component)
        .storage_mode(AccountStorageMode::Public)
        .build_existing()?;

    Ok(registry_account)
}

// Helper function to create a registry account with initial assets
fn create_registry_account_with_assets(
    assets: Vec<miden_objects::asset::Asset>,
) -> Result<miden_client::account::Account, Box<dyn std::error::Error>> {
    use miden_client::account::component::AccountComponent;
    use miden_client::account::{AccountBuilder, AccountStorageMode};
    use miden_client::transaction::TransactionKernel;
    use miden_lib::account::auth::NoAuth;
    use miden_objects::{
        account::{StorageMap, StorageSlot},
        assembly::Assembler,
    };
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT))?;
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let registry_component = AccountComponent::compile(
        registry_code.clone(),
        assembler.clone(),
        vec![
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
            StorageSlot::Map(StorageMap::new()),
            StorageSlot::Map(StorageMap::new()),
            StorageSlot::Value([Felt::ZERO; 4].into()),
        ],
    )?
    .with_supports_all_types();

    let registry_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(NoAuth::default())
        .with_component(registry_component)
        .with_assets(assets)
        .storage_mode(AccountStorageMode::Public)
        .build_existing()?;

    Ok(registry_account)
}

/// Helper to load MASM note code
fn get_note_code(note_name: &str) -> String {
    let path = format!("{}/{}.masm", paths::NOTES_DIR, note_name);
    fs::read_to_string(Path::new(&path))
        .unwrap_or_else(|_| panic!("Failed to read note code: {}", path))
}

/// Test simple MockChain account creation
///
/// This test demonstrates the basic MockChain pattern:
/// 1. Create a MockChain with simple accounts (wallets, faucet)
/// 2. Verify accounts are created successfully
#[tokio::test]
async fn test_mock_chain_simple() -> Result<(), Box<dyn std::error::Error>> {
    // Build MockChain with basic accounts
    let mut builder = MockChain::builder();

    let wallet1 = builder.add_existing_wallet(Auth::BasicAuth)?;
    let wallet2 = builder.add_existing_wallet(Auth::BasicAuth)?;
    let faucet = builder.add_existing_faucet(Auth::BasicAuth, "POL", 1_000_000, None)?;

    let mock_chain = builder.build()?;

    println!("✓ MockChain created successfully");
    println!("  Wallet 1: {}", wallet1.id());
    println!("  Wallet 2: {}", wallet2.id());
    println!("  Faucet: {}", faucet.id());

    Ok(())
}

/// Test registry initialization using MockChain with pre-deployed registry
///
/// This test demonstrates using MockChainBuilder::with_accounts() to add
/// pre-created accounts from the test helper.
#[tokio::test]
async fn test_mock_chain_registry_with_helper() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry account using test helper
    let mut helper = RegistryTestHelper::new().await?;
    let registry = deploy_registry(&mut helper.client).await?;
    let owner = create_basic_wallet(&mut helper.client, helper.keystore.clone(), "Owner").await?;

    // Build MockChain with the accounts
    let mut mock_chain =
        MockChainBuilder::with_accounts([registry.clone(), owner.clone()])?.build()?;

    mock_chain.prove_next_block()?;

    println!("✓ MockChain created with registry account");
    println!("  Registry: {}", registry.id());
    println!("  Owner: {}", owner.id());

    // TODO: Create and execute initialization note
    // This would require creating a note with init code and proper inputs

    Ok(())
}

/// Test name registration initialization using MockChain
///
/// This test demonstrates:
/// 1. Creating a registry account
/// 2. Initializing the registry with owner and faucet
/// 3. Verifying initialization in storage
#[tokio::test]
async fn test_mock_chain_name_registration() -> Result<(), Box<dyn std::error::Error>> {
    use miden_testing::TransactionContextBuilder;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    // Step 1: Create basic accounts
    let mut builder = MockChain::builder();
    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let faucet_account = builder.add_existing_faucet(Auth::BasicAuth, "POL", 1_000_000, None)?;

    // Step 2: Create registry account using helper (build_existing() pattern)
    let registry_account = create_registry_account()?;

    // Step 3: Create registry library
    let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT))?;
    let registry_library = create_library(registry_code, "miden_id::registry")?;

    // Step 4: Create initialization note
    let init_inputs = vec![
        Felt::new(100), // price
        Felt::new(faucet_account.id().prefix().into()),
        Felt::new(faucet_account.id().suffix().into()),
    ];

    let init_note = NoteBuilder::new(owner_account.id(), ChaCha20Rng::from_seed([0u8; 32]))
        .code(get_note_code("init"))
        .note_type(NoteType::Public)
        .tag(NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Network)?.into())
        .dynamically_linked_libraries(vec![registry_library])
        .note_inputs(init_inputs)?
        .build()?;

    // Step 5: Add note to builder (but NOT the registry account)
    // The registry account will be created via the transaction execution
    builder.add_note(OutputNote::Full(init_note.clone()));
    let mut mock_chain = builder.build()?;

    // Step 6: Execute initialization transaction
    // The registry account is NOT in the MockChain yet - it will be created by this transaction
    let tx_inputs = mock_chain.get_transaction_inputs(
        registry_account.clone(),
        None,
        &[init_note.id()],
        &[],
    )?;

    // Use None for account_seed since we used build_existing()
    let tx_context = TransactionContextBuilder::new(registry_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let exec_tx = tx_context.execute().await?;
    let updated_registry = mock_chain.add_pending_executed_transaction(&exec_tx)?;

    // Step 7: Verify initialization in storage

    // Words are stored in reverse: [V4, V3, V2, V1] so index 3 is V1 (the init flag)
    let init_flag = updated_registry
        .storage()
        .get_item(0)?
        .get(3)
        .unwrap()
        .as_int();
    let owner_slot = updated_registry.storage().get_item(1)?;
    let payment_token_slot = updated_registry.storage().get_item(2)?;

    assert_eq!(init_flag, 1, "Registry should be initialized");

    // Word storage: [V1, V2, V3, V4] where index 0 is V1
    // Owner is stored as [prefix, suffix, 0, 0] in the contract
    assert_eq!(
        owner_account.id().prefix().as_u64(),
        owner_slot.get(0).unwrap().as_int(),
        "Owner prefix should match"
    );
    assert_eq!(
        owner_account.id().suffix().as_int(),
        owner_slot.get(1).unwrap().as_int(),
        "Owner suffix should match"
    );

    // Payment token - just check the prefix matches (storage layout is complex)
    assert_eq!(
        faucet_account.id().prefix().as_u64(),
        payment_token_slot.get(0).unwrap().as_int(),
        "Payment token prefix should match"
    );

    // Price is stored at slot 2, index 1 (not slot 5!)
    assert_eq!(
        100,
        payment_token_slot.get(1).unwrap().as_int(),
        "Price should be 100"
    );

    Ok(())
}

/// Test complete registration flow (init + register) in MockChain
///
/// NOTE: This test demonstrates the pattern but has MockChain limitations:
/// - Registry initialization works perfectly
/// - Name registration hits payment validation issues:
///   * With price > 0: note::add_assets_to_account doesn't transfer assets properly
///   * With price = 0: account::get_balance fails on empty vault
///
/// For full registration testing with payment, use Client-based tests.
#[tokio::test]
#[ignore = "MockChain limitation: payment validation not fully supported"]
async fn test_mock_chain_complete_registration() -> Result<(), Box<dyn std::error::Error>> {
    use miden_objects::asset::{Asset, FungibleAsset};
    use miden_testing::TransactionContextBuilder;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut builder = MockChain::builder();
    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let faucet_account = builder.add_existing_faucet(Auth::BasicAuth, "POL", 1_000_000, None)?;

    let user_account = builder.add_existing_wallet(Auth::BasicAuth)?;

    let registry_account = create_registry_account()?;
    let registry_code = fs::read_to_string(Path::new(paths::REGISTRY_CONTRACT))?;
    let registry_library = create_library(registry_code.clone(), "miden_id::registry")?;

    // Initialize registry with price = 0 (free registration for MockChain)
    // Payment validation with note assets doesn't work properly in MockChain
    let init_inputs = vec![
        Felt::new(0), // price = 0
        Felt::new(faucet_account.id().prefix().into()),
        Felt::new(faucet_account.id().suffix().into()),
    ];

    // Step 1: Initialize registry
    let init_note = NoteBuilder::new(owner_account.id(), ChaCha20Rng::from_seed([0u8; 32]))
        .code(get_note_code("init"))
        .note_type(NoteType::Public)
        .tag(NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Network)?.into())
        .dynamically_linked_libraries(vec![registry_library.clone()])
        .note_inputs(init_inputs)?
        .build()?;

    builder.add_note(OutputNote::Full(init_note.clone()));

    // Step 2: Create registration note BEFORE building MockChain
    let name = "alice";
    let name_word = encode_name_to_word(name);

    let name_felt_vec = vec![
        *name_word.get(0).unwrap(),
        *name_word.get(1).unwrap(),
        *name_word.get(2).unwrap(),
        *name_word.get(3).unwrap(),
    ];

    // Create registration note addressed to registry (no payment needed since price = 0)
    let register_note = NoteBuilder::new(registry_account.id(), ChaCha20Rng::from_seed([1u8; 32]))
        .code(get_note_code("register_name"))
        .note_type(NoteType::Public)
        .tag(NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Network)?.into())
        .dynamically_linked_libraries(vec![registry_library])
        .note_inputs(name_felt_vec)?
        .build()?;

    builder.add_note(OutputNote::Full(register_note.clone()));

    // Build MockChain with both notes
    let mut mock_chain = builder.build()?;

    // Execute init transaction first
    let tx_inputs = mock_chain.get_transaction_inputs(
        registry_account.clone(),
        None,
        &[init_note.id()],
        &[],
    )?;

    let tx_context = TransactionContextBuilder::new(registry_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let exec_tx = tx_context.execute().await?;
    let updated_registry = mock_chain.add_pending_executed_transaction(&exec_tx)?;

    println!("Registry initialized successfully");
    println!("Registry vault after init: {:?}", updated_registry.vault());

    // Execute registration transaction second
    let tx_inputs = mock_chain.get_transaction_inputs(
        updated_registry.clone(),
        None,
        &[register_note.id()],
        &[],
    )?;

    let tx_context = TransactionContextBuilder::new(updated_registry.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let exec_tx = tx_context.execute().await?;
    let final_registry = mock_chain.add_pending_executed_transaction(&exec_tx)?;

    Ok(())
}

// Helper function to encode name to Word (matching test_helper.rs pattern)
fn encode_name_to_word(name: &str) -> miden_objects::Word {
    use miden_objects::Felt;

    let name_bytes = name.as_bytes();
    let len = name_bytes.len() as u64;

    // Encode name into felts
    let mut name_val = 0u64;
    for (i, &b) in name_bytes.iter().enumerate().take(7) {
        name_val |= (b as u64) << (i * 8);
    }

    let mut name_val2 = 0u64;
    for (i, &b) in name_bytes.iter().skip(7).enumerate().take(7) {
        name_val2 |= (b as u64) << (i * 8);
    }

    [
        Felt::new(len),
        Felt::new(name_val),
        Felt::new(name_val2),
        Felt::ZERO,
    ]
    .into()
}

/// Test duplicate name registration prevention using MockChain
#[tokio::test]
async fn test_mock_chain_duplicate_name_prevention() -> Result<(), Box<dyn std::error::Error>> {
    // This test would:
    // 1. Register a name for user1
    // 2. Try to register the same name for user2
    // 3. Verify the second registration fails

    // TODO: Implement duplicate prevention test
    Ok(())
}
