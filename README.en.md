<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# <img width="32" height="32" src="crates/translator-app/assets/icon.png"> Translator Overlay [![CI](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml/badge.svg)](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml) [![GitHub Release](https://img.shields.io/github/v/release/OKTW-Network/Translator-Overlay)](https://github.com/OKTW-Network/Translator-Overlay/releases) [![GitHub Downloads](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2FOKTW-Network%2FTranslator-Overlay%2Fbadges%2Fdownloads.json)](https://github.com/OKTW-Network/Translator-Overlay/releases/latest)

[繁體中文](README.md) | English

Real-time translation overlay for the Windows desktop: pick a target window → capture it → recognize text locally with PP-OCRv6 / PP-OCRv5 → translate with a built-in local model, an OpenAI-compatible API, or a local CLI → render translations in place on a click-through overlay.

## Screenshots

<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<img width="100%" alt="TranslatorOverlay Demo" src="https://github.com/user-attachments/assets/7db970ab-4c72-4f3f-8e8c-ed203d62148f" />

## Features

### Capture & Local OCR

- **Window & region capture**: Captures any chosen window via Windows Graphics Capture. Draw multiple OCR regions and save them as named presets, or leave regions empty to scan the whole window
- **Multi-model local OCR**: Ships with PP-OCRv6 general models (tiny / small / medium) and 10 PP-OCRv5 single-script models. Runs on ONNX Runtime with WebGPU or DirectML acceleration (automatic CPU fallback) and a configurable detection resolution limit
- **Noise filtering & stability gate**: Filters low-confidence hits, lone single characters, and animated text misreads. Waits for text to settle before translating, with optional max force-translate wait, wait-for-all-text-to-settle, and hold-after-gone timing
- **Flexible line merge**: Off, Position rules, Smart detection (PP-DocLayoutV3 layout analysis), or Merge all. Configurable separator (none / space / line break) and horizontal or vertical (right-to-left) reading order

### LLM Translation

- **Multiple backends**:
  - **Built-in model** (default): Fully local inference using GGUF (`llama.cpp`, Vulkan GPU / CPU) or ONNX (`ONNX Runtime`, WebGPU / CPU), plus support for your own custom GGUF files
  - **OpenAI-compatible API**: Supports both `/chat/completions` and `/responses` with streaming, provider model-list autocomplete, Structured Outputs, reasoning replay, sampling / reasoning-effort controls, timeouts, and exponential backoff retries
  - **Local CLI**: Grok / OpenCode / Codex (with Priority mode) / Claude Code, reusing your existing CLI logins
- **Reply mode**: Choose **Structured** (translate all blocks in one JSON call) or **Plain text** (one block at a time, suited for models weak at JSON)
- **Consistency & accuracy**: Session LFU translation cache (avoids re-translating identical lines), multi-turn conversation history, named glossary profiles, customizable system prompt style, plus **Repeat instruction** and **Repeat glossary** reminders for smaller models
- **Live streaming & error feedback**: Built-in models and HTTP APIs stream translations directly to the overlay and translation window; the control window's taskbar button flashes on translation failure

### Display & Controls

- **Dual display modes**: Click-through **In-place overlay** (tracks the target window and scrolled text, hides when minimized, prevents overlapping captions, fits source size or uses a fixed font size) + an independent always-on-top **Translation window** (draggable and resizable). Text and backgrounds support `#AARRGGBB` alpha colors
- **Control bar & hotkeys**: Optional always-on-top draggable control bar (Start / Pause / Stop + status) and configurable hotkeys (global or only while the target window is focused)
- **WinUI 3 settings UI**: Dashboard, API, Translation, OCR, Overlay, and Hotkeys pages with named API profiles and region presets. Localized in English, 繁體中文, and 简体中文 (auto-selected from system locale on first launch)

## Requirements

- **Operating system (x64)**
  - Minimum: Windows 10 1903 (build 18362)
  - Recommended: Windows 11 24H2 (build 26100) or later (no yellow capture border and throttled capture updates)
- **Runtime**
  - [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
  - Microsoft Visual C++ Redistributable (x64), if the CRT is missing
- **Estimated memory** (the settings pages compute live estimates for your current configuration)

| Component (default setup) | Est. VRAM | Est. RAM |
| --- | --- | --- |
| OCR: PP-OCRv6 small (GPU, 1280 px) | ~700 MB | ~256 MB |
| OCR: PP-OCRv6 medium (GPU, 1280 px) | ~1.6 GB | ~256 MB |
| Line merge: Smart detection (PP-DocLayoutV3) | +~1 GB | — |
| Built-in model: Gemma 4 E4B Q4_K_XL (GPU, context 4096) | ~2.9 GB | ~2.1 GB |

## Quick start

1. Download and extract `TranslatorOverlay-*-win-x64.zip` from [GitHub Releases](https://github.com/OKTW-Network/Translator-Overlay/releases/latest) (runtime DLLs live in `lib/`)
2. Launch `translator-app.exe`
3. Open the **API** page and choose a Provider:
   - **Built-in model** (default): pick a model and device; see [Built-in translation models](#built-in-translation-models)
   - **OpenAI-compatible**: fill in `Base URL`, `API key`, and `Model`
   - **Grok CLI** / **OpenCode CLI** / **Codex CLI** / **Claude Code**: uses local `grok` / `opencode` / `codex` / `claude` and its existing login (Codex requires `0.154.0` or later)
4. Click **Save**
5. Select a target window on the **Dashboard** and click **Start** at the bottom of the left navigation

> [!NOTE]
> OCR and built-in translation models load only after you click **Start**. Missing files are automatically downloaded into `models/` next to the exe with progress shown in the status bar. **Stop** waits until models are fully unloaded and VRAM is freed.

## Usage

### Basic workflow

- **Target window & regions**: Pick a window on the **Dashboard**. Optionally click **Select regions** to draw OCR boxes on the target window (brings it to the front; drag to move or resize, right-click to remove, press `Esc` or click **Done** to finish). No regions = whole window. Save regions as named presets to reuse with **Load**.
- **Translation controls**: While active, **Retry** translates immediately (skipping the settle wait), **Cancel** aborts the in-flight request, and **Clear history** resets multi-turn context. **Pause** pauses translation and hides captions; **Stop** stops and clears them.
- **Applying settings**: Display toggles on the **Overlay** page apply immediately; other changes require **Save**. **Reload** re-reads `config.toml` from disk, and **Discard** reverts unsaved edits.

### Hotkeys

Turn on the master switch on the **Hotkeys** page, then press a key chord for each action (`Ctrl` / `Alt` / `Shift` + one key; the `Win` key is disallowed). Available actions: **Start / Pause**, **Stop**, **Select region**, **Retry**, and **Clear history**, optionally restricted to when the target window is focused. Pressing the Select region hotkey again applies the selection.

### Capturing with OBS

When adding a **Window Capture** source in OBS:
- Pick `Translator Overlay Captions` (in-place overlay) or `Translator Overlay Translation` (translation window) — **not** the control window `Translator Overlay`.
- Set the capture method to **Windows 10** (BitBlt cannot capture the overlay).
- **Game Capture** only captures the game itself; stack a Window Capture layer above it to include translations.

### Portable files

All configuration, models, and logs live next to `translator-app.exe`:

| Path | Contents |
| --- | --- |
| `config.toml` | Main settings. Invalid fields fall back to defaults; a file with syntax errors is moved aside |
| `region-presets.toml` | Named OCR region presets |
| `api-profiles.toml` | Named API profiles |
| `dictionaries/*.toml` | Named glossary profiles |
| `models/` | Auto-downloaded OCR and built-in models (place custom GGUF files in `models/custom/`) |
| `translator-app.log` / `translator-app.prev.log` | Logs for the current and previous runs |

## OCR models

Model files come from [PaddlePaddle](https://huggingface.co/PaddlePaddle)'s official Hugging Face ONNX repos (`<model>_onnx`) and are downloaded automatically when missing (byte lengths are verified on download; loading checks file existence). Changing the model, size, resolution limit, or device reloads on **Save**.

### PP-OCRv6 General (default)

Smaller is faster; larger is more accurate:

| Size | Detection | Recognition | Dictionary |
| --- | --- | --- | --- |
| tiny | `PP-OCRv6_tiny_det.onnx` | `PP-OCRv6_tiny_rec.onnx` | `PP-OCRv6_tiny_rec.yml` |
| small (default) | `PP-OCRv6_small_det.onnx` | `PP-OCRv6_small_rec.onnx` | `PP-OCRv6_small_rec.yml` |
| medium | `PP-OCRv6_medium_det.onnx` | `PP-OCRv6_medium_rec.onnx` | `PP-OCRv6_medium_rec.yml` |

### PP-OCRv5 single-script models

When the general model struggles with a specific script, switch to a dedicated PP-OCRv5 model (recognizes only that script): English, Latin, Korean, Cyrillic, East Slavic, Greek, Devanagari, Tamil, Telugu, or Thai.
- Detection size: **Mobile** (default, `PP-OCRv5_mobile_det.onnx`, ~5 MB) or **Server** (`PP-OCRv5_server_det.onnx`, ~84 MB; more accurate, but significantly slower and uses more memory)
- Recognition & dictionary: `<script>_PP-OCRv5_mobile_rec.onnx` + `.yml` (~8–13 MB)

### Layout model (Smart detection)

Selecting **Smart detection** line merge automatically downloads `pp-doclayoutv3_fp16.onnx` (~65 MB) from this repo's [models-v1](https://github.com/OKTW-Network/Translator-Overlay/releases/tag/models-v1) release — the official [PaddlePaddle/PP-DocLayoutV3_onnx](https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_onnx) converted to FP16 with unused mask outputs stripped via `uv run scripts/optimize-doclayout.py inference.onnx pp-doclayoutv3_fp16.onnx`.

## Built-in translation models

Built-in models are available in two formats:
- **GGUF**: Runs on `llama.cpp` (Vulkan GPU / CPU), typically faster
- **ONNX**: Runs on `ONNX Runtime` (WebGPU / CPU), broader system compatibility

The compute device (GPU / CPU) is configured independently from OCR, along with context length, reply length ratio, and DRY repetition penalty. **Stop** releases VRAM.

### Model catalog

Download sizes are shown in parentheses; each family downloads into its own subdirectory under `models/`:

| Model | GGUF (llama.cpp) | ONNX (ONNX Runtime) |
| --- | --- | --- |
| Gemma 4 E4B | Q4_K_XL (4.2 GB, default) | q4f16 (4.3 GB), q4 (4.8 GB) |
| Gemma 4 12B | Q4_K_XL (6.7 GB) | q4f16 (6.8 GB), q4 (7.6 GB) |
| Gemma 4 26B A4B | Q4_K_XL (14.2 GB) | q4f16 (14.3 GB), q4 (15.9 GB) |
| Gemma 4 31B | Q4_K_XL (17.3 GB) | — |
| Hy-MT2 1.8B | Q4_K_M (1.1 GB), Q6_K (1.5 GB), Q8_0 (1.9 GB) | q4f16 (1.4 GB), fp16 (3.6 GB) |
| Hy-MT2 7B | Q4_K_M (4.6 GB), Q6_K (6.2 GB), Q8_0 (8.0 GB) | q4f16 (5.0 GB), fp16 (15.0 GB) |
| Hy-MT2 30B A3B | Q4_K_M (18.2 GB), Q8_0 (32.0 GB) | q4f16 (16.3 GB) |
| Qwen3.8 27B | Q4_K_M (16.6 GB) | q4f16 (15.2 GB) |

Sources:
- **GGUF**: Publisher releases from [Unsloth](https://huggingface.co/unsloth) (Gemma 4 QAT, Qwen3.8 27B) and [Tencent](https://huggingface.co/tencent) (Hy-MT2)
- **ONNX**: [Gemma 4 QAT ONNX](https://huggingface.co/collections/james58899/gemma-4-qat-onnx) (from Google's QAT checkpoints), [Hy-MT2 ONNX](https://huggingface.co/collections/james58899/hy-mt2-onnx) (fp16 from Tencent's weights; q4f16 from Intel's AutoRound int4 checkpoints), and [Qwen3.8 27B 3-bit GSQ ONNX](https://huggingface.co/james58899/Qwen3.8-27B-3Bit-GSQ-ONNX) (from ISTA-DASLab's 3-bit GSQ checkpoint)

### Custom GGUF

You can run any external GGUF model (including multi-part splits `*-00001-of-0000N.gguf`):

1. Place `.gguf` files into `models/custom/` next to the exe (created on first launch)
2. On the **API** page, pick **Custom GGUF** as the built-in model, click the refresh button, and choose your file
3. Click **Save**

The GGUF file must embed a chat template (most published GGUFs do). Unset sampling parameters fall back to `llama.cpp` defaults.

## Pipeline

```text
Target window
  → Windows Graphics Capture
  → PP-OCRv6 / PP-OCRv5 (ONNX Runtime + WebGPU / DirectML, falls back to CPU)
  → confidence filter / single-char filter / line merge (rules or PP-DocLayoutV3) / block persistence
  → stability gate
  → LLM translation (session cache + multi-turn context; streaming for built-in models and HTTP)
  → click-through overlay (follows target window) + independent translation window
```

## Building from source

Requires **Rust 1.100+** (edition 2024) and **Visual Studio 2026**. First build ONNX Runtime 1.31 (DirectML + WebGPU) and fetch prebuilt `llama.cpp` DLLs once:

```powershell
.\scripts\build-onnxruntime.ps1
.\scripts\fetch-llama.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

Output: `dist/TranslatorOverlay-<version>-win-x64.zip`, containing `translator-app.exe`, ONNX Runtime / DirectML / WebGPU DLLs in `lib/`, and `llama.cpp` DLLs in `lib/llama.cpp/`. Target machines still require Windows App Runtime 2.4.

Format and lint during development:

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

## Project layout

```text
crates/
  translator-app         # WinUI 3 control window, settings UI, and pipeline orchestration
  translator-capture     # Windows Graphics Capture and window tracking
  translator-core        # config, paths, region presets, API profiles, glossaries, shared types
  translator-ocr         # PP-OCRv6 / PP-OCRv5, downloads, stability gate, filters, line merge
  translator-overlay     # click-through overlay, region picker, independent translation window
  translator-translate   # built-in models (GGUF / ONNX), HTTP, Grok / OpenCode / Codex / Claude CLI, cache
  translator-winui       # WinUI 3 bindings (generated by winui-bindgen) and startup
  winui-bindgen          # regenerates translator-winui bindings
```

## License

Released into the public domain under [The Unlicense](LICENSE).
