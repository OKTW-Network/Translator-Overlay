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
    /// Hy-MT2 1.8B.
    HyMt2Small,
    /// Hy-MT2 7B.
    HyMt2Medium,
    /// Hy-MT2 30B-A3B.
    HyMt2Moe,
}

impl BuiltinArch {
    /// Prompt text for `messages`, ending with an open model turn.
    pub fn render_prompt(self, messages: &[ChatMessage], empty_thought: bool) -> String {
        // The Hy-MT2 templates keep every message untrimmed and only read a leading system message.
        let (system, turns) = match messages.split_first() {
            Some((first, rest)) if first.role == "system" => (Some(first.content.as_str()), rest),
            _ => (None, messages),
        };
        match self {
            Self::HyMt2Small => {
                let mut out = String::from("<｜hy_begin▁of▁sentence｜>");
                if let Some(system) = system {
                    out.push_str(&format!("{system}<｜hy_place▁holder▁no▁3｜>"));
                }
                for m in turns {
                    match m.role.as_str() {
                        "user" => out.push_str(&format!("<｜hy_User｜>{}", m.content)),
                        "assistant" => out.push_str(&format!("<｜hy_Assistant｜>{}<｜hy_place▁holder▁no▁2｜>", m.content)),
                        _ => {}
                    }
                }
                out.push_str("<｜hy_Assistant｜>");
                out
            }
            // No generation prompt: the model turn opens right after the user's `<|extra_0|>`.
            Self::HyMt2Medium => {
                let mut out = String::new();
                // A non-empty system message takes the first user turn's `<|startoftext|>`.
                let mut head = false;
                if let Some(system) = system.filter(|s| !s.is_empty()) {
                    out.push_str(&format!("<|startoftext|>{system}<|extra_4|>"));
                    head = true;
                }
                for m in turns {
                    match m.role.as_str() {
                        "user" if std::mem::take(&mut head) => out.push_str(&format!("{}<|extra_0|>", m.content)),
                        "user" => out.push_str(&format!("<|startoftext|>{}<|extra_0|>", m.content)),
                        "assistant" => out.push_str(&format!("{}<|eos|>", m.content)),
                        _ => {}
                    }
                }
                out
            }
            // Thinking stays off: `reasoning_effort:no_think` and an empty think block in every model turn.
            Self::HyMt2Moe => {
                let mut out = format!("<｜hy_begin▁of▁sentence｜>{}<｜reasoning_mode｜>reasoning_effort:no_think", system.unwrap_or(""));
                for m in turns {
                    match m.role.as_str() {
                        "user" => out.push_str(&format!("<｜hy_User｜>{}", m.content)),
                        "assistant" => out.push_str(&format!("<｜hy_Assistant｜><think></think>{}<eos:6124c78e>", m.content)),
                        _ => {}
                    }
                }
                out.push_str("<｜hy_Assistant｜><think></think>");
                out
            }
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
            Self::HyMt2Small => &[120020],
            Self::HyMt2Medium => &[127960, 127967],
            Self::HyMt2Moe => &[120025, 120026],
        }
    }

    /// `(temperature, top_p, top_k)` from the model's `generation_config.json`, or its model card when
    /// that has none. `top_k` 0 is off.
    pub fn sampling_defaults(self) -> (f32, f32, u32) {
        match self {
            Self::Gemma4 => (1.0, 0.95, 64),
            Self::HyMt2Small | Self::HyMt2Medium => (0.7, 0.8, 20),
            Self::HyMt2Moe => (0.7, 1.0, 0),
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
            // Every Hy-MT2 entry sets `kv_gqa_only`.
            Self::HyMt2Small | Self::HyMt2Medium | Self::HyMt2Moe => vec![true; inputs.len()],
        }
    }
}

/// Decoder shape that weights, KV, and a WebGPU prefill run's largest buffers scale with.
#[derive(Debug)]
pub struct ArchShape {
    /// Bytes at the start of the embedding's external data that the decoder's tied `lm_head` reads
    /// as its own initializer.
    pub lm_head_bytes: u64,
    /// KV elements per context token: Σ heads × head_dim over every `past_key_values.*` input.
    pub kv_elems_per_token: u64,
    /// Query heads per layer.
    pub heads: u64,
    /// Layers attending with plain `MatMul` + `Softmax` (E4B's KV-shared layers), so their
    /// attention scores are graph tensors.
    pub matmul_attn_layers: u64,
    /// `GroupQueryAttention` layers with head_dim 512.
    pub gqa_512_layers: u64,
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
    /// Activation and KV element size: 4 for fp32 (q4), 2 for fp16 (q4f16).
    pub elem_bytes: u64,
    pub shape: &'static ArchShape,
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

    /// Tokens per prefill run. A WebGPU run holds the buffers ORT's cache drops until the GPU
    /// finishes it, so each fed token costs VRAM until the run ends (see `memory_estimate`).
    /// fp32 (q4) holds several MB per token (12B: +13 GB at 1500 tokens), and 256-token runs cap it
    /// at no speed cost; fp16 holds far less, and 512-token runs save ~0.4 GB for ~10% slower
    /// prefill, where smaller ones slow it much more.
    pub fn prefill_chunk(&self) -> u64 {
        if self.elem_bytes == 4 { 256 } else { 512 }
    }

    /// `(VRAM, RAM)` bytes after a prefill that fills `context`, from the decoder's weights, KV, and
    /// the prefill buffers ORT's WebGPU buffer cache cannot keep, and the embedding tables. The
    /// tokenizer, ORT, and Dawn take a flat allowance; activations kept between runs are left out.
    pub fn memory_estimate(&self, device: OcrDevice, context: u32) -> (u64, u64) {
        /// ORT WebGPU's largest buffer-cache bucket. Bigger buffers bypass the cache and stay
        /// allocated until the GPU finishes the run.
        const MAX_BUCKET: u64 = 160 << 20;
        /// Tokenizer, ORT, and Dawn.
        const HOST: u64 = 512 << 20;
        let ctx = u64::from(context);
        let (s, e) = (self.shape, self.elem_bytes);
        let kv = s.kv_elems_per_token * e * ctx;
        // ORT WebGPU uploads initializers at their own size, outside the buffer-cache buckets.
        let weights = self.decoder.iter().map(|a| a.expected_bytes).sum::<u64>() + s.lm_head_bytes;
        // The embedding session always runs on the CPU, straight from its mmapped external data: a
        // row stays resident once a token reads it, and over time every row can.
        let ram = self.embed.iter().map(|a| a.expected_bytes).sum::<u64>() + HOST;
        match device {
            OcrDevice::Webgpu => {
                let chunk = self.prefill_chunk();
                let mut held = 0;
                // `MatMul` attention: scores (`e` bytes) → fp32 Cast → Where. Softmax and the second
                // Cast reuse the first two buffers (ORT's allocation planner).
                let scores = s.heads * chunk * ctx;
                if scores * 4 > MAX_BUCKET {
                    held += s.matmul_attn_layers * scores * (e + 8);
                }
                // fp32 head_dim-512 attention: dense prefill needs 2 × 512 × 4 B × 16 keys = 64 KiB of
                // workgroup storage, over D3D12's 32 KiB, so ORT takes the split-reduce path: per
                // 64-token tile, 512 fp32 partial outputs and 2 fp32 stats per query and head.
                if e == 4 {
                    let split = s.heads * chunk * ctx.div_ceil(64) * (512 + 2) * 4;
                    if split > MAX_BUCKET {
                        held += s.gqa_512_layers * split;
                    }
                }
                (weights + kv + held, ram)
            }
            // [`crate::builtin::ensure_loaded`] rejects DirectML, so nothing gets loaded.
            OcrDevice::Directml => (0, 0),
            // Each 32-weight 4-bit block is 16 bytes plus its scale, which ORT's CPU kernels keep as
            // fp32: q4f16's 2-byte scales grow it by 10/9.
            // shortcut: 26B's QMoE keeps its FC3 scales fp16, so 26B q4f16 is ~0.5 GB high, and Hy-MT2
            // (fp16 weights, or 4-bit blocks of 128) comes out 7–11% high; fine unless CPU estimates need
            // to be exact.
            OcrDevice::Cpu => (0, if e == 2 { weights * 10 / 9 } else { weights } + kv + ram),
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

const GEMMA4_E4B: &str = "https://huggingface.co/james58899/gemma-4-E4B-it-qat-ONNX/resolve/644d96e6a33edcf3f5e10a960d8fb3fddef5d079";
const GEMMA4_E4B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-E4B-it-qat-ONNX/resolve/644d96e6a33edcf3f5e10a960d8fb3fddef5d079/onnx";
const GEMMA4_E4B_TOKENIZER: ModelArtifact = art(GEMMA4_E4B, "tokenizer.json", 32_169_626);
/// The last 18 of 42 layers reuse earlier layers' KV through `MatMul` attention.
const GEMMA4_E4B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 335_544_320,
    // 24 layers × K/V × 2 heads: 20 with head_dim 256, 4 with 512.
    kv_elems_per_token: 2 * 2 * (20 * 256 + 4 * 512),
    heads: 8,
    matmul_attn_layers: 18,
    gqa_512_layers: 4,
};

const GEMMA4_12B: &str = "https://huggingface.co/james58899/gemma-4-12B-it-qat-ONNX/resolve/60555e645e7c456b4a013cc2e93e9bd6379e3006";
const GEMMA4_12B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-12B-it-qat-ONNX/resolve/60555e645e7c456b4a013cc2e93e9bd6379e3006/onnx";
const GEMMA4_12B_TOKENIZER: ModelArtifact = art(GEMMA4_12B, "tokenizer.json", 32_169_626);
const GEMMA4_12B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 503_316_480,
    // 48 layers × K/V: 40 sliding with 8 heads × 256, 8 global with 1 head × 512.
    kv_elems_per_token: 2 * (40 * 8 * 256 + 8 * 512),
    heads: 16,
    matmul_attn_layers: 0,
    gqa_512_layers: 8,
};

const GEMMA4_26B: &str = "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/85d6d068fe1484b3958ad5a4b4eab2eb9dde2316";
const GEMMA4_26B_ONNX: &str =
    "https://huggingface.co/james58899/gemma-4-26B-A4B-it-qat-ONNX/resolve/85d6d068fe1484b3958ad5a4b4eab2eb9dde2316/onnx";
const GEMMA4_26B_TOKENIZER: ModelArtifact = art(GEMMA4_26B, "tokenizer.json", 32_169_626);
const GEMMA4_26B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 369_098_752,
    // 30 layers × K/V: 25 sliding with 8 heads × 256, 5 global with 2 heads × 512.
    kv_elems_per_token: 2 * (25 * 8 * 256 + 5 * 2 * 512),
    heads: 16,
    matmul_attn_layers: 0,
    gqa_512_layers: 5,
};

const HY_MT2_1_8B: &str = "https://huggingface.co/james58899/Hy-MT2-1.8B-ONNX/resolve/4eba3702a3c289d4842b684161cfa4d0b9e8e3ff";
const HY_MT2_1_8B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-1.8B-ONNX/resolve/4eba3702a3c289d4842b684161cfa4d0b9e8e3ff/onnx";
const HY_MT2_1_8B_TOKENIZER: ModelArtifact = art(HY_MT2_1_8B, "tokenizer.json", 9_527_287);
const HY_MT2_1_8B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 494_870_528,
    // 32 layers × K/V × 4 heads × 128.
    kv_elems_per_token: 32 * 2 * 4 * 128,
    heads: 16,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

const HY_MT2_7B: &str = "https://huggingface.co/james58899/Hy-MT2-7B-ONNX/resolve/a1390f8c0cfc1272e0b62d97289d75a7cd395e3b";
const HY_MT2_7B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-7B-ONNX/resolve/a1390f8c0cfc1272e0b62d97289d75a7cd395e3b/onnx";
const HY_MT2_7B_TOKENIZER: ModelArtifact = art(HY_MT2_7B, "tokenizer.json", 16_388_221);
const HY_MT2_7B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 1_049_944_064,
    // 32 layers × K/V × 8 heads × 128.
    kv_elems_per_token: 32 * 2 * 8 * 128,
    heads: 32,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

const HY_MT2_30B: &str = "https://huggingface.co/james58899/Hy-MT2-30B-A3B-ONNX/resolve/eb24537e937a4faa6be21d594ca62b19e4b94300";
const HY_MT2_30B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-30B-A3B-ONNX/resolve/eb24537e937a4faa6be21d594ca62b19e4b94300/onnx";
/// The untied `lm_head` sits in the decoder's own external data.
const HY_MT2_30B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 0,
    // 48 layers × K/V × 4 heads × 128.
    kv_elems_per_token: 48 * 2 * 4 * 128,
    heads: 32,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

/// Grouped by family, each sorted by download size.
pub const BUILTIN_MODELS: &[BuiltinModelSpec] = &[
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4f16",
        label: "Gemma 4 E4B · q4f16 (4.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        tokenizer: GEMMA4_E4B_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4f16.onnx", 1_283),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4f16.onnx_data", 1_962_934_272),
        ],
        decoder: &[
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx", 425_326),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_981_871_104),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 296_493_056),
        ],

        elem_bytes: 2,
        shape: &GEMMA4_E4B_SHAPE,
        kv_gqa_only: false,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4",
        label: "Gemma 4 E4B · q4 (4.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        tokenizer: GEMMA4_E4B_TOKENIZER,
        embed: &[
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx", 1_263),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx_data", 1_828_716_544),
            art(GEMMA4_E4B_ONNX, "embed_tokens_q4.onnx_data_1", 352_321_536),
        ],
        decoder: &[
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx", 477_484),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx_data", 1_995_997_184),
            art(GEMMA4_E4B_ONNX, "decoder_model_merged_q4.onnx_data_1", 573_476_864),
        ],

        elem_bytes: 4,
        shape: &GEMMA4_E4B_SHAPE,
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

        elem_bytes: 2,
        shape: &GEMMA4_12B_SHAPE,
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

        elem_bytes: 4,
        shape: &GEMMA4_12B_SHAPE,
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

        elem_bytes: 2,
        shape: &GEMMA4_26B_SHAPE,
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

        elem_bytes: 4,
        shape: &GEMMA4_26B_SHAPE,
        kv_gqa_only: true,
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "hy-mt2-1.8b-q4f16",
        label: "Hy-MT2 1.8B · q4f16 (1.3 GB)",
        arch: BuiltinArch::HyMt2Small,
        dir: "hy-mt2-1.8b",
        tokenizer: HY_MT2_1_8B_TOKENIZER,
        embed: &[
            art(HY_MT2_1_8B_ONNX, "embed_tokens_q4f16.onnx", 355),
            art(HY_MT2_1_8B_ONNX, "embed_tokens_q4f16.onnx_data", 494_870_528),
        ],
        decoder: &[
            art(HY_MT2_1_8B_ONNX, "decoder_model_merged_q4f16.onnx", 239_026),
            art(HY_MT2_1_8B_ONNX, "decoder_model_merged_q4f16.onnx_data", 796_135_424),
        ],

        elem_bytes: 2,
        shape: &HY_MT2_1_8B_SHAPE,
        kv_gqa_only: true,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "hy-mt2-1.8b-fp16",
        label: "Hy-MT2 1.8B · fp16 (3.6 GB)",
        arch: BuiltinArch::HyMt2Small,
        dir: "hy-mt2-1.8b",
        tokenizer: HY_MT2_1_8B_TOKENIZER,
        embed: &[
            art(HY_MT2_1_8B_ONNX, "embed_tokens_fp16.onnx", 354),
            art(HY_MT2_1_8B_ONNX, "embed_tokens_fp16.onnx_data", 494_870_528),
        ],
        decoder: &[
            art(HY_MT2_1_8B_ONNX, "decoder_model_merged_fp16.onnx", 189_471),
            art(HY_MT2_1_8B_ONNX, "decoder_model_merged_fp16.onnx_data", 1_950_523_392),
            art(HY_MT2_1_8B_ONNX, "decoder_model_merged_fp16.onnx_data_1", 1_136_750_592),
        ],

        elem_bytes: 2,
        shape: &HY_MT2_1_8B_SHAPE,
        kv_gqa_only: true,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "hy-mt2-7b-q4f16",
        label: "Hy-MT2 7B · q4f16 (4.7 GB)",
        arch: BuiltinArch::HyMt2Medium,
        dir: "hy-mt2-7b",
        tokenizer: HY_MT2_7B_TOKENIZER,
        embed: &[
            art(HY_MT2_7B_ONNX, "embed_tokens_q4f16.onnx", 356),
            art(HY_MT2_7B_ONNX, "embed_tokens_q4f16.onnx_data", 1_049_944_064),
        ],
        decoder: &[
            art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx", 239_944),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_994_293_248),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_604_952_064),
        ],

        elem_bytes: 2,
        shape: &HY_MT2_7B_SHAPE,
        kv_gqa_only: true,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "hy-mt2-7b-fp16",
        label: "Hy-MT2 7B · fp16 (15.0 GB)",
        arch: BuiltinArch::HyMt2Medium,
        dir: "hy-mt2-7b",
        tokenizer: HY_MT2_7B_TOKENIZER,
        embed: &[
            art(HY_MT2_7B_ONNX, "embed_tokens_fp16.onnx", 355),
            art(HY_MT2_7B_ONNX, "embed_tokens_fp16.onnx_data", 1_049_944_064),
        ],
        decoder: &[
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx", 189_886),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data", 1_828_798_464),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_1", 1_979_777_024),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_2", 1_946_238_976),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_3", 1_979_777_024),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_4", 1_946_238_976),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_5", 1_979_777_024),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_6", 1_946_238_976),
            art(HY_MT2_7B_ONNX, "decoder_model_merged_fp16.onnx_data_7", 352_329_728),
        ],

        elem_bytes: 2,
        shape: &HY_MT2_7B_SHAPE,
        kv_gqa_only: true,
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "hy-mt2-30b-a3b-q4f16",
        label: "Hy-MT2 30B A3B · q4f16 (16.3 GB)",
        arch: BuiltinArch::HyMt2Moe,
        dir: "hy-mt2-30b-a3b",
        tokenizer: art(HY_MT2_30B, "tokenizer.json", 9_527_108),
        embed: &[
            art(HY_MT2_30B_ONNX, "embed_tokens_q4f16.onnx", 355),
            art(HY_MT2_30B_ONNX, "embed_tokens_q4f16.onnx_data", 494_927_872),
        ],
        decoder: &[
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx", 498_896),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_990_320_128),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_4", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_5", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_6", 1_947_893_760),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_7", 1_612_460_032),
            art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_8", 494_927_872),
        ],

        elem_bytes: 2,
        shape: &HY_MT2_30B_SHAPE,
        kv_gqa_only: true,
        empty_thought: false,
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
            assert!(m.shape.kv_elems_per_token > 0, "{}", m.id);
            assert!(m.shape.lm_head_bytes <= m.embed[1].expected_bytes, "{}", m.id);
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

    #[test]
    fn hy_mt2_prompts_match_chat_templates() {
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
        // Each checkpoint's `apply_chat_template(messages, add_generation_prompt=True)`.
        let cases = [
            (
                BuiltinArch::HyMt2Small,
                "<｜hy_begin▁of▁sentence｜>sys\n<｜hy_place▁holder▁no▁3｜><｜hy_User｜> u1<｜hy_Assistant｜>a1\n<｜hy_place▁holder▁no▁2｜><｜hy_User｜>u2<｜hy_Assistant｜>",
            ),
            (BuiltinArch::HyMt2Medium, "<|startoftext|>sys\n<|extra_4|> u1<|extra_0|>a1\n<|eos|><|startoftext|>u2<|extra_0|>"),
            (
                BuiltinArch::HyMt2Moe,
                "<｜hy_begin▁of▁sentence｜>sys\n<｜reasoning_mode｜>reasoning_effort:no_think<｜hy_User｜> u1<｜hy_Assistant｜><think></think>a1\n<eos:6124c78e><｜hy_User｜>u2<｜hy_Assistant｜><think></think>",
            ),
        ];
        for (arch, expected) in cases {
            assert_eq!(arch.render_prompt(&messages, false), expected, "{arch:?}");
        }
        // Without a system message, 7B's first user turn opens with `<|startoftext|>`.
        assert_eq!(BuiltinArch::HyMt2Medium.render_prompt(&messages[1..2], false), "<|startoftext|> u1<|extra_0|>");
    }
}
