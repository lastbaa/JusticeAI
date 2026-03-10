use crate::state::{
    AppSettings, ChatSession, ChunkMetadata, Citation, DocumentPage, EmbeddedChunkEntry, FileInfo,
    ModelStatus, QueryResult, RagState,
};
use base64::Engine;
use llama_cpp_2::llama_backend::LlamaBackend;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Emitter;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

const SCORE_THRESHOLD: f32 = 0.20;
const GGUF_MIN_SIZE: u64 = 4_000_000_000;

const SAUL_GGUF_URL: &str = "https://huggingface.co/MaziyarPanahi/Saul-Instruct-v1-GGUF/resolve/main/Saul-Instruct-v1.Q4_K_M.gguf";

/// Rules-only system prompt — document context goes in the user turn so Llama 2
/// pays full attention to it (system-prompt content is under-weighted by the model).
const RULES_PROMPT: &str = "You are Justice AI, a legal document research assistant. \
Answer questions using ONLY the document excerpts provided in the user message. \
For every claim, cite the source inline using this exact format: [filename, p. N]. \
Example: \"The agreement expires December 31, 2025 [employment_contract.pdf, p. 4].\" \
Do not group all citations at the end — cite inline after each individual claim. \
When the answer contains numbers, dollar amounts, dates, or specific figures, \
extract and state them EXACTLY as written in the source — do not paraphrase or round. \
Include a direct quoted excerpt from the source. \
If the answer is not in the provided excerpts, say exactly: \
\"I could not find information about this in your loaded documents.\" \
Never use outside knowledge. Never guess. Never hallucinate. \
Do not give legal advice — you are a research tool.";

// ── Singletons ────────────────────────────────────────────────────────────────
// Both models are loaded once per process and cached for all subsequent calls.

// fastembed TextEmbedding: ~22 MB ONNX, downloaded to model_dir/fastembed/ on first use.
static EMBED_MODEL: OnceLock<Arc<Mutex<Option<fastembed::TextEmbedding>>>> = OnceLock::new();

// llama.cpp backend stored as Option so init failures don't poison the OnceLock.
// Once set to None (init failed), every subsequent call fast-fails with a clear error.
static LLAMA_BACKEND: OnceLock<Option<LlamaBackend>> = OnceLock::new();

fn get_llama_backend() -> Result<&'static LlamaBackend, String> {
    let slot = LLAMA_BACKEND.get_or_init(|| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(LlamaBackend::init)) {
            Ok(Ok(b)) => Some(b),
            Ok(Err(e)) => {
                log::error!("LlamaBackend::init failed: {e}");
                None
            }
            Err(_) => {
                log::error!("LlamaBackend::init panicked");
                None
            }
        }
    });
    slot.as_ref()
        .ok_or_else(|| "llama.cpp backend failed to initialize. The app may need to be restarted.".to_string())
}

/// Validate GGUF magic bytes before loading — prevents llama.cpp from calling
/// abort() on a corrupted or incomplete file, which would kill the process.
fn validate_gguf(path: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open model file: {e}"))?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|_| "Model file is too small — it may be incomplete. Try re-downloading.".to_string())?;
    if &magic != b"GGUF" {
        return Err("Model file appears corrupted (missing GGUF header). Please delete it and restart to re-download.".to_string());
    }
    Ok(())
}

// ── Model Management ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_models(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<ModelStatus, String> {
    let gguf_path = {
        let s = state.lock().await;
        s.model_dir.join("saul.gguf")
    };
    let size = gguf_path.metadata().ok().map(|m| m.len()).unwrap_or(0);
    Ok(ModelStatus {
        llm_ready: size > GGUF_MIN_SIZE,
        llm_size_gb: size as f32 / 1e9,
        download_required_gb: 4.5,
    })
}

#[tauri::command]
pub async fn download_models(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let model_dir = {
        let s = state.lock().await;
        s.model_dir.clone()
    };

    tokio::fs::create_dir_all(&model_dir)
        .await
        .map_err(|e| e.to_string())?;

    let gguf_path = model_dir.join("saul.gguf");
    let tmp_path = model_dir.join("saul.gguf.tmp");

    // Already complete — emit done immediately
    if gguf_path
        .metadata()
        .map(|m| m.len() > GGUF_MIN_SIZE)
        .unwrap_or(false)
    {
        window
            .emit(
                "download-progress",
                serde_json::json!({"percent": 100, "downloadedBytes": 0, "totalBytes": 0, "done": true}),
            )
            .ok();
        return Ok(());
    }

    // Resume partial download if tmp file exists
    let already_downloaded = tmp_path.metadata().map(|m| m.len()).unwrap_or(0);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.get(SAUL_GGUF_URL);
    if already_downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", already_downloaded));
    }

    let mut response = request.send().await.map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() && status.as_u16() != 206 {
        return Err(format!("Download failed: HTTP {status}"));
    }

    let total_bytes: u64 = if already_downloaded > 0 && status.as_u16() == 206 {
        response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').last())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    use tokio::io::AsyncWriteExt;

    // Open the tmp file for writing.
    // Only append to an existing partial download when the server actually honoured
    // the Range request (HTTP 206). If it returned 200, the server is sending the
    // full file from byte 0 — appending would corrupt it, so we truncate instead.
    let resuming = already_downloaded > 0 && status.as_u16() == 206;
    let mut file = if resuming {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&tmp_path)
            .await
            .map_err(|e| e.to_string())?
    } else {
        tokio::fs::File::create(&tmp_path)
            .await
            .map_err(|e| e.to_string())?
    };

    // Byte counter starts at whatever was already on disk (0 if not resuming).
    let mut downloaded: u64 = if resuming { already_downloaded } else { 0 };

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let percent: u8 = if total_bytes > 0 {
            (downloaded * 100 / total_bytes).min(99) as u8
        } else {
            0
        };

        window
            .emit(
                "download-progress",
                serde_json::json!({
                    "percent": percent,
                    "downloadedBytes": downloaded,
                    "totalBytes": total_bytes,
                    "done": false
                }),
            )
            .ok();
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    tokio::fs::rename(&tmp_path, &gguf_path)
        .await
        .map_err(|e| e.to_string())?;

    window
        .emit(
            "download-progress",
            serde_json::json!({
                "percent": 100,
                "downloadedBytes": downloaded,
                "totalBytes": total_bytes,
                "done": true
            }),
        )
        .ok();

    Ok(())
}

/// Called by the frontend close-confirmation handler immediately before
/// `appWindow.close()`. Setting this flag allows the `on_window_event` handler
/// in lib.rs to let the close proceed instead of intercepting it again.
#[tauri::command]
pub fn set_can_close(state: tauri::State<'_, crate::state::CloseAllowed>) {
    state.0.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Write text content to an arbitrary file path chosen by the user via a save dialog.
/// Called from the export-chat and export-citations features.
#[tauri::command]
pub fn save_file(file_path: String, content: String) -> Result<(), String> {
    std::fs::write(&file_path, content.as_bytes())
        .map_err(|e| format!("Failed to write file: {e}"))
}

// ── Local Embedding via fastembed ─────────────────────────────────────────────
// The model is loaded once into EMBED_MODEL on the first call and reused for
// every subsequent chunk or query. Re-initializing per call was silent-failing.

pub async fn embed_text(text: &str, is_query: bool, model_dir: &Path) -> Result<Vec<f32>, String> {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    let cache_dir = model_dir.join("fastembed-bge");
    // BGE uses asymmetric retrieval: queries get a prefix that shifts the embedding into the
    // retrieval space. Document chunks are embedded without the prefix.
    let text_owned = if is_query {
        format!("Represent this sentence for searching relevant passages: {}", text)
    } else {
        text.to_string()
    };

    tokio::task::spawn_blocking(move || {
        // Get or create the Arc<Mutex<Option<Model>>> wrapper (infallible).
        let model_arc = EMBED_MODEL.get_or_init(|| Arc::new(Mutex::new(None)));

        let mut guard = model_arc
            .lock()
            .map_err(|e| format!("Embed model mutex poisoned: {e}"))?;

        // Initialize the model exactly once; errors here are propagated, not silently dropped.
        if guard.is_none() {
            std::fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Cannot create fastembed cache dir: {e}"))?;
            let model = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15)
                    .with_cache_dir(cache_dir)
                    .with_show_download_progress(false),
            )
            .map_err(|e| format!("Failed to initialize embedding model: {e}"))?;
            *guard = Some(model);
        }

        let model = guard.as_ref()
            .ok_or_else(|| "Embedding model unavailable after initialization".to_string())?;
        let embeddings = model
            .embed(vec![text_owned], None)
            .map_err(|e| e.to_string())?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| "No embedding returned".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Re-embed all stored chunks using BGE-small-en-v1.5.
/// Called once at startup when stale AllMiniL vectors are detected.
/// Text is stored in chunk metadata, so no file re-parsing is needed.
pub async fn migrate_embeddings(state: &mut crate::state::RagState) {
    let total = state.embedded_chunks.len();
    if total == 0 {
        state.embed_model = "bge-small-en-v1.5".to_string();
        state.save_embed_model().await;
        return;
    }
    log::info!("Migrating {} chunk embeddings from AllMiniL → BGE-small-en-v1.5…", total);
    let model_dir = state.model_dir.clone();
    for (i, entry) in state.embedded_chunks.iter_mut().enumerate() {
        match embed_text(&entry.meta.text, false, &model_dir).await {
            Ok(vec) => entry.vector = vec,
            Err(e) => log::error!("Migration embed error for chunk {}: {}", i, e),
        }
        if (i + 1) % 20 == 0 || i + 1 == total {
            log::info!("Embedding migration: {}/{}", i + 1, total);
        }
    }
    state.embed_model = "bge-small-en-v1.5".to_string();
    state.save_embed_model().await;
    state.save_chunks().await;
    log::info!("Embedding migration complete.");
}

// ── Local LLM via llama-cpp-2 ─────────────────────────────────────────────────
//
// The model (4.5 GB) is loaded once and cached in `model_cache`.
// A fresh LlamaContext is created per query (lightweight — just KV cache alloc).

/// Format prior conversation turns as labeled text for the model.
fn format_history(history: &[(String, String)]) -> String {
    let mut s = String::from("[Prior conversation — for follow-up context only:]\n");
    // Cap to the last 4 turns so long conversations don't exhaust the context window.
    let recent = if history.len() > 4 { &history[history.len() - 4..] } else { history };
    for (user, assistant) in recent {
        // Trim each side to avoid bloating the prompt with long prior answers
        let u = if user.len() > 400 { &user[..400] } else { user };
        let a = if assistant.len() > 600 { &assistant[..600] } else { assistant };
        s.push_str(&format!("User: {u}\nAssistant: {a}\n\n"));
    }
    s
}

async fn ask_saul(
    user_question: &str,
    context: &str,
    history: &[(String, String)],
    model_dir: &Path,
    model_cache: Arc<Mutex<Option<llama_cpp_2::model::LlamaModel>>>,
    window: tauri::Window,
) -> Result<String, String> {
    use llama_cpp_2::{
        context::params::LlamaContextParams,
        llama_batch::LlamaBatch,
        model::{params::LlamaModelParams, AddBos, LlamaModel},
        sampling::LlamaSampler,
    };
    use std::num::NonZeroU32;

    let gguf_path = model_dir.join("saul.gguf");

    // Build history prefix (empty string when there are no prior turns).
    let history_prefix = if history.is_empty() {
        String::new()
    } else {
        format_history(history)
    };

    // Put context in the user turn — Llama 2 models pay far more attention to
    // user-turn content than to the system prompt, so this is the reliable way
    // to ground answers in the retrieved document chunks.
    let user_content = if context.trim().is_empty() {
        if history_prefix.is_empty() {
            format!("Question: {user_question}")
        } else {
            format!("{history_prefix}Current question: {user_question}")
        }
    } else if history_prefix.is_empty() {
        format!(
            "Below are excerpts from the user's loaded legal documents. \
Answer the question using ONLY these excerpts.\n\n\
{context}\n\n---\n\nQuestion: {user_question}"
        )
    } else {
        format!(
            "{history_prefix}\
Below are excerpts from the user's loaded legal documents. \
Answer the current question using ONLY these excerpts.\n\n\
{context}\n\n---\n\nCurrent question: {user_question}"
        )
    };

    // "Answer:" is placed AFTER [/INST] (in the assistant turn), not before it.
    // Pre-filling the assistant's first token primes the model to start answering
    // immediately. Putting it before [/INST] (in the user turn) causes the model
    // to generate an EOG token immediately → empty response.
    let prompt = format!("[INST] <<SYS>>\n{RULES_PROMPT}\n<</SYS>>\n\n{user_content} [/INST] Answer:");

    tokio::task::spawn_blocking(move || {
        // Get (or lazily initialize) the global llama.cpp backend.
        // Returns Err if init failed rather than panicking.
        let backend = get_llama_backend()?;

        // Validate GGUF magic bytes before loading. A corrupted file would cause
        // llama.cpp to call abort() — which kills the whole process immediately.
        validate_gguf(&gguf_path)?;

        // Lock model cache; load from disk on first call only
        let mut model_guard = model_cache
            .lock()
            .map_err(|e| format!("Model mutex poisoned: {e}"))?;

        if model_guard.is_none() {
            log::info!("Loading Saul model from disk (first query)…");
            // Offload all layers to Metal GPU on Apple Silicon — dramatically reduces
            // RSS and prevents OOM kills on 8 GB machines.
            let model_params = LlamaModelParams::default().with_n_gpu_layers(100);
            let model = LlamaModel::load_from_file(backend, &gguf_path, &model_params)
                .map_err(|e| format!("Failed to load Saul model: {e}"))?;
            *model_guard = Some(model);
            log::info!("Saul model loaded and cached.");
        }

        let model = model_guard.as_ref()
            .ok_or_else(|| "Saul model unavailable after initialization".to_string())?;

        // Create a fresh context per query (allocates KV cache, ~seconds not minutes).
        // n_ctx=4096 comfortably fits legal document context + system prompt + generation.
        // Previously 2048 caused ggml_abort() (uncatchable SIGABRT) when prompts exceeded
        // the KV cache size — the root cause of the crash reported on Thread 15.
        let n_ctx_size: u32 = 4096;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx_size));
        let mut ctx = model
            .new_context(backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {e}"))?;

        // Tokenize prompt
        let mut tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| format!("Tokenize error: {e}"))?;

        let n_tokens = tokens.len();
        if n_tokens == 0 {
            return Err("Empty token sequence".to_string());
        }

        // Safety guard: if the prompt still overflows after the char-level pre-truncation,
        // keep the FIRST 180 tokens (BOS + [INST] <<SYS>> rules <</SYS>>\n\n) and the
        // LAST (budget - 180) tokens (end of context + question + [/INST]).
        // This preserves both the instruction-mode markers and the user question
        // instead of blindly draining from the front (which destroys [INST]/[/INST]).
        let max_prompt_tokens = n_ctx_size as usize - 512;
        if n_tokens > max_prompt_tokens {
            log::warn!(
                "Prompt ({} tokens) exceeds safe limit ({}). Preserving head+tail.",
                n_tokens,
                max_prompt_tokens
            );
            let head = 180usize.min(max_prompt_tokens / 2);
            let tail = max_prompt_tokens - head;
            let tail_start = n_tokens - tail;
            let mut kept: Vec<_> = tokens[..head].to_vec();
            kept.extend_from_slice(&tokens[tail_start..]);
            tokens = kept;
        }
        let n_tokens = tokens.len();

        // Decode the prompt in chunks to respect llama.cpp's n_batch limit.
        // Default n_batch = 512; decoding more tokens at once triggers ggml_abort()
        // which kills the entire process — the root cause of the SIGABRT crash.
        const DECODE_BATCH_SIZE: usize = 512;
        let mut batch = LlamaBatch::new(DECODE_BATCH_SIZE, 1);
        let mut chunk_start = 0;
        while chunk_start < n_tokens {
            let chunk_end = (chunk_start + DECODE_BATCH_SIZE).min(n_tokens);
            batch.clear();
            for pos in chunk_start..chunk_end {
                let is_last = pos == n_tokens - 1;
                batch
                    .add(tokens[pos], pos as i32, &[0], is_last)
                    .map_err(|e| format!("Batch add error: {e}"))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| format!("Prompt decode error: {e}"))?;
            chunk_start = chunk_end;
        }

        // Autoregressive generation.
        // penalties() prevents repetition loops. top_k + top_p narrow the candidate
        // set, then dist(42) samples probabilistically.
        //
        // IMPORTANT: temp=0.2 combined with freq/presence penalties (0.2/0.2) caused
        // logit collapse — the distribution became degenerate after penalties heavily
        // suppressed tokens and then temperature sharpened further, leaving dist() with
        // essentially no valid candidates → empty or garbage output.
        // Fix: raise temp to 0.6 and drop freq/presence penalties (only repeat penalty
        // is needed to prevent loops; temp handles diversity).
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(64, 1.1, 0.0, 0.0),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::temp(0.3),
            LlamaSampler::dist(42),
        ]);
        let mut response = String::new();
        let mut pos = n_tokens;
        let max_new_tokens = 1024usize;

        for _ in 0..max_new_tokens {
            // Hard stop if we've consumed the entire context window — decoding one
            // more token would overflow the KV cache and trigger ggml_abort().
            if pos >= n_ctx_size as usize {
                log::warn!("Generation stopped: reached context window limit ({n_ctx_size} tokens).");
                break;
            }

            // idx = -1 samples from the last computed logit position
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            // Convert token to UTF-8 bytes; lossy-decode (handles BPE fragments)
            let output_bytes = model
                .token_to_piece_bytes(token, 128, false, None)
                .map_err(|e| format!("Token decode error: {e}"))?;
            let token_piece = String::from_utf8_lossy(&output_bytes).into_owned();
            window.emit("query-token", token_piece.as_str()).ok();
            response.push_str(&token_piece);

            // Feed the generated token back for next-step prediction
            batch.clear();
            batch
                .add(token, pos as i32, &[0], true)
                .map_err(|e| format!("Gen batch add error: {e}"))?;
            ctx.decode(&mut batch)
                .map_err(|e| format!("Gen decode error: {e}"))?;
            pos += 1;
        }

        // Strip common generation artifacts before returning to the UI.
        let answer = response
            .trim()
            .trim_start_matches("<s>")
            .trim()
            .trim_end_matches("</s>")
            .trim_end_matches("[INST]")
            .trim_end_matches("[/INST]")
            .trim()
            .to_string();

        // Final sanity pass: remove any non-printable / private-use-area characters
        // that could have leaked from a garbled document into the model's context and
        // been echoed back. Users should never see "encrypted-looking" characters.
        let answer: String = answer
            .chars()
            .filter(|&c| {
                let code = c as u32;
                c == '\n'
                    || c == '\t'
                    || (!c.is_control()
                        && !(0xE000..=0xF8FF).contains(&code)
                        && code < 0xFFF0)
            })
            .collect();

        // Strip "Answer:" prefix if the model echoed the priming suffix back.
        let answer = answer
            .strip_prefix("Answer:")
            .or_else(|| answer.strip_prefix("Answer: "))
            .unwrap_or(&answer)
            .trim()
            .to_string();

        Ok(answer)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── File Loading ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_files(
    file_paths: Vec<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<FileInfo>, String> {
    let (settings, model_dir) = {
        let s = state.lock().await;
        (s.settings.clone(), s.model_dir.clone())
    };

    // Expand directories to individual files
    let mut expanded: Vec<String> = Vec::new();
    for fp in &file_paths {
        if let Ok(entries) = std::fs::read_dir(fp) {
            for entry in entries.flatten() {
                let path = entry.path();
                let lower = path.to_string_lossy().to_lowercase();
                if lower.ends_with(".pdf") || lower.ends_with(".docx") {
                    expanded.push(path.to_string_lossy().to_string());
                }
            }
        } else {
            expanded.push(fp.clone());
        }
    }

    let mut results: Vec<FileInfo> = Vec::new();
    let mut last_error: Option<String> = None;
    for file_path in expanded {
        match process_file(&file_path, &settings, &model_dir, &state).await {
            Ok(info) => {
                if info.chunk_count == 0 {
                    let msg = format!("File loaded but embedding failed — check that the embedding model downloaded correctly: {}", info.file_name);
                    log::warn!("{}", msg);
                    last_error = Some(msg);
                }
                results.push(info);
            }
            Err(e) => {
                log::error!("Failed to load {}: {}", file_path, e);
                last_error = Some(e);
            }
        }
    }

    if results.is_empty() {
        return Err(last_error.unwrap_or_else(|| "No files could be loaded.".to_string()));
    }

    Ok(results)
}

async fn process_file(
    file_path: &str,
    settings: &AppSettings,
    model_dir: &PathBuf,
    state: &tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<FileInfo, String> {
    use super::doc_parser;

    let lower = file_path.to_lowercase();
    let pages = if lower.ends_with(".pdf") {
        doc_parser::parse_pdf(file_path)?
    } else if lower.ends_with(".docx") {
        doc_parser::parse_docx(file_path)?
    } else {
        return Err(format!("Unsupported file type: {}", file_path));
    };

    let file_name = std::path::Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    let word_count: u32 = pages
        .iter()
        .map(|p| p.text.split_whitespace().count() as u32)
        .sum();

    let total_pages = pages.len() as u32;
    let doc_id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let chunks = chunk_document(&pages, settings);

    // Embed all chunks first without holding the state lock, then insert atomically.
    // This prevents partial-write state if the process is interrupted mid-embedding.
    let mut new_entries: Vec<(String, EmbeddedChunkEntry)> = Vec::new();
    for chunk in &chunks {
        // Quality gate: skip chunks that are mostly private-use-area or control
        // characters — real encoding garbage from bad PDF fonts.
        // IMPORTANT: use the same definition as is_printable_pdf_char() in the
        // parser, NOT is_ascii_punctuation(). The ASCII-only variant incorrectly
        // rejects em-dashes, smart quotes, §, ©, •, accented letters and other
        // chars that are perfectly valid in real legal documents, causing the entire
        // chunk to be silently dropped and leaving the LLM with no context.
        let total_chars = chunk.text.chars().count();
        if total_chars > 0 {
            let printable = chunk.text
                .chars()
                .filter(|&c| {
                    let code = c as u32;
                    c == '\n' || c == '\t'
                        || (!c.is_control()
                            && !(0xE000..=0xF8FF).contains(&code)
                            && code < 0xFFF0)
                })
                .count();
            let ratio = printable as f32 / total_chars as f32;
            if ratio < 0.20 {
                log::warn!(
                    "Skipping chunk {} — only {:.0}% printable chars (PDF encoding garbage)",
                    chunk.chunk_index,
                    ratio * 100.0
                );
                continue;
            }
        }

        match embed_text(&chunk.text, false, model_dir).await {
            Ok(vector) => {
                let item_id = Uuid::new_v4().to_string();
                let entry = EmbeddedChunkEntry {
                    id: item_id.clone(),
                    vector,
                    meta: ChunkMetadata {
                        id: chunk.id.clone(),
                        document_id: doc_id.clone(),
                        file_name: file_name.clone(),
                        file_path: file_path.to_string(),
                        page_number: chunk.page_number,
                        chunk_index: chunk.chunk_index,
                        text: chunk.text.clone(),
                        token_count: chunk.token_count,
                    },
                };
                new_entries.push((item_id, entry));
            }
            Err(e) => log::error!("Embed error for chunk {}: {}", chunk.chunk_index, e),
        }
    }

    let item_ids: Vec<String> = new_entries.iter().map(|(id, _)| id.clone()).collect();
    let file_info = FileInfo {
        id: doc_id.clone(),
        file_name: file_name.clone(),
        file_path: file_path.to_string(),
        total_pages,
        word_count,
        loaded_at: now,
        chunk_count: item_ids.len(),
    };

    // Single lock acquisition: insert all chunks + registry entries + save.
    {
        let mut s = state.lock().await;
        for (item_id, entry) in new_entries {
            s.chunk_registry.insert(item_id.clone(), entry.meta.clone());
            s.embedded_chunks.push(entry);
        }
        s.doc_chunk_ids.insert(doc_id.clone(), item_ids);
        s.file_registry.insert(doc_id.clone(), file_info.clone());
        s.save_chunks().await;
    }

    Ok(file_info)
}

// ── Chunking ──────────────────────────────────────────────────────────────────

struct TempChunk {
    id: String,
    page_number: u32,
    chunk_index: usize,
    text: String,
    token_count: usize,
}

#[derive(PartialEq)]
enum FragKind { Normal, ParagraphBreak }

struct SentenceFrag<'a> { text: &'a str, kind: FragKind }

/// Split `text` into sub-slices each at most `max_bytes` bytes long,
/// always cutting at a valid UTF-8 char boundary so no character is mangled.
fn split_at_char_boundaries(text: &str, max_bytes: usize) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let raw_end = (start + max_bytes).min(text.len());
        let end = text.floor_char_boundary(raw_end);
        // floor_char_boundary can return `start` when max_bytes < one char's width.
        // In that case scan forward to the next boundary so we always make progress.
        let end = if end <= start {
            let mut e = start + 1;
            while e < text.len() && !text.is_char_boundary(e) { e += 1; }
            e
        } else {
            end
        };
        let s = text[start..end].trim();
        if !s.is_empty() { parts.push(s); }
        start = end;
    }
    parts
}

fn chunk_document(pages: &[DocumentPage], settings: &AppSettings) -> Vec<TempChunk> {
    let mut chunks = Vec::new();
    let mut global_idx = 0usize;

    for page in pages {
        let text = &page.text;
        if text.trim().is_empty() {
            continue;
        }

        let frags = split_sentences(text);
        let mut current = String::new();
        let mut sentence_buf: Vec<&str> = Vec::new();
        let mut pending_header: Option<String> = None;

        let flush = |current: &str,
                     global_idx: &mut usize,
                     chunks: &mut Vec<TempChunk>,
                     page_num: u32| {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                chunks.push(TempChunk {
                    id: Uuid::new_v4().to_string(),
                    page_number: page_num,
                    chunk_index: *global_idx,
                    text: trimmed.to_string(),
                    token_count: (trimmed.len() / 3).max(1),
                });
                *global_idx += 1;
            }
        };

        for frag in &frags {
            if frag.kind == FragKind::ParagraphBreak {
                // Short line with no sentence-ending punctuation: treat as a
                // section/clause header — park it so it prepends the next chunk.
                // Only park as an orphan header when the structural pattern is
                // explicitly recognised. is_section_header already encodes all
                // the length and punctuation guards, so no separate checks needed.
                let is_orphan = is_section_header(frag.text);

                if is_orphan {
                    if !current.is_empty() {
                        flush(&current, &mut global_idx, &mut chunks, page.page_number);
                        current.clear();
                        sentence_buf.clear();
                    }
                    // Accumulate consecutive headers (ARTICLE → SECTION → (a))
                    // rather than overwriting so no structural context is dropped.
                    pending_header = Some(match pending_header.take() {
                        Some(existing) => format!("{existing}\n{}", frag.text),
                        None => frag.text.to_string(),
                    });
                    continue;
                }

                // Non-orphan paragraph break: flush current, no overlap carried across
                if !current.is_empty() {
                    flush(&current, &mut global_idx, &mut chunks, page.page_number);
                    current.clear();
                    sentence_buf.clear();
                }
                if let Some(h) = pending_header.take() {
                    current.push_str(&h);
                    current.push('\n');
                }
                // Apply sub-span splitting in case the paragraph-break fragment
                // itself exceeds chunk_size (same guard as the Normal path).
                let pb_subs = if frag.text.len() > settings.chunk_size {
                    split_at_char_boundaries(frag.text, settings.chunk_size)
                } else {
                    vec![frag.text]
                };
                for sub in pb_subs {
                    if !current.is_empty() && current.len() + sub.len() + 1 > settings.chunk_size {
                        flush(&current, &mut global_idx, &mut chunks, page.page_number);
                        current.clear();
                        sentence_buf.clear();
                    }
                    if !current.is_empty() { current.push(' '); }
                    current.push_str(sub);
                    sentence_buf.push(sub);
                }
                continue;
            }

            // Normal fragment: apply any parked header first
            if let Some(h) = pending_header.take() {
                if !current.is_empty() {
                    flush(&current, &mut global_idx, &mut chunks, page.page_number);
                    current.clear();
                    sentence_buf.clear();
                }
                current.push_str(&h);
                current.push('\n');
            }

            // If a single fragment is larger than chunk_size, split into sub-spans
            // at valid UTF-8 char boundaries (not raw bytes).
            let sub_sentences: Vec<&str> = if frag.text.len() > settings.chunk_size {
                split_at_char_boundaries(frag.text, settings.chunk_size)
            } else {
                vec![frag.text]
            };

            for sub in sub_sentences {
                if !current.is_empty() && current.len() + sub.len() + 1 > settings.chunk_size {
                    flush(&current, &mut global_idx, &mut chunks, page.page_number);

                    let mut overlap_parts: Vec<&str> = Vec::new();
                    let mut overlap_len = 0usize;
                    for s in sentence_buf.iter().rev() {
                        if overlap_len + s.len() + 1 > settings.chunk_overlap {
                            break;
                        }
                        overlap_parts.push(s);
                        overlap_len += s.len() + 1;
                    }
                    overlap_parts.reverse();
                    current = overlap_parts.join(" ");
                    sentence_buf.clear();
                }

                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(sub);
                sentence_buf.push(sub);
            }
        }

        // Flush remainder; if a lone header is pending, emit it as its own chunk
        if let Some(h) = pending_header {
            if !current.is_empty() {
                flush(&current, &mut global_idx, &mut chunks, page.page_number);
                current.clear();
            }
            current.push_str(&h);
        }
        flush(&current, &mut global_idx, &mut chunks, page.page_number);
    }

    chunks
}

fn is_section_header(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || t.len() >= 80 { return false; }
    // Numbered clause — checked BEFORE the ends_with('.') guard because bare
    // markers like "2." or "3.1" end with a structural dot, not a sentence terminator.
    // Prevents content sentences like "1. The Employee shall receive..." from
    // being orphaned (their after-text is > 40 chars so they fall through).
    if t.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        if let Some(dot_pos) = t.find('.') {
            if t[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let after = t[dot_pos + 1..].trim();
                // Reject if the label content itself ends with sentence punctuation
                // (e.g. "1. The salary is $50,000." — after = "The salary is $50,000.")
                if after.len() <= 40
                    && !after.ends_with('.')
                    && !after.ends_with('!')
                    && !after.ends_with('?') {
                    return true;
                }
            }
        }
    }
    // Lines ending with sentence punctuation are content, not headers
    // (this guard intentionally comes AFTER the numbered-clause check above)
    if t.ends_with('.') || t.ends_with('!') || t.ends_with('?') { return false; }
    let u = t.to_uppercase();
    if u.starts_with("SECTION") || u.starts_with("ARTICLE") || u.starts_with("WHEREAS")
        || u.starts_with("NOW THEREFORE") || u.starts_with("SCHEDULE")
        || u.starts_with("EXHIBIT") || u.starts_with("ANNEX") {
        return true;
    }
    // All-uppercase line (≤ 8 words, ≥ 6 chars, at least one letter):
    // catches "DEFINITIONS", "GOVERNING LAW", "LIMITATION OF LIABILITY", etc.
    if t.len() >= 6
        && t.chars().any(|c| c.is_alphabetic())
        && t.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
        && t.split_whitespace().count() <= 8 {
        return true;
    }
    // Lettered sub-clause: "(a)", "(b)", "(aa)", "(abc)" etc. — any sequence of
    // letters between parens, only if the label content after is short (≤ 40 chars).
    if t.starts_with('(') {
        if let Some(close) = t.find(')') {
            let inner = &t[1..close];
            if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_alphabetic()) {
                let after = t[close + 1..].trim();
                if after.len() <= 40 { return true; }
            }
        }
    }
    false
}

fn split_sentences(text: &str) -> Vec<SentenceFrag<'_>> {
    let mut frags = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut next_para_break = false;

    while i < len {
        let b = bytes[i];
        if (b == b'.' || b == b'!' || b == b'?')
            && i + 1 < len
            && bytes[i + 1].is_ascii_whitespace()
        {
            // For '.', skip known abbreviations and single-letter initials so that
            // "Mr. Smith", "Dr. Jones", "U.S. Code", "Inc." etc. don't create a
            // sentence break and produce tiny nonsensical chunks.
            let is_boundary = if b == b'.' {
                let mut word_start = i;
                while word_start > start && !bytes[word_start - 1].is_ascii_whitespace() {
                    word_start -= 1;
                }
                let word = &bytes[word_start..i];
                if word.is_empty() || (word.len() == 1 && word[0].is_ascii_alphabetic()) {
                    false
                } else {
                    const ABBREVS: &[&[u8]] = &[
                        b"mr", b"mrs", b"ms", b"dr", b"prof", b"sr", b"jr",
                        b"vs", b"etc", b"inc", b"corp", b"ltd", b"co",
                        b"no", b"sec", b"art", b"fig", b"est", b"approx",
                        b"jan", b"feb", b"mar", b"apr", b"jun", b"jul",
                        b"aug", b"sep", b"oct", b"nov", b"dec",
                    ];
                    let word_lower: Vec<u8> =
                        word.iter().map(|c| c.to_ascii_lowercase()).collect();
                    !ABBREVS.iter().any(|abbr| *abbr == word_lower.as_slice())
                }
            } else {
                true
            };

            if is_boundary {
                let s = text[start..=i].trim();
                if !s.is_empty() {
                    let kind = if next_para_break || is_section_header(s) {
                        next_para_break = false;
                        FragKind::ParagraphBreak
                    } else {
                        FragKind::Normal
                    };
                    frags.push(SentenceFrag { text: s, kind });
                }
                let mut j = i + 1;
                let mut newline_count = 0usize;
                while j < len && bytes[j].is_ascii_whitespace() {
                    if bytes[j] == b'\n' { newline_count += 1; }
                    j += 1;
                }
                // A blank line (2+ newlines) after a sentence-ending punctuation mark
                // signals a paragraph boundary, even though the whitespace was consumed
                // here rather than by the \n branch. Flag the next fragment accordingly.
                if newline_count >= 2 { next_para_break = true; }
                start = j;
                i = j;
            } else {
                i += 1;
            }
        } else if b == b'\n' {
            // Treat newlines as line boundaries — critical for structured documents.
            // Track blank lines (2+ consecutive newlines) as paragraph breaks.
            let s = text[start..i].trim();
            let mut j = i + 1;
            while j < len && bytes[j] == b'\n' { j += 1; }
            let blank = (j - i) >= 2;

            if !s.is_empty() {
                let kind = if next_para_break || is_section_header(s) {
                    FragKind::ParagraphBreak
                } else {
                    FragKind::Normal
                };
                next_para_break = false;
                frags.push(SentenceFrag { text: s, kind });
            }
            if blank { next_para_break = true; }
            start = j;
            i = j;
        } else {
            i += 1;
        }
    }

    let remainder = text[start..].trim();
    if !remainder.is_empty() {
        let kind = if next_para_break || is_section_header(remainder) {
            FragKind::ParagraphBreak
        } else {
            FragKind::Normal
        };
        frags.push(SentenceFrag { text: remainder, kind });
    }

    frags
}

// ── RAG Query ─────────────────────────────────────────────────────────────────

/// Expand query keywords with common legal/employment synonyms so that
/// "salary" matches chunks containing "compensation", "wages", etc.
fn expand_keywords(keywords: &std::collections::HashSet<String>) -> std::collections::HashSet<String> {
    const SYNONYMS: &[(&str, &[&str])] = &[
        ("salary",          &["compensation", "remuneration", "pay", "wage", "wages", "earnings", "income"]),
        ("compensation",    &["salary", "pay", "remuneration", "wage", "wages", "earnings"]),
        ("wage",            &["salary", "pay", "compensation", "earnings", "income"]),
        ("pay",             &["salary", "compensation", "wage", "payment", "remuneration"]),
        ("offer",           &["proposal", "agreement", "letter", "terms", "offeror"]),
        ("job",             &["position", "role", "employment", "work", "post"]),
        ("hire",            &["employ", "employment", "onboard", "recruit", "position"]),
        ("employee",        &["candidate", "staff", "worker", "personnel", "applicant"]),
        ("employer",        &["company", "organization", "firm", "corporation", "employer"]),
        ("contract",        &["agreement", "terms", "letter", "document"]),
        ("benefit",         &["benefits", "perk", "perks", "bonus", "allowance", "bonuses"]),
        ("start",           &["commence", "begin", "effective", "commencement", "joining"]),
        ("date",            &["effective", "commencement", "period", "term"]),
        ("annual",          &["yearly", "per year", "per annum"]),
        ("breach",          &["violation", "default", "failure", "infringement", "non-performance"]),
        ("damages",         &["liability", "remedy", "award", "loss", "penalty", "compensation"]),
        ("covenant",        &["agreement", "clause", "promise", "obligation", "undertaking"]),
        ("warranty",        &["representation", "guarantee", "assurance", "certification"]),
        ("jurisdiction",    &["venue", "court", "forum", "governing law", "choice of law"]),
        ("indemnify",       &["indemnification", "hold harmless", "defend", "reimburse"]),
        ("confidential",    &["confidentiality", "proprietary", "trade secret", "nda", "privileged"]),
        ("terminate",       &["termination", "cancel", "rescind", "dissolve", "expire", "end"]),
        ("consideration",   &["payment", "fee", "exchange", "value", "price"]),
        ("liability",       &["obligation", "responsibility", "exposure", "risk"]),
        ("amendment",       &["modification", "addendum", "revision", "supplement"]),
        ("party",           &["parties", "signatory", "counterpart", "entity"]),
        ("arbitration",     &["dispute resolution", "mediation", "adr", "tribunal", "hearing"]),
        ("force majeure",   &["act of god", "unforeseeable", "impossibility", "beyond control"]),
        ("assignment",      &["transfer", "delegate", "convey", "assign", "succession"]),
    ];

    let mut expanded = keywords.clone();
    for kw in keywords.iter() {
        for (key, syns) in SYNONYMS {
            if kw == key {
                for &syn in *syns {
                    expanded.insert(syn.to_string());
                }
            }
        }
    }
    expanded
}

/// Maximal Marginal Relevance — select `top_k` diverse, relevant chunks.
///
/// At each step picks the candidate that maximises:
///   MMR(d) = λ · relevance(d) − (1−λ) · max_sim(d, already_selected)
///
/// λ = 1.0 → pure relevance (equivalent to top-k by score)
/// λ = 0.0 → pure diversity
/// λ = 0.7 → good RAG default: mostly relevant, penalises near-duplicate passages
///
/// This prevents the LLM receiving 6 nearly-identical excerpts from the same
/// paragraph while relevant context from other sections is ignored.
fn mmr_select(
    mut candidates: Vec<(f32, ChunkMetadata, Vec<f32>)>,
    top_k: usize,
    lambda: f32,
) -> Vec<(f32, ChunkMetadata)> {
    let mut selected: Vec<(f32, ChunkMetadata, Vec<f32>)> = Vec::with_capacity(top_k);

    for _ in 0..top_k {
        if candidates.is_empty() {
            break;
        }
        let best_idx = candidates
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let mmr_a = if selected.is_empty() {
                    a.0
                } else {
                    let max_sim = selected
                        .iter()
                        .map(|(_, _, v)| RagState::cosine_similarity(&a.2, v))
                        .fold(0.0f32, f32::max);
                    lambda * a.0 - (1.0 - lambda) * max_sim
                };
                let mmr_b = if selected.is_empty() {
                    b.0
                } else {
                    let max_sim = selected
                        .iter()
                        .map(|(_, _, v)| RagState::cosine_similarity(&b.2, v))
                        .fold(0.0f32, f32::max);
                    lambda * b.0 - (1.0 - lambda) * max_sim
                };
                mmr_a
                    .partial_cmp(&mmr_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        if let Some(idx) = best_idx {
            selected.push(candidates.remove(idx));
        }
    }

    selected
        .into_iter()
        .map(|(score, meta, _)| (score, meta))
        .collect()
}

#[tauri::command]
pub async fn query(
    question: String,
    history: Vec<(String, String)>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
    window: tauri::Window,
) -> Result<QueryResult, String> {
    let (settings, model_dir, model_cache) = {
        let s = state.lock().await;
        (
            s.settings.clone(),
            s.model_dir.clone(),
            Arc::clone(&s.llama_model),
        )
    };

    window.emit("query-status", serde_json::json!({"phase": "embedding"})).ok();
    let query_vec = embed_text(&question, true, &model_dir).await?;

    // Build a set of meaningful query keywords for hybrid re-ranking.
    // Hybrid score = 0.80 * cosine_sim + 0.20 * keyword_overlap.
    // BGE-small-en-v1.5 is a retrieval model so cosine scores are reliable; keyword is a light fallback.
    let stop_words: std::collections::HashSet<&str> = [
        "a","an","the","is","are","was","were","be","been","being","have","has","had",
        "do","does","did","will","would","could","should","may","might","shall","can",
        "i","me","my","we","our","you","your","he","she","it","they","what","which",
        "who","this","that","these","those","of","in","on","at","by","for","with",
        "about","as","into","to","from","and","but","or","not","any","all","some",
        "how","when","where","why","there","find","show","tell","explain","give",
        "please","provide","describe",
    ].iter().cloned().collect();

    let base_keywords: std::collections::HashSet<String> = question
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !stop_words.contains(*w))
        .map(|w| w.to_string())
        .collect();
    // Expand with synonyms so "salary" also matches "compensation", etc.
    let query_keywords = expand_keywords(&base_keywords);

    let candidate_k = (settings.top_k * 6).min(60);
    let results = {
        let s = state.lock().await;

        window.emit("query-status", serde_json::json!({"phase": "searching", "chunks": s.embedded_chunks.len()})).ok();

        // No chunks at all → no files were successfully embedded.
        if s.embedded_chunks.is_empty() {
            return Ok(QueryResult {
                answer: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded.".to_string(),
                citations: vec![],
                not_found: true,
            });
        }

        // Include vectors so MMR can compute inter-chunk similarity.
        let mut scored: Vec<(f32, ChunkMetadata, Vec<f32>)> = s
            .embedded_chunks
            .iter()
            .map(|entry| {
                let cosine = RagState::cosine_similarity(&query_vec, &entry.vector);

                // Keyword overlap bonus: fraction of query keywords found in chunk text.
                let text_lower = entry.meta.text.to_lowercase();
                let kw_hits = query_keywords
                    .iter()
                    .filter(|kw| text_lower.contains(kw.as_str()))
                    .count();
                let kw_score = if query_keywords.is_empty() {
                    0.0f32
                } else {
                    kw_hits as f32 / query_keywords.len() as f32
                };

                let hybrid = 0.80 * cosine + 0.20 * kw_score;
                (hybrid, entry.meta.clone(), entry.vector.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Keep everything above threshold; if nothing qualifies, use bare top-k so
        // the LLM always has some context (it will say "not found" if truly irrelevant).
        let above_threshold: Vec<_> = scored
            .iter()
            .filter(|(score, _, _)| *score >= SCORE_THRESHOLD)
            .cloned()
            .collect();

        let pool = if above_threshold.is_empty() {
            scored.into_iter().take(candidate_k).collect::<Vec<_>>()
        } else {
            above_threshold.into_iter().take(candidate_k).collect::<Vec<_>>()
        };

        // MMR: select top_k maximally diverse chunks from the candidate pool.
        // lambda=0.7 → 70 % relevance / 30 % diversity.
        // This replaces the old per-page cap + take: MMR naturally prevents the LLM
        // from receiving multiple near-identical excerpts from the same paragraph
        // while genuinely relevant passages from other sections are ignored.
        mmr_select(pool, settings.top_k, 0.7)
    };

    let context_parts: Vec<String> = results
        .iter()
        .enumerate()
        .map(|(i, (_, meta))| {
            format!(
                "SOURCE {} — {}, Page {}:\n\"{}\"",
                i + 1,
                meta.file_name,
                meta.page_number,
                meta.text
            )
        })
        .collect();

    // Neighbor chunk expansion: include adjacent chunks (same file + page) for surrounding context.
    let selected_ids: std::collections::HashSet<&str> =
        results.iter().map(|(_, m)| m.id.as_str()).collect();
    let neighbor_context_parts = {
        let s = state.lock().await;
        let mut parts: Vec<String> = Vec::new();
        for (_, meta) in &results {
            for delta in [-1i64, 1i64] {
                let nbr_idx = meta.chunk_index as i64 + delta;
                if nbr_idx < 0 {
                    continue;
                }
                if let Some(nbr) = s.chunk_registry.values().find(|c| {
                    c.file_path == meta.file_path
                        && c.chunk_index == nbr_idx as usize
                        && !selected_ids.contains(c.id.as_str())
                }) {
                    parts.push(nbr.text.clone());
                }
            }
        }
        parts
    };

    let mut raw_context = context_parts.join("\n\n---\n\n");
    if !neighbor_context_parts.is_empty() {
        raw_context.push_str("\n\n--- Surrounding Context ---\n\n");
        raw_context.push_str(&neighbor_context_parts.join("\n\n"));
        log::info!(
            "Neighbor expansion added {} extra chunks ({} total context chars).",
            neighbor_context_parts.len(),
            raw_context.len()
        );
    }

    // Pre-truncate context chars before prompt assembly.
    // Saul-7B has a 4096-token context. Budget: 4096 − 1024 (generation) − 250 (overhead) = 2822
    // tokens for context. Legal text tokenizes at ~2.5 chars/token → 2822 × 2.5 ≈ 7055 chars max.
    // Using 7000 fits cleanly and avoids the head+tail truncation fallback.
    const MAX_CONTEXT_CHARS: usize = 7_000;
    let context = if raw_context.len() > MAX_CONTEXT_CHARS {
        let safe_end = raw_context.floor_char_boundary(MAX_CONTEXT_CHARS);
        log::warn!(
            "Document context truncated from {} to {} chars to fit context window.",
            raw_context.len(),
            safe_end
        );
        raw_context[..safe_end].to_string()
    } else {
        raw_context
    };

    window.emit("query-status", serde_json::json!({"phase": "generating"})).ok();
    let answer = ask_saul(&question, &context, &history, &model_dir, model_cache, window.clone()).await?;

    let answer_lower = answer.to_lowercase();
    // Only mark as truly not-found for the model's explicit "not found" signal.
    // Avoid matching partial phrases that appear in valid answers.
    let not_found = answer.is_empty()
        || answer_lower.contains("i could not find information")
        || answer_lower.contains("could not find information about this")
        || answer_lower.contains("documents do not contain");

    // Always build citations from the retrieved chunks — even on not_found,
    // showing the sources lets the user investigate the document directly.
    let citations: Vec<Citation> = results
        .iter()
        .map(|(score, meta)| Citation {
            file_name: meta.file_name.clone(),
            file_path: meta.file_path.clone(),
            page_number: meta.page_number,
            excerpt: RagState::best_excerpt(&meta.text, &question),
            score: *score,
        })
        .collect();

    Ok(QueryResult {
        answer,
        citations,
        not_found,
    })
}

// ── File Registry ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_files(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<FileInfo>, String> {
    let s = state.lock().await;
    Ok(s.file_registry.values().cloned().collect())
}

#[tauri::command]
pub async fn remove_file(
    file_id: String,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    let item_ids: Vec<String> = s.doc_chunk_ids.get(&file_id).cloned().unwrap_or_default();
    for id in &item_ids {
        s.chunk_registry.remove(id);
    }
    s.embedded_chunks.retain(|e| !item_ids.contains(&e.id));
    s.doc_chunk_ids.remove(&file_id);
    s.file_registry.remove(&file_id);
    s.save_chunks().await;
    Ok(())
}

// ── Document Viewer ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_file_data(file_path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Could not read file {}: {}", file_path, e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

#[tauri::command]
pub async fn get_page_text(
    file_path: String,
    page_number: u32,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<String, String> {
    let s = state.lock().await;
    Ok(s.get_page_text(&file_path, page_number))
}

// ── Settings ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<AppSettings, String> {
    let s = state.lock().await;
    Ok(s.settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    s.settings = settings;
    s.save_settings().await;
    Ok(())
}

// ── Chat Sessions ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn save_session(
    session: ChatSession,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<bool, String> {
    let mut s = state.lock().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    if let Some(existing) = s.sessions.iter_mut().find(|sess| sess.id == session.id) {
        *existing = ChatSession {
            updated_at: now,
            ..session
        };
    } else {
        s.sessions.insert(0, ChatSession { updated_at: now, ..session });
        s.sessions.truncate(50);
    }

    s.save_sessions().await;
    Ok(true)
}

#[tauri::command]
pub async fn get_sessions(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<ChatSession>, String> {
    let s = state.lock().await;
    Ok(s.sessions.clone())
}

#[tauri::command]
pub async fn delete_session(
    session_id: String,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<bool, String> {
    let mut s = state.lock().await;
    s.sessions.retain(|sess| sess.id != session_id);
    s.save_sessions().await;
    Ok(true)
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppSettings;

    fn default_settings() -> AppSettings {
        AppSettings {
            chunk_size: 500,
            chunk_overlap: 50,
            top_k: 6,
        }
    }

    // ── is_section_header ──────────────────────────────────────────────────

    #[test]
    fn header_keyword_prefixes() {
        assert!(is_section_header("SECTION 3 — COMPENSATION"));
        assert!(is_section_header("Article IV: Termination"));
        assert!(is_section_header("WHEREAS the parties agree"));
        assert!(is_section_header("NOW THEREFORE the parties"));
        assert!(is_section_header("Schedule A"));
        assert!(is_section_header("Exhibit B — Fee Schedule"));
        assert!(is_section_header("ANNEX 1"));
    }

    #[test]
    fn header_all_caps() {
        assert!(is_section_header("DEFINITIONS"));
        assert!(is_section_header("GOVERNING LAW"));
        assert!(is_section_header("LIMITATION OF LIABILITY"));
        assert!(is_section_header("INDEMNIFICATION"));
        assert!(is_section_header("RECITALS"));
        assert!(is_section_header("IN WITNESS WHEREOF"));
    }

    #[test]
    fn header_numbered_short_label() {
        assert!(is_section_header("1. Definitions"));
        assert!(is_section_header("3.1 Grant of License"));
        assert!(is_section_header("12. Governing Law"));
        // Bare number-dot with nothing after — still a header
        assert!(is_section_header("2."));
    }

    #[test]
    fn header_numbered_long_content_is_not_header() {
        // Fix 1: content sentences starting with a number must NOT be orphaned
        assert!(!is_section_header("1. The Employee shall receive five weeks of paid vacation annually"));
        assert!(!is_section_header("3. The Company agrees to provide health insurance benefits for the employee"));
        assert!(!is_section_header("2. All notices under this Agreement shall be delivered in writing"));
    }

    #[test]
    fn header_lettered_subclause_short_label() {
        assert!(is_section_header("(a) Base Salary"));
        assert!(is_section_header("(b) Bonus"));
        assert!(is_section_header("(aa) General Provisions"));
        // Bare letter-paren with nothing after
        assert!(is_section_header("(c)"));
    }

    #[test]
    fn header_lettered_subclause_long_content_is_not_header() {
        // Fix 1 (lettered variant): long clause content must NOT be orphaned
        assert!(!is_section_header("(a) The licensor hereby grants a non-exclusive, non-transferable license"));
        assert!(!is_section_header("(b) The Employee shall not disclose any confidential information"));
    }

    #[test]
    fn header_rejects_sentence_punctuation() {
        assert!(!is_section_header("This is a sentence."));
        assert!(!is_section_header("1. The salary is $50,000."));
        assert!(!is_section_header("DEFINITIONS."));
    }

    #[test]
    fn header_rejects_long_lines() {
        // ≥ 80 chars always rejected regardless of content
        let long = "SECTION 1 — THIS IS A VERY LONG HEADER THAT EXCEEDS EIGHTY CHARACTERS IN TOTAL LENGTH";
        assert!(long.len() >= 80);
        assert!(!is_section_header(long));
    }

    // ── split_at_char_boundaries ───────────────────────────────────────────

    #[test]
    fn char_boundary_split_ascii() {
        let parts = split_at_char_boundaries("hello world foo bar", 10);
        // Each part ≤ 10 bytes, no part empty
        for p in &parts { assert!(p.len() <= 10); assert!(!p.is_empty()); }
        // Reassembled text contains all words
        let joined = parts.join(" ");
        assert!(joined.contains("hello"));
        assert!(joined.contains("bar"));
    }

    #[test]
    fn char_boundary_split_multibyte() {
        // Em-dash (3 bytes), section sign § (2 bytes), smart quote " (3 bytes)
        let text = "\u{00a7} 4.1 \u{2014} Compensation \u{201c}as defined\u{201d}";
        let parts = split_at_char_boundaries(text, 8);
        // Every slice must be valid UTF-8 (from_utf8 would panic if not)
        for p in &parts {
            assert!(std::str::from_utf8(p.as_bytes()).is_ok());
            assert!(!p.is_empty());
        }
    }

    #[test]
    fn char_boundary_split_empty_input() {
        assert!(split_at_char_boundaries("", 100).is_empty());
    }

    // ── split_sentences ────────────────────────────────────────────────────

    #[test]
    fn split_tags_blank_line_as_para_break() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let frags = split_sentences(text);
        // "First paragraph." → Normal (no blank line before it)
        // "Second paragraph." → ParagraphBreak (blank line preceded it)
        let kinds: Vec<_> = frags.iter().map(|f| &f.kind).collect();
        assert_eq!(kinds.len(), 2);
        assert!(matches!(kinds[0], FragKind::Normal));
        assert!(matches!(kinds[1], FragKind::ParagraphBreak));
    }

    #[test]
    fn split_tags_section_header_as_para_break() {
        let text = "Prior content.\nSECTION 4 — COMPENSATION\nNext content.";
        let frags = split_sentences(text);
        let header = frags.iter().find(|f| f.text.contains("COMPENSATION")).unwrap();
        assert!(matches!(header.kind, FragKind::ParagraphBreak));
    }

    #[test]
    fn split_normal_sentence_after_single_newline_is_normal() {
        let text = "Line one\nLine two";
        let frags = split_sentences(text);
        // Single newline, no blank line — second line is Normal unless it's a header
        let second = frags.iter().find(|f| f.text.contains("Line two")).unwrap();
        assert!(matches!(second.kind, FragKind::Normal));
    }

    // ── chunk_document ─────────────────────────────────────────────────────

    fn make_page(text: &str) -> crate::state::DocumentPage {
        crate::state::DocumentPage { page_number: 1, text: text.to_string() }
    }

    #[test]
    fn chunk_header_prepended_to_following_content() {
        // Fix 2+3: a known section header should be prepended to the chunk that follows it
        let text = "SECTION 1 — COMPENSATION\n\nThe base salary shall be one hundred thousand dollars per year as agreed.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        // There should be exactly one chunk (the header + content, no orphan)
        assert_eq!(chunks.len(), 1, "Expected 1 chunk, got {}: {:?}", chunks.len(), chunks.iter().map(|c| &c.text).collect::<Vec<_>>());
        assert!(chunks[0].text.contains("SECTION 1"), "Header missing from chunk: {}", chunks[0].text);
        assert!(chunks[0].text.contains("base salary"), "Content missing from chunk: {}", chunks[0].text);
    }

    #[test]
    fn chunk_numbered_content_not_orphaned() {
        // Fix 1: "1. The Employee shall receive..." must NOT be treated as orphan header
        let text = "Preamble text here.\n\n1. The Employee shall receive five weeks of paid vacation per year.\n\n2. The Employee is entitled to full medical coverage.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        // Each numbered clause should be its own chunk (paragraph break), NOT prepended to the next
        let has_clause_1 = chunks.iter().any(|c| c.text.contains("five weeks"));
        let has_clause_2 = chunks.iter().any(|c| c.text.contains("medical coverage"));
        assert!(has_clause_1, "Clause 1 content missing");
        assert!(has_clause_2, "Clause 2 content missing");
        // Clause 1 text should NOT be prepended to clause 2's chunk
        for c in &chunks {
            if c.text.contains("medical coverage") {
                assert!(!c.text.contains("five weeks"), "Clause 1 was incorrectly prepended to clause 2");
            }
        }
    }

    #[test]
    fn chunk_consecutive_headers_accumulated() {
        // Fix 3: ARTICLE IV → SECTION 4.1 should both appear in the following chunk
        let text = "ARTICLE IV\nSECTION 4.1\n\nThe termination provisions are as follows and shall apply.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        let content_chunk = chunks.iter().find(|c| c.text.contains("termination provisions")).unwrap();
        assert!(content_chunk.text.contains("ARTICLE IV"), "ARTICLE IV missing: {}", content_chunk.text);
        assert!(content_chunk.text.contains("SECTION 4.1"), "SECTION 4.1 missing: {}", content_chunk.text);
    }

    #[test]
    fn chunk_no_overlap_across_paragraph_breaks() {
        // Paragraph breaks should flush without carrying overlap into the next chunk.
        // Sentences from paragraph A must not appear in paragraph B's chunk.
        let text = "Alpha sentence one. Alpha sentence two.\n\nBeta sentence one. Beta sentence two.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        for c in &chunks {
            if c.text.contains("Beta") {
                assert!(!c.text.contains("Alpha"), "Alpha leaked into Beta chunk via overlap: {}", c.text);
            }
        }
    }

    #[test]
    fn chunk_token_count_uses_div3() {
        // Fix (token count): token_count should be len/3, not len/4
        let text = "The quick brown fox jumps over the lazy dog and keeps running away.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        assert!(!chunks.is_empty());
        let expected = (chunks[0].text.len() / 3).max(1);
        assert_eq!(chunks[0].token_count, expected, "token_count should use /3");
    }

    // ── format_history ─────────────────────────────────────────────────────

    #[test]
    fn format_history_capped_at_4_turns() {
        // Fix 8: only the last 4 turns should appear in the formatted history
        let history: Vec<(String, String)> = (0..8)
            .map(|i| (format!("user{i}"), format!("assistant{i}")))
            .collect();
        let result = format_history(&history);
        // Turns 0-3 should be absent; turns 4-7 should be present
        assert!(!result.contains("user0"), "Turn 0 should be excluded");
        assert!(!result.contains("user3"), "Turn 3 should be excluded");
        assert!(result.contains("user4"), "Turn 4 should be included");
        assert!(result.contains("user7"), "Turn 7 should be included");
    }

    #[test]
    fn format_history_short_history_unchanged() {
        let history = vec![
            ("q1".to_string(), "a1".to_string()),
            ("q2".to_string(), "a2".to_string()),
        ];
        let result = format_history(&history);
        assert!(result.contains("q1"));
        assert!(result.contains("q2"));
    }
}
