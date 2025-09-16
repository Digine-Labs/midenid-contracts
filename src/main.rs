use std::{fs, path::Path};

use midenid_contracts::common::create_public_immutable_contract;



use miden_client::{
    account::{StorageSlot},
    Word, rpc::Endpoint, transaction::TransactionRequestBuilder, Felt
};
use miden_objects::account::NetworkId;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    Ok(())
}
