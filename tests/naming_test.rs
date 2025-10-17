use core::slice;
use anyhow::Ok;
use miden_client::{asset::FungibleAsset, note::{NoteExecutionMode, NoteTag}, testing::{account_id::ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1, NoteBuilder}, transaction::{AccountInterface, OutputNote}};
use miden_crypto::Felt;
use miden_testing::{Auth, MockChain, MockChainBuilder, TransactionContextBuilder};
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use miden_objects::note::{NoteType};
use midenid_contracts::utils::{create_account, create_naming_account, create_naming_library};
use midenid_contracts::notes::{get_note_code, create_naming_initialize_note};




// Develop test like that
// https://github.com/0xMiden/miden-base/blob/719ff03d1482e6ce2ad4e986f59ec7b9a8ddf962/crates/miden-testing/src/kernel_tests/tx/test_fpi.rs#L515

#[tokio::test]
async fn test_naming_init() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();

    let owner_account = builder.add_existing_wallet(Auth::BasicAuth)?;
    let treasury_account= builder.add_existing_wallet(Auth::BasicAuth)?;

    let naming_account = create_naming_account();

    let initialize_input_note = create_naming_initialize_note(owner_account.clone(), treasury_account.clone(), naming_account.clone()).await.unwrap();

    builder.add_note(OutputNote::Full(initialize_input_note.clone()));

    let mut mock_chain = builder.build()?;

    let tx_inputs = mock_chain.get_transaction_inputs(naming_account.clone(), None, &[initialize_input_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(naming_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    let updated_naming_account = mock_chain.add_pending_executed_transaction(&_exec_tx)?;
    
    let init_flag = updated_naming_account.storage().get_item(0)?.get(0).unwrap().as_int();
    let owner_slot = updated_naming_account.storage().get_item(1)?;
    let trasury_slot = updated_naming_account.storage().get_item(2)?;

    assert_eq!(init_flag, 1);
    assert_eq!(owner_account.id().prefix().as_u64(), owner_slot.get(1).unwrap().as_int());
    assert_eq!(owner_account.id().suffix().as_int(), owner_slot.get(0).unwrap().as_int());
    assert_eq!(treasury_account.id().prefix().as_u64(), trasury_slot.get(1).unwrap().as_int());
    assert_eq!(treasury_account.id().suffix().as_int(), trasury_slot.get(0).unwrap().as_int());
    Ok(())
}

#[tokio::test]
async fn test_naming_register() -> anyhow::Result<()> {
    let mut builder = MockChain::builder();
    let fungible_asset = FungibleAsset::new(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1.try_into().unwrap(), 100000).unwrap();

    // TODO
    Ok(())
}