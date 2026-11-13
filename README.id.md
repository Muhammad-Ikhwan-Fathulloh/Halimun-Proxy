# Halimun Proxy

[![CI](https://github.com/Muhammad-Ikhwan-Fathulloh/Halimun-Proxy/actions/workflows/rust.yml/badge.svg)](https://github.com/Muhammad-Ikhwan-Fathulloh/Halimun-Proxy/actions)
[![Docker Hub](https://img.shields.io/docker/pulls/ikhwan17/halimun-proxy)](https://hub.docker.com/r/ikhwan17/halimun-proxy)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Halimun adalah reverse proxy terenkripsi yang ditulis dengan Rust. Klien membungkus tujuan, method, header, dan body ke dalam satu token terenkripsi. Halimun memvalidasi token, memblokir target internal, lalu meneruskan request ke backend Anda.

Baca dalam bahasa lain: [English](README.md) · [Bahasa Indonesia](README.id.md)

- **Dokumentasi lengkap:** [docs/PANDUAN.md](docs/PANDUAN.md)
- **Image:** [ikhwan17/halimun-proxy](https://hub.docker.com/r/ikhwan17/halimun-proxy) di Docker Hub

## Fitur

- Enkripsi AES-256-CBC untuk token dan body request, dengan lapisan obfuscation
- Tanda tangan HMAC-SHA256 atas JSON kanonik, dengan pengikatan hash body
- Perlindungan replay (nonce) dan pengecekan masa berlaku token
- Perlindungan SSRF: alamat privat dan loopback diblokir kecuali di-whitelist secara eksplisit
- Rate limit per IP, batas ukuran body dan respons, serta timeout ke backend
- Admin API dan dashboard web untuk log trafik, service, dan pembuatan kunci
- Endpoint metrik Prometheus
- Image runtime kecil (Alpine, user non-root)

## Mulai cepat (Docker)

1. Ambil image dan buat kunci:

   ```bash
   docker pull ikhwan17/halimun-proxy:latest
   docker run --rm ikhwan17/halimun-proxy:latest --keygen --format=env > .env
   ```

   PowerShell:

   ```powershell
   docker run --rm ikhwan17/halimun-proxy:latest --keygen --format=env | Out-File -Encoding ASCII .env
   ```

   Tambahkan `HALIMUN_ADMIN_KEY` (minimal 16 karakter) ke `.env`.

2. Salin konfigurasi contoh:

   ```bash
   cp config.example.yaml config.yaml
   ```

3. Buat akun login dashboard (basic auth Nginx):

   ```bash
   docker run --rm httpd:2.4-alpine htpasswd -Bbn admin "password-anda" > nginx/.htpasswd
   ```

4. Jalankan stack:

   ```bash
   docker compose pull
   docker compose up -d
   ```

5. Buka `http://localhost/dashboard/`, login dengan user Nginx, lalu masukkan admin key.

Build dari source, HTTPS, dan detail protokol dijelaskan di [docs/PANDUAN.md](docs/PANDUAN.md).

## Konfigurasi

Rahasia dibaca dari environment variable (`HALIMUN_AES_KEY`, `HALIMUN_HMAC_KEY`, `HALIMUN_XOR_KEY`, `HALIMUN_ADMIN_KEY`) dan menimpa nilai di `config.yaml`. Server menolak start jika kunci kosong, nol, atau masih berupa contoh.

Validasi konfigurasi tanpa menjalankan server:

```bash
halimun-proxy --config config.yaml --check-config
```

Referensi lengkap ada di [docs/PANDUAN.md, bagian 5](docs/PANDUAN.md#5-referensi-konfigurasi).

## Contoh klien

Klien Python di [`examples/clients/python`](examples/clients/python) adalah implementasi referensi dan sudah diuji terhadap server.

```python
from client import HalimunClient

client = HalimunClient("http://localhost/proxy/1", AES_KEY_HEX, HMAC_KEY_HEX, 172)
res = client.request(
    "POST",
    "https://api.contoh.com/v1/login",
    body=b'{"email": "user@contoh.com"}',
    headers={"Content-Type": "application/json"},
)
print(res.status_code, res.text)
```

Klien Go, Node.js, dan PHP di `examples/clients/` belum diperbarui ke protokol terbaru.

## Arsitektur

```
Klien ──► Nginx (TLS, rate limit, basic auth dashboard) ──► Halimun ──► Backend
                                                             │
                                              validasi token, cek SSRF, forward
```

Hanya Nginx yang dipublikasikan ke host. Halimun dan Admin API berjalan di jaringan internal Docker.

## Keamanan

- Buat kunci dengan `--keygen`. Jangan pakai nilai contoh, dan jangan commit `.env` atau `config.yaml`.
- Jalankan Halimun di belakang konfigurasi Nginx yang disertakan, dan aktifkan HTTPS di depannya.
- Biarkan `security.allow_key_generation` nonaktif kecuali benar-benar diperlukan.
- Rotasi kunci: buat kunci baru, perbarui `.env`, lalu restart. Klien harus memakai kunci yang sama.

Jangan melaporkan kerentanan lewat issue publik. Gunakan fitur private vulnerability reporting di GitHub untuk repositori ini.

## Pengembangan

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

`tests/e2e.rs` menjalankan backend dan proxy di loopback, lalu memeriksa alur request, penolakan replay, integritas body, validasi HMAC, dan pemblokiran SSRF.

Struktur proyek:

```
src/
  config.rs        pemuatan dan validasi konfigurasi
  crypto/          AES-CBC, HMAC, Base32, obfuscation, codec
  token/           payload token, validator, replay guard
  security/        rate limiter, pengecekan dan resolver SSRF
  services/        registry, health check, log, admin API
  proxy/           handler HTTP
dashboard/         antarmuka admin web (satu file HTML)
nginx/             konfigurasi gateway
examples/clients/  implementasi klien
docs/              panduan penggunaan
```

Sebelum membuka pull request, baca [CONTRIBUTING.md](CONTRIBUTING.md).

## Referensi dan inspirasi

Halimun dibangun dengan ide dari proyek open-source berikut:

| Proyek | Deskripsi | Stack |
| --- | --- | --- |
| [Latebra](https://github.com/exphert/Latebra) | Proxy request terenkripsi dengan AES-256-CBC, HMAC-SHA256, dan pencegahan replay berbasis nonce | PHP / Laravel |
| [rathole](https://github.com/rathole-org/rathole) | Reverse proxy berkinerja tinggi untuk NAT traversal dengan Noise Protocol dan TLS | Rust |
| [frp](https://github.com/fatedier/frp) | Reverse proxy cepat untuk TCP, UDP, HTTP, dan HTTPS dengan TLS | Go |
| [Ghostunnel](https://github.com/ghostunnel/ghostunnel) | Proxy TLS dengan autentikasi mutual TLS untuk backend non-TLS | Go |
| [rustunnel](https://github.com/joaoh82/rustunnel) | Server tunnel self-hosted untuk mengekspos layanan lokal lewat WebSocket terenkripsi TLS | Rust |
| [Proxytunnel](https://github.com/proxytunnel/proxytunnel) | Tunneling koneksi melalui proxy HTTP(S) dengan SSL/TLS | C |
| [RustCrypto](https://github.com/RustCrypto) | Primitif kriptografi (AES, HMAC, SHA-2) | Rust |

## Lisensi

Dirilis di bawah [lisensi MIT](LICENSE).
