# Contributing to Halimun Proxy

Thank you for considering a contribution. Useful contributions include bug fixes, tests, documentation improvements, and client implementations for other languages.

## Getting started

1. Fork the repository and clone your fork.
2. Create a branch from `main`, for example `fix/nonce-cleanup` or `docs/client-guide`.
3. Make your change. Keep the scope small, and update documentation when behaviour changes.

## Before opening a pull request

Run the same checks as CI:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

If you changed the dashboard, open `dashboard/index.html` in a browser against a running Admin API and check the affected views.

## Commit messages

Use short, imperative messages with a type prefix, for example:

- `fix: reject bodies without matching body_sha256`
- `feat: add per-service timeout to health checks`
- `docs: clarify key rotation`

## Pull requests

- Describe what changed and why.
- Note any protocol, configuration, or API change. These affect existing clients and deployments.
- Link related issues.

## Reporting bugs

Open an issue and include:

- A short description of the problem and the expected behaviour.
- Steps to reproduce.
- Your OS, Docker or Rust version, and the Halimun version.
- Relevant logs from `docker compose logs halimun-proxy` or the cargo output.

Do not include real keys, passwords, or `.env` contents in issues.

## Security issues

Do not open a public issue for a vulnerability. Use GitHub's private vulnerability reporting for this repository.
