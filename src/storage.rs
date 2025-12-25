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
        StorageSlot::Map(StorageMap::new()), // domain prices map([0, letter_count, payment_token_prefix, payment_token_suffix] -> [PRICE])
        StorageSlot::Map(StorageMap::new()), // account to domain
        StorageSlot::Map(StorageMap::new()), // domain to account
        StorageSlot::Map(StorageMap::new()), // domain to owner
        StorageSlot::Map(StorageMap::new()), // ref rate slot
        StorageSlot::Map(StorageMap::new()), // ref -> total revenue
        StorageSlot::Map(StorageMap::new()), // ref -> claimed revenue
        empty_storage_value(), // domain count
        StorageSlot::Map(StorageMap::new()), // token -> total revenue
        StorageSlot::Map(StorageMap::new()), // total -> claimed revenue
        empty_storage_value(), // onchain init dummy
        ];
    return storage_slots;
}