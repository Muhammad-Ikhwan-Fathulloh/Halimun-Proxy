//! Base32 codec dengan alfabet yang bisa dikustomisasi.
//!
//! Tabel lookup 256-entri dibangun sekali (O(1) per karakter), sehingga
//! decode tidak lagi O(alfabet) per karakter dan tidak membuat string bit.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct Base32Error(pub String);

impl fmt::Display for Base32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Base32Error {}

const NOT_IN_ALPHABET: u8 = 0xFF;

#[derive(Clone)]
pub struct Base32 {
    alphabet: [u8; 32],
    decode_table: [u8; 256],
}

impl Base32 {
    /// Alfabet harus tepat 32 karakter ASCII tercetak yang unik.
    pub fn new(alphabet: &str) -> Result<Self, Base32Error> {
        let bytes = alphabet.as_bytes();
        if bytes.len() != 32 || !bytes.iter().all(|b| b.is_ascii_graphic()) {
            return Err(Base32Error(
                "base32_alphabet harus 32 karakter ASCII yang tercetak".into(),
            ));
        }
        let mut alpha = [0u8; 32];
        let mut table = [NOT_IN_ALPHABET; 256];
        for (i, &b) in bytes.iter().enumerate() {
            if table[b as usize] != NOT_IN_ALPHABET {
                return Err(Base32Error(
                    "base32_alphabet mengandung karakter ganda".into(),
                ));
            }
            alpha[i] = b;
            table[b as usize] = i as u8;
        }
        Ok(Self {
            alphabet: alpha,
            decode_table: table,
        })
    }

    /// Encode tanpa padding. O(n) dengan akumulator bit.
    pub fn encode(&self, data: &[u8]) -> String {
        let mut out = String::with_capacity((data.len() * 8).div_ceil(5));
        let mut buffer: u32 = 0;
        let mut bits: u32 = 0;
        for &byte in data {
            buffer = (buffer << 8) | byte as u32;
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                out.push(self.alphabet[((buffer >> bits) & 31) as usize] as char);
            }
            buffer &= (1 << bits) - 1;
        }
        if bits > 0 {
            out.push(self.alphabet[((buffer << (5 - bits)) & 31) as usize] as char);
        }
        out
    }

    /// Decode. Karakter di luar alfabet (termasuk '=') diabaikan.
    /// Bit sisa kurang dari 8 dibuang. O(n).
    pub fn decode(&self, encoded: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(encoded.len() * 5 / 8);
        let mut buffer: u32 = 0;
        let mut bits: u32 = 0;
        for c in encoded.bytes() {
            let v = self.decode_table[c.to_ascii_uppercase() as usize];
            if v == NOT_IN_ALPHABET {
                continue;
            }
            buffer = ((buffer << 5) | v as u32) & 0xFFFF;
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ALPHA: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

    #[test]
    fn roundtrip() {
        let b = Base32::new(ALPHA).unwrap();
        for len in 0..40usize {
            let data: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37)).collect();
            assert_eq!(b.decode(&b.encode(&data)), data, "len={len}");
        }
    }

    #[test]
    fn rfc4648_vector() {
        let b = Base32::new(ALPHA).unwrap();
        assert_eq!(b.encode(b"foobar"), "MZXW6YTBOI");
        assert_eq!(b.decode("MZXW6YTBOI"), b"foobar");
    }

    #[test]
    fn rejects_bad_alphabet() {
        assert!(Base32::new("SHORT").is_err());
        assert!(Base32::new("AACDEFGHIJKLMNOPQRSTUVWXYZ234567").is_err());
    }
}
