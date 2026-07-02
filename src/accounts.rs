use miden_client::{
    Client,
    account::{Account, AccountBuilder, AccountId, AccountType},
    auth::{AuthScheme, AuthSecretKey, NoAuth},
    keystore::{FilesystemKeyStore, Keystore},
};
use miden_protocol::account::{AccountComponent, AccountComponentMetadata};
use miden_standards::{
    account::auth::{AuthNetworkAccount, AuthSingleSig},
    account::wallets::BasicWallet,
};
use rand::RngCore;
use std::{collections::BTreeSet, sync::Arc};

use crate::notes::{compile_init_on_chain_tx_script, compile_note_script_with_lib, naming_library};
use crate::storage::naming_storage;

/// Note scripts a deployed network naming account is allowed to auto-consume. Only notes whose
/// `call.naming::*` target exists in `naming.masm` are included (excludes the referral notes and
/// extend/clear notes, whose procedures live in `naming_discount.masm`).
const NETWORK_ALLOWED_NOTES: &[&str] = &[
    "initialize_naming",
    "set_all_prices",
    "set_all_prices_testnet",
    "register_name",
    "activate_domain",
    "transfer_domain",
    "transfer_ownership",
    "claim_protocol_revenue",
    "withdraw_assets",
];

pub async fn create_deployer_account(
    client: &mut Client<FilesystemKeyStore>,
    keystore: &mut Arc<FilesystemKeyStore>,
) -> anyhow::Result<Account> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = AuthSecretKey::new_falcon512_poseidon2();

    // Build the account
    let deployer_account = AccountBuilder::new(init_seed)
        // 0.15: storage mode is merged into AccountType (Public = on-chain state).
        .account_type(AccountType::Public)
        .with_auth_component(AuthSingleSig::new(
            key_pair.public_key().to_commitment(),
            AuthScheme::Falcon512Poseidon2,
        ))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    // Add the account to the client
    client.add_account(&deployer_account, false).await?;

    // Add the key pair to the keystore
    keystore.add_key(&key_pair, deployer_account.id()).await?;

    println!(
        "Deployer account ID: {:?}",
        deployer_account.id().to_string()
    );
    Ok(deployer_account)
}

pub async fn create_naming_account(
    client: &mut Client<FilesystemKeyStore>,
    is_network: bool,
) -> anyhow::Result<Account> {
    // Assemble the naming contract once and reuse it for the account component and every
    // allowlisted note-script root.
    let library = naming_library(is_network)?;

    let account_component = AccountComponent::new(
        library.clone(),
        naming_storage(),
        AccountComponentMetadata::new("midenid-naming"),
    )?;

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    // 0.15: a network account is defined by the standardized NetworkAccountNoteAllowlist slot,
    // provided by the AuthNetworkAccount auth component. It pins the set of note-script roots the
    // network may auto-consume against this account. Non-network deployments use NoAuth.
    let auth_component: AccountComponent = if is_network {
        let mut allowed_roots = BTreeSet::new();
        for name in NETWORK_ALLOWED_NOTES {
            allowed_roots.insert(compile_note_script_with_lib(name, &library)?.root());
        }
        // The deploy's init_on_chain self-transaction must also be allowlisted, otherwise the
        // network account rejects it (empty tx-script allowlist blocks all tx scripts).
        let mut allowed_tx_scripts = BTreeSet::new();
        allowed_tx_scripts.insert(compile_init_on_chain_tx_script(is_network)?.root());
        AuthNetworkAccount::with_allowed_notes(allowed_roots)?
            .with_allowed_tx_scripts(allowed_tx_scripts)
            .into()
    } else {
        NoAuth.into()
    };

    let account = AccountBuilder::new(seed)
        .account_type(AccountType::Public)
        .with_auth_component(auth_component)
        .with_component(account_component.clone())
        .build()?;

    client.add_account(&account, false).await.unwrap();

    println!("Naming account ID: {:?}", account.id().to_string());
    Ok(account)
}

pub async fn safe_account_import(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
) -> anyhow::Result<()> {
    if client.get_account(account_id).await?.is_none() {
        if let Err(e) = client.import_account_by_id(account_id).await {
            eprintln!("Warning: Failed to import account: {:?}", e);
        }
    }
    Ok(())
}
