<img width="960" height="200" alt="AI Slop" src="https://github.com/user-attachments/assets/9d681a9b-7548-43e6-9cff-8d01d9f00f54" />

# <img width="32" height="32" src="crates/translator-app/assets/icon.png"> Translator Overlay [![CI](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml/badge.svg)](https://github.com/OKTW-Network/Translator-Overlay/actions/workflows/ci.yml) [![GitHub Release](https://img.shields.io/github/v/release/OKTW-Network/Translator-Overlay)](https://github.com/OKTW-Network/Translator-Overlay/releases) [![GitHub Downloads](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2FOKTW-Network%2FTranslator-Overlay%2Fbadges%2Fdownloads.json)](https://github.com/OKTW-Network/Translator-Overlay/releases/latest)

繁體中文 | [English](README.en.md)

Windows 桌面即時翻譯 Overlay：選取目標視窗 → 擷取畫面 → 以本機 PP-OCRv6 / PP-OCRv5 辨識文字 → 透過內建本機模型、OpenAI 相容 API 或本機 CLI 翻譯 → 在原文位置以點擊可穿透的 Overlay 顯示譯文。

## 截圖

<img width="100%" alt="TranslatorOverlay Screenshot UI" src="https://github.com/user-attachments/assets/4a03a129-3a99-4ab2-8b64-89591473f5ce" />
<img width="100%" alt="TranslatorOverlay Demo" src="https://github.com/user-attachments/assets/7db970ab-4c72-4f3f-8e8c-ed203d62148f" />

## 功能特色

### 擷取與本機 OCR

- **視窗與區域擷取**：透過 Windows Graphics Capture 擷取指定視窗；可框選多個辨識區域並存成具名設定集，或不設區域辨識整窗
- **多模型本機 OCR**：支援 PP-OCRv6 通用模型（tiny / small / medium）與 10 種 PP-OCRv5 單一語系模型；透過 ONNX Runtime 使用 WebGPU 或 DirectML 加速（失敗自動回退 CPU），並可設定偵測解析度上限
- **雜訊過濾與穩定閘門**：可過濾低信心、單字元與動畫誤辨識；等畫面文字穩定後才送翻譯，支援最長強制等待、等待全部文字穩定與文字消失後保留時間
- **彈性多行合併**：支援關閉、位置規則、智慧偵測（PP-DocLayoutV3 版面分析）與全部合併；可自訂間隔符（無 / 空白 / 換行）及橫排／直排（由右至左）閱讀順序

### LLM 翻譯引擎

- **多種翻譯後端**：
  - **內建模型**（預設）：本機離線執行，支援 GGUF（llama.cpp，Vulkan GPU / CPU）與 ONNX（ONNX Runtime，WebGPU / CPU），亦可載入自訂 GGUF 檔
  - **OpenAI 相容 API**：支援 `/chat/completions` 與 `/responses`，提供串流、Model 清單自動完成、結構化輸出、推理內容回送、取樣與推理強度參數、逾時與指數退避重試
  - **本機 CLI**：支援 Grok / OpenCode / Codex（含優先模式）/ Claude Code，沿用本機既有登入
- **回應模式**：可切換**結構化**（單次 JSON 翻譯所有區塊）或**純文字**（逐區塊翻譯，適合不擅長 JSON 的模型）
- **一致與準確性**：內建工作階段 LFU 翻譯快取（避免重複翻譯同一句）、多輪對話歷史、翻譯對照表（詞彙設定檔）、自訂系統提示風格，以及針對小模型的**重複指令**與**重複翻譯對照**提醒
- **即時串流與錯誤提醒**：內建模型與 HTTP API 支援串流，譯文邊生成邊更新；翻譯失敗時控制視窗的工作列按鈕會閃爍提示

### 顯示與操作

- **雙顯示模式**：點擊可穿透的原位 **Overlay**（自動跟隨目標視窗與捲動文字、目標最小化時隱藏、譯文防重疊並自動調整排版，字級可貼合原文或固定）＋獨立置頂的**翻譯視窗**（可拖曳與縮放）；文字與背景皆支援透明色（`#AARRGGBB`）
- **浮動控制列與快捷鍵**：可開啟置頂浮動控制列（開始 / 暫停 / 停止與狀態）；支援全域或僅目標視窗聚焦時觸發的快捷鍵
- **WinUI 3 設定介面**：包含儀表板、API、翻譯、OCR、Overlay、快捷鍵等頁面，支援 API 設定檔與區域設定集管理，介面提供繁體中文 / 简体中文 / English（首次啟動依系統語系決定）

## 系統需求

- **作業系統（x64）**
  - 最低：Windows 10 1903（build 18362）
  - 建議：Windows 11 24H2（build 26100）以上（擷取時不顯示黃框，且支援限制擷取更新頻率）
- **執行階段**
  - [Windows App Runtime 2.4](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads)
  - Microsoft Visual C++ Redistributable（x64），若系統缺少 CRT
- **預估記憶體用量**（設定頁會依目前選項即時計算顯示）

| 項目（預設組態） | 預估 VRAM | 預估 RAM |
| --- | --- | --- |
| OCR：PP-OCRv6 small（GPU、1280 px） | 約 700 MB | 約 256 MB |
| OCR：PP-OCRv6 medium（GPU、1280 px） | 約 1.6 GB | 約 256 MB |
| 多行合併：智慧偵測（PP-DocLayoutV3） | 另加約 1 GB | — |
| 內建翻譯模型：Gemma 4 E4B Q4_K_XL（GPU、上下文 4096） | 約 2.9 GB | 約 2.1 GB |

## 快速開始

1. 從 [GitHub Releases](https://github.com/OKTW-Network/Translator-Overlay/releases/latest) 下載並解壓 `TranslatorOverlay-*-win-x64.zip`（執行階段 DLL 位於 `lib/`）
2. 執行 `translator-app.exe`
3. 到 **API** 頁選擇提供者：
   - **內建模型**（預設）：選擇模型與裝置即可，詳見[內建翻譯模型](#內建翻譯模型)
   - **OpenAI-compatible**：填入 `Base URL`、`API 金鑰` 與 `模型`
   - **Grok CLI** / **OpenCode CLI** / **Codex CLI** / **Claude Code**：呼叫本機 `grok` / `opencode` / `codex` / `claude`，沿用既有登入（Codex 需 `0.154.0` 或更新版本）
4. 按下 **儲存**
5. 切回 **儀表板** 選擇目標視窗，點選左側導覽底部的 **開始**

> [!NOTE]
> OCR 與內建翻譯模型皆在按下 **開始** 後才載入；缺少時會自動下載至 `exe` 同目錄的 `models/` 並於狀態列顯示進度。按下 **停止** 會等模型完整卸載並釋放 VRAM。

## 使用說明

### 基本操作

- **選取視窗與區域**：在 **儀表板** 選擇目標視窗；點選 **選取區域** 可在目標視窗上框選辨識範圍（自動置前；拖曳移動或縮放、右鍵刪除、按 `Esc` 或點 **完成** 結束）。未框選區域時預設辨識整窗，框選結果可存成設定集重複 **載入**。
- **翻譯控制**：開始後可按 **重試** 略過穩定等待立刻重翻、按 **取消** 中斷進行中的翻譯，或按 **清除歷史** 重設多輪上下文。**暫停** 會暫停翻譯並隱藏譯文；**停止** 會停止翻譯並清除譯文。
- **套用設定**：**Overlay** 頁的顯示開關即時生效；其餘設定需按 **儲存** 套用。**重新載入** 會自磁碟重讀 `config.toml`，**捨棄** 則還原未儲存的修改。

### 快捷鍵

至 **快捷鍵** 頁開啟總開關，依序按下欲綁定的組合鍵（`Ctrl` / `Alt` / `Shift` + 單一按鍵，不可使用 `Win` 鍵），可綁定「開始 / 暫停」、「停止」、「選取區域」、「重試」、「清除歷史」，並可限定只在目標視窗取得焦點時觸發。再次按下「選取區域」快捷鍵即可直接套用框選範圍。

### OBS 擷取

在 OBS 新增 **視窗擷取（Window Capture）** 時：
- 視窗請選 `Translator Overlay Captions`（原位 Overlay）或 `Translator Overlay Translation`（獨立翻譯視窗），**不要**選控制視窗 `Translator Overlay`。
- 擷取方式請選 **Windows 10**（BitBlt 無法擷取透明疊層）。
- 若使用 **遊戲擷取（Game Capture）**，因只會擷取遊戲本體，請額外疊加一層視窗擷取顯示譯文。

### 可攜式資料檔案

所有設定、模型與紀錄都存放在 `translator-app.exe` 同目錄下：

| 路徑 | 說明 |
| --- | --- |
| `config.toml` | 主要設定；單一欄位格式錯誤會退回預設值，語法錯誤的檔案會被移至一旁備份 |
| `region-presets.toml` | 區域設定集 |
| `api-profiles.toml` | API 設定檔 |
| `dictionaries/*.toml` | 翻譯對照設定檔 |
| `models/` | 自動下載的 OCR 與內建翻譯模型（自訂 GGUF 放於 `models/custom/`） |
| `translator-app.log` / `translator-app.prev.log` | 本次與上次執行的紀錄檔 |

## OCR 模型

模型檔案來自 [PaddlePaddle](https://huggingface.co/PaddlePaddle) 官方 Hugging Face ONNX 儲存庫（`<模型>_onnx`），缺少時自動下載（下載時驗證檔案大小，載入時檢查檔案存在）。切換模型、尺寸、解析度上限或裝置後，按 **儲存** 即會重新載入。

### PP-OCRv6 通用模型（預設）

較小較快，較大較準：

| 尺寸 | Detection | Recognition | Dictionary |
| --- | --- | --- | --- |
| tiny | `PP-OCRv6_tiny_det.onnx` | `PP-OCRv6_tiny_rec.onnx` | `PP-OCRv6_tiny_rec.yml` |
| small（預設） | `PP-OCRv6_small_det.onnx` | `PP-OCRv6_small_rec.onnx` | `PP-OCRv6_small_rec.yml` |
| medium | `PP-OCRv6_medium_det.onnx` | `PP-OCRv6_medium_rec.onnx` | `PP-OCRv6_medium_rec.yml` |

### PP-OCRv5 單一語系模型

當通用模型辨識特定語系不夠準確時，可改選單一語系模型（僅辨識該語系）：英文、拉丁語系、韓文、西里爾語系、東斯拉夫語、希臘文、天城文、坦米爾文、泰盧固文、泰文。
- 偵測模型可選 **Mobile**（預設，`PP-OCRv5_mobile_det.onnx`，約 5 MB）或 **Server**（`PP-OCRv5_server_det.onnx`，約 84 MB，較準但較慢且記憶體用量高）
- 辨識模型與字典為 `<語系>_PP-OCRv5_mobile_rec.onnx` + `.yml`（約 8–13 MB）

### 版面分析模型（智慧偵測）

多行合併選用 **智慧偵測** 時，會自動從本專案 [models-v1](https://github.com/OKTW-Network/Translator-Overlay/releases/tag/models-v1) 下載 `pp-doclayoutv3_fp16.onnx`（約 65 MB）。該檔由官方 [PaddlePaddle/PP-DocLayoutV3_onnx](https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_onnx) 經 `uv run scripts/optimize-doclayout.py inference.onnx pp-doclayoutv3_fp16.onnx` 轉為 FP16 並移除未使用的 mask 輸出。

## 內建翻譯模型

內建模型可依格式選擇執行引擎：
- **GGUF**：以 `llama.cpp`（Vulkan GPU / CPU）執行，通常速度較快
- **ONNX**：以 `ONNX Runtime`（WebGPU / CPU）執行，系統相容性較佳

裝置（GPU / CPU）與 OCR 裝置分開設定，並可調整上下文長度、回覆長度倍率與重複懲罰（DRY）。按下 **停止** 後會釋放 VRAM。

Gemma 4、Hy-MT2 30B A3B 與 Qwen3.8 27B 可透過 **推理強度** 開啟思考（預設關閉）；思考期間狀態列顯示「思考中」，思考內容不會出現在譯文中。

### 模型目錄

括號內為下載大小，各自下載至 `models/` 下的子目錄：

| 模型 | GGUF（llama.cpp） | ONNX（ONNX Runtime） |
| --- | --- | --- |
| Gemma 4 E4B | Q4_K_XL (4.2 GB，預設) | q4f16 (4.3 GB)、q4 (4.8 GB) |
| Gemma 4 12B | Q4_K_XL (6.7 GB) | q4f16 (6.8 GB)、q4 (7.6 GB) |
| Gemma 4 26B A4B | Q4_K_XL (14.2 GB) | q4f16 (14.3 GB)、q4 (15.9 GB) |
| Gemma 4 31B | Q4_K_XL (17.3 GB) | — |
| Hy-MT2 1.8B | Q4_K_M (1.1 GB)、Q6_K (1.5 GB)、Q8_0 (1.9 GB) | q4f16 (1.4 GB)、fp16 (3.6 GB) |
| Hy-MT2 7B | Q4_K_M (4.6 GB)、Q6_K (6.2 GB)、Q8_0 (8.0 GB) | q4f16 (5.0 GB)、fp16 (15.0 GB) |
| Hy-MT2 30B A3B | Q4_K_M (18.2 GB)、Q8_0 (32.0 GB) | q4f16 (16.3 GB) |
| Qwen3.8 27B | IQ3_S (11.8 GB) | q4f16 (15.2 GB) |

模型來源：
- **GGUF**：[Unsloth](https://huggingface.co/unsloth)（Gemma 4 QAT）與 [Tencent](https://huggingface.co/tencent)（Hy-MT2）官方發布檔、[ISTA-DASLab](https://huggingface.co/ISTA-DASLab) GSQ-RCO（Qwen3.8 27B）
- **ONNX**：[Gemma 4 QAT ONNX](https://huggingface.co/collections/james58899/gemma-4-qat-onnx)（由 Google QAT 權重轉換）、[Hy-MT2 ONNX](https://huggingface.co/collections/james58899/hy-mt2-onnx)（fp16 由騰訊原權重轉換，q4f16 由 Intel AutoRound int4 轉換）、[Qwen3.8 27B 3-bit GSQ ONNX](https://huggingface.co/james58899/Qwen3.8-27B-3Bit-GSQ-ONNX)（由 ISTA-DASLab 3-bit GSQ 權重轉換）

### 自訂 GGUF

可使用上表以外、自行下載的 GGUF 模型（亦支援分割檔 `*-00001-of-0000N.gguf`）：

1. 將 `.gguf` 檔案放入 `exe` 同目錄的 `models/custom/`（首次啟動後自動建立）
2. 至 **API** 頁將內建模型選為 **自訂 GGUF**，點右側重新整理按鈕後從清單選擇檔案
3. 按下 **儲存**

GGUF 檔案須內含 chat template（大多數公開發布的 GGUF 皆已包含）；取樣參數留空時會使用 `llama.cpp` 的預設值。

## 管線流程

```text
目標視窗
  → Windows Graphics Capture
  → PP-OCRv6 / PP-OCRv5（ONNX Runtime + WebGPU / DirectML，失敗回退 CPU）
  → 信心過濾 / 單字元過濾 / 多行合併（規則或 PP-DocLayoutV3）/ 區塊持續追蹤
  → 穩定閘門
  → LLM 翻譯（工作階段快取 + 多輪上下文；內建模型與 HTTP 支援串流）
  → 點擊可穿透 Overlay（跟隨目標視窗）+ 獨立翻譯視窗
```

## 從原始碼建置

需要 **Rust 1.100+**（edition 2024）與 **Visual Studio 2026**。首次建置請先編譯 ONNX Runtime 1.31（DirectML + WebGPU）並下載 `llama.cpp` 預編譯 DLL：

```powershell
.\scripts\build-onnxruntime.ps1
.\scripts\fetch-llama.ps1
cargo build --release -p translator-app
.\scripts\package-portable.ps1
```

封裝產物為 `dist/TranslatorOverlay-<version>-win-x64.zip`，包含 `translator-app.exe`、`lib/` 內的 ONNX Runtime / DirectML / WebGPU DLL，以及 `lib/llama.cpp/` 內的 `llama.cpp` DLL。目標機器仍需安裝 Windows App Runtime 2.4。

開發時格式化與檢查：

```powershell
cargo +nightly fmt
cargo clippy --all-targets -- -D warnings
```

## 專案結構

```text
crates/
  translator-app         # WinUI 3 控制視窗、設定介面與管線協調
  translator-capture     # Windows Graphics Capture 擷取與視窗監控
  translator-core        # 設定、路徑、區域設定集、API 設定檔、翻譯對照與共用型別
  translator-ocr         # PP-OCRv6 / PP-OCRv5、模型下載、穩定閘門、過濾與多行合併
  translator-overlay     # 點擊可穿透 Overlay、區域框選與獨立翻譯視窗
  translator-translate   # 內建模型（GGUF / ONNX）、HTTP、Grok / OpenCode / Codex / Claude CLI、快取
  translator-winui       # WinUI 3 bindings（由 winui-bindgen 產生）與啟動
  winui-bindgen          # 重新產生 translator-winui bindings 的工具
```

## 授權

本專案採用 [The Unlicense](LICENSE) 釋出至公有領域（Public Domain）。
