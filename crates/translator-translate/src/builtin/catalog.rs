//! Built-in ONNX model catalog. A new model is one more [`BuiltinModelSpec`] entry.
//!
//! Each repo's files are pinned to a commit and checked by exact byte length on download.

use translator_core::OcrDevice;
use translator_ocr::ModelArtifact;

use crate::http::ChatMessage;

/// Chat template, stop tokens, and I/O conventions shared by one model family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinArch {
    Gemma4,
}

impl BuiltinArch {
    /// Prompt text for `messages`, ending with an open model turn.
    pub fn render_prompt(self, messages: &[ChatMessage], empty_thought: bool) -> String {
        match self {
            // Thinking stays off: no `<|think|>` in the system turn.
            Self::Gemma4 => {
                // Earlier model turns keep the empty thought channel and their untrimmed reply (the
                // official template drops and trims both): that matches their cached KV, so it gets reused.
                let model = if empty_thought {
                    "model\n<|channel>thought\n<channel|>"
                } else {
                    "model\n"
                };
                let mut out = String::from("<bos>");
                for m in messages {
                    if m.role == "assistant" {
                        out.push_str(&format!("<|turn>{model}{}<turn|>\n", m.content));
                    } else {
                        out.push_str(&format!("<|turn>{}\n{}<turn|>\n", m.role, m.content.trim()));
                    }
                }
                out.push_str("<|turn>");
                out.push_str(model);
                out
            }
        }
    }

    pub fn eos_ids(self) -> &'static [u32] {
        match self {
            Self::Gemma4 => &[1, 106, 50],
        }
    }

    /// `(temperature, top_p, top_k)` from the model's `generation_config.json`.
    pub fn sampling_defaults(self) -> (f32, f32, u32) {
        match self {
            Self::Gemma4 => (1.0, 0.95, 64),
        }
    }

    /// For KV inputs given as `(layer, head_dim)` in decoder input order: whether only
    /// `GroupQueryAttention` reads it, which appends into a preallocated
    /// `[1, heads, context, head_dim]` buffer bound as both past and present.
    /// `gqa_only`: see [`BuiltinModelSpec::kv_gqa_only`].
    pub fn kv_in_place(self, inputs: &[(&str, i64)], gqa_only: bool) -> Vec<bool> {
        if gqa_only {
            return vec![true; inputs.len()];
        }
        match self {
            Self::Gemma4 => {
                // The last layer of each type (sliding 256, global 512) lends its present to the
                // KV-shared layers, which mask it by the attention mask's length, so it must not be padded.
                let donor = |d| inputs.iter().rev().find(|&&(_, d2)| d2 == d).map(|&(layer, _)| layer);
                inputs.iter().map(|&(layer, d)| Some(layer) != donor(d)).collect()
            }
        }
    }
}

/// One selectable model (family + quantization).
#[derive(Debug)]
pub struct BuiltinModelSpec {
    /// Stored in [`translator_core::ApiConfig::builtin_model`].
    pub id: &'static str,
    pub label: &'static str,
    pub arch: BuiltinArch,
    /// Subdirectory under `models_dir`. External data must sit next to its `.onnx`, and
    /// tokenizers differ per repo, so each repo gets its own flat directory.
    pub dir: &'static str,
    pub tokenizer: ModelArtifact,
    /// `.onnx` first, then its `.onnx_data*` files.
    pub embed: &'static [ModelArtifact],
    /// `.onnx` first, then its `.onnx_data*` files.
    pub decoder: &'static [ModelArtifact],
    /// KV bytes per context token: Σ heads × head_dim over every `past_key_values.*` input × element size.
    pub kv_bytes_per_token: u64,
    /// Every layer's attention is `GroupQueryAttention` and none shares another layer's KV (our own
    /// exports), so every KV input goes in place.
    pub kv_gqa_only: bool,
    /// The model's chat template opens the model turn with an empty thought channel when thinking
    /// is off (Gemma 4 12B and up), so it answers without thinking.
    pub empty_thought: bool,
}

impl BuiltinModelSpec {
    pub fn artifacts(&self) -> impl Iterator<Item = &ModelArtifact> {
        std::iter::once(&self.tokenizer).chain(self.embed).chain(self.decoder)
    }

    /// Rough `(VRAM, RAM)` bytes with a full `context`. Calibrated on q4 with a Radeon RX 9070 XT.
    pub fn memory_estimate(&self, device: OcrDevice, context: u32) -> (u64, u64) {
        let bytes = |arts: &[ModelArtifact]| arts.iter().map(|a| a.expected_bytes).sum::<u64>();
        let kv = self.kv_bytes_per_token * u64::from(context);
        match device {
            // The decoder sits on the GPU, plus ~6% runtime buffers; ~30% of it stays resident in RAM.
            // The embedding runs on the CPU (see below).
            OcrDevice::Webgpu | OcrDevice::Directml => {
                let weights = bytes(self.decoder);
                (weights * 106 / 100 + kv, weights * 3 / 10)
            }
            // The embedding tables are memory-mapped and only the rows for seen tokens get paged in.
            OcrDevice::Cpu => (0, bytes(self.decoder) + kv),
        }
    }
}

/// Model used when the config leaves `builtin_model` empty.
const DEFAULT_ID: &str = "gemma-4-e4b-it-qat-q4";

/// Empty `id` selects `DEFAULT_ID`.
pub fn find(id: &str) -> Option<&'static BuiltinModelSpec> {
    let id = match id.trim() {
        "" => DEFAULT_ID,
        id => id,
    };
    BUILTIN_MODELS.iter().find(|m| m.id == id)
}

const fn art(release: &'static str, file_name: &'static str, expected_bytes: u64) -> ModelArtifact {
    ModelArtifact {
        file_name,
        release,
        expected_bytes,
    }
}

/// 24 layers × K/V × 2 heads: 20 with head_dim 256, 4 with 512.
const GEMMA4_E4B_KV_ELEMS: u64 = 2 * 2 * (20 * 256 + 4 * 512);

const GEMMA4_E4B_QAT: &str = "https://huggingface.co/james58899/gemma-4-E4B-it-qat-ONNX/resolve/644d96e6a33edcf3f5e10a960d8fb3fddef5d079";
const GEMMA4_E4B_QAT_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-E4B-it-qat-ONNX/resolve/644d96e6a33edcf3f5e10a960d8fb3fddef5d079/onnx";
const GEMMA4_E4B_QAT_TOKENIZER: ModelArtifact = art(GEMMA4_E4B_QAT, "tokenizer.json", 32_169_626);

const GEMMA4_12B: &str = "https://huggingface.co/james58899/gemma-4-12B-it-qat-ONNX/resolve/60555e645e7c456b4a013cc2e93e9bd6379e3006";
const GEMMA4_12B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-12B-it-qat-ONNX/resolve/60555e645e7c456b4a013cc2e93e9bd6379e3006/onnx";
const GEMMA4_12B_TOKENIZER: ModelArtifact = art(GEMMA4_12B, "tokenizer.json", 32_169_626);
/// 48 layers × K/V: 40 sliding with 8 heads × 256, 8 global with 1 head × 512.
const GEMMA4_12B_KV_ELEMS: u64 = 2 * (40 * 8 * 256 + 8 * 512);

const GEMMA4_26B: &str = "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/85d6d068fe1484b3958ad5a4b4eab2eb9dde2316";
const GEMMA4_26B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/85d6d068fe1484b3958ad5a4b4eab2eb9dde2316/onnx";
const GEMMA4_26B_TOKENIZER: ModelArtifact = art(GEMMA4_26B, "tokenizer.json", 32_169_626);
/// 30 layers × K/V: 25 sliding with 8 heads × 256, 5 global with 2 heads × 512.
const GEMMA4_26B_KV_ELEMS: u64 = 2 * (25 * 8 * 256 + 5 * 2 * 512);

/// Sorted by download size.
pub const BUILTIN_MODELS: &[BuiltinModelSpec] = &[
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4f16",
        label: "Gemma 4 E4B · q4f16 (4.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        tokenizer: GEMMA4_E4B_QAT_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_QAT_ONNX, "embed_tokens_q4f16.onnx", 1_283),
            art(GEMMA4_E4B_QAT_ONNX, "embed_tokens_q4f16.onnx_data", 1_962_934_272),
        ],
        decoder: &[
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4f16.onnx", 425_326),
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_981_871_104),
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 296_493_056),
        ],
        kv_bytes_per_token: GEMMA4_E4B_KV_ELEMS * 2,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4",
        label: "Gemma 4 E4B · q4 (4.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        tokenizer: GEMMA4_E4B_QAT_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_QAT_ONNX, "embed_tokens_q4.onnx", 1_263),
            art(GEMMA4_E4B_QAT_ONNX, "embed_tokens_q4.onnx_data", 1_828_716_544),
            art(GEMMA4_E4B_QAT_ONNX, "embed_tokens_q4.onnx_data_1", 352_321_536),
        ],
        decoder: &[
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4.onnx", 477_484),
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4.onnx_data", 1_995_997_184),
            art(GEMMA4_E4B_QAT_ONNX, "decoder_model_merged_q4.onnx_data_1", 573_476_864),
        ],
        kv_bytes_per_token: GEMMA4_E4B_KV_ELEMS * 4,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-12b-it-qat-q4f16",
        label: "Gemma 4 12B · q4f16 (6.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-12b-it-qat",
        tokenizer: GEMMA4_12B_TOKENIZER,
        embed: &[
            art(GEMMA4_12B_ONNX, "embed_tokens_q4f16.onnx", 585),
            art(GEMMA4_12B_ONNX, "embed_tokens_q4f16.onnx_data", 566_231_040),
        ],
        decoder: &[
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4f16.onnx", 455_338),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_997_070_336),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_988_599_296),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_977_876_480),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 232_169_472),
        ],
        kv_bytes_per_token: GEMMA4_12B_KV_ELEMS * 2,
        kv_gqa_only: true,
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "gemma-4-12b-it-qat-q4",
        label: "Gemma 4 12B · q4 (7.6 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-12b-it-qat",
        tokenizer: GEMMA4_12B_TOKENIZER,
        embed: &[
            art(GEMMA4_12B_ONNX, "embed_tokens_q4.onnx", 580),
            art(GEMMA4_12B_ONNX, "embed_tokens_q4.onnx_data", 629_145_600),
        ],
        decoder: &[
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4.onnx", 539_467),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4.onnx_data", 1_998_962_688),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4.onnx_data_1", 1_998_978_048),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4.onnx_data_2", 1_995_258_880),
            art(GEMMA4_12B_ONNX, "decoder_model_merged_q4.onnx_data_3", 948_256_768),
        ],
        kv_bytes_per_token: GEMMA4_12B_KV_ELEMS * 4,
        kv_gqa_only: true,
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "gemma-4-26b-a4b-it-qat-q4f16",
        label: "Gemma 4 26B A4B · q4f16 (14.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-26b-a4b-it-qat",
        tokenizer: GEMMA4_26B_TOKENIZER,
        embed: &[
            art(GEMMA4_26B_ONNX, "embed_tokens_q4f16.onnx", 585),
            art(GEMMA4_26B_ONNX, "embed_tokens_q4f16.onnx_data", 415_236_096),
        ],
        decoder: &[
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx", 381_035),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_991_159_808),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 1_888_265_728),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_4", 1_976_745_984),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_5", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_6", 1_984_878_080),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_7", 46_137_344),
        ],
        kv_bytes_per_token: GEMMA4_26B_KV_ELEMS * 2,
        kv_gqa_only: true,
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "gemma-4-26b-a4b-it-qat-q4",
        label: "Gemma 4 26B A4B · q4 (15.9 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-26b-a4b-it-qat",
        tokenizer: GEMMA4_26B_TOKENIZER,
        embed: &[
            art(GEMMA4_26B_ONNX, "embed_tokens_q4.onnx", 579),
            art(GEMMA4_26B_ONNX, "embed_tokens_q4.onnx_data", 461_373_440),
        ],
        decoder: &[
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx", 432_798),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data", 1_881_825_280),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_1", 1_890_861_056),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_2", 1_890_860_032),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_3", 1_974_407_168),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_4", 1_922_580_480),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_5", 1_881_849_856),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_6", 1_890_860_032),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_7", 1_983_418_368),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4.onnx_data_8", 124_030_976),
        ],
        kv_bytes_per_token: GEMMA4_26B_KV_ELEMS * 4,
        kv_gqa_only: true,
        empty_thought: true,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_entries_are_complete_and_pinned() {
        for (i, m) in BUILTIN_MODELS.iter().enumerate() {
            assert!(BUILTIN_MODELS[..i].iter().all(|o| o.id != m.id), "duplicate id {}", m.id);
            assert_eq!(m.tokenizer.file_name, "tokenizer.json", "{}", m.id);
            assert!(m.embed[0].file_name.ends_with(".onnx") && m.decoder[0].file_name.ends_with(".onnx"), "{}", m.id);
            assert!(m.kv_bytes_per_token > 0, "{}", m.id);
            for a in m.artifacts() {
                assert!(a.expected_bytes > 0, "{} {}", m.id, a.file_name);
                assert!(a.download_url().contains("/resolve/") && !a.download_url().contains("/resolve/main/"), "{}", a.download_url());
            }
        }
        assert_eq!(find("").map(|m| m.id), Some(DEFAULT_ID));
        assert_eq!(find(" gemma-4-e4b-it-qat-q4 ").map(|m| m.id), Some("gemma-4-e4b-it-qat-q4"));
        assert!(find("nope").is_none());
    }

    #[test]
    fn gemma4_in_place_kv_skips_donor_layers() {
        let inputs = [
            ("0", 256),
            ("0", 256),
            ("1", 512),
            ("1", 512),
            ("2", 256),
            ("2", 256),
            ("3", 512),
            ("3", 512),
        ];
        assert_eq!(BuiltinArch::Gemma4.kv_in_place(&inputs, false), [true, true, true, true, false, false, false, false]);
        assert_eq!(BuiltinArch::Gemma4.kv_in_place(&inputs, true), [true; 8]);
    }

    #[test]
    fn gemma4_prompt_matches_chat_template() {
        let msg = |role: &str, content: &str| ChatMessage {
            role: role.into(),
            content: content.into(),
            reasoning_content: None,
        };
        let messages = [
            msg("system", "sys\n"),
            msg("user", " u1"),
            msg("assistant", "a1\n"),
            msg("user", "u2"),
        ];
        let expected =
            "<bos><|turn>system\nsys<turn|>\n<|turn>user\nu1<turn|>\n<|turn>model\na1\n<turn|>\n<|turn>user\nu2<turn|>\n<|turn>model\n";
        assert_eq!(BuiltinArch::Gemma4.render_prompt(&messages, false), expected);
        // Earlier model turns keep the reply untrimmed and the empty thought channel, so the cached KV stays a prefix.
        assert_eq!(
            BuiltinArch::Gemma4.render_prompt(&messages, true),
            expected.replace("<|turn>model\n", "<|turn>model\n<|channel>thought\n<channel|>")
        );
    }
}
