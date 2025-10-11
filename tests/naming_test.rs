use miden_client::account::AccountStorage;
use miden_lib::transaction::memory::StorageSlot;

use crate::utils::{empty_storage_map, empty_storage_value};
mod utils;

fn naming_storage() -> Vec<StorageSlot> {
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

#[tokio::test]
async fn test_naming_initialize() -> Result<()>{
    let storage_slots = vec![AccountStorage::mock_item_0().slot]
}