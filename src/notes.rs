use miden_assembly::{
    DefaultSourceManager, Library, LibraryPath,
    ast::{Module, ModuleKind},
};
use miden_client::{
    Client, ScriptBuilder, account::AccountId, note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteTag, NoteType,
    }, transaction::TransactionKernel, keystore::FilesystemKeyStore,
};
use miden_crypto::{Felt, Word};
use rand::{RngCore, rngs::StdRng, Rng};
use std::{fs, path::Path, sync::Arc};

pub async fn create_note_for_naming_with_client(
    name: String,
    inputs: NoteInputs,
    sender: AccountId,
    _target_id: AccountId,
    assets: NoteAssets,
    client: &mut Client<FilesystemKeyStore<StdRng>>
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;

    let serial_num =     Word::new([
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
    ]);

    let note_script = client
        .script_builder()
        .with_dynamically_linked_library(&library)?
        .compile_note_script(&note_code)?;


    let recipient = NoteRecipient::new(serial_num, note_script, inputs.clone());
    //let tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let tag = NoteTag::from_account_id(_target_id);
    let metadata = NoteMetadata::new(
        sender,
        NoteType::Public,
        tag,
        NoteExecutionHint::none(),
        Felt::new(0),
    )?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

pub async fn create_note_for_naming(
    name: String,
    inputs: NoteInputs,
    sender: AccountId,
    _target_id: AccountId,
    assets: NoteAssets,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;
    let serial = generate_random_serial_number();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let recipient = NoteRecipient::new(serial, note_script, inputs.clone());
    let tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        sender,
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Felt::new(0),
    )?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

pub fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new(library_path)?,
            account_code,
            &source_manager,
        )
        .unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    Ok(library)
}

/// Generates a random serial number for note creation
///
/// Similar to TypeScript's generateRandomSerialNumber()
/// Returns a Word containing 4 random Felt values
pub fn generate_random_serial_number() -> Word {
    let mut rng = rand::rng();

    Word::new([
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
    ])
}
