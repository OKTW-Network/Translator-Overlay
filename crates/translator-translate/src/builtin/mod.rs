//! Built-in ONNX LLM: download, load, and generation with an on-device KV cache.
//!
//! Models follow the onnx-community layout: `embed_tokens` (ids → embeddings) feeding a merged
//! decoder with `past_key_values.*` inputs and `present.*` outputs.

mod catalog;

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
use translator_core::{ApiConfig, ModelKind, OcrDevice, PipelineStatus};
use translator_ocr::{download_artifact, initialize_ort_environment, part_path};

pub use crate::builtin::catalog::{BUILTIN_MODELS, BuiltinModelSpec, find as find_builtin_model};
use crate::{TranslateError, http::ChatMessage};

/// A reply whose last `LOOP_TOKENS` tokens (and at least three periods) repeat one span of up to
/// `LOOP_TOKENS` tokens is stuck in a loop.
const LOOP_TOKENS: usize = 64;

/// DRY settings: llama.cpp's recommended multiplier and base.
const DRY_MULTIPLIER: f32 = 0.8;
const DRY_BASE: f32 = 1.75;
/// Above llama.cpp's 2, so a short repeat the source itself has (`あああ`, a doubled word) still
/// translates as written.
const DRY_ALLOWED_LENGTH: usize = 4;
/// A repeat match stops at a token holding one of these, so JSON quoting never counts as a repeat.
const DRY_BREAKERS: [char; 4] = ['\n', ':', '"', '*'];

/// The loaded model, if any. Lives behind a mutex on [`crate::TranslateClient`].
#[derive(Default)]
pub(crate) struct BuiltinBackend {
    loaded: Option<Loaded>,
}

impl BuiltinBackend {
    /// Drop the ONNX sessions (frees VRAM). The next request loads again.
    pub(crate) fn unload(&mut self) {
        self.loaded = None;
    }
}

/// Download / load the model selected in `api` unless it is already loaded. `on_status` gets
/// each download / load phase, then `None` once loading ends (either way).
pub(crate) async fn ensure_loaded(
    backend: &mut BuiltinBackend,
    api: &ApiConfig,
    models_dir: &Path,
    cancel: &CancellationToken,
    on_status: &mut impl FnMut(Option<PipelineStatus>),
) -> Result<&'static BuiltinModelSpec, TranslateError> {
    let spec = find_builtin_model(&api.builtin_model).ok_or_else(|| {
        let ids: Vec<_> = BUILTIN_MODELS.iter().map(|m| m.id).collect();
        TranslateError::Other(format!("unknown built-in model {:?} (available: {})", api.builtin_model, ids.join(", ")))
    })?;
    let device = api.builtin_device;
    // DirectML's GroupQueryAttention ignores `local_window_size`: Gemma 4's sliding-window layers
    // then read the whole context and the reply turns to garbage past 512 tokens.
    if device == OcrDevice::Directml {
        return Err(TranslateError::Other("built-in models do not support DirectML; use WebGPU or CPU".into()));
    }
    if backend.loaded.as_ref().is_some_and(|l| l.spec.id == spec.id && l.device == device) {
        return Ok(spec);
    }
    // A preload cancelled by Stop may only get the lock after Stop unloaded.
    if cancel.is_cancelled() {
        return Err(TranslateError::Cancelled);
    }
    backend.unload();
    let dir = models_dir.join(spec.dir);
    let loaded: Result<Loaded, TranslateError> = async {
        download_missing(spec, &dir, cancel, on_status).await?;
        on_status(Some(PipelineStatus::LoadingModels {
            kind: ModelKind::Translation,
        }));
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || Loaded::load(spec, device, &dir))
            .await
            .map_err(|e| TranslateError::Other(format!("built-in model load task: {e}")))?
            .map_err(|e| TranslateError::Other(format!("load {}: {e}", spec.id)))
    }
    .await;
    on_status(None);
    backend.loaded = Some(loaded?);
    info!(model = spec.id, ?device, "built-in model loaded");
    Ok(spec)
}

/// Load the model if needed (see [`ensure_loaded`]), then generate one completion.
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

    let prompt = spec.arch.render_prompt(messages, spec.empty_thought);
    // The last message is the source payload, and the reply mirrors its JSON.
    let source = messages.last().map_or_else(String::new, |m| m.content.clone());
    let output_ratio = api.builtin_output_ratio;
    let max_tokens_cap = api.max_tokens.map_or(usize::MAX, |n| n as usize);
    let context = api.builtin_context_tokens as usize;
    // Unset knobs take the model's recommended values; temperature 0 is greedy.
    let (temperature, top_p, top_k) = spec.arch.sampling_defaults();
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
    };
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let cancel_job = cancel.clone();
    let job = tokio::task::spawn_blocking(move || {
        let Some(loaded) = backend.loaded.as_mut() else {
            return Ok(Err(TranslateError::Cancelled));
        };
        let max_tokens = ((output_ratio * loaded.tokenizer.encode(source, false)?.len() as f32).ceil() as usize).min(max_tokens_cap);
        loaded.generate(&prompt, max_tokens, context, decoding, &cancel_job, |text| drop(tx.send(text.to_owned())))
    });
    // Ends when the job drops its sender.
    while let Some(text) = rx.recv().await {
        on_text(&text);
    }
    match job.await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => Err(TranslateError::Other(format!("{}: {e}", spec.id))),
        Err(e) => Err(TranslateError::Other(format!("built-in model task: {e}"))),
    }
}

async fn download_missing(
    spec: &BuiltinModelSpec,
    dir: &Path,
    cancel: &CancellationToken,
    on_status: &mut impl FnMut(Option<PipelineStatus>),
) -> Result<(), TranslateError> {
    let missing: Vec<_> = spec.artifacts().filter(|a| !dir.join(a.file_name).is_file()).collect();
    if missing.is_empty() {
        return Ok(());
    }
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| TranslateError::Other(format!("create {}: {e}", dir.display())))?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("translator-overlay/", env!("CARGO_PKG_VERSION")))
        .build()?;
    for (index, art) in missing.iter().enumerate() {
        let download = download_artifact(&client, art, dir, |percent| {
            on_status(Some(PipelineStatus::DownloadingModels {
                kind: ModelKind::Translation,
                file: art.file_name.to_string(),
                file_index: index as u32 + 1,
                file_count: missing.len() as u32,
                percent,
            }));
        });
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                let _ = tokio::fs::remove_file(part_path(dir, art)).await;
                return Err(TranslateError::Cancelled);
            }
            result = download => result.map_err(|e| TranslateError::Other(e.to_string()))?,
        }
    }
    Ok(())
}

struct Loaded {
    spec: &'static BuiltinModelSpec,
    device: OcrDevice,
    tokenizer: Tokenizer,
    embed: Session,
    decoder: Session,
    /// In decoder input order.
    kv: Vec<KvSlot>,
    kv_type: TensorElementType,
    per_layer_inputs: bool,
    cache: Option<KvCache>,
    rng: fastrand::Rng,
    /// Tokens holding a [`DRY_BREAKERS`] character.
    dry_breakers: HashSet<u32>,
    /// Allocates the in-place KV buffers. Declared after `cache` so it outlives them.
    kv_alloc: Allocator,
}

struct KvSlot {
    past: String,
    present: String,
    heads: i64,
    head_dim: i64,
    /// See [`crate::builtin::catalog::BuiltinArch::kv_in_place`].
    in_place: bool,
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
}

fn session(device: OcrDevice, path: &Path) -> ort::Result<Session> {
    // [`complete`] rejects DirectML before loading.
    let eps = match device {
        OcrDevice::Webgpu => vec![ep::WebGPU::default().build().error_on_failure(), ep::CPU::default().build()],
        OcrDevice::Cpu | OcrDevice::Directml => vec![ep::CPU::default().build()],
    };
    Session::builder()?.with_execution_providers(eps)?.commit_from_file(path)
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

/// How [`Loaded::generate`] picks each token.
#[derive(Clone, Copy)]
struct Decoding {
    /// `None` decodes greedily.
    sampling: Option<Sampling>,
    /// See [`apply_dry`].
    dry: bool,
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

impl Loaded {
    fn load(spec: &'static BuiltinModelSpec, device: OcrDevice, dir: &Path) -> ort::Result<Self> {
        initialize_ort_environment()?;
        let tokenizer = Tokenizer::from_file(dir.join(spec.tokenizer.file_name))?;
        // A table lookup: as fast on the CPU, and keeps the embedding tables out of VRAM.
        let embed = session(OcrDevice::Cpu, &dir.join(spec.embed[0].file_name))?;
        let decoder = session(device, &dir.join(spec.decoder[0].file_name))?;

        let mut kv = Vec::new();
        let mut kv_type = TensorElementType::Float32;
        for input in decoder.inputs() {
            let Some(layer) = input.name().strip_prefix("past_key_values.") else {
                continue;
            };
            let dims = input.dtype().tensor_shape().ok_or_else(|| missing("KV shape"))?;
            let &[_, heads, _, head_dim] = &dims[..] else {
                return Err(ort::Error::new(format!("{}: expected a 4-D KV input", input.name())));
            };
            kv.push(KvSlot {
                past: input.name().to_string(),
                present: format!("present.{layer}"),
                heads,
                head_dim,
                in_place: false,
            });
            kv_type = input.dtype().tensor_type().unwrap_or(kv_type);
        }
        // `present.22.key` -> layer `22`.
        let layers: Vec<_> = kv
            .iter()
            .map(|s| (s.present["present.".len()..].rsplit_once('.').map_or("", |(layer, _)| layer), s.head_dim))
            .collect();
        let in_place = spec.arch.kv_in_place(&layers, spec.kv_gqa_only);
        for (slot, in_place) in kv.iter_mut().zip(in_place) {
            slot.in_place = in_place;
        }
        let per_layer_inputs = decoder.inputs().iter().any(|i| i.name() == "per_layer_inputs");
        let kv_alloc = Allocator::new(&decoder, kv_memory(device)?)?;
        let dry_breakers = tokenizer
            .get_vocab(true)
            .into_iter()
            .filter(|(t, _)| t.contains(DRY_BREAKERS))
            .map(|(_, id)| id)
            .collect();
        let in_place = kv.iter().filter(|s| s.in_place).count();
        info!(model = spec.id, kv_inputs = kv.len(), in_place, ?kv_type, per_layer_inputs, "built-in model sessions ready");
        Ok(Self {
            spec,
            device,
            tokenizer,
            embed,
            decoder,
            kv,
            kv_type,
            per_layer_inputs,
            cache: None,
            rng: fastrand::Rng::new(),
            dry_breakers,
            kv_alloc,
        })
    }

    /// The cache for a fresh prefill: in-place slots get a `context`-long buffer, the rest are empty.
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
                    return Ok(DynTensor::new(&self.kv_alloc, self.kv_type, [1, s.heads, context as i64, s.head_dim])?.into_dyn());
                }
                let shape = [1, s.heads, 0, s.head_dim];
                Ok(match self.kv_type {
                    TensorElementType::Float16 => Tensor::<f16>::from_array((shape, Vec::new()))?.into_dyn(),
                    _ => Tensor::<f32>::from_array((shape, Vec::new()))?.into_dyn(),
                })
            })
            .collect()
    }

    /// `on_text` gets the whole reply so far after each token. The inner error is
    /// [`TranslateError::Cancelled`], [`TranslateError::ContextFull`], or [`TranslateError::Looping`]
    /// ([`TranslateError::Other`] when decoding greedily).
    fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        context: usize,
        decoding: Decoding,
        cancel: &CancellationToken,
        mut on_text: impl FnMut(&str),
    ) -> ort::Result<Result<String, TranslateError>> {
        // shortcut: only a whole cached prefix is reused (the out-of-place layers' KV cannot be cut);
        // trimmed history or a new system prompt re-prefills everything.
        let (cache, stale) = match self.cache.take() {
            Some(c) if c.context == context && prompt.len() > c.text.len() && prompt.starts_with(&c.text) => (Some(c), Vec::new()),
            Some(c) if c.context == context => (None, c.past),
            _ => (None, Vec::new()),
        };
        let mut ids = match &cache {
            Some(c) => [c.ids.as_slice(), self.tokenizer.encode(&prompt[c.text.len()..], false)?.get_ids()].concat(),
            None => self.tokenizer.encode(prompt, false)?.get_ids().to_vec(),
        };
        let prompt_len = ids.len();
        if prompt_len > context {
            return Ok(Err(TranslateError::ContextFull(context)));
        }
        let (mut past, mut fed) = match cache {
            Some(c) => (c.past, c.ids.len()),
            None => (self.empty_past(context, stale)?, 0),
        };
        let reused = fed;
        let start = Instant::now();
        let mut prefill = None;
        let kv_mem = kv_memory(self.device)?;
        let cpu_mem = MemoryInfo::new(AllocationDevice::CPU, 0, AllocatorType::Device, MemoryType::Default)?;
        let eos = self.spec.arch.eos_ids();
        let mut binding = self.decoder.create_binding()?;
        let mut text = String::new();
        let mut logits_f32 = Vec::new();
        let mut looping = false;
        let chunk = self.spec.prefill_chunk() as usize;
        loop {
            if cancel.is_cancelled() {
                return Ok(Err(TranslateError::Cancelled));
            }
            let total = ids.len().min(fed.saturating_add(chunk));
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
            for (slot, value) in self.kv.iter().zip(std::mem::take(&mut past)) {
                binding.bind_input(&slot.past, &value)?;
                if slot.in_place {
                    // Same buffer as the input: GQA writes the new tokens at their positions.
                    binding.bind_output(&slot.present, value)?;
                } else {
                    binding.bind_output_to_device(&slot.present, &kv_mem)?;
                }
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
            if decoding.dry {
                apply_dry(&mut logits_f32, &ids[prompt_len..], &self.dry_breakers);
            }
            let next = match &decoding.sampling {
                Some(s) => s.pick(&logits_f32, &mut self.rng),
                None => argmax(&logits_f32),
            };
            if eos.contains(&next) {
                break;
            }
            ids.push(next);
            text = self.tokenizer.decode(&ids[prompt_len..], true)?;
            on_text(&text);
            if ends_in_loop(&ids[prompt_len..]) {
                looping = true;
                break;
            }
            if ids.len() - prompt_len >= max_tokens {
                break;
            }
            // The next step would feed all of `ids`; a cut-off reply would not parse anyway.
            if ids.len() > context {
                return Ok(Err(TranslateError::ContextFull(context)));
            }
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
        self.cache = Some(KvCache {
            text: format!("{prompt}{}", self.tokenizer.decode(&ids[prompt_len..fed], true)?),
            ids: ids[..fed].to_vec(),
            past,
            context,
        });
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
    use super::*;

    #[test]
    fn argmax_picks_first_max() {
        assert_eq!(argmax(&[1.0, 3.0, 3.0, -1.0]), 1);
    }

    #[test]
    fn loop_needs_a_long_repeated_tail() {
        let period = |p: u32, n: usize| (0..n).map(|i| i as u32 % p).collect::<Vec<_>>();
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
        let pick = |s: Sampling, rng: &mut fastrand::Rng| (0..200).map(|_| s.pick(&logits, rng)).collect::<Vec<_>>();
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
}
