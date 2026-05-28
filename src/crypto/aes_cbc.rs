use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn encrypt(data: &[u8], key: &[u8], iv: &[u8; 16]) -> Option<Vec<u8>> {
    if key.len() != 32 {
        return None;
    }
    let enc = Aes256CbcEnc::new_from_slices(key, iv).ok()?;
    let mut buf = data.to_vec();
    // Add dummy padding room
    let pos = buf.len();
    buf.resize(pos + 16, 0);
    let pt_len = enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pos).ok()?.len();
    buf.truncate(pt_len);

    // Prefix IV to ciphertext
    let mut final_bytes = iv.to_vec();
    final_bytes.extend_from_slice(&buf);
    Some(final_bytes)
}

pub fn decrypt(ciphertext_with_iv: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    if key.len() != 32 || ciphertext_with_iv.len() < 16 {
        return None;
    }
    let iv = &ciphertext_with_iv[..16];
    let ciphertext = &ciphertext_with_iv[16..];

    let dec = Aes256CbcDec::new_from_slices(key, iv).ok()?;
    let mut buf = ciphertext.to_vec();
    let pt_len = dec.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?.len();
    buf.truncate(pt_len);
    Some(buf)
}
