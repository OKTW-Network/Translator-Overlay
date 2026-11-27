//! GGUF models on llama.cpp: the prebuilt `llama.dll` that `scripts/fetch-llama.ps1` stages.
//!
//! llama.cpp runs the model and owns its KV; picking tokens stays with [`crate::builtin`].

use std::{
    collections::HashSet,
    env,
    ffi::{CStr, CString, c_char, c_void},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr, slice,
    sync::OnceLock,
    thread,
    time::Instant,
};

use minijinja::{Environment, ErrorKind, Value, context, syntax::SyntaxConfig};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use windows::{
    Win32::System::LibraryLoader::{LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW},
    core::PCWSTR,
};

use crate::{
    TranslateError,
    builtin::{DRY_BREAKERS, Decoding, MAX_CHECKPOINTS, ShapeState, THOUGHT_TOKENS, checkpoint_at, end_thought, ends_in_loop, next_token},
    http::ChatMessage,
};

/// Tokens per `llama_process` call, so a cancel waits for one chunk at most.
const PREFILL_CHUNK: usize = 512;

/// `llama_model_params` of the tag in `scripts/fetch-llama.ps1`.
#[repr(C)]
struct ModelParams {
    devices: *mut *mut c_void,
    tensor_buft_overrides: *const c_void,
    n_gpu_layers: i32,
    split_mode: i32,
    load_mode: i32,
    lazy_mode: i32,
    main_gpu: i32,
    tensor_split: *const f32,
    progress_callback: *const c_void,
    progress_callback_user_data: *mut c_void,
    kv_overrides: *const c_void,
    vocab_only: bool,
    check_tensors: bool,
    use_extra_bufts: bool,
    no_host: bool,
    no_alloc: bool,
    load_mtp: bool,
}

/// `llama_context_params` of the tag in `scripts/fetch-llama.ps1`.
#[repr(C)]
struct ContextParams {
    n_ctx: u32,
    n_batch: u32,
    n_ubatch: u32,
    n_seq_max: u32,
    n_rs_seq: u32,
    n_outputs_max: u32,
    n_outputs_max_per_seq: u32,
    n_threads: i32,
    n_threads_batch: i32,
    ctx_type: i32,
    rope_scaling_type: i32,
    pooling_type: i32,
    attention_type: i32,
    flash_attn_type: i32,
    rope_freq_base: f32,
    rope_freq_scale: f32,
    yarn_ext_factor: f32,
    yarn_attn_factor: f32,
    yarn_beta_fast: f32,
    yarn_beta_slow: f32,
    yarn_orig_ctx: u32,
    cb_eval: *const c_void,
    cb_eval_user_data: *mut c_void,
    type_k: i32,
    type_v: i32,
    moe_cache_size: usize,
    moe_cache_cpu_share: f32,
    abort_callback: *const c_void,
    abort_callback_data: *mut c_void,
    embeddings: bool,
    offload_kqv: bool,
    no_perf: bool,
    op_offload: bool,
    swa_full: bool,
    kv_unified: bool,
    samplers: *mut c_void,
    n_samplers: usize,
    ctx_other: *mut c_void,
}

// No import library: rustc builds the imports from these declarations. The app delay-loads both
// DLLs (its `build.rs`), so [`init`] can load them from `lib/llama.cpp`.
// `llama_token` is an `int32_t` that is never negative here, declared `u32` like the ONNX path's ids.
#[link(name = "llama", kind = "raw-dylib")]
unsafe extern "C" {
    fn llama_backend_init();
    fn llama_log_set(callback: unsafe extern "C" fn(i32, *const c_char, *mut c_void), user_data: *mut c_void);
    fn llama_model_default_params() -> ModelParams;
    fn llama_context_default_params() -> ContextParams;
    fn llama_model_load_from_file(path: *const c_char, params: ModelParams) -> *mut c_void;
    fn llama_model_free(model: *mut c_void);
    fn llama_model_get_vocab(model: *const c_void) -> *const c_void;
    fn llama_init_from_model(model: *mut c_void, params: ContextParams) -> *mut c_void;
    fn llama_free(ctx: *mut c_void);
    fn llama_get_memory(ctx: *const c_void) -> *mut c_void;
    fn llama_memory_clear(memory: *mut c_void, data: bool);
    fn llama_memory_seq_rm(memory: *mut c_void, seq_id: i32, p0: i32, p1: i32) -> bool;
    /// Holds up to `ctx`'s `n_batch` tokens and must not outlive `ctx`.
    fn llama_batch_ext_init(ctx: *mut c_void) -> *mut c_void;
    fn llama_batch_ext_free(batch: *mut c_void);
    fn llama_batch_ext_clear(batch: *mut c_void);
    /// The entry's index, negative when the batch is full.
    fn llama_batch_ext_add_token(batch: *mut c_void, seq_id: i32, token: u32) -> i32;
    fn llama_batch_ext_set_pos(batch: *mut c_void, idx: i32, pos: *const i32) -> bool;
    fn llama_batch_ext_set_output_logits(batch: *mut c_void, idx: i32, value: bool) -> bool;
    /// 0 on success, 1 when the KV has no room (memory unchanged), else an error.
    fn llama_process(ctx: *mut c_void, kind: i32, batch: *mut c_void) -> i32;
    fn llama_get_logits_ith(ctx: *mut c_void, i: i32) -> *mut f32;
    fn llama_vocab_n_tokens(vocab: *const c_void) -> i32;
    fn llama_vocab_is_eog(vocab: *const c_void, token: u32) -> bool;
    fn llama_tokenize(
        vocab: *const c_void,
        text: *const c_char,
        text_len: i32,
        tokens: *mut u32,
        n_tokens_max: i32,
        add_special: bool,
        parse_special: bool,
    ) -> i32;
    fn llama_token_to_piece(vocab: *const c_void, token: u32, buf: *mut c_char, length: i32, lstrip: i32, special: bool) -> i32;
    /// Negative when the vocab has none.
    fn llama_vocab_bos(vocab: *const c_void) -> i32;
    /// Negative when the vocab has none.
    fn llama_vocab_eos(vocab: *const c_void) -> i32;
    fn llama_vocab_get_add_bos(vocab: *const c_void) -> bool;
    fn llama_model_chat_template(model: *const c_void, name: *const c_char) -> *const c_char;
    fn llama_model_is_recurrent(model: *const c_void) -> bool;
    fn llama_model_is_hybrid(model: *const c_void) -> bool;
    fn llama_state_seq_get_size_ext(ctx: *mut c_void, seq_id: i32, flags: u32) -> usize;
    fn llama_state_seq_get_data_ext(ctx: *mut c_void, dst: *mut u8, size: usize, seq_id: i32, flags: u32) -> usize;
    /// 0 when the state does not load.
    fn llama_state_seq_set_data_ext(ctx: *mut c_void, src: *const u8, size: usize, dest_seq_id: i32, flags: u32) -> usize;
}

/// `LLAMA_STATE_SEQ_FLAGS_PARTIAL_ONLY`: only the state `llama_memory_seq_rm` cannot cut (recurrent
/// layers, SWA KV).
const STATE_PARTIAL_ONLY: u32 = 1;

/// `LLAMA_PROCESS_TYPE_DECODE`.
const PROCESS_DECODE: i32 = 1;

#[link(name = "ggml", kind = "raw-dylib")]
unsafe extern "C" {
    fn ggml_backend_load_all_from_path(dir_path: *const c_char);
}

/// llama.cpp's log: warnings and errors (`GGML_LOG_LEVEL_WARN` = 3, `_ERROR` = 4) carry the reason a
/// load failed.
unsafe extern "C" fn log(level: i32, text: *const c_char, _user_data: *mut c_void) {
    // SAFETY: llama.cpp passes a NUL-terminated string that lives through this call.
    let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let text = text.trim_end();
    if matches!(level, 3 | 4) {
        warn!(target: "llama", "{text}");
    } else {
        debug!(target: "llama", "{text}");
    }
}

/// Load `llama.dll` and its backends, once: from `lib/llama.cpp` next to the app, or
/// `LLAMA_LIB_DIR` (set for `cargo run` / `cargo test` in `.cargo/config.toml`).
fn init() -> ort::Result<()> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    INIT.get_or_init(|| {
        let packaged = env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.join("lib").join("llama.cpp")));
        let dir = packaged
            .into_iter()
            .chain(env::var_os("LLAMA_LIB_DIR").map(PathBuf::from))
            .find(|dir| dir.join("llama.dll").is_file())
            .ok_or("llama.dll not found; run scripts/fetch-llama.ps1")?;
        // By full path, so the delay-loaded imports and the backends (the CPU ones need OpenMP)
        // find them already loaded whatever the DLL search path is.
        for name in ["libomp.dll", "llama.dll"] {
            let path: Vec<u16> = dir.join(name).as_os_str().encode_wide().chain([0]).collect();
            // SAFETY: `path` is NUL-terminated; the handle is leaked so the library stays loaded.
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_WITH_ALTERED_SEARCH_PATH) }
                .map_err(|e| format!("load {name} from {}: {e}", dir.display()))?;
        }
        let c_dir = CString::new(dir.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        // SAFETY: `log` matches `ggml_log_callback` and never unwinds.
        unsafe { llama_log_set(log, ptr::null_mut()) };
        // SAFETY: no preconditions.
        unsafe { llama_backend_init() };
        // SAFETY: `c_dir` is NUL-terminated and lives through the call.
        unsafe { ggml_backend_load_all_from_path(c_dir.as_ptr()) };
        Ok(())
    })
    .clone()
    .map_err(ort::Error::new)
}

fn tokenize(vocab: *const c_void, text: &str) -> ort::Result<Vec<u32>> {
    let text_len = i32::try_from(text.len()).map_err(|_| ort::Error::new("text too long to tokenize"))?;
    let mut ids = vec![0_u32; text.len()];
    loop {
        // The prompt's own special tokens (`<bos>`, turn markers) are text here, so parse them and add none.
        // SAFETY: `vocab` is a loaded model's; `text` and `ids` are valid for the lengths passed.
        let n = unsafe { llama_tokenize(vocab, text.as_ptr().cast(), text_len, ids.as_mut_ptr(), ids.len() as i32, false, true) };
        // A negative count is the size the tokens need; `i32::MIN` is an overflow.
        if n >= 0 {
            ids.truncate(n as usize);
            return Ok(ids);
        }
        if n == i32::MIN {
            return Err(ort::Error::new("text too long to tokenize"));
        }
        ids.resize(n.unsigned_abs() as usize, 0);
    }
}

/// A token's bytes, which can end mid-character. `special` renders control tokens as their text
/// instead of nothing.
fn piece(vocab: *const c_void, token: u32, special: bool) -> Vec<u8> {
    let mut buf = vec![0_u8; 64];
    loop {
        // SAFETY: `vocab` is a loaded model's; `buf` is writable for the length passed.
        let n = unsafe { llama_token_to_piece(vocab, token, buf.as_mut_ptr().cast(), buf.len() as i32, 0, special) };
        // A negative count is the size the piece needs.
        if n >= 0 {
            buf.truncate(n as usize);
            return buf;
        }
        buf.resize(n.unsigned_abs() as usize, 0);
    }
}

/// A loaded GGUF model and its context.
pub(crate) struct Model {
    id: String,
    model: *mut c_void,
    vocab: *const c_void,
    n_vocab: usize,
    /// Null until the first reply; recreated when the context length changes.
    ctx: *mut c_void,
    /// `ctx`'s `llama_batch_ext`, null with it.
    batch: *mut c_void,
    context: usize,
    /// Tokens in `ctx`'s KV.
    ids: Vec<u32>,
    /// `ids` as text once a reply completes, else empty. Matched as text because re-encoding a reply
    /// can split it differently from the tokens the model generated.
    text: String,
    rng: fastrand::Rng,
    /// Tokens holding a [`DRY_BREAKERS`] character.
    dry_breakers: HashSet<u32>,
    /// The vocab's [`THOUGHT_TOKENS`], never sampled.
    thought_tokens: Vec<u32>,
    /// The model has recurrent state (Qwen3.5's linear-attention layers), which cannot be cut back.
    recurrent: bool,
    /// Recurrent state at each recent prompt's checkpoint (see [`MAX_CHECKPOINTS`]), oldest first:
    /// `(tokens, state)`, each state that of `ids[..tokens]`.
    checkpoints: Vec<(usize, Vec<u8>)>,
}

// SAFETY: llama.cpp models and contexts are not tied to a thread, and the backend mutex gives one
// thread at a time access.
unsafe impl Send for Model {}

impl Drop for Model {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            // SAFETY: `batch` came from `llama_batch_ext_init` and is freed once, before its context.
            unsafe { llama_batch_ext_free(self.batch) };
            // SAFETY: `ctx` came from `llama_init_from_model` and is freed once.
            unsafe { llama_free(self.ctx) };
        }
        // SAFETY: `model` came from `llama_model_load_from_file`; its only context is freed above.
        unsafe { llama_model_free(self.model) };
    }
}

impl Model {
    /// `gpu` offloads every layer to the GPU (Vulkan); otherwise llama.cpp gets no GPU device at all.
    pub(crate) fn load(id: String, path: &Path, gpu: bool) -> ort::Result<Self> {
        init()?;
        let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| ort::Error::new(e.to_string()))?;
        // SAFETY: no preconditions.
        let mut params = unsafe { llama_model_default_params() };
        // LLAMA_LOAD_MODE_NONE: read the file instead of mapping it, so the weights are not held twice (page cache + VRAM).
        params.load_mode = 0;
        // An empty device list; null would mean every device.
        let mut no_devices = [ptr::null_mut()];
        if gpu {
            params.n_gpu_layers = -1;
        } else {
            params.n_gpu_layers = 0;
            params.devices = no_devices.as_mut_ptr();
        }
        // SAFETY: `c_path` is NUL-terminated; `params` and `no_devices` live through the call.
        let model = unsafe { llama_model_load_from_file(c_path.as_ptr(), params) };
        if model.is_null() {
            return Err(ort::Error::new(format!("llama.cpp could not load {} (see the log)", path.display())));
        }
        // SAFETY: `model` is loaded; its vocab lives as long as it does.
        let vocab = unsafe { llama_model_get_vocab(model) };
        // SAFETY: `vocab` is the loaded model's.
        let n_vocab = unsafe { llama_vocab_n_tokens(vocab) } as usize;
        let dry_breakers = (0..n_vocab as u32)
            .filter(|&t| String::from_utf8_lossy(&piece(vocab, t, true)).contains(DRY_BREAKERS))
            .collect();
        // A marker the vocab lacks tokenizes into several plain pieces, which stay allowed.
        let mut thought_tokens = Vec::new();
        for t in THOUGHT_TOKENS {
            if let [id] = tokenize(vocab, t)?[..] {
                thought_tokens.push(id);
            }
        }
        // SAFETY: `model` is loaded.
        let recurrent = unsafe { llama_model_is_recurrent(model) };
        // SAFETY: `model` is loaded.
        let hybrid = unsafe { llama_model_is_hybrid(model) };
        Ok(Self {
            id,
            model,
            vocab,
            n_vocab,
            ctx: ptr::null_mut(),
            batch: ptr::null_mut(),
            context: 0,
            ids: Vec::new(),
            text: String::new(),
            rng: fastrand::Rng::new(),
            dry_breakers,
            thought_tokens,
            recurrent: recurrent || hybrid,
            checkpoints: Vec::new(),
        })
    }

    pub(crate) fn token_count(&self, text: &str) -> ort::Result<usize> {
        Ok(tokenize(self.vocab, text)?.len())
    }

    /// `messages` through the file's own Jinja chat template with thinking off, ending with an open
    /// model turn, set up like Hugging Face's `apply_chat_template`.
    pub(crate) fn chat_prompt(&self, messages: &[ChatMessage]) -> ort::Result<String> {
        // SAFETY: `model` is loaded; a null name asks for the default template, which lives as long as the model.
        let template = unsafe { llama_model_chat_template(self.model, ptr::null()) };
        if template.is_null() {
            return Err(ort::Error::new("the GGUF file has no chat template"));
        }
        // SAFETY: `template` is the model's NUL-terminated string, which lives as long as the model.
        let template = unsafe { CStr::from_ptr(template) }.to_string_lossy();
        // A negative id means the vocab has no such token.
        let token = |id: i32| {
            u32::try_from(id).map_or_else(|_| String::new(), |t| String::from_utf8_lossy(&piece(self.vocab, t, true)).into_owned())
        };
        // SAFETY: `vocab` is the loaded model's.
        let bos = token(unsafe { llama_vocab_bos(self.vocab) });
        // SAFETY: `vocab` is the loaded model's.
        let eos = token(unsafe { llama_vocab_eos(self.vocab) });
        // Python's string and dict methods, `raise_exception`, and HF's `trim_blocks` and `lstrip_blocks`.
        let mut env = Environment::new();
        let syntax = SyntaxConfig::builder().trim_blocks(true).lstrip_blocks(true).build();
        env.set_syntax(syntax.map_err(|e| ort::Error::new(e.to_string()))?);
        env.set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
        env.add_function("raise_exception", |message: String| -> Result<Value, minijinja::Error> {
            Err(minijinja::Error::new(ErrorKind::InvalidOperation, message))
        });
        let render = |messages: Vec<Value>| {
            let vars = context! { messages, add_generation_prompt => true, enable_thinking => false, bos_token => &bos, eos_token => &eos };
            env.render_str(&template, vars)
        };
        let message = |m: &ChatMessage| context! { role => &m.role, content => &m.content };
        let mut prompt = render(messages.iter().map(message).collect());
        // Templates with no system role (Gemma 2, Mistral v0.1) raise on one; like llama-server, fold it
        // into the first user turn.
        if let (Err(_), [system, user, rest @ ..]) = (&prompt, messages)
            && system.role == "system"
            && user.role == "user"
        {
            let content = format!("{}\n\n{}", system.content, user.content);
            let first = context! { role => "user", content };
            prompt = render(std::iter::once(first).chain(rest.iter().map(message)).collect());
        }
        let prompt = prompt.map_err(|e| ort::Error::new(format!("GGUF chat template: {e:#}")))?;
        // [`tokenize`] adds no special tokens, so a BOS the template leaves to the tokenizer goes in as text.
        // SAFETY: `vocab` is the loaded model's.
        let add_bos = unsafe { llama_vocab_get_add_bos(self.vocab) };
        Ok(if add_bos && !prompt.starts_with(&bos) {
            format!("{bos}{prompt}")
        } else {
            prompt
        })
    }

    /// Save the recurrent state of `ids[..tokens]` (all of `self.ids`) as a checkpoint.
    fn checkpoint(&mut self, tokens: usize) {
        // SAFETY: `ctx` is live.
        let size = unsafe { llama_state_seq_get_size_ext(self.ctx, 0, STATE_PARTIAL_ONLY) };
        let mut state = vec![0_u8; size];
        // SAFETY: `ctx` is live; `state` is writable for `size` bytes.
        let written = unsafe { llama_state_seq_get_data_ext(self.ctx, state.as_mut_ptr(), size, 0, STATE_PARTIAL_ONLY) };
        if written != size {
            warn!(size, written, "llama.cpp state checkpoint failed");
            return;
        }
        self.checkpoints.retain(|&(t, _)| t < tokens);
        if self.checkpoints.len() == MAX_CHECKPOINTS {
            self.checkpoints.remove(0);
        }
        self.checkpoints.push((tokens, state));
    }

    /// A fresh `context`-token context in place of the current one.
    fn new_context(&mut self, context: usize) -> ort::Result<()> {
        if !self.ctx.is_null() {
            // SAFETY: `batch` came from `llama_batch_ext_init` and is nulled right after, before its context.
            unsafe { llama_batch_ext_free(self.batch) };
            self.batch = ptr::null_mut();
            // SAFETY: `ctx` came from `llama_init_from_model` and is nulled right after.
            unsafe { llama_free(self.ctx) };
            self.ctx = ptr::null_mut();
        }
        self.ids.clear();
        self.text.clear();
        self.checkpoints.clear();
        // SAFETY: no preconditions.
        let mut params = unsafe { llama_context_default_params() };
        params.n_ctx = context as u32;
        params.n_batch = PREFILL_CHUNK as u32;
        // Only a batch's last token outputs logits; the default reserves a vocab-sized row for each of the 512.
        params.n_outputs_max = 1;
        params.no_perf = true;
        // llama.cpp defaults to 4 threads. Hyper-threads only slow the CPU backend down.
        let threads = (thread::available_parallelism().map_or(4, usize::from) / 2).max(1) as i32;
        params.n_threads = threads;
        params.n_threads_batch = threads;
        // SAFETY: `model` is loaded and outlives the context (see `Drop`).
        self.ctx = unsafe { llama_init_from_model(self.model, params) };
        if self.ctx.is_null() {
            return Err(ort::Error::new(format!("llama.cpp could not create a {context}-token context (see the log)")));
        }
        // SAFETY: `ctx` is live; the batch is freed before it (see `Drop`).
        self.batch = unsafe { llama_batch_ext_init(self.ctx) };
        self.context = context;
        Ok(())
    }

    /// Same contract as the ONNX model's `generate`.
    pub(crate) fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        context: usize,
        decoding: Decoding,
        cancel: &CancellationToken,
        mut on_text: impl FnMut(Option<&str>),
    ) -> ort::Result<Result<String, TranslateError>> {
        if self.ctx.is_null() || self.context != context {
            self.new_context(context)?;
        }
        let vocab = self.vocab;
        // The open thought's end token and the tokens that follow it (`None` once it ends), and the
        // tokens that opened it.
        let (mut thought, open) = match decoding.thought {
            Some((open, end, after)) => match tokenize(vocab, end)?[..] {
                [id] => (Some((id, tokenize(vocab, after)?)), tokenize(vocab, open)?),
                _ => return Err(ort::Error::new(format!("model has no {end} token"))),
            },
            None => (None, Vec::new()),
        };
        let thinking_decoding = Decoding {
            shape: None,
            thought: None,
            ..decoding
        };
        let (mut ids, mut reused) = if !self.text.is_empty() && prompt.len() > self.text.len() && prompt.starts_with(&self.text) {
            ([self.ids.as_slice(), &tokenize(vocab, &prompt[self.text.len()..])?].concat(), self.ids.len())
        } else {
            // A retry, a request after a cancelled one, or trimmed history still reuses the shared
            // tokens. Feed at least one token for the logits.
            let ids = tokenize(vocab, prompt)?;
            let shared = ids
                .iter()
                .zip(&self.ids)
                .take_while(|(a, b)| a == b)
                .count()
                .min(ids.len().saturating_sub(1));
            (ids, shared)
        };
        let prompt_len = ids.len();
        if prompt_len > context {
            return Ok(Err(TranslateError::ContextFull(context)));
        }
        let checkpoint_at = checkpoint_at(&ids, &open);
        // SAFETY: `ctx` is live; its memory lives as long as it does.
        let memory = unsafe { llama_get_memory(self.ctx) };
        if reused < self.ids.len() {
            self.checkpoints.retain(|&(tokens, _)| tokens <= reused);
            // SAFETY: `memory` is `ctx`'s; this drops sequence 0 from position `reused` on.
            let cut = unsafe { llama_memory_seq_rm(memory, 0, reused as i32, -1) };
            // Recurrent state cannot be cut back: load the last checkpoint within the shared tokens,
            // then cut the attention KV there.
            let restored = cut
                || match self.checkpoints.last() {
                    Some(&(tokens, ref state)) => {
                        reused = tokens;
                        // SAFETY: `ctx` is live; `state` came from `llama_state_seq_get_data_ext` with the same flags.
                        let loaded =
                            unsafe { llama_state_seq_set_data_ext(self.ctx, state.as_ptr(), state.len(), 0, STATE_PARTIAL_ONLY) } != 0;
                        // SAFETY: `memory` is `ctx`'s; this drops sequence 0 from position `tokens` on.
                        loaded && unsafe { llama_memory_seq_rm(memory, 0, tokens as i32, -1) }
                    }
                    None => false,
                };
            if !restored {
                // SAFETY: `memory` is `ctx`'s.
                unsafe { llama_memory_clear(memory, true) };
                self.checkpoints.clear();
                reused = 0;
            }
        }
        self.ids.truncate(reused);
        self.text.clear();

        let start = Instant::now();
        let mut prefill = None;
        let mut fed = reused;
        let mut logits = Vec::new();
        // The answer's bytes.
        let mut reply = Vec::new();
        let mut text = String::new();
        let mut looping = false;
        let mut shape_state = ShapeState::default();
        // Where the answer starts: after the thought, if any.
        let mut answer = prompt_len;
        loop {
            if cancel.is_cancelled() {
                return Ok(Err(TranslateError::Cancelled));
            }
            let mut total = ids.len().min(fed + PREFILL_CHUNK);
            // Stop at the checkpoint for the state there.
            if fed < checkpoint_at {
                total = total.min(checkpoint_at);
            }
            // Positions continue the `fed` tokens in the KV; only the last token outputs logits.
            // SAFETY: `batch` is `ctx`'s.
            unsafe { llama_batch_ext_clear(self.batch) };
            for (pos, &token) in (fed as i32..).zip(&ids[fed..total]) {
                // SAFETY: `batch` is `ctx`'s.
                let idx = unsafe { llama_batch_ext_add_token(self.batch, 0, token) };
                // `ctx`'s `n_batch` is `PREFILL_CHUNK`, at least `total - fed`.
                debug_assert!(idx >= 0, "llama.cpp batch full");
                // SAFETY: `idx` is the entry just added; `pos` lives through the call.
                unsafe { llama_batch_ext_set_pos(self.batch, idx, &pos) };
            }
            // SAFETY: the batch holds `total - fed` entries.
            unsafe { llama_batch_ext_set_output_logits(self.batch, (total - fed - 1) as i32, true) };
            // SAFETY: `ctx` is live and `batch` is its.
            match unsafe { llama_process(self.ctx, PROCESS_DECODE, self.batch) } {
                0 => {}
                // No KV slot; the memory is as it was before the call.
                1 => return Ok(Err(TranslateError::ContextFull(context))),
                // Part of the batch may be in the KV, so start over next time.
                status => {
                    // SAFETY: `memory` is `ctx`'s.
                    unsafe { llama_memory_clear(memory, true) };
                    self.ids.clear();
                    self.checkpoints.clear();
                    return Err(ort::Error::new(format!("llama_process failed ({status})")));
                }
            }
            self.ids.extend_from_slice(&ids[fed..total]);
            fed = total;
            if fed == checkpoint_at && self.recurrent {
                self.checkpoint(fed);
            }
            if fed < ids.len() {
                continue;
            }
            if prefill.is_none() {
                prefill = Some(start.elapsed());
                if thought.is_some() {
                    on_text(None);
                }
            }

            // SAFETY: `ctx` just decoded a batch that outputs its last token's logits.
            let last = unsafe { llama_get_logits_ith(self.ctx, -1) };
            if last.is_null() {
                return Err(ort::Error::new("llama.cpp returned no logits"));
            }
            logits.clear();
            // SAFETY: the row holds one logit per vocab token and lives until the next decode.
            logits.extend_from_slice(unsafe { slice::from_raw_parts(last, self.n_vocab) });
            let step_decoding = if thought.is_some() {
                &thinking_decoding
            } else {
                for &t in &self.thought_tokens {
                    logits[t as usize] = f32::NEG_INFINITY;
                }
                &decoding
            };
            // SAFETY: `vocab` is the loaded model's.
            let is_end = |t: u32| unsafe { llama_vocab_is_eog(vocab, t) };
            let Some(next) =
                next_token(step_decoding, &mut logits, &ids[answer..], &self.dry_breakers, &mut self.rng, &mut shape_state, |t| {
                    Ok((!is_end(t)).then(|| String::from_utf8_lossy(&piece(vocab, t, true)).into_owned()))
                })?
            else {
                return Ok(Err(TranslateError::Other("no token fits the reply JSON".into())));
            };
            if let Some(start) = end_thought(&mut thought, next, &mut ids) {
                answer = start;
            } else if is_end(next) {
                break;
            } else {
                ids.push(next);
                if thought.is_none() {
                    reply.extend(piece(vocab, next, false));
                    text = String::from_utf8_lossy(&reply).into_owned();
                    // A token can end mid-character, which decodes to U+FFFD until the next token completes it.
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
            // The next step feeds the token just added; a cut-off reply would not parse anyway.
            if ids.len() > context {
                return Ok(Err(TranslateError::ContextFull(context)));
            }
        }
        let prefill = prefill.unwrap_or_default();
        let decode = start.elapsed() - prefill;
        info!(
            model = self.id.as_str(),
            prompt_tokens = prompt_len,
            reused,
            output_tokens = ids.len() - prompt_len,
            prefill_ms = prefill.as_millis() as u64,
            prefill_tps = format_args!("{:.1}", (prompt_len - reused) as f64 / prefill.as_secs_f64()),
            decode_ms = decode.as_millis() as u64,
            decode_tps = format_args!("{:.1}", (fed - prompt_len) as f64 / decode.as_secs_f64()),
            "built-in model generated"
        );
        // With control tokens, so a thought's markers stay in.
        let fed_reply: Vec<u8> = ids[prompt_len..fed].iter().flat_map(|&t| piece(vocab, t, true)).collect();
        self.text = format!("{prompt}{}", String::from_utf8_lossy(&fed_reply));
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
