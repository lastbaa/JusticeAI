use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── Shared Types (mirror of shared/src/types.ts) ────────────────────────────

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 150,
            top_k: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub llm_ready: bool,
    pub llm_size_gb: f32,
    pub download_required_gb: f32,
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

        // Load embedded chunks
        if let Ok(data) = tokio::fs::read(&self.chunks_path()).await {
            if let Ok(chunks) = serde_json::from_slice::<Vec<EmbeddedChunkEntry>>(&data) {
                for entry in chunks {
                    self.chunk_registry.insert(entry.id.clone(), entry.meta.clone());
                    let ids = self.doc_chunk_ids.entry(entry.meta.document_id.clone()).or_default();
                    if !ids.contains(&entry.id) {
                        ids.push(entry.id.clone());
                    }
                    self.embedded_chunks.push(entry);
                }
                // Rebuild file registry
                self.rebuild_file_registry();
            }
        }
    }

    fn rebuild_file_registry(&mut self) {
        let mut doc_map: HashMap<String, (ChunkMetadata, usize, u32)> = HashMap::new();
        for chunk in &self.embedded_chunks {
            let meta = &chunk.meta;
            let entry = doc_map.entry(meta.document_id.clone()).or_insert((meta.clone(), 0, 0));
            entry.1 += 1;
            if meta.page_number > entry.2 {
                entry.2 = meta.page_number;
            }
        }
        for (doc_id, (meta, count, max_page)) in doc_map {
            if !self.file_registry.contains_key(&doc_id) {
                self.file_registry.insert(doc_id.clone(), FileInfo {
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
                });
            }
        }
    }

    pub async fn save_chunks(&self) {
        if let Ok(data) = serde_json::to_vec(&self.embedded_chunks) {
            tokio::fs::write(&self.chunks_path(), data).await.ok();
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

    /// Cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Get all text chunks for a specific file+page from chunk registry
    pub fn get_page_text(&self, file_path: &str, page_number: u32) -> String {
        let mut texts: Vec<&str> = Vec::new();
        for chunk in self.chunk_registry.values() {
            if chunk.file_path == file_path && chunk.page_number == page_number {
                texts.push(&chunk.text);
            }
        }
        texts.join(" ")
    }

    /// Extract best excerpt sentence from chunk text relevant to the query
    pub fn best_excerpt(text: &str, query: &str) -> String {
        let sentences: Vec<&str> = text
            .split(|c: char| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| s.len() > 20)
            .collect();

        if sentences.is_empty() {
            let end = text.char_indices().nth(280).map(|(i, _)| i).unwrap_or(text.len());
            return text[..end].to_string();
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
            let hits = words.iter()
                .filter(|w| query_words.contains(&w.to_lowercase()))
                .count();
            let score = hits as f32 / (words.len() as f32).sqrt();
            if score > best_score {
                best_score = score;
                best = sentence;
            }
        }

        let end = best.char_indices().nth(320).map(|(i, _)| i).unwrap_or(best.len());
        if best.len() > 320 {
            format!("{}…", &best[..end])
        } else {
            best.to_string()
        }
    }
}
