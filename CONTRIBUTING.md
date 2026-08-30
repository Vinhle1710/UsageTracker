# Contributing

Thanks for helping improve Usage Tracker Overlay.

## Development setup

The desktop application targets Windows 10/11 and requires Node.js 22, Rust, WebView2, and the Windows desktop build tools.

```powershell
npm ci
npm run dev
npm run tauri dev
```

## Before opening a pull request

Run the same checks enforced by CI:

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Keep changes focused, include regression tests for behavior changes, and never commit real provider credentials, cookies, account payloads, or unredacted usage fixtures.

## Security reports

Do not open a public issue for a vulnerability. Follow [SECURITY.md](SECURITY.md) and use the repository's private Security advisories page.
