//! Codec gabungan: AES-CBC -> obfuscation XOR/balik -> Base32 (dan kebalikannya).
//! Dibangun sekali saat startup, sehingga tidak ada hex-decode / parsing kunci per request.

use crate::config::EncryptionConfig;
use crate::crypto::{aes_cbc, base32::Base32, obfuscation};
use rand::RngCore;

#[derive(Clone)]
pub struct Codec {
    aes_key: [u8; 32],
    xor_key: u8,
    base32: Base32,
}

impl Codec {
    pub fn new(
        aes_key: [u8; 32],
        xor_key: u8,
        alphabet: &str,
    ) -> Result<Self, crate::crypto::base32::Base32Error> {
        Ok(Self {
            aes_key,
            xor_key,
            base32: Base32::new(alphabet)?,
        })
    }

    pub fn from_config(cfg: &EncryptionConfig, aes_key: [u8; 32]) -> Result<Self, String> {
        Self::new(aes_key, cfg.xor_key, &cfg.base32_alphabet).map_err(|e| e.to_string())
    }

    /// Dekripsi satu segmen/body: Base32 -> deobfuscate -> AES-CBC decrypt.
    /// Mengembalikan None jika padding/format tidak valid.
    pub fn open(&self, encoded: &str) -> Option<Vec<u8>> {
        let bytes = self.base32.decode(encoded);
        if bytes.is_empty() {
            return None;
        }
        let deob = obfuscation::custom_deobfuscate(&bytes, self.xor_key);
        aes_cbc::decrypt(&deob, &self.aes_key)
    }

    /// Enkripsi plaintext (dipakai untuk pengujian dan contoh).
    pub fn seal(&self, plaintext: &[u8]) -> Option<String> {
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);
        let ct = aes_cbc::encrypt(plaintext, &self.aes_key, &iv)?;
        Some(
            self.base32
                .encode(&obfuscation::custom_obfuscate(&ct, self.xor_key)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_then_open() {
        let c = Codec::new([9u8; 32], 172, "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567").unwrap();
        let sealed = c.seal(b"hello halimun").unwrap();
        assert_eq!(c.open(&sealed).unwrap(), b"hello halimun");
        assert!(c.open("AAAA").is_none());
    }
}
