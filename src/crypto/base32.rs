use std::fmt;

#[derive(Debug)]
pub struct Base32Error(String);

impl fmt::Display for Base32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Base32Error {}

pub fn encode(data: &[u8], alphabet: &str) -> String {
    let alphabet_chars: Vec<char> = alphabet.chars().collect();
    let alphabet_len = alphabet_chars.len();

    let mut bits = String::new();
    for byte in data {
        bits.push_str(&format!("{:08b}", byte));
    }

    let mut encoded = String::new();
    let mut i = 0;
    while i < bits.len() {
        let mut chunk = String::from(&bits[i..std::cmp::min(i + 5, bits.len())]);
        if chunk.len() < 5 {
            chunk.push_str(&"0".repeat(5 - chunk.len()));
        }
        let idx = usize::from_str_radix(&chunk, 2).unwrap_or(0);
        encoded.push(alphabet_chars[idx % alphabet_len]);
        i += 5;
    }

    encoded
}

pub fn decode(encoded: &str, alphabet: &str) -> Result<Vec<u8>, Base32Error> {
    let encoded_upper = encoded.to_uppercase();
    let mut bits = String::new();

    for c in encoded_upper.chars() {
        if let Some(pos) = alphabet.find(c) {
            bits.push_str(&format!("{:05b}", pos));
        }
        // Exclude characters not in alphabet silently as per PHP reference
    }

    let mut decoded = Vec::new();
    let mut i = 0;
    while i + 8 <= bits.len() {
        let byte_str = &bits[i..i + 8];
        let byte = u8::from_str_radix(byte_str, 2).map_err(|e| Base32Error(e.to_string()))?;
        decoded.push(byte);
        i += 8;
    }

    Ok(decoded)
}
