import os
import json
import requests
from Cryptodome.Cipher import AES
from Cryptodome.Util.Padding import pad

# Configuration
AES_KEY = bytes.fromhex('0000000000000000000000000000000000000000000000000000000000000000') # 32 Bytes
PROXY_URL = 'http://localhost:80/proxy/1/abc/dummy/xyz/qwe/123'

# 1. Obfuscation XOR (rotate 0xAC)
def obfuscate(data: bytes) -> bytes:
    return bytes(b ^ 0xAC for b in data)

# 2. Custom Base32 Encoding (No Padding)
ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
def base32_encode(data: bytes) -> str:
    bits = 0
    value = 0
    output = []
    
    for b in data:
        value = (value << 8) | b
        bits += 8
        while bits >= 5:
            output.append(ALPHABET[(value >> (bits - 5)) & 31])
            bits -= 5
            
    if bits > 0:
        output.append(ALPHABET[(value << (5 - bits)) & 31])
        
    return "".join(output)

# 3. Encrypt Payload
def encrypt_payload(data: dict) -> str:
    iv = os.urandom(16)
    cipher = AES.new(AES_KEY, AES.MODE_CBC, iv)
    
    # Needs string encoding and PKCS7 padding
    json_bytes = json.dumps(data).encode('utf-8')
    padded_json = pad(json_bytes, AES.block_size)
    
    encrypted = cipher.encrypt(padded_json)
    
    # Prepend IV
    combined = iv + encrypted
    
    # Obfuscate
    obfuscated = obfuscate(combined)
    
    # Base32 Encode
    return base32_encode(obfuscated)

# ========================
# Usage Example
# ========================
if __name__ == "__main__":
    print("Preparing Secure Payload...")
    payload = {
        "email": "ikhwan@example.com",
        "action": "login"
    }
    
    encrypted_body = encrypt_payload(payload)
    print(f"Encrypted Output: {encrypted_body}\n")
    
    print("Sending to Halimun Proxy...")
    try:
        response = requests.post(PROXY_URL, data={'x': encrypted_body})
        print(f"Response Status: {response.status_code}")
        print(f"Response Body: {response.text}")
    except requests.exceptions.RequestException as e:
        print(f"Failed to reach proxy: {e}")
