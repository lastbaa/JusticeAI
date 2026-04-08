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
    #[serde(default)]
    pub summary: String,
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

    /// Generate a short summary describing what a chunk covers.
    /// Layer 1: extract heading from the first few lines.
    /// Layer 2: keyword extraction fallback.
    pub fn summarize_chunk(text: &str) -> String {
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
            let hits = words
                .iter()
                .filter(|w| query_words.contains(&w.to_lowercase()))
                .count();
            let score = hits as f32 / (words.len() as f32).sqrt();
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
}
