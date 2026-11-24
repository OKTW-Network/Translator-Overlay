//! Built-in model catalog (ONNX and GGUF). A new model is one more [`BuiltinModelSpec`] entry.
//!
//! Each repo's files are pinned to a commit and checked by exact byte length on download.

use translator_core::{ModelArtifact, OcrDevice};

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
    /// Qwen3.5 family (Qwen3.8 included): ChatML, linear-attention layers with fixed-size state.
    Qwen3_5,
}

impl BuiltinArch {
    /// Prompt text for `messages`, ending with an open model turn. `effort` (a reasoning effort, `None`
    /// for off) turns thinking on and leaves the turn inside an open thought (see [`Self::thought`]).
    pub fn render_prompt(self, messages: &[ChatMessage], empty_thought: bool, effort: Option<&str>) -> String {
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
            // The template takes `high`, `low`, or `no_think`. Earlier model turns keep an empty think block.
            Self::HyMt2Moe => {
                let mode = match effort {
                    None => "no_think",
                    Some("minimal" | "low") => "low",
                    Some(_) => "high",
                };
                let mut out = format!("<｜hy_begin▁of▁sentence｜>{}<｜reasoning_mode｜>reasoning_effort:{mode}", system.unwrap_or(""));
                for m in turns {
                    match m.role.as_str() {
                        "user" => out.push_str(&format!("<｜hy_User｜>{}", m.content)),
                        "assistant" => out.push_str(&format!("<｜hy_Assistant｜><think></think>{}<eos:6124c78e>", m.content)),
                        _ => {}
                    }
                }
                out.push_str(if effort.is_some() {
                    "<｜hy_Assistant｜><think>"
                } else {
                    "<｜hy_Assistant｜><think></think>"
                });
                out
            }
            // An empty think block opens every earlier model turn, and the new one when thinking is off
            // (`enable_thinking=false`). Earlier model turns keep their untrimmed reply (the official
            // template trims it), like Gemma 4. Thinking on: the template's effort sentence (`medium` has none).
            Self::Qwen3_5 => {
                let effort_note = match effort {
                    None | Some("medium") => "",
                    Some("minimal" | "low") => {
                        "Reasoning effort is set to low. Keep your thinking brief and focused, moving directly to the conclusion without unnecessary elaboration."
                    }
                    Some(_) => {
                        "Reasoning effort is set to xhigh. Please think carefully through the task, validate key assumptions, consider plausible alternatives, and prioritize correctness, consistency, and clarity in the final answer."
                    }
                };
                let mut out = String::new();
                match (effort_note, system.map(str::trim).filter(|s| !s.is_empty())) {
                    ("", Some(system)) => out.push_str(&format!("<|im_start|>system\n{system}<|im_end|>\n")),
                    (note, Some(system)) => out.push_str(&format!("<|im_start|>system\n{note}\n\n{system}<|im_end|>\n")),
                    ("", None) => {}
                    (note, None) => out.push_str(&format!("<|im_start|>system\n{note}<|im_end|>\n")),
                }
                for m in turns {
                    match m.role.as_str() {
                        "user" => out.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", m.content.trim())),
                        "assistant" => out.push_str(&format!("<|im_start|>assistant\n<think>\n\n</think>\n\n{}<|im_end|>\n", m.content)),
                        _ => {}
                    }
                }
                out.push_str(if effort.is_some() {
                    "<|im_start|>assistant\n<think>\n"
                } else {
                    "<|im_start|>assistant\n<think>\n\n</think>\n\n"
                });
                out
            }
            // Thinking on: `<|think|>` opens the system turn (an otherwise empty one without a system message).
            Self::Gemma4 => {
                // Earlier model turns keep the empty thought channel and their untrimmed reply (the
                // official template drops and trims both): that matches their cached KV, so it gets reused.
                let model = if empty_thought {
                    "model\n<|channel>thought\n<channel|>"
                } else {
                    "model\n"
                };
                let think = if effort.is_some() { "<|think|>\n" } else { "" };
                let mut out = String::from("<bos>");
                if system.is_none() && effort.is_some() {
                    out.push_str("<|turn>system\n<|think|>\n<turn|>\n");
                }
                for m in messages {
                    match m.role.as_str() {
                        "assistant" => out.push_str(&format!("<|turn>{model}{}<turn|>\n", m.content)),
                        "system" => out.push_str(&format!("<|turn>system\n{think}{}<turn|>\n", m.content.trim())),
                        role => out.push_str(&format!("<|turn>{role}\n{}<turn|>\n", m.content.trim())),
                    }
                }
                out.push_str("<|turn>");
                out.push_str(if effort.is_some() { "model\n<|channel>thought\n" } else { model });
                out
            }
        }
    }

    /// A thought: `(open, end, after)`. A thinking prompt ends with `open`, which history's model turns
    /// do not share; the `end` token closes the thought, and the model turn continues with `after`
    /// before the answer. `None`: the model cannot think.
    pub fn thought(self) -> Option<(&'static str, &'static str, &'static str)> {
        match self {
            Self::Gemma4 => Some(("<|channel>thought\n", "<channel|>", "")),
            Self::HyMt2Small | Self::HyMt2Medium => None,
            Self::HyMt2Moe => Some(("<think>", "</think>", "")),
            Self::Qwen3_5 => Some(("<think>\n", "</think>", "\n\n")),
        }
    }

    pub fn eos_ids(self) -> &'static [u32] {
        match self {
            Self::Gemma4 => &[1, 106, 50],
            Self::HyMt2Small => &[120020],
            Self::HyMt2Medium => &[127960, 127967],
            Self::HyMt2Moe => &[120025, 120026],
            Self::Qwen3_5 => &[248046, 248044],
        }
    }

    /// `(temperature, top_p, top_k)` from the model's `generation_config.json`, or its model card when
    /// that has none. `top_k` 0 is off.
    pub fn sampling_defaults(self) -> (f32, f32, u32) {
        match self {
            Self::Gemma4 => (1.0, 0.95, 64),
            Self::HyMt2Small | Self::HyMt2Medium => (0.7, 0.8, 20),
            Self::HyMt2Moe => (0.7, 1.0, 0),
            Self::Qwen3_5 => (1.0, 0.95, 20),
        }
    }

    /// For KV inputs given as `(layer, head_dim)` in decoder input order: whether only
    /// `GroupQueryAttention` reads it, which appends into a preallocated
    /// `[1, heads, context, head_dim]` buffer bound as both past and present. The loader keeps
    /// fixed-size state out of place whatever this returns.
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
            // Every Hy-MT2 and Qwen entry sets `kv_gqa_only`.
            Self::HyMt2Small | Self::HyMt2Medium | Self::HyMt2Moe | Self::Qwen3_5 => vec![true; inputs.len()],
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
    /// The model menu's first level: `"Gemma 4 E4B"`.
    pub family: &'static str,
    /// Its row under the family: `"q4f16 (4.3 GB)"`.
    pub variant: &'static str,
    pub arch: BuiltinArch,
    /// Subdirectory under `models_dir`. External data must sit next to its `.onnx`, and
    /// tokenizers differ per repo, so each repo gets its own flat directory.
    pub dir: &'static str,
    pub files: BuiltinFiles,
    /// The model's chat template opens the model turn with an empty thought channel when thinking
    /// is off (Gemma 4 12B and up), so it answers without thinking.
    pub empty_thought: bool,
}

/// What a model downloads, which picks the runtime.
#[derive(Debug)]
pub enum BuiltinFiles {
    /// Run on ONNX Runtime.
    Onnx(OnnxFiles),
    /// Run on llama.cpp.
    Gguf { file: ModelArtifact, mem: GgufMem },
}

/// What a GGUF model's memory takes besides its file size.
#[derive(Debug)]
pub struct GgufMem {
    /// See [`ArchShape::kv_elems_per_token`].
    pub kv_elems_per_token: u64,
    /// The token embedding tables, which llama.cpp keeps in RAM on any device.
    pub embed_bytes: u64,
    /// The part of `embed_bytes` tied to the output head, which the GPU holds a copy of.
    pub tied_bytes: u64,
}

#[derive(Debug)]
pub struct OnnxFiles {
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
}

impl OnnxFiles {
    /// Tokens per prefill run. A WebGPU run holds the buffers ORT's cache drops until the GPU
    /// finishes it, so each fed token costs VRAM until the run ends (see `memory_estimate`).
    /// fp32 (q4) holds several MB per token (12B: +13 GB at 1500 tokens), and 256-token runs cap it
    /// at no speed cost; fp16 holds far less, and 512-token runs save ~0.4 GB for ~10% slower
    /// prefill, where smaller ones slow it much more.
    pub fn prefill_chunk(&self) -> u64 {
        if self.elem_bytes == 4 { 256 } else { 512 }
    }
}

impl BuiltinModelSpec {
    pub fn is_gguf(&self) -> bool {
        matches!(self.files, BuiltinFiles::Gguf { .. })
    }

    pub fn artifacts(&self) -> Vec<&ModelArtifact> {
        match &self.files {
            BuiltinFiles::Onnx(onnx) => std::iter::once(&onnx.tokenizer).chain(onnx.embed).chain(onnx.decoder).collect(),
            BuiltinFiles::Gguf { file, .. } => vec![file],
        }
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
        let onnx = match &self.files {
            BuiltinFiles::Onnx(onnx) => onnx,
            // Measured on Hy-MT2 1.8B and Gemma 4 E4B (Vulkan and CPU, 4k–32k context), within ~5%.
            BuiltinFiles::Gguf { file, mem } => {
                /// Compute buffers plus Vulkan's or the CPU backend's own, and in RAM the vocab.
                const LLAMA: u64 = 256 << 20;
                // llama.cpp's KV is fp16 and sized for the whole context up front.
                let kv = mem.kv_elems_per_token * 2 * ctx;
                return match device {
                    // The CPU backend repacks Q4_0 weights (Gemma's tied table) into a second copy.
                    OcrDevice::Cpu => {
                        let repacked = if matches!(self.arch, BuiltinArch::Gemma4) {
                            mem.tied_bytes
                        } else {
                            0
                        };
                        (0, file.expected_bytes + repacked + kv + LLAMA)
                    }
                    OcrDevice::Webgpu | OcrDevice::Directml => {
                        (file.expected_bytes - mem.embed_bytes + mem.tied_bytes + kv + LLAMA, mem.embed_bytes + LLAMA)
                    }
                };
            }
        };
        let (s, e) = (onnx.shape, onnx.elem_bytes);
        let kv = s.kv_elems_per_token * e * ctx;
        // ORT WebGPU uploads initializers at their own size, outside the buffer-cache buckets.
        let weights = onnx.decoder.iter().map(|a| a.expected_bytes).sum::<u64>() + s.lm_head_bytes;
        // The embedding session always runs on the CPU, straight from its mmapped external data: a
        // row stays resident once a token reads it, and over time every row can.
        let ram = onnx.embed.iter().map(|a| a.expected_bytes).sum::<u64>() + HOST;
        match device {
            OcrDevice::Webgpu => {
                let chunk = onnx.prefill_chunk();
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
            // shortcut: 26B's QMoE keeps its FC3 scales fp16, so 26B q4f16 is ~0.5 GB high, and Hy-MT2 fp16
            // (and 30B, whose QMoE keeps blocks of 128) comes out 7–11% high; fine unless CPU estimates need
            // to be exact.
            OcrDevice::Cpu => (0, if e == 2 { weights * 10 / 9 } else { weights } + kv + ram),
        }
    }
}

/// Model used when the config leaves `builtin_model` empty.
const DEFAULT_ID: &str = "gemma-4-e4b-it-qat-gguf-q4_k_xl";

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
        remote_name: file_name,
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

/// GGUF only.
const GEMMA4_31B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 704_643_072,
    // 60 layers × K/V: 50 sliding with 16 heads × 256, 10 global with 4 heads × 512.
    kv_elems_per_token: 2 * (50 * 16 * 256 + 10 * 4 * 512),
    heads: 32,
    matmul_attn_layers: 0,
    gqa_512_layers: 10,
};

const HY_MT2_1_8B: &str = "https://huggingface.co/james58899/Hy-MT2-1.8B-ONNX/resolve/b9a81a759e3c5dd319d258ec446dede24724ada4";
const HY_MT2_1_8B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-1.8B-ONNX/resolve/b9a81a759e3c5dd319d258ec446dede24724ada4/onnx";
const HY_MT2_1_8B_TOKENIZER: ModelArtifact = art(HY_MT2_1_8B, "tokenizer.json", 9_527_287);
const HY_MT2_1_8B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 494_870_528,
    // 32 layers × K/V × 4 heads × 128.
    kv_elems_per_token: 32 * 2 * 4 * 128,
    heads: 16,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

const HY_MT2_7B: &str = "https://huggingface.co/james58899/Hy-MT2-7B-ONNX/resolve/b006db3cf76363334fb60b82a1c50ed5ac682ce1";
const HY_MT2_7B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-7B-ONNX/resolve/b006db3cf76363334fb60b82a1c50ed5ac682ce1/onnx";
const HY_MT2_7B_TOKENIZER: ModelArtifact = art(HY_MT2_7B, "tokenizer.json", 16_388_221);
const HY_MT2_7B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 1_049_944_064,
    // 32 layers × K/V × 8 heads × 128.
    kv_elems_per_token: 32 * 2 * 8 * 128,
    heads: 32,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

const HY_MT2_30B: &str = "https://huggingface.co/james58899/Hy-MT2-30B-A3B-ONNX/resolve/31dd1708b5909445b9a363b9b774e62c67959f2b";
const HY_MT2_30B_ONNX: &str = "https://huggingface.co/james58899/Hy-MT2-30B-A3B-ONNX/resolve/31dd1708b5909445b9a363b9b774e62c67959f2b/onnx";
/// The untied `lm_head` sits in the decoder's own external data.
const HY_MT2_30B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 0,
    // 48 layers × K/V × 4 heads × 128.
    kv_elems_per_token: 48 * 2 * 4 * 128,
    heads: 32,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

const QWEN3_8_27B: &str = "https://huggingface.co/james58899/Qwen3.8-27B-3Bit-GSQ-ONNX/resolve/8fb863e04e3f92f68cb853356a83a7ce47409f7d";
const QWEN3_8_27B_ONNX: &str =
    "https://huggingface.co/james58899/Qwen3.8-27B-3Bit-GSQ-ONNX/resolve/8fb863e04e3f92f68cb853356a83a7ce47409f7d/onnx";
/// The untied `lm_head` sits in the decoder's own external data. The 48 linear-attention layers
/// keep 78 MB of fixed-size state, left to the flat allowance.
const QWEN3_8_27B_SHAPE: ArchShape = ArchShape {
    lm_head_bytes: 0,
    // 16 full-attention layers × K/V × 4 heads × 256.
    kv_elems_per_token: 16 * 2 * 4 * 256,
    heads: 24,
    matmul_attn_layers: 0,
    gqa_512_layers: 0,
};

/// Unsloth's and Tencent's GGUF files.
const GEMMA4_E4B_GGUF: &str = "https://huggingface.co/unsloth/gemma-4-E4B-it-qat-GGUF/resolve/8c5a9e4fd5482e2be20fe0bf013b4c262a8f4265";
const GEMMA4_12B_GGUF: &str = "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/980b060c40a8539ac159e0501a3e0f66a6365af3";
const GEMMA4_26B_GGUF: &str = "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-qat-GGUF/resolve/7b92b5b28818151e8669af2e45e88d6086f490dd";
const GEMMA4_31B_GGUF: &str = "https://huggingface.co/unsloth/gemma-4-31B-it-qat-GGUF/resolve/43cc1aeb31adf47ec06a854507ce552cd9862e6f";
const HY_MT2_1_8B_GGUF: &str = "https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/a0c709d9fac510f2c807aa3af52872340dc37a4a";
const HY_MT2_7B_GGUF: &str = "https://huggingface.co/tencent/Hy-MT2-7B-GGUF/resolve/ab8472660ac61fac25f1af43fac2599d52a8a775";
const HY_MT2_30B_GGUF: &str = "https://huggingface.co/tencent/Hy-MT2-30B-A3B-GGUF/resolve/fd3dcbb6b31e9e03923ef4f9f42500f74c3851c3";
const QWEN3_8_27B_GGUF: &str = "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/4af51b3542f775b0a8e4b310bfb3f17e01c306c9";

/// GGUF token embedding tables. Q4_0 packs 32 weights into 18 bytes and Q8_0 into 34; Q4_K packs
/// 256 into 144 and Q6_K into 210.
const GEMMA4_E4B_EMBD: u64 = 262_144 * 2560 * 18 / 32;
/// E4B's per-layer table: 42 layers × 256.
const GEMMA4_E4B_LAYER_EMBD: u64 = 262_144 * 42 * 256 * 18 / 32;
const GEMMA4_12B_EMBD: u64 = 262_144 * 3840 * 18 / 32;
const GEMMA4_26B_EMBD: u64 = 262_144 * 2816 * 18 / 32;
const GEMMA4_31B_EMBD: u64 = 262_144 * 5376 * 18 / 32;
/// Q4_K_M keeps the table Q6_K too.
const HY_MT2_1_8B_EMBD_Q6_K: u64 = 120_818 * 2048 * 210 / 256;
const HY_MT2_1_8B_EMBD_Q8_0: u64 = 120_818 * 2048 * 34 / 32;
const HY_MT2_7B_EMBD_Q6_K: u64 = 128_167 * 4096 * 210 / 256;
const HY_MT2_7B_EMBD_Q8_0: u64 = 128_167 * 4096 * 34 / 32;
const HY_MT2_30B_EMBD_Q4_K: u64 = 120_832 * 2048 * 144 / 256;
const HY_MT2_30B_EMBD_Q8_0: u64 = 120_832 * 2048 * 34 / 32;
const QWEN3_8_27B_EMBD_Q4_K: u64 = 248_320 * 5120 * 144 / 256;

const fn mem(shape: &ArchShape, embed_bytes: u64, tied_bytes: u64) -> GgufMem {
    GgufMem {
        kv_elems_per_token: shape.kv_elems_per_token,
        embed_bytes,
        tied_bytes,
    }
}

const fn gguf(
    id: &'static str,
    (family, variant): (&'static str, &'static str),
    arch: BuiltinArch,
    dir: &'static str,
    file: ModelArtifact,
    mem: GgufMem,
    empty_thought: bool,
) -> BuiltinModelSpec {
    BuiltinModelSpec {
        id,
        family,
        variant,
        arch,
        dir,
        files: BuiltinFiles::Gguf { file, mem },
        empty_thought,
    }
}

/// Grouped by family, each sorted by download size.
pub const BUILTIN_MODELS: &[BuiltinModelSpec] = &[
    gguf(
        "gemma-4-e4b-it-qat-gguf-q4_k_xl",
        ("Gemma 4 E4B", "Q4_K_XL (4.2 GB)"),
        BuiltinArch::Gemma4,
        "gemma-4-e4b-it-qat",
        art(GEMMA4_E4B_GGUF, "gemma-4-E4B-it-qat-UD-Q4_K_XL.gguf", 4_215_695_776),
        mem(&GEMMA4_E4B_SHAPE, GEMMA4_E4B_EMBD + GEMMA4_E4B_LAYER_EMBD, GEMMA4_E4B_EMBD),
        false,
    ),
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4f16",
        family: "Gemma 4 E4B",
        variant: "q4f16 (4.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "gemma-4-e4b-it-qat-q4",
        family: "Gemma 4 E4B",
        variant: "q4 (4.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-e4b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: false,
    },
    gguf(
        "gemma-4-12b-it-qat-gguf-q4_k_xl",
        ("Gemma 4 12B", "Q4_K_XL (6.7 GB)"),
        BuiltinArch::Gemma4,
        "gemma-4-12b-it-qat",
        art(GEMMA4_12B_GGUF, "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf", 6_716_356_800),
        mem(&GEMMA4_12B_SHAPE, GEMMA4_12B_EMBD, GEMMA4_12B_EMBD),
        true,
    ),
    BuiltinModelSpec {
        id: "gemma-4-12b-it-qat-q4f16",
        family: "Gemma 4 12B",
        variant: "q4f16 (6.8 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-12b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "gemma-4-12b-it-qat-q4",
        family: "Gemma 4 12B",
        variant: "q4 (7.6 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-12b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: true,
    },
    gguf(
        "gemma-4-26b-a4b-it-qat-gguf-q4_k_xl",
        ("Gemma 4 26B A4B", "Q4_K_XL (14.2 GB)"),
        BuiltinArch::Gemma4,
        "gemma-4-26b-a4b-it-qat",
        art(GEMMA4_26B_GGUF, "gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf", 14_249_047_104),
        mem(&GEMMA4_26B_SHAPE, GEMMA4_26B_EMBD, GEMMA4_26B_EMBD),
        true,
    ),
    BuiltinModelSpec {
        id: "gemma-4-26b-a4b-it-qat-q4f16",
        family: "Gemma 4 26B A4B",
        variant: "q4f16 (14.3 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-26b-a4b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: true,
    },
    BuiltinModelSpec {
        id: "gemma-4-26b-a4b-it-qat-q4",
        family: "Gemma 4 26B A4B",
        variant: "q4 (15.9 GB)",
        arch: BuiltinArch::Gemma4,
        dir: "gemma-4-26b-a4b-it-qat",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: true,
    },
    gguf(
        "gemma-4-31b-it-qat-gguf-q4_k_xl",
        ("Gemma 4 31B", "Q4_K_XL (17.3 GB)"),
        BuiltinArch::Gemma4,
        "gemma-4-31b-it-qat",
        art(GEMMA4_31B_GGUF, "gemma-4-31B-it-qat-UD-Q4_K_XL.gguf", 17_287_670_048),
        mem(&GEMMA4_31B_SHAPE, GEMMA4_31B_EMBD, GEMMA4_31B_EMBD),
        true,
    ),
    gguf(
        "hy-mt2-1.8b-gguf-q4_k_m",
        ("Hy-MT2 1.8B", "Q4_K_M (1.1 GB)"),
        BuiltinArch::HyMt2Small,
        "hy-mt2-1.8b",
        art(HY_MT2_1_8B_GGUF, "Hy-MT2-1.8B-Q4_K_M.gguf", 1_133_080_448),
        mem(&HY_MT2_1_8B_SHAPE, HY_MT2_1_8B_EMBD_Q6_K, HY_MT2_1_8B_EMBD_Q6_K),
        false,
    ),
    BuiltinModelSpec {
        id: "hy-mt2-1.8b-q4f16",
        family: "Hy-MT2 1.8B",
        variant: "q4f16 (1.4 GB)",
        arch: BuiltinArch::HyMt2Small,
        dir: "hy-mt2-1.8b",
        files: BuiltinFiles::Onnx(OnnxFiles {
            tokenizer: HY_MT2_1_8B_TOKENIZER,
            embed: &[
                art(HY_MT2_1_8B_ONNX, "embed_tokens_q4f16.onnx", 355),
                art(HY_MT2_1_8B_ONNX, "embed_tokens_q4f16.onnx_data", 494_870_528),
            ],
            decoder: &[
                art(HY_MT2_1_8B_ONNX, "decoder_model_merged_q4f16.onnx", 238_998),
                art(HY_MT2_1_8B_ONNX, "decoder_model_merged_q4f16.onnx_data", 868_487_168),
            ],

            elem_bytes: 2,
            shape: &HY_MT2_1_8B_SHAPE,
            kv_gqa_only: true,
        }),
        empty_thought: false,
    },
    gguf(
        "hy-mt2-1.8b-gguf-q6_k",
        ("Hy-MT2 1.8B", "Q6_K (1.5 GB)"),
        BuiltinArch::HyMt2Small,
        "hy-mt2-1.8b",
        art(HY_MT2_1_8B_GGUF, "Hy-MT2-1.8B-Q6_K.gguf", 1_474_785_120),
        mem(&HY_MT2_1_8B_SHAPE, HY_MT2_1_8B_EMBD_Q6_K, HY_MT2_1_8B_EMBD_Q6_K),
        false,
    ),
    gguf(
        "hy-mt2-1.8b-gguf-q8_0",
        ("Hy-MT2 1.8B", "Q8_0 (1.9 GB)"),
        BuiltinArch::HyMt2Small,
        "hy-mt2-1.8b",
        art(HY_MT2_1_8B_GGUF, "Hy-MT2-1.8B-Q8_0.gguf", 1_908_528_192),
        mem(&HY_MT2_1_8B_SHAPE, HY_MT2_1_8B_EMBD_Q8_0, HY_MT2_1_8B_EMBD_Q8_0),
        false,
    ),
    BuiltinModelSpec {
        id: "hy-mt2-1.8b-fp16",
        family: "Hy-MT2 1.8B",
        variant: "fp16 (3.6 GB)",
        arch: BuiltinArch::HyMt2Small,
        dir: "hy-mt2-1.8b",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: false,
    },
    gguf(
        "hy-mt2-7b-gguf-q4_k_m",
        ("Hy-MT2 7B", "Q4_K_M (4.6 GB)"),
        BuiltinArch::HyMt2Medium,
        "hy-mt2-7b",
        art(HY_MT2_7B_GGUF, "Hy-MT2-7B-Q4_K_M.gguf", 4_624_648_896),
        mem(&HY_MT2_7B_SHAPE, HY_MT2_7B_EMBD_Q6_K, HY_MT2_7B_EMBD_Q6_K),
        false,
    ),
    BuiltinModelSpec {
        id: "hy-mt2-7b-q4f16",
        family: "Hy-MT2 7B",
        variant: "q4f16 (5.0 GB)",
        arch: BuiltinArch::HyMt2Medium,
        dir: "hy-mt2-7b",
        files: BuiltinFiles::Onnx(OnnxFiles {
            tokenizer: HY_MT2_7B_TOKENIZER,
            embed: &[
                art(HY_MT2_7B_ONNX, "embed_tokens_q4f16.onnx", 356),
                art(HY_MT2_7B_ONNX, "embed_tokens_q4f16.onnx_data", 1_049_944_064),
            ],
            decoder: &[
                art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx", 240_352),
                art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_986_805_760),
                art(HY_MT2_7B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_939_595_264),
            ],

            elem_bytes: 2,
            shape: &HY_MT2_7B_SHAPE,
            kv_gqa_only: true,
        }),
        empty_thought: false,
    },
    // Tencent names these two files `HY-`.
    gguf(
        "hy-mt2-7b-gguf-q6_k",
        ("Hy-MT2 7B", "Q6_K (6.2 GB)"),
        BuiltinArch::HyMt2Medium,
        "hy-mt2-7b",
        art(HY_MT2_7B_GGUF, "HY-MT2-7B-Q6_K.gguf", 6_164_482_720),
        mem(&HY_MT2_7B_SHAPE, HY_MT2_7B_EMBD_Q6_K, HY_MT2_7B_EMBD_Q6_K),
        false,
    ),
    gguf(
        "hy-mt2-7b-gguf-q8_0",
        ("Hy-MT2 7B", "Q8_0 (8.0 GB)"),
        BuiltinArch::HyMt2Medium,
        "hy-mt2-7b",
        art(HY_MT2_7B_GGUF, "HY-MT2-7B-Q8_0.gguf", 7_981_928_896),
        mem(&HY_MT2_7B_SHAPE, HY_MT2_7B_EMBD_Q8_0, HY_MT2_7B_EMBD_Q8_0),
        false,
    ),
    BuiltinModelSpec {
        id: "hy-mt2-7b-fp16",
        family: "Hy-MT2 7B",
        variant: "fp16 (15.0 GB)",
        arch: BuiltinArch::HyMt2Medium,
        dir: "hy-mt2-7b",
        files: BuiltinFiles::Onnx(OnnxFiles {
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
        }),
        empty_thought: false,
    },
    BuiltinModelSpec {
        id: "hy-mt2-30b-a3b-q4f16",
        family: "Hy-MT2 30B A3B",
        variant: "q4f16 (16.3 GB)",
        arch: BuiltinArch::HyMt2Moe,
        dir: "hy-mt2-30b-a3b",
        files: BuiltinFiles::Onnx(OnnxFiles {
            tokenizer: art(HY_MT2_30B, "tokenizer.json", 9_527_108),
            embed: &[
                art(HY_MT2_30B_ONNX, "embed_tokens_q4f16.onnx", 355),
                art(HY_MT2_30B_ONNX, "embed_tokens_q4f16.onnx_data", 494_927_872),
            ],
            decoder: &[
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx", 498_756),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_999_667_200),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_955_577_856),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_954_529_280),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 1_954_529_280),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_4", 1_954_529_280),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_5", 1_954_529_280),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_6", 1_954_529_280),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_7", 1_617_104_896),
                art(HY_MT2_30B_ONNX, "decoder_model_merged_q4f16.onnx_data_8", 494_927_872),
            ],

            elem_bytes: 2,
            shape: &HY_MT2_30B_SHAPE,
            kv_gqa_only: true,
        }),
        empty_thought: false,
    },
    gguf(
        "hy-mt2-30b-a3b-gguf-q4_k_m",
        ("Hy-MT2 30B A3B", "Q4_K_M (18.2 GB)"),
        BuiltinArch::HyMt2Moe,
        "hy-mt2-30b-a3b",
        art(HY_MT2_30B_GGUF, "Hy-MT2-30B-A3B-Q4_K_M.gguf", 18_236_702_880),
        mem(&HY_MT2_30B_SHAPE, HY_MT2_30B_EMBD_Q4_K, 0),
        false,
    ),
    gguf(
        "hy-mt2-30b-a3b-gguf-q8_0",
        ("Hy-MT2 30B A3B", "Q8_0 (32.0 GB)"),
        BuiltinArch::HyMt2Moe,
        "hy-mt2-30b-a3b",
        art(HY_MT2_30B_GGUF, "Hy-MT2-30B-A3B-Q8_0.gguf", 31_985_729_376),
        mem(&HY_MT2_30B_SHAPE, HY_MT2_30B_EMBD_Q8_0, 0),
        false,
    ),
    // ISTA-DASLab's 3-bit GSQ levels, stored losslessly in 4-bit blocks (ORT has no 3-bit kernel). Each
    // 128-weight group is four blocks of 32 sharing its scale: ORT WebGPU's fast prefill path takes
    // only block 32 (7× the prefill speed for 1.2 GB of repeated scales).
    BuiltinModelSpec {
        id: "qwen3.8-27b-q4f16",
        family: "Qwen3.8 27B",
        variant: "q4f16 (15.2 GB)",
        arch: BuiltinArch::Qwen3_5,
        dir: "qwen3.8-27b",
        files: BuiltinFiles::Onnx(OnnxFiles {
            tokenizer: art(QWEN3_8_27B, "tokenizer.json", 12_809_320),
            embed: &[
                art(QWEN3_8_27B_ONNX, "embed_tokens_q4f16.onnx", 717),
                art(QWEN3_8_27B_ONNX, "embed_tokens_q4f16.onnx_data", 685_363_200),
            ],
            decoder: &[
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx", 616_490),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data", 1_999_589_376),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_1", 1_983_737_856),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_2", 1_976_776_704),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_3", 1_983_758_336),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_4", 1_999_568_896),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_5", 1_976_776_704),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_6", 1_817_501_696),
                art(QWEN3_8_27B_ONNX, "decoder_model_merged_q4f16.onnx_data_7", 715_161_600),
            ],

            elem_bytes: 2,
            shape: &QWEN3_8_27B_SHAPE,
            kv_gqa_only: true,
        }),
        empty_thought: false,
    },
    gguf(
        "qwen3.8-27b-gguf-q4_k_m",
        ("Qwen3.8 27B", "Q4_K_M (16.6 GB)"),
        BuiltinArch::Qwen3_5,
        "qwen3.8-27b",
        art(QWEN3_8_27B_GGUF, "Qwen3.8-27B-UD-Q4_K_M.gguf", 16_564_259_744),
        mem(&QWEN3_8_27B_SHAPE, QWEN3_8_27B_EMBD_Q4_K, 0),
        false,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_entries_are_complete_and_pinned() {
        for (i, m) in BUILTIN_MODELS.iter().enumerate() {
            assert!(BUILTIN_MODELS[..i].iter().all(|o| o.id != m.id), "duplicate id {}", m.id);
            let earlier = &BUILTIN_MODELS[..i];
            assert!(earlier.iter().all(|o| (o.family, o.variant) != (m.family, m.variant)), "{}", m.id);
            // The family menu lists each family once, so its models sit together.
            assert!(earlier.last().is_none_or(|o| o.family == m.family) || earlier.iter().all(|o| o.family != m.family), "{}", m.id);
            match &m.files {
                BuiltinFiles::Onnx(o) => {
                    assert_eq!(o.tokenizer.file_name, "tokenizer.json", "{}", m.id);
                    assert!(o.embed[0].file_name.ends_with(".onnx") && o.decoder[0].file_name.ends_with(".onnx"), "{}", m.id);
                    assert!(o.shape.kv_elems_per_token > 0, "{}", m.id);
                    assert!(o.shape.lm_head_bytes <= o.embed[1].expected_bytes, "{}", m.id);
                }
                BuiltinFiles::Gguf { file, mem } => {
                    assert!(file.file_name.ends_with(".gguf") && mem.kv_elems_per_token > 0, "{}", m.id);
                    assert!(mem.tied_bytes <= mem.embed_bytes && mem.embed_bytes < file.expected_bytes, "{}", m.id);
                }
            }
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
        assert_eq!(BuiltinArch::Gemma4.render_prompt(&messages, false, None), expected);
        // Earlier model turns keep the reply untrimmed and the empty thought channel, so the cached KV stays a prefix.
        assert_eq!(
            BuiltinArch::Gemma4.render_prompt(&messages, true, None),
            expected.replace("<|turn>model\n", "<|turn>model\n<|channel>thought\n<channel|>")
        );
        // Thinking: `<|think|>` opens the system turn (its own turn without a system message) and the
        // new model turn opens the thought channel.
        assert_eq!(
            BuiltinArch::Gemma4.render_prompt(&messages, false, Some("low")),
            "<bos><|turn>system\n<|think|>\nsys<turn|>\n<|turn>user\nu1<turn|>\n<|turn>model\na1\n<turn|>\n<|turn>user\nu2<turn|>\n<|turn>model\n<|channel>thought\n"
        );
        assert_eq!(
            BuiltinArch::Gemma4.render_prompt(&messages[1..2], true, Some("high")),
            "<bos><|turn>system\n<|think|>\n<turn|>\n<|turn>user\nu1<turn|>\n<|turn>model\n<|channel>thought\n"
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
            assert_eq!(arch.render_prompt(&messages, false, None), expected, "{arch:?}");
        }
        // Qwen: `apply_chat_template(messages, add_generation_prompt=True, enable_thinking=False)`, but
        // with the earlier reply untrimmed.
        let qwen = "<|im_start|>system\nsys<|im_end|>\n<|im_start|>user\nu1<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\na1\n<|im_end|>\n<|im_start|>user\nu2<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n";
        assert_eq!(BuiltinArch::Qwen3_5.render_prompt(&messages, false, None), qwen);
        // Without a system message, 7B's first user turn opens with `<|startoftext|>`.
        assert_eq!(BuiltinArch::HyMt2Medium.render_prompt(&messages[1..2], false, None), "<|startoftext|> u1<|extra_0|>");

        // Thinking on: the template's effort setting, and the new model turn left inside an open thought.
        let hy_moe = cases[2].1;
        assert_eq!(
            BuiltinArch::HyMt2Moe.render_prompt(&messages, false, Some("minimal")),
            hy_moe.replace("no_think", "low").strip_suffix("</think>").unwrap()
        );
        assert!(
            BuiltinArch::HyMt2Moe
                .render_prompt(&messages, false, Some("max"))
                .contains("reasoning_effort:high<")
        );
        let qwen_open = qwen.strip_suffix("\n</think>\n\n").unwrap();
        assert_eq!(BuiltinArch::Qwen3_5.render_prompt(&messages, false, Some("medium")), qwen_open);
        assert_eq!(
            BuiltinArch::Qwen3_5.render_prompt(&messages, false, Some("low")),
            qwen_open.replace(
                "system\nsys",
                "system\nReasoning effort is set to low. Keep your thinking brief and focused, moving directly to the conclusion without unnecessary elaboration.\n\nsys"
            )
        );
        assert!(
            BuiltinArch::Qwen3_5
                .render_prompt(&messages[1..2], false, Some("high"))
                .starts_with("<|im_start|>system\nReasoning effort is set to xhigh. ")
        );
        // Models that cannot think ignore the effort.
        assert_eq!(BuiltinArch::HyMt2Small.render_prompt(&messages, false, Some("high")), cases[0].1);
    }

    #[test]
    fn next_prompt_shares_the_prompt_before_its_thought() {
        let msg = |role: &str, content: &str| ChatMessage {
            role: role.into(),
            content: content.into(),
            reasoning_content: None,
        };
        let asked = [msg("system", "sys"), msg("user", "u1")];
        let next = [msg("system", "sys"), msg("user", "u1"), msg("assistant", "a1"), msg("user", "u2")];
        for arch in [BuiltinArch::Gemma4, BuiltinArch::HyMt2Moe, BuiltinArch::Qwen3_5] {
            for empty_thought in [false, true] {
                // The checkpoint the model takes before `open`, which the next page's history (its
                // thought dropped) still starts with.
                let (open, ..) = arch.thought().unwrap();
                let prompt = arch.render_prompt(&asked, empty_thought, Some("high"));
                let checkpoint = prompt.strip_suffix(open).unwrap_or_else(|| panic!("{arch:?}: {prompt}"));
                let next = arch.render_prompt(&next, empty_thought, Some("high"));
                assert!(next.starts_with(checkpoint), "{arch:?}:\n{next}\n{checkpoint}");
            }
        }
    }

    #[test]
    fn memory_estimate_covers_gguf_and_onnx_across_devices() {
        let gemma_gguf = find("gemma-4-e4b-it-qat-gguf-q4_k_xl").unwrap();
        let hymt_gguf = find("hy-mt2-1.8b-gguf-q4_k_m").unwrap();
        let onnx_q4 = find("gemma-4-e4b-it-qat-q4").unwrap();
        let onnx_q4f16 = find("gemma-4-e4b-it-qat-q4f16").unwrap();

        assert_eq!(gemma_gguf.memory_estimate(OcrDevice::Cpu, 4096).0, 0, "CPU uses no VRAM");

        // GPU GGUF: DirectML matches WebGPU; context grows VRAM only.
        let (vram_gpu_h4, ram_gpu_h4) = hymt_gguf.memory_estimate(OcrDevice::Webgpu, 4096);
        let (vram_gpu_h8, ram_gpu_h8) = hymt_gguf.memory_estimate(OcrDevice::Webgpu, 8192);
        assert_eq!(hymt_gguf.memory_estimate(OcrDevice::Directml, 4096), (vram_gpu_h4, ram_gpu_h4));
        assert_eq!(ram_gpu_h8, ram_gpu_h4);
        assert!(vram_gpu_h8 > vram_gpu_h4);

        // ONNX: no DirectML estimate; WebGPU context grows VRAM only, CPU grows RAM only.
        assert_eq!(onnx_q4.memory_estimate(OcrDevice::Directml, 4096), (0, 0));
        let (vram_w4, ram_w4) = onnx_q4.memory_estimate(OcrDevice::Webgpu, 4096);
        let (vram_w8, ram_w8) = onnx_q4.memory_estimate(OcrDevice::Webgpu, 8192);
        assert_eq!(ram_w4, ram_w8);
        assert!(vram_w8 > vram_w4);

        let (vram_c4, ram_c4) = onnx_q4f16.memory_estimate(OcrDevice::Cpu, 4096);
        let (vram_c8, ram_c8) = onnx_q4f16.memory_estimate(OcrDevice::Cpu, 8192);
        assert_eq!((vram_c4, vram_c8), (0, 0));
        assert!(ram_c8 > ram_c4);
    }

    #[test]
    fn render_prompt_handles_empty_and_whitespace_system_prompts() {
        let user = ChatMessage {
            role: "user".into(),
            content: "hi".into(),
            reasoning_content: None,
        };
        let empty_sys = ChatMessage {
            role: "system".into(),
            content: "".into(),
            reasoning_content: None,
        };
        let ws_sys = ChatMessage {
            role: "system".into(),
            content: "  \t\n ".into(),
            reasoning_content: None,
        };

        assert_eq!(BuiltinArch::HyMt2Medium.render_prompt(&[empty_sys.clone(), user.clone()], false, None), "<|startoftext|>hi<|extra_0|>");
        assert_eq!(
            BuiltinArch::Qwen3_5.render_prompt(&[ws_sys.clone(), user.clone()], false, None),
            "<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        // A blank system message with thinking on is just the effort sentence.
        assert!(
            BuiltinArch::Qwen3_5
                .render_prompt(&[ws_sys.clone(), user.clone()], false, Some("low"))
                .starts_with("<|im_start|>system\nReasoning effort is set to low. Keep your thinking brief and focused, moving directly to the conclusion without unnecessary elaboration.<|im_end|>\n")
        );
        assert_eq!(
            BuiltinArch::Gemma4.render_prompt(&[ws_sys, user.clone()], false, None),
            "<bos><|turn>system\n<turn|>\n<|turn>user\nhi<turn|>\n<|turn>model\n"
        );
        assert!(
            BuiltinArch::HyMt2Small
                .render_prompt(&[empty_sys.clone(), user.clone()], false, None)
                .contains("<｜hy_place▁holder▁no▁3｜>")
        );
        assert!(
            BuiltinArch::HyMt2Moe
                .render_prompt(&[empty_sys, user], false, None)
                .starts_with("<｜hy_begin▁of▁sentence｜><｜reasoning_mode｜>")
        );
    }
}
