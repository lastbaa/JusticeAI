use crate::state::{
    AppSettings, ChatSession, ChunkMetadata, Citation, DocumentPage, EmbeddedChunkEntry, FileInfo,
    QueryResult, RagState,
};
use base64::Engine;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const SCORE_THRESHOLD: f32 = 0.35;
const MAX_CHUNKS_PER_PAGE: usize = 2;

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

// ── File Dialogs ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_file_dialog(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let result = app
        .dialog()
        .file()
        .add_filter("Documents", &["pdf", "docx"])
        .blocking_pick_files();

    Ok(match result {
        Some(files) => files
            .into_iter()
            .filter_map(|f| match f {
                FilePath::Path(p) => Some(p.to_string_lossy().to_string()),
                _ => None,
            })
            .collect(),
        None => vec![],
    })
}

#[tauri::command]
pub async fn open_folder_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let result = app.dialog().file().blocking_pick_folder();

    Ok(match result {
        Some(FilePath::Path(p)) => Some(p.to_string_lossy().to_string()),
        _ => None,
    })
}

// ── File Loading ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_files(
    file_paths: Vec<String>,
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<Vec<FileInfo>, String> {
    let settings = {
        let s = state.lock().await;
        s.settings.clone()
    };

    // Expand directories to files
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

    for file_path in expanded {
        match process_file(&file_path, &settings, &state).await {
            Ok(info) => results.push(info),
            Err(e) => log::error!("Failed to load {}: {}", file_path, e),
        }
    }

    Ok(results)
}

async fn process_file(
    file_path: &str,
    settings: &AppSettings,
    state: &tauri::State<'_, Arc<Mutex<RagState>>>,
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

    // Chunk document
    let chunks = chunk_document("", "", "", &pages, settings);
    let mut item_ids: Vec<String> = Vec::new();

    for chunk in &chunks {
        match embed_text(&chunk.text, &settings.hf_token).await {
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

struct TempChunk {
    id: String,
    page_number: u32,
    chunk_index: usize,
    text: String,
    token_count: usize,
}

fn chunk_document(
    _doc_id: &str,
    _file_name: &str,
    _file_path: &str,
    pages: &[DocumentPage],
    settings: &AppSettings,
) -> Vec<TempChunk> {
    let mut chunks = Vec::new();
    let mut global_idx = 0usize;

    for page in pages {
        let text = &page.text;
        if text.trim().is_empty() {
            continue;
        }

        // Split into sentences (rough sentence boundary)
        let sentences: Vec<&str> = split_sentences(text);
        let mut current = String::new();
        let mut sentence_buf: Vec<&str> = Vec::new();

        let flush = |current: &str, global_idx: &mut usize, chunks: &mut Vec<TempChunk>, page_num: u32| {
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

                // Overlap: carry last N chars of sentences into next chunk
                let mut overlap = String::new();
                for s in sentence_buf.iter().rev() {
                    let candidate = format!("{} {}", s, overlap);
                    if candidate.len() > settings.chunk_overlap {
                        break;
                    }
                    overlap = candidate;
                }
                current = overlap.trim().to_string();
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
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if (chars[i] == '.' || chars[i] == '!' || chars[i] == '?')
            && i + 1 < len
            && chars[i + 1].is_whitespace()
        {
            // Find byte position of end
            let end = text
                .char_indices()
                .nth(i + 1)
                .map(|(b, _)| b)
                .unwrap_or(text.len());
            let s = text[start..end].trim();
            if !s.is_empty() {
                sentences.push(s);
            }
            // Skip whitespace
            let next_start = text
                .char_indices()
                .skip(i + 1)
                .find(|(_, c)| !c.is_whitespace())
                .map(|(b, _)| b)
                .unwrap_or(text.len());
            start = next_start;
            i += 1;
        }
        i += 1;
    }

    let remainder = text[start..].trim();
    if !remainder.is_empty() {
        sentences.push(remainder);
    }

    sentences
}

// ── Embedding via HuggingFace ────────────────────────────────────────────────

const EMBED_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";

pub async fn embed_text(text: &str, hf_token: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://api-inference.huggingface.co/pipeline/feature-extraction/{}",
        EMBED_MODEL
    );

    let body = serde_json::json!({
        "inputs": text,
        "options": { "wait_for_model": true }
    });

    let resp = client
        .post(&url)
        .bearer_auth(hf_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HF embed request error: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("HF embed ({status}): {err}"));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // HF feature-extraction returns [[f32]] or [f32] depending on model
    let embedding: Vec<f32> = match &data {
        serde_json::Value::Array(outer) if !outer.is_empty() => {
            match &outer[0] {
                serde_json::Value::Array(inner) => {
                    // [[f32, ...]] — take first row
                    inner.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect()
                }
                serde_json::Value::Number(_) => {
                    // [f32, ...] — flat array
                    outer.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect()
                }
                _ => return Err("Unexpected HF embedding format".to_string()),
            }
        }
        _ => return Err("HF returned no embedding data".to_string()),
    };

    if embedding.is_empty() {
        return Err("HF returned empty embedding".to_string());
    }

    Ok(embedding)
}

// ── HuggingFace LLM ───────────────────────────────────────────────────────────

async fn ask_saul(system_prompt: &str, user_question: &str, hf_token: &str) -> Result<String, String> {
    const HF_API_URL: &str =
        "https://api-inference.huggingface.co/models/Equall/Saul-7B-Instruct-v1/v1/chat/completions";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "Equall/Saul-7B-Instruct-v1",
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_question }
        ],
        "max_tokens": 1024
    });

    let resp = client
        .post(HF_API_URL)
        .bearer_auth(hf_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HuggingFace request error: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("HuggingFace API ({status}): {err}"));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    data["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No response content from HuggingFace".to_string())
}

// ── RAG Query ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn query(
    question: String,
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<QueryResult, String> {
    let settings = {
        let s = state.lock().await;
        s.settings.clone()
    };

    if settings.hf_token.trim().is_empty() {
        return Err(
            "HuggingFace token is not configured. Open Settings to add your token.".to_string(),
        );
    }

    // Embed the query
    let query_vec = embed_text(&question, &settings.hf_token).await?;

    // Search vector store
    let candidate_k = (settings.top_k * 3).min(30);
    let results = {
        let s = state.lock().await;
        let mut scored: Vec<(f32, ChunkMetadata)> = s
            .embedded_chunks
            .iter()
            .map(|entry| {
                let score = RagState::cosine_similarity(&query_vec, &entry.vector);
                (score, entry.meta.clone())
            })
            .filter(|(score, _)| *score >= SCORE_THRESHOLD)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(candidate_k);

        // Diversity: cap chunks per (file_path, page_number)
        let mut page_count: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        scored
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

    if results.is_empty() {
        return Ok(QueryResult {
            answer: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded.".to_string(),
            citations: vec![],
            not_found: true,
        });
    }

    // Build context
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

    let answer = ask_saul(&system_with_context, &question, &settings.hf_token).await?;

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
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<Vec<FileInfo>, String> {
    let s = state.lock().await;
    Ok(s.file_registry.values().cloned().collect())
}

#[tauri::command]
pub async fn remove_file(
    file_id: String,
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
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
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<String, String> {
    let s = state.lock().await;
    Ok(s.get_page_text(&file_path, page_number))
}

// ── Settings ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<AppSettings, String> {
    let s = state.lock().await;
    Ok(s.settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
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
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
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
        // Keep last 50 sessions
        s.sessions.truncate(50);
    }

    s.save_sessions().await;
    Ok(true)
}

#[tauri::command]
pub async fn get_sessions(
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<Vec<ChatSession>, String> {
    let s = state.lock().await;
    Ok(s.sessions.clone())
}

#[tauri::command]
pub async fn delete_session(
    session_id: String,
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<bool, String> {
    let mut s = state.lock().await;
    s.sessions.retain(|sess| sess.id != session_id);
    s.save_sessions().await;
    Ok(true)
}
