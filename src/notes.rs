use anyhow::Error;
use miden_crypto::{Felt, Word};
use std::{fs, path::Path, sync::Arc};
use miden_client::{account::{Account, AccountId}, note::{Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag, NoteType}, ScriptBuilder};

use crate::utils::{create_library, get_naming_account_code, get_pricing_account_code};

pub fn get_note_code(note_name: String) -> String {
    fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", note_name))).unwrap()
}

pub async fn create_naming_initialize_note(owner: Account, treasury: Account, naming: Account) -> Result<Note, Error> {
    let note_code = get_note_code("initialize_naming".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(treasury.id().suffix().into()),
        Felt::new(treasury.id().prefix().into()),
        Felt::new(owner.id().suffix().into()),
        Felt::new(owner.id().prefix().into())
    ].to_vec()).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(naming.id());

    let note_metadata = NoteMetadata::new(owner.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_pricing_initialize_note(tx_sender: Account, token: AccountId, setter: Account, pricing: Account) -> Result<Note, Error> {
    let note_code = get_note_code("initialize_pricing".to_string());
    let account_code= get_pricing_account_code();

    let library_path = "miden_name::pricing";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(setter.id().suffix().into()),
        Felt::new(setter.id().prefix().into()),
        Felt::new(token.suffix().into()),
        Felt::new(token.prefix().into())
    ].to_vec()).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(pricing.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}