//! Built-in LLM: download, load, and generation with an on-device KV cache.
//!
//! GGUF models run on llama.cpp ([`crate::builtin::llama`]); the rest of this file runs ONNX ones.
//! Those follow the onnx-community layout: `embed_tokens` (ids → embeddings) feeding a merged
//! decoder with `past_key_values.*` inputs and `present.*` outputs: attention KV that grows with the
//! context, and fixed-size state (Qwen3.5's linear-attention layers).

mod catalog;
mod custom;
mod llama;

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Instant,
};

use half::f16;
use ort::{
    ep,
    memory::{AllocationDevice, Allocator, AllocatorType, MemoryInfo, MemoryType},
    session::Session,
    value::{DynTensor, DynValue, Tensor, TensorElementType},
};
use tokenizers::Tokenizer;
use tokio::sync::{OwnedMutexGuard, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::info;
use translator_core::{ApiConfig, ModelKind, OcrDevice, PipelineStatus, ReplyMode, download_artifacts, part_path};

pub use crate::builtin::{
    catalog::{BUILTIN_MODELS, BuiltinModelSpec, find as find_builtin_model},
    custom::{CUSTOM_ID, custom_models},
};
use crate::{
    TranslateError,
    builtin::{
        catalog::{BuiltinFiles, OnnxFiles},
        custom::{CUSTOM_DIR, SAMPLING_DEFAULTS},
    },
    http::ChatMessage,
};

/// A reply whose last `LOOP_TOKENS` tokens (and at least three periods) repeat one span of up to
/// `LOOP_TOKENS` tokens is stuck in a loop.
const LOOP_TOKENS: usize = 64;

/// State checkpoints a model keeps for state its KV cache cannot cut back: one per recent prompt,
/// taken before the prompt's open thought (which history does not repeat), so a retry, a prompt after
/// a cancelled one, or one whose history dropped a reply's thought restarts from the last prompt it
/// shares. Each is a copy of that state (Qwen3.8 27B: ~150 MB).
const MAX_CHECKPOINTS: usize = 3;

/// DRY settings: llama.cpp's recommended multiplier and base.
const DRY_MULTIPLIER: f32 = 0.8;
const DRY_BASE: f32 = 1.75;
/// Above llama.cpp's 2, so a short repeat the source itself has (`あああ`, a doubled word) still
/// translates as written.
const DRY_ALLOWED_LENGTH: usize = 4;
/// A repeat match stops at a token holding one of these, so JSON quoting never counts as a repeat.
const DRY_BREAKERS: [char; 4] = ['\n', ':', '"', '*'];
/// Gemma 4's thought channel markers. No answer may hold one (a thought only comes before it);
/// sampled anyway, they would show as text and teach the history to repeat them.
const THOUGHT_TOKENS: [&str; 2] = ["<|channel>", "<channel|>"];

/// The loaded model, if any. Lives behind a mutex on [`crate::TranslateClient`].
#[derive(Default)]
pub(crate) struct BuiltinBackend {
    loaded: Option<Loaded>,
}

impl BuiltinBackend {
    /// Drop the model (frees VRAM). The next request loads again.
    pub(crate) fn unload(&mut self) {
        self.loaded = None;
    }
}

struct Loaded {
    id: String,
    device: OcrDevice,
    engine: Engine,
}

enum Engine {
    Onnx(Box<OnnxModel>),
    Llama(Box<llama::Model>),
}

/// Download / load the model selected in `api` unless it is already loaded: its catalog spec, or
/// `None` for the user's own GGUF. `on_status` gets each download / load phase, then `None` once
/// loading ends (either way).
pub(crate) async fn ensure_loaded(
    backend: &mut BuiltinBackend,
    api: &ApiConfig,
    models_dir: &Path,
    cancel: &CancellationToken,
    on_status: &mut impl FnMut(Option<PipelineStatus>),
) -> Result<Option<&'static BuiltinModelSpec>, TranslateError> {
    let picked = api.builtin_model.trim();
    let spec = match picked.strip_prefix(CUSTOM_ID) {
        Some("") => return Err(TranslateError::Other("no custom GGUF file picked".into())),
        // A bare file name, so the path stays in the folder.
        Some(file) if file.contains(['/', '\\', ':']) => {
            return Err(TranslateError::Other(format!("invalid custom GGUF file name {file:?}")));
        }
        Some(_) => None,
        None => Some(find_builtin_model(picked).ok_or_else(|| {
            let ids: Vec<_> = BUILTIN_MODELS.iter().map(|m| m.id).collect();
            TranslateError::Other(format!("unknown built-in model {picked:?} (available: {})", ids.join(", ")))
        })?),
    };
    // The catalog's id when `picked` is empty.
    let id = spec.map_or(picked, |s| s.id);
    let device = api.builtin_device;
    // DirectML's GroupQueryAttention ignores `local_window_size`: Gemma 4's sliding-window layers
    // then read the whole context and the reply turns to garbage past 512 tokens.
    if device == OcrDevice::Directml && spec.is_some_and(|s| matches!(s.files, BuiltinFiles::Onnx(_))) {
        return Err(TranslateError::Other("built-in models do not support DirectML; use WebGPU or CPU".into()));
    }
    if backend.loaded.as_ref().is_some_and(|l| l.id == id && l.device == device) {
        return Ok(spec);
    }
    // A preload cancelled by Stop may only get the lock after Stop unloaded.
    if cancel.is_cancelled() {
        return Err(TranslateError::Cancelled);
    }
    backend.unload();
    let engine: Result<Engine, TranslateError> = async {
        let loading = PipelineStatus::LoadingModels {
            kind: ModelKind::Translation,
        };
        // Vulkan for any GPU device.
        let gpu = device != OcrDevice::Cpu;
        let model_id = id.to_owned();
        let job = match spec {
            Some(spec) => {
                let dir = models_dir.join(spec.dir);
                download_missing(spec, &dir, cancel, on_status).await?;
                on_status(Some(loading));
                tokio::task::spawn_blocking(move || match &spec.files {
                    BuiltinFiles::Onnx(onnx) => OnnxModel::load(spec, onnx, device, &dir).map(|m| Engine::Onnx(Box::new(m))),
                    BuiltinFiles::Gguf { file, .. } => {
                        llama::Model::load(model_id, &dir.join(file.file_name), gpu).map(|m| Engine::Llama(Box::new(m)))
                    }
                })
            }
            None => {
                let path = models_dir.join(CUSTOM_DIR).join(&id[CUSTOM_ID.len()..]);
                if !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
                    return Err(TranslateError::Other(format!("{} not found", path.display())));
                }
                on_status(Some(loading));
                tokio::task::spawn_blocking(move || llama::Model::load(model_id, &path, gpu).map(|m| Engine::Llama(Box::new(m))))
            }
        };
        job.await
            .map_err(|e| TranslateError::Other(format!("built-in model load task: {e}")))?
            .map_err(|e| TranslateError::Other(format!("load {id}: {e}")))
    }
    .await;
    on_status(None);
    backend.loaded = Some(Loaded {
        id: id.to_owned(),
        device,
        engine: engine?,
    });
    info!(model = id, ?device, "built-in model loaded");
    Ok(spec)
}

/// Load the model if needed (see [`ensure_loaded`]), then generate one completion: the answer only.
/// A thought stays out of history (it would fill the context); the next prompt restarts from the
/// checkpoint the model took before the reply.
pub(crate) async fn complete(
    mut backend: OwnedMutexGuard<BuiltinBackend>,
    api: &ApiConfig,
    models_dir: &Path,
    messages: &[ChatMessage],
    cancel: &CancellationToken,
    on_status: &mut impl FnMut(Option<PipelineStatus>),
    on_text: &mut impl FnMut(&str),
) -> Result<String, TranslateError> {
    let spec = ensure_loaded(&mut backend, api, models_dir, cancel, on_status).await?;

    // Catalog models that can think do when an effort is set; the user's own GGUF never does.
    let effort = api
        .reasoning_effort
        .as_deref()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !matches!(e.as_str(), "" | "none" | "off"));
    let thought = effort.as_ref().and(spec.and_then(|s| s.arch.thought()));
    // `None` for the user's own GGUF, which the loaded model renders.
    let prompt = spec.map(|s| s.arch.render_prompt(messages, s.empty_thought, effort.as_deref()));
    let chat = if prompt.is_none() { messages.to_vec() } else { Vec::new() };
    // The last message is the source payload, and a structured reply mirrors its JSON.
    let source = messages.last().map_or_else(String::new, |m| m.content.clone());
    let output_ratio = api.builtin_output_ratio;
    let max_tokens_cap = api.max_tokens.map_or(usize::MAX, |n| n as usize);
    let context = api.builtin_context_tokens as usize;
    // Unset knobs take the model's recommended values; temperature 0 is greedy.
    let (temperature, top_p, top_k) = spec.map_or(SAMPLING_DEFAULTS, |s| s.arch.sampling_defaults());
    let decoding = Decoding {
        sampling: Some(Sampling {
            temperature: api.temperature.unwrap_or(temperature),
            top_p: api.top_p.unwrap_or(top_p),
            // 0 turns top-k off.
            top_k: match api.top_k.unwrap_or(top_k) {
                0 => usize::MAX,
                k => k as usize,
            },
        })
        .filter(|s| s.temperature > 0.0),
        dry: api.builtin_dry,
        shape: Some(ReplyShape {
            ids: crate::peek_translation_pairs(&source).into_iter().map(|(id, _)| id).collect(),
        })
        .filter(|s| !s.ids.is_empty() && api.reply_mode == ReplyMode::Structured),
        thought,
    };
    // `None`: one more thought token.
    let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();
    let cancel_job = cancel.clone();
    let job = tokio::task::spawn_blocking(move || {
        let result = (|| {
            let Some(loaded) = backend.loaded.as_mut() else {
                return Ok(Err(TranslateError::Cancelled));
            };
            // A bare plain-text source can be a token or two; the floor leaves its reply room.
            let max_tokens = |source_tokens: usize| ((output_ratio * source_tokens as f32).ceil() as usize).max(16).min(max_tokens_cap);
            let on_text = |text: Option<&str>| drop(tx.send(text.map(str::to_owned)));
            match &mut loaded.engine {
                Engine::Onnx(model) => {
                    let max_tokens = max_tokens(model.tokenizer.encode(source, false)?.len());
                    // Every ONNX model is a catalog one, so it has a prompt.
                    model.generate(&prompt.unwrap_or_default(), max_tokens, context, decoding, &cancel_job, on_text)
                }
                Engine::Llama(model) => {
                    let max_tokens = max_tokens(model.token_count(&source)?);
                    let prompt = match prompt {
                        Some(prompt) => prompt,
                        None => model.chat_prompt(&chat)?,
                    };
                    model.generate(&prompt, max_tokens, context, decoding, &cancel_job, on_text)
                }
            }
        })();
        // The lock outlives the sender, so the next job's status cannot land before this one's cleanup.
        (backend, result)
    });
    // Ends when the job drops its sender. The thinking status ends with the first answer text, or
    // with the job however it ends.
    let mut thought_tokens = 0;
    while let Some(text) = rx.recv().await {
        match text {
            None => {
                thought_tokens += 1;
                on_status(Some(PipelineStatus::Thinking {
                    tokens: Some(thought_tokens),
                }));
            }
            Some(text) => {
                if std::mem::take(&mut thought_tokens) > 0 {
                    on_status(None);
                }
                on_text(&text);
            }
        }
    }
    if thought_tokens > 0 {
        on_status(None);
    }
    match job.await {
        Ok((_, Ok(result))) => result,
        Ok((_, Err(e))) => Err(TranslateError::Other(format!("{}: {e}", spec.map_or(api.builtin_model.trim(), |s| s.id)))),
        Err(e) => Err(TranslateError::Other(format!("built-in model task: {e}"))),
    }
}

async fn download_missing(
    spec: &BuiltinModelSpec,
    dir: &Path,
    cancel: &CancellationToken,
    on_status: &mut impl FnMut(Option<PipelineStatus>),
) -> Result<(), TranslateError> {
    let missing: Vec<_> = spec.artifacts().into_iter().filter(|a| !dir.join(a.file_name).is_file()).collect();
    if missing.is_empty() {
        return Ok(());
    }
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| TranslateError::Other(format!("create {}: {e}", dir.display())))?;
    let download = download_artifacts(ModelKind::Translation, &missing, dir, |status| on_status(Some(status)));
    tokio::select! {
        biased;
        () = cancel.cancelled() => {
            for art in &missing {
                let _ = tokio::fs::remove_file(part_path(dir, art)).await;
            }
            Err(TranslateError::Cancelled)
        }
        result = download => result.map_err(|e| TranslateError::Other(e.to_string())),
    }
}

struct OnnxModel {
    spec: &'static BuiltinModelSpec,
    onnx: &'static OnnxFiles,
    device: OcrDevice,
    tokenizer: Tokenizer,
    embed: Session,
    /// In decoder input order.
    kv: Vec<KvSlot>,
    kv_type: TensorElementType,
    per_layer_inputs: bool,
    cache: Option<KvCache>,
    rng: fastrand::Rng,
    /// Tokens holding a [`DRY_BREAKERS`] character.
    dry_breakers: HashSet<u32>,
    /// The vocab's [`THOUGHT_TOKENS`], never sampled.
    thought_tokens: Vec<u32>,
    /// Allocates the in-place KV buffers.
    kv_alloc: Allocator,
    /// Declared last so it drops last: freeing a KV buffer or `kv_alloc` after the session that
    /// owns their WebGPU context is an access violation inside ORT.
    decoder: Session,
}

struct KvSlot {
    past: String,
    present: String,
    /// The input's shape, dynamic dimensions negative: `[batch, heads, sequence, head_dim]` for
    /// attention KV. A shape whose only dynamic dimension is the batch, or with none, is fixed-size state.
    dims: Vec<i64>,
    /// See [`crate::builtin::catalog::BuiltinArch::kv_in_place`].
    in_place: bool,
}

impl KvSlot {
    /// `dims` for batch 1 and a `sequence`-long sequence axis.
    fn shape(&self, sequence: i64) -> Vec<i64> {
        self.dims
            .iter()
            .enumerate()
            .map(|(i, &d)| match (i, d) {
                (_, d) if d >= 0 => d,
                (0, _) => 1,
                _ => sequence,
            })
            .collect()
    }
}

/// Device KV for `ids` (every token fed so far), reused when the next prompt extends it.
struct KvCache {
    /// `ids` as text. Matched as text because re-encoding a reply can split it differently
    /// from the tokens the model generated (`算了`+`吧` vs `算`+`了吧`).
    text: String,
    ids: Vec<u32>,
    past: Vec<DynValue>,
    /// Sequence capacity of the in-place buffers.
    context: usize,
    /// Out-of-place KV and fixed-size state at each recent prompt's checkpoint, oldest first (see
    /// [`MAX_CHECKPOINTS`]). In-place KV needs none: it gets overwritten past the shared tokens.
    checkpoints: Vec<Checkpoint>,
}

/// The out-of-place `past` values (decoder input order) for `ids[..tokens]`.
struct Checkpoint {
    tokens: usize,
    past: Vec<DynValue>,
}

fn session(device: OcrDevice, path: &Path) -> ort::Result<Session> {
    // [`complete`] rejects DirectML before loading.
    let eps = match device {
        OcrDevice::Webgpu => vec![ep::WebGPU::default().build().error_on_failure(), ep::CPU::default().build()],
        OcrDevice::Cpu | OcrDevice::Directml => vec![ep::CPU::default().build()],
    };
    Session::builder(translator_core::ort_environment()?)?
        .with_execution_providers(eps)?
        .commit_from_file(path)
}

/// Where the KV cache lives. Build this rather than reading [`Allocator::memory_info`]: ort's
/// `MemoryInfo::to_owned` (used by [`Allocator::new`]) always returns CPU memory info.
fn kv_memory(device: OcrDevice) -> ort::Result<MemoryInfo<'static>> {
    let kv_device = match device {
        OcrDevice::Webgpu => AllocationDevice::WEBGPU_BUFFER,
        OcrDevice::Cpu | OcrDevice::Directml => AllocationDevice::CPU,
    };
    MemoryInfo::new(kv_device, 0, AllocatorType::Device, MemoryType::Default)
}

fn missing(what: &str) -> ort::Error {
    ort::Error::new(format!("model has no {what}"))
}

fn argmax(values: &[f32]) -> u32 {
    let mut best = (0, f32::NEG_INFINITY);
    for (i, &v) in values.iter().enumerate() {
        if v > best.1 {
            best = (i, v);
        }
    }
    best.0 as u32
}

/// Whether `reply` ends in a loop (see [`LOOP_TOKENS`]).
fn ends_in_loop(reply: &[u32]) -> bool {
    (1..=LOOP_TOKENS).any(|period| {
        let span = LOOP_TOKENS.max(3 * period);
        reply.len() >= span && {
            let tail = &reply[reply.len() - span..];
            tail[period..].iter().zip(tail).all(|(a, b)| a == b)
        }
    })
}

/// Where the checkpoint of prompt `ids` sits: before its open thought (`open`, which history does not
/// repeat), and before its last token, which a retry of the same prompt still feeds for the logits.
fn checkpoint_at(ids: &[u32], open: &[u32]) -> usize {
    let at = if ids.ends_with(open) { ids.len() - open.len() } else { ids.len() };
    at.min(ids.len().saturating_sub(1))
}

/// If `next` is the open thought's end token, ends the thought: pushes `next` and the tokens that
/// follow it, and returns where the answer starts.
fn end_thought(thought: &mut Option<(u32, Vec<u32>)>, next: u32, ids: &mut Vec<u32>) -> Option<usize> {
    let (end, after) = thought.take_if(|(end, _)| *end == next)?;
    ids.push(end);
    ids.extend(after);
    Some(ids.len())
}

/// DRY ("Don't Repeat Yourself"): a token that would extend a span of `reply` already seen earlier
/// in `reply` loses `DRY_MULTIPLIER × DRY_BASE^(len − DRY_ALLOWED_LENGTH)` from its logit, `len`
/// being the span's length. Only the reply counts: copying names from the source is expected.
fn apply_dry(logits: &mut [f32], reply: &[u32], breakers: &HashSet<u32>) {
    let Some(last) = reply.len().checked_sub(1) else {
        return;
    };
    let mut longest = HashMap::<u32, usize>::new();
    for end in 0..last {
        let len = (0..=end)
            .take_while(|&k| reply[end - k] == reply[last - k] && !breakers.contains(&reply[end - k]))
            .count();
        if len >= DRY_ALLOWED_LENGTH {
            let next = longest.entry(reply[end + 1]).or_default();
            *next = (*next).max(len);
        }
    }
    for (token, len) in longest {
        if let Some(logit) = logits.get_mut(token as usize) {
            *logit -= DRY_MULTIPLIER * DRY_BASE.powi((len - DRY_ALLOWED_LENGTH) as i32);
        }
    }
}

/// How a reply's tokens get picked (see [`next_token`]).
struct Decoding {
    /// `None` decodes greedily.
    sampling: Option<Sampling>,
    /// See [`apply_dry`].
    dry: bool,
    /// `None` leaves the reply unconstrained.
    shape: Option<ReplyShape>,
    /// The prompt opened a thought: see [`crate::builtin::catalog::BuiltinArch::thought`]. The
    /// reply's text, shape, and `max_tokens` cover only the answer after it.
    thought: Option<(&'static str, &'static str, &'static str)>,
}

/// Holds a reply to `{"b":[[id,"text"],...]}` with exactly the source's ids, in order. Small models
/// otherwise write `，` for `,` or a stray `]`, and the parser loses every block after the slip.
struct ReplyShape {
    /// Not empty.
    ids: Vec<u32>,
}

/// Progress through a [`ReplyShape`]: block `lit`'s literal (`lit` = ids.len() is the closing one),
/// then its text.
#[derive(Clone, Copy, Default)]
struct ShapeState {
    lit: usize,
    /// Bytes of literal `lit` written.
    written: usize,
    in_text: bool,
    escape: Escape,
    /// The text's last char is `]`. A text never holds `]]`, `],` or `]}`: that is the model closing
    /// it without `"`, after a `」` it took for the quote.
    bracket: bool,
}

#[derive(Clone, Copy, Default)]
enum Escape {
    #[default]
    None,
    Backslash,
    /// `\u` hex digits left.
    Hex(u8),
}

impl ReplyShape {
    /// The fixed text before block `i`'s string content, or the reply's end for the last `i`.
    fn literal(&self, i: usize) -> String {
        match self.ids.get(i) {
            Some(id) if i == 0 => format!("{{\"b\":[[{id},\""),
            Some(id) => format!("\"],[{id},\""),
            None => "\"]]}".into(),
        }
    }

    fn done(&self, s: ShapeState) -> bool {
        s.lit == self.ids.len() && s.written == self.literal(s.lit).len()
    }

    /// The state after `piece`, or `None` if `piece` breaks the shape.
    fn feed(&self, mut s: ShapeState, piece: &str) -> Option<ShapeState> {
        let mut lit = self.literal(s.lit);
        for c in piece.chars() {
            if !s.in_text {
                if !lit[s.written..].starts_with(c) {
                    return None;
                }
                s.written += c.len_utf8();
                // The closing literal has no text after it.
                s.in_text = s.written == lit.len() && s.lit < self.ids.len();
                continue;
            }
            if s.bracket && matches!(c, ']' | ',' | '}') {
                return None;
            }
            s.bracket = c == ']' && matches!(s.escape, Escape::None);
            s.escape = match (s.escape, c) {
                (Escape::None, '"') => {
                    s.in_text = false;
                    s.lit += 1;
                    lit = self.literal(s.lit);
                    s.written = 1;
                    Escape::None
                }
                (Escape::None, '\\') => Escape::Backslash,
                (Escape::None, c) if c < ' ' => return None,
                (Escape::None, _) => Escape::None,
                (Escape::Backslash, 'u') => Escape::Hex(4),
                (Escape::Backslash, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') => Escape::None,
                (Escape::Hex(1), c) if c.is_ascii_hexdigit() => Escape::None,
                (Escape::Hex(n), c) if c.is_ascii_hexdigit() => Escape::Hex(n - 1),
                _ => return None,
            };
        }
        Some(s)
    }
}

/// Random token choice. Without it a retry after an unparsable reply repeats the same reply.
#[derive(Clone, Copy)]
struct Sampling {
    /// Above 0.
    temperature: f32,
    /// 1 keeps every token.
    top_p: f32,
    /// At least 1; `usize::MAX` keeps every token.
    top_k: usize,
}

impl Sampling {
    fn pick(&self, logits: &[f32], rng: &mut fastrand::Rng) -> u32 {
        // One pass keeping the best `top_k`; most tokens lose to the weakest kept one and are skipped.
        let mut cand: Vec<(u32, f32)> = Vec::with_capacity(self.top_k.min(logits.len()));
        let mut weakest = 0;
        for (i, &l) in logits.iter().enumerate() {
            let c = (i as u32, l);
            if cand.len() < self.top_k {
                cand.push(c);
            } else if c.1 > cand[weakest].1 {
                cand[weakest] = c;
            } else {
                continue;
            }
            if cand.len() == self.top_k {
                weakest = (0..cand.len()).min_by(|&a, &b| cand[a].1.total_cmp(&cand[b].1)).unwrap_or(0);
            }
        }
        let by_logit = |a: &(u32, f32), b: &(u32, f32)| b.1.total_cmp(&a.1);
        // Only top-p needs the order; a full-vocab sort per token is the slow part.
        if self.top_p < 1.0 {
            cand.sort_unstable_by(by_logit);
        }
        let max = cand.iter().map(|c| c.1).fold(f32::NEG_INFINITY, f32::max);
        let weights: Vec<f32> = cand.iter().map(|c| ((c.1 - max) / self.temperature).exp()).collect();
        let mut kept = weights.len();
        if self.top_p < 1.0 {
            let target = self.top_p * weights.iter().sum::<f32>();
            let mut sum = 0.0;
            kept = weights
                .iter()
                .position(|w| {
                    sum += w;
                    sum >= target
                })
                .map_or(kept, |i| i + 1);
        }
        let mut r = rng.f32() * weights[..kept].iter().sum::<f32>();
        for (c, w) in cand.iter().zip(&weights[..kept]) {
            if r < *w {
                return c.0;
            }
            r -= w;
        }
        // Float rounding left `r` past the last weight.
        cand[kept - 1].0
    }
}

/// The reply's next token from `logits`: DRY, sampling, then the reply shape, whose state it advances.
/// `piece` gives a token's text, or `None` for one that ends the reply. `None`: no token fits the shape.
fn next_token(
    decoding: &Decoding,
    logits: &mut [f32],
    reply: &[u32],
    dry_breakers: &HashSet<u32>,
    rng: &mut fastrand::Rng,
    shape_state: &mut ShapeState,
    piece: impl Fn(u32) -> ort::Result<Option<String>>,
) -> ort::Result<Option<u32>> {
    if decoding.dry {
        apply_dry(logits, reply, dry_breakers);
    }
    let next = match &decoding.sampling {
        Some(s) => s.pick(logits, rng),
        None => argmax(logits),
    };
    let Some(shape) = &decoding.shape else {
        return Ok(Some(next));
    };
    // The shape ends the reply itself, so no end token fits it.
    let fits = |t: u32| -> ort::Result<Option<ShapeState>> { Ok(piece(t)?.and_then(|p| shape.feed(*shape_state, &p))) };
    let fitted = match fits(next)? {
        Some(s) => Some((next, s)),
        // The model's best token that fits. Ending, or closing a text without `"`, means the
        // text is done, so then the best one that closes it.
        None => {
            let closing = shape_state.in_text && piece(next)?.is_none_or(|p| p.starts_with([']', ',', '}']));
            let mut order: Vec<u32> = (0..logits.len() as u32).collect();
            order.sort_unstable_by(|&a, &b| logits[b as usize].total_cmp(&logits[a as usize]));
            let mut found = None;
            for t in order {
                if let Some(s) = fits(t)?
                    && (!closing || s.lit > shape_state.lit)
                {
                    found = Some((t, s));
                    break;
                }
            }
            found
        }
    };
    let Some((next, state)) = fitted else {
        return Ok(None);
    };
    *shape_state = state;
    Ok(Some(next))
}

impl OnnxModel {
    fn load(spec: &'static BuiltinModelSpec, onnx: &'static OnnxFiles, device: OcrDevice, dir: &Path) -> ort::Result<Self> {
        let tokenizer = Tokenizer::from_file(dir.join(onnx.tokenizer.file_name))?;
        // A table lookup: as fast on the CPU, and keeps the embedding tables out of VRAM.
        let embed = session(OcrDevice::Cpu, &dir.join(onnx.embed[0].file_name))?;
        let decoder = session(device, &dir.join(onnx.decoder[0].file_name))?;

        let mut kv = Vec::new();
        let mut kv_type = TensorElementType::Float32;
        for input in decoder.inputs() {
            let Some(layer) = input.name().strip_prefix("past_key_values.") else {
                continue;
            };
            let dims = input.dtype().tensor_shape().ok_or_else(|| missing("KV shape"))?;
            kv.push(KvSlot {
                past: input.name().to_string(),
                present: format!("present.{layer}"),
                dims: dims.to_vec(),
                in_place: false,
            });
            kv_type = input.dtype().tensor_type().unwrap_or(kv_type);
        }
        // `present.22.key` -> layer `22`.
        let layers: Vec<_> = kv
            .iter()
            .map(|s| (s.present["present.".len()..].rsplit_once('.').map_or("", |(layer, _)| layer), s.dims.last().copied().unwrap_or(0)))
            .collect();
        let in_place = spec.arch.kv_in_place(&layers, onnx.kv_gqa_only);
        for (slot, in_place) in kv.iter_mut().zip(in_place) {
            // Fixed-size state has no sequence axis to append along.
            slot.in_place = in_place && slot.dims.iter().skip(1).any(|&d| d < 0);
        }
        let per_layer_inputs = decoder.inputs().iter().any(|i| i.name() == "per_layer_inputs");
        let kv_alloc = Allocator::new(&decoder, kv_memory(device)?)?;
        let dry_breakers = tokenizer
            .get_vocab(true)
            .into_iter()
            .filter(|(t, _)| t.contains(DRY_BREAKERS))
            .map(|(_, id)| id)
            .collect();
        let thought_tokens = THOUGHT_TOKENS.iter().filter_map(|t| tokenizer.token_to_id(t)).collect();
        let in_place = kv.iter().filter(|s| s.in_place).count();
        info!(model = spec.id, kv_inputs = kv.len(), in_place, ?kv_type, per_layer_inputs, "built-in model sessions ready");
        Ok(Self {
            spec,
            onnx,
            device,
            tokenizer,
            embed,
            kv,
            kv_type,
            per_layer_inputs,
            cache: None,
            rng: fastrand::Rng::new(),
            dry_breakers,
            thought_tokens,
            kv_alloc,
            decoder,
        })
    }

    /// The cache for a fresh prefill: in-place slots get a `context`-long buffer, the rest are empty
    /// (fixed-size state: zeros).
    /// In-place buffers of `stale` (a dropped cache with the same `context`, or empty) are kept: the
    /// prefill overwrites them from position 0, and ORT frees a dropped buffer only after the next
    /// run, so new ones would briefly hold a second copy of the KV.
    fn empty_past(&self, context: usize, stale: Vec<DynValue>) -> ort::Result<Vec<DynValue>> {
        let mut stale = stale.into_iter();
        self.kv
            .iter()
            .map(|s| {
                let old = stale.next();
                if s.in_place {
                    if let Some(old) = old {
                        return Ok(old);
                    }
                    return Ok(DynTensor::new(&self.kv_alloc, self.kv_type, s.shape(context as i64))?.into_dyn());
                }
                let shape = s.shape(0);
                let len = shape.iter().product::<i64>() as usize;
                Ok(match self.kv_type {
                    TensorElementType::Float16 => Tensor::from_array((shape, vec![f16::ZERO; len]))?.into_dyn(),
                    _ => Tensor::from_array((shape, vec![0_f32; len]))?.into_dyn(),
                })
            })
            .collect()
    }

    /// The answer. `on_text` gets `None` after each thought token, then the whole answer so far after
    /// each token. The inner error is [`TranslateError::Cancelled`], [`TranslateError::ContextFull`], or
    /// [`TranslateError::Looping`] ([`TranslateError::Other`] when decoding greedily).
    fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        context: usize,
        decoding: Decoding,
        cancel: &CancellationToken,
        mut on_text: impl FnMut(Option<&str>),
    ) -> ort::Result<Result<String, TranslateError>> {
        // The open thought's end token and the tokens that follow it (`None` once it ends), and the
        // tokens that opened it.
        let (mut thought, open) = match decoding.thought {
            Some((open, end, after)) => (
                Some((
                    self.tokenizer.token_to_id(end).ok_or_else(|| missing(end))?,
                    self.tokenizer.encode(after, false)?.get_ids().to_vec(),
                )),
                self.tokenizer.encode(open, false)?.get_ids().to_vec(),
            ),
            None => (None, Vec::new()),
        };
        let thinking_decoding = Decoding {
            shape: None,
            thought: None,
            ..decoding
        };
        let all_in_place = self.kv.iter().all(|s| s.in_place);
        let cache = self.cache.take().filter(|c| c.context == context);
        let (mut ids, reused) = match &cache {
            // Empty text: a request stopped mid-prompt; its tokens still match below.
            Some(c) if !c.text.is_empty() && prompt.len() > c.text.len() && prompt.starts_with(&c.text) => {
                ([c.ids.as_slice(), self.tokenizer.encode(&prompt[c.text.len()..], false)?.get_ids()].concat(), c.ids.len())
            }
            // In-place KV past the shared tokens just gets overwritten, so a retry, a request after a
            // cancelled one, or history that dropped a reply's thought still reuses what comes before.
            // Out-of-place KV and fixed-size state go back to the last checkpoint within it.
            // shortcut: trimmed history shares only the system prompt, where no checkpoint sits, so those
            // models re-prefill everything; checkpoint after the system prompt too if that gets slow.
            Some(c) => {
                let ids = self.tokenizer.encode(prompt, false)?.get_ids().to_vec();
                // Feed at least one token for the logits.
                let shared = ids
                    .iter()
                    .zip(&c.ids)
                    .take_while(|(a, b)| a == b)
                    .count()
                    .min(ids.len().saturating_sub(1));
                // All of the cache shared (e.g. after a stop mid-prompt): its state goes on as is.
                let reused = if all_in_place || shared == c.ids.len() {
                    shared
                } else {
                    c.checkpoints.iter().map(|k| k.tokens).filter(|&t| t <= shared).max().unwrap_or(0)
                };
                (ids, reused)
            }
            None => (self.tokenizer.encode(prompt, false)?.get_ids().to_vec(), 0),
        };
        let prompt_len = ids.len();
        if prompt_len > context {
            self.cache = cache;
            return Ok(Err(TranslateError::ContextFull(context)));
        }
        let checkpoint_at = checkpoint_at(&ids, &open);
        let (mut past, mut checkpoints) = match cache {
            Some(mut c) if reused > 0 => {
                c.checkpoints.retain(|k| k.tokens <= reused);
                if reused < c.ids.len() && !all_in_place {
                    // `reused` is the last checkpoint's; it moves into `past`.
                    let mut restored = c.checkpoints.pop().map(|k| k.past).unwrap_or_default().into_iter();
                    c.past = self
                        .kv
                        .iter()
                        .zip(c.past)
                        .map(|(slot, value)| if slot.in_place { Some(value) } else { restored.next() })
                        .collect::<Option<_>>()
                        .ok_or_else(|| missing("checkpoint value"))?;
                }
                (c.past, c.checkpoints)
            }
            stale => (self.empty_past(context, stale.map_or_else(Vec::new, |c| c.past))?, Vec::new()),
        };
        let mut fed = reused;
        let start = Instant::now();
        let mut prefill = None;
        let kv_mem = kv_memory(self.device)?;
        let cpu_mem = MemoryInfo::new(AllocationDevice::CPU, 0, AllocatorType::Device, MemoryType::Default)?;
        let eos = self.spec.arch.eos_ids();
        let mut binding = self.decoder.create_binding()?;
        let mut text = String::new();
        let mut logits_f32 = Vec::new();
        let mut looping = false;
        let mut shape_state = ShapeState::default();
        // Where the answer starts: after the thought, if any.
        let mut answer = prompt_len;
        let chunk = self.onnx.prefill_chunk() as usize;
        // Ends the request early; the cache is kept all the same.
        let mut stopped = None;
        loop {
            if cancel.is_cancelled() {
                stopped = Some(TranslateError::Cancelled);
                break;
            }
            let mut total = ids.len().min(fed.saturating_add(chunk));
            // Stop at the checkpoint for the state there.
            if fed < checkpoint_at {
                total = total.min(checkpoint_at);
            }
            let step: Vec<i64> = ids[fed..total].iter().map(|&t| i64::from(t)).collect();
            let n = step.len();
            let mut embedded = self.embed.run(ort::inputs!["input_ids" => Tensor::from_array(([1, n], step))?])?;
            binding.bind_input("inputs_embeds", &embedded.remove("inputs_embeds").ok_or_else(|| missing("inputs_embeds"))?)?;
            if self.per_layer_inputs {
                binding.bind_input("per_layer_inputs", &embedded.remove("per_layer_inputs").ok_or_else(|| missing("per_layer_inputs"))?)?;
            }
            drop(embedded);
            binding.bind_input("attention_mask", &Tensor::from_array(([1, total], vec![1_i64; total]))?)?;
            binding.bind_input("position_ids", &Tensor::from_array(([1, n], (fed as i64..total as i64).collect::<Vec<_>>()))?)?;
            binding.bind_input("num_logits_to_keep", &Tensor::from_array(((), vec![1_i64]))?)?;
            // Rebind every step: a device-bound output keeps the last run's buffer, and the KV grows.
            binding.clear_outputs();
            binding.bind_output_to_device("logits", &cpu_mem)?;
            // The state at the checkpoint, kept. A run only reads its out-of-place inputs, so they
            // stay as they are.
            let mut kept = (fed == checkpoint_at && !all_in_place).then(Vec::new);
            for (slot, value) in self.kv.iter().zip(std::mem::take(&mut past)) {
                binding.bind_input(&slot.past, &value)?;
                if slot.in_place {
                    // Same buffer as the input: GQA writes the new tokens at their positions.
                    binding.bind_output(&slot.present, value)?;
                } else {
                    binding.bind_output_to_device(&slot.present, &kv_mem)?;
                    if let Some(kept) = &mut kept {
                        kept.push(value);
                    }
                }
            }
            if let Some(past) = kept {
                checkpoints.retain(|k| k.tokens < fed);
                if checkpoints.len() == MAX_CHECKPOINTS {
                    checkpoints.remove(0);
                }
                checkpoints.push(Checkpoint { tokens: fed, past });
            }

            let mut outputs = self.decoder.run_binding(&binding)?;
            let logits = outputs.remove("logits").ok_or_else(|| missing("logits"))?;
            past = self
                .kv
                .iter()
                .map(|s| outputs.remove(&s.present).ok_or_else(|| missing(&s.present)))
                .collect::<ort::Result<_>>()?;
            drop(outputs);
            fed = total;
            if fed < ids.len() {
                continue;
            }
            prefill.get_or_insert_with(|| start.elapsed());

            logits_f32.clear();
            match logits.dtype().tensor_type() {
                Some(TensorElementType::Float16) => logits_f32.extend(logits.try_extract_tensor::<f16>()?.1.iter().map(|v| v.to_f32())),
                _ => logits_f32.extend_from_slice(logits.try_extract_tensor::<f32>()?.1),
            }
            let piece = |t: u32| -> ort::Result<Option<String>> {
                Ok(if eos.contains(&t) {
                    None
                } else {
                    Some(self.tokenizer.decode(&[t], false)?)
                })
            };
            let step_decoding = if thought.is_some() {
                &thinking_decoding
            } else {
                for &t in &self.thought_tokens {
                    logits_f32[t as usize] = f32::NEG_INFINITY;
                }
                &decoding
            };
            let reply = &ids[answer..];
            let Some(next) = next_token(step_decoding, &mut logits_f32, reply, &self.dry_breakers, &mut self.rng, &mut shape_state, piece)?
            else {
                stopped = Some(TranslateError::Other("no token fits the reply JSON".into()));
                break;
            };
            if let Some(start) = end_thought(&mut thought, next, &mut ids) {
                answer = start;
            } else if eos.contains(&next) {
                break;
            } else {
                ids.push(next);
                if thought.is_some() {
                    on_text(None);
                } else {
                    text = self.tokenizer.decode(&ids[answer..], true)?;
                    // A byte-level token can end mid-character, which decodes to U+FFFD until the next token completes it.
                    if !text.ends_with('\u{FFFD}') {
                        on_text(Some(&text));
                    }
                    if decoding.shape.as_ref().is_some_and(|s| s.done(shape_state)) {
                        break;
                    }
                }
            }
            if ends_in_loop(&ids[prompt_len..]) {
                looping = true;
                break;
            }
            // A thought is bounded by the context only.
            if thought.is_none() && ids.len() - answer >= max_tokens {
                break;
            }
            // The next step would feed all of `ids`; a cut-off reply would not parse anyway.
            if ids.len() > context {
                stopped = Some(TranslateError::ContextFull(context));
                break;
            }
        }
        // With special tokens, so a thought's markers stay in. Empty while the prompt is only partly
        // fed: the next request matches tokens instead.
        let text_fed = if fed < prompt_len {
            String::new()
        } else {
            format!("{prompt}{}", self.tokenizer.decode(&ids[prompt_len..fed], false)?)
        };
        self.cache = Some(KvCache {
            text: text_fed,
            ids: ids[..fed].to_vec(),
            past,
            context,
            checkpoints,
        });
        if let Some(e) = stopped {
            return Ok(Err(e));
        }
        // The prefill feeds every uncached prompt token; each later step feeds one.
        let prefill = prefill.unwrap_or_default();
        let decode = start.elapsed() - prefill;
        info!(
            model = self.spec.id,
            prompt_tokens = prompt_len,
            reused,
            output_tokens = ids.len() - prompt_len,
            prefill_ms = prefill.as_millis() as u64,
            prefill_tps = format_args!("{:.1}", (prompt_len - reused) as f64 / prefill.as_secs_f64()),
            decode_ms = decode.as_millis() as u64,
            decode_tps = format_args!("{:.1}", (fed - prompt_len) as f64 / decode.as_secs_f64()),
            "built-in model generated"
        );
        Ok(match (looping, decoding.sampling) {
            (false, _) => Ok(text),
            (true, Some(_)) => Err(TranslateError::Looping),
            // A greedy retry would replay the same loop, so fail without one.
            (true, None) => {
                Err(TranslateError::Other("model reply got stuck repeating itself; temperature 0 would repeat it on retry".into()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn argmax_picks_first_max() {
        assert_eq!(argmax(&[1.0, 3.0, 3.0, -1.0]), 1);
    }

    #[test]
    fn a_thought_ends_on_its_end_token_only_while_open() {
        let mut thought = Some((9, vec![7]));
        let mut ids = vec![1, 2];
        assert_eq!(end_thought(&mut thought, 5, &mut ids), None);
        assert_eq!((ids.as_slice(), thought.is_some()), (&[1, 2][..], true));
        assert_eq!(end_thought(&mut thought, 9, &mut ids), Some(4));
        assert_eq!((ids.as_slice(), thought.is_none()), (&[1, 2, 9, 7][..], true));
        // Once ended, the same token is left to the EOS check.
        assert_eq!(end_thought(&mut thought, 9, &mut ids), None);

        // Before the open thought, else before the last token.
        assert_eq!(checkpoint_at(&[1, 2, 3, 4], &[3, 4]), 2);
        assert_eq!(checkpoint_at(&[1, 2, 3, 4], &[]), 3);
        assert_eq!(checkpoint_at(&[1, 2, 3, 4], &[5]), 3);
        assert_eq!(checkpoint_at(&[4], &[4]), 0);
    }

    #[test]
    fn loop_needs_a_long_repeated_tail() {
        let period = |p: u32, n: usize| Vec::from_fn(n, |i| i as u32 % p);
        assert!(ends_in_loop(&period(1, LOOP_TOKENS)));
        assert!(!ends_in_loop(&period(1, LOOP_TOKENS - 1)));
        assert!(ends_in_loop(&[vec![99; 5], period(7, LOOP_TOKENS)].concat()));
        // A long period needs three repeats, not just `LOOP_TOKENS` tokens.
        assert!(!ends_in_loop(&period(40, 2 * 40)));
        assert!(ends_in_loop(&period(40, 3 * 40)));
        assert!(!ends_in_loop(&(0..500).collect::<Vec<_>>()));
    }

    #[test]
    fn dry_penalizes_extending_a_repeat() {
        let reply = [1, 2, 3, 4, 5, 9, 1, 2, 3, 4, 5];
        let mut logits = [0.0; 10];
        apply_dry(&mut logits, &reply, &HashSet::new());
        // `1 2 3 4 5` repeats (length 5), so `9` would extend it.
        let mut expected = [0.0; 10];
        expected[9] = -DRY_MULTIPLIER * DRY_BASE;
        assert_eq!(logits, expected);
        // A breaker inside the span cuts the match below the allowed length.
        let mut logits = [0.0; 10];
        apply_dry(&mut logits, &reply, &HashSet::from([3]));
        assert_eq!(logits, [0.0; 10]);
    }

    #[test]
    fn sampling_respects_top_k_and_top_p() {
        let logits = [0.0, 2.0, 1.9, -1.0, 1.8];
        let mut rng = fastrand::Rng::with_seed(7);
        let pick = |s: Sampling, rng: &mut fastrand::Rng| Vec::from_fn(200, |_| s.pick(&logits, rng));
        let s = Sampling {
            temperature: 1.0,
            top_p: 1.0,
            top_k: 2,
        };
        let picks = pick(s, &mut rng);
        assert!(picks.iter().all(|&t| t == 1 || t == 2) && picks.contains(&1) && picks.contains(&2));
        // Tokens 1 and 2 hold ~0.34 and ~0.31 of the mass, so 0.5 keeps exactly those two.
        let picks = pick(
            Sampling {
                top_p: 0.5,
                top_k: usize::MAX,
                ..s
            },
            &mut rng,
        );
        assert!(picks.iter().all(|&t| t == 1 || t == 2) && picks.contains(&2));
        assert!(
            pick(
                Sampling {
                    top_p: 0.0,
                    top_k: usize::MAX,
                    ..s
                },
                &mut rng
            )
            .iter()
            .all(|&t| t == 1)
        );
    }

    #[test]
    fn reply_shape_holds_the_source_ids() {
        let shape = ReplyShape { ids: vec![3, 12] };
        let feed = |pieces: &[&str]| {
            pieces
                .iter()
                .try_fold(ShapeState::default(), |s, p| shape.feed(s, p))
                .map(|s| shape.done(s))
        };
        // Any split into tokens; `"`, `\\`, `\n`, `\t` and `\uXXXX` escapes stay inside the text.
        assert_eq!(feed(&[r#"{"b":[[3,"早"#, r#"安\"\\\u00"#, r#"e9\n\t"],"#, r#"[12,""#, r#""]]}"#]), Some(true));
        assert_eq!(feed(&[r#"{"b":[[3,"a"],[12,"#]), Some(false));
        assert_eq!(feed(&[r#"{"b":[[3,"[HP] [MP]"#]), Some(false));
        // The slips 1.8B and 7B make (7B closes a text with `」`, then `]]}`), a wrong or skipped id,
        // anything after the end, control chars, or an incomplete `\u` closed early.
        for bad in [
            r#"{"b":[[3，"#,
            r#"{"b":[[3],"#,
            r#"{"b":[[3,"a」]]"#,
            r#"{"b":[[3,"a」],"#,
            r#"{"b":[[3,"a」]}"#,
            r#"{"b":[[4,"#,
            r#"{"b":[[3,"a"]]}"#,
            r#"{"b":[[3,"a"],[12,"b"]]} "#,
            "{\"b\":[[3,\"a\nb",
            "{\"b\":[[3,\"a\tb",
            r#"{"b":[[3,"\x"#,
            r#"{"b":[[3,"\u00e"]"#,
            r#"{"b":[[3,"\u00g"#,
        ] {
            assert_eq!(feed(&[bad]), None, "{bad}");
        }
    }

    #[test]
    fn dry_threshold_sampling_temperature_and_kv_slot_shape() {
        // Len 3 < DRY_ALLOWED_LENGTH (4) leaves logits intact; len == 4 penalizes by -DRY_MULTIPLIER.
        let mut logits_short = [0.0; 10];
        apply_dry(&mut logits_short, &[1, 2, 3, 9, 1, 2, 3], &HashSet::new());
        assert_eq!(logits_short, [0.0; 10]);

        let mut logits_exact = [0.0; 10];
        apply_dry(&mut logits_exact, &[1, 2, 3, 4, 9, 1, 2, 3, 4], &HashSet::new());
        assert_eq!(logits_exact[9], -DRY_MULTIPLIER);

        // Sampling excludes NEG_INFINITY and sharpens at low temperature.
        let logits = [f32::NEG_INFINITY, 3.0, 2.0, f32::NEG_INFINITY];
        let mut rng = fastrand::Rng::with_seed(11);
        let cold = Sampling {
            temperature: 0.25,
            top_p: 1.0,
            top_k: usize::MAX,
        };
        let picks = Vec::from_fn(200, |_| cold.pick(&logits, &mut rng));
        assert!(picks.iter().all(|&t| t == 1 || t == 2));
        assert!(picks.iter().filter(|&&t| t == 1).count() > 180);

        // KvSlot::shape preserves fixed state and substitutes dynamic batch/sequence dims.
        let attn = KvSlot {
            past: "past_key_values.0.key".into(),
            present: "present.0.key".into(),
            dims: vec![-1, 4, -1, 256],
            in_place: true,
        };
        assert_eq!(attn.shape(128), [1, 4, 128, 256]);
        let fixed = KvSlot {
            past: "past_key_values.0.conv_state".into(),
            present: "present.0.conv_state".into(),
            dims: vec![-1, 8, 128],
            in_place: false,
        };
        assert_eq!(fixed.shape(128), [1, 8, 128]);
    }

    #[test]
    fn next_token_handles_fallback_closing_heuristic_and_exhaustion() {
        let shape = ReplyShape { ids: vec![1] };
        let decoding = Decoding {
            sampling: None,
            dry: false,
            shape: Some(shape),
            thought: None,
        };
        let breakers = HashSet::new();
        let mut rng = fastrand::Rng::new();

        // Vocab: 0 = "wrong", 1 = "{\"b\":[[1,\"", 2 = "a", 3 = "]}", 4 = "\"]]}", 5 = EOS (None)
        let piece = |t: u32| -> ort::Result<Option<String>> {
            Ok(match t {
                0 => Some("wrong".into()),
                1 => Some(r#"{"b":[[1,""#.into()),
                2 => Some("a".into()),
                3 => Some("]}".into()),
                4 => Some(r#""]]}"#.into()),
                _ => None,
            })
        };

        // Out-of-shape argmax falls back to highest-logit fitting token.
        let mut state = ShapeState::default();
        let mut logits = [10.0, 2.0, 5.0, 1.0, 0.0, -5.0];
        assert_eq!(next_token(&decoding, &mut logits, &[], &breakers, &mut rng, &mut state, piece).unwrap(), Some(1));
        assert!(state.in_text);

        // Closing heuristic: when model picks `]}` (token 3) or EOS inside text, prefer token 4 (`"]]}`) even though `a` has higher logit.
        let mut logits = [-10.0, -10.0, 5.0, 10.0, 1.0, -5.0];
        let mut s2 = state;
        assert_eq!(next_token(&decoding, &mut logits, &[1], &breakers, &mut rng, &mut s2, piece).unwrap(), Some(4));
        assert_eq!(s2.lit, 1);

        let mut logits = [-10.0, -10.0, 5.0, -10.0, 1.0, 10.0];
        let mut s3 = state;
        assert_eq!(next_token(&decoding, &mut logits, &[1], &breakers, &mut rng, &mut s3, piece).unwrap(), Some(4));

        // No token fits once shape is finished (all logits map to non-empty or wrong tokens for done state).
        let mut logits = [1.0; 2];
        assert_eq!(next_token(&decoding, &mut logits, &[], &breakers, &mut rng, &mut s3, |t| Ok(Some(format!("bad{t}")))).unwrap(), None);
    }

    #[tokio::test]
    async fn ensure_loaded_validates_custom_catalog_directml_and_cancellation() {
        let dir = tempdir().unwrap();
        let mut backend = BuiltinBackend::default();

        for (bad_model, needle) in [
            ("custom:", "no custom GGUF"),
            ("custom:../escape.gguf", "invalid custom GGUF"),
            ("custom:sub\\file.gguf", "invalid custom GGUF"),
            ("no-such-model-id", "unknown built-in model"),
        ] {
            let api = ApiConfig {
                builtin_model: bad_model.into(),
                ..ApiConfig::default()
            };
            let err = ensure_loaded(&mut backend, &api, dir.path(), &CancellationToken::new(), &mut |_| {})
                .await
                .unwrap_err();
            assert!(err.to_string().contains(needle), "model={bad_model} err={err}");
        }

        let directml_api = ApiConfig {
            builtin_model: "gemma-4-e4b-it-qat-q4".into(),
            builtin_device: OcrDevice::Directml,
            ..ApiConfig::default()
        };
        let err = ensure_loaded(&mut backend, &directml_api, dir.path(), &CancellationToken::new(), &mut |_| {})
            .await
            .unwrap_err();
        assert!(err.to_string().contains("DirectML"), "{err}");

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let valid_api = ApiConfig {
            builtin_model: "gemma-4-e4b-it-qat-q4".into(),
            builtin_device: OcrDevice::Cpu,
            ..ApiConfig::default()
        };
        let err = ensure_loaded(&mut backend, &valid_api, dir.path(), &cancelled, &mut |_| {})
            .await
            .unwrap_err();
        assert!(err.is_cancelled(), "{err}");
    }
}
