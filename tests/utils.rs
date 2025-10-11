use std::{fs, path::Path, sync::Arc};

use miden_assembly::{ast::{Module, ModuleKind}, Assembler, DefaultSourceManager, Library, LibraryPath};
use miden_client::{account::{AccountBuilder, AccountType, StorageMap, StorageSlot}, crypto::SecretKey, testing::NoteBuilder};
use miden_crypto::{Felt, Word};
use miden_objects::account::{AccountComponent, AccountStorageMode, Account, AccountId};
use miden_objects::note::{NoteType};
use miden_lib::{account::{auth, wallets::BasicWallet}, transaction::TransactionKernel};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub fn naming_storage() -> Vec<StorageSlot> {
    let storage_slots: Vec<StorageSlot> = vec![
        empty_storage_value(), // Init flag
        empty_storage_value(), // owner
        empty_storage_value(), // treasury
        empty_storage_map(), // payment token -> price contract
        empty_storage_map(), // account to domain
        empty_storage_map(), // domain to account
        empty_storage_map(), // domain to owner
        ];
    return storage_slots;
}

mod paths {
    pub const NAMING_ACCOUNT: &str = "./masm/accounts/naming.masm";
}

pub fn empty_storage_value() -> StorageSlot {
    StorageSlot::Value(Word::new([
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ]))
}

pub fn empty_storage_map() -> StorageSlot {
    StorageSlot::Map(StorageMap::new())
}

pub fn get_naming_account_code() -> String {
    fs::read_to_string(Path::new(paths::NAMING_ACCOUNT)).unwrap()
}

pub fn get_note_code(note_name: String) -> String {
    fs::read_to_string(Path::new(&format!("./masm/notes/{}.masm", note_name))).unwrap()
}

pub fn create_account() -> anyhow::Result<Account> {
    let mut rng = ChaCha20Rng::from_os_rng();
    let key_pair = SecretKey::with_rng(&mut rng);
    let (account, seed) = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(auth::AuthRpoFalcon512::new(key_pair.public_key().clone()))
        .with_component(BasicWallet).build()?;

    Ok(account)
}

pub fn create_naming_account() -> Account {
    let storage_slots = naming_storage();
    let account_code = get_naming_account_code();

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .with_component(account_component)
        .storage_mode(AccountStorageMode::Public)
        .build_existing().unwrap();
    return account;
}

pub fn create_public_note(note_name: String, sender: AccountId) {

}

pub fn create_naming_library() -> Result<Library, Box<dyn std::error::Error>> {
    let account_code = get_naming_account_code();
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new("miden_name::naming")?,
        account_code,
        &source_manager,
    )?;
    let library = assembler.clone().assemble_library([module])?;
    Ok(library)
}