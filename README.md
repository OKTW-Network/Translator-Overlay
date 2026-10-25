<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# <img width="32" height="32" src="crates/translator-app/assets/icon.png"> Translator Overlay [![CI](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml/badge.svg)](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml) [![GitHub Release](https://img.shields.io/github/v/release/OKTW-Network/Translator-Overlay)](https://github.com/OKTW-Network/Translator-Overlay/releases) [![GitHub Downloads](https://img.shields.io/github/downloads/OKTW-Network/Translator-Overlay/latest/total?label=downloads&color=67ac09)](https://github.com/OKTW-Network/Translator-Overlay/releases/latest)
繁體中文 | [English](README.en.md)

Windows 桌面即時翻譯 Overlay：選取目標視窗 → 擷取畫面 → PP-OCRv6 辨識文字 → 用內建模型、OpenAI 相容 API 或本機 CLI 翻譯 → 在原位置以點擊可穿透的 Overlay 顯示譯文。

## 功能

- **視窗擷取**：Windows Graphics Capture。可在目標視窗上框選多個 OCR 區域，或辨識整窗；區域可存成具名設定集
- **本機 OCR**：PP-OCRv6（tiny / small / medium），ONNX Runtime + WebGPU 或 DirectML（失敗時回退 CPU）。畫面穩定後才送翻譯（文字一直變動時可設定強制翻譯時間）；可過濾單字元雜訊、動畫誤辨識
- **多行合併**：關閉 / 位置規則 / 智慧偵測（PP-DocLayoutV3 版面分析）/ 整區合併；分隔可選無、空白或換行；閱讀順序可選橫排或直排，直排文字一律由右至左讀
- **LLM 翻譯**：內建模型（本機執行）、OpenAI 相容 API（Chat Completions 或 Responses），或本機 Grok / OpenCode / Codex / Claude Code。串流時 Overlay 與翻譯視窗會隨譯文即時更新；可自訂 System prompt（只取代風格，輸出格式與語言不變）
- **API 選項**：Structured outputs、串流、回送 reasoning、Temperature / Top P / Max tokens / Reasoning effort、Priority 模式；逾時、重試次數與退避；Model 欄位可從 Provider 清單自動完成。翻譯失敗時控制視窗的工作列按鈕會閃爍
- **翻譯對照**：可把原文→譯文詞條存成具名設定檔並送給模型；詞條愈多 Token 消耗愈多
- **翻譯記憶與上下文**：同一句原文本會話只翻一次（LFU 快取，可調大小、手動清空，關閉程式後清空）；多輪歷史維持用語一致，可 **Clear history** 重新開始
- **顯示**：點擊可穿透的 Overlay（跟隨目標視窗與捲動的文字，目標最小化時隱藏；譯文不重疊，必要時加寬、縮小或換行）+ 獨立置頂翻譯視窗（拖曳移動、邊緣縮放）。文字與背景色可含透明度；Overlay 字級可自動貼合原文或固定大小
- **控制列**：置頂的 Start / Pause / Stop 與狀態列，可拖曳。Pause 會暫停並隱藏譯文，Stop 會停止並清除
- **快捷鍵**：Start / Pause、Stop、框選區域、Retry、Clear history，可限定只在目標視窗有焦點時生效。預設關閉、未綁定
- **設定 UI**：Dashboard + API / Translation / OCR / Overlay / Hotkeys；介面語言 English / 繁體中文 / 简体中文（首次啟動依系統語系）。API 可存成具名設定檔（Save / Load，與區域設定集相同）

## 截圖
<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<img width="100%" alt="TranslatorOverlay Demo" src="https://github.com/user-attachments/assets/7db970ab-4c72-4f3f-8e8c-ed203d62148f" />

## 系統需求

- 作業系統（**x64**）
  - 最低：Windows 10 1903（build 18362）
  - 建議：Windows 11 24H2（build 26100）以上：擷取時不顯示黃框、可限制擷取更新頻率
- [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
- Microsoft Visual C++ Redistributable（x64），若系統缺少 CRT

## 快速開始

1. 解壓 `TranslatorOverlay-*-win-x64.zip`（OCR 用的 DLL 在 `lib/`）
2. 雙擊 `translator-app.exe`
3. **API** 頁選 Provider：
   - **內建模型**（預設）：選模型與裝置即可，見[內建翻譯模型](#內建翻譯模型)
   - **OpenAI-compatible**：填 `base_url` / `api_key` / `model`
   - **Grok CLI** / **OpenCode CLI** / **Codex CLI** / **Claude Code**：本機 `grok` / `opencode` / `codex` / `claude`，沿用既有登入（Codex 需 0.154.0 或更新）
4. **Save**
5. **Dashboard** 選視窗 → 左側導覽底部 **Start**

首次執行若缺少 OCR 模型，會從 [oar-ocr v0.7.0](https://github.com/GreatV/oar-ocr/releases/tag/v0.7.0) 下載到 exe 同目錄的 `models/`；完成前無法 Start。使用內建模型時，第一次 Start 會在背景下載翻譯模型，進度顯示在狀態列。

## 使用

**Dashboard** 選目標視窗。可選 **Select regions** 在該視窗上框選辨識範圍（開啟時會把目標視窗帶到前景；拖曳移動或縮放、右鍵刪除、Esc 完成）；空區域 = 整窗。框選結果可存成設定集再 **Load**。開始翻譯後可 **Retry** 立刻再翻譯一次（略過穩定等待），或 **Cancel** 取消進行中的翻譯。

**Overlay** 頁開關 Overlay、獨立翻譯視窗與控制列（即時生效）。其餘選項改完需 **Save**；**Reload** 從磁碟重新讀取 `config.toml`，**Discard** 放棄未儲存的修改。

**Hotkeys** 頁先打開總開關，再逐項按下要綁定的按鍵（Ctrl / Alt / Shift + 一個鍵，不能用 Win 鍵）。框選區域的快捷鍵再按一次即套用。

所有設定與資料都放在 exe 同目錄（可攜式）：

| 檔案 | 內容 |
| --- | --- |
| `config.toml` | 主要設定；格式錯誤的欄位會改用預設值，語法錯誤的檔案會被移到旁邊 |
| `region-presets.toml` | 區域設定集 |
| `api-profiles.toml` | API 設定檔 |
| `dictionaries/*.toml` | 翻譯對照設定檔 |
| `translator-app.log` / `translator-app.prev.log` | 本次 / 上次執行的 log |

**OBS：** Window Capture 選 `Translator Overlay Captions`（Overlay）或 `Translator Overlay Translation`（翻譯視窗），不要選控制視窗 `Translator Overlay`。擷取方式要設成 **Windows 10**，不能用 BitBlt。Game Capture 只抓得到遊戲本身，需再加一層 Window Capture 疊譯文。

## 管線

```
目標視窗
  → Windows Graphics Capture
  → PP-OCRv6（oar-ocr / ONNX + WebGPU 或 DirectML，失敗回退 CPU）
  → 信心過濾 / 單字元過濾 / 多行合併（規則或 PP-DocLayoutV3）/ block 持續追蹤
  → 穩定閘門
  → LLM 翻譯（session cache + 多輪上下文；內建模型與 HTTP 可串流）
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

多行合併選 **智慧偵測** 時另需 `pp-doclayoutv3_fp16.onnx`（約 65 MB，同樣自動下載，來自本 repo [models-v1](https://github.com/OKTW-Network/Translator-Overlay/releases/tag/models-v1)）：官方 [PaddlePaddle/PP-DocLayoutV3_onnx](https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_onnx) 經 `uv run scripts/optimize-doclayout.py inference.onnx pp-doclayoutv3_fp16.onnx` 轉成 FP16 並移除未使用的 mask 輸出。

## 內建翻譯模型

來源：[onnx-community/gemma-4-E4B-it-ONNX](https://huggingface.co/onnx-community/gemma-4-E4B-it-ONNX)，下載到 `models/gemma-4-e4b-it/`。

| 量化 | 下載大小 |
| --- | --- |
| q4（預設） | 5.6 GB |
| q4f16 | 4.9 GB |
| int8 | 9.8 GB |

裝置可選 WebGPU（預設）或 CPU，與 OCR 的裝置分開設定。Stop 後會釋放 VRAM。

## 從原始碼建置

需要 Rust 1.95+（edition 2024）與 Visual Studio 2026。第一次先自建 ONNX Runtime 1.31（DirectML + WebGPU）：

```powershell
.\scripts\build-onnxruntime.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

產物：`dist/TranslatorOverlay-<version>-win-x64.zip`（`translator-app.exe` 與 `lib/` 內的 `onnxruntime.dll`、`DirectML.dll`、`webgpu_dawn.dll`、`dxcompiler.dll`）。目標機器仍需 Windows App Runtime 2.4。

開發時格式與 lint：

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

## 專案結構

```
crates/
  translator-app         # WinUI 3 控制視窗與管線
  translator-capture     # Windows Graphics Capture
  translator-core        # 設定、路徑、區域設定集、共用型別
  translator-ocr         # PP-OCRv6、下載、穩定閘門、過濾、合併
  translator-overlay     # 點擊可穿透的 Overlay、區域選取、翻譯視窗
  translator-translate   # 內建 ONNX 模型 / HTTP / Grok ACP / OpenCode ACP / Codex app-server / Claude Code、session cache
  translator-winui       # WinUI 3 bindings（winui-bindgen 產生）與啟動
  winui-bindgen          # 重新產生 translator-winui bindings 的工具
```
