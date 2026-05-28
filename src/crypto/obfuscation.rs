pub fn custom_obfuscate(data: &[u8], xor_key: u8) -> Vec<u8> {
    if data.is_empty() {
        return vec![];
    }

    let mut bytes = data.to_vec();

    // Step 1: XOR each byte
    for byte in bytes.iter_mut() {
        *byte ^= xor_key;
    }

    // Step 2: Reverse every 4 bytes
    for chunk in bytes.chunks_mut(4) {
        if chunk.len() == 4 {
            chunk.reverse();
        }
    }

    // Step 3: Rotate left by 1
    if !bytes.is_empty() {
        let first = bytes.remove(0);
        bytes.push(first);
    }

    bytes
}

pub fn custom_deobfuscate(data: &[u8], xor_key: u8) -> Vec<u8> {
    if data.is_empty() {
        return vec![];
    }

    let mut bytes = data.to_vec();

    // Step 1: Rotate right by 1
    if !bytes.is_empty() {
        let last = bytes.pop().unwrap();
        bytes.insert(0, last);
    }

    // Step 2: Reverse every 4 bytes
    for chunk in bytes.chunks_mut(4) {
        if chunk.len() == 4 {
            chunk.reverse();
        }
    }

    // Step 3: XOR each byte
    for byte in bytes.iter_mut() {
        *byte ^= xor_key;
    }

    bytes
}
