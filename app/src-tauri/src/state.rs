use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Shared flag: set to `true` by the `set_can_close` command before calling
/// `appWindow.close()` from JS. The `on_window_event` handler checks this to
/// distinguish a user-confirmed close from a raw OS close request.
pub struct CloseAllowed(pub AtomicBool);

// ── Shared Types (mirror of shared/src/types.ts) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JurisdictionLevel {
    Federal,
    State,
    County,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Jurisdiction {
    pub level: JurisdictionLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub county: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Jurisdiction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPage {
    pub page_number: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub total_pages: u32,
    pub word_count: u32,
    pub loaded_at: u64,
    pub chunk_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_jurisdiction: Option<Jurisdiction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub file_name: String,
    pub file_path: String,
    pub page_number: u32,
    pub excerpt: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub not_found: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<crate::assertions::AssertionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Jurisdiction>,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 150,
            top_k: 6,
            theme: default_theme(),
            jurisdiction: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub llm_ready: bool,
    pub llm_size_gb: f32,
    pub download_required_gb: f32,
    pub ocr_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_streaming: Option<bool>,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_found: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub name: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<OllamaModel>,
    pub has_llm_model: bool,
    pub has_embed_model: bool,
    pub llm_model_name: String,
    pub embed_model_name: String,
}

// ── RAG State ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkMetadata {
    pub id: String,
    pub document_id: String,
    pub file_name: String,
    pub file_path: String,
    pub page_number: u32,
    pub chunk_index: usize,
    pub text: String,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedChunkEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub meta: ChunkMetadata,
}

pub struct RagState {
    pub file_registry: HashMap<String, FileInfo>,
    pub chunk_registry: HashMap<String, ChunkMetadata>,
    pub doc_chunk_ids: HashMap<String, Vec<String>>,
    pub embedded_chunks: Vec<EmbeddedChunkEntry>,
    pub data_dir: PathBuf,
    pub model_dir: PathBuf,
    pub settings: AppSettings,
    pub sessions: Vec<ChatSession>,
    pub cases: Vec<Case>,
    /// Which embedding model produced the stored vectors. Empty string = pre-BGE (stale).
    pub embed_model: String,
    /// Cached llama model — loaded once on first query, reused thereafter.
    pub llama_model: Arc<Mutex<Option<llama_cpp_2::model::LlamaModel>>>,
}

impl RagState {
    pub fn new(data_dir: PathBuf) -> Self {
        let model_dir = data_dir.join("models");
        Self {
            file_registry: HashMap::new(),
            chunk_registry: HashMap::new(),
            doc_chunk_ids: HashMap::new(),
            embedded_chunks: Vec::new(),
            model_dir,
            data_dir,
            settings: AppSettings::default(),
            sessions: Vec::new(),
            cases: Vec::new(),
            embed_model: String::new(),
            llama_model: Arc::new(Mutex::new(None)),
        }
    }

    fn chunks_path(&self) -> PathBuf {
        self.data_dir.join("chunks.json")
    }

    fn settings_path(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    fn sessions_path(&self) -> PathBuf {
        self.data_dir.join("sessions.json")
    }

    fn embed_model_path(&self) -> PathBuf {
        self.data_dir.join("embed_model.json")
    }

    fn cases_path(&self) -> PathBuf {
        self.data_dir.join("cases.json")
    }

    fn file_registry_path(&self) -> PathBuf {
        self.data_dir.join("file_registry.json")
    }

    pub async fn load_from_disk(&mut self) {
        // Load settings
        if let Ok(data) = tokio::fs::read(&self.settings_path()).await {
            if let Ok(s) = serde_json::from_slice::<AppSettings>(&data) {
                self.settings = s;
            }
        }

        // Load sessions
        if let Ok(data) = tokio::fs::read(&self.sessions_path()).await {
            if let Ok(s) = serde_json::from_slice::<Vec<ChatSession>>(&data) {
                self.sessions = s;
            }
        }

        // Load cases
        if let Ok(data) = tokio::fs::read(&self.cases_path()).await {
            if let Ok(c) = serde_json::from_slice::<Vec<Case>>(&data) {
                self.cases = c;
            }
        }

        // Load embed model version (empty = pre-BGE, triggers migration)
        if let Ok(data) = tokio::fs::read(&self.embed_model_path()).await {
            if let Ok(s) = serde_json::from_slice::<String>(&data) {
                self.embed_model = s;
            }
        }

        // Load embedded chunks
        if let Ok(data) = tokio::fs::read(&self.chunks_path()).await {
            if let Ok(chunks) = serde_json::from_slice::<Vec<EmbeddedChunkEntry>>(&data) {
                for entry in chunks {
                    self.chunk_registry
                        .insert(entry.id.clone(), entry.meta.clone());
                    let ids = self
                        .doc_chunk_ids
                        .entry(entry.meta.document_id.clone())
                        .or_default();
                    if !ids.contains(&entry.id) {
                        ids.push(entry.id.clone());
                    }
                    self.embedded_chunks.push(entry);
                }
                // Rebuild file registry
                self.rebuild_file_registry();
            }
        }

        // Restore case_id assignments from saved file registry
        if let Ok(data) = tokio::fs::read(&self.file_registry_path()).await {
            if let Ok(saved) = serde_json::from_slice::<HashMap<String, FileInfo>>(&data) {
                for (id, saved_info) in saved {
                    if let Some(entry) = self.file_registry.get_mut(&id) {
                        entry.case_id = saved_info.case_id;
                        entry.detected_jurisdiction = saved_info.detected_jurisdiction;
                    }
                }
            }
        }
    }

    fn rebuild_file_registry(&mut self) {
        let mut doc_map: HashMap<String, (ChunkMetadata, usize, u32)> = HashMap::new();
        for chunk in &self.embedded_chunks {
            let meta = &chunk.meta;
            let entry = doc_map
                .entry(meta.document_id.clone())
                .or_insert((meta.clone(), 0, 0));
            entry.1 += 1;
            if meta.page_number > entry.2 {
                entry.2 = meta.page_number;
            }
        }
        for (doc_id, (meta, count, max_page)) in doc_map {
            if !self.file_registry.contains_key(&doc_id) {
                self.file_registry.insert(
                    doc_id.clone(),
                    FileInfo {
                        id: doc_id,
                        file_name: meta.file_name,
                        file_path: meta.file_path,
                        total_pages: max_page,
                        word_count: 0,
                        loaded_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        chunk_count: count,
                        case_id: None,
                        detected_jurisdiction: None,
                    },
                );
            }
        }
    }

    pub async fn save_chunks(&self) {
        if let Ok(data) = serde_json::to_vec(&self.embedded_chunks) {
            tokio::fs::write(&self.chunks_path(), data).await.ok();
        }
    }

    pub async fn save_embed_model(&self) {
        if let Ok(data) = serde_json::to_vec(&self.embed_model) {
            tokio::fs::write(&self.embed_model_path(), data).await.ok();
        }
    }

    pub async fn save_settings(&self) {
        if let Ok(data) = serde_json::to_vec(&self.settings) {
            tokio::fs::write(&self.settings_path(), data).await.ok();
        }
    }

    pub async fn save_sessions(&self) {
        if let Ok(data) = serde_json::to_vec(&self.sessions) {
            tokio::fs::write(&self.sessions_path(), data).await.ok();
        }
    }

    pub async fn save_cases(&self) {
        if let Ok(data) = serde_json::to_vec(&self.cases) {
            tokio::fs::write(&self.cases_path(), data).await.ok();
        }
    }

    pub async fn save_file_registry(&self) {
        if let Ok(data) = serde_json::to_vec(&self.file_registry) {
            tokio::fs::write(&self.file_registry_path(), data).await.ok();
        }
    }

    /// Cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 || !norm_a.is_finite() || !norm_b.is_finite() {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Get all text chunks for a specific file+page from chunk registry,
    /// sorted by chunk_index and with overlapping prefix text deduplicated.
    pub fn get_page_text(&self, file_path: &str, page_number: u32) -> String {
        let mut chunks: Vec<&ChunkMetadata> = self
            .chunk_registry
            .values()
            .filter(|c| c.file_path == file_path && c.page_number == page_number)
            .collect();
        chunks.sort_by_key(|c| c.chunk_index);

        // Join chunks, stripping overlap: if the end of result matches the
        // start of the next chunk, skip the duplicated prefix.
        let mut result = String::new();
        for chunk in &chunks {
            let text = chunk.text.trim();
            if text.is_empty() {
                continue;
            }
            if result.is_empty() {
                result.push_str(text);
            } else {
                // Find longest suffix of result that equals a prefix of text.
                // Cap search at 80 chars AND at text.len()-1 so skip never equals text.len().
                let overlap_max = result.len().min(80).min(text.len().saturating_sub(1));
                let mut skip = 0;
                for n in (1..=overlap_max).rev() {
                    if result.ends_with(&text[..n]) {
                        skip = n;
                        break;
                    }
                }
                result.push(' ');
                result.push_str(&text[skip..]);
            }
        }
        result
    }

    /// Extract best excerpt sentence from chunk text relevant to the query.
    /// Strips private-use-area codepoints and control chars so the UI never
    /// displays "encrypted"-looking characters from badly-encoded PDFs.
    pub fn best_excerpt(text: &str, query: &str) -> String {
        // Sanitize first — PUA chars (U+E000–F8FF) and control chars from
        // lopdf/Identity-H font encoding look like garbage in every UI component
        // that renders the excerpt (SourceCard, ContextPanel, DocumentViewer).
        let sanitized: String = text
            .chars()
            .filter(|&c| {
                let code = c as u32;
                c == '\n'
                    || c == '\t'
                    || (!c.is_control() && !(0xE000..=0xF8FF).contains(&code) && code < 0xFFF0)
            })
            .collect();

        let sentences: Vec<&str> = sanitized
            .split(|c: char| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| s.len() > 20)
            .collect();

        if sentences.is_empty() {
            let end = sanitized
                .char_indices()
                .nth(280)
                .map(|(i, _)| i)
                .unwrap_or(sanitized.len());
            return sanitized[..end].to_string();
        }

        let query_words: std::collections::HashSet<String> = query
            .to_lowercase()
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.len() > 3)
            .map(|w| w.to_string())
            .collect();

        let mut best = sentences[0];
        let mut best_score = 0.0f32;

        for sentence in &sentences {
            let words: Vec<&str> = sentence.split(|c: char| !c.is_alphabetic()).collect();
            let hits = words
                .iter()
                .filter(|w| query_words.contains(&w.to_lowercase()))
                .count();
            let score = hits as f32 / (words.len() as f32).sqrt();
            if score > best_score {
                best_score = score;
                best = sentence;
            }
        }

        let end = best
            .char_indices()
            .nth(320)
            .map(|(i, _)| i)
            .unwrap_or(best.len());
        if best.len() > 320 {
            format!("{}…", &best[..end])
        } else {
            best.to_string()
        }
    }
}
