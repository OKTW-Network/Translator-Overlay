# Agent instructions — Translator Overlay

Rust workspace (Windows) for real-time OCR + LLM translation overlay. Crates live under `crates/`.

## Mandatory checks

**Always run format + Clippy before considering Rust work done** (and before any commit that touches Rust sources):

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

Use `cargo +nightly fmt` (not plain `cargo fmt`) so workspace `rustfmt` unstable options apply.

Use `cargo clippy --all-targets -- -D warnings` (not plain `cargo clippy`). `--all-targets` lints tests too; `-D warnings` makes **any** warning fail the command (plain Clippy exits 0 even when it prints warnings).

Requirements:

- Both commands must exit **0**.
- Clippy output must be warning-free. Do not treat a zero exit from Clippy without `-D warnings` as clean.
- Format with nightly so style matches the repo `rustfmt` config.
- Fix every Clippy warning you see, including tests and pre-existing ones — not only lines you introduced.
- Prefer fixing warnings over `#[allow(...)]` unless there is a documented, local reason.

If format or Clippy fails, fix the code and re-run until clean. Do not hand off or commit with either still dirty.

Optional (when tests exist for the changed crate):

```powershell
cargo test
```

## Style

- Same-crate imports use `crate::…` paths (not `super::`, `self::`, or bare child-module paths).
- Exception: unit tests may keep `use super::*;` to pull the parent module under test.

## Build

```powershell
cargo build -p translator-app
# release / package:
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```
