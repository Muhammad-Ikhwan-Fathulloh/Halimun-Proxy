//! AES-256-CBC dengan PKCS#7. Format: IV(16 byte) || ciphertext.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn encrypt(data: &[u8], key: &[u8], iv: &[u8; 16]) -> Option<Vec<u8>> {
    if key.len() != 32 {
        return None;
    }
    let enc = Aes256CbcEnc::new_from_slices(key, iv).ok()?;
    // Ruang tambahan untuk padding (maksimal satu blok)
    let pos = data.len();
    let mut buf = vec![0u8; pos + 16];
    buf[..pos].copy_from_slice(data);
    let ct_len = enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pos).ok()?.len();
    buf.truncate(ct_len);

    let mut out = Vec::with_capacity(16 + ct_len);
    out.extend_from_slice(iv);
    out.extend_from_slice(&buf);
    Some(out)
}

pub fn decrypt(ciphertext_with_iv: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    if key.len() != 32 || ciphertext_with_iv.len() < 32 || ciphertext_with_iv.len() % 16 != 0 {
        return None;
    }
    let (iv, ciphertext) = ciphertext_with_iv.split_at(16);
    let dec = Aes256CbcDec::new_from_slices(key, iv).ok()?;
    let mut buf = ciphertext.to_vec();
    let pt_len = dec.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?.len();
    buf.truncate(pt_len);
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_bad_input() {
        let key = [7u8; 32];
        let iv = [1u8; 16];
        let msg = br#"{"api_url":"http://example.com"}"#;
        let ct = encrypt(msg, &key, &iv).unwrap();
        assert_eq!(decrypt(&ct, &key).unwrap(), msg);
        assert!(decrypt(&ct[..20], &key).is_none());
        assert!(decrypt(&ct, &[8u8; 16]).is_none());
    }
}
