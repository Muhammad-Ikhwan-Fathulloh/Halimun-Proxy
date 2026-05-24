# Contributing to Halimun Proxy

First off, thank you for considering contributing to Halimun Proxy! It's people like you that make open-source a great community.

## 🤝 How Can You Help?

Because Halimun is an ultra-fast encrypted reverse proxy gateway, we are always looking for improvements in:
1. **Cryptography Optimization**: Refining the AES-256-CBC, HMAC checks, or the XOR/Base32 rotation algorithms to reduce CPU cycle overhead.
2. **Caching & Rate Limiting**: Expanding the `DashMap` implementations to support scalable Redis clustering for multi-node deployments.
3. **Observability**: Giving better insight into the Dashboard (e.g. integrations with OpenTelemetry, Prometheus, etc).
4. **Documentation**: Adding more examples or fixing typos.

## 🛠️ Development Workflow

1. Fork the repository.
2. Clone your fork locally (`git clone https://github.com/Muhammad-Ikhwan-Fathulloh/Halimun-Proxy.git`).
3. Create a branch (`git checkout -b feature/amazing-feature`).
4. Make your changes in Rust or the HTML Dashboard.
5. Run the tests before pushing:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo build --release
   ```
6. Commit your changes (`git commit -m "feat: Add Redis cache layer"`).
7. Push to the branch (`git push origin feature/amazing-feature`).
8. Open a Pull Request!

## 🐛 Reporting Bugs

If you find a bug, please create an Issue using the GitHub Issue Tracker. Include:
- A clear description of the problem.
- Your OS and Rust version.
- Related error logs (from the `docker logs` or the Cargo build trace).

Thank you for helping us keep Halimun secure and performant!
