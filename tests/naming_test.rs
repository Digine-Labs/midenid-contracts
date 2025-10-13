use core::slice;
use miden_client::{note::{NoteExecutionMode, NoteTag}, testing::NoteBuilder, transaction::{AccountInterface, OutputNote}};
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

    let initialize_input_note = create_naming_initialize_note(owner_account, treasury_account, naming_account.clone()).await.unwrap();

    builder.add_note(OutputNote::Full(initialize_input_note.clone()));

    let mock_chain = builder.build()?;

    let tx_inputs = mock_chain.get_transaction_inputs(naming_account.clone(), None, &[initialize_input_note.id()], &[])?;

    let tx_context = TransactionContextBuilder::new(naming_account.clone())
        .account_seed(None)
        .tx_inputs(tx_inputs)
        .build()?;

    let _exec_tx = tx_context.execute().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "reason"]
async fn test_naming_initialize() -> anyhow::Result<()>{
    let owner_account = create_account()?;
    let treasury_account = create_account()?;
    let naming_account = create_naming_account();


    // Reverse ordered. TODO: create utility function to reverse order words.
    let initialize_inputs = vec![
        Felt::new(treasury_account.id().suffix().into()), 
        Felt::new(treasury_account.id().prefix().into()),
        Felt::new(owner_account.id().suffix().into()),
        Felt::new(owner_account.id().prefix().into())
    ];

    let naming_library = create_naming_library().unwrap();
    
    let note= NoteBuilder::new(owner_account.id(), ChaCha20Rng::from_os_rng())
        .code(get_note_code("initialize".to_string()))
        .note_type(NoteType::Public)
        .tag(NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Network).unwrap().into())
        .add_assets(vec![])
        .dynamically_linked_libraries(vec![naming_library])
        .note_inputs(initialize_inputs.clone()).unwrap().build()?;

    let mut mock_chain = MockChainBuilder::with_accounts([naming_account.clone(), owner_account.clone(), treasury_account.clone()]).unwrap()
    .build()?;
    mock_chain.prove_next_block()?;

    let sender_account_interface = AccountInterface::from(&owner_account);

    let send_note_tx_script = sender_account_interface.build_send_notes_script(
        slice::from_ref(&note.clone().into()), 
        Some(10u16), 
        true
    )?;

    let _executed_tx = mock_chain
        .build_tx_context(owner_account.id(), &[], &[])
        .expect("failed to build tx context")
        .tx_script(send_note_tx_script)
        .build()?
        .execute()
        .await?;
    
    Ok(())
}