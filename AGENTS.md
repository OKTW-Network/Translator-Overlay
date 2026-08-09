# Agent instructions — Translator Overlay

Rust workspace (Windows) for real-time OCR + LLM translation overlay. Crates live under `crates/`.

## Mandatory checks

**Always run Clippy before considering Rust work done** (and before any commit that touches Rust sources):

```powershell
cargo clippy
```

Requirements:

- Exit code must be **0**.
- Fix **all** Clippy warnings in code you touch; do not leave new warnings behind.
- Prefer fixing warnings over `#[allow(...)]` unless there is a documented, local reason.

If Clippy fails or warns, fix the code and re-run until clean. Do not hand off or commit with Clippy still dirty.

Optional (when tests exist for the changed crate):

```powershell
cargo test
```

## Build

```powershell
cargo build -p translator-app
# release / package:
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```
