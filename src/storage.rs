use miden_client::account::{StorageMap, StorageSlot};
use miden_crypto::{Felt, Word};

fn empty_storage_value() -> StorageSlot {
    StorageSlot::Value(Word::new([
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ]))
}

pub fn naming_storage() -> Vec<StorageSlot> {
    let storage_slots: Vec<StorageSlot> = vec![
        empty_storage_value(), // Init flag
        empty_storage_value(), // owner
        StorageSlot::Map(StorageMap::new()), // payment token -> price contract
        StorageSlot::Map(StorageMap::new()), // account to domain
        StorageSlot::Map(StorageMap::new()), // domain to account
        StorageSlot::Map(StorageMap::new()), // domain to owner
        StorageSlot::Map(StorageMap::new()), // calculate price root
        StorageSlot::Map(StorageMap::new()),
        StorageSlot::Map(StorageMap::new()),
        empty_storage_value(),
        StorageSlot::Map(StorageMap::new()),
        StorageSlot::Map(StorageMap::new()),
        StorageSlot::Map(StorageMap::new()),
        empty_storage_value(), // ONE YEAR TIMESTAMP
        ];
    return storage_slots;
}