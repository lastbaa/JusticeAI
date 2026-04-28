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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum DocumentRole {
    #[default]
    ClientDocument,
    LegalAuthority,
    Evidence,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FactSheet {
    pub parties: Vec<String>,
    pub dates: Vec<String>,
    pub amounts: Vec<String>,
    pub key_clauses: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub source_file: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InferenceMode {
    Quick,
    #[default]
    Balanced,
    Extended,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_context: Option<String>,
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
    #[serde(default)]
    pub role: DocumentRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_sheet: Option<FactSheet>,
    // ── File Dimensions ────────────────────────────────────
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub file_name: String,
    pub file_path: String,
    pub page_number: u32,
    pub excerpt: String,
    #[serde(default)]
    pub summary: String,
    pub score: f32,
    #[serde(default)]
    pub start_char_offset: Option<usize>,
    #[serde(default)]
    pub end_char_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub not_found: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<crate::assertions::AssertionResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
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
    #[serde(default)]
    pub inference_mode: InferenceMode,
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
            inference_mode: InferenceMode::default(),
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
    pub upgrade_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_streaming: Option<bool>,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_found: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_assertions: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_greeting: Option<bool>,
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

// ── BM25 Cache ───────────────────────────────────────────────────────────────

/// Cached BM25 index — rebuilt only when chunks change.
/// Avoids O(corpus_size) tokenization + doc-frequency computation on every query.
pub struct CachedBm25 {
    /// Number of documents when this index was built.
    pub doc_count: usize,
    /// Pre-tokenized documents (lowercase words, filtered).
    pub doc_tokens: Vec<Vec<String>>,
    /// Document lengths (token count per doc).
    pub doc_lens: Vec<usize>,
    /// Average document length.
    pub avg_dl: f32,
    /// Document frequency per term.
    pub doc_freq: HashMap<String, usize>,
    /// Whether the cache is valid.
    pub valid: bool,
}

impl Default for CachedBm25 {
    fn default() -> Self {
        Self {
            doc_count: 0,
            doc_tokens: Vec::new(),
            doc_lens: Vec::new(),
            avg_dl: 1.0,
            doc_freq: HashMap::new(),
            valid: false,
        }
    }
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
    #[serde(default)]
    pub role: DocumentRole,
    #[serde(default)]
    pub start_char_offset: Option<usize>,
    #[serde(default)]
    pub end_char_offset: Option<usize>,
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
    /// Cached BM25 index — rebuilt only when chunks change.
    pub bm25_cache: CachedBm25,
    /// Extracted entities from all loaded documents.
    pub entity_registry: Vec<EntityEntry>,
    /// Maps file_path → content hash for incremental indexing (skip unchanged files).
    pub file_hashes: HashMap<String, String>,
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
            bm25_cache: CachedBm25::default(),
            entity_registry: Vec::new(),
            file_hashes: HashMap::new(),
        }
    }

    /// Mark the BM25 cache as stale. Must be called whenever chunks are
    /// added, removed, or replaced so the next query rebuilds the index.
    pub fn invalidate_bm25_cache(&mut self) {
        self.bm25_cache.valid = false;
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

    fn file_hashes_path(&self) -> PathBuf {
        self.data_dir.join("file_hashes.json")
    }

    pub fn get_file_hash(&self, path: &str) -> Option<&String> {
        self.file_hashes.get(path)
    }

    pub fn set_file_hash(&mut self, path: String, hash: String) {
        self.file_hashes.insert(path, hash);
    }

    pub fn remove_file_hash(&mut self, path: &str) {
        self.file_hashes.remove(path);
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

        // Load file hashes (incremental indexing cache)
        if let Ok(data) = tokio::fs::read(&self.file_hashes_path()).await {
            if let Ok(h) = serde_json::from_slice::<HashMap<String, String>>(&data) {
                self.file_hashes = h;
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
                        entry.role = saved_info.role;
                        entry.fact_sheet = saved_info.fact_sheet;
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
                        role: meta.role,  // Preserve role from saved chunk metadata
                        fact_sheet: None,
                        image_width: None,
                        image_height: None,
                    },
                );
            }
        }
    }

    pub async fn save_chunks(&self) {
        match serde_json::to_vec(&self.embedded_chunks) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.chunks_path(), data).await {
                    log::error!("Failed to write chunks.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize chunks: {e}"),
        }
    }

    pub async fn save_embed_model(&self) {
        match serde_json::to_vec(&self.embed_model) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.embed_model_path(), data).await {
                    log::error!("Failed to write embed_model.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize embed_model: {e}"),
        }
    }

    pub async fn save_settings(&self) {
        match serde_json::to_vec(&self.settings) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.settings_path(), data).await {
                    log::error!("Failed to write settings.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize settings: {e}"),
        }
    }

    pub async fn save_sessions(&self) {
        match serde_json::to_vec(&self.sessions) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.sessions_path(), data).await {
                    log::error!("Failed to write sessions.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize sessions: {e}"),
        }
    }

    pub async fn save_cases(&self) {
        match serde_json::to_vec(&self.cases) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.cases_path(), data).await {
                    log::error!("Failed to write cases.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize cases: {e}"),
        }
    }

    pub async fn save_file_registry(&self) {
        match serde_json::to_vec(&self.file_registry) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.file_registry_path(), data).await {
                    log::error!("Failed to write file_registry.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize file_registry: {e}"),
        }
    }

    pub async fn save_file_hashes(&self) {
        match serde_json::to_vec(&self.file_hashes) {
            Ok(data) => {
                if let Err(e) = tokio::fs::write(&self.file_hashes_path(), data).await {
                    log::error!("Failed to write file_hashes.json: {e}");
                }
            }
            Err(e) => log::error!("Failed to serialize file_hashes: {e}"),
        }
    }

    /// Compute cosine similarity between two vectors.
    /// Uses manual loop unrolling for better auto-vectorization and f64
    /// accumulators for numerical stability.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let n = a.len();

        // Process 4 elements at a time for auto-vectorization
        let chunks = n / 4;
        let remainder_start = chunks * 4;

        let mut dot = 0.0f64;
        let mut norm_a = 0.0f64;
        let mut norm_b = 0.0f64;

        // Unrolled loop — compiler will auto-vectorize this
        for i in 0..chunks {
            let base = i * 4;
            let a0 = a[base] as f64;
            let a1 = a[base + 1] as f64;
            let a2 = a[base + 2] as f64;
            let a3 = a[base + 3] as f64;
            let b0 = b[base] as f64;
            let b1 = b[base + 1] as f64;
            let b2 = b[base + 2] as f64;
            let b3 = b[base + 3] as f64;

            dot += a0 * b0 + a1 * b1 + a2 * b2 + a3 * b3;
            norm_a += a0 * a0 + a1 * a1 + a2 * a2 + a3 * a3;
            norm_b += b0 * b0 + b1 * b1 + b2 * b2 + b3 * b3;
        }

        // Handle remainder
        for i in remainder_start..n {
            let ai = a[i] as f64;
            let bi = b[i] as f64;
            dot += ai * bi;
            norm_a += ai * ai;
            norm_b += bi * bi;
        }

        let denom = (norm_a * norm_b).sqrt();
        if denom < 1e-10 { 0.0 } else { (dot / denom) as f32 }
    }

    /// Pre-compute L2 norms for a set of embedding slices.
    pub fn precompute_norms(embeddings: &[&[f32]]) -> Vec<f64> {
        embeddings.iter().map(|emb| {
            let sum: f64 = emb.iter().map(|&x| (x as f64) * (x as f64)).sum();
            sum.sqrt()
        }).collect()
    }

    /// Pre-compute L2 norms for owned embedding vectors.
    pub fn precompute_norms_owned(embeddings: &[Vec<f32>]) -> Vec<f64> {
        embeddings.iter().map(|emb| {
            let sum: f64 = emb.iter().map(|&x| (x as f64) * (x as f64)).sum();
            sum.sqrt()
        }).collect()
    }

    /// Cosine similarity with pre-computed norms (avoids redundant norm computation).
    pub fn cosine_similarity_with_norms(a: &[f32], b: &[f32], norm_a: f64, norm_b: f64) -> f32 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(&x, &y)| (x as f64) * (y as f64)).sum();
        let denom = norm_a * norm_b;
        if denom < 1e-10 { 0.0 } else { (dot / denom) as f32 }
    }

    /// Compute cosine similarity between a query vector and all corpus vectors.
    /// Pre-computes the query norm once. Returns a Vec of similarity values
    /// aligned with the corpus index.
    pub fn batch_cosine_similarity(query: &[f32], corpus: &[&[f32]]) -> Vec<f32> {
        let query_norm: f64 = query.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt();
        if query_norm < 1e-10 {
            return vec![0.0; corpus.len()];
        }

        corpus.iter().map(|doc| {
            let dot: f64 = query.iter().zip(doc.iter()).map(|(&q, &d)| (q as f64) * (d as f64)).sum();
            let doc_norm: f64 = doc.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt();
            if doc_norm < 1e-10 { 0.0 } else { (dot / (query_norm * doc_norm)) as f32 }
        }).collect()
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
        if !text.is_char_boundary(n) {
            continue;
        }
        if result.ends_with(&text[..n]) {
            skip = n;
            break;
        }
    }
    // Advance skip forward to the next char boundary if needed.
    while skip < text.len() && !text.is_char_boundary(skip) {
        skip += 1;
    }
    result.push(' ');
    result.push_str(&text[skip..]);
}
        }
        result
    }

    /// Generate a short summary describing what a chunk covers.
    /// Layer 0: structured data detection (tables, form fields).
    /// Layer 1: extract heading from the first few lines.
    /// Layer 2: keyword extraction fallback.
    pub fn summarize_chunk(text: &str) -> String {
        // Layer 0: Structured data detection
        // Tables: lines with 2+ tabs or 2+ pipe chars
        let table_lines = text.lines().filter(|l| l.matches('\t').count() >= 2 || l.matches('|').count() >= 2).count();
        if table_lines >= 2 {
            let _total_lines = text.lines().count();
            return format!("Table ({} rows): {}", table_lines,
                text.lines().next().unwrap_or("").trim().chars().take(60).collect::<String>());
        }

        // Field-value pairs: lines matching "Label: Value" pattern
        let field_lines = text.lines().filter(|l| {
            let parts: Vec<&str> = l.splitn(2, ':').collect();
            parts.len() == 2 && parts[0].trim().len() > 1 && parts[0].trim().len() < 40 && parts[1].trim().len() > 0
        }).count();
        if field_lines >= 3 {
            return format!("Form data ({} fields): {}", field_lines,
                text.lines().take(2).map(|l| l.trim()).collect::<Vec<_>>().join(", "));
        }

        let lines: Vec<&str> = text.lines().take(3).collect();

        // Layer 1 — heading extraction
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.len() < 3 || trimmed.len() > 60 {
                continue;
            }
            // ALL-CAPS heading (no trailing period)
            if trimmed.len() >= 3
                && !trimmed.ends_with('.')
                && trimmed.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
                && trimmed.chars().any(|c| c.is_alphabetic())
            {
                // Titlecase it
                let title: String = trimmed
                    .split_whitespace()
                    .map(|w| {
                        let mut chars = w.chars();
                        match chars.next() {
                            Some(c) => {
                                let upper: String = c.to_uppercase().collect();
                                let lower: String = chars.flat_map(|ch| ch.to_lowercase()).collect();
                                format!("{}{}", upper, lower)
                            }
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                return Self::truncate_summary(&title, 80);
            }
            // Section/Article/Clause/Part + number
            if trimmed.starts_with("Section ")
                || trimmed.starts_with("Article ")
                || trimmed.starts_with("Clause ")
                || trimmed.starts_with("Part ")
                || trimmed.starts_with("SECTION ")
                || trimmed.starts_with("ARTICLE ")
                || trimmed.starts_with("CLAUSE ")
                || trimmed.starts_with("PART ")
            {
                return Self::truncate_summary(trimmed, 80);
            }
            // Numbered heading like "5. Termination" or "12.1 Indemnification"
            let bytes = trimmed.as_bytes();
            if !bytes.is_empty()
                && bytes[0].is_ascii_digit()
                && (trimmed.contains(". ") || trimmed.contains(' '))
            {
                // Make sure there's a text part after the number
                if let Some(pos) = trimmed.find(|c: char| c.is_alphabetic()) {
                    if pos < trimmed.len() && trimmed.len() <= 60 {
                        return Self::truncate_summary(trimmed, 80);
                    }
                }
            }
        }

        // Layer 2 — keyword extraction fallback
        let full_text = text.trim();
        if full_text.len() < 150 {
            // Very short chunk: use first sentence truncated
            let end = full_text.find(|c: char| c == '.' || c == '!' || c == '?');
            if let Some(pos) = end {
                return Self::truncate_summary(&full_text[..=pos], 80);
            }
            return Self::truncate_summary(full_text, 80);
        }

        // Stopwords: English + legal boilerplate
        const STOPWORDS: &[&str] = &[
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will", "would",
            "could", "should", "may", "might", "can", "this", "that", "these",
            "those", "it", "its", "not", "no", "nor", "if", "then", "than",
            "so", "as", "any", "all", "each", "every", "such", "other", "into",
            "upon", "under", "over", "between", "through", "after", "before",
            // Legal boilerplate
            "shall", "hereby", "thereof", "herein", "pursuant", "notwithstanding",
            "hereinafter", "therein", "thereto", "whereas", "hereunder", "hereof",
            "aforesaid", "foregoing", "witnesseth", "provided",
        ];
        let stopset: std::collections::HashSet<&str> = STOPWORDS.iter().copied().collect();

        let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for word in full_text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
        {
            let lower = word.to_lowercase();
            if !stopset.contains(lower.as_str()) {
                *freq.entry(lower).or_insert(0) += 1;
            }
        }

        let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
        terms.sort_by(|a, b| b.1.cmp(&a.1));

        let top: Vec<&str> = terms.iter().take(4).map(|(t, _)| t.as_str()).collect();
        if top.is_empty() {
            return Self::truncate_summary(full_text, 80);
        }

        if top.len() <= 2 {
            format!("Covers: {}", top.join(", "))
        } else {
            let last = top.last().unwrap();
            let rest = &top[..top.len() - 1];
            format!("Covers: {}, and {}", rest.join(", "), last)
        }
    }

    fn truncate_summary(s: &str, max_chars: usize) -> String {
        if s.chars().count() <= max_chars {
            s.to_string()
        } else {
            let end = s
                .char_indices()
                .nth(max_chars)
                .map(|(i, _)| i)
                .unwrap_or(s.len());
            format!("{}…", &s[..end])
        }
    }

    /// Extract the best excerpt from chunk text relevant to the query.
    /// Returns 2–3 sentences around the most relevant sentence for richer context.
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

        let sentences = Self::split_sentences(&sanitized);

        if sentences.is_empty() {
            let end = sanitized
                .char_indices()
                .nth(450)
                .map(|(i, _)| i)
                .unwrap_or(sanitized.len());
            return sanitized[..end].to_string();
        }

        let query_words: std::collections::HashSet<String> = query
            .to_lowercase()
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.len() > 2)
            .map(|w| w.to_string())
            .collect();

        let mut best_idx = 0;
        let mut best_score = -1.0f32;

        for (i, sentence) in sentences.iter().enumerate() {
            let words: Vec<&str> = sentence
                .split(|c: char| !c.is_alphabetic())
                .filter(|w| !w.is_empty())
                .collect();
            if words.is_empty() {
                continue;
            }
            let match_count = words
                .iter()
                .filter(|w| query_words.contains(&w.to_lowercase()))
                .count();
            // Prefer longer sentences (more context) with good match density
            let length_bonus = (sentence.len() as f64 / 100.0).min(1.0); // 0-1 bonus for length up to 100 chars
            let adjusted_score = match_count as f64 + length_bonus * 0.5;
            let score = adjusted_score as f32 / (words.len() as f32).sqrt();
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        // Gather best sentence + 1 before and 1 after for richer context
        let start = if best_idx > 0 { best_idx - 1 } else { 0 };
        let end = (best_idx + 2).min(sentences.len());
        let excerpt = sentences[start..end].join(". ");

        // Truncate to 500 chars
        let char_end = excerpt
            .char_indices()
            .nth(500)
            .map(|(i, _)| i)
            .unwrap_or(excerpt.len());
        if excerpt.len() > char_end {
            format!("{}…", &excerpt[..char_end])
        } else {
            excerpt
        }
    }

    /// Split text into sentences, handling common legal abbreviations
    /// (U.S., Inc., No., Art., Sec., etc.) that would otherwise cause
    /// false sentence breaks.
    fn split_sentences(text: &str) -> Vec<&str> {
        // Abbreviations where a period does NOT end a sentence
        const ABBREVS: &[&str] = &[
            "U.S", "u.s", "Inc", "Corp", "Ltd", "Co", "Jr", "Sr", "Dr",
            "Mr", "Mrs", "Ms", "Prof", "Rev", "Gen", "Gov", "Sgt",
            "No", "Nos", "Art", "Sec", "Dept", "Div", "Ch", "Vol",
            "Fig", "App", "Exh", "Cl", "Par", "Sub", "Amdt",
            "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep",
            "Oct", "Nov", "Dec", "St", "Ave", "Blvd", "Ct", "Rd",
            "v", "vs", "et", "al", "e.g", "i.e", "cf",
        ];

        let mut result = Vec::new();
        let mut start = 0;
        let bytes = text.as_bytes();
        let len = bytes.len();

        let mut i = 0;
        while i < len {
            let b = bytes[i];
            if b == b'.' || b == b'!' || b == b'?' {
                // Check if this period is part of an abbreviation
                if b == b'.' {
                    // Look back for the word before this period
                    let word_start = text[..i]
                        .rfind(|c: char| c.is_whitespace() || c == '(' || c == '"')
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let word = &text[word_start..i];
                    // Skip if it's a known abbreviation or a single capital letter (initials)
                    if ABBREVS.iter().any(|a| word.ends_with(a))
                        || (word.len() == 1 && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                        // Skip decimal numbers like "3.5"
                        || (i + 1 < len && bytes[i + 1].is_ascii_digit())
                    {
                        i += 1;
                        continue;
                    }
                }
                // This is a real sentence boundary
                let sentence = text[start..=i].trim();
                if sentence.len() > 15 {
                    result.push(sentence);
                }
                start = i + 1;
            }
            i += 1;
        }
        // Trailing text without terminal punctuation
        let tail = text[start..].trim();
        if tail.len() > 15 {
            result.push(tail);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_all_caps_heading() {
        let text = "TERMINATION AND DEFAULT\nThe landlord may terminate this agreement upon 30 days written notice to the tenant if the tenant fails to pay rent.";
        let summary = RagState::summarize_chunk(text);
        assert_eq!(summary, "Termination And Default");
    }

    #[test]
    fn summarize_section_heading() {
        let text = "Section 5. Indemnification\nEach party shall indemnify the other against all losses arising from breach.";
        let summary = RagState::summarize_chunk(text);
        assert_eq!(summary, "Section 5. Indemnification");
    }

    #[test]
    fn summarize_article_heading() {
        let text = "ARTICLE III\nThe company shall maintain adequate insurance coverage at all times during the term of this agreement.";
        let summary = RagState::summarize_chunk(text);
        assert_eq!(summary, "Article Iii");
    }

    #[test]
    fn summarize_numbered_heading() {
        let text = "12.1 Governing Law\nThis agreement shall be governed by the laws of the State of California.";
        let summary = RagState::summarize_chunk(text);
        assert_eq!(summary, "12.1 Governing Law");
    }

    #[test]
    fn summarize_keyword_fallback() {
        // Long chunk with no heading — should produce keyword summary
        let text = "The tenant agrees to pay monthly rent of $2,500 on the first day of each calendar month. Late payment fees of $100 will be assessed for any payment received after the fifth day. The landlord reserves the right to increase rent upon 60 days written notice.";
        let summary = RagState::summarize_chunk(text);
        assert!(summary.starts_with("Covers: "), "got: {}", summary);
    }

    #[test]
    fn summarize_short_chunk_uses_first_sentence() {
        let text = "Rent is due on the first of each month.";
        let summary = RagState::summarize_chunk(text);
        assert_eq!(summary, "Rent is due on the first of each month.");
    }

    #[test]
    fn summarize_empty_input() {
        let summary = RagState::summarize_chunk("");
        assert_eq!(summary, "");
    }

    #[test]
    fn summarize_caps_with_trailing_period_not_matched() {
        // ALL-CAPS with trailing period should NOT be treated as heading
        let text = "AGREEMENT TERMINATED.\nThis is the rest of the document content that explains the termination in detail with enough words to exceed the minimum threshold.";
        let summary = RagState::summarize_chunk(text);
        // Should fall through to keyword or short-text fallback, not titlecase heading
        assert!(!summary.contains("Agreement Terminated"), "got: {}", summary);
    }

    #[test]
    fn summarize_long_caps_line_skipped() {
        // Line > 60 chars should not be treated as heading
        let text = "THIS IS A VERY LONG LINE THAT EXCEEDS SIXTY CHARACTERS AND SHOULD NOT BE A HEADING\nActual content follows here with enough words to be meaningful.";
        let summary = RagState::summarize_chunk(text);
        assert!(!summary.starts_with("This Is A Very"), "got: {}", summary);
    }

    #[test]
    fn truncate_summary_respects_limit() {
        let long = "A".repeat(100);
        let result = RagState::truncate_summary(&long, 80);
        assert!(result.chars().count() <= 81); // 80 + ellipsis
        assert!(result.ends_with('…'));
    }

    // ── Cosine similarity tests ──────────────────────────────────────────

    #[test]
    fn cosine_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sim = RagState::cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5, "identical vectors should have sim ~1.0, got {sim}");
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = RagState::cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5, "orthogonal vectors should have sim ~0.0, got {sim}");
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = RagState::cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-5, "opposite vectors should have sim ~-1.0, got {sim}");
    }

    #[test]
    fn cosine_empty_returns_zero() {
        let sim = RagState::cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = RagState::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths_returns_zero() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = RagState::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn cosine_remainder_elements() {
        // 5 elements: 4 in unrolled loop + 1 remainder
        let a = vec![1.0, 0.0, 0.0, 0.0, 1.0];
        let b = vec![1.0, 0.0, 0.0, 0.0, 1.0];
        let sim = RagState::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5, "got {sim}");
    }

    #[test]
    fn cosine_with_norms_matches_standard() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![4.0, 3.0, 2.0, 1.0];
        let standard = RagState::cosine_similarity(&a, &b);

        let norm_a: f64 = a.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt();
        let with_norms = RagState::cosine_similarity_with_norms(&a, &b, norm_a, norm_b);

        assert!((standard - with_norms).abs() < 1e-5,
            "standard={standard} with_norms={with_norms}");
    }

    #[test]
    fn precompute_norms_correct() {
        let v1 = vec![3.0f32, 4.0];
        let v2 = vec![1.0f32, 0.0];
        let norms = RagState::precompute_norms_owned(&[v1, v2]);
        assert!((norms[0] - 5.0).abs() < 1e-10, "3-4-5 triangle: got {}", norms[0]);
        assert!((norms[1] - 1.0).abs() < 1e-10, "unit vector: got {}", norms[1]);
    }

    #[test]
    fn batch_cosine_matches_individual() {
        let query = vec![1.0, 2.0, 3.0];
        let c0 = vec![3.0f32, 2.0, 1.0];
        let c1 = vec![0.0f32, 1.0, 0.0];
        let c2 = vec![1.0f32, 1.0, 1.0];
        let corpus: Vec<&[f32]> = vec![c0.as_slice(), c1.as_slice(), c2.as_slice()];

        let batch = RagState::batch_cosine_similarity(&query, &corpus);
        for (i, doc) in corpus.iter().enumerate() {
            let individual = RagState::cosine_similarity(&query, doc);
            assert!((batch[i] - individual).abs() < 1e-5,
                "mismatch at {i}: batch={} individual={}", batch[i], individual);
        }
    }

    #[test]
    fn batch_cosine_zero_query() {
        let query = vec![0.0, 0.0, 0.0];
        let c0 = vec![1.0f32, 2.0, 3.0];
        let corpus: Vec<&[f32]> = vec![c0.as_slice()];
        let batch = RagState::batch_cosine_similarity(&query, &corpus);
        assert_eq!(batch[0], 0.0);
    }

    // ── Structured data summarization tests (Gap 13) ────────────────────

    #[test]
    fn summarize_table_content() {
        let text = "Name\tAge\tCity\nAlice\t30\tNY\nBob\t25\tLA";
        let summary = RagState::summarize_chunk(text);
        assert!(summary.starts_with("Table ("), "got: {}", summary);
        assert!(summary.contains("rows"), "got: {}", summary);
    }

    #[test]
    fn summarize_pipe_table() {
        let text = "| Name | Age | City |\n| Alice | 30 | NY |\n| Bob | 25 | LA |";
        let summary = RagState::summarize_chunk(text);
        assert!(summary.starts_with("Table ("), "got: {}", summary);
    }

    #[test]
    fn summarize_form_fields() {
        let text = "Name: John Smith\nAddress: 123 Main St\nPhone: 555-1234\nEmail: john@example.com";
        let summary = RagState::summarize_chunk(text);
        assert!(summary.starts_with("Form data ("), "got: {}", summary);
        assert!(summary.contains("fields"), "got: {}", summary);
    }

    #[test]
    fn summarize_single_table_line_not_triggered() {
        // Only 1 table line — should not trigger table detection
        let text = "Name\tAge\tCity\nThis is a normal sentence without tabs.";
        let summary = RagState::summarize_chunk(text);
        assert!(!summary.starts_with("Table ("), "got: {}", summary);
    }

    // ── Length bonus in best_excerpt tests (Gap 14) ─────────────────────

    #[test]
    fn best_excerpt_prefers_longer_sentence() {
        // Short sentence has exact match but longer sentence has match + more context
        let text = "Fee: $100. The monthly fee of $100 is due on the first business day of each calendar month.";
        let excerpt = RagState::best_excerpt(text, "fee monthly");
        // Should prefer the longer sentence with more context
        assert!(excerpt.contains("monthly fee"), "got: {}", excerpt);
    }
}
