const crypto = require('crypto');

// Configuration
const AES_KEY = Buffer.from('0000000000000000000000000000000000000000000000000000000000000000', 'hex'); // 32 Bytes
const PROXY_URL = 'http://localhost:80/proxy/1/abc/dummy/xyz/qwe/123';

// 1. Obfuscation XOR (rotate 0xAC)
function obfuscate(buffer) {
    for (let i = 0; i < buffer.length; i++) {
        buffer[i] = buffer[i] ^ 0xAC;
    }
    return buffer;
}

// 2. Custom Base32 Encoding (No Padding, custom alphabet)
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
function base32Encode(buffer) {
    let bits = 0;
    let value = 0;
    let output = '';

    for (let i = 0; i < buffer.length; i++) {
        value = (value << 8) | buffer[i];
        bits += 8;
        while (bits >= 5) {
            output += ALPHABET[(value >>> (bits - 5)) & 31];
            bits -= 5;
        }
    }
    if (bits > 0) {
        output += ALPHABET[(value << (5 - bits)) & 31];
    }
    return output;
}

// 3. Encrypt Payload
function encryptPayload(data) {
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv('aes-256-cbc', AES_KEY, iv);
    
    // Encrypt
    let encrypted = cipher.update(JSON.stringify(data));
    encrypted = Buffer.concat([encrypted, cipher.final()]);
    
    // Prepend IV
    let combined = Buffer.concat([iv, encrypted]);
    
    // Obfuscate XOR
    combined = obfuscate(combined);
    
    // Return custom Base32
    return base32Encode(combined);
}

// ========================
// Usage Example
// ========================
async function sendSecureRequest() {
    console.log("Preparing Secure Payload...");
    const payload = {
        email: "ikhwan@example.com",
        action: "login",
        timestamp: Date.now()
    };
    
    const encryptedBodyBase32 = encryptPayload(payload);
    console.log("Encrypted Output:", encryptedBodyBase32);

    console.log("\nSending to Halimun Proxy...");
    const params = new URLSearchParams();
    params.append('x', encryptedBodyBase32);

    try {
        const response = await fetch(PROXY_URL, {
            method: 'POST',
            body: params,
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' }
        });
        
        const responseData = await response.text();
        console.log("Response Status:", response.status);
        console.log("Response Body:", responseData);
    } catch (e) {
        console.error("Failed to reach proxy:", e.message);
    }
}

sendSecureRequest();
