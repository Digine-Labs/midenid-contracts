use miden_client::{note::{NoteExecutionMode, NoteTag}, testing::NoteBuilder};
use miden_crypto::Felt;
use miden_testing::{MockChainBuilder};
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use miden_objects::note::{NoteType};
use crate::utils::{create_account, create_naming_account, create_naming_library, get_note_code};
mod utils;



// Develop test like that
// https://github.com/0xMiden/miden-base/blob/719ff03d1482e6ce2ad4e986f59ec7b9a8ddf962/crates/miden-testing/src/kernel_tests/tx/test_fpi.rs#L515

#[tokio::test]
async fn test_naming_initialize() -> anyhow::Result<()>{
    let owner_account = create_account()?;
    let treasury_account = create_account()?;
    let naming_account = create_naming_account();

    let mut mock_chain = MockChainBuilder::with_accounts([naming_account.clone()]).unwrap().build()?;
    mock_chain.prove_next_block()?;

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

    let tx_context = mock_chain.build_tx_context(owner_account.id(), &[note.id()], &[note])
        .expect("failed to build tx")
        .build()?;

    tx_context.execute();
    
    Ok(())
}