<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# Translator Overlay
<img align="left" width="50" height="50" src="crates/translator-app/assets/icon.png">
Windows 桌面即時翻譯 Overlay：選取目標視窗 → 擷取畫面 → PP-OCRv6 辨識文字 → 經 OpenAI 相容 API 或本機 CLI 翻譯 → 在原位置以點擊可穿透的 Overlay 顯示譯文。
<br clear="left"/>

## 功能

- **視窗擷取**：Windows Graphics Capture。可在目標視窗上框選多個 OCR 區域，或辨識整窗；區域可存成具名設定集
- **本機 OCR**：PP-OCRv6（tiny / small / medium），ONNX Runtime + WebGPU 或 DirectML（失敗時回退 CPU）。畫面穩定後才送翻譯；可過濾單字元雜訊、動畫誤辨識，並合併多行
- **LLM 翻譯**：OpenAI 相容 API，或本機 Grok / OpenCode / Codex / Claude Code。串流時 Overlay 與翻譯視窗會隨譯文即時更新
- **翻譯記憶與上下文**：同一句原文本會話只翻一次（關閉程式後清空）；多輪歷史維持用語一致
- **顯示**：點擊可穿透的 Overlay（跟隨目標視窗，目標最小化時隱藏）+ 獨立置頂翻譯視窗（拖曳移動、邊緣縮放）
- **設定 UI**：Dashboard + API / Translation / OCR / Overlay。API 可存成具名設定檔（Save / Load，與區域設定集相同）

## 截圖
<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<div align="center">
   <img width="48%" alt="TranslatorOverlay Screenshot Overlay" src="https://github.com/user-attachments/assets/e4b389af-857e-4761-a517-478343672db7" />
   &nbsp;
   <img width="48%" alt="TranslatorOverlay Screenshot Overlay Select" src="https://github.com/user-attachments/assets/f53b6f66-d529-4d56-ad84-df72749d4fd2" />
</div>

## 系統需求

- Windows 11 **x64**（build 22000 以上）
- [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
- Microsoft Visual C++ Redistributable（x64），若系統缺少 CRT

## 快速開始

1. 解壓 `TranslatorOverlay-*-win-x64.zip`（OCR 用的 DLL 在 `lib/`）
2. 雙擊 `translator-app.exe`
3. **API** 頁選 Provider：
   - **OpenAI-compatible**：填 `base_url` / `api_key` / `model`
   - **Grok CLI** / **OpenCode CLI** / **Codex CLI** / **Claude Code**：本機 `grok` / `opencode` / `codex` / `claude`，沿用既有登入（Codex 需 0.154.0 或更新）
4. **Save**
5. **Dashboard** 選視窗 → 左側導覽底部 **Start**

首次執行若缺少 OCR 模型，會從 [oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0) 下載到 exe 同目錄的 `models/`；完成前無法 Start。

## 使用

**Dashboard** 選目標視窗。可選 **Select regions** 在該視窗上框選辨識範圍（開啟時會把目標視窗帶到前景）；空區域 = 整窗。框選結果可存成設定集再 **Load**。擷取中可 **Retry** 立刻 OCR + 翻譯（略過穩定等待），或 **Cancel** 取消進行中的翻譯。

**Overlay** 頁開關 Overlay 與獨立翻譯視窗（即時生效）。其餘選項改完需 **Save**。

**OBS：** Window Capture 選 `Translator Overlay Captions`，不要選控制視窗 `Translator Overlay`。Game Capture 只抓得到遊戲本身，需再加一層 Window Capture 疊譯文。

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

來源：[GreatV/oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0)。載入時只檢查檔案是否存在，大小在下載時檢查。

| 尺寸 | Detection | Recognition | Dictionary |
| --- | --- | --- | --- |
| tiny | `pp-ocrv6_tiny_det.onnx` | `pp-ocrv6_tiny_rec.onnx` | `ppocrv6_tiny_dict.txt` |
| small（預設） | `pp-ocrv6_small_det.onnx` | `pp-ocrv6_small_rec.onnx` | `ppocrv6_dict.txt` |
| medium | `pp-ocrv6_medium_det.onnx` | `pp-ocrv6_medium_rec.onnx` | `ppocrv6_dict.txt`（與 small 共用） |

較小較快，較大較準。換尺寸或裝置後 **Save** 會重新載入。

## 從原始碼建置

需要 Rust 1.95+（edition 2024）與 Visual Studio 2026。第一次先自建 ONNX Runtime 1.30（DirectML + WebGPU）：

```powershell
.\scripts\build-onnxruntime.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

產物：`dist/TranslatorOverlay-<version>-win-x64.zip`（`translator-app.exe` 與 `lib/` 內的 `onnxruntime.dll`、`DirectML.dll`、`webgpu_dawn.dll`、`dxcompiler.dll`、`dxil.dll`）。目標機器仍需 Windows App Runtime 2.4。

開發時格式與 lint：

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
