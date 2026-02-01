use miden_client::account::StorageSlot;
use miden_protocol::account::StorageSlotName;

pub fn slot_name(name: &str) -> StorageSlotName {
    StorageSlotName::new(name).expect("Invalid storage slot name")
}

pub const INIT_FLAG_SLOT: &str = "naming::init_flag";
pub const OWNER_SLOT: &str = "naming::owner";
pub const PRICES_SLOT: &str = "naming::prices";
pub const ID_TO_DOMAIN_SLOT: &str = "naming::id_to_domain";
pub const DOMAIN_TO_ACCOUNT_SLOT: &str = "naming::domain_to_account";
pub const DOMAIN_TO_OWNER_SLOT: &str = "naming::domain_to_owner";
pub const CLAIMED_REVENUE_SLOT: &str = "naming::claimed_revenue";
pub const REFERRAL_REVENUE_SLOT: &str = "naming::referral_revenue";
pub const REFERRAL_RATE_SLOT: &str = "naming::referral_rate";
pub const TOTAL_DOMAIN_COUNT_SLOT: &str = "naming::total_domain_count";
pub const PROTOCOL_REVENUE_SLOT: &str = "naming::protocol_revenue";
pub const REFERRAL_CLAIMED_SLOT: &str = "naming::referral_claimed";
pub const DOMAIN_EXPIRY_SLOT: &str = "naming::domain_expiry";
pub const ONE_YEAR_TIMESTAMP_SLOT: &str = "naming::one_year_timestamp";

pub fn naming_storage() -> Vec<StorageSlot> {
    vec![
        StorageSlot::with_empty_value(slot_name(INIT_FLAG_SLOT)),
        StorageSlot::with_empty_value(slot_name(OWNER_SLOT)),
        StorageSlot::with_empty_map(slot_name(PRICES_SLOT)),
        StorageSlot::with_empty_map(slot_name(ID_TO_DOMAIN_SLOT)),
        StorageSlot::with_empty_map(slot_name(DOMAIN_TO_ACCOUNT_SLOT)),
        StorageSlot::with_empty_map(slot_name(DOMAIN_TO_OWNER_SLOT)),
        StorageSlot::with_empty_map(slot_name(CLAIMED_REVENUE_SLOT)),
        StorageSlot::with_empty_map(slot_name(REFERRAL_REVENUE_SLOT)),
        StorageSlot::with_empty_map(slot_name(REFERRAL_RATE_SLOT)),
        StorageSlot::with_empty_value(slot_name(TOTAL_DOMAIN_COUNT_SLOT)),
        StorageSlot::with_empty_map(slot_name(PROTOCOL_REVENUE_SLOT)),
        StorageSlot::with_empty_map(slot_name(REFERRAL_CLAIMED_SLOT)),
        StorageSlot::with_empty_map(slot_name(DOMAIN_EXPIRY_SLOT)),
        StorageSlot::with_empty_value(slot_name(ONE_YEAR_TIMESTAMP_SLOT)),
    ]
}
