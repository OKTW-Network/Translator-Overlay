<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# Translator Overlay
<img align="left" width="50" height="50" src="crates/translator-app/assets/icon.png">
Windows 桌面即時翻譯 Overlay。選好目標視窗後，程式會擷取畫面，用 PP-OCRv6 辨識文字，再交給 OpenAI 相容 API 或本機 CLI 翻譯，最後把譯文疊在原文位置上。這層 Overlay 不攔截滑鼠點擊。
<br clear="left"/>

## 功能

- 用 Windows Graphics Capture 擷取視窗。可以在目標視窗上框選多個 OCR 區域，也可以辨識整個視窗，框好的區域能存成具名設定集。
- 在本機跑 PP-OCRv6（tiny、small、medium），使用 ONNX Runtime 搭配 WebGPU 或 DirectML，兩者都失敗時改用 CPU。畫面穩定後才送去翻譯，並會濾掉單字元雜訊與動畫造成的誤辨識，也會把多行合併成段落。
- 翻譯可走 OpenAI 相容 API，或本機的 Grok、OpenCode、Codex、Claude Code。串流回應時，Overlay 與翻譯視窗會跟著譯文即時更新。
- 同一句原文在一次執行中只翻一次，關閉程式後這份記憶就清空。多輪對話歷史讓前後用語保持一致。
- Overlay 不攔截點擊，會跟著目標視窗移動，目標最小化時隱藏。另外還有一個獨立的置頂翻譯視窗，可以拖曳移動、拉邊緣縮放。
- 設定介面分成 Dashboard、API、Translation、OCR、Overlay 幾頁。API 設定可以存成具名設定檔，存取方式和區域設定集一樣。

## 截圖
<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<div align="center">
   <img width="48%" alt="TranslatorOverlay Screenshot Overlay" src="https://github.com/user-attachments/assets/e4b389af-857e-4761-a517-478343672db7" />
   &nbsp;
   <img width="48%" alt="TranslatorOverlay Screenshot Overlay Select" src="https://github.com/user-attachments/assets/f53b6f66-d529-4d56-ad84-df72749d4fd2" />
</div>

## 系統需求

- Windows 11 x64，build 22000 以上
- [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
- 系統缺少 CRT 時，需要 Microsoft Visual C++ Redistributable（x64）

## 快速開始

1. 解壓 `TranslatorOverlay-*-win-x64.zip`。OCR 用的 DLL 放在 `lib/`。
2. 雙擊 `translator-app.exe`。
3. 在 **API** 頁選 Provider。
   - 選 **OpenAI-compatible** 時，填入 `base_url`、`api_key`、`model`。
   - 選 **Grok CLI**、**OpenCode CLI**、**Codex CLI** 或 **Claude Code** 時，會使用本機的 `grok`、`opencode`、`codex` 或 `claude`，沿用它們原本的登入。Codex 需要 0.154.0 或更新版本。
4. 按 **Save**。
5. 在 **Dashboard** 選視窗，再按左側導覽底部的 **Start**。

第一次執行時若缺少 OCR 模型，程式會從 [oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0) 下載到 exe 同目錄的 `models/`，下載完成前無法 Start。

## 使用

在 **Dashboard** 選目標視窗。想限定辨識範圍時，按 **Select regions** 直接在該視窗上框選，開啟時會把目標視窗帶到前景。沒有框選區域就辨識整個視窗。框好的區域可以存成設定集，之後用 **Load** 載回。擷取期間按 **Retry** 會立刻重新 OCR 並翻譯，不等畫面穩定；按 **Cancel** 會取消進行中的翻譯。

**Overlay** 頁的 Overlay 與獨立翻譯視窗開關會立即生效，其他選項改完要按 **Save**。

用 OBS 時，Window Capture 請選 `Translator Overlay Captions`，不要選控制視窗 `Translator Overlay`。Game Capture 只抓得到遊戲畫面本身，要再加一層 Window Capture 才能疊上譯文。

## 管線

```
目標視窗
  → Windows Graphics Capture
  → PP-OCRv6（oar-ocr / ONNX + WebGPU 或 DirectML，失敗回退 CPU）
  → 信心過濾 / 單字元過濾 / 多行合併 / block 持續追蹤
  → 穩定閘門
  → LLM 翻譯（session cache + 多輪上下文；HTTP 可串流）
  → 點擊可穿透的 Overlay（跟隨目標視窗）+ 獨立翻譯視窗
```

## OCR 模型

模型來自 [GreatV/oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0)。載入時只檢查檔案是否存在，檔案大小在下載時檢查。

| 尺寸 | Detection | Recognition | Dictionary |
| --- | --- | --- | --- |
| tiny | `pp-ocrv6_tiny_det.onnx` | `pp-ocrv6_tiny_rec.onnx` | `ppocrv6_tiny_dict.txt` |
| small（預設） | `pp-ocrv6_small_det.onnx` | `pp-ocrv6_small_rec.onnx` | `ppocrv6_dict.txt` |
| medium | `pp-ocrv6_medium_det.onnx` | `pp-ocrv6_medium_rec.onnx` | `ppocrv6_dict.txt`（與 small 共用） |

模型越小越快，越大越準。換了尺寸或裝置後按 **Save**，引擎會重新載入。

## 從原始碼建置

需要 Rust 1.95 以上（edition 2024）與 Visual Studio 2026。第一次建置前，要先自行編譯含 DirectML 與 WebGPU 的 ONNX Runtime 1.30。

```powershell
.\scripts\build-onnxruntime.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

產物是 `dist/TranslatorOverlay-<version>-win-x64.zip`，裡面有 `translator-app.exe`，以及 `lib/` 內的 `onnxruntime.dll`、`DirectML.dll`、`webgpu_dawn.dll`、`dxcompiler.dll`、`dxil.dll`。目標機器仍需要安裝 Windows App Runtime 2.4。

開發時用這兩個指令檢查格式與 lint。

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

## 專案結構

```
crates/
  translator-app         # WinUI 3 控制視窗（windows-reactor）與管線
  translator-capture     # Windows Graphics Capture
  translator-core        # 設定、路徑、區域設定集、共用型別
  translator-ocr         # PP-OCRv6、下載、穩定閘門、過濾、合併
  translator-overlay     # 點擊可穿透的 Overlay、區域選取、翻譯視窗
  translator-translate   # HTTP / Grok ACP / OpenCode ACP / Codex app-server / Claude Code、session cache
```
