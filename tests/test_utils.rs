use std::{fs, ops::Not, path::Path, sync::Arc};

use anyhow::Ok;
use miden_assembly::{Assembler, DefaultSourceManager, Library, LibraryPath, ast::{Module, ModuleKind}};
use miden_client::{ScriptBuilder, account::{Account, AccountBuilder, AccountId, AccountStorageMode}, asset::FungibleAsset, note::{Note, NoteAssets, NoteExecutionHint, NoteId, NoteInputs, NoteMetadata, NoteRecipient, NoteTag, NoteType}, testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1, transaction::OutputNote};
use miden_crypto::{Felt, Word};
use miden_lib::{account::auth, transaction::TransactionKernel};
use miden_objects::account::AccountComponent;
use miden_testing::{Auth, MockChain, MockChainBuilder, TransactionContextBuilder};
use midenname_contracts::storage::naming_storage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

fn create_test_naming_account() -> Account {
    let storage_slots = naming_storage();
    let code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();

    let component = AccountComponent::compile(
        code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .with_component(component)
        .storage_mode(AccountStorageMode::Public)
        .build_existing().unwrap();

    account
}

pub fn get_test_prices() -> Vec<Felt> {
    vec![Felt::new(0), Felt::new(123123), Felt::new(45645), Felt::new(789), Felt::new(555), Felt::new(123)]
}

pub struct TestingContext {
    pub builder: MockChainBuilder,
    //pub chain: MockChain,
    pub owner: Account,
    pub registrar_1: Account,
    pub registrar_2: Account,
    pub registrar_3: Account,
    pub naming: Account,
    pub fungible_asset: FungibleAsset,
    pub one_year: u32,
    pub initialize_note: Note,
    pub set_prices_note: Note
}

pub async fn init_naming() -> anyhow::Result<TestingContext> {
    let mut builder = MockChain::builder();
    let fungible_asset_1 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();
    let fungible_asset_2 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 50000).unwrap();
    let fungible_asset_3 = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 20000).unwrap();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let domain_registrar_account = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_1.into()])?;
    let domain_registrar_account_2 = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_2.into()])?;
    let domain_registrar_account_3 = builder.add_existing_wallet_with_assets(Auth::BasicAuth, vec![fungible_asset_3.into()])?;
    let mut naming_account = create_test_naming_account();
    builder.add_account(naming_account.clone())?;
    //let mut mockchain = builder.build()?;
    let one_year_time: u32 = 500;

    let initialize_inputs = NoteInputs::new([
        Felt::new(owner_account.id().suffix().into()),
        Felt::new(owner_account.id().prefix().into()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(one_year_time.into()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ].to_vec())?;
    let init_note = create_note_for_naming("initialize_naming".to_string(), initialize_inputs, owner_account.id(), naming_account.id(), NoteAssets::new(vec![]).unwrap()).await?;
    
    //execute_note(&mut mockchain, init_note, &mut naming_account).await?;
    add_note_to_builder(&mut builder, init_note.clone())?;
    // Set prices

    let note_inputs = NoteInputs::new([
            Felt::new(fungible_asset_1.faucet_id().suffix().into()),
            Felt::new(fungible_asset_1.faucet_id().prefix().into()),
        ].to_vec())?;
    let set_prices_note = create_note_for_naming("set_all_prices".to_string(), note_inputs, owner_account.id(), naming_account.id(), NoteAssets::new(vec![]).unwrap()).await?;

    add_note_to_builder(&mut builder, set_prices_note.clone())?;
    //set_test_prices(&mut mockchain, owner_account.id(), &mut naming_account, fungible_asset_1.faucet_id()).await?;
    //add_set_prices_notes(&mut builder,owner_account.id(), &mut naming_account, fungible_asset_1.faucet_id()).await?;

    Ok(TestingContext { builder: builder, owner: owner_account, registrar_1: domain_registrar_account, 
        registrar_2: domain_registrar_account_2, registrar_3: domain_registrar_account_3, naming: naming_account, 
        fungible_asset: fungible_asset_1, one_year: one_year_time, initialize_note: init_note, set_prices_note: set_prices_note })
}

pub fn add_note_to_builder(builder: &mut MockChainBuilder, note: Note) -> anyhow::Result<()> {
    builder.add_output_note(OutputNote::Full(note.clone()));

    Ok(())
}

pub async fn execute_notes_and_build_chain(builder: MockChainBuilder, note_ids: &[NoteId], target: &mut Account) -> anyhow::Result<MockChain> {
    let mut chain = builder.build()?;

    for note_id in note_ids {
        execute_note(&mut chain, *note_id, target).await?;
    }
    Ok(chain)
}

// Target must be updated account always which is returned from this function. do not use ctx.naming all the time
pub async fn execute_note(chain: &mut MockChain, note_id: NoteId, target: &mut Account) -> anyhow::Result<()> {
    let tx_ctx = chain.build_tx_context(target.id(), &[note_id], &[])?.build()?;

    let executed_tx = tx_ctx.execute().await?;

    target.apply_delta(&executed_tx.account_delta())?;
    chain.add_pending_executed_transaction(&executed_tx)?;
    chain.prove_next_block()?;

    Ok(())
}

pub async fn create_note_for_naming(name: String, inputs: NoteInputs, sender: AccountId, target_id: AccountId, assets: NoteAssets) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let recipient = NoteRecipient::new(Word::default(), note_script, inputs.clone());
    let tag = NoteTag::from_account_id(target_id);
    let metadata = NoteMetadata::new(sender, NoteType::Public, tag, NoteExecutionHint::Always, Felt::new(0))?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new(library_path)?,
        account_code,
        &source_manager,
    ).unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    Ok(library)
}