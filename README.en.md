<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# <img width="32" height="32" src="crates/translator-app/assets/icon.png"> Translator Overlay [![CI](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml/badge.svg)](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml) [![GitHub Release](https://img.shields.io/github/v/release/OKTW-Network/Translator-Overlay)](https://github.com/OKTW-Network/Translator-Overlay/releases) [![GitHub Downloads](https://img.shields.io/github/downloads/OKTW-Network/Translator-Overlay/latest/total?label=downloads&color=67ac09)](https://github.com/OKTW-Network/Translator-Overlay/releases/latest)
[繁體中文](README.md) | English

Real-time translation overlay for the Windows desktop: pick a target window → capture it → recognize text with PP-OCRv6 → translate through an OpenAI-compatible API or a local CLI → draw the translation in place on a click-through overlay.

## Features

- **Window capture**: Windows Graphics Capture. Draw several OCR regions on the target window, or recognize the whole window. Regions can be saved as named presets
- **Local OCR**: PP-OCRv6 (tiny / small / medium) on ONNX Runtime with WebGPU or DirectML (falls back to CPU). Text is sent for translation only after the screen settles (with an optional force-translate time for text that never settles). Filters single-character noise and animations that OCR misreads
- **Line merge**: Off / Position rules / Smart detection (PP-DocLayoutV3 layout analysis) / Merge all. Lines are joined with nothing, a space, or a line break. Reading order is rows or columns; vertical text columns always read right to left
- **LLM translation**: OpenAI-compatible API (Chat Completions or Responses), or local Grok / OpenCode / Codex / Claude Code. When streaming, the overlay and translation window update as the translation arrives. The system prompt can be customized (it replaces the style only; output format and languages stay)
- **API options**: Structured outputs, streaming, reasoning replay, Temperature / Top P / Max tokens / Reasoning effort, Priority mode; timeout, retries, and backoff. The Model field autocompletes from the provider's model list. The control window's taskbar button flashes when translation fails
- **Glossary**: save source → translation term pairs as named profiles and send them to the model. More terms use more tokens
- **Translation memory and context**: each source line is translated once per session (LFU cache with adjustable size, manual clear, emptied when the app closes). Multi-turn history keeps wording consistent; **Clear history** starts fresh
- **Display**: click-through overlay (follows the target window and scrolled text, hides when the target is minimized; captions never overlap and widen, shrink, or wrap as needed) + a separate always-on-top translation window (drag to move, resize from the edges). Text and background colors support alpha; overlay captions fit the source text or use a fixed size
- **Control bar**: always-on-top Start / Pause / Stop bar with status, draggable. Pause stops translating and hides the translation; Stop stops and clears it
- **Hotkeys**: Start / Pause, Stop, Select region, Retry, Clear history, optionally only while the target window is focused. Off and unbound by default
- **Settings UI**: Dashboard + API / Translation / OCR / Overlay / Hotkeys. UI language English / 繁體中文 / 简体中文 (picked from the system locale on first launch). API settings can be saved as named profiles (Save / Load, same as region presets)

## Screenshots
<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<img width="100%" alt="TranslatorOverlay Demo" src="https://github.com/user-attachments/assets/7db970ab-4c72-4f3f-8e8c-ed203d62148f" />

## Requirements

- Windows 11 **x64** (21H2 / build 22000 or later)
- [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
- Microsoft Visual C++ Redistributable (x64), if the CRT is missing

## Quick start

1. Extract `TranslatorOverlay-*-win-x64.zip` (OCR DLLs are in `lib/`)
2. Double-click `translator-app.exe`
3. On the **API** page, pick a Provider:
   - **OpenAI-compatible**: fill in `base_url` / `api_key` / `model`
   - **Grok CLI** / **OpenCode CLI** / **Codex CLI** / **Claude Code**: uses the local `grok` / `opencode` / `codex` / `claude` and its existing login (Codex needs 0.154.0 or later)
4. **Save**
5. Pick a window on the **Dashboard** → **Start** at the bottom of the left navigation

If OCR models are missing on first run, they are downloaded from [oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0) into `models/` next to the exe. **Start** is unavailable until that finishes.

## Usage

Pick the target window on the **Dashboard**. Optionally use **Select regions** to draw recognition areas on that window (it brings the target to the front; drag to move or resize, right-click to remove, Esc to finish). No regions = whole window. Regions can be saved as a preset and applied later with **Load**. While translating, **Retry** translates again right away (skipping the stable wait), and **Cancel** aborts the translation in progress.

The **Overlay** page toggles the overlay, translation window, and control bar (applied immediately). Other options need **Save**; **Reload** re-reads `config.toml` from disk, and **Discard** drops unsaved edits.

On the **Hotkeys** page, turn on the master switch, then press the key for each action (Ctrl / Alt / Shift + one key; the Win key is not allowed). Press the Select region hotkey again to apply the regions.

All settings and data live next to the exe (portable):

| File | Contents |
| --- | --- |
| `config.toml` | Main settings. Invalid fields fall back to defaults; a file with a syntax error is moved aside |
| `region-presets.toml` | Region presets |
| `api-profiles.toml` | API profiles |
| `dictionaries/*.toml` | Glossary profiles |
| `translator-app.log` / `translator-app.prev.log` | Log of this run / the previous run |

**OBS:** for Window Capture, pick `Translator Overlay Captions` (overlay) or `Translator Overlay Translation` (translation window), not the control window `Translator Overlay`. Set the capture method to **Windows 10**; BitBlt does not work. Game Capture only captures the game itself, so add a Window Capture layer on top for the translation.

## Pipeline

```
Target window
  → Windows Graphics Capture
  → PP-OCRv6 (oar-ocr / ONNX + WebGPU or DirectML, falls back to CPU)
  → confidence filter / single-char filter / line merge (rules or PP-DocLayoutV3) / block persistence
  → stability gate
  → LLM translation (session cache + multi-turn context; HTTP can stream)
  → click-through overlay (follows the target window) + separate translation window
```

## OCR models

Source: [GreatV/oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0). Loading only checks that files exist; sizes are checked at download time.

| Size | Detection | Recognition | Dictionary |
| --- | --- | --- | --- |
| tiny | `pp-ocrv6_tiny_det.onnx` | `pp-ocrv6_tiny_rec.onnx` | `ppocrv6_tiny_dict.txt` |
| small (default) | `pp-ocrv6_small_det.onnx` | `pp-ocrv6_small_rec.onnx` | `ppocrv6_dict.txt` |
| medium | `pp-ocrv6_medium_det.onnx` | `pp-ocrv6_medium_rec.onnx` | `ppocrv6_dict.txt` (shared with small) |

Smaller is faster, larger is more accurate. Changing the size or device reloads on **Save**.

The **Smart detection** line merge also needs `pp-doclayoutv3_fp16.onnx` (about 65 MB, also downloaded automatically, from this repo's [models-v1](https://github.com/OKTW-Network/Translator-Overlay/releases/tag/models-v1)): the official [PaddlePaddle/PP-DocLayoutV3_onnx](https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_onnx) converted to FP16 with the unused mask output removed via `uv run scripts/optimize-doclayout.py inference.onnx pp-doclayoutv3_fp16.onnx`.

## Building from source

Requires Rust 1.95+ (edition 2024) and Visual Studio 2026. Build ONNX Runtime 1.31 (DirectML + WebGPU) once first:

```powershell
.\scripts\build-onnxruntime.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

Output: `dist/TranslatorOverlay-<version>-win-x64.zip` (`translator-app.exe` plus `onnxruntime.dll`, `DirectML.dll`, `webgpu_dawn.dll`, `dxcompiler.dll` in `lib/`). Target machines still need Windows App Runtime 2.4.

Format and lint during development:

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

## Project layout

```
crates/
  translator-app         # WinUI 3 control window (windows-reactor) and pipeline
  translator-capture     # Windows Graphics Capture
  translator-core        # config, paths, region presets, shared types
  translator-ocr         # PP-OCRv6, downloads, stability gate, filters, merge
  translator-overlay     # click-through overlay, region picker, translation window
  translator-translate   # HTTP / Grok ACP / OpenCode ACP / Codex app-server / Claude Code, session cache
```
