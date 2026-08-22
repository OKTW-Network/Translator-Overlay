# Translator Overlay

Windows 桌面即時翻譯覆蓋層：選取目標視窗 → 擷取畫面 → PP-OCRv6 辨識文字 → 經 OpenAI 相容 API 翻譯 → 在原位置以可點穿（click-through）覆蓋層顯示譯文。

## 功能

- **視窗擷取**：Windows Graphics Capture，可選清單視窗或前景視窗
- **多區域 OCR**：在目標視窗上框選多個辨識範圍；可存成可命名的 region profile（`region-profiles.toml`）；無框時仍辨識整窗
- **本機 OCR**：PP-OCRv6（tiny / small / medium），ONNX Runtime + DirectML（GPU，失敗時回退 CPU）
- **穩定門檻**：畫面文字穩定一段時間後才送翻譯，減少抖動與誤觸發
- **區塊過濾**：過濾單字元雜訊、動畫圖示誤辨識；可調多行合併（幾何規則）
- **LLM 翻譯**：OpenAI 相容 Chat Completions，或本機長駐 Grok ACP / Codex app-server（只 append 新 turn）
- **翻譯記憶**：同一句原文只翻一次，人名、按鈕、重播台詞不會一直重送。可在 Translation 頁調整記住多少句或按 Clear cache 清空；關閉程式後會忘掉
- **對話上下文**：多輪歷史壓縮，維持用語一致
- **透明覆蓋層**：WS_EX_LAYERED + 點穿，跟隨目標視窗位置與 OCR 框；可即時開關
- **譯文懸浮窗**：無邊框半透明置頂視窗顯示最新譯文（拖曳移動、邊緣縮放），樣式與 overlay 相同，不跟隨遊戲焦點
- **設定 UI**：Dashboard + API / Translation / OCR / Overlay 分頁，設定寫入 `config.toml`

## 系統需求

| 項目 | 說明 |
|------|------|
| 作業系統 | Windows 10 / 11 **x64** |
| 執行階段 | Windows App Runtime（framework-dependent） |
| 執行庫 | Microsoft Visual C++ Redistributable（x64），若系統缺少 CRT |
| 網路 | 首次下載 OCR 模型（若本機尚無）；翻譯 API 連線 |
| 開發建置 | Rust（edition 2024）、Visual Studio Build Tools（Windows 目標） |

## 快速開始（預建置 / 可攜包）

1. 解壓 `TranslatorOverlay-*-win-x64.zip`（保持 DLL 與 exe 同目錄）
2. 雙擊 `translator-app.exe`
3. 在 **API** 頁選 Provider：HTTP 填 `api_key`，或改用本機 Grok / Codex CLI；然後 **Save**
4. 回 **Dashboard**：重新整理視窗清單 → 選取目標 → **Start**

`config.toml`、`region-profiles.toml` 與 `models/` 會放在可執行檔同目錄。

可攜包內建議包含：

- `translator-app.exe`
- `Microsoft.WindowsAppRuntime.Bootstrap.dll`
- `DirectML.dll`（ONNX Runtime DirectML EP）
- `resources.pri`

## 從原始碼建置

```powershell
cargo build --release -p translator-app
```

執行：

```powershell
.\target\release\translator-app.exe
```

打包可攜 ZIP：

```powershell
.\scripts\package-portable.ps1
# 或略過編譯、只用現有 release 產物：
.\scripts\package-portable.ps1 -SkipBuild
```

產物位於 `dist/TranslatorOverlay-<version>-win-x64.zip`。

## 使用方式

1. **API**：選 Provider（OpenAI-compatible / Grok CLI / Codex CLI）。HTTP 填 `base_url`、`api_key`、`model`；CLI 用本機已登入的 `grok` / `codex`，可選 `cli_path`
2. **Translation**：來源語 / 目標語（預設 `auto` → `zh-TW`）、可選提示詞。可開關翻譯記憶、設定記住多少句，以及清空已記住的譯文。譯文不滿意時按 **Retry** 會重翻這一頁並更新記憶
3. **OCR**：模型等級、信心閾值、穩定時間、區塊持續過濾、行合併等
4. **Overlay**：開關 in-place overlay / 譯文窗、譯文窗字級，以及文字色、背景色（ARGB）
5. **Dashboard**：
   - 選視窗後 **Start** 連續擷取（畫面上直接顯示擷取預覽）
   - **Select regions**：在目標視窗上拖曳畫多個 OCR 框（可改大小／移動；右鍵刪除該框）。選取層與翻譯 overlay 一樣，只在目標視窗前景時顯示。再按同一顆按鈕（**Done**）套用；**Clear** 回到整窗。框預設只在本次執行有效；可用 **Profiles** ComboBox 選擇後 Load / Delete，或在 Done 後按 Save 輸入名稱寫入 `{exe 目錄}/region-profiles.toml`。
   - **Once** 立刻拍一幀並 OCR + 翻譯（略過穩定等待）
   - 翻譯進行中可取消；失敗可 **Retry**
   - 可重置對話歷史

首次載入 OCR 時，若 `models/` 缺少對應 ONNX（或檔案大小不符），程式會在背景從 GitHub Releases 下載到 `models_dir`（預設 `models/`），狀態列會顯示進度；下載／載入完成前無法開始擷取，其餘 UI 仍可操作。

## 設定（`config.toml`）

設定檔預設路徑：`{exe 目錄}/config.toml`。不存在時會自動建立預設值。

OCR 區域 profile 存在獨立檔 `{exe 目錄}/region-profiles.toml`（與 `config.toml` 分開；首次 Save 才會建立）。

精簡範例：

```toml
[api]
provider = "openai_compatible"   # openai_compatible | grok_cli | codex_cli
# cli_path = ""                  # 空 = PATH 上的 grok / codex
service_tier = "standard"       # standard | priority；目前 Codex CLI 支援 Priority（Fast mode）
base_url = "https://api.openai.com/v1"
api_key = ""
model = "gpt-4o-mini"
request_timeout_secs = 60
max_retries = 2
retry_backoff_ms = 500
# temperature = 0.3
# top_p = 0.9
# max_tokens = 2048
# reasoning_effort = "medium"

[translation]
source_lang = "auto"
target_lang = "zh-TW"
history_max_items = 8
conversation_max_turns = 20
cache_enabled = true
cache_max_entries = 128

[ocr]
model_tier = "small"   # tiny | small | medium
models_dir = "models"
confidence_threshold = 0.5
stable_duration_ms = 500
filter_single_char = true
block_persist_ms = 450
block_max_miss_ms = 700

[ocr.line_merge]
enabled = true
merge_whole_region = false          # join every line in each hand-drawn OCR region (ignored if no regions)
order = "left_to_right_top_to_bottom"  # or top_to_bottom_left_to_right
join_with_space = true              # false concatenates (typical for CJK)
reject_short_long = true            # do not glue a short line onto a wider line below
# gap_ratio = 0.015                 # allowed |vertical gap| × window height
# height_delta_ratio = 0.45         # allowed |h1 − h2| / larger height
# width_delta_ratio = 0.40          # allowed (lower − upper) / lower width (when reject_short_long)
# overlap_ratio = 0.35              # vs shorter line width
# align_ratio = 0.012               # × window width (left or center)
# align_overlap_ratio = 0.10        # overlap floor on the align path
# order_band_ratio = 0.012          # reading-order row/column band
# below_mid_ratio = 0.25            # stacked-vs-side-by-side slack

[capture]
min_interval_ms = 300

[overlay]
enabled = true          # in-place click-through overlay
reader_enabled = true   # independent always-on-top translation window
reader_font_px = 20     # translation-window font size
# ARGB hex: 0xAARRGGBB
text_color_argb = "0xFFFFFFFF"
background_color_argb = "0xC8000000"
```

### OCR 模型檔

| 等級 | 偵測 | 辨識 | 字典 |
|------|------|------|------|
| tiny | `pp-ocrv6_tiny_det.onnx` | `pp-ocrv6_tiny_rec.onnx` | `ppocrv6_tiny_dict.txt` |
| small | `pp-ocrv6_small_det.onnx` | `pp-ocrv6_small_rec.onnx` | `ppocrv6_dict.txt` |
| medium | `pp-ocrv6_medium_det.onnx` | `pp-ocrv6_medium_rec.onnx` | `ppocrv6_dict.txt` |

## 管線架構

```
┌─────────────┐    ┌──────────┐    ┌────────────────┐    ┌────────────┐    ┌─────────────┐
│  Capture    │ →  │   OCR    │ →  │ Stability /    │ →  │  Translate │ →  │   Overlay   │
│  (WGC)      │    │ PP-OCRv6 │    │ block filter   │    │  (LLM API) │    │ (layered)   │
└─────────────┘    └──────────┘    └────────────────┘    └────────────┘    └─────────────┘
        ▲                                                                          │
        └────────────────── 跟隨目標 HWND / 客戶區 座標 ──────────────────────────┘
```

控制 UI（WinUI 3）與背景 pipeline 執行緒分離；UI 透過命令通道控制擷取、設定套用與翻譯取消。

## 專案結構

```
crates/
  translator-app/         # 主程式、UI、pipeline 協調
  translator-capture/     # Windows Graphics Capture 視窗擷取
  translator-core/        # config / state / 共用型別
  translator-ocr/         # OCR 引擎、模型目錄、穩定門檻、行合併
  translator-overlay/     # 透明點穿覆蓋視窗
  translator-translate/   # HTTP / Grok ACP / Codex app-server 翻譯客戶端
scripts/
  package-portable.ps1    # release 建置 + 可攜 ZIP
config.toml               # 開發用預設設定範本
```
