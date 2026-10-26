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
    pub fn render_prompt(self, messages: &[ChatMessage]) -> String {
        match self {
            // Thinking stays off: no `<|think|>` in the system turn.
            Self::Gemma4 => {
                let mut out = String::from("<bos>");
                for m in messages {
                    let role = if m.role == "assistant" { "model" } else { m.role.as_str() };
                    out.push_str(&format!("<|turn>{role}\n{}<turn|>\n", m.content.trim()));
                }
                out.push_str("<|turn>model\n");
                out
            }
        }
    }

    pub fn eos_ids(self) -> &'static [u32] {
        match self {
            Self::Gemma4 => &[1, 106, 50],
        }
    }

    /// For KV inputs given as `(layer, head_dim)` in decoder input order: whether only
    /// `GroupQueryAttention` reads it, which appends into a preallocated
    /// `[1, heads, context, head_dim]` buffer bound as both past and present.
    pub fn kv_in_place(self, inputs: &[(&str, i64)]) -> Vec<bool> {
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
            // Both sessions sit on the GPU, plus ~6% runtime buffers; ~30% of the weights stays resident in RAM.
            OcrDevice::Webgpu | OcrDevice::Directml => {
                let weights = bytes(self.embed) + bytes(self.decoder);
                (weights * 106 / 100 + kv, weights * 3 / 10)
            }
            // The embedding tables are memory-mapped and only the rows for seen tokens get paged in.
            OcrDevice::Cpu => (0, bytes(self.decoder) + kv),
        }
    }
}

/// Empty `id` selects the first entry.
pub fn find(id: &str) -> Option<&'static BuiltinModelSpec> {
    let id = id.trim();
    if id.is_empty() {
        return BUILTIN_MODELS.first();
    }
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

pub const BUILTIN_MODELS: &[BuiltinModelSpec] = &[
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
    },
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
        assert_eq!(find("").map(|m| m.id), BUILTIN_MODELS.first().map(|m| m.id));
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
        assert_eq!(BuiltinArch::Gemma4.kv_in_place(&inputs), [true, true, false, false, false, false, false, false]);
    }

    #[test]
    fn gemma4_prompt_matches_chat_template() {
        let msg = |role: &str, content: &str| ChatMessage {
            role: role.into(),
            content: content.into(),
            reasoning_content: None,
        };
        let prompt = BuiltinArch::Gemma4.render_prompt(&[
            msg("system", "sys\n"),
            msg("user", " u1"),
            msg("assistant", "a1"),
            msg("user", "u2"),
        ]);
        assert_eq!(
            prompt,
            "<bos><|turn>system\nsys<turn|>\n<|turn>user\nu1<turn|>\n<|turn>model\na1<turn|>\n<|turn>user\nu2<turn|>\n<|turn>model\n"
        );
    }
}
