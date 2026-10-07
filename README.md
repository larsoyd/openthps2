# thps2-rs

Tony Hawk's Pro Skater 2 reimplementation in Rust and Bevy. Original game files are required and are not distributed here.

## Resource inspection

Use the `ALL.PKR` from the supplied Windows USA rerelease CD (`SETUP/data`). Keep the CD and extracted data outside this source directory.

```bash
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data --manifest
```

The first command validates the archive and decodes the Hangar and Tony Hawk resources through Bevy. The second emits tab-separated resource names, sizes and SHA-256 hashes. These commands inspect resources; they do not launch a playable session.

The supported archive SHA-256 is `7e2ae173577c41598dc644824b52f7eaf9664ea478e177a8703f77c0185207da`. Other archives are rejected by the application. The format library reports unsupported compression explicitly.

## Development

Rust 1.89 or newer, edition 2024, Bevy 0.18.1. Dependencies are locked in Cargo.lock. Omit `--offline` if the dependencies are not cached. If a configured compiler wrapper cannot run in your sandbox, set `RUSTC_WRAPPER=`.

```bash
cargo fmt --all --check
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
```
