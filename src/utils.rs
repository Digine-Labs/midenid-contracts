
use std::{fs, path::Path, sync::Arc};

use anyhow::Error;
use miden_assembly::{ast::{Module, ModuleKind}, Assembler, DefaultSourceManager, Library, LibraryPath};
use miden_client::{account::{AccountBuilder, AccountType, StorageMap, StorageSlot}, crypto::SecretKey, testing::NoteBuilder};
use miden_crypto::{Felt, Word};
use miden_objects::account::{AccountComponent, AccountStorageMode, Account, AccountId};
use miden_objects::note::{NoteType};
use miden_lib::{account::{auth, wallets::BasicWallet}, transaction::TransactionKernel};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub fn naming_storage() -> Vec<StorageSlot> {
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

mod paths {
    pub const NAMING_ACCOUNT: &str = "./masm/accounts/naming.masm";
}

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

pub fn get_naming_account_code() -> String {
    fs::read_to_string(Path::new(paths::NAMING_ACCOUNT)).unwrap()
}



pub fn create_account() -> anyhow::Result<Account> {
    let mut rng = ChaCha20Rng::from_os_rng();
    let key_pair = SecretKey::with_rng(&mut rng);
    let (account, seed) = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(auth::AuthRpoFalcon512::new(key_pair.public_key().clone()))
        .with_component(BasicWallet).build()?;

    Ok(account)
}

pub fn create_naming_account() -> Account {
    let storage_slots = naming_storage();
    let account_code = get_naming_account_code();

    let account_component = AccountComponent::compile(
        account_code.clone(), 
        TransactionKernel::assembler().with_debug_mode(true), 
        storage_slots
    ).unwrap().with_supports_all_types();

    let account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_auth_component(auth::NoAuth)
        .with_component(account_component)
        .storage_mode(AccountStorageMode::Public)
        .build_existing().unwrap();
    return account;
}



pub fn create_library(
    account_code: String,
    library_path: &str,
) -> Result<Library, Box<dyn std::error::Error>> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new(library_path)?,
        account_code,
        &source_manager,
    )?;
    let library = assembler.clone().assemble_library([module])?;
    Ok(library)
}

pub fn create_naming_library() -> Result<Library, Box<dyn std::error::Error>> {
    let account_code = get_naming_account_code();
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new("miden_name::naming")?,
        account_code,
        &source_manager,
    )?;
    let library = assembler.clone().assemble_library([module])?;
    Ok(library)
}

// Helper function to encode a single character to its numeric representation
pub fn encode_char(chr: char) -> Option<u8> {
    match chr {
        'a' => Some(1), 'b' => Some(2), 'c' => Some(3), 'd' => Some(4),
        'e' => Some(5), 'f' => Some(6), 'g' => Some(7), 'h' => Some(8),
        'i' => Some(9), 'j' => Some(10), 'k' => Some(11), 'l' => Some(12),
        'm' => Some(13), 'n' => Some(14), 'o' => Some(15), 'p' => Some(16),
        'q' => Some(17), 'r' => Some(18), 's' => Some(19), 't' => Some(20),
        'u' => Some(21), 'v' => Some(22), 'w' => Some(23), 'x' => Some(24),
        'y' => Some(25), 'z' => Some(26),
        '0' => Some(27), '1' => Some(28), '2' => Some(29), '3' => Some(30),
        '4' => Some(31), '5' => Some(32), '6' => Some(33), '7' => Some(34),
        '8' => Some(35), '9' => Some(36),
        _ => None,
    }
}

// Helper function to decode a numeric value back to a character
pub fn decode_char(encoded: u8) -> Option<char> {
    match encoded {
        1 => Some('a'), 2 => Some('b'), 3 => Some('c'), 4 => Some('d'),
        5 => Some('e'), 6 => Some('f'), 7 => Some('g'), 8 => Some('h'),
        9 => Some('i'), 10 => Some('j'), 11 => Some('k'), 12 => Some('l'),
        13 => Some('m'), 14 => Some('n'), 15 => Some('o'), 16 => Some('p'),
        17 => Some('q'), 18 => Some('r'), 19 => Some('s'), 20 => Some('t'),
        21 => Some('u'), 22 => Some('v'), 23 => Some('w'), 24 => Some('x'),
        25 => Some('y'), 26 => Some('z'),
        27 => Some('0'), 28 => Some('1'), 29 => Some('2'), 30 => Some('3'),
        31 => Some('4'), 32 => Some('5'), 33 => Some('6'), 34 => Some('7'),
        35 => Some('8'), 36 => Some('9'),
        _ => None,
    }
}

// Name encoding decoding
// 7 bits per felt
// Total 4 felts
// Felts in word must not be reversed in storage
// So we have to reverse here
// [P4, P3, P2, P1] -> on MASM [P1, P2, P3, P4]
pub fn encode_domain(domain: String) -> Word {
    // Validate length: must be > 0 and <= 20
    let len = domain.len();
    assert!(len > 0, "Domain name must have at least 1 character");
    assert!(len <= 20, "Domain name must be at most 20 characters");

    // Encode each character and store in a vector
    let mut encoded_chars: Vec<u8> = Vec::new();
    for c in domain.chars() {
        let char_code = encode_char(c)
            .expect(&format!("Invalid character '{}' in domain name", c));
        encoded_chars.push(char_code);
    }

    // Pack characters into Felts (7 characters per Felt, 8 bits each)
    // First 7 characters go into felt3, next 7 into felt2, next 6 into felt1
    let mut felt1: u64 = 0;
    let mut felt2: u64 = 0;
    let mut felt3: u64 = 0;

    for (i, &char_code) in encoded_chars.iter().enumerate() {
        let bit_shift = (i % 7) * 8;

        if i < 7 {
            // First 7 characters go into felt3
            felt3 |= (char_code as u64) << bit_shift;
        } else if i < 14 {
            // Next 7 characters go into felt2
            felt2 |= (char_code as u64) << bit_shift;
        } else {
            // Remaining characters go into felt1
            felt1 |= (char_code as u64) << bit_shift;
        }
    }

    // Format: [felt1, felt2, felt3, length]
    // This is reversed for MASM storage (becomes [length, felt3, felt2, felt1] on stack)
    Word::new([
        Felt::new(felt1),
        Felt::new(felt2),
        Felt::new(felt3),
        Felt::new(len as u64),
    ])
}

pub fn decode_domain(encoded_domain: Word) -> String {
    let felts = encoded_domain.to_vec();

    // Extract length from the 4th felt
    let length = felts[3].as_int() as usize;

    // Extract the three data felts
    let felt1 = felts[0].as_int();
    let felt2 = felts[1].as_int();
    let felt3 = felts[2].as_int();

    let mut decoded_chars: Vec<char> = Vec::new();

    // Decode characters from each felt (7 characters per felt, 8 bits each)
    for i in 0..length {
        let char_code = if i < 7 {
            // First 7 characters from felt3
            ((felt3 >> (i * 8)) & 0xFF) as u8
        } else if i < 14 {
            // Next 7 characters from felt2
            ((felt2 >> ((i - 7) * 8)) & 0xFF) as u8
        } else {
            // Remaining characters from felt1
            ((felt1 >> ((i - 14) * 8)) & 0xFF) as u8
        };

        if let Some(chr) = decode_char(char_code) {
            decoded_chars.push(chr);
        } else {
            panic!("Invalid character code {} at position {}", char_code, i);
        }
    }

    decoded_chars.into_iter().collect()
}

