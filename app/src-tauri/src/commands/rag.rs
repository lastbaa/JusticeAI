use crate::pipeline::{
    self, chunk_document, embed_text, greeting_response, is_non_document_query,
    is_simple_greeting, RetrievalBackend, GGUF_MIN_SIZE, SAUL_GGUF_URL,
    SCORE_THRESHOLD,
};
use crate::state::{
    AppSettings, Case, ChatSession, ChunkMetadata, Citation, DocumentRole, EmbeddedChunkEntry,
    EntityEntry, FactSheet, FileInfo, Jurisdiction, ModelStatus, QueryResult, RagState,
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

    use tokio::io::AsyncWriteExt;

    const MAX_RETRIES: u32 = 3;
    let mut attempt: u32 = 0;
    let mut downloaded: u64;
    let mut total_bytes: u64;

    loop {
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

        let response_result = request.send().await;
        let mut response = match response_result {
            Ok(r) => r,
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RETRIES {
                    return Err(format!("Download failed after {MAX_RETRIES} attempts: {e}"));
                }
                let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                log::warn!("Download attempt {attempt} failed: {e}. Retrying in {delay:?}…");
                window.emit("download-progress", serde_json::json!({
                    "percent": 0, "downloadedBytes": 0, "totalBytes": 0,
                    "done": false, "retrying": true, "attempt": attempt
                })).ok();
                tokio::time::sleep(delay).await;
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() && status.as_u16() != 206 {
            attempt += 1;
            if attempt >= MAX_RETRIES {
                return Err(format!("Download failed after {MAX_RETRIES} attempts: HTTP {status}"));
            }
            let delay = std::time::Duration::from_secs(2u64.pow(attempt));
            log::warn!("Download attempt {attempt} got HTTP {status}. Retrying in {delay:?}…");
            window.emit("download-progress", serde_json::json!({
                "percent": 0, "downloadedBytes": 0, "totalBytes": 0,
                "done": false, "retrying": true, "attempt": attempt
            })).ok();
            tokio::time::sleep(delay).await;
            continue;
        }

        total_bytes = if already_downloaded > 0 && status.as_u16() == 206 {
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
        downloaded = if resuming { already_downloaded } else { 0 };
        let mut last_emit = std::time::Instant::now();
        let mut stream_failed = false;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
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
                Ok(None) => break, // Stream complete
                Err(e) => {
                    log::warn!("Stream error during download: {e}");
                    stream_failed = true;
                    break;
                }
            }
        }

        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);

        if stream_failed {
            attempt += 1;
            if attempt >= MAX_RETRIES {
                return Err(format!("Download failed after {MAX_RETRIES} attempts: stream error"));
            }
            let delay = std::time::Duration::from_secs(2u64.pow(attempt));
            log::warn!("Download stream failed on attempt {attempt}. Retrying in {delay:?}…");
            window.emit("download-progress", serde_json::json!({
                "percent": 0, "downloadedBytes": downloaded, "totalBytes": total_bytes,
                "done": false, "retrying": true, "attempt": attempt
            })).ok();
            tokio::time::sleep(delay).await;
            continue;
        }

        break; // Download completed successfully
    }

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
    // Reject path traversal and system directories
    if file_path.contains("..") {
        return Err("Path traversal (..) is not allowed.".to_string());
    }
    #[cfg(unix)]
    {
        let blocked = ["/etc", "/usr", "/System", "/Library", "/bin", "/sbin", "/var"];
        for prefix in blocked {
            if file_path.starts_with(prefix) {
                return Err(format!("Writing to {prefix} is not allowed."));
            }
        }
    }

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

/// Detect documents whose stored chunks contain garbled text (from old lopdf/
/// pdf-extract extraction) and re-parse them with the improved pdf_oxide engine.
/// Scans embedded_chunks directly (works even if file_registry/doc_chunk_ids are empty).
pub async fn migrate_garbled_chunks(state: &mut RagState) {
    use super::doc_parser;
    use std::collections::{HashMap, HashSet};

    if state.embedded_chunks.is_empty() {
        return;
    }

    // 1. Group chunks by document_id, collecting file metadata.
    struct DocInfo {
        file_name: String,
        file_path: String,
        total_chars: usize,
        alpha_chars: usize,
    }
    let mut docs: HashMap<String, DocInfo> = HashMap::new();
    for entry in &state.embedded_chunks {
        let m = &entry.meta;
        let doc = docs.entry(m.document_id.clone()).or_insert_with(|| DocInfo {
            file_name: m.file_name.clone(),
            file_path: m.file_path.clone(),
            total_chars: 0,
            alpha_chars: 0,
        });
        for ch in m.text.chars() {
            if ch.is_whitespace() { continue; }
            doc.total_chars += 1;
            if ch.is_alphabetic() { doc.alpha_chars += 1; }
        }
    }

    // 2. Find garbled PDF documents (<40% alphabetic characters).
    let mut garbled: Vec<(String, String, String)> = Vec::new(); // (doc_id, file_name, file_path)
    for (doc_id, info) in &docs {
        if !info.file_name.to_lowercase().ends_with(".pdf") { continue; }
        if info.total_chars < 20 { continue; }
        let ratio = info.alpha_chars as f32 / info.total_chars as f32;
        if ratio < 0.40 {
            log::info!(
                "Garbled chunks for '{}' ({:.1}% alphabetic) — will re-parse with pdf_oxide",
                info.file_name, ratio * 100.0
            );
            garbled.push((doc_id.clone(), info.file_name.clone(), info.file_path.clone()));
        }
    }

    if garbled.is_empty() { return; }
    log::info!("Re-parsing {} garbled PDF(s)…", garbled.len());

    let settings = state.settings.clone();
    let model_dir = state.model_dir.clone();

    for (doc_id, file_name, file_path) in &garbled {
        let pages = match doc_parser::parse_pdf(file_path) {
            Ok(p) if !p.is_empty() => p,
            Ok(_) => { log::warn!("Re-parse of '{}' returned empty — skipping", file_name); continue; }
            Err(e) => { log::warn!("Re-parse of '{}' failed: {} — skipping", file_name, e); continue; }
        };

        // Remove old chunks for this document.
        let old_ids: HashSet<String> = state.embedded_chunks.iter()
            .filter(|e| &e.meta.document_id == doc_id)
            .map(|e| e.id.clone())
            .collect();
        state.embedded_chunks.retain(|e| !old_ids.contains(&e.id));
        state.invalidate_bm25_cache();
        for oid in &old_ids { state.chunk_registry.remove(oid); }
        state.doc_chunk_ids.remove(doc_id);

        // Re-chunk and re-embed.
        let chunks = chunk_document(&pages, &settings);
        let mut new_ids: Vec<String> = Vec::new();

        for chunk in &chunks {
            match embed_text(&chunk.text, false, &model_dir).await {
                Ok(vector) => {
                    let item_id = Uuid::new_v4().to_string();
                    let entry = EmbeddedChunkEntry {
                        id: item_id.clone(),
                        vector,
                        meta: ChunkMetadata {
                            id: chunk.id.clone(),
                            document_id: doc_id.clone(),
                            file_name: file_name.clone(),
                            file_path: file_path.clone(),
                            page_number: chunk.page_number,
                            chunk_index: chunk.chunk_index,
                            text: chunk.text.clone(),
                            token_count: chunk.token_count,
                            role: DocumentRole::default(),
                        },
                    };
                    state.chunk_registry.insert(item_id.clone(), entry.meta.clone());
                    state.embedded_chunks.push(entry);
                    new_ids.push(item_id);
                }
                Err(e) => log::error!("Re-embed error for chunk {}: {}", chunk.chunk_index, e),
            }
        }

        // Rebuild registries for this document.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_millis() as u64;
        state.doc_chunk_ids.insert(doc_id.clone(), new_ids.clone());
        state.file_registry.insert(doc_id.clone(), FileInfo {
            id: doc_id.clone(),
            file_name: file_name.clone(),
            file_path: file_path.clone(),
            total_pages: pages.len() as u32,
            word_count: pages.iter().map(|p| p.text.split_whitespace().count() as u32).sum(),
            loaded_at: now,
            chunk_count: new_ids.len(),
            case_id: None,
            detected_jurisdiction: None,
            role: DocumentRole::default(),
            fact_sheet: None,
        });

        log::info!("Re-parsed '{}': {} pages, {} chunks", file_name, pages.len(), new_ids.len());
    }

    state.save_chunks().await;
    state.save_file_registry().await;
    log::info!("Garbled chunk migration complete.");
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
    let detect_text = pipeline::safe_truncate(&detect_text, 10_000);
    let detected_jurisdiction = pipeline::detect_jurisdiction(detect_text).map(|r| {
        log::info!(
            "Jurisdiction detected for {}: {:?} (confidence: {:.2}, signal: {})",
            file_name, r.jurisdiction, r.confidence, r.signal
        );
        r.jurisdiction
    });

    // Extract fact sheet and entities from document text (regex-based, fast)
    let fact_sheet = extract_fact_sheet(&pages);
    let new_entities = extract_entities(&pages, &file_name);

    let chunks = chunk_document(&pages, settings);

    // Embed all chunks first without holding the state lock, then insert atomically.
    // This prevents partial-write state if the process is interrupted mid-embedding.

    // Quality gate: filter out chunks that are mostly private-use-area or control
    // characters — real encoding garbage from bad PDF fonts.
    // IMPORTANT: use the same definition as is_printable_pdf_char() in the
    // parser, NOT is_ascii_punctuation(). The ASCII-only variant incorrectly
    // rejects em-dashes, smart quotes, etc. that are perfectly valid.
    let good_chunks: Vec<&pipeline::TempChunk> = chunks
        .iter()
        .filter(|chunk| {
            let total_chars = chunk.text.chars().count();
            if total_chars == 0 {
                return true;
            }
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
                return false;
            }
            true
        })
        .collect();

    // Batch-embed all surviving chunks in a single call for efficiency.
    let texts: Vec<&str> = good_chunks.iter().map(|c| c.text.as_str()).collect();
    let embeddings = pipeline::embed_texts_batch(&texts, false, model_dir).await?;

    let mut new_entries: Vec<(String, EmbeddedChunkEntry)> = Vec::new();
    for (chunk, vector) in good_chunks.iter().zip(embeddings) {
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
                role: DocumentRole::ClientDocument,
            },
        };
        new_entries.push((item_id, entry));
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
        role: DocumentRole::ClientDocument,
        fact_sheet: Some(fact_sheet),
    };

    // Single lock acquisition: insert all chunks + registry entries + entities + save.
    {
        let mut s = state.lock().await;
        for (item_id, entry) in new_entries {
            s.chunk_registry.insert(item_id.clone(), entry.meta.clone());
            s.embedded_chunks.push(entry);
        }
        s.invalidate_bm25_cache();
        s.doc_chunk_ids.insert(doc_id.clone(), item_ids);
        s.file_registry.insert(doc_id.clone(), file_info.clone());
        // Add extracted entities (dedup by name)
        for entity in new_entities {
            if !s.entity_registry.iter().any(|e| e.name == entity.name) {
                s.entity_registry.push(entity);
            }
        }
        s.save_chunks().await;
    }

    Ok(file_info)
}

// ── Chunking (moved to pipeline.rs) ──────────────────────────────────────────

// chunk_document, is_section_header, split_sentences, expand_keywords, mmr_select
// are all defined in pipeline.rs and re-exported via the use statement at the top.

// ── Context Assembly Helpers ───────────────────────────────────────────────────

/// Allocate context budget proportionally to chunk scores.
/// Higher-scored chunks get more characters, lower-scored get fewer.
fn allocate_chunk_budgets(
    scores: &[f32],
    total_budget: usize,
    min_per_chunk: usize, // minimum 200 chars per chunk
) -> Vec<usize> {
    if scores.is_empty() {
        return vec![];
    }

    let total_score: f64 = scores.iter().map(|s| *s as f64).sum();
    if total_score <= 0.0 {
        // Equal allocation
        let per = total_budget / scores.len();
        return vec![per.max(min_per_chunk); scores.len()];
    }

    let mut budgets: Vec<usize> = scores
        .iter()
        .map(|s| {
            let ratio = *s as f64 / total_score;
            let budget = (ratio * total_budget as f64) as usize;
            budget.max(min_per_chunk)
        })
        .collect();

    // Adjust to not exceed total — remove from lowest-scored chunks first
    let sum: usize = budgets.iter().sum();
    if sum > total_budget {
        let mut excess = sum - total_budget;
        for i in (0..budgets.len()).rev() {
            if excess == 0 {
                break;
            }
            let reduction = excess.min(budgets[i].saturating_sub(min_per_chunk));
            budgets[i] -= reduction;
            excess -= reduction;
        }
    }

    budgets
}

/// Build a map of chunk neighbors in one pass over the registry.
/// Key: (file_path, chunk_index) → registry index.
/// Also builds a page map: (file_path) → sorted list of chunk_indices on that path.
fn build_neighbor_map(
    chunks: &[EmbeddedChunkEntry],
) -> std::collections::HashMap<(String, usize), usize> {
    let mut map: std::collections::HashMap<(String, usize), usize> =
        std::collections::HashMap::with_capacity(chunks.len());

    for (idx, chunk) in chunks.iter().enumerate() {
        let key = (chunk.meta.file_path.clone(), chunk.meta.chunk_index);
        map.insert(key, idx);
    }

    map
}

/// Only include neighbor if it shares >= 3 significant words with the retrieved chunk.
fn neighbor_is_relevant(target_text: &str, neighbor_text: &str) -> bool {
    let target_words: std::collections::HashSet<&str> = target_text
        .split_whitespace()
        .filter(|w| w.len() > 4)
        .collect();
    let neighbor_words: std::collections::HashSet<&str> = neighbor_text
        .split_whitespace()
        .filter(|w| w.len() > 4)
        .collect();
    let shared = target_words.intersection(&neighbor_words).count();
    shared >= 3
}

/// Truncate text to a budget without cutting mid-word or mid-sentence.
/// Prefers sentence boundaries (`. `), falls back to word boundaries.
fn truncate_to_budget(text: &str, budget: usize) -> String {
    if text.len() <= budget {
        return text.to_string();
    }

    let truncated = &text[..budget];
    // Find last sentence boundary
    if let Some(pos) = truncated.rfind(". ") {
        return format!("{}.", &truncated[..pos]);
    }
    // Find last word boundary
    if let Some(pos) = truncated.rfind(' ') {
        return format!("{}...", &truncated[..pos]);
    }
    format!("{}...", truncated)
}

#[tauri::command]
pub async fn query(
    question: String,
    history: Vec<(String, String)>,
    case_id: Option<String>,
    case_context: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
    window: tauri::Window,
) -> Result<QueryResult, String> {
    let (settings, model_dir, model_cache, resolved_jurisdiction, has_chunks) = {
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

        let has_chunks = !s.embedded_chunks.is_empty();

        (
            s.settings.clone(),
            s.model_dir.clone(),
            Arc::clone(&s.llama_model),
            jurisdiction,
            has_chunks,
        )
    };

    // ── Layer 1: Query intent classification ─────────────────────────────────
    // Detect greetings, chitchat, and off-topic queries BEFORE any retrieval.
    // Route them to the general chat prompt to prevent hallucination.
    if has_chunks && is_non_document_query(&question) {
        log::info!("Non-document query detected ('{}'); routing to general chat.", &question);

        // Simple greetings → hardcoded response, no LLM inference at all.
        // Saul-7B regurgitates bullet-point instructions as numbered lists,
        // so bypassing inference is the only reliable approach.
        if is_simple_greeting(&question) {
            log::info!("Simple greeting with docs loaded — hardcoded response.");
            let response = greeting_response(true);
            for word in response.split_inclusive(' ') {
                window.emit("query-token", word).ok();
            }
            return Ok(QueryResult {
                answer: response,
                citations: vec![],
                not_found: false,
                assertions: vec![],
                confidence: None,
            });
        }

        // Non-greeting general questions: use LLM with a safe, non-enumerable prompt
        let model_dir_clone = model_dir.clone();
        let window_clone = window.clone();
        window.emit("query-status", serde_json::json!({"phase": "generating"})).ok();
        let mut chat_params = pipeline::InferenceParams::from_mode(&settings.inference_mode);
        chat_params.system_prompt_override = Some(
            "You are Justice AI, a legal research assistant running locally on the user's device. \
            The user has documents loaded but is asking a general question. \
            Answer naturally and concisely. Mention that you are ready to help with their documents \
            whenever they have questions. Do not reference or fabricate any document content, \
            court names, case details, or legal citations.".to_string(),
        );
        let general_answer = pipeline::ask_saul(
            &question,
            "",
            &history,
            &model_dir_clone,
            Arc::clone(&model_cache),
            move |tok| { window_clone.emit("query-token", tok.as_str()).ok(); },
            resolved_jurisdiction.as_ref(),
            chat_params,
        ).await?;
        let answer_confidence = crate::assertions::compute_confidence(&general_answer, &[]);
        return Ok(QueryResult {
            answer: general_answer,
            citations: vec![],
            not_found: false,
            assertions: vec![],
            confidence: Some(answer_confidence),
        });
    }

    window
        .emit("query-status", serde_json::json!({"phase": "embedding"}))
        .ok();
    let query_vec = embed_text(&question, true, &model_dir).await?;

    let retrieval_params = pipeline::RetrievalModeParams::from_mode(&settings.inference_mode);
    let inference_params = pipeline::InferenceParams::from_mode(&settings.inference_mode);
    // Save inference param values before ask_saul consumes the struct (no Clone)
    let saved_max_new_tokens = inference_params.max_new_tokens;
    let saved_temperature = inference_params.temperature;
    let saved_system_prompt_suffix = inference_params.system_prompt_suffix;
    let saved_system_prompt_override = inference_params.system_prompt_override.clone();
    let saved_timeout_secs = inference_params.timeout_secs;
    let saved_is_quick = inference_params.is_quick;
    let candidate_k = retrieval_params.candidate_pool_k;

    // ── Single consolidated state lock for retrieval + cosine check + form injection + neighbor map ──
    let (mut results, best_cosine, form_chunks_to_inject, neighbor_context_parts) = {
        let mut s = state.lock().await;

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

        // No chunks at all → no documents loaded.
        if filtered_chunks.is_empty() {
            // Simple greetings → hardcoded response, no inference.
            if is_simple_greeting(&question) {
                log::info!("No documents + simple greeting — hardcoded response.");
                let response = greeting_response(false);
                for word in response.split_inclusive(' ') {
                    window.emit("query-token", word).ok();
                }
                return Ok(QueryResult {
                    answer: response,
                    citations: vec![],
                    not_found: false,
                    assertions: vec![],
                    confidence: None,
                });
            }

            // Drop the state lock BEFORE running LLM inference to avoid
            // blocking all other IPC commands during generation.
            drop(s);

            // Non-greeting general questions: use LLM with safe prompt
            window
                .emit("query-status", serde_json::json!({"phase": "generating"}))
                .ok();
            let window_clone = window.clone();
            let mut chat_params = inference_params;
            chat_params.system_prompt_override = Some(
                "You are Justice AI, a legal research assistant running locally on the user's device. \
                No documents are currently loaded. Answer the user's question naturally and concisely. \
                You can answer general legal knowledge questions, but remind the user you are not a lawyer. \
                Suggest they add documents to get cited, page-level answers from their own files. \
                Do not fabricate case citations, statutes, or specific legal advice.".to_string(),
            );
            let general_answer = pipeline::ask_saul(
                &question,
                "",
                &history,
                &model_dir,
                Arc::clone(&model_cache),
                move |tok| {
                    window_clone.emit("query-token", tok.as_str()).ok();
                },
                resolved_jurisdiction.as_ref(),
                chat_params,
            )
            .await?;
            let answer_confidence = crate::assertions::compute_confidence(&general_answer, &[]);
            return Ok(QueryResult {
                answer: general_answer,
                citations: vec![],
                not_found: false,
                assertions: vec![],
                confidence: Some(answer_confidence),
            });
        }

        // Use the pluggable retrieval backend.
        let backend = pipeline::default_backend();

        // Eagerly collect corpus data from filtered_chunks to release the immutable
        // borrow on `s.embedded_chunks` before we need `&mut s.bm25_cache`.
        let corpus_texts: Vec<String> = filtered_chunks.iter().map(|e| e.meta.text.clone()).collect();
        let corpus_vectors: Vec<Vec<f32>> = filtered_chunks.iter().map(|e| e.vector.clone()).collect();
        let corpus_chunk_indices: Vec<usize> = filtered_chunks.iter().map(|e| e.meta.chunk_index).collect();
        let filtered_count = filtered_chunks.len();
        // Drop the immutable borrow on s.embedded_chunks
        drop(filtered_chunks);

        // Build or reuse the BM25 index from cache (full corpus only; case-filtered
        // queries build a fresh index since the corpus subset differs).
        let text_refs: Vec<&str> = corpus_texts.iter().map(|t| t.as_str()).collect();
        let cached_bm25 = if case_id.is_none() {
            if !s.bm25_cache.valid || s.bm25_cache.doc_count != filtered_count {
                let fresh = pipeline::Bm25Index::build(&text_refs);
                fresh.write_to_cache(&mut s.bm25_cache);
                Some(pipeline::Bm25Index::from_cache(&s.bm25_cache))
            } else {
                Some(pipeline::Bm25Index::from_cache(&s.bm25_cache))
            }
        } else {
            None
        };
        let vector_refs: Vec<&[f32]> = corpus_vectors.iter().map(|v| v.as_slice()).collect();
        let corpus = pipeline::RetrievalCorpus {
            texts: text_refs,
            vectors: vector_refs,
            chunk_indices: corpus_chunk_indices,
            bm25_index: cached_bm25,
        };
        let config = pipeline::RetrievalConfig {
            top_k: retrieval_params.top_k,
            candidate_pool_k: candidate_k,
            score_threshold: SCORE_THRESHOLD,
            mmr_lambda: retrieval_params.mmr_lambda,
            expand_keywords: true,
            jaccard_threshold: retrieval_params.jaccard_threshold,
            adaptive_k_gap: retrieval_params.adaptive_k_gap,
        };
        let ranked = backend.retrieve(&question, &query_vec, &corpus, &config);

        // Map ScoredResult indices back via global_indices to (score, ChunkMetadata).
        let results: Vec<(f32, ChunkMetadata)> = ranked
            .into_iter()
            .map(|r| {
                let global_idx = global_indices[r.chunk_index];
                (r.score, s.embedded_chunks[global_idx].meta.clone())
            })
            .collect();

        // ── Cosine floor check (inside same lock) ────────────────────────────
        let best_cosine = if results.is_empty() {
            0.0f32
        } else {
            let mut best = 0.0f32;
            for (_, meta) in &results {
                if let Some(entry) = s.embedded_chunks.iter().find(|e| e.meta.id == meta.id) {
                    let cos = crate::state::RagState::cosine_similarity(&query_vec, &entry.vector);
                    if cos > best { best = cos; }
                }
            }
            best
        };

        // ── Form data injection (inside same lock) ───────────────────────────
        let form_chunks: Vec<(f32, ChunkMetadata)> = s
            .embedded_chunks
            .iter()
            .filter(|e| e.meta.text.starts_with("FILLED FORM DATA"))
            .filter(|e| !results.iter().any(|(_, m)| m.id == e.meta.id))
            .take(2)
            .map(|e| (0.5, e.meta.clone()))
            .collect();

        // ── Single-pass neighbor map (inside same lock) ──────────────────────
        // Build O(1) lookup map, then find neighbors without scanning the registry.
        let neighbor_parts = if settings.inference_mode == crate::state::InferenceMode::Quick {
            Vec::new()
        } else {
            let selected_ids: std::collections::HashSet<&str> =
                results.iter().map(|(_, m)| m.id.as_str()).collect();
            let neighbor_map = build_neighbor_map(&s.embedded_chunks);
            let mut parts: Vec<String> = Vec::new();
            let mut seen_neighbors: std::collections::HashSet<(String, usize)> =
                std::collections::HashSet::new();
            for (_, meta) in &results {
                for delta in [-1i64, 1i64] {
                    let nbr_idx = meta.chunk_index as i64 + delta;
                    if nbr_idx < 0 {
                        continue;
                    }
                    let key = (meta.file_path.clone(), nbr_idx as usize);
                    if seen_neighbors.contains(&key) {
                        continue;
                    }
                    if let Some(&global_idx) = neighbor_map.get(&key) {
                        let nbr = &s.embedded_chunks[global_idx];
                        if !selected_ids.contains(nbr.meta.id.as_str())
                            && neighbor_is_relevant(&meta.text, &nbr.meta.text)
                        {
                            parts.push(nbr.meta.text.clone());
                            seen_neighbors.insert(key);
                        }
                    }
                }
            }
            parts
        };

        (results, best_cosine, form_chunks, neighbor_parts)
    };
    // ── State lock released ──────────────────────────────────────────────────

    // ── Layer 2: Cosine floor check ───────────────────────────────────────────
    // If retrieval returned nothing (all below threshold) OR the raw cosine
    // similarity of the best chunk is too low, route to general chat.
    // This prevents hallucination when the query is unrelated to loaded documents.
    let mode_cosine_floor = retrieval_params.cosine_floor;
    let route_to_general = if results.is_empty() {
        log::info!("Retrieval returned no results above threshold; routing to general chat.");
        true
    } else if best_cosine < mode_cosine_floor {
        log::info!(
            "Best cosine similarity ({:.3}) below floor ({:.2}); routing to general chat.",
            best_cosine, mode_cosine_floor
        );
        true
    } else {
        false
    };

    if route_to_general {
        // Return a hardcoded not-found response instead of running inference.
        // Saul-7B regurgitates system prompt instructions as numbered lists,
        // so bypassing it entirely avoids that problem.
        log::info!("No relevant chunks found — returning hardcoded not-found response.");
        let not_found_msg = "I could not find relevant information in your loaded documents.\n\n\
            **Suggestions**\n\
            - Rephrase your question with different keywords\n\
            - Check that the relevant documents are loaded\n\
            - Try a broader or more specific question".to_string();
        for word in not_found_msg.split_inclusive(' ') {
            window.emit("query-token", word).ok();
        }
        return Ok(QueryResult {
            answer: not_found_msg,
            citations: vec![],
            not_found: true,
            assertions: vec![],
            confidence: None,
        });
    }

    // Inject filled form data chunks AFTER cosine floor check passes.
    for fc in form_chunks_to_inject {
        results.insert(1.min(results.len()), fc);
    }

    // ── Priority-weighted context assembly ────────────────────────────────────
    // Saul-7B has a 4096-token context. Budget is mode-dependent (see RetrievalModeParams).
    // When jurisdiction is active, reduce budget to account for extra prompt tokens.
    let max_context_chars: usize = {
        let base = if resolved_jurisdiction.is_some() {
            retrieval_params.max_context_chars_jur
        } else {
            retrieval_params.max_context_chars_no_jur
        };
        let prompt_overhead_chars = 1800; // System prompt + question + history + formatting
        base.saturating_sub(prompt_overhead_chars)
    };
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

    // Inject case background from case.case_context (Feature B)
    {
        let s = state.lock().await;
        if let Some(ref cid) = case_id {
            if let Some(case) = s.cases.iter().find(|c| c.id == *cid) {
                if let Some(ref ctx) = case.case_context {
                    context.push_str(&format!("CASE BACKGROUND: {}\n\n", ctx));
                }
            }
        }

        // Inject identified entities (Feature D)
        let case_files: std::collections::HashSet<String> = s
            .file_registry
            .values()
            .filter(|fi| fi.case_id.as_ref() == case_id.as_ref())
            .map(|fi| fi.file_name.clone())
            .collect();
        let relevant_entities: Vec<&EntityEntry> = s
            .entity_registry
            .iter()
            .filter(|e| case_files.contains(&e.source_file))
            .collect();
        if !relevant_entities.is_empty() {
            let entity_lines: Vec<String> = relevant_entities
                .iter()
                .map(|e| {
                    if let Some(ref role) = e.role {
                        format!("{}: {} [{}]", role, e.name, e.source_file)
                    } else {
                        format!("{} [{}]", e.name, e.source_file)
                    }
                })
                .collect();
            context.push_str(&format!("IDENTIFIED PARTIES:\n{}\n\n", entity_lines.join("\n")));
        }

        // Inject condensed fact sheet (Feature C)
        let mut fact_parts = Vec::new();
        for fi in s.file_registry.values() {
            if fi.case_id.as_ref() == case_id.as_ref() {
                if let Some(ref fs) = fi.fact_sheet {
                    if !fs.parties.is_empty() {
                        fact_parts.push(format!("Parties ({}): {}", fi.file_name, fs.parties.join(", ")));
                    }
                    if !fs.amounts.is_empty() {
                        fact_parts.push(format!("Amounts ({}): {}", fi.file_name, fs.amounts.join(", ")));
                    }
                    if !fs.dates.is_empty() {
                        fact_parts.push(format!("Key dates ({}): {}", fi.file_name, fs.dates.join(", ")));
                    }
                }
            }
        }
        if !fact_parts.is_empty() {
            context.push_str(&format!("KEY FACTS:\n{}\n\n", fact_parts.join("\n")));
        }
    }

    // Calculate remaining budget after injected context
    let remaining_budget = max_context_chars.saturating_sub(context.len());

    // Reserve ~20% of remaining budget for neighbor expansion (non-Quick modes)
    let neighbor_budget = if settings.inference_mode == crate::state::InferenceMode::Quick {
        0
    } else {
        remaining_budget / 5
    };
    let primary_budget = remaining_budget.saturating_sub(neighbor_budget);

    // Allocate primary budget proportionally to chunk scores
    let scores: Vec<f32> = results.iter().map(|(s, _)| *s).collect();
    let chunk_budgets = allocate_chunk_budgets(&scores, primary_budget, 200);

    // Build per-chunk formatted strings with budget-aware truncation
    let formatted_chunks: Vec<(DocumentRole, String)> = results
        .iter()
        .enumerate()
        .map(|(i, (_, meta))| {
            let section = extract_section_header(&meta.text);
            let source_label = match section {
                Some(ref hdr) => format!(
                    "[Source: {}, p. {}, Section: {}]",
                    meta.file_name, meta.page_number, hdr
                ),
                None => format!(
                    "[Source: {}, p. {}]",
                    meta.file_name, meta.page_number
                ),
            };
            let header = format!("SOURCE {} — {}\n", i + 1, source_label);
            let footer = "\n---";
            let overhead = header.len() + footer.len();
            let text_budget = chunk_budgets.get(i).copied().unwrap_or(200).saturating_sub(overhead);
            let truncated_text = truncate_to_budget(&meta.text, text_budget);
            (meta.role.clone(), format!("{}{}{}", header, truncated_text, footer))
        })
        .collect();

    // Group chunks by document role for role-aware context assembly
    let mut client_parts = Vec::new();
    let mut authority_parts = Vec::new();
    let mut evidence_parts = Vec::new();
    let mut reference_parts = Vec::new();
    for (role, text) in &formatted_chunks {
        match role {
            DocumentRole::ClientDocument => client_parts.push(text.as_str()),
            DocumentRole::LegalAuthority => authority_parts.push(text.as_str()),
            DocumentRole::Evidence => evidence_parts.push(text.as_str()),
            DocumentRole::Reference => reference_parts.push(text.as_str()),
        }
    }

    let mut context_parts: Vec<String> = Vec::new();
    if !client_parts.is_empty() {
        context_parts.push("--- CLIENT DOCUMENTS ---".to_string());
        for p in &client_parts { context_parts.push(p.to_string()); }
    }
    if !authority_parts.is_empty() {
        context_parts.push("--- LEGAL AUTHORITY ---".to_string());
        for p in &authority_parts { context_parts.push(p.to_string()); }
    }
    if !evidence_parts.is_empty() {
        context_parts.push("--- EVIDENCE ---".to_string());
        for p in &evidence_parts { context_parts.push(p.to_string()); }
    }
    if !reference_parts.is_empty() {
        context_parts.push("--- REFERENCE ---".to_string());
        for p in &reference_parts { context_parts.push(p.to_string()); }
    }

    // Add primary context chunks with budget enforcement
    for part in &context_parts {
        let addition = if context.is_empty() { part.len() } else { part.len() + separator.len() };
        if context.len() + addition > context.len() + primary_budget.saturating_sub(context.len()) + separator.len() {
            // Double-check against absolute max
            if context.len() + addition > max_context_chars {
                log::warn!(
                    "Context budget exhausted at {} chars; skipping remaining chunks.",
                    context.len()
                );
                break;
            }
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
    let model_cache_for_retry = Arc::clone(&model_cache);
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
        inference_params,
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
    // Normalize scores to 0–1 so the frontend can display meaningful strength
    // labels (raw RRF scores are tiny, ~0.01–0.03, which breaks threshold UIs).
    let max_score = results.iter().map(|(s, _)| *s).fold(0.0f32, f32::max);
    let citations: Vec<Citation> = results
        .iter()
        .map(|(score, meta)| Citation {
            file_name: meta.file_name.clone(),
            file_path: meta.file_path.clone(),
            page_number: meta.page_number,
            excerpt: RagState::best_excerpt(&meta.text, &question),
            summary: RagState::summarize_chunk(&meta.text),
            score: if max_score > 0.0 { score / max_score } else { 0.0 },
        })
        .collect();

    // Helper: count assertion violations by type
    fn count_violations(assertions: &[crate::assertions::AssertionResult]) -> (usize, usize) {
        let hallucinations = assertions.iter().filter(|a| {
            !a.passed && matches!(a.assertion_type, crate::assertions::AssertionType::Hallucination)
        }).count();
        let fabrications = assertions.iter().filter(|a| {
            !a.passed && matches!(a.assertion_type, crate::assertions::AssertionType::FabricatedEntity)
        }).count();
        (hallucinations, fabrications)
    }

    // Run answer quality assertions
    let known_files: Vec<&str> = results.iter().map(|(_, m)| m.file_name.as_str()).collect();
    let chunk_texts: Vec<&str> = results.iter().map(|(_, m)| m.text.as_str()).collect();
    let mut assertions = crate::assertions::check_citations(&answer, Some(&known_files));
    assertions.extend(crate::assertions::check_no_hallucination(&answer, &chunk_texts));
    assertions.extend(crate::assertions::check_fabricated_entities(&answer, &chunk_texts));

    // ── Layer 4: Quality retry loop ──────────────────────────────────────────
    // If hallucination or fabrication detected, retry ONCE with a tighter prompt
    // and lower temperature. Use the retry only if it has fewer violations.
    let (orig_hall, orig_fab) = count_violations(&assertions);
    let has_hallucination = orig_hall > 0;
    let has_fabrication = orig_fab > 0;

    let (mut final_answer, mut final_not_found, mut final_assertions) =
        (answer.clone(), not_found, assertions);

    // Compute confidence BEFORE retry decision so low confidence can trigger a retry
    let chunk_strings_for_confidence: Vec<String> = chunk_texts.iter().map(|s| s.to_string()).collect();
    let answer_confidence = crate::assertions::compute_confidence(&answer, &chunk_strings_for_confidence);
    let should_retry = has_hallucination || has_fabrication || answer_confidence < 0.4;

    if should_retry {
        log::warn!(
            "Quality check failed (hallucinations: {}, fabrications: {}, confidence: {:.2}). Attempting retry.",
            orig_hall, orig_fab, answer_confidence
        );
        window
            .emit("query-status", serde_json::json!({"phase": "retrying"}))
            .ok();

        // Build retry prompt with stricter grounding instruction
        let retry_question = format!(
            "{}\n\nIMPORTANT: Your previous response contained claims not found in the \
            provided documents. Answer again using ONLY facts explicitly stated in the \
            context above. If a fact is not in the context, do not include it.",
            question
        );

        // Retry with halved temperature (min 0.05)
        let retry_temp = (saved_temperature / 2.0).max(0.05);
        let retry_params = pipeline::InferenceParams {
            max_new_tokens: saved_max_new_tokens,
            temperature: retry_temp,
            system_prompt_suffix: saved_system_prompt_suffix,
            system_prompt_override: saved_system_prompt_override,
            timeout_secs: saved_timeout_secs,
            is_quick: saved_is_quick,
        };

        // Reuse the model_cache Arc cloned before the first ask_saul call.
        let model_cache_retry = model_cache_for_retry;
        let window_retry = window.clone();
        let retry_answer = pipeline::ask_saul(
            &retry_question,
            &context,
            &history,
            &model_dir,
            model_cache_retry,
            move |tok| {
                window_retry.emit("query-token", tok.as_str()).ok();
            },
            resolved_jurisdiction.as_ref(),
            retry_params,
        )
        .await;

        match retry_answer {
            Ok(retry_text) => {
                // Re-run assertions on retry response
                let mut retry_assertions =
                    crate::assertions::check_citations(&retry_text, Some(&known_files));
                retry_assertions.extend(crate::assertions::check_no_hallucination(
                    &retry_text,
                    &chunk_texts,
                ));
                retry_assertions.extend(crate::assertions::check_fabricated_entities(
                    &retry_text,
                    &chunk_texts,
                ));

                let (retry_hall, retry_fab) = count_violations(&retry_assertions);
                let orig_total = orig_hall + orig_fab;
                let retry_total = retry_hall + retry_fab;

                if retry_total < orig_total {
                    log::info!(
                        "Retry improved quality: violations {} -> {}. Using retry response.",
                        orig_total, retry_total
                    );
                    let retry_lower = retry_text.to_lowercase();
                    let retry_not_found = retry_text.is_empty()
                        || retry_lower.contains("i could not find information")
                        || retry_lower.contains("could not find information about this")
                        || retry_lower.contains("documents do not contain");
                    final_answer = retry_text;
                    final_not_found = retry_not_found;
                    final_assertions = retry_assertions;
                } else {
                    log::info!(
                        "Retry did not improve quality: violations {} -> {}. Keeping original.",
                        orig_total, retry_total
                    );
                }
            }
            Err(e) => {
                log::warn!("Retry inference failed: {}. Keeping original response.", e);
            }
        }
    }

    // ── Layer 5: Output cleanup pipeline ─────────────────────────────────────
    // Apply cleanup in order: collapse repetitions, strip excessive hedging,
    // repair malformed citations, then strip ungrounded claims.
    final_answer = collapse_repetitions(&final_answer);
    final_answer = strip_excessive_hedging(&final_answer);
    final_answer = repair_citations(&final_answer);

    // Last-resort cleanup: remove individual sentences with ungrounded proper
    // nouns or numbers, rather than failing the whole response.
    let chunk_strings: Vec<String> = chunk_texts.iter().map(|s| s.to_string()).collect();
    final_answer = crate::assertions::strip_ungrounded_claims(&final_answer, &chunk_strings);

    // ── Layer 6: Fabrication failsafe ────────────────────────────────────────
    // If fabrication STILL present after retry + stripping, replace with safe error.
    let (_, still_fabricated) = count_violations(&final_assertions);
    if still_fabricated > 0 {
        log::warn!("Fabricated entity still present after retry — replacing with safe response.");
        final_answer = "I found some relevant excerpts in your documents, but I was unable to produce \
            a fully grounded answer. Some details may not be supported by your documents.\n\n\
            Please try rephrasing your question more specifically, or review the source \
            excerpts below to find the information you need.".to_string();
        final_not_found = true;
    }

    // ── Confidence scoring ───────────────────────────────────────────────────
    let confidence = crate::assertions::compute_confidence(&final_answer, &chunk_strings);

    Ok(QueryResult {
        answer: final_answer,
        citations,
        not_found: final_not_found,
        assertions: final_assertions,
        confidence: Some(confidence),
    })
}

// ── Output Cleanup Helpers ────────────────────────────────────────────────────

/// Extract a recognizable section header from the first line of a chunk.
/// Detects ALL-CAPS headers, "Section N"/"Article N" patterns, and numbered headings.
fn extract_section_header(text: &str) -> Option<String> {
    let first_line = text.lines().next()?.trim();
    // Check if first line is a section header
    if first_line.len() > 3 && first_line.len() < 80 {
        // ALL-CAPS header
        if first_line
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase())
            && first_line.chars().filter(|c| c.is_alphabetic()).count() > 3
        {
            return Some(first_line.to_string());
        }
        // "Section N" or "Article N" header
        if first_line.starts_with("Section ")
            || first_line.starts_with("SECTION ")
            || first_line.starts_with("Article ")
            || first_line.starts_with("ARTICLE ")
        {
            return Some(first_line.to_string());
        }
        // Numbered header: "1.1", "2.3.1"
        if first_line
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_digit())
            && first_line.contains('.')
        {
            return Some(first_line.to_string());
        }
    }
    None
}

/// Detect and remove excessive hedging when >50% of sentences hedge.
fn strip_excessive_hedging(answer: &str) -> String {
    let hedging_phrases = [
        "it appears that",
        "it seems that",
        "it may be",
        "it might be",
        "it could be",
        "possibly",
        "perhaps",
        "it is unclear",
        "it is uncertain",
        "this may not",
        "this might not",
    ];

    let sentences: Vec<&str> = answer.split(". ").collect();
    let hedge_count = sentences
        .iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            hedging_phrases.iter().any(|h| lower.contains(h))
        })
        .count();

    // If >50% of sentences hedge, strip the hedging sentences (keep only grounded ones)
    if sentences.len() > 2 && hedge_count as f64 / sentences.len() as f64 > 0.5 {
        let grounded: Vec<&str> = sentences
            .iter()
            .filter(|s| {
                let lower = s.to_lowercase();
                !hedging_phrases.iter().any(|h| lower.contains(h))
            })
            .copied()
            .collect();
        if !grounded.is_empty() {
            return grounded.join(". ") + ".";
        }
    }
    answer.to_string()
}

/// Collapse repeated sentences in the output.
fn collapse_repetitions(answer: &str) -> String {
    let sentences: Vec<&str> = answer.split(". ").collect();
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();

    for sentence in &sentences {
        let normalized = sentence.trim().to_lowercase();
        if normalized.len() < 10 || seen.insert(normalized) {
            unique.push(*sentence);
        }
    }

    unique.join(". ")
}

/// Fix incomplete or malformed citations in the output.
fn repair_citations(answer: &str) -> String {
    let mut result = answer.to_string();

    // Fix unclosed brackets: "[file, p. 3" → "[file, p. 3]"
    let re = regex::Regex::new(r"\[([^\]]+,\s*p\.\s*\d+)(?!\])").unwrap();
    result = re.replace_all(&result, "[$1]").to_string();

    // Fix empty citations: "[, p. ]" → remove entirely
    let empty_re = regex::Regex::new(r"\[\s*,?\s*p\.\s*\]").unwrap();
    result = empty_re.replace_all(&result, "").to_string();

    // Fix double brackets: "[[file, p. 3]]" → "[file, p. 3]"
    result = result.replace("[[", "[").replace("]]", "]");

    result
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
    let item_ids: std::collections::HashSet<String> = s.doc_chunk_ids.get(&file_id).cloned().unwrap_or_default().into_iter().collect();
    for id in &item_ids {
        s.chunk_registry.remove(id);
    }
    s.embedded_chunks.retain(|e| !item_ids.contains(&e.id));
    s.invalidate_bm25_cache();
    s.doc_chunk_ids.remove(&file_id);
    s.file_registry.remove(&file_id);
    s.save_chunks().await;
    Ok(())
}

// ── Document Viewer ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_file_data(
    file_path: String,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<String, String> {
    // Validate the requested path is in the file registry.
    // Compare both raw paths and canonicalized paths to handle macOS symlinks
    // (e.g. /var → /private/var) without failing on missing files.
    {
        let s = state.lock().await;
        let registered = s.file_registry.values().any(|f| {
            if f.file_path == file_path {
                return true;
            }
            // Fall back to canonicalize for symlink resolution
            match (std::fs::canonicalize(&file_path), std::fs::canonicalize(&f.file_path)) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            }
        });
        if !registered {
            return Err("Access denied: file not in registry".to_string());
        }
    }
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
            let item_ids: std::collections::HashSet<String> = s.doc_chunk_ids.get(file_id).cloned().unwrap_or_default().into_iter().collect();
            for id in &item_ids {
                s.chunk_registry.remove(id);
            }
            s.embedded_chunks.retain(|e| !item_ids.contains(&e.id));
            s.doc_chunk_ids.remove(file_id);
            s.file_registry.remove(file_id);
        }
        s.invalidate_bm25_cache();
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

// ── Document Roles ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn set_document_role(
    file_id: String,
    role: DocumentRole,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(file_info) = s.file_registry.get_mut(&file_id) {
        file_info.role = role.clone();
    }
    for chunk in &mut s.embedded_chunks {
        if chunk.meta.document_id == file_id {
            chunk.meta.role = role.clone();
        }
    }
    for meta in s.chunk_registry.values_mut() {
        if meta.document_id == file_id {
            meta.role = role.clone();
        }
    }
    s.save_chunks().await;
    s.save_file_registry().await;
    Ok(())
}

// ── Case Context ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn save_case_context(
    case_id: String,
    context: String,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<(), String> {
    let mut s = state.lock().await;
    if let Some(case) = s.cases.iter_mut().find(|c| c.id == case_id) {
        case.case_context = if context.trim().is_empty() {
            None
        } else {
            Some(context)
        };
    }
    s.save_cases().await;
    Ok(())
}

// ── Entity Registry ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_entities(
    case_id: Option<String>,
    state: tauri::State<'_, Arc<AsyncMutex<RagState>>>,
) -> Result<Vec<EntityEntry>, String> {
    let s = state.lock().await;
    let case_files: std::collections::HashSet<String> = s
        .file_registry
        .values()
        .filter(|fi| fi.case_id.as_ref() == case_id.as_ref())
        .map(|fi| fi.file_name.clone())
        .collect();
    Ok(s.entity_registry
        .iter()
        .filter(|e| case_files.contains(&e.source_file))
        .cloned()
        .collect())
}

// ── Fact Sheet Extraction (regex-based, no LLM) ──────────────────────────────

fn extract_fact_sheet(pages: &[crate::state::DocumentPage]) -> FactSheet {
    let full_text = pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Extract parties — "between X and Y", role keywords
    let mut parties = Vec::new();
    if let Ok(party_re) = regex::Regex::new(
        r"(?i)(?:between|by and between)\s+([A-Z][a-zA-Z\s,\.]+?)(?:\s+\(|,\s+a\s+|,\s+an\s+|\s+and\s+)",
    ) {
        for cap in party_re.captures_iter(&full_text) {
            parties.push(cap[1].trim().to_string());
        }
    }
    if let Ok(role_re) = regex::Regex::new(
        r"(?i)(landlord|tenant|buyer|seller|employer|employee|licensor|licensee|borrower|lender|plaintiff|defendant|lessor|lessee)\s*[:]\s*([A-Z][a-zA-Z\s\.]+)",
    ) {
        for cap in role_re.captures_iter(&full_text) {
            let role = cap[1].to_string();
            let name = cap[2].trim().to_string();
            parties.push(format!("{}: {}", role, name));
        }
    }
    parties.dedup();

    // Extract dates
    let mut dates = Vec::new();
    if let Ok(date_re) = regex::Regex::new(
        r"\b(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}\b|\b\d{1,2}/\d{1,2}/\d{2,4}\b|\b\d{4}-\d{2}-\d{2}\b",
    ) {
        for m in date_re.find_iter(&full_text) {
            dates.push(m.as_str().to_string());
        }
    }
    dates.dedup();

    // Extract dollar amounts
    let mut amounts = Vec::new();
    if let Ok(amount_re) = regex::Regex::new(r"\$[\d,]+(?:\.\d{2})?") {
        for m in amount_re.find_iter(&full_text) {
            amounts.push(m.as_str().to_string());
        }
    }
    amounts.dedup();

    // Extract key clauses — section headers
    let mut key_clauses = Vec::new();
    if let Ok(clause_re) = regex::Regex::new(
        r"(?m)^(?:Section\s+\d+[.:]\s*|Article\s+[IVX\d]+[.:]\s*|ARTICLE\s+[IVX\d]+[.:]\s*|\d+\.\d+\s+)(.{5,80})$",
    ) {
        for cap in clause_re.captures_iter(&full_text) {
            key_clauses.push(cap[1].trim().to_string());
        }
    }
    // Also detect ALL-CAPS section headers
    for line in full_text.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 3
            && trimmed.len() < 80
            && trimmed
                .chars()
                .filter(|c| c.is_alphabetic())
                .all(|c| c.is_uppercase())
            && trimmed.chars().filter(|c| c.is_alphabetic()).count() > 3
        {
            key_clauses.push(trimmed.to_string());
        }
    }
    key_clauses.dedup();
    key_clauses.truncate(15);

    // Build summary (first 200 chars of document)
    let summary = full_text.chars().take(200).collect::<String>().trim().to_string();

    FactSheet {
        parties,
        dates,
        amounts,
        key_clauses,
        summary,
    }
}

// ── Entity Extraction (regex-based, no LLM) ──────────────────────────────────

fn extract_entities(pages: &[crate::state::DocumentPage], file_name: &str) -> Vec<EntityEntry> {
    let full_text = pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut entities = Vec::new();

    let role_patterns: &[(&str, &str)] = &[
        (r"(?i)landlord\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Landlord"),
        (r"(?i)tenant\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Tenant"),
        (r"(?i)buyer\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Buyer"),
        (r"(?i)seller\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Seller"),
        (r"(?i)employer\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Employer"),
        (r"(?i)employee\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Employee"),
        (r"(?i)plaintiff\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Plaintiff"),
        (r"(?i)defendant\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Defendant"),
        (r"(?i)licensor\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Licensor"),
        (r"(?i)licensee\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Licensee"),
        (r"(?i)borrower\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Borrower"),
        (r"(?i)lender\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Lender"),
        (r"(?i)lessor\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Lessor"),
        (r"(?i)lessee\s*[:]\s*([A-Z][a-zA-Z\s\.]+?)(?:\n|,|\()", "Lessee"),
    ];

    for (pattern, role) in role_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            for cap in re.captures_iter(&full_text) {
                let name = cap[1].trim().to_string();
                if name.len() > 2 && name.len() < 60 {
                    entities.push(EntityEntry {
                        name: name.clone(),
                        role: Some(role.to_string()),
                        source_file: file_name.to_string(),
                        aliases: Vec::new(),
                    });
                }
            }
        }
    }

    // "between X and Y" patterns
    if let Ok(between_re) = regex::Regex::new(
        r"(?i)between\s+([A-Z][a-zA-Z\s\.]+?)\s+(?:\(.*?\)\s+)?and\s+([A-Z][a-zA-Z\s\.]+?)(?:\s*\(|\s*,|\s*\.)",
    ) {
        for cap in between_re.captures_iter(&full_text) {
            let name1 = cap[1].trim().to_string();
            let name2 = cap[2].trim().to_string();
            if name1.len() > 2 && name1.len() < 60 && !entities.iter().any(|e| e.name == name1) {
                entities.push(EntityEntry {
                    name: name1,
                    role: None,
                    source_file: file_name.to_string(),
                    aliases: Vec::new(),
                });
            }
            if name2.len() > 2 && name2.len() < 60 && !entities.iter().any(|e| e.name == name2) {
                entities.push(EntityEntry {
                    name: name2,
                    role: None,
                    source_file: file_name.to_string(),
                    aliases: Vec::new(),
                });
            }
        }
    }

    entities
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
            inference_mode: crate::state::InferenceMode::default(),
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
        // Fix 1: "1. The Employee shall receive..." must NOT be treated as orphan header.
        // Use longer text to avoid small-chunk merging (<100 bytes).
        let text = "Preamble text here with additional context to ensure this chunk exceeds the minimum size threshold for standalone chunks.\n\n1. The Employee shall receive five weeks of paid vacation per year, subject to the terms and conditions of the employment agreement herein.\n\n2. The Employee is entitled to full medical coverage including dental and vision benefits as outlined in the company benefits package.";
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
        // Use longer text to exceed 100-byte small-chunk merge threshold.
        let text =
            "Alpha sentence one with enough detail to exceed the minimum chunk size. Alpha sentence two continues with more substantive content here.\n\nBeta sentence one starts a new paragraph with different content entirely. Beta sentence two also has enough text to stand alone as a chunk.";
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
    fn format_history_capped_at_2_turns() {
        // Only the last 2 turns should appear in the formatted history
        let history: Vec<(String, String)> = (0..8)
            .map(|i| (format!("user{i}"), format!("assistant{i}")))
            .collect();
        let result = format_history(&history);
        // Turns 0-5 should be absent; turns 6-7 should be present
        assert!(!result.contains("user0"), "Turn 0 should be excluded");
        assert!(!result.contains("user5"), "Turn 5 should be excluded");
        assert!(result.contains("user6"), "Turn 6 should be included");
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
