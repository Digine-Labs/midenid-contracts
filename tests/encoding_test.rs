use miden_crypto::{Felt, Word};
use midenname_contracts::domain::{decode_domain, encode_domain};

#[test]
fn encode_letter() {
    let letter = "a".to_string();

    let encoded: Word = encode_domain(letter);
    let felts: Vec<Felt> = encoded.to_vec();

    assert_eq!(felts[0], Felt::new(0).unwrap());
    assert_eq!(felts[1], Felt::new(0).unwrap());
    assert_eq!(felts[2], Felt::new(1).unwrap());
    assert_eq!(felts[3], Felt::new(1).unwrap());
}

#[test]
fn encode_letters() {
    let domain = "alice".to_string();

    let encoded: Word = encode_domain(domain);
    let felts: Vec<Felt> = encoded.to_vec();

    assert_eq!(felts[0], Felt::new(0).unwrap());
    assert_eq!(felts[1], Felt::new(0).unwrap());
    assert_eq!(felts[2], Felt::new(0x503090c01).unwrap());
    assert_eq!(felts[3], Felt::new(5).unwrap());
}

#[test]
fn decode_letters() {
    let encoded: u64 = 0x503090c01;

    let encoded_word: Word = Word::new([
        Felt::new(0).unwrap(),
        Felt::new(0).unwrap(),
        Felt::new(encoded).unwrap(),
        Felt::new(5_u64).unwrap(),
    ]);

    let decoded_domain: String = decode_domain(encoded_word);
    assert_eq!(decoded_domain, "alice");
}

#[test]
fn encode_multiple_felts() {
    let domain = "aliceandbobandjoe".to_string();
    let encoded = encode_domain(domain);

    let felts = encoded.to_vec();

    assert_eq!(felts[0], Felt::new(0x050f0a).unwrap()); // joe
    assert_eq!(felts[1], Felt::new(0x40e01020f0204).unwrap()); // dboband
    assert_eq!(felts[2], Felt::new(0xe010503090c01).unwrap()); // alicean
    assert_eq!(felts[3], Felt::new(17).unwrap());
}

#[test]
fn decode_multiple_felts() {
    let encoded_word: Word = Word::new([
        Felt::new(0x50f0a).unwrap(),
        Felt::new(0x40e01020f0204).unwrap(),
        Felt::new(0xe010503090c01).unwrap(),
        Felt::new(17_u64).unwrap(),
    ]);

    let decoded_domain = decode_domain(encoded_word);

    assert_eq!(decoded_domain, "aliceandbobandjoe");
}
