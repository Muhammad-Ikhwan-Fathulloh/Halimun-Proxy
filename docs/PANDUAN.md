# Panduan Halimun Proxy

Panduan ini menjelaskan cara **menjalankan**, **mengonfigurasi**, **memakai**, dan **mengamankan** Halimun Proxy (v0.2.0).

Daftar isi:
1. [Gambaran singkat](#1-gambaran-singkat)
2. [Prasyarat](#2-prasyarat)
3. [Menjalankan dengan Docker (disarankan)](#3-menjalankan-dengan-docker-disarankan)
4. [Menjalankan tanpa Docker](#4-menjalankan-tanpa-docker)
5. [Referensi konfigurasi](#5-referensi-konfigurasi)
6. [Protokol: cara membuat request terenkripsi](#6-protokol-cara-membuat-request-terenkripsi)
7. [Memakai klien (Python)](#7-memakai-klien-python)
8. [Target internal (bypass whitelist)](#8-target-internal-bypass-whitelist)
9. [Admin API dan dashboard](#9-admin-api-dan-dashboard)
10. [Kode respons dan pemecahan masalah](#10-kode-respons-dan-pemecahan-masalah)
11. [Keamanan: apa yang dilindungi dan checklist](#11-keamanan-apa-yang-dilindungi-dan-checklist)
12. [Performa dan kompleksitas (Big-O)](#12-performa-dan-kompleksitas-big-o)
13. [Menguji](#13-menguji)
14. [Perubahan penting dari versi sebelumnya](#14-perubahan-penting-dari-versi-sebelumnya)
15. [Batasan yang perlu diketahui](#15-batasan-yang-perlu-diketahui)

---

## 1. Gambaran singkat

Halimun adalah reverse proxy yang menerima request **terenkripsi**. Klien membungkus alamat tujuan, method, header, dan body ke dalam satu token, lalu mengirimkannya lewat URL yang tampak acak. Halimun:

1. Mendekripsi token (AES-256-CBC, lalu obfuscation, lalu Base32).
2. Memverifikasi tanda tangan HMAC-SHA256 dan masa berlaku token.
3. Menolak nonce yang sudah pernah dipakai (anti-replay).
4. Menolak tujuan yang mengarah ke alamat internal (anti-SSRF), kecuali host yang di-whitelist.
5. Meneruskan request ke backend dan mengembalikan respons backend.

```
Klien ──(POST /proxy/1/SEG1/SEG2/SEG3/SEG4/SEG5, x=BODY)──► Nginx ──► Halimun ──► Backend
                                                             (rate limit,      (validasi token,
                                                              batas body)       SSRF, forward)
```

Hanya Nginx yang terbuka ke publik. Halimun dan Admin API berada di jaringan internal Docker.

---

## 2. Prasyarat

| Kebutuhan | Untuk apa |
|---|---|
| Docker Desktop / Docker Engine + Compose | Menjalankan stack (disarankan) |
| Rust 1.75+ (`cargo`) | Build/test tanpa Docker |
| Python 3.9+ dan paket `requests`, `pycryptodome` | Klien Python |

---

## 3. Menjalankan dengan Docker (disarankan)

### Langkah 1 - Salin file contoh

Linux / macOS:
```bash
cp .env.example .env
cp config.example.yaml config.yaml
```

PowerShell (Windows):
```powershell
Copy-Item .env.example .env
Copy-Item config.example.yaml config.yaml
```

### Langkah 2 - Buat kunci kriptografi

Build image dulu, lalu cetak kunci:

```bash
docker build -t halimun-proxy:local .
docker run --rm halimun-proxy:local --keygen --format=env
```

Salin hasilnya ke `.env` untuk `HALIMUN_AES_KEY`, `HALIMUN_HMAC_KEY`, dan `HALIMUN_XOR_KEY`. Jangan tulis kunci ke `config.yaml`, dan jangan commit `.env`.

Buat juga `HALIMUN_ADMIN_KEY` (minimal 16 karakter, acak):

```bash
openssl rand -hex 24
```

```powershell
-join ((48..57 + 97..102) | Get-Random -Count 48 | ForEach-Object { [char]$_ })
```

Isi `.env` Anda akan terlihat seperti ini (nilai dummy):

```bash
HALIMUN_AES_KEY=<64 karakter hex>
HALIMUN_HMAC_KEY=<64 karakter hex>
HALIMUN_XOR_KEY=172
HALIMUN_ADMIN_KEY=<minimal 16 karakter>
```

> **Penting:** jika `.env` ada di folder project, Halimun ikut membacanya saat dijalankan dari folder itu. Nilai di `.env` **menimpa** nilai di `config.yaml`. Pastikan hanya ada satu sumber kebenaran.

### Langkah 3 - Buat akun login dashboard (Nginx basic auth)

Dashboard dilindungi basic auth. Buat user dengan bcrypt:

```bash
docker run --rm httpd:2.4-alpine htpasswd -Bbn admin "GANTI-PASSWORD-ANDA" > nginx/.htpasswd
```

PowerShell (hindari `>` karena PowerShell 5.1 menulis UTF-16):
```powershell
docker run --rm httpd:2.4-alpine htpasswd -Bbn admin "GANTI-PASSWORD-ANDA" | Out-File -Encoding ascii nginx\.htpasswd
```

`nginx/.htpasswd` sudah di-`.gitignore`. Jika file ini pernah ter-commit ke repo, hapus dari index (file di disk tetap ada) lalu ganti password tersebut:

```bash
git rm --cached nginx/.htpasswd
```

### Langkah 4 - Jalankan

```bash
docker compose up -d --build
docker compose ps
docker compose logs -f halimun-proxy
```

Verifikasi:
```bash
curl -i http://localhost/dashboard/                 # harus 401 (minta login)
curl -i -u admin:GANTI-PASSWORD-ANDA http://localhost/dashboard/   # 200
```

Buka `http://localhost/dashboard/`, login dengan user Nginx tadi, lalu masukkan `HALIMUN_ADMIN_KEY` di kolom **Admin Key**.

### Langkah 5 - Atur backend Anda

Edit `services` di `config.yaml` atau tambahkan lewat dashboard. Backend contoh `backend_dummy` ada di `docker-compose.yml`. Ganti atau hapus sesuai kebutuhan.

Jika backend Anda berada di luar Docker, gunakan `bypass_whitelist` (lihat [bagian 8](#8-target-internal-bypass-whitelist)).

### Menghentikan

```bash
docker compose down        # hentikan
docker compose down -v     # hentikan + hapus volume (jarang perlu)
```

---

## 4. Menjalankan tanpa Docker

```bash
# 1. Kunci (simpan ke .env atau export)
cargo run --release -- --keygen --format=env

# 2. Konfigurasi
cp config.example.yaml config.yaml
# Edit: server.admin_host: "127.0.0.1", server.host: "127.0.0.1" untuk uji lokal
# Edit: security.trust_proxy_headers: false (tidak ada Nginx di depan)

# 3. Set rahasia (contoh Linux/macOS)
export HALIMUN_AES_KEY=...
export HALIMUN_HMAC_KEY=...
export HALIMUN_ADMIN_KEY=...

# 4. Validasi konfigurasi lalu jalankan
cargo run --release -- --config config.yaml --check-config
cargo run --release -- --config config.yaml
```

Output sukses:
```
Halimun Proxy listening on 127.0.0.1:8080
Admin API listening on 127.0.0.1:9090
Telemetry (Prometheus) on 127.0.0.1:9091
```

Server menolak start jika konfigurasi tidak valid (kunci kosong, kunci contoh berisi nol, admin key lemah, URL service salah, dsb.). Pesan errornya menjelaskan apa yang harus diperbaiki.

Contoh standalone tanpa file config (untuk uji cepat, port 8080):
```bash
cargo run --example standalone_proxy
```

---

## 5. Referensi konfigurasi

Semua nilai bisa diisi di `config.yaml`. Rahasia sebaiknya lewat environment.

| Kunci | Default | Keterangan |
|---|---|---|
| `server.host` | `127.0.0.1` | Alamat bind proxy publik. Di Docker: `0.0.0.0`. |
| `server.port` | `8080` | Port proxy. Di bawah 1024 tidak bisa dipakai user non-root. |
| `server.admin_host` | `127.0.0.1` | Alamat bind Admin API. Di Docker: `0.0.0.0` (tidak dipublikasikan ke host). |
| `server.admin_port` | `8081` | Port Admin API. |
| `server.admin_api_key` | *(wajib)* | Min. 16 karakter, bukan nilai contoh. Env: `HALIMUN_ADMIN_KEY`. |
| `server.telemetry_host` / `telemetry_port` | `127.0.0.1` / `9090` | Endpoint Prometheus (`/metrics`). |
| `encryption.aes_key` | *(wajib)* | 64 hex (32 byte). Env: `HALIMUN_AES_KEY`. |
| `encryption.hmac_key` | *(wajib)* | Min. 64 hex. Env: `HALIMUN_HMAC_KEY`. Harus berbeda dari `aes_key`. |
| `encryption.xor_key` | `172` | 0-255. Harus sama dengan klien. Env: `HALIMUN_XOR_KEY`. |
| `encryption.base32_alphabet` | `A-Z2-7` | Tepat 32 karakter ASCII unik. Harus sama dengan klien. |
| `encryption.ttl_seconds` | `60` | Parameter informasional. Masa berlaku token diatur oleh field `expired` di token. |
| `security.rate_limit_per_minute` | `120` | Request per IP per menit. |
| `security.nonce_ttl_seconds` | `300` | Parameter informasional. TTL nonce mengikuti field `expired` token (maks 3600). |
| `security.bypass_whitelist` | `[]` | Host yang boleh dituju dengan `bypass_url: true`. Dicocokkan persis. |
| `security.max_body_bytes` | `65536` | Batas body request masuk (maks 16 MB). |
| `security.max_response_bytes` | `10485760` | Batas body respons backend. |
| `security.upstream_timeout_ms` | `15000` | Timeout panggilan ke backend (100 - 120000). |
| `security.allowed_methods` | GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS | Method yang boleh diteruskan. |
| `security.trust_proxy_headers` | `false` | `true` hanya jika ada Nginx yang menimpa `X-Real-IP`. |
| `security.allow_key_generation` | `false` | Mengaktifkan `GET /keys/generate` di Admin API. |
| `services[]` | `[]` | Daftar service: `name`, `type`, `url`, `health`, `timeout_ms`. |

Variabel environment yang didukung: `HALIMUN_AES_KEY`, `HALIMUN_HMAC_KEY`, `HALIMUN_XOR_KEY`, `HALIMUN_ADMIN_KEY`.

---

## 6. Protokol: cara membuat request terenkripsi

### 6.1 Bentuk request

```http
POST /proxy/1/SEG1/SEG2/SEG3/SEG4/SEG5 HTTP/1.1
Content-Type: application/x-www-form-urlencoded

x=BODY_TERENKRIPSI
```

- Tepat **5 segmen** path. Satu segmen berisi token terenkripsi, empat lainnya boleh string acak (dekoy). Posisi token bebas.
- Field `x` opsional. Jika ada, isinya body yang sudah dienkripsi dengan pipeline yang sama.

### 6.2 Isi token (JSON sebelum dienkripsi)

```json
{
  "api_url": "https://api.contoh.com/v1/login",
  "api_header": { "Content-Type": "application/json", "Authorization": "Bearer ..." },
  "method": "POST",
  "timestamp": 1767225600,
  "expired": 300,
  "nonce": "7c1f0e0a9d2b4c3e8f5a6b7c8d9e0f12",
  "bypass_url": false,
  "body_sha256": "<sha256 hex dari plaintext body, WAJIB jika ada body>",
  "hmac": "<lihat 6.3>"
}
```

| Field | Wajib | Arti |
|---|---|---|
| `api_url` | ya | URL tujuan (http/https, tanpa user:password di URL). |
| `api_header` | tidak | Header yang diteruskan. Header hop-by-hop dan `Host`, `Content-Length`, `X-Forwarded-For` diabaikan. |
| `method` | ya | Harus termasuk `allowed_methods`. |
| `timestamp` | ya | Unix detik saat token dibuat. Tidak boleh lebih dari 60 detik di masa depan. |
| `expired` | ya | Masa berlaku dalam detik (1 - 3600). Token kedaluwarsa jika `now > timestamp + expired`. |
| `nonce` | ya | String unik per request (1 - 128 karakter). Dipakai sekali. |
| `bypass_url` | tidak | `true` untuk target di `bypass_whitelist`. |
| `body_sha256` | ya, jika ada body | SHA-256 (hex) dari **plaintext** body. Mengikat body ke tanda tangan. |
| `hmac` | ya | Tanda tangan (lihat 6.3). |

### 6.3 Aturan HMAC (JSON kanonik)

1. Buat objek JSON **tanpa** field `hmac`.
2. Serialisasi dengan **kunci terurut secara leksikografis** dan **tanpa spasi**. Contoh: `{"api_url":"...","expired":300,...}`.
3. `hmac = HMAC-SHA256(hmac_key_bytes, string_json_kanonik)`, ditulis dalam hex huruf kecil.
4. Tambahkan field `hmac` ke objek lalu enkripsi.

Server melakukan hal yang sama. Perbedaan spasi, urutan kunci, atau tipe angka akan menghasilkan `Invalid HMAC`.

### 6.4 Pipeline enkripsi (urutan wajib)

```
plaintext
  -> AES-256-CBC, PKCS#7, IV acak 16 byte (IV ditaruh di depan)     => IV || CT
  -> obfuscation: XOR setiap byte dengan xor_key
                  balik urutan tiap blok 4 byte penuh
                  rotasi kiri 1 byte (byte pertama pindah ke belakang)
  -> Base32 (alfabet dari config, TANPA padding '=')
  -> string segmen URL / nilai field x
```

Dekripsi adalah kebalikan: Base32 decode -> rotasi kanan 1 byte -> balik blok 4 byte -> XOR -> AES decrypt (ambil 16 byte pertama sebagai IV).

### 6.5 Validasi yang dilakukan server (urutan)

1. Dekripsi setiap segmen. Segmen yang gagal didekripsi diabaikan.
2. Parse JSON, verifikasi HMAC (konstan-waktu).
3. Cek nonce, `expired`, dan `timestamp`.
4. Cek nonce belum pernah dipakai (anti-replay). Nonce hanya dicatat **setelah** HMAC valid.
5. Validasi `api_url` (SSRF) dan `method`.
6. Dekripsi body, lalu cocokkan SHA-256-nya dengan `body_sha256`.
7. Teruskan ke backend.

---

## 7. Memakai klien (Python)

`examples/clients/python/client.py` adalah **implementasi referensi** dan sudah diuji end-to-end terhadap server ini.

Instalasi:
```bash
pip install requests pycryptodome
```

Set kunci (dari `.env` yang sama dengan server):
```bash
export HALIMUN_PROXY_URL=http://localhost/proxy/1
export HALIMUN_AES_KEY=...
export HALIMUN_HMAC_KEY=...
export HALIMUN_XOR_KEY=172
```

Contoh penggunaan di kode Anda:
```python
import json, os
from client import HalimunClient

client = HalimunClient(
    "http://localhost/proxy/1",
    os.environ["HALIMUN_AES_KEY"],
    os.environ["HALIMUN_HMAC_KEY"],
    int(os.environ.get("HALIMUN_XOR_KEY", 172)),
)

res = client.request(
    "POST",
    "https://api.contoh.com/v1/login",             # URL backend tujuan
    body=json.dumps({"email": "user@example.com"}).encode(),
    headers={"Content-Type": "application/json"},
)
print(res.status_code, res.text)
```

Argumen `request()`:
- `method`, `api_url`: wajib.
- `body`: bytes. Kosongkan jika tidak ada body.
- `headers`: dict header yang diteruskan ke backend.
- `bypass`: `True` untuk target internal di whitelist.
- `ttl`: masa berlaku token (detik), default 60.
- `nonce`: opsional. Jika tidak diisi, dibuat acak.

Uji cepat dari terminal:
```bash
cd examples/clients/python
python client.py
```

> **Klien lain (Go, Node.js, PHP) di `examples/clients/` belum diperbarui ke protokol v0.2** (kunci JSON terurut, `body_sha256`). Jangan dipakai sampai diport dan diuji. Gunakan Python sebagai referensi.

---

## 8. Target internal (bypass whitelist)

Target yang berada di jaringan privat (mis. `backend_app`, `10.x`, `192.168.x`, `localhost`) **diblokir** secara default.

Untuk mengizinkan satu host internal:

1. Tambahkan host **persis** ke `security.bypass_whitelist`:
   ```yaml
   security:
     bypass_whitelist:
       - "api.internal.perusahaan"
   ```
2. Kirim token dengan `"bypass_url": true` dan `api_url` yang host-nya sama persis.

Yang tetap diblokir walaupun di-whitelist: skema selain http/https, user:password di URL, host yang cocok hanya sebagian (mis. `evil.com/?h=api.internal.perusahaan`).

---

## 9. Admin API dan dashboard

Admin API hanya bisa dijangkau lewat Nginx (`/admin/`, dengan basic auth) dan wajib header `x-admin-key`.

Contoh (lewat Nginx di Docker):
```bash
BASE=http://localhost/admin
AUTH="-u admin:GANTI-PASSWORD-ANDA"
KEY="x-admin-key: $HALIMUN_ADMIN_KEY"

curl $AUTH -H "$KEY" $BASE/stats                 # status, uptime, jumlah service
curl $AUTH -H "$KEY" $BASE/services              # daftar service
curl $AUTH -H "$KEY" $BASE/logs                  # 100 log terakhir (URL sudah dibersihkan dari query)

curl $AUTH -H "$KEY" -H "Content-Type: application/json" -X POST $BASE/services -d '{
  "name": "api-internal",
  "type": "backend",
  "url": "http://backend_app:80",
  "health": "/health",
  "timeout_ms": 10000
}'

curl $AUTH -H "$KEY" -X DELETE $BASE/services/api-internal
```

Aturan:
- Nama service hanya huruf, angka, `-`, `_` (maks 64 karakter). URL harus http/https.
- Perubahan `services` ditulis ke `config.yaml` (hanya bagian `services`). Jika penulisan gagal, perubahan di memori dibatalkan dan API membalas 500.
- `GET /keys/generate` hanya aktif jika `security.allow_key_generation: true`. Endpoint ini mengembalikan rahasia dalam plaintext, jadi biarkan nonaktif kecuali benar-benar perlu.
- Endpoint `/keys/dynamic` sudah dihapus karena tidak pernah benar-benar dipakai.

Dashboard (`/dashboard/`) memanggil Admin API yang sama. Menampilkan log, daftar service, dan kunci (jika diaktifkan).

Metrics Prometheus: `http://<host>:9091/metrics` (di Docker tidak dipublikasikan; scrape dari jaringan internal).

---

## 10. Kode respons dan pemecahan masalah

| Status | Pesan | Penyebab umum |
|---|---|---|
| 200 | (respons backend) | Berhasil diteruskan. |
| 400 | `Decryption failed or invalid token` | Kunci AES, `xor_key`, atau `base32_alphabet` tidak sama dengan klien. Atau `.env` menimpa `config.yaml`. |
| 400 | `Decryption failed for body` | Body `x=` tidak bisa didekripsi. Periksa kunci dan pipeline. |
| 403 | `Invalid HMAC` | JSON kanonik tidak sama (urutan kunci/spasi), `hmac_key` salah, atau token diubah. |
| 403 | `Expired` | `now > timestamp + expired`. Periksa jam klien (NTP). |
| 403 | `Token timestamp is in the future` | Jam klien lebih dari 60 detik di depan. |
| 403 | `Nonce replayed` | Nonce yang sama dikirim dua kali. Buat nonce baru per request. |
| 403 | `Forbidden: Cannot proxy to internal addresses` | Target privat tanpa whitelist/bypass. Lihat bagian 8. |
| 403 | `Body integrity check failed` | `body_sha256` tidak cocok atau tidak ada. |
| 403 | `URL not in bypass whitelist` / `Bypass not configured` | `bypass_url: true` tanpa host di whitelist. |
| 405 | `Method not allowed` | Method tidak ada di `allowed_methods`. |
| 413 | (dari Nginx/Axum) | Body melebihi `max_body_bytes` (Nginx juga membatasi 64 KB). |
| 429 | `Too many requests` | Melebihi `rate_limit_per_minute`. |
| 502 | `Failed to reach backend service` | Backend mati, DNS gagal, atau alamat diblokir resolver. Detail ada di log server (`docker compose logs`). |
| 502 | `Response too large` | Respons melebihi `max_response_bytes`. |
| 503 | `Server busy, try again later` | Penyimpanan nonce penuh (1 juta entri). |
| 504 | `Backend timeout` | Backend melebihi `upstream_timeout_ms`. |

Pemecahan masalah umum:

- **Server tidak mau start:** jalankan `halimun-proxy --config config.yaml --check-config` dan baca pesannya.
- **Kunci berubah setelah `.env` diedit:** restart container (`docker compose up -d`). Environment hanya dibaca saat start.
- **Dashboard 401 terus:** user/password Nginx salah, atau `nginx/.htpasswd` berformat UTF-16 (buat ulang dengan `Out-File -Encoding ascii`).
- **Dashboard tidak menampilkan data:** Admin Key di kolom dashboard salah.
- **Simpan service gagal dengan 500:** `config.yaml` tidak bisa ditulis (cek permission file dan mount Docker).

---

## 11. Keamanan: apa yang dilindungi dan checklist

### 11.1 Yang dilindungi

| Ancaman | Perlindungan |
|---|---|
| Penyadapan isi request | AES-256-CBC per token dan per body. |
| Pemalsuan / perubahan token | HMAC-SHA256 atas JSON kanonik (verifikasi konstan-waktu). |
| Perubahan body | `body_sha256` di dalam token yang ditandatangani. |
| Replay | Nonce dicatat atomik setelah HMAC valid. Expired token ditolak. |
| SSRF ke jaringan internal | Validasi URL + resolver DNS yang membuang alamat privat + redirect dimatikan. |
| Akses Admin API oleh orang lain | Basic auth Nginx + `x-admin-key` (dibandingkan konstan-waktu). |
| Bocornya info internal | Pesan error generik; detail hanya di log server; URL di log tanpa query/userinfo. |
| Brute force / banjir | Rate limit per IP (Halimun dan Nginx), batas body, batas respons, timeout. |
| XSS di dashboard | Semua data dari API di-escape. |
| Header spoofing `X-Forwarded-For` / `X-Real-IP` | Diabaikan kecuali `trust_proxy_headers: true` di belakang Nginx. |
| Kunci lemah / contoh | Server menolak start jika kunci nol, kosong, sama, atau admin key lemah. |
| Container dijalankan root | Image berjalan sebagai user `halimun`; compose memakai `no-new-privileges`, `cap_drop: ALL`. |

### 11.2 Checklist sebelum produksi

- [ ] Kunci dibuat dengan `--keygen`. Tidak ada yang sama dengan contoh.
- [ ] `aes_key`, `hmac_key`, `admin_api_key` hanya di `.env`/secret manager, bukan di Git.
- [ ] `.env`, `config.yaml`, dan `nginx/.htpasswd` tidak ter-commit (`git status` bersih dari file itu).
- [ ] Password dashboard sudah diganti dan kuat.
- [ ] Port 8080, 9090, 9091 **tidak** dipublikasikan ke host (hanya 80/443 via Nginx).
- [ ] HTTPS aktif di depan Nginx (lihat di bawah). Jangan biarkan dashboard dan `/proxy/` berjalan di HTTP polos di internet.
- [ ] `security.allow_key_generation: false`.
- [ ] `bypass_whitelist` berisi host seminimal mungkin.
- [ ] Klien membangkitkan nonce acak dan menyimpan `expired` pendek (60-300 detik).
- [ ] Jam server dan klien tersinkron (NTP).
- [ ] Log dan `docker compose logs` dipantau; rotasi log diatur.
- [ ] Rahasia yang pernah ter-commit (mis. `.htpasswd` lama, kunci contoh lama) sudah **dirotasi**.

### 11.3 Deploy HTTPS

Nginx menerima HTTPS, Halimun tetap HTTP di jaringan internal. Tambahkan blok `listen 443 ssl;` dengan `ssl_certificate` dan `ssl_certificate_key` di `nginx/nginx.conf`, dan mount sertifikatnya di `docker-compose.yml`. Alternatif yang lebih mudah: letakkan Caddy atau Traefik di depan Nginx (mereka mengurus sertifikat Let's Encrypt otomatis).

### 11.4 Catatan jujur tentang keamanan

- **Obfuscation XOR/rotasi bukan kriptografi.** Keamanan sebenarnya dari AES-256-CBC + HMAC. Jangan mengandalkan kerahasiaan algoritma obfuscation.
- **Dekoy hanya menyamarkan.** Pola request tetap bisa diamati (panjang URL, frekuensi). Gunakan TLS agar isi URL tidak terlihat di jaringan.
- **Nonce disimpan di memori per proses.** Jika menjalankan lebih dari satu instance, replay bisa lolos antar instance. Jalankan satu instance, atau tambahkan penyimpanan nonce bersama (mis. Redis) sebagai pengembangan lanjutan.
- **`bypass_whitelist` dicocokkan per host**, bukan per IP. Pastikan DNS untuk host itu tepercaya.

---

## 12. Performa dan kompleksitas (Big-O)

Notasi: `L` = panjang string/byte payload, `N` = jumlah service, `I` = jumlah IP aktif, `S` = jumlah segmen (tetap 5), `W` = jumlah nonce aktif.

| Operasi | Sebelum v0.2 | Sekarang | Catatan |
|---|---|---|---|
| Decode Base32 | O(L x alfabet) + string bit (banyak alokasi) | **O(L)**, tabel lookup 256 entri, tanpa alokasi bit | `crypto/base32.rs` |
| Obfuscation XOR/balik/rotasi | O(L), tapi `remove(0)`/`insert(0)` = O(L) per operasi | **O(L)**, `rotate_left/right` | `crypto/obfuscation.rs` |
| Dekripsi token | O(S x L) | **O(S x L) = O(L)** (S = 5 konstan) | Segmen dekoy gagal cepat di padding |
| State per request | `Clone` seluruh `AppConfig` (O(N + whitelist)) + hex-decode kunci | **O(1)**: hanya menaikkan Arc. Kunci didecode sekali saat start | `proxy/handler.rs` |
| Rate limit per request | O(1) rata-rata, memori bertambah per IP **per menit** | **O(1)** rata-rata, memori **O(I)** dengan pembersihan | `security/rate_limiter.rs` |
| Cek nonce | O(1) rata-rata, dua pencarian + celah race | **O(1)** rata-rata, satu operasi `entry` atomik, batas 1 juta | `token/replay_guard.rs` |
| Pengecekan kesehatan service | **O(N x timeout)** berurutan, hasil dibuang | **O(max timeout)** paralel per siklus, hasil disimpan | `services/health.rs` |
| Baca log | Lock async | Mutex sinkron, O(1) push/pop, O(N_log) untuk salinan | `services/logs.rs` |
| Simpan service (admin) | Baca + tulis seluruh config | Tulis hanya bagian `services` (temp + rename), diserialisasi | `services/admin.rs` |
| Body respons | Seluruh body di memori tanpa batas | Dibaca bertahap, berhenti di `max_response_bytes` | `proxy/handler.rs` |
| Forward ke backend | 1 panggilan HTTP | 1 panggilan HTTP (dengan pool koneksi) | Tidak ada fan-out |

**N+1:** Halimun tidak memakai database, jadi pola N+1 query klasik tidak ada. Pola serupa yang dulu ada, yaitu menyalin konfigurasi per request dan mengecek service satu per satu secara berurutan, sudah dihapus.

Panduan praktis:
- Naikkan `security.upstream_timeout_ms` hanya jika backend memang lambat. Timeout yang panjang menahan koneksi.
- Pantau `halimun_http_requests_total` di `/metrics`.
- Untuk beban tinggi, jalankan beberapa replika di belakang Nginx dan ingat batasan nonce per proses (lihat 11.4).

---

## 13. Menguji

```bash
# Unit + integrasi (butuh toolchain Rust)
cargo test

# Format dan lint (sama dengan CI)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Uji integrasi `tests/e2e.rs` menjalankan backend palsu dan proxy di loopback. Ia memeriksa:
- request terenkripsi diteruskan dan body didekripsi,
- replay ditolak,
- body tanpa `body_sha256` ditolak,
- HMAC yang dirusak ditolak,
- target loopback tanpa whitelist diblokir (SSRF),
- segmen acak ditolak.

Uji manual dari klien Python terhadap server yang berjalan: lihat bagian 7.

---

## 14. Perubahan penting dari versi sebelumnya

Ini **perubahan yang memutus kompatibilitas**. Klien lama harus diperbarui.

1. **HMAC memakai JSON kanonik** (kunci terurut, tanpa spasi). Sebelumnya urutan bergantung pada struct dan `HashMap`, sehingga verifikasi bisa gagal secara acak.
2. **`body_sha256` wajib** jika request punya body.
3. **Kunci wajib dari konfigurasi/environment.** Nilai kosong atau contoh (nol) membuat server menolak start.
4. **Admin key wajib minimal 16 karakter** dan tidak boleh nilai contoh (`admin-secret-key`).
5. **Admin API listen di `127.0.0.1` secara default.**
6. **`/keys/dynamic` dihapus.** `/keys/generate` nonaktif kecuali diaktifkan.
7. **`bypass_whitelist` dicocokkan persis**, bukan substring.
8. **Redirect dimatikan** di klien proxy.
9. **Body `x=` yang tidak bisa didekripsi** sekarang menghasilkan 400 (sebelumnya diam-diam dikirim kosong).
10. **Header dari token** yang berbahaya (`Host`, `Content-Length`, `X-Forwarded-For`, dsb.) diabaikan.
11. **Dependensi Redis dihapus** (modul `redis_guard` tidak pernah dipakai).
12. **Image Docker berjalan sebagai non-root** dan proxy listen di port 8080 (bukan 80).
13. **`nginx/.htpasswd` tidak lagi tercantum** di panduan sebagai kredensial default.

---

## 15. Batasan yang perlu diketahui

- Klien selain Python belum diperbarui ke protokol v0.2.
- Nonce tersimpan di memori satu proses (lihat 11.4).
- Halimun tidak menyediakan TLS sendiri. Gunakan Nginx/Caddy/Traefik di depannya.
- Tidak ada rotasi kunci otomatis. Rotasi dilakukan dengan membuat kunci baru, memperbarui `.env`, lalu restart. Selama rotasi, klien dan server harus memakai kunci yang sama.
- Tidak ada rate limit per akun (hanya per IP).
