mod utils;

use miden_crypto::{Felt, Word};
use utils::{encode_domain, decode_domain};

#[test]
fn encode_letter() {
    let letter = "a".to_string();

    let encoded: Word = encode_domain(letter);
    let felts: Vec<Felt> = encoded.to_vec();

    assert_eq!(felts[0], Felt::new(0));
    assert_eq!(felts[1], Felt::new(0));
    assert_eq!(felts[2], Felt::new(1));
    assert_eq!(felts[3], Felt::new(1));
}

#[test]
fn encode_letters() {
    let domain = "alice".to_string();

    let encoded: Word = encode_domain(domain);
    let felts: Vec<Felt> = encoded.to_vec();

    println!("0x{:x}", felts[2].as_int());

    assert_eq!(felts[0], Felt::new(0));
    assert_eq!(felts[1], Felt::new(0));
    assert_eq!(felts[2], Felt::new(0x503090c01));
    assert_eq!(felts[3], Felt::new(5));
}

#[test]
fn decode_letters() {
    let encoded: u64 = 0x503090c01;

    let encoded_word: Word = Word::new([Felt::new(0), Felt::new(0), Felt::new(encoded), Felt::new(5_u64)]);

    let decoded_domain: String = decode_domain(encoded_word);
    assert_eq!(decoded_domain, "alice");
}

#[test]
fn encode_multiple_felts() {
    let domain = "aliceandbobandjoe".to_string();
    let encoded = encode_domain(domain);

    let felts = encoded.to_vec();
    
    println!("0x{:x}", felts[0].as_int());
    println!("0x{:x}", felts[1].as_int());
    println!("0x{:x}", felts[2].as_int());
    println!("0x{:x}", felts[3].as_int());

    assert_eq!(felts[0], Felt::new(0x050f0a)); // joe
    assert_eq!(felts[1], Felt::new(0x40e01020f0204)); // dboband
    assert_eq!(felts[2], Felt::new(0xe010503090c01)); // alicean
    assert_eq!(felts[3], Felt::new(17));
}

#[test]
fn decode_multiple_felts() {
    let encoded_word: Word = Word::new([Felt::new(0x50f0a), Felt::new(0x40e01020f0204), Felt::new(0xe010503090c01), Felt::new(17_u64)]);

    let decoded_domain = decode_domain(encoded_word);

    assert_eq!(decoded_domain, "aliceandbobandjoe");
}