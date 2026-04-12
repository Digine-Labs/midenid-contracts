use miden_assembly::{
    DefaultSourceManager, Library,
    ast::{Module, ModuleKind},
};
use miden_client::{
    Client,
    account::AccountId,
    keystore::FilesystemKeyStore,
    note::{Note, NoteAssets, NoteMetadata, NoteRecipient, NoteStorage, NoteTag, NoteType},
};
use miden_crypto::{Felt, Word};
use miden_protocol::transaction::TransactionKernel;
use miden_standards::code_builder::CodeBuilder;
use miden_standards::note::{NetworkAccountTarget, NoteExecutionHint};
use rand::{Rng, RngCore};
use std::{fs, path::Path, sync::Arc};

pub async fn create_note_for_naming_with_client(
    name: String,
    inputs: NoteStorage,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
    client: &mut Client<FilesystemKeyStore>,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming_unsafe.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;

    let serial_num = Word::new([
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
        Felt::new(client.rng().next_u64()),
    ]);

    let note_script = CodeBuilder::default()
        .with_dynamically_linked_library(&library)?
        .compile_note_script(note_code)?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs.clone());
    let tag = NoteTag::with_account_target(target_id);
    let mut metadata = NoteMetadata::new(sender, NoteType::Public).with_tag(tag);
    if target_id.is_network() {
        let network_target = NetworkAccountTarget::new(target_id, NoteExecutionHint::Always)?;
        metadata = metadata.with_attachment(network_target.into());
    }
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

pub async fn create_note_for_naming(
    name: String,
    inputs: NoteStorage,
    sender: AccountId,
    target_id: AccountId,
    assets: NoteAssets,
) -> anyhow::Result<Note> {
    let note_code = fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", name)))?;
    let naming_code = fs::read_to_string(Path::new("./masm/accounts/naming.masm")).unwrap();
    let library = create_library(naming_code, "miden_name::naming")?;
    let serial = generate_random_serial_number();

    let note_script = CodeBuilder::default()
        .with_dynamically_linked_library(&library)?
        .compile_note_script(note_code)?;

    let recipient = NoteRecipient::new(serial, note_script, inputs.clone());
    let tag = NoteTag::with_account_target(target_id);
    let metadata = NoteMetadata::new(sender, NoteType::Public).with_tag(tag);
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

pub fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let assembler = TransactionKernel::assembler_with_source_manager(source_manager.clone())
        .with_dynamic_library(miden_standards::StandardsLib::default())
        .expect("failed to load standards lib");
    let module = Module::parser(ModuleKind::Library)
        .parse_str(library_path, account_code, source_manager)
        .unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    Ok((*library).clone())
}

/// Generates a random serial number for note creation
pub fn generate_random_serial_number() -> Word {
    let mut rng = rand::rng();

    Word::new([
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
        Felt::new(rng.random::<u32>() as u64),
    ])
}
