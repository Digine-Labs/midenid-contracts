use anyhow::Error;
use miden_crypto::{Felt, Word, rand::FeltRng};
use std::{fs, path::Path};
use miden_client::{ScriptBuilder, account::{Account, AccountId}, asset::FungibleAsset, note::{Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient, NoteTag, NoteType}, rpc::Endpoint};

use crate::utils::{create_library, get_naming_account_code, get_pricing_account_code};
use crate::deploy::{instantiate_client};

pub fn get_note_code(note_name: String) -> String {
    fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", note_name))).unwrap()
}

pub async fn create_naming_initialize_note(tx_sender: AccountId, owner: AccountId, treasury: AccountId, naming: Account) -> Result<Note, Error> {
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
        Felt::new(treasury.suffix().into()),
        Felt::new(treasury.prefix().into()),
        Felt::new(owner.suffix().into()),
        Felt::new(owner.prefix().into())
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    //let note_recipient = NoteRecipient::new(Word::default(), note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(naming.id());

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_naming_set_payment_token_contract(tx_sender: AccountId, token: AccountId, pricing: AccountId, naming: AccountId) -> Result<Note, Error> {
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

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(naming);

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)  
}

pub async fn create_naming_set_pricing_root(tx_sender: AccountId, root: Word, pricing_contract: AccountId, naming: AccountId) -> Result<Note, Error> {
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
        Felt::new(pricing_contract.suffix().as_int()),
        Felt::new(pricing_contract.prefix().as_felt().as_int()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(root.get(0).unwrap().as_int()),
        Felt::new(root.get(1).unwrap().as_int()),
        Felt::new(root.get(2).unwrap().as_int()),
        Felt::new(root.get(3).unwrap().as_int())
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag =  NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?;//NoteTag::from_account_id(naming);

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)  
}

pub async fn create_naming_transfer_owner_note(tx_sender: AccountId, new_owner: AccountId, naming: AccountId) -> Result<Note, Error> {
    let note_code = get_note_code("transfer_ownership".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(new_owner.suffix().into()),
        Felt::new(new_owner.prefix().into()),
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?;//NoteTag::from_account_id(naming);

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_pricing_initialize_note(tx_sender: AccountId, token: AccountId, setter: AccountId, pricing: Account) -> Result<Note, Error> {
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
        Felt::new(setter.suffix().into()),
        Felt::new(setter.prefix().into()),
        Felt::new(token.suffix().into()),
        Felt::new(token.prefix().into())
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(pricing.id());

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_naming_register_name_note(tx_sender: AccountId, payment_token: AccountId, domain: Word, asset: FungibleAsset, naming: Account) -> Result<Note, Error> {
       let note_code = get_note_code("register_name".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(payment_token.suffix().as_int()),
        Felt::new(payment_token.prefix().as_felt().as_int()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(domain.get(0).unwrap().as_int()),
        Felt::new(domain.get(1).unwrap().as_int()),
        Felt::new(domain.get(2).unwrap().as_int()),
        Felt::new(domain.get(3).unwrap().as_int())
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(naming.id());

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![asset.into()]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)   
}

pub async fn create_naming_transfer_note(tx_sender: Account, receiver: AccountId, domain: Word, naming: Account) -> Result<Note, Error> {
    let note_code = get_note_code("transfer_domain".to_string());
    let account_code= get_naming_account_code();

    let library_path = "miden_name::naming";
    let library = create_library(account_code, library_path).unwrap();

    let note_script = ScriptBuilder::new(true)
        .with_dynamically_linked_library(&library)
        .unwrap()
        .compile_note_script(note_code)
        .unwrap();

    let note_inputs =NoteInputs::new([
        Felt::new(receiver.suffix().as_int()),
        Felt::new(receiver.prefix().as_felt().as_int()),
        Felt::new(0),
        Felt::new(0),
        Felt::new(domain.get(0).unwrap().as_int()),
        Felt::new(domain.get(1).unwrap().as_int()),
        Felt::new(domain.get(2).unwrap().as_int()),
        Felt::new(domain.get(3).unwrap().as_int())
    ].to_vec()).unwrap();

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(naming.id());

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

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?;// NoteTag::from_account_id(pricing.id());

    let note_metadata = NoteMetadata::new(tx_sender.id(), NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}

pub async fn create_price_set_note(tx_sender: AccountId, inputs: Vec<Felt>, pricing: AccountId) -> Result<Note, Error> {
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

    let mut client = instantiate_client(Endpoint::testnet())
    .await?;
    let serial_number = client.rng().draw_word();
    let note_recipient = NoteRecipient::new(serial_number, note_script, note_inputs.clone());

    let note_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?; //NoteTag::from_account_id(pricing);

    let note_metadata = NoteMetadata::new(tx_sender, NoteType::Public, note_tag, NoteExecutionHint::Always, Felt::new(0)).unwrap();

    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note = Note::new(note_assets, note_metadata, note_recipient);
    Ok(note)
}