use miden_client::account::Account;
use miden_objects::{Felt, FieldElement, Word, note::NoteInputs};

/// Utilities for encoding/decoding Miden data types
pub struct EncodingUtils;

impl EncodingUtils {
    /// Encodes a name string into a Miden Word.
    ///
    /// The encoding scheme:
    /// - Felt[0]: Length of the name (0-20 characters)
    /// - Felt[1-3]: Name bytes packed 7 characters per felt (56 bits used per felt)
    ///
    /// # Arguments
    ///
    /// * `name` - Name to encode (max 20 characters)
    ///
    /// # Returns
    ///
    /// Word containing encoded name
    ///
    /// # Panics
    ///
    /// Panics if name exceeds 20 characters
    pub fn encode_name_to_word(name: &str) -> Word {
        assert!(name.len() <= 20, "Name must not exceed 20 characters");

        let bytes = name.as_bytes();
        let mut felts = [Felt::ZERO; 4];

        // Felt[0]: Store name length
        felts[0] = Felt::new(bytes.len() as u64);

        // Felt[1-3]: Pack 7 ASCII characters per felt (56 bits used)
        for (i, chunk) in bytes.chunks(7).enumerate() {
            if i >= 3 {
                break;
            }

            let mut value = 0u64;
            for (j, &byte) in chunk.iter().enumerate() {
                value |= (byte as u64) << (j * 8);
            }
            felts[i + 1] = Felt::new(value);
        }

        Word::new(felts)
    }

    /// Decodes a Miden Word back to a name string.
    ///
    /// Reverses the encoding done by `encode_name_to_word`.
    ///
    /// # Arguments
    ///
    /// * `word` - Word containing encoded name
    ///
    /// # Returns
    ///
    /// Decoded name string
    pub fn decode_name_word(word: &Word) -> String {
        let length = word.get(0).map(|f| f.as_int() as usize).unwrap_or(0);
        if length == 0 {
            return String::new();
        }

        let mut bytes = Vec::new();

        // Extract ASCII characters from felts 1-3
        for idx in 1..4 {
            if let Some(felt) = word.get(idx) {
                let mut value = felt.as_int();
                for _ in 0..7 {
                    if bytes.len() >= length {
                        break;
                    }
                    let byte = (value & 0xFF) as u8;
                    if byte == 0 {
                        break;
                    }
                    bytes.push(byte);
                    value >>= 8;
                }
            }
            if bytes.len() >= length {
                break;
            }
        }

        String::from_utf8(bytes).unwrap_or_default()
    }

    /// Encodes an account ID into a Word.
    ///
    /// Format: [suffix, prefix, 0, 0]
    ///
    /// # Arguments
    ///
    /// * `account` - Account to encode
    ///
    /// # Returns
    ///
    /// Word containing encoded account ID
    pub fn encode_account_to_word(account: &Account) -> Word {
        Word::new([
            Felt::new(account.id().suffix().as_int()),
            Felt::new(account.id().prefix().as_felt().as_int()),
            Felt::ZERO,
            Felt::ZERO,
        ])
    }

    /// Decodes an account ID from a Word.
    ///
    /// Expects format: [suffix, prefix, 0, 0]
    ///
    /// # Arguments
    ///
    /// * `word` - Word containing encoded account ID
    ///
    /// # Returns
    ///
    /// Tuple of (prefix, suffix)
    pub fn decode_account_word(word: &Word) -> (u64, u64) {
        let suffix = word.get(0).map(|felt| felt.as_int()).unwrap_or(0);
        let prefix = word.get(1).map(|felt| felt.as_int()).unwrap_or(0);
        (prefix, suffix)
    }

    /// Checks if a Word contains all zeros.
    ///
    /// # Arguments
    ///
    /// * `word` - Word to check
    ///
    /// # Returns
    ///
    /// `true` if all felts are zero, `false` otherwise
    pub fn is_zero_word(word: &Word) -> bool {
        (0..4).all(|idx| word.get(idx).map(|felt| felt.as_int()).unwrap_or(0) == 0)
    }

    /// Converts a Word to NoteInputs.
    ///
    /// Extracts all 4 felts from the word and creates NoteInputs.
    ///
    /// # Arguments
    ///
    /// * `word` - Word to convert
    ///
    /// # Returns
    ///
    /// NoteInputs containing the word's felts
    pub fn word_to_note_inputs(word: &Word) -> NoteInputs {
        let felts: Vec<Felt> = (0..4).map(|i| *word.get(i).unwrap()).collect();
        NoteInputs::new(felts).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_name() {
        let name = "alice";
        let word = EncodingUtils::encode_name_to_word(name);
        let decoded = EncodingUtils::decode_name_word(&word);
        assert_eq!(decoded, name);
    }

    #[test]
    fn test_encode_long_name() {
        let name = "verylongname12345678"; // 20 chars (max)
        let word = EncodingUtils::encode_name_to_word(name);
        let decoded = EncodingUtils::decode_name_word(&word);
        assert_eq!(decoded, name);
    }

    #[test]
    #[should_panic(expected = "Name must not exceed 20 characters")]
    fn test_encode_name_too_long() {
        let name = "thisnameiswaytooolong"; // 21 chars
        EncodingUtils::encode_name_to_word(name);
    }

    #[test]
    fn test_is_zero_word() {
        let zero_word = Word::new([Felt::ZERO; 4]);
        assert!(EncodingUtils::is_zero_word(&zero_word));

        let non_zero_word = Word::new([Felt::new(1), Felt::ZERO, Felt::ZERO, Felt::ZERO]);
        assert!(!EncodingUtils::is_zero_word(&non_zero_word));
    }
}
