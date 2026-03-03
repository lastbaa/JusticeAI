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

const SCORE_THRESHOLD: f32 = 0.15;
const MAX_CHUNKS_PER_PAGE: usize = 2;
const GGUF_MIN_SIZE: u64 = 4_000_000_000;

const SAUL_GGUF_URL: &str = "https://huggingface.co/MaziyarPanahi/Saul-Instruct-v1-GGUF/resolve/main/Saul-Instruct-v1.Q4_K_M.gguf";

const SYSTEM_PROMPT: &str = r#"You are Justice AI, a secure legal research assistant designed for legal professionals.

Your only job is to help the user find information within the documents they have loaded. You are NOT providing legal advice. You are a research and retrieval tool to support the legal professional using you.

Rules you must never break:
1. Answer ONLY using the document excerpts provided in the context below.
2. Always cite the exact filename and page number for every claim you make.
3. Always include a direct quoted excerpt from the source document to support your answer.
4. If the answer cannot be found in the provided documents, respond only with: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded."
5. Never use pretrained knowledge to fill gaps. Never guess. Never hallucinate.
6. Never provide legal advice or legal conclusions. If asked for a legal opinion, remind the user that Justice AI is a research tool and that legal conclusions are theirs to make.

Context from loaded documents:
{context}"#;

// ── Singletons ────────────────────────────────────────────────────────────────
// Both models are loaded once per process and cached for all subsequent calls.

// fastembed TextEmbedding: ~22 MB ONNX, downloaded to model_dir/fastembed/ on first use.
static EMBED_MODEL: OnceLock<Arc<Mutex<Option<fastembed::TextEmbedding>>>> = OnceLock::new();

// llama.cpp backend: AtomicBool guard means init() must only be called once.
static LLAMA_BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

fn get_llama_backend() -> &'static LlamaBackend {
    LLAMA_BACKEND.get_or_init(|| {
        LlamaBackend::init().expect("Failed to initialize llama.cpp backend")
    })
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

    let mut file = if already_downloaded > 0 {
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

    let mut downloaded = already_downloaded;

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

// ── Local Embedding via fastembed ─────────────────────────────────────────────
// The model is loaded once into EMBED_MODEL on the first call and reused for
// every subsequent chunk or query. Re-initializing per call was silent-failing.

pub async fn embed_text(text: &str, model_dir: &Path) -> Result<Vec<f32>, String> {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    let cache_dir = model_dir.join("fastembed");
    let text_owned = text.to_string();

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
                InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                    .with_cache_dir(cache_dir)
                    .with_show_download_progress(false),
            )
            .map_err(|e| format!("Failed to initialize embedding model: {e}"))?;
            *guard = Some(model);
        }

        let model = guard.as_ref().unwrap();
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

// ── Local LLM via llama-cpp-2 ─────────────────────────────────────────────────
//
// The model (4.5 GB) is loaded once and cached in `model_cache`.
// A fresh LlamaContext is created per query (lightweight — just KV cache alloc).

async fn ask_saul(
    system_prompt: &str,
    user_question: &str,
    model_dir: &Path,
    model_cache: Arc<Mutex<Option<llama_cpp_2::model::LlamaModel>>>,
) -> Result<String, String> {
    use llama_cpp_2::{
        context::params::LlamaContextParams,
        llama_batch::LlamaBatch,
        model::{params::LlamaModelParams, AddBos, LlamaModel},
        sampling::LlamaSampler,
    };
    use std::num::NonZeroU32;

    let gguf_path = model_dir.join("saul.gguf");
    let prompt = format!("[INST] <<SYS>>\n{system_prompt}\n<</SYS>>\n\n{user_question} [/INST]");

    tokio::task::spawn_blocking(move || {
        // Get (or lazily initialize) the global llama.cpp backend
        let backend = get_llama_backend();

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

        let model = model_guard.as_ref().unwrap();

        // Create a fresh context per query (allocates KV cache, ~seconds not minutes)
        // 2048 tokens is plenty for citation-grounded answers and uses half the KV memory.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048));
        let mut ctx = model
            .new_context(backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {e}"))?;

        // Tokenize prompt
        let tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| format!("Tokenize error: {e}"))?;

        let n_tokens = tokens.len();
        if n_tokens == 0 {
            return Err("Empty token sequence".to_string());
        }

        // Decode the prompt as a batch (only the last token needs logits)
        let mut batch = LlamaBatch::new(n_tokens, 1);
        for (pos, token) in tokens.iter().enumerate() {
            let is_last = pos == n_tokens - 1;
            batch
                .add(*token, pos as i32, &[0], is_last)
                .map_err(|e| format!("Batch add error: {e}"))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Prompt decode error: {e}"))?;

        // Autoregressive generation
        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
        let mut response = String::new();
        let mut pos = n_tokens;
        let max_new_tokens = 1024usize;

        for _ in 0..max_new_tokens {
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
            response.push_str(&String::from_utf8_lossy(&output_bytes));

            // Feed the generated token back for next-step prediction
            batch.clear();
            batch
                .add(token, pos as i32, &[0], true)
                .map_err(|e| format!("Gen batch add error: {e}"))?;
            ctx.decode(&mut batch)
                .map_err(|e| format!("Gen decode error: {e}"))?;
            pos += 1;
        }

        Ok(response)
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
    let mut item_ids: Vec<String> = Vec::new();

    for chunk in &chunks {
        match embed_text(&chunk.text, model_dir).await {
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
                let mut s = state.lock().await;
                s.chunk_registry.insert(item_id.clone(), entry.meta.clone());
                s.embedded_chunks.push(entry);
                item_ids.push(item_id);
            }
            Err(e) => log::error!("Embed error for chunk {}: {}", chunk.chunk_index, e),
        }
    }

    let file_info = FileInfo {
        id: doc_id.clone(),
        file_name: file_name.clone(),
        file_path: file_path.to_string(),
        total_pages,
        word_count,
        loaded_at: now,
        chunk_count: item_ids.len(),
    };

    {
        let mut s = state.lock().await;
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

fn chunk_document(pages: &[DocumentPage], settings: &AppSettings) -> Vec<TempChunk> {
    let mut chunks = Vec::new();
    let mut global_idx = 0usize;

    for page in pages {
        let text = &page.text;
        if text.trim().is_empty() {
            continue;
        }

        let sentences = split_sentences(text);
        let mut current = String::new();
        let mut sentence_buf: Vec<&str> = Vec::new();

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
                    token_count: (trimmed.len() / 4).max(1),
                });
                *global_idx += 1;
            }
        };

        for sentence in &sentences {
            if !current.is_empty() && current.len() + sentence.len() + 1 > settings.chunk_size {
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
            current.push_str(sentence);
            sentence_buf.push(sentence);
        }

        flush(&current, &mut global_idx, &mut chunks, page.page_number);
    }

    chunks
}

fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if (b == b'.' || b == b'!' || b == b'?')
            && i + 1 < len
            && bytes[i + 1].is_ascii_whitespace()
        {
            let s = text[start..=i].trim();
            if !s.is_empty() {
                sentences.push(s);
            }
            let mut j = i + 1;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            start = j;
            i = j;
        } else {
            i += 1;
        }
    }

    let remainder = text[start..].trim();
    if !remainder.is_empty() {
        sentences.push(remainder);
    }

    sentences
}

// ── RAG Query ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn query(
    question: String,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<QueryResult, String> {
    let (settings, model_dir, model_cache) = {
        let s = state.lock().await;
        (
            s.settings.clone(),
            s.model_dir.clone(),
            Arc::clone(&s.llama_model),
        )
    };

    let query_vec = embed_text(&question, &model_dir).await?;

    let candidate_k = (settings.top_k * 3).min(30);
    let results = {
        let s = state.lock().await;

        // No chunks at all → no files were successfully embedded.
        if s.embedded_chunks.is_empty() {
            return Ok(QueryResult {
                answer: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded.".to_string(),
                citations: vec![],
                not_found: true,
            });
        }

        let mut scored: Vec<(f32, ChunkMetadata)> = s
            .embedded_chunks
            .iter()
            .map(|entry| {
                let score = RagState::cosine_similarity(&query_vec, &entry.vector);
                (score, entry.meta.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Apply threshold filter; if nothing passes, fall back to bare top-k so the
        // LLM always has context to work with (it will say "not found" if truly unrelated).
        let above_threshold: Vec<_> = scored
            .iter()
            .filter(|(score, _)| *score >= SCORE_THRESHOLD)
            .cloned()
            .collect();

        let pool = if above_threshold.is_empty() {
            scored.into_iter().take(candidate_k).collect::<Vec<_>>()
        } else {
            above_threshold.into_iter().take(candidate_k).collect::<Vec<_>>()
        };

        let mut page_count: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        pool
            .into_iter()
            .filter(|(_, meta)| {
                let key = format!("{}::{}", meta.file_path, meta.page_number);
                let count = page_count.entry(key).or_insert(0);
                if *count >= MAX_CHUNKS_PER_PAGE {
                    return false;
                }
                *count += 1;
                true
            })
            .take(settings.top_k)
            .collect::<Vec<_>>()
    };

    let context_parts: Vec<String> = results
        .iter()
        .enumerate()
        .map(|(i, (_, meta))| {
            format!(
                "[{}] File: \"{}\" | Page {}\n{}",
                i + 1,
                meta.file_name,
                meta.page_number,
                meta.text
            )
        })
        .collect();
    let context = context_parts.join("\n\n---\n\n");
    let system_with_context = SYSTEM_PROMPT.replace("{context}", &context);

    let answer = ask_saul(&system_with_context, &question, &model_dir, model_cache).await?;

    let not_found = answer.to_lowercase().contains("i could not find")
        || answer.to_lowercase().contains("no relevant");

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
        citations: if not_found { vec![] } else { citations },
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
