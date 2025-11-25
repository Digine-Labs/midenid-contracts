use miden_client::{Felt, Word};
use miden_lib::account::auth;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{Account, AccountBuilder, AccountComponent, AccountId, AccountStorageMode, StorageMap, StorageSlot},
    assembly::{Assembler, DefaultSourceManager, Library, LibraryPath, Module, ModuleKind},
    asset::FungibleAsset,
    note::{Note, NoteAssets, NoteExecutionHint, NoteId, NoteInputs, NoteMetadata, NoteRecipient, NoteTag, NoteType},
    testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1,
    transaction::OutputNote,
};
use miden_testing::{Auth, MockChain, MockChainBuilder};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::{fs, path::Path, sync::Arc};

// ================================================================================================
// CONSTANTS
// ================================================================================================

// Using the public fungible faucet from miden_objects::testing
fn get_test_faucet_id() -> AccountId {
    AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1).unwrap()
}

// ================================================================================================
// STORAGE HELPERS
// ================================================================================================

/// Creates storage slots for the naming/registry contract
pub fn naming_storage() -> Vec<StorageSlot> {
    vec![
        empty_storage_value(), // Slot 0: Initialization flag
        empty_storage_value(), // Slot 1: Owner
        empty_storage_value(), // Slot 2: Payment token
        empty_storage_map(),   // Slot 3: Name->ID mapping
        empty_storage_map(),   // Slot 4: ID->Name mapping
        empty_storage_value(), // Slot 5: Price
    ]
}

fn empty_storage_value() -> StorageSlot {
    StorageSlot::Value(Word::new([
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ]))
}

fn empty_storage_map() -> StorageSlot {
    StorageSlot::Map(StorageMap::new())
}

// ================================================================================================
// ACCOUNT CREATION
// ================================================================================================

/// Creates a test naming/registry account with proper storage slots
pub fn create_test_naming_account() -> Account {
    let storage_slots = naming_storage();
    let code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();

    let component = AccountComponent::compile(
        code.clone(),
        TransactionKernel::assembler().with_debug_mode(true),
        storage_slots,
    )
    .unwrap()
    .with_supports_all_types();

    let account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .with_component(component)
        .storage_mode(AccountStorageMode::Public)
        .build_existing()
        .unwrap();

    account
}

// ================================================================================================
// NOTE CREATION
// ================================================================================================

/// Creates a note for the naming contract with library linking
pub async fn create_note_for_naming(
    name: String,
    inputs: NoteInputs,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
    let library = create_library(naming_code, "miden_id::registry")?;

    let note_script = miden_client::ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let recipient = NoteRecipient::new(Word::default(), note_script, inputs.clone());
    let tag = NoteTag::from_account_id(target_id);
    let metadata = NoteMetadata::new(
        sender,
        NoteType::Public,
        tag,
        NoteExecutionHint::Always,
        Felt::new(0),
    )?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

/// Creates a note with a custom serial number
pub async fn create_note_for_naming_with_custom_serial_num(
    name: String,
    inputs: NoteInputs,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
    serial_num: Word,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/miden_id.masm")).unwrap();
    let library = create_library(naming_code, "miden_id::registry")?;

    let note_script = miden_client::ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let recipient = NoteRecipient::new(serial_num, note_script, inputs.clone());
    let tag = NoteTag::from_account_id(target_id);
    let metadata = NoteMetadata::new(
        sender,
        NoteType::Public,
        tag,
        NoteExecutionHint::Always,
        Felt::new(0),
    )?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

// ================================================================================================
// LIBRARY CREATION
// ================================================================================================

fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new(library_path).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            account_code,
            &source_manager,
        )
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let library = assembler
        .clone()
        .assemble_library([module])
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    Ok(library)
}

// ================================================================================================
// TEST CONTEXT
// ================================================================================================

/// Testing context that holds all necessary state for tests
pub struct TestingContext {
    pub builder: MockChainBuilder,
    pub owner: Account,
    pub user_1: Account,
    pub user_2: Account,
    pub user_3: Account,
    pub naming: Account,
    pub fungible_asset: FungibleAsset,
    pub initialize_note: Note,
}

/// Initializes a complete testing context with registry and users
pub async fn init_naming() -> anyhow::Result<TestingContext> {
    let mut builder = MockChain::builder();
    let faucet_id = get_test_faucet_id();
    let fungible_asset = FungibleAsset::new(faucet_id, 100000).unwrap();
    let fungible_asset_2 = FungibleAsset::new(faucet_id, 50000).unwrap();
    let fungible_asset_3 = FungibleAsset::new(faucet_id, 20000).unwrap();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let user_account_1 = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth,
        vec![fungible_asset.into()],
    )?;
    let user_account_2 = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth,
        vec![fungible_asset_2.into()],
    )?;
    let user_account_3 = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth,
        vec![fungible_asset_3.into()],
    )?;
    let naming_account = create_test_naming_account();
    builder.add_account(naming_account.clone())?;

    // Create initialization note
    // init function expects: [price, token_prefix, token_suffix]
    let initialize_inputs = NoteInputs::new(
        [
            Felt::new(100), // price
            fungible_asset.faucet_id().prefix().as_felt(),
            Felt::new(fungible_asset.faucet_id().suffix().into()),
        ]
        .to_vec(),
    )?;
    let init_note = create_note_for_naming(
        "init".to_string(),
        initialize_inputs,
        owner_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    add_note_to_builder(&mut builder, init_note.clone())?;

    Ok(TestingContext {
        builder,
        owner: owner_account,
        user_1: user_account_1,
        user_2: user_account_2,
        user_3: user_account_3,
        naming: naming_account,
        fungible_asset,
        initialize_note: init_note,
    })
}

// ================================================================================================
// MOCKCHAIN HELPERS
// ================================================================================================

/// Adds a note to the MockChainBuilder
pub fn add_note_to_builder(builder: &mut MockChainBuilder, note: Note) -> anyhow::Result<()> {
    builder.add_output_note(OutputNote::Full(note.clone()));
    Ok(())
}

/// Executes notes and builds the chain
pub async fn execute_notes_and_build_chain(
    builder: MockChainBuilder,
    note_ids: &[NoteId],
    target: &mut Account,
) -> anyhow::Result<MockChain> {
    let mut chain = builder.build()?;

    for note_id in note_ids {
        execute_note(&mut chain, *note_id, target).await?;
    }
    Ok(chain)
}

/// Executes a single note on the mockchain
/// Target account must be updated with the returned account state
pub async fn execute_note(
    chain: &mut MockChain,
    note_id: NoteId,
    target: &mut Account,
) -> anyhow::Result<()> {
    let tx_ctx = chain
        .build_tx_context(target.id(), &[note_id], &[])?
        .build()?;

    let executed_tx = tx_ctx.execute().await?;

    target.apply_delta(&executed_tx.account_delta())?;
    chain.add_pending_executed_transaction(&executed_tx)?;
    chain.prove_next_block()?;

    Ok(())
}

// ================================================================================================
// ENCODING HELPERS
// ================================================================================================

/// Encodes a domain name as a Word for storage
pub fn encode_domain(name: String) -> Word {
    let felts = encode_domain_as_felts(name);
    Word::new([felts[0], felts[1], felts[2], felts[3]])
}

/// Encodes a domain name as a vector of Felts
pub fn encode_domain_as_felts(name: String) -> Vec<Felt> {
    let bytes = name.as_bytes();
    let len = bytes.len() as u64;

    // Encode: [length, packed_bytes, 0, 0]
    // Pack bytes into a single felt (simplified - real implementation may differ)
    let mut packed: u64 = 0;
    for (i, &byte) in bytes.iter().enumerate().take(8) {
        packed |= (byte as u64) << (i * 8);
    }

    vec![Felt::new(len), Felt::new(packed), Felt::new(0), Felt::new(0)]
}
