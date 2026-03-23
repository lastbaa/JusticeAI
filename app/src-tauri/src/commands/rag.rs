use crate::pipeline::{
    self, chunk_document, embed_text, RetrievalBackend, GGUF_MIN_SIZE, SAUL_GGUF_URL,
    SCORE_THRESHOLD,
};
use crate::state::{
    AppSettings, Case, ChatSession, ChunkMetadata, Citation, EmbeddedChunkEntry, FileInfo,
    Jurisdiction, ModelStatus, QueryResult, RagState,
};
use base64::Engine;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

// ── Disk space helper ────────────────────────────────────────────────────────

/// Returns available disk space in bytes for the volume containing `path`.
/// Returns `None` if the check cannot be performed on this platform.
#[cfg(unix)]
fn available_disk_space(path: &std::path::Path) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(path.to_str()?).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return None;
    }
    let stat = unsafe { stat.assume_init() };
    Some(stat.f_bavail as u64 * stat.f_frsize as u64)
}

#[cfg(windows)]
fn available_disk_space(path: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    // GetDiskFreeSpaceExW requires a null-terminated wide string.
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free_bytes: u64 = 0;
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_bytes as *mut u64,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok != 0 { Some(free_bytes) } else { None }
}

#[cfg(not(any(unix, windows)))]
fn available_disk_space(_path: &std::path::Path) -> Option<u64> {
    None // On other platforms, skip the check gracefully
}

// ── Model Management ─────────────────────────────────────────────────────────

const OCR_DETECTION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const OCR_RECOGNITION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

#[tauri::command]
pub async fn check_models(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<ModelStatus, String> {
    let (gguf_path, model_dir) = {
        let s = state.lock().await;
        (s.model_dir.join("saul.gguf"), s.model_dir.clone())
    };
    let size = gguf_path.metadata().ok().map(|m| m.len()).unwrap_or(0);
    let ocr_dir = model_dir.join("ocr");
    let ocr_ready = ocr_dir.join("text-detection.rten").metadata().map(|m| m.len() > 1024).unwrap_or(false)
        && ocr_dir.join("text-recognition.rten").metadata().map(|m| m.len() > 1024).unwrap_or(false);
    Ok(ModelStatus {
        llm_ready: size > GGUF_MIN_SIZE,
        llm_size_gb: size as f32 / 1e9,
        download_required_gb: 4.5,
        ocr_ready,
        ocr_message: if ocr_ready {
            None
        } else {
            Some("OCR models will be downloaded during setup.".to_string())
        },
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

    // ── Disk space check ──────────────────────────────────────────────────────
    // Require ~5 GB free to download models safely.
    const REQUIRED_BYTES: u64 = 5_000_000_000;
    if let Some(available) = available_disk_space(&model_dir) {
        if available < REQUIRED_BYTES {
            let avail_gb = available as f64 / 1e9;
            return Err(format!(
                "Insufficient disk space: {avail_gb:.1} GB available, ~5 GB required. Free up space and try again."
            ));
        }
    }

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
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        // Throttle progress events to ~5 per second to prevent UI jitter
        let now = std::time::Instant::now();
        if now.duration_since(last_emit).as_millis() >= 200 {
            last_emit = now;
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
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    // Rename tmp → final. On Windows, antivirus or indexing services may hold
    // the file briefly after close, so retry a few times before giving up.
    let mut rename_err = None;
    for attempt in 0..10 {
        match tokio::fs::rename(&tmp_path, &gguf_path).await {
            Ok(()) => { rename_err = None; break; }
            Err(e) => {
                rename_err = Some(e);
                if attempt < 9 {
                    log::warn!("Rename attempt {} failed, retrying in 500ms…", attempt + 1);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }
    if let Some(e) = rename_err {
        // Last resort: try copy + delete instead of atomic rename
        log::warn!("Rename failed after retries, falling back to copy: {e}");
        tokio::fs::copy(&tmp_path, &gguf_path)
            .await
            .map_err(|e2| format!("Failed to move model file: rename={e}, copy={e2}"))?;
        tokio::fs::remove_file(&tmp_path).await.ok();
    }

    // Download OCR models (~10MB total, fast)
    download_ocr_models(&model_dir).await?;

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

/// Download OCR detection + recognition ONNX models for the `ocrs` crate.
async fn download_ocr_models(model_dir: &std::path::Path) -> Result<(), String> {
    let ocr_dir = model_dir.join("ocr");
    tokio::fs::create_dir_all(&ocr_dir)
        .await
        .map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    for (url, filename) in [
        (OCR_DETECTION_URL, "text-detection.rten"),
        (OCR_RECOGNITION_URL, "text-recognition.rten"),
    ] {
        let dest = ocr_dir.join(filename);
        // Skip if already downloaded
        if dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            continue;
        }
        log::info!("Downloading OCR model: {filename}");
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Failed to download {filename}: HTTP {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        tokio::fs::write(&dest, &bytes)
            .await
            .map_err(|e| e.to_string())?;
        log::info!("Downloaded OCR model: {filename} ({} bytes)", bytes.len());
    }

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
    // Validate the filename portion for Windows-reserved names and invalid characters.
    // The Tauri save dialog normally returns safe paths, but we guard against edge cases
    // so the user gets a clear error instead of a cryptic OS message.
    #[cfg(windows)]
    {
        let path = std::path::Path::new(&file_path);
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let upper = stem.to_uppercase();
            const RESERVED: &[&str] = &[
                "CON", "PRN", "AUX", "NUL",
                "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
            ];
            if RESERVED.contains(&upper.as_str()) {
                return Err(format!("'{stem}' is a reserved filename on Windows. Please choose a different name."));
            }
        }
    }

    // Create parent directories if they don't exist (e.g. user typed a new folder in the dialog)
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
    }

    std::fs::write(&file_path, content.as_bytes()).map_err(|e| format!("Failed to write file: {e}"))
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
    log::info!(
        "Migrating {} chunk embeddings from AllMiniL → BGE-small-en-v1.5…",
        total
    );
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

// ── File Loading ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_files(
    file_paths: Vec<String>,
    case_id: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<FileInfo>, String> {
    let (settings, model_dir) = {
        let s = state.lock().await;
        (s.settings.clone(), s.model_dir.clone())
    };

    // Expand directories to individual files.
    // Use Path::extension() instead of string manipulation so we handle
    // Windows paths with backslashes and non-UTF-8 filenames correctly.
    let mut expanded: Vec<String> = Vec::new();
    for fp in &file_paths {
        let p = std::path::Path::new(fp);
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext_match = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .map(|e| {
                            super::doc_parser::SUPPORTED_EXTENSIONS
                                .iter()
                                .any(|sup| *sup == e)
                        })
                        .unwrap_or(false);
                    if ext_match {
                        // to_string_lossy is fine here — if the OS returned the
                        // path, it's valid for the current platform's filesystem.
                        expanded.push(path.to_string_lossy().to_string());
                    }
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
            Ok(mut info) => {
                if info.chunk_count == 0 {
                    let msg = format!("File loaded but embedding failed — check that the embedding model downloaded correctly: {}", info.file_name);
                    log::warn!("{}", msg);
                    last_error = Some(msg);
                }
                // Assign to case if specified
                if let Some(ref cid) = case_id {
                    info.case_id = Some(cid.clone());
                    let mut s = state.lock().await;
                    if let Some(fr) = s.file_registry.get_mut(&info.id) {
                        fr.case_id = Some(cid.clone());
                    }
                    s.save_file_registry().await;
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

    let ext = doc_parser::detect_supported_extension(file_path)?;
    let pages = doc_parser::parse_by_extension(file_path, &ext, model_dir)?;

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

    // Detect jurisdiction from first few pages (cap at 10,000 chars)
    let detect_text: String = pages.iter().take(3).map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let detect_text = if detect_text.len() > 10_000 { &detect_text[..10_000] } else { &detect_text };
    let detected_jurisdiction = pipeline::detect_jurisdiction(detect_text).map(|r| {
        log::info!(
            "Jurisdiction detected for {}: {:?} (confidence: {:.2}, signal: {})",
            file_name, r.jurisdiction, r.confidence, r.signal
        );
        r.jurisdiction
    });

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
            let printable = chunk
                .text
                .chars()
                .filter(|&c| {
                    let code = c as u32;
                    c == '\n'
                        || c == '\t'
                        || (!c.is_control() && !(0xE000..=0xF8FF).contains(&code) && code < 0xFFF0)
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
        case_id: None,
        detected_jurisdiction,
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

// ── Chunking (moved to pipeline.rs) ──────────────────────────────────────────

// chunk_document, is_section_header, split_sentences, expand_keywords, mmr_select
// are all defined in pipeline.rs and re-exported via the use statement at the top.

#[tauri::command]
pub async fn query(
    question: String,
    history: Vec<(String, String)>,
    case_id: Option<String>,
    case_context: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
    window: tauri::Window,
) -> Result<QueryResult, String> {
    let (settings, model_dir, model_cache, resolved_jurisdiction) = {
        let s = state.lock().await;

        // Resolve jurisdiction: case override > file consensus > global default > None
        let jurisdiction = if let Some(ref cid) = case_id {
            // Check case-level override first
            let case_j = s.cases.iter()
                .find(|c| c.id == *cid)
                .and_then(|c| c.jurisdiction.clone());
            if case_j.is_some() {
                case_j
            } else {
                // Check if all files in the case agree on a detected jurisdiction
                let file_jurisdictions: Vec<&Jurisdiction> = s.file_registry.values()
                    .filter(|f| f.case_id.as_deref() == Some(cid.as_str()))
                    .filter_map(|f| f.detected_jurisdiction.as_ref())
                    .collect();
                if !file_jurisdictions.is_empty() && file_jurisdictions.windows(2).all(|w| w[0] == w[1]) {
                    Some(file_jurisdictions[0].clone())
                } else {
                    s.settings.jurisdiction.clone()
                }
            }
        } else {
            s.settings.jurisdiction.clone()
        };

        (
            s.settings.clone(),
            s.model_dir.clone(),
            Arc::clone(&s.llama_model),
            jurisdiction,
        )
    };

    window
        .emit("query-status", serde_json::json!({"phase": "embedding"}))
        .ok();
    let query_vec = embed_text(&question, true, &model_dir).await?;

    let candidate_k = (settings.top_k * 6).min(60);
    let results = {
        let s = state.lock().await;

        // When case_id is set, filter to only chunks belonging to that case's files.
        // Build a mapping from filtered index → global index so we can map results back.
        let (filtered_chunks, global_indices): (Vec<&EmbeddedChunkEntry>, Vec<usize>) =
            if let Some(ref cid) = case_id {
                let case_doc_ids: std::collections::HashSet<&str> = s
                    .file_registry
                    .values()
                    .filter(|f| f.case_id.as_deref() == Some(cid.as_str()))
                    .map(|f| f.id.as_str())
                    .collect();
                s.embedded_chunks
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| case_doc_ids.contains(e.meta.document_id.as_str()))
                    .map(|(i, e)| (e, i))
                    .unzip()
            } else {
                s.embedded_chunks
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (e, i))
                    .unzip()
            };

        window
            .emit(
                "query-status",
                serde_json::json!({"phase": "searching", "chunks": filtered_chunks.len()}),
            )
            .ok();

        // No chunks at all → no files were successfully embedded.
        if filtered_chunks.is_empty() {
            return Ok(QueryResult {
                answer: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded.".to_string(),
                citations: vec![],
                not_found: true,
                assertions: vec![],
            });
        }

        // Use the pluggable retrieval backend.
        let backend = pipeline::default_backend();
        let corpus = pipeline::RetrievalCorpus {
            texts: filtered_chunks.iter().map(|e| e.meta.text.as_str()).collect(),
            vectors: filtered_chunks.iter().map(|e| e.vector.as_slice()).collect(),
        };
        let config = pipeline::RetrievalConfig {
            top_k: settings.top_k,
            candidate_pool_k: candidate_k,
            score_threshold: SCORE_THRESHOLD,
            mmr_lambda: 0.7,
            expand_keywords: true,
        };
        let mut ranked = backend.retrieve(&question, &query_vec, &corpus, &config);

        // Always include filled form data chunks — they're tiny and contain the actual answers.
        pipeline::ensure_form_data_included(&mut ranked, &corpus, 2);

        // Map ScoredResult indices back via global_indices to (score, ChunkMetadata).
        ranked
            .into_iter()
            .map(|r| {
                let global_idx = global_indices[r.chunk_index];
                (r.score, s.embedded_chunks[global_idx].meta.clone())
            })
            .collect::<Vec<(f32, ChunkMetadata)>>()
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

    // Assemble context with per-chunk budget so no chunk is cut mid-sentence.
    // Saul-7B has a 4096-token context. Budget: 4096 − 1024 (generation) − 250 (overhead) = 2822
    // tokens for context. Legal text tokenizes at ~2.5 chars/token → 2822 × 2.5 ≈ 7055 chars max.
    // When jurisdiction is active, reduce budget to account for extra prompt tokens.
    let max_context_chars: usize = if resolved_jurisdiction.is_some() { 6_600 } else { 7_000 };
    let separator = "\n\n---\n\n";
    let mut context = String::new();

    // Prepend cross-conversation case context if available
    if let Some(ref cc) = case_context {
        if !cc.is_empty() {
            let prefix = format!("--- Related conversations in this case ---\n{cc}\n\n");
            if prefix.len() <= max_context_chars {
                context.push_str(&prefix);
            }
        }
    }

    // Add primary context chunks (each stays complete)
    for part in &context_parts {
        let addition = if context.is_empty() { part.len() } else { part.len() + separator.len() };
        if context.len() + addition > max_context_chars {
            log::warn!(
                "Context budget exhausted at {} chars; skipping remaining chunks.",
                context.len()
            );
            break;
        }
        if !context.is_empty() {
            context.push_str(separator);
        }
        context.push_str(part);
    }

    // Append neighbor context only if budget remains
    if !neighbor_context_parts.is_empty() {
        let header = "\n\n--- Surrounding Context ---\n\n";
        let header_len = header.len();
        let mut added = 0usize;
        let mut neighbor_buf = String::new();
        for part in &neighbor_context_parts {
            let addition = if neighbor_buf.is_empty() { part.len() } else { part.len() + separator.len() };
            if context.len() + header_len + neighbor_buf.len() + addition > max_context_chars {
                break;
            }
            if !neighbor_buf.is_empty() {
                neighbor_buf.push_str("\n\n");
            }
            neighbor_buf.push_str(part);
            added += 1;
        }
        if !neighbor_buf.is_empty() {
            context.push_str(header);
            context.push_str(&neighbor_buf);
            log::info!(
                "Neighbor expansion added {} extra chunks ({} total context chars).",
                added,
                context.len()
            );
        }
    }

    window
        .emit("query-status", serde_json::json!({"phase": "generating"}))
        .ok();
    let window_clone = window.clone();
    let answer: String = pipeline::ask_saul(
        &question,
        &context,
        &history,
        &model_dir,
        model_cache,
        move |tok| {
            window_clone.emit("query-token", tok.as_str()).ok();
        },
        resolved_jurisdiction.as_ref(),
    )
    .await?;

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

    // Run answer quality assertions
    let known_files: Vec<&str> = results.iter().map(|(_, m)| m.file_name.as_str()).collect();
    let chunk_texts: Vec<&str> = results.iter().map(|(_, m)| m.text.as_str()).collect();
    let mut assertions = crate::assertions::check_citations(&answer, Some(&known_files));
    assertions.extend(crate::assertions::check_no_hallucination(&answer, &chunk_texts));

    Ok(QueryResult {
        answer,
        citations,
        not_found,
        assertions,
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
        s.sessions.insert(
            0,
            ChatSession {
                updated_at: now,
                ..session
            },
        );
        s.sessions.truncate(200);
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

// ── Case Management ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_cases(
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<Case>, String> {
    let s = state.lock().await;
    Ok(s.cases.clone())
}

#[tauri::command]
pub async fn save_case(
    case: Case,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(existing) = s.cases.iter_mut().find(|c| c.id == case.id) {
        *existing = case;
    } else {
        s.cases.push(case);
    }
    s.save_cases().await;
    Ok(())
}

#[tauri::command]
pub async fn delete_case(
    case_id: String,
    delete_contents: bool,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    s.cases.retain(|c| c.id != case_id);

    if delete_contents {
        // Remove all sessions belonging to this case
        s.sessions.retain(|session| session.case_id.as_deref() != Some(&case_id));

        // Remove all files belonging to this case (and their chunks)
        let file_ids: Vec<String> = s.file_registry.iter()
            .filter(|(_, f)| f.case_id.as_deref() == Some(&case_id))
            .map(|(id, _)| id.clone())
            .collect();
        for file_id in &file_ids {
            let item_ids: Vec<String> = s.doc_chunk_ids.get(file_id).cloned().unwrap_or_default();
            for id in &item_ids {
                s.chunk_registry.remove(id);
            }
            s.embedded_chunks.retain(|e| !item_ids.contains(&e.id));
            s.doc_chunk_ids.remove(file_id);
            s.file_registry.remove(file_id);
        }
        s.save_chunks().await;
    } else {
        // Orphan sessions and files belonging to this case
        for session in &mut s.sessions {
            if session.case_id.as_deref() == Some(&case_id) {
                session.case_id = None;
            }
        }
        for file in s.file_registry.values_mut() {
            if file.case_id.as_deref() == Some(&case_id) {
                file.case_id = None;
            }
        }
    }

    s.save_cases().await;
    s.save_sessions().await;
    s.save_file_registry().await;
    Ok(())
}

#[tauri::command]
pub async fn assign_file_to_case(
    file_id: String,
    case_id: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(file) = s.file_registry.get_mut(&file_id) {
        file.case_id = case_id;
    }
    s.save_file_registry().await;
    Ok(())
}

#[tauri::command]
pub async fn assign_session_to_case(
    session_id: String,
    case_id: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(session) = s.sessions.iter_mut().find(|sess| sess.id == session_id) {
        session.case_id = case_id;
    }
    s.save_sessions().await;
    Ok(())
}

#[tauri::command]
pub async fn set_case_jurisdiction(
    case_id: String,
    jurisdiction: Option<Jurisdiction>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(case) = s.cases.iter_mut().find(|c| c.id == case_id) {
        case.jurisdiction = jurisdiction;
        s.save_cases().await;
        Ok(())
    } else {
        Err(format!("Case not found: {case_id}"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseSummary {
    pub session_id: String,
    pub summary: String,
}

#[tauri::command]
pub async fn get_case_summaries(
    case_id: String,
    exclude_session_id: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<CaseSummary>, String> {
    let s = state.lock().await;
    let summaries: Vec<CaseSummary> = s
        .sessions
        .iter()
        .filter(|sess| {
            sess.case_id.as_deref() == Some(&case_id)
                && sess
                    .summary
                    .as_ref()
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                && exclude_session_id
                    .as_ref()
                    .map(|eid| sess.id != *eid)
                    .unwrap_or(true)
        })
        .map(|sess| CaseSummary {
            session_id: sess.id.clone(),
            summary: sess.summary.clone().unwrap_or_default(),
        })
        .collect();
    Ok(summaries)
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::pipeline::{
        chunk_document, format_history, is_section_header, split_at_char_boundaries,
        split_sentences, FragKind,
    };
    use crate::state::AppSettings;

    fn default_settings() -> AppSettings {
        AppSettings {
            chunk_size: 500,
            chunk_overlap: 50,
            top_k: 6,
            theme: "dark".to_string(),
            jurisdiction: None,
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
        assert!(!is_section_header(
            "1. The Employee shall receive five weeks of paid vacation annually"
        ));
        assert!(!is_section_header(
            "3. The Company agrees to provide health insurance benefits for the employee"
        ));
        assert!(!is_section_header(
            "2. All notices under this Agreement shall be delivered in writing"
        ));
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
        assert!(!is_section_header(
            "(a) The licensor hereby grants a non-exclusive, non-transferable license"
        ));
        assert!(!is_section_header(
            "(b) The Employee shall not disclose any confidential information"
        ));
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
        let long =
            "SECTION 1 — THIS IS A VERY LONG HEADER THAT EXCEEDS EIGHTY CHARACTERS IN TOTAL LENGTH";
        assert!(long.len() >= 80);
        assert!(!is_section_header(long));
    }

    // ── split_at_char_boundaries ───────────────────────────────────────────

    #[test]
    fn char_boundary_split_ascii() {
        let parts = split_at_char_boundaries("hello world foo bar", 10);
        // Each part ≤ 10 bytes, no part empty
        for p in &parts {
            assert!(p.len() <= 10);
            assert!(!p.is_empty());
        }
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
        let header = frags
            .iter()
            .find(|f| f.text.contains("COMPENSATION"))
            .unwrap();
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
        crate::state::DocumentPage {
            page_number: 1,
            text: text.to_string(),
        }
    }

    #[test]
    fn chunk_header_prepended_to_following_content() {
        // Fix 2+3: a known section header should be prepended to the chunk that follows it
        let text = "SECTION 1 — COMPENSATION\n\nThe base salary shall be one hundred thousand dollars per year as agreed.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        // There should be exactly one chunk (the header + content, no orphan)
        assert_eq!(
            chunks.len(),
            1,
            "Expected 1 chunk, got {}: {:?}",
            chunks.len(),
            chunks.iter().map(|c| &c.text).collect::<Vec<_>>()
        );
        assert!(
            chunks[0].text.contains("SECTION 1"),
            "Header missing from chunk: {}",
            chunks[0].text
        );
        assert!(
            chunks[0].text.contains("base salary"),
            "Content missing from chunk: {}",
            chunks[0].text
        );
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
                assert!(
                    !c.text.contains("five weeks"),
                    "Clause 1 was incorrectly prepended to clause 2"
                );
            }
        }
    }

    #[test]
    fn chunk_consecutive_headers_accumulated() {
        // Fix 3: ARTICLE IV → SECTION 4.1 should both appear in the following chunk
        let text =
            "ARTICLE IV\nSECTION 4.1\n\nThe termination provisions are as follows and shall apply.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        let content_chunk = chunks
            .iter()
            .find(|c| c.text.contains("termination provisions"))
            .unwrap();
        assert!(
            content_chunk.text.contains("ARTICLE IV"),
            "ARTICLE IV missing: {}",
            content_chunk.text
        );
        assert!(
            content_chunk.text.contains("SECTION 4.1"),
            "SECTION 4.1 missing: {}",
            content_chunk.text
        );
    }

    #[test]
    fn chunk_no_overlap_across_paragraph_breaks() {
        // Paragraph breaks should flush without carrying overlap into the next chunk.
        // Sentences from paragraph A must not appear in paragraph B's chunk.
        let text =
            "Alpha sentence one. Alpha sentence two.\n\nBeta sentence one. Beta sentence two.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        for c in &chunks {
            if c.text.contains("Beta") {
                assert!(
                    !c.text.contains("Alpha"),
                    "Alpha leaked into Beta chunk via overlap: {}",
                    c.text
                );
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
