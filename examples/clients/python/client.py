"""
Klien referensi Halimun (Python) — sudah diuji terhadap server.

Alur enkripsi (sama persis dengan src/crypto/codec.rs):
  JSON (+ body_sha256 jika ada body) -> tambah field "hmac"
        ->  AES-256-CBC (IV acak di depan)
        ->  obfuscate (XOR, balik tiap 4 byte, rotasi kiri 1 byte)
        ->  Base32 tanpa padding

Instalasi:
  pip install requests pycryptodome

Pemakaian (kunci diambil dari environment, JANGAN ditulis di kode):
  export HALIMUN_AES_KEY=...        # dari: halimun-proxy --keygen
  export HALIMUN_HMAC_KEY=...
  export HALIMUN_XOR_KEY=172
  export HALIMUN_PROXY_URL=http://localhost/proxy/1
  python client.py
"""

import hashlib
import hmac as hmac_lib
import json
import os
import secrets
import sys

import requests
try:  # pycryptodome (paket "pycryptodome" memakai modul Crypto)
    from Crypto.Cipher import AES
    from Crypto.Util.Padding import pad, unpad
except ImportError:  # pycryptodomex
    from Cryptodome.Cipher import AES
    from Cryptodome.Util.Padding import pad, unpad

ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
DEFAULT_XOR = 172


# ---------------------------------------------------------------- obfuscation
def obfuscate(data: bytes, xor_key: int) -> bytes:
    out = bytearray(b ^ xor_key for b in data)
    for i in range(0, len(out) - len(out) % 4, 4):  # balik tiap blok 4 byte penuh
        out[i:i + 4] = out[i:i + 4][::-1]
    if out:
        out = out[1:] + out[:1]  # rotasi kiri 1 byte
    return bytes(out)


def deobfuscate(data: bytes, xor_key: int) -> bytes:
    out = bytearray(data)
    if out:
        out = out[-1:] + out[:-1]  # rotasi kanan 1 byte
    for i in range(0, len(out) - len(out) % 4, 4):
        out[i:i + 4] = out[i:i + 4][::-1]
    return bytes(b ^ xor_key for b in out)


# --------------------------------------------------------------------- base32
def base32_encode(data: bytes, alphabet: str = ALPHABET) -> str:
    value, bits, out = 0, 0, []
    for b in data:
        value = ((value << 8) | b) & 0xFFFF
        bits += 8
        while bits >= 5:
            bits -= 5
            out.append(alphabet[(value >> bits) & 31])
    if bits > 0:
        out.append(alphabet[(value << (5 - bits)) & 31])
    return "".join(out)


def base32_decode(text: str, alphabet: str = ALPHABET) -> bytes:
    value, bits, out = 0, 0, bytearray()
    for c in text.upper():
        idx = alphabet.find(c)
        if idx < 0:
            continue
        value = ((value << 5) | idx) & 0xFFFF
        bits += 5
        if bits >= 8:
            bits -= 8
            out.append((value >> bits) & 0xFF)
    return bytes(out)


# ------------------------------------------------------------------ crypto
def seal(plaintext: bytes, aes_key: bytes, xor_key: int) -> str:
    iv = secrets.token_bytes(16)
    ct = AES.new(aes_key, AES.MODE_CBC, iv).encrypt(pad(plaintext, 16))
    return base32_encode(obfuscate(iv + ct, xor_key))


def open_sealed(encoded: str, aes_key: bytes, xor_key: int) -> bytes:
    raw = deobfuscate(base32_decode(encoded), xor_key)
    return unpad(AES.new(aes_key, AES.MODE_CBC, raw[:16]).decrypt(raw[16:]), 16)


def canonical(obj: dict) -> str:
    # Harus identik dengan serde_json pada server: kunci terurut, tanpa spasi.
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def sign_token(token: dict, hmac_key: bytes) -> str:
    token = {k: v for k, v in token.items() if k != "hmac"}
    token["hmac"] = hmac_lib.new(hmac_key, canonical(token).encode("utf-8"), hashlib.sha256).hexdigest()
    return json.dumps(token, ensure_ascii=False)


def decoy_segment(length: int = 16) -> str:
    return "".join(secrets.choice(ALPHABET) for _ in range(length))


# ------------------------------------------------------------------- client
class HalimunClient:
    def __init__(self, base_url: str, aes_key_hex: str, hmac_key_hex: str, xor_key: int = DEFAULT_XOR):
        self.base_url = base_url.rstrip("/")  # contoh: http://localhost/proxy/1
        self.aes_key = bytes.fromhex(aes_key_hex)
        self.hmac_key = bytes.fromhex(hmac_key_hex)
        self.xor_key = xor_key
        if len(self.aes_key) != 32 or len(self.hmac_key) < 32:
            raise ValueError("aes_key harus 32 byte dan hmac_key minimal 32 byte (hex)")

    def request(self, method: str, api_url: str, body: bytes = b"", headers=None,
                bypass: bool = False, ttl: int = 60, nonce: str | None = None):
        import time

        token = {
            "api_url": api_url,
            "api_header": headers or {},
            "method": method.upper(),
            "timestamp": int(time.time()),
            "expired": ttl,
            "nonce": nonce or secrets.token_hex(16),
            "bypass_url": bypass,
        }
        if body:
            # Mengikat body ke token yang ditandatangani (wajib jika ada body)
            token["body_sha256"] = hashlib.sha256(body).hexdigest()
        signed = sign_token(token, self.hmac_key)
        segment = seal(signed.encode("utf-8"), self.aes_key, self.xor_key)

        # Satu segmen berisi token, empat lainnya dekoy acak
        segments = [decoy_segment() for _ in range(4)]
        segments.insert(secrets.randbelow(5), segment)
        url = f"{self.base_url}/" + "/".join(segments)

        data = {}
        if body:
            data = {"x": seal(body, self.aes_key, self.xor_key)}
        return requests.post(url, data=data, timeout=30)


if __name__ == "__main__":
    base = os.environ.get("HALIMUN_PROXY_URL", "http://localhost/proxy/1")
    aes = os.environ.get("HALIMUN_AES_KEY")
    hm = os.environ.get("HALIMUN_HMAC_KEY")
    xor = int(os.environ.get("HALIMUN_XOR_KEY", DEFAULT_XOR))
    if not aes or not hm:
        sys.exit("Set HALIMUN_AES_KEY dan HALIMUN_HMAC_KEY terlebih dahulu (lihat README).")

    client = HalimunClient(base, aes, hm, xor)
    payload = json.dumps({"email": "user@example.com", "action": "login"}).encode()
    res = client.request(
        "POST",
        "https://httpbin.org/post",  # ganti dengan URL backend Anda
        body=payload,
        headers={"Content-Type": "application/json"},
    )
    print("Status:", res.status_code)
    print(res.text[:500])
