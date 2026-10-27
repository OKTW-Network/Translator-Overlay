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
                // Global layers (head_dim 512) use a Concat attention. The last sliding layer lends its
                // present (and a mask sized from it) to the KV-shared layers, which span all of it.
                let donor = inputs.iter().rev().find(|&&(_, d)| d == 256).map(|&(layer, _)| layer);
                inputs.iter().map(|&(layer, d)| d == 256 && Some(layer) != donor).collect()
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
    /// is off (Gemma 4 26B and up), so it answers without thinking.
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
const DEFAULT_ID: &str = "gemma-4-e4b-it-q4";

/// Empty `id` selects `DEFAULT_ID`.
pub fn find(id: &str) -> Option<&'static BuiltinModelSpec> {
    let id = match id.trim() {
        "" => DEFAULT_ID,
        id => id,
    };
    BUILTIN_MODELS.iter().find(|m| m.id == id)
}

const GEMMA4_E4B: &str = "https://huggingface.co/onnx-community/gemma-4-E4B-it-ONNX/resolve/2b6d2e26a962f70a33da8f7cd81e1afdd8d9cfb2";
const GEMMA4_E4B_ONNX: &str =
    "https://huggingface.co/onnx-community/gemma-4-E4B-it-ONNX/resolve/2b6d2e26a962f70a33da8f7cd81e1afdd8d9cfb2/onnx";

const fn art(release: &'static str, file_name: &'static str, expected_bytes: u64) -> ModelArtifact {
    ModelArtifact {
        file_name,
        release,
        expected_bytes,
    }
}

const GEMMA4_E4B_TOKENIZER: ModelArtifact = art(GEMMA4_E4B, "tokenizer.json", 19_439_251);
/// 24 layers × K/V × 2 heads: 20 with head_dim 256, 4 with 512.
const GEMMA4_E4B_KV_ELEMS: u64 = 2 * 2 * (20 * 256 + 4 * 512);

const GEMMA4_26B: &str = "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/7ca3feb896b2c28d199f6addc432be487dfd00b7";
const GEMMA4_26B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/7ca3feb896b2c28d199f6addc432be487dfd00b7/onnx";

/// Sorted by download size.
pub const BUILTIN_MODELS: &[BuiltinModelSpec] = &[
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-q4f16",
        label: "Gemma 4 E4B · q4f16 (4.9 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it",
        tokenizer: GEMMA4_E4B_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4f16.onnx", 5_619),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4f16.onnx_data", 2_017_460_224),
        ],
        decoder: &[
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx", 850_610),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx_data", 2_074_847_232),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 812_318_720),
        ],
        kv_bytes_per_token: GEMMA4_E4B_KV_ELEMS * 2,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-q4",
        label: "Gemma 4 E4B · q4 (5.6 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it",
        tokenizer: GEMMA4_E4B_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx", 5_134),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx_data", 1_839_202_304),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx_data_1", 396_361_728),
        ],
        decoder: &[
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx", 814_829),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx_data", 2_093_703_168),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx_data_1", 1_286_205_440),
        ],
        kv_bytes_per_token: GEMMA4_E4B_KV_ELEMS * 4,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-int8",
        label: "Gemma 4 E4B · int8 (9.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it",
        tokenizer: GEMMA4_E4B_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_ONNX, "embed_tokens_quantized.onnx", 5_171),
            art(GEMMA4_E4B_ONNX, "embed_tokens_quantized.onnx_data", 775_946_240),
            art(GEMMA4_E4B_ONNX, "embed_tokens_quantized.onnx_data_1", 2_818_572_288),
            art(GEMMA4_E4B_ONNX, "embed_tokens_quantized.onnx_data_2", 440_401_920),
        ],
        decoder: &[
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_quantized.onnx", 824_032),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_quantized.onnx_data", 2_096_242_688),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_quantized.onnx_data_1", 2_053_529_600),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_quantized.onnx_data_2", 1_624_637_440),
        ],
        kv_bytes_per_token: GEMMA4_E4B_KV_ELEMS * 4,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-26b-a4b-it-qat-q4f16",
        label: "Gemma 4 26B A4B · QAT q4f16 (14.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-26b-a4b-it-qat",
        tokenizer: art(GEMMA4_26B, "tokenizer.json", 32_169_626),
        embed: &[
            art(GEMMA4_26B_ONNX, "embed_tokens_q4f16.onnx", 585),
            art(GEMMA4_26B_ONNX, "embed_tokens_q4f16.onnx_data", 415_236_096),
        ],
        decoder: &[
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx", 406_510),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_991_159_808),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 1_888_265_728),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_4", 1_976_745_984),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_5", 1_984_856_064),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_6", 1_984_878_080),
            art(GEMMA4_26B_ONNX, "decoder_model_merged_q4f16.onnx_data_7", 46_137_344),
        ],
        // 30 layers × K/V: 25 sliding with 8 heads × 256, 5 global with 2 heads × 512; fp16.
        kv_bytes_per_token: 2 * (25 * 8 * 256 + 5 * 2 * 512) * 2,
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
        assert_eq!(find(" gemma-4-e4b-it-q4 ").map(|m| m.id), Some("gemma-4-e4b-it-q4"));
        assert!(find("nope").is_none());
    }

    #[test]
    fn gemma4_in_place_kv_skips_global_and_donor_layers() {
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
        assert_eq!(BuiltinArch::Gemma4.kv_in_place(&inputs, false), [true, true, false, false, false, false, false, false]);
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
