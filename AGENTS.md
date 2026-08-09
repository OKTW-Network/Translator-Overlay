# Agent instructions — Translator Overlay

Rust workspace (Windows) for real-time OCR + LLM translation overlay. Crates live under `crates/`.

## Mandatory checks

**Always run format + Clippy before considering Rust work done** (and before any commit that touches Rust sources):

```powershell
cargo +nightly fmt
cargo clippy
```

Use `cargo +nightly fmt` (not plain `cargo fmt`) so workspace `rustfmt` unstable options apply.

Requirements:

- Both commands must exit **0**.
- Format with nightly so style matches the repo `rustfmt` config.
- Fix **all** Clippy warnings in code you touch; do not leave new warnings behind.
- Prefer fixing warnings over `#[allow(...)]` unless there is a documented, local reason.

If format or Clippy fails or warns, fix the code and re-run until clean. Do not hand off or commit with either still dirty.

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
