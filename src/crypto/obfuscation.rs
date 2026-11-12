//! Obfuskasi ringan (BUKAN kriptografi): XOR, balik tiap 4 byte, rotasi 1 byte.
//! Keamanan sebenarnya berasal dari AES-256-CBC + HMAC-SHA256.

pub fn custom_obfuscate(data: &[u8], xor_key: u8) -> Vec<u8> {
    let mut bytes = data.to_vec();
    for byte in bytes.iter_mut() {
        *byte ^= xor_key;
    }
    for chunk in bytes.chunks_exact_mut(4) {
        chunk.reverse();
    }
    if !bytes.is_empty() {
        bytes.rotate_left(1);
    }
    bytes
}

pub fn custom_deobfuscate(data: &[u8], xor_key: u8) -> Vec<u8> {
    let mut bytes = data.to_vec();
    if !bytes.is_empty() {
        bytes.rotate_right(1);
    }
    for chunk in bytes.chunks_exact_mut(4) {
        chunk.reverse();
    }
    for byte in bytes.iter_mut() {
        *byte ^= xor_key;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for len in 0..33u8 {
            let data: Vec<u8> = (0..len).collect();
            let obf = custom_obfuscate(&data, 172);
            assert_eq!(custom_deobfuscate(&obf, 172), data);
        }
    }
}
