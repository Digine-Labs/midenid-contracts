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

pub async fn create_naming_set_payment_token_contract(tx_sender: Account, token: AccountId, pricing: AccountId, naming: Account) -> Result<Note, Error> {
    let note_code = get_note_code("set_payment_token".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(pricing.suffix().into()),
        Felt::new(pricing.prefix().into()),
        Felt::new(token.suffix().into()),
        Felt::new(token.prefix().into())
    ].to_vec()).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(naming.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)  
}

pub async fn create_naming_set_pricing_root(tx_sender: Account, root: Word, naming: Account) -> Result<Note, Error> {
    let note_code = get_note_code("set_pricing_root".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(root.get(0).unwrap().as_int()),
        Felt::new(root.get(1).unwrap().as_int()),
        Felt::new(root.get(2).unwrap().as_int()),
        Felt::new(root.get(3).unwrap().as_int())
    ].to_vec()).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(naming.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

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

pub async fn create_pricing_calculate_cost_note(tx_sender: Account, domain_word: Word, pricing: Account, expected_price: u64) -> Result<Note, Error> {
    let domain_price_note_code = format!(
        r#"
    use.miden_name::pricing
    use.miden::note
    use.std::sys

    const.WRONG_PRICE="Wrong price returned"

    begin
        push.{f4}.{f3}.{f2}.{length}
        call.pricing::calculate_domain_cost
        # [price]
        push.{expected_price}
        eq assert.err=WRONG_PRICE
        exec.sys::truncate_stack
    end
    "#,
        length = domain_word[3],
        f2 = domain_word[2],
        f3 = domain_word[1],
        f4 = domain_word[0],
        expected_price = expected_price // Replace with actual expected price value
    );
    //let note_code = fs::read_to_string(Path::new("./masm/scripts/calculate_domain_price.masm")).unwrap();
    let account_code = get_pricing_account_code();

    let library_path = "miden_name::pricing";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(domain_price_note_code)
        .unwrap();

    // domain_word format: [length, felt1, felt2, felt3]
    let note_inputs = NoteInputs::new(vec![
        domain_word[0],
        domain_word[1],
        domain_word[2],
        domain_word[3]
    ]).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(pricing.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_price_set_note(tx_sender: Account,inputs: Vec<Felt>, pricing: Account) -> Result<Note, Error> {
    let note_code = get_note_code("pricing_set_price".to_string());
    let account_code = get_pricing_account_code();

    let library_path = "miden_name::pricing";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs = NoteInputs::new(inputs).unwrap();

    let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::from_account_id(pricing.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}