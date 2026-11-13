# Halimun Proxy

[![CI](https://github.com/Muhammad-Ikhwan-Fathulloh/Halimun-Proxy/actions/workflows/rust.yml/badge.svg)](https://github.com/Muhammad-Ikhwan-Fathulloh/Halimun-Proxy/actions)
[![Docker Hub](https://img.shields.io/docker/pulls/ikhwan17/halimun-proxy)](https://hub.docker.com/r/ikhwan17/halimun-proxy)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Halimun is an encrypted reverse proxy written in Rust. Clients wrap the destination, method, headers, and body in an encrypted token. Halimun validates the token, blocks internal targets, and forwards the request to your backend.

Read this in other languages: [English](README.md) · [Bahasa Indonesia](README.id.md)

- **Documentation:** [docs/PANDUAN.md](docs/PANDUAN.md) (Indonesian, full guide)
- **Image:** [ikhwan17/halimun-proxy](https://hub.docker.com/r/ikhwan17/halimun-proxy) on Docker Hub

## Features

- AES-256-CBC encryption of tokens and request bodies, with an obfuscation layer
- HMAC-SHA256 signature over canonical JSON, with body hash binding
- Nonce replay protection and token expiry checks
- SSRF protection: private and loopback addresses are blocked unless explicitly whitelisted
- Per-IP rate limiting, body and response size limits, and upstream timeouts
- Admin API and web dashboard for traffic logs, services, and key generation
- Prometheus metrics endpoint
- Small runtime image (Alpine, non-root user)

## Quick start (Docker)

1. Pull the image and generate keys:

   ```bash
   docker pull ikhwan17/halimun-proxy:latest
   docker run --rm ikhwan17/halimun-proxy:latest --keygen --format=env > .env
   ```

   PowerShell:

   ```powershell
   docker run --rm ikhwan17/halimun-proxy:latest --keygen --format=env | Out-File -Encoding ASCII .env
   ```

   Add `HALIMUN_ADMIN_KEY` (at least 16 characters) to `.env`.

2. Copy the example configuration:

   ```bash
   cp config.example.yaml config.yaml
   ```

3. Create the dashboard login (Nginx basic auth):

   ```bash
   docker run --rm httpd:2.4-alpine htpasswd -Bbn admin "your-password" > nginx/.htpasswd
   ```

4. Start the stack:

   ```bash
   docker compose pull
   docker compose up -d
   ```

5. Open `http://localhost/dashboard/`, sign in with the Nginx user, and enter the admin key.

Building from source, HTTPS, and the protocol details are covered in [docs/PANDUAN.md](docs/PANDUAN.md).

## Configuration

Secrets are read from environment variables (`HALIMUN_AES_KEY`, `HALIMUN_HMAC_KEY`, `HALIMUN_XOR_KEY`, `HALIMUN_ADMIN_KEY`) and override values in `config.yaml`. The server refuses to start with missing, zero, or example keys.

Validate a configuration without starting the server:

```bash
halimun-proxy --config config.yaml --check-config
```

The full reference is in [docs/PANDUAN.md, section 5](docs/PANDUAN.md#5-referensi-konfigurasi).

## Client example

The Python client in [`examples/clients/python`](examples/clients/python) is the reference implementation and is tested against the server.

```python
from client import HalimunClient

client = HalimunClient("http://localhost/proxy/1", AES_KEY_HEX, HMAC_KEY_HEX, 172)
res = client.request(
    "POST",
    "https://api.example.com/v1/login",
    body=b'{"email": "user@example.com"}',
    headers={"Content-Type": "application/json"},
)
print(res.status_code, res.text)
```

Clients in Go, Node.js, and PHP under `examples/clients/` have not yet been updated to the current protocol.

## Architecture

```
Client ──► Nginx (TLS, rate limit, basic auth for dashboard) ──► Halimun ──► Backend
                                                                 │
                                                  validate token, SSRF check, forward
```

Only Nginx is published to the host. Halimun and the Admin API run on the internal Docker network.

## Security

- Generate keys with `--keygen`. Never reuse example values, and never commit `.env` or `config.yaml`.
- Run Halimun behind the bundled Nginx configuration, and terminate HTTPS in front of it.
- Keep `security.allow_key_generation` disabled unless you need it.
- Rotate keys by generating new ones, updating `.env`, and restarting. Clients must use the same keys.

Do not report security issues in public issues. Use GitHub's private vulnerability reporting for this repository.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

`tests/e2e.rs` runs a backend and the proxy on loopback and checks the request flow, replay rejection, body integrity, HMAC validation, and SSRF blocking.

Project layout:

```
src/
  config.rs        configuration loading and validation
  crypto/          AES-CBC, HMAC, Base32, obfuscation, codec
  token/           token payload, validator, replay guard
  security/        rate limiter, SSRF checks and resolver
  services/        registry, health checks, logs, admin API
  proxy/           HTTP handler
dashboard/         admin web UI (single HTML file)
nginx/             gateway configuration
examples/clients/  client implementations
docs/              usage guide
```

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## References and inspiration

Halimun was built with ideas drawn from these open-source projects:

| Project | Description | Stack |
| --- | --- | --- |
| [Latebra](https://github.com/exphert/Latebra) | Encrypted request proxy with AES-256-CBC, HMAC-SHA256, and nonce-based replay prevention | PHP / Laravel |
| [rathole](https://github.com/rathole-org/rathole) | High-performance reverse proxy for NAT traversal with Noise Protocol and TLS | Rust |
| [frp](https://github.com/fatedier/frp) | Fast reverse proxy supporting TCP, UDP, HTTP, and HTTPS with TLS | Go |
| [Ghostunnel](https://github.com/ghostunnel/ghostunnel) | TLS proxy with mutual TLS authentication for non-TLS backends | Go |
| [rustunnel](https://github.com/joaoh82/rustunnel) | Self-hosted tunnel server exposing local services over TLS-encrypted WebSockets | Rust |
| [Proxytunnel](https://github.com/proxytunnel/proxytunnel) | Tunneling connections through HTTP(S) proxies with SSL/TLS | C |
| [RustCrypto](https://github.com/RustCrypto) | Cryptographic primitives (AES, HMAC, SHA-2) | Rust |

## License

Released under the [MIT License](LICENSE).
