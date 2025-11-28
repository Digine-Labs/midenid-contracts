use miden_assembly::{DefaultSourceManager, Library, LibraryPath, ast::{Module, ModuleKind}};
use miden_client::{
    Client, ScriptBuilder, account::{AccountBuilder, AccountId, AccountStorageMode, AccountType}, auth::NoAuth, keystore::FilesystemKeyStore, note::{Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag, NoteType}, transaction::TransactionKernel
};
use miden_crypto::{Felt, Word};
use miden_objects::account::AccountComponent;
use rand::{RngCore, rngs::StdRng};
use std::{fs, path::Path, sync::Arc};

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
    let metadata = NoteMetadata::new(sender, NoteType::Public, tag, NoteExecutionHint::none(), Felt::new(0))?;
    let note = Note::new(assets, metadata, recipient);
    Ok(note)
}

pub fn create_library(account_code: String, library_path: &str) -> anyhow::Result<Library> {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new(library_path)?,
        account_code,
        &source_manager,
    ).unwrap();
    let library = assembler.clone().assemble_library([module]).unwrap();

    Ok(library)
}