use std::{fs, path::Path, sync::Arc};

use miden_assembly::{
    DefaultSourceManager, Library,
    ast::{Module, ModuleKind},
};
use miden_client::{
    account::{Account, AccountBuilder, AccountId, AccountType},
    asset::{Asset, FungibleAsset},
    note::{
        Note, NoteAssets, NoteId, NoteRecipient, NoteStorage, NoteTag, NoteType,
        PartialNoteMetadata,
    },
    testing::account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1,
};
use miden_crypto::{Felt, Word};
use miden_protocol::{
    account::{AccountComponent, AccountComponentMetadata},
    transaction::{RawOutputNote, TransactionKernel},
};
use miden_standards::StandardsLib;
use miden_standards::code_builder::CodeBuilder;
use miden_standards::{account::auth, note::P2idNoteStorage};
use miden_testing::{Auth, MockChain, MockChainBuilder};
use midenname_contracts::storage::naming_storage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Path of the contract variant the plain (non-`_with_contract`) helpers build against.
pub const NAMING_CONTRACT: &str = "./masm/accounts/naming.masm";
pub const NAMING_DISCOUNT_CONTRACT: &str = "./masm/accounts/naming_discount.masm";

pub fn create_test_naming_account() -> Account {
    create_test_account_from(NAMING_CONTRACT)
}

/// Builds a test account from any of the naming contract variants. They all share
/// [`naming_storage`], so only the code differs.
pub fn create_test_account_from(contract_path: &str) -> Account {
    let storage_slots = naming_storage();
    let code = fs::read_to_string(Path::new(contract_path)).unwrap();

    let source_manager = Arc::new(DefaultSourceManager::default());
    let assembler = TransactionKernel::assembler_with_source_manager(source_manager.clone())
        .with_dynamic_library(StandardsLib::default())
        .expect("failed to load standards lib");
    let module = Module::parser(ModuleKind::Library)
        .parse_str("naming", code, source_manager)
        .unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    let component = AccountComponent::new(
        (*library).clone(),
        storage_slots,
        AccountComponentMetadata::new("midenid-naming"),
    )
    .unwrap();

    let account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .account_type(AccountType::Public)
        .with_auth_component(auth::NoAuth)
        .with_component(component)
        .build_existing()
        .unwrap();

    account
}

pub async fn create_note_for_naming(
    name: String,
    inputs: NoteStorage,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
) -> anyhow::Result<Note> {
    create_note_for_naming_with_contract(name, inputs, sender, target_id, assets, NAMING_CONTRACT)
        .await
}

/// Same as [`create_note_for_naming`] but links the note script against a chosen contract
/// variant, so notes can target the discount contract's procedure signatures.
pub async fn create_note_for_naming_with_contract(
    name: String,
    inputs: NoteStorage,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
    contract_path: &str,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new(contract_path)).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;

    let note_script = CodeBuilder::default()
        .with_dynamically_linked_library(&library)?
        .compile_note_script(note_code)?;

    let recipient = NoteRecipient::new(Word::default(), note_script, inputs.clone());
    let tag = NoteTag::with_account_target(target_id);
    let partial = PartialNoteMetadata::new(sender, NoteType::Public).with_tag(tag);
    let note = Note::new(assets, partial, recipient);
    Ok(note)
}

pub async fn create_note_for_naming_with_custom_serial_num(
    name: String,
    inputs: NoteStorage,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
    serial_num: Word,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;

    let note_script = CodeBuilder::default()
        .with_dynamically_linked_library(&library)?
        .compile_note_script(note_code)?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs.clone());
    let tag = NoteTag::with_account_target(target_id);
    let partial = PartialNoteMetadata::new(sender, NoteType::Public).with_tag(tag);
    let note = Note::new(assets, partial, recipient);
    Ok(note)
}

pub fn create_p2id_note_exact(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    serial_num: Word,
) -> anyhow::Result<Note> {
    let recipient = build_p2id_recipient(target, serial_num)?;

    let tag = NoteTag::with_account_target(target);

    let partial = PartialNoteMetadata::new(sender, note_type).with_tag(tag);
    let vault = NoteAssets::new(assets)?;

    Ok(Note::new(vault, partial, recipient))
}

pub fn build_p2id_recipient(target: AccountId, serial_num: Word) -> anyhow::Result<NoteRecipient> {
    let p2id_storage = P2idNoteStorage::new(target);
    Ok(p2id_storage.into_recipient(serial_num))
}

pub fn get_test_prices() -> Vec<Felt> {
    vec![
        Felt::new(0).unwrap(),
        Felt::new(123123).unwrap(),
        Felt::new(45645).unwrap(),
        Felt::new(789).unwrap(),
        Felt::new(555).unwrap(),
        Felt::new(123).unwrap(),
    ]
}

pub struct TestingContext {
    pub builder: MockChainBuilder,
    pub owner: Account,
    pub registrar_1: Account,
    pub registrar_2: Account,
    pub registrar_3: Account,
    pub naming: Account,
    pub fungible_asset: FungibleAsset,
    pub one_year: u32,
    pub initialize_note: Note,
    pub set_prices_note: Note,
}

pub async fn init_naming() -> anyhow::Result<TestingContext> {
    init_naming_with(100000, "set_all_prices").await
}

/// Same as [`init_naming`] but lets a test choose how much registrar_1 is funded with and which
/// price-table note is used. Needed to exercise balances and prices above `u32::MAX`.
pub async fn init_naming_with(
    registrar_1_funding: u64,
    prices_note: &str,
) -> anyhow::Result<TestingContext> {
    let mut builder = MockChain::builder();
    let fungible_asset_1 = FungibleAsset::new(
        ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(),
        registrar_1_funding,
    )
    .unwrap();
    let fungible_asset_2 = FungibleAsset::new(
        ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(),
        50000,
    )
    .unwrap();
    let fungible_asset_3 = FungibleAsset::new(
        ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(),
        20000,
    )
    .unwrap();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth {
        auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
    })?;
    let domain_registrar_account = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth {
            auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
        },
        vec![fungible_asset_1.into()],
    )?;
    let domain_registrar_account_2 = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth {
            auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
        },
        vec![fungible_asset_2.into()],
    )?;
    let domain_registrar_account_3 = builder.add_existing_wallet_with_assets(
        Auth::BasicAuth {
            auth_scheme: miden_client::auth::AuthScheme::Falcon512Poseidon2,
        },
        vec![fungible_asset_3.into()],
    )?;
    let naming_account = create_test_naming_account();
    builder.add_account(naming_account.clone())?;
    let one_year_time: u32 = 500;

    let initialize_inputs = NoteStorage::new(
        [
            owner_account.id().suffix(),
            owner_account.id().prefix().as_felt(),
            Felt::new(0)?,
            Felt::new(0)?,
        ]
        .to_vec(),
    )?;
    let init_note = create_note_for_naming(
        "initialize_naming".to_string(),
        initialize_inputs,
        owner_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    add_note_to_builder(&mut builder, init_note.clone())?;

    let note_inputs = NoteStorage::new(
        [
            fungible_asset_1.faucet_id().suffix(),
            fungible_asset_1.faucet_id().prefix().as_felt(),
        ]
        .to_vec(),
    )?;
    let set_prices_note = create_note_for_naming(
        prices_note.to_string(),
        note_inputs,
        owner_account.id(),
        naming_account.id(),
        NoteAssets::new(vec![]).unwrap(),
    )
    .await?;

    add_note_to_builder(&mut builder, set_prices_note.clone())?;

    Ok(TestingContext {
        builder,
        owner: owner_account,
        registrar_1: domain_registrar_account,
        registrar_2: domain_registrar_account_2,
        registrar_3: domain_registrar_account_3,
        naming: naming_account,
        fungible_asset: fungible_asset_1,
        one_year: one_year_time,
        initialize_note: init_note,
        set_prices_note,
    })
}

pub fn add_note_to_builder(builder: &mut MockChainBuilder, note: Note) -> anyhow::Result<()> {
    builder.add_output_note(RawOutputNote::Full(note.clone()));

    Ok(())
}

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

// Target must be updated account always which is returned from this function. do not use ctx.naming all the time
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

pub async fn execute_note_with_expected_output(
    chain: &mut MockChain,
    note_id: NoteId,
    target: &mut Account,
    expected_output_notes: Vec<RawOutputNote>,
) -> anyhow::Result<()> {
    let tx_ctx = chain
        .build_tx_context(target.id(), &[note_id], &[])?
        .extend_expected_output_notes(expected_output_notes)
        .build()?;

    let executed_tx = tx_ctx.execute().await?;

    target.apply_delta(&executed_tx.account_delta())?;
    chain.add_pending_executed_transaction(&executed_tx)?;
    chain.prove_next_block()?;

    Ok(())
}

fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let assembler = TransactionKernel::assembler_with_source_manager(source_manager.clone())
        .with_dynamic_library(StandardsLib::default())
        .expect("failed to load standards lib");
    let module = Module::parser(ModuleKind::Library)
        .parse_str(library_path, account_code, source_manager)
        .unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    Ok((*library).clone())
}
