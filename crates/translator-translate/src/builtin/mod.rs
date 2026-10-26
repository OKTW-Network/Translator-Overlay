//! Built-in ONNX LLM: download, load, and generation with an on-device KV cache.
//!
//! Models follow the onnx-community layout: `embed_tokens` (ids → embeddings) feeding a merged
//! decoder with `past_key_values.*` inputs and `present.*` outputs.

mod catalog;

use std::path::Path;

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

const DEFAULT_MAX_TOKENS: usize = 2048;

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

    let prompt = spec.arch.render_prompt(messages);
    let max_tokens = api.max_tokens.map_or(DEFAULT_MAX_TOKENS, |n| n as usize);
    let context = api.builtin_context_tokens as usize;
    // Unset knobs take the model's recommended values; temperature 0 is greedy.
    let (temperature, top_p, top_k) = spec.arch.sampling_defaults();
    let sampling = Some(Sampling {
        temperature: api.temperature.unwrap_or(temperature),
        top_p: api.top_p.unwrap_or(top_p),
        // 0 turns top-k off.
        top_k: match api.top_k.unwrap_or(top_k) {
            0 => usize::MAX,
            k => k as usize,
        },
    })
    .filter(|s| s.temperature > 0.0);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let cancel_job = cancel.clone();
    let job = tokio::task::spawn_blocking(move || {
        let Some(loaded) = backend.loaded.as_mut() else {
            return Ok(Err(TranslateError::Cancelled));
        };
        loaded.generate(&prompt, max_tokens, context, sampling, &cancel_job, |text| drop(tx.send(text.to_owned())))
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

fn argmax<T: Copy>(values: &[T], to_f32: impl Fn(T) -> f32) -> u32 {
    let mut best = (0, f32::NEG_INFINITY);
    for (i, &v) in values.iter().enumerate() {
        let v = to_f32(v);
        if v > best.1 {
            best = (i, v);
        }
    }
    best.0 as u32
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
    fn pick<T: Copy>(&self, logits: &[T], to_f32: impl Fn(T) -> f32, rng: &mut fastrand::Rng) -> u32 {
        // One pass keeping the best `top_k`; most tokens lose to the weakest kept one and are skipped.
        let mut cand: Vec<(u32, f32)> = Vec::with_capacity(self.top_k.min(logits.len()));
        let mut weakest = 0;
        for (i, &l) in logits.iter().enumerate() {
            let c = (i as u32, to_f32(l));
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
        let embed = session(device, &dir.join(spec.embed[0].file_name))?;
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
        let in_place = spec.arch.kv_in_place(&layers);
        for (slot, in_place) in kv.iter_mut().zip(in_place) {
            slot.in_place = in_place;
        }
        let per_layer_inputs = decoder.inputs().iter().any(|i| i.name() == "per_layer_inputs");
        let kv_alloc = Allocator::new(&decoder, kv_memory(device)?)?;
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
            kv_alloc,
        })
    }

    /// The cache for a fresh prefill: in-place slots get a `context`-long buffer, the rest are empty.
    fn empty_past(&self, context: usize) -> ort::Result<Vec<DynValue>> {
        self.kv
            .iter()
            .map(|s| {
                if s.in_place {
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

    /// Decode greedily when `sampling` is `None`. `on_text` gets the whole reply so far after each token.
    /// The inner error is [`TranslateError::Cancelled`] or [`TranslateError::ContextFull`].
    fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        context: usize,
        sampling: Option<Sampling>,
        cancel: &CancellationToken,
        mut on_text: impl FnMut(&str),
    ) -> ort::Result<Result<String, TranslateError>> {
        // shortcut: only a whole cached prefix is reused (the Concat layers' KV cannot be cut);
        // trimmed history or a new system prompt re-prefills everything.
        // `filter` drops a stale cache before `empty_past` allocates the next one.
        let cache = self
            .cache
            .take()
            .filter(|c| c.context == context && prompt.len() > c.text.len() && prompt.starts_with(&c.text));
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
            None => (self.empty_past(context)?, 0),
        };
        info!(model = self.spec.id, prompt_tokens = prompt_len, reused = fed, "built-in model prefill");
        let kv_mem = kv_memory(self.device)?;
        let cpu_mem = MemoryInfo::new(AllocationDevice::CPU, 0, AllocatorType::Device, MemoryType::Default)?;
        let eos = self.spec.arch.eos_ids();
        let mut binding = self.decoder.create_binding()?;
        let mut text = String::new();
        loop {
            if cancel.is_cancelled() {
                return Ok(Err(TranslateError::Cancelled));
            }
            let step: Vec<i64> = ids[fed..].iter().map(|&t| i64::from(t)).collect();
            let (n, total) = (step.len(), ids.len());
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

            let next = match logits.dtype().tensor_type() {
                Some(TensorElementType::Float16) => {
                    let values = logits.try_extract_tensor::<f16>()?.1;
                    match &sampling {
                        Some(s) => s.pick(values, f16::to_f32, &mut self.rng),
                        None => argmax(values, f16::to_f32),
                    }
                }
                _ => {
                    let values = logits.try_extract_tensor::<f32>()?.1;
                    match &sampling {
                        Some(s) => s.pick(values, |v| v, &mut self.rng),
                        None => argmax(values, |v| v),
                    }
                }
            };
            if eos.contains(&next) {
                break;
            }
            ids.push(next);
            text = self.tokenizer.decode(&ids[prompt_len..], true)?;
            on_text(&text);
            if ids.len() - prompt_len >= max_tokens {
                break;
            }
            // The next step would feed all of `ids`; a cut-off reply would not parse anyway.
            if ids.len() > context {
                return Ok(Err(TranslateError::ContextFull(context)));
            }
        }
        self.cache = Some(KvCache {
            text: format!("{prompt}{}", self.tokenizer.decode(&ids[prompt_len..fed], true)?),
            ids: ids[..fed].to_vec(),
            past,
            context,
        });
        Ok(Ok(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argmax_picks_first_max() {
        assert_eq!(argmax(&[1.0_f32, 3.0, 3.0, -1.0], |v| v), 1);
        assert_eq!(argmax(&[f16::from_f32(-2.0), f16::from_f32(0.5)], f16::to_f32), 1);
    }

    #[test]
    fn sampling_respects_top_k_and_top_p() {
        let logits = [0.0, 2.0, 1.9, -1.0, 1.8];
        let mut rng = fastrand::Rng::with_seed(7);
        let pick = |s: Sampling, rng: &mut fastrand::Rng| (0..200).map(|_| s.pick(&logits, |v| v, rng)).collect::<Vec<_>>();
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
