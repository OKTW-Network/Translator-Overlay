# Agent instructions for Translator Overlay

This is a Windows Rust workspace for a real-time OCR and LLM translation overlay. The crates live under `crates/`.

## Mandatory checks

Always run format and Clippy before you consider Rust work done, and before any commit that touches Rust sources.

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

Use `cargo +nightly fmt`, not plain `cargo fmt`, so the workspace's unstable `rustfmt` options apply.

Use `cargo clippy --all-targets -- -D warnings`, not plain `cargo clippy`. `--all-targets` lints the tests too, and `-D warnings` makes any warning fail the command. Plain Clippy exits 0 even when it prints warnings.

The rules are these.

- Both commands must exit 0.
- Clippy output must have no warnings. A zero exit from Clippy without `-D warnings` does not count as clean.
- Format with nightly so the style matches the repo's `rustfmt` config.
- Fix every Clippy warning you see, including those in tests and those that were already there, not only the lines you added.
- Fix warnings rather than adding `#[allow(...)]`, unless there is a documented local reason.

If format or Clippy fails, fix the code and run them again until both are clean. Do not hand off or commit while either one fails.

When the changed crate has tests, you can also run them.

```powershell
cargo test
```

## Style

- Imports from the same crate use `crate::…` paths, not `super::`, `self::`, or bare child-module paths.
- The one exception is unit tests, which may keep `use super::*;` to pull in the parent module under test.
- Do not extract a helper that has a single call site, or that exists only so a one-liner can be unit-tested. Inline it instead. Extract only when there is a second call site or non-trivial shared logic.

## Build

```powershell
cargo build -p translator-app
# Release build and package:
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```
