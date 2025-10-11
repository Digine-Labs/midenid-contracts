use miden_client::account::{StorageMap, StorageSlot};
use miden_crypto::{Felt, Word};

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