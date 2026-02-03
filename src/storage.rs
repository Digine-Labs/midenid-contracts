use miden_protocol::account::{StorageMap, StorageSlot, StorageSlotName};
use miden_crypto::Word;

pub fn slot_name(name: &str) -> StorageSlotName {
    StorageSlotName::new(name).expect("invalid storage slot name")
}

pub fn naming_storage() -> Vec<StorageSlot> {
    vec![
        StorageSlot::with_value(slot_name("naming::init_flag"), Word::default()),
        StorageSlot::with_value(slot_name("naming::owner"), Word::default()),
        StorageSlot::with_map(slot_name("naming::prices"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::account_to_domain"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::domain_to_account"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::domain_to_owner"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::ref_rate"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::ref_total_revenue"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::ref_claimed_revenue"), StorageMap::new()),
        StorageSlot::with_value(slot_name("naming::domain_count"), Word::default()),
        StorageSlot::with_map(slot_name("naming::total_revenue"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::claimed_revenue"), StorageMap::new()),
        StorageSlot::with_map(slot_name("naming::domain_expiry"), StorageMap::new()),
        StorageSlot::with_value(slot_name("naming::one_year_timestamp"), Word::default()),
        StorageSlot::with_value(slot_name("naming::onchain_init"), Word::default()),
    ]
}
