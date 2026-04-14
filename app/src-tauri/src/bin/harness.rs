//! CLI test harness for the Justice AI RAG pipeline.
//!
//! Usage: harness --pdf <path> --query <text> [--data-dir <path>] [--skip-llm] [--backend <name>]
//!        harness --eval <eval.json> [--data-dir <path>] [--backend <name>] [--report out.json] [--json-out <path>]
//!        harness --eval <eval.json> --compare backend1,backend2
//!        harness --eval <eval.json> --diff baseline.json [--report out.json]
//!        harness --eval <eval.json> --benchmark-modes [--report out.json]
//!        harness --eval <eval.json> --mode quick|balanced|extended
//!
//! Defaults:
//!   --data-dir: macOS: ~/Library/Application Support/com.justiceai.app
//!               Linux: $XDG_DATA_HOME/com.justiceai.app (~/.local/share/com.justiceai.app)
//!               Windows: %APPDATA%/com.justiceai.app
//!   --skip-llm: false (run LLM if model exists)
//!   --backend:  hybrid-bm25-cosine
//!
//! Eval JSON format:
//! ```json
//! [
//!   {
//!     "pdf": "tests/fixtures/irs_w9_filled.pdf",
//!     "query": "What is the person's name?",
//!     "expected": ["Liam Neild"],
//!     "type": "standard",
//!     "difficulty": "easy",
//!     "tags": ["w9"],
//!     "notes": "Simple name lookup",
//!     "must_not_contain": []
//!   }
//! ]
//! ```

use app_lib::commands::doc_parser;
use app_lib::pipeline::{self, RetrievalBackend, RetrievalConfig, RetrievalCorpus};
use app_lib::state::{AppSettings, ChunkMetadata, DocumentPage, DocumentRole, InferenceMode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// == Persistent embedding cache ================================================

/// On-disk embedding cache keyed by blake3(text).
/// Stored at `target/eval-cache/embed_cache.bin` (bincode-serialized).
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct EmbedDiskCache {
    entries: HashMap<String, Vec<f32>>,
}

impl EmbedDiskCache {
    fn cache_dir() -> PathBuf {
        // Walk up from the binary's location to find the workspace target/ dir.
        // Fallback: use `CARGO_MANIFEST_DIR` at compile time or just `target/`.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("eval-cache");
        dir
    }

    fn cache_path() -> PathBuf {
        Self::cache_dir().join("embed_cache.bin")
    }

    fn load() -> Self {
        let path = Self::cache_path();
        match std::fs::read(&path) {
            Ok(bytes) => {
                match bincode::deserialize::<EmbedDiskCache>(&bytes) {
                    Ok(cache) => {
                        eprintln!("[cache] Loaded {} cached embeddings from {}", cache.entries.len(), path.display());
                        cache
                    }
                    Err(e) => {
                        eprintln!("[cache] Corrupt cache file, starting fresh: {e}");
                        Self::default()
                    }
                }
            }
            Err(_) => {
                eprintln!("[cache] No cache file found, starting fresh.");
                Self::default()
            }
        }
    }

    fn save(&self) {
        let dir = Self::cache_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("[cache] Cannot create cache dir: {e}");
            return;
        }
        let path = Self::cache_path();
        match bincode::serialize(self) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&path, &bytes) {
                    eprintln!("[cache] Failed to write cache: {e}");
                } else {
                    eprintln!("[cache] Saved {} embeddings to {}", self.entries.len(), path.display());
                }
            }
            Err(e) => eprintln!("[cache] Serialization error: {e}"),
        }
    }

    fn key(text: &str, is_query: bool) -> String {
        // Include is_query in the hash since BGE uses a different prefix for queries
        let mut hasher = blake3::Hasher::new();
        hasher.update(if is_query { b"q:" } else { b"d:" });
        hasher.update(text.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    fn get(&self, text: &str, is_query: bool) -> Option<&Vec<f32>> {
        self.entries.get(&Self::key(text, is_query))
    }

    fn insert(&mut self, text: &str, is_query: bool, vec: Vec<f32>) {
        self.entries.insert(Self::key(text, is_query), vec);
    }
}

fn print_banner(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!(" {title}");
    println!("{}", "=".repeat(60));
}

// == Eval types ================================================================

#[derive(serde::Deserialize, Clone)]
struct EvalCase {
    pdf: String,
    query: String,
    expected: Vec<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default = "default_top_k")]
    #[allow(dead_code)]
    top_k: usize,
    #[serde(default = "default_case_type", rename = "type")]
    case_type: String,
    #[serde(default = "default_difficulty")]
    difficulty: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    must_not_contain: Vec<String>,
}

fn default_top_k() -> usize { 6 }
fn default_case_type() -> String { "standard".to_string() }
fn default_difficulty() -> String { "medium".to_string() }

/// Whether this case type expects NO expected terms to be found.
fn is_negative_type(case_type: &str) -> bool {
    matches!(case_type, "negative" | "adversarial")
}

#[derive(Clone, serde::Serialize)]
struct EvalResult {
    query: String,
    pdf: String,
    label: String,
    case_type: String,
    difficulty: String,
    recall: f32,
    partial_score: f32,
    mrr: f32,
    precision_at_1: bool,
    answer_rank: Option<usize>,
    passed: bool,
    missed: Vec<String>,
    must_not_violations: Vec<String>,
    top_scores: Vec<(f32, usize)>,
    tags: Vec<String>,
    notes: String,
}

// == JSON report types (for --report / --diff / --json-out) ====================

#[derive(serde::Serialize, serde::Deserialize)]
struct EvalReport {
    backend: String,
    timestamp: String,
    cases: Vec<CaseReport>,
    summary: SummaryReport,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CaseReport {
    label: String,
    query: String,
    pdf: String,
    case_type: String,
    difficulty: String,
    recall: f32,
    partial_score: f32,
    mrr: f32,
    precision_at_1: bool,
    answer_rank: Option<usize>,
    passed: bool,
    missed: Vec<String>,
    must_not_violations: Vec<String>,
    tags: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SummaryReport {
    total: usize,
    passed: usize,
    avg_recall: f32,
    mrr: f32,
    p_at_1: f32,
    avg_partial_score: f32,
}

/// Extended JSON output for --json-out flag.
#[derive(serde::Serialize)]
struct JsonOutput {
    backend: String,
    total_cases: usize,
    passed: usize,
    aggregate: AggregateMetrics,
    by_difficulty: HashMap<String, AggregateMetrics>,
    by_type: HashMap<String, AggregateMetrics>,
    by_pdf: HashMap<String, PdfGroupMetrics>,
    results: Vec<EvalResult>,
}

#[derive(serde::Serialize, Clone)]
struct AggregateMetrics {
    count: usize,
    passed: usize,
    avg_recall: f32,
    avg_mrr: f32,
    precision_at_1: f32,
    avg_partial_score: f32,
}

#[derive(serde::Serialize, Clone)]
struct PdfGroupMetrics {
    count: usize,
    passed: usize,
    avg_mrr: f32,
    results: Vec<PdfCaseSummary>,
}

#[derive(serde::Serialize, Clone)]
struct PdfCaseSummary {
    query: String,
    difficulty: String,
    case_type: String,
    passed: bool,
    mrr: f32,
    rank: Option<usize>,
}

impl EvalResult {
    fn to_case_report(&self) -> CaseReport {
        CaseReport {
            label: self.label.clone(),
            query: self.query.clone(),
            pdf: self.pdf.clone(),
            case_type: self.case_type.clone(),
            difficulty: self.difficulty.clone(),
            recall: self.recall,
            partial_score: self.partial_score,
            mrr: self.mrr,
            precision_at_1: self.precision_at_1,
            answer_rank: self.answer_rank,
            passed: self.passed,
            missed: self.missed.clone(),
            must_not_violations: self.must_not_violations.clone(),
            tags: self.tags.clone(),
        }
    }
}

fn make_label(case: &EvalCase) -> String {
    if let Some(ref l) = case.label {
        l.clone()
    } else {
        let file = std::path::Path::new(&case.pdf)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| case.pdf.clone());
        let q: String = case.query.chars().take(30).collect();
        format!("{}: {}", file, q)
    }
}

fn compute_summary(results: &[EvalResult]) -> SummaryReport {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let avg_recall = if total > 0 {
        results.iter().map(|r| r.recall).sum::<f32>() / total as f32
    } else { 0.0 };
    let mrr = if total > 0 {
        results.iter().map(|r| r.mrr).sum::<f32>() / total as f32
    } else { 0.0 };
    let p_at_1 = if total > 0 {
        results.iter().filter(|r| r.precision_at_1).count() as f32 / total as f32
    } else { 0.0 };
    let avg_partial = if total > 0 {
        results.iter().map(|r| r.partial_score).sum::<f32>() / total as f32
    } else { 0.0 };
    SummaryReport { total, passed, avg_recall, mrr, p_at_1, avg_partial_score: avg_partial }
}

fn compute_aggregate(results: &[EvalResult]) -> AggregateMetrics {
    let count = results.len();
    if count == 0 {
        return AggregateMetrics {
            count: 0, passed: 0, avg_recall: 0.0, avg_mrr: 0.0,
            precision_at_1: 0.0, avg_partial_score: 0.0,
        };
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let avg_recall = results.iter().map(|r| r.recall).sum::<f32>() / count as f32;
    let avg_mrr = results.iter().map(|r| r.mrr).sum::<f32>() / count as f32;
    let p1 = results.iter().filter(|r| r.precision_at_1).count() as f32 / count as f32;
    let avg_partial = results.iter().map(|r| r.partial_score).sum::<f32>() / count as f32;
    AggregateMetrics { count, passed, avg_recall, avg_mrr, precision_at_1: p1, avg_partial_score: avg_partial }
}

fn build_report(backend_name: &str, results: &[EvalResult]) -> EvalReport {
    let summary = compute_summary(results);
    let cases: Vec<CaseReport> = results.iter().map(|r| r.to_case_report()).collect();
    let timestamp = {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}", d.as_secs())
    };
    EvalReport { backend: backend_name.to_string(), timestamp, cases, summary }
}

fn save_report(report: &EvalReport, path: &str) {
    let json = serde_json::to_string_pretty(report)
        .unwrap_or_else(|e| { eprintln!("Failed to serialize report: {e}"); std::process::exit(1); });
    std::fs::write(path, json)
        .unwrap_or_else(|e| { eprintln!("Failed to write report to {path}: {e}"); std::process::exit(1); });
    println!("\nReport saved to {path}");
}

// == Shared helpers ============================================================

fn parse_and_chunk(pdf_path: &str, settings: &AppSettings) -> Result<(Vec<DocumentPage>, Vec<pipeline::TempChunk>), String> {
    let lower = pdf_path.to_lowercase();
    let pages = if lower.ends_with(".pdf") {
        doc_parser::parse_pdf(pdf_path)?
    } else if lower.ends_with(".docx") {
        doc_parser::parse_docx(pdf_path)?
    } else {
        return Err(format!("Unsupported file type: {pdf_path}"));
    };
    let chunks = pipeline::chunk_document(&pages, settings);
    Ok((pages, chunks))
}

/// Embed query + all chunks, then score using the given backend.
async fn embed_and_retrieve(
    chunks: &[pipeline::TempChunk],
    query: &str,
    pdf_path: &str,
    model_dir: &Path,
    backend: &dyn RetrievalBackend,
    config: &RetrievalConfig,
) -> Result<Vec<(f32, ChunkMetadata, usize)>, String> {
    let query_vec = pipeline::embed_text(query, true, model_dir).await?;

    let mut chunk_vecs: Vec<Vec<f32>> = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        chunk_vecs.push(pipeline::embed_text(&chunk.text, false, model_dir).await?);
    }

    let corpus = RetrievalCorpus {
        texts: chunks.iter().map(|c| c.text.as_str()).collect(),
        vectors: chunk_vecs.iter().map(|v| v.as_slice()).collect(),
        chunk_indices: chunks.iter().map(|c| c.chunk_index).collect(),
        bm25_index: None,
    };

    let mut ranked = backend.retrieve(query, &query_vec, &corpus, config);
    pipeline::ensure_form_data_included(&mut ranked, &corpus, 2);

    let file_name = std::path::Path::new(pdf_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| pdf_path.to_string());

    Ok(ranked
        .into_iter()
        .map(|r| {
            let chunk = &chunks[r.chunk_index];
            let meta = ChunkMetadata {
                id: chunk.id.clone(),
                document_id: "harness-doc".to_string(),
                file_name: file_name.clone(),
                file_path: pdf_path.to_string(),
                page_number: chunk.page_number,
                chunk_index: chunk.chunk_index,
                text: chunk.text.clone(),
                token_count: chunk.token_count,
                role: DocumentRole::default(),
                start_char_offset: Some(chunk.start_char_offset),
                end_char_offset: Some(chunk.end_char_offset),
            };
            (r.score, meta, r.chunk_index)
        })
        .collect())
}

/// Retrieve using pre-cached chunk embeddings (avoids re-embedding per query).
fn retrieve_with_cached_embeddings(
    chunks: &[pipeline::TempChunk],
    chunk_vecs: &[Vec<f32>],
    query: &str,
    query_vec: &[f32],
    pdf_path: &str,
    backend: &dyn RetrievalBackend,
    config: &RetrievalConfig,
) -> Vec<(f32, ChunkMetadata, usize)> {
    let corpus = RetrievalCorpus {
        texts: chunks.iter().map(|c| c.text.as_str()).collect(),
        vectors: chunk_vecs.iter().map(|v| v.as_slice()).collect(),
        chunk_indices: chunks.iter().map(|c| c.chunk_index).collect(),
        bm25_index: None,
    };

    let mut ranked = backend.retrieve(query, query_vec, &corpus, config);
    pipeline::ensure_form_data_included(&mut ranked, &corpus, 2);

    let file_name = std::path::Path::new(pdf_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| pdf_path.to_string());

    ranked
        .into_iter()
        .map(|r| {
            let chunk = &chunks[r.chunk_index];
            let meta = ChunkMetadata {
                id: chunk.id.clone(),
                document_id: "harness-doc".to_string(),
                file_name: file_name.clone(),
                file_path: pdf_path.to_string(),
                page_number: chunk.page_number,
                chunk_index: chunk.chunk_index,
                text: chunk.text.clone(),
                token_count: chunk.token_count,
                role: DocumentRole::default(),
                start_char_offset: Some(chunk.start_char_offset),
                end_char_offset: Some(chunk.end_char_offset),
            };
            (r.score, meta, r.chunk_index)
        })
        .collect()
}

/// Select the retrieval backend by name.
fn select_backend(name: &str, model_dir: &Path) -> Box<dyn RetrievalBackend> {
    match name {
        "hybrid-bm25-cosine" | "hybrid" | "default" => {
            Box::new(pipeline::default_backend())
        }
        "reranker" | "jina" | "reranker-jina" => {
            Box::new(pipeline::RerankerBackend::new(model_dir.to_path_buf()))
        }
        other => {
            eprintln!("Unknown backend: '{other}'.");
            eprintln!("Available: hybrid-bm25-cosine, reranker");
            std::process::exit(1);
        }
    }
}

// == Core eval runner (with per-PDF deduplication) =============================

async fn run_eval_cases(
    cases: &[EvalCase],
    model_dir: &Path,
    backend: &dyn RetrievalBackend,
    verbose: bool,
    mode: &InferenceMode,
) -> Vec<EvalResult> {
    let settings = AppSettings::default();
    let retrieval_params = pipeline::RetrievalModeParams::from_mode(mode);
    let mut eval_results: Vec<EvalResult> = Vec::new();

    // -- Persistent disk cache for embeddings ---------------------------------
    let mut disk_cache = EmbedDiskCache::load();
    let mut cache_hits: usize = 0;
    let mut cache_misses: usize = 0;

    // -- Per-PDF deduplication: parse + embed each PDF only once ---------------
    let mut pdf_cache: HashMap<String, (Vec<DocumentPage>, Vec<pipeline::TempChunk>)> = HashMap::new();
    let mut embed_cache: HashMap<String, Vec<Vec<f32>>> = HashMap::new();

    // Collect unique PDFs in order of first appearance
    let unique_pdfs: Vec<String> = {
        let mut seen = Vec::new();
        for c in cases {
            if !seen.contains(&c.pdf) {
                seen.push(c.pdf.clone());
            }
        }
        seen
    };

    if verbose {
        println!("Backend: {}", backend.name());
        println!("Parsing {} unique PDFs...", unique_pdfs.len());
    }

    for pdf in &unique_pdfs {
        if verbose { print!("  Parsing {} ... ", pdf); }
        match parse_and_chunk(pdf, &settings) {
            Ok(result) => {
                if verbose {
                    println!("OK ({} pages, {} chunks)", result.0.len(), result.1.len());
                }
                // Pre-embed all chunks for this PDF (with disk cache)
                let mut chunk_vecs: Vec<Vec<f32>> = Vec::with_capacity(result.1.len());
                let mut embed_ok = true;
                for chunk in &result.1 {
                    if let Some(cached) = disk_cache.get(&chunk.text, false) {
                        chunk_vecs.push(cached.clone());
                        cache_hits += 1;
                    } else {
                        match pipeline::embed_text(&chunk.text, false, model_dir).await {
                            Ok(v) => {
                                disk_cache.insert(&chunk.text, false, v.clone());
                                chunk_vecs.push(v);
                                cache_misses += 1;
                            }
                            Err(e) => {
                                if verbose { println!("    EMBED ERROR: {e}"); }
                                embed_ok = false;
                                break;
                            }
                        }
                    }
                }
                if embed_ok {
                    embed_cache.insert(pdf.clone(), chunk_vecs);
                }
                pdf_cache.insert(pdf.clone(), result);
            }
            Err(e) => {
                if verbose { println!("ERROR: {e}"); }
            }
        }
    }

    if verbose {
        println!("Embedding cache: {} hits, {} misses", cache_hits, cache_misses);
    }

    if verbose { println!("\nRunning {} eval cases...\n", cases.len()); }

    for (ci, case) in cases.iter().enumerate() {
        let label = make_label(case);
        let is_neg = is_negative_type(&case.case_type);
        let diff_tag = format!("[{}]", case.difficulty);
        let type_tag = format!("[{}]", case.case_type);

        if verbose {
            print!("[{}/{}] {diff_tag} {type_tag} {} -> \"{}\" ... ",
                ci + 1, cases.len(), case.pdf, case.query);
        }

        // Check if PDF was parsed and embedded successfully
        let chunks = match pdf_cache.get(&case.pdf) {
            Some((_pages, chunks)) => chunks,
            None => {
                if verbose { println!("SKIP (parse failed)"); }
                eval_results.push(EvalResult {
                    query: case.query.clone(), pdf: case.pdf.clone(), label,
                    case_type: case.case_type.clone(), difficulty: case.difficulty.clone(),
                    recall: 0.0, partial_score: 0.0, mrr: 0.0, precision_at_1: false,
                    answer_rank: None, passed: false,
                    missed: case.expected.clone(), must_not_violations: vec![],
                    top_scores: vec![], tags: case.tags.clone(), notes: case.notes.clone(),
                });
                continue;
            }
        };

        let chunk_vecs = match embed_cache.get(&case.pdf) {
            Some(v) => v,
            None => {
                if verbose { println!("SKIP (embed failed)"); }
                eval_results.push(EvalResult {
                    query: case.query.clone(), pdf: case.pdf.clone(), label,
                    case_type: case.case_type.clone(), difficulty: case.difficulty.clone(),
                    recall: 0.0, partial_score: 0.0, mrr: 0.0, precision_at_1: false,
                    answer_rank: None, passed: false,
                    missed: case.expected.clone(), must_not_violations: vec![],
                    top_scores: vec![], tags: case.tags.clone(), notes: case.notes.clone(),
                });
                continue;
            }
        };

        // Embed query (unique per case, with disk cache)
        let query_vec = if let Some(cached) = disk_cache.get(&case.query, true) {
            cached.clone()
        } else {
            match pipeline::embed_text(&case.query, true, model_dir).await {
                Ok(v) => {
                    disk_cache.insert(&case.query, true, v.clone());
                    v
                }
                Err(e) => {
                    if verbose { println!("EMBED ERROR (query): {e}"); }
                    eval_results.push(EvalResult {
                        query: case.query.clone(), pdf: case.pdf.clone(), label,
                        case_type: case.case_type.clone(), difficulty: case.difficulty.clone(),
                        recall: 0.0, partial_score: 0.0, mrr: 0.0, precision_at_1: false,
                        answer_rank: None, passed: false,
                        missed: case.expected.clone(), must_not_violations: vec![],
                        top_scores: vec![], tags: case.tags.clone(), notes: case.notes.clone(),
                    });
                    continue;
                }
            }
        };

        let config = RetrievalConfig {
            top_k: retrieval_params.top_k,
            candidate_pool_k: retrieval_params.candidate_pool_k,
            score_threshold: 0.0,
            expand_keywords: true,
            mmr_lambda: retrieval_params.mmr_lambda,
            jaccard_threshold: retrieval_params.jaccard_threshold,
            adaptive_k_gap: retrieval_params.adaptive_k_gap,
        };

        let scored = retrieve_with_cached_embeddings(
            chunks, chunk_vecs, &case.query, &query_vec, &case.pdf,
            backend, &config,
        );

        // Per-chunk text for rank-aware metrics
        let chunk_texts: Vec<String> = scored.iter()
            .map(|(_, m, _)| m.text.to_lowercase())
            .collect();


        let top_scores: Vec<(f32, usize)> = scored.iter().map(|(s, _, idx)| (*s, *idx)).collect();

        if is_neg && case.expected.is_empty() {
            // -- Negative / adversarial case ----------------------------------
            // PASS = top retrieval scores are low AND no must_not_contain violations.
            // For negative cases expected is empty, so we only check score confidence.
            let max_score = top_scores.iter().map(|(s, _)| *s).fold(0.0_f32, f32::max);
            let scores_low = max_score < 0.65;

            let all_text: String = chunk_texts.join(" ");
            let must_not_violations: Vec<String> = case.must_not_contain.iter()
                .filter(|term| all_text.contains(&term.to_lowercase()))
                .cloned()
                .collect();

            let passed = scores_low && must_not_violations.is_empty();

            if verbose {
                if passed {
                    println!("PASS (negative, max_score={:.3})", max_score);
                } else {
                    let reasons: Vec<String> = [
                        if !scores_low { Some(format!("high_score={:.3}", max_score)) } else { None },
                        if !must_not_violations.is_empty() { Some(format!("must_not_violated={:?}", must_not_violations)) } else { None },
                    ].into_iter().flatten().collect();
                    println!("FAIL (negative) {}", reasons.join(", "));
                }
            }

            eval_results.push(EvalResult {
                query: case.query.clone(), pdf: case.pdf.clone(), label,
                case_type: case.case_type.clone(), difficulty: case.difficulty.clone(),
                recall: if passed { 1.0 } else { 0.0 },
                partial_score: if passed { 1.0 } else { 0.0 },
                mrr: if passed { 1.0 } else { 0.0 },
                precision_at_1: passed,
                answer_rank: None,
                passed,
                missed: vec![],
                must_not_violations,
                top_scores,
                tags: case.tags.clone(),
                notes: case.notes.clone(),
            });
        } else {
            // -- Standard / multi-fact / cross-reference case -----------------
            // Find rank (1-indexed) of first chunk containing ANY expected substring
            let first_hit_rank: Option<usize> = chunk_texts.iter().enumerate().find_map(|(i, text)| {
                if case.expected.iter().any(|exp| text.contains(&exp.to_lowercase())) {
                    Some(i + 1)
                } else {
                    None
                }
            });

            let mrr = first_hit_rank.map(|r| 1.0 / r as f32).unwrap_or(0.0);
            let p_at_1 = first_hit_rank == Some(1);

            // Recall: check across ALL retrieved chunks (concatenated)
            let top_text: String = chunk_texts.join(" ");
            let mut found = Vec::new();
            let mut missed = Vec::new();
            for exp in &case.expected {
                if top_text.contains(&exp.to_lowercase()) {
                    found.push(exp.clone());
                } else {
                    missed.push(exp.clone());
                }
            }
            let partial_score = if case.expected.is_empty() { 1.0 } else { found.len() as f32 / case.expected.len() as f32 };
            let recall = partial_score;

            // Check must_not_contain violations — sentence-level scope.
            // Find the first-hit chunk, split it into sentences, and only check
            // must_not against sentences that contain an expected term.  This avoids
            // false failures when confusable entities (e.g. two addresses) appear
            // in the same chunk but in *different* sentences.
            let first_hit_idx = chunk_texts.iter().position(|text| {
                case.expected.iter().any(|exp| text.contains(&exp.to_lowercase()))
            });
            let answer_sentences: String = first_hit_idx
                .map(|i| {
                    // Split on sentence boundaries (period/excl/question followed by space or end)
                    let chunk = &chunk_texts[i];
                    chunk.split(|c: char| c == '.' || c == '!' || c == '?')
                        .filter(|sent| {
                            let s = sent.to_lowercase();
                            case.expected.iter().any(|exp| s.contains(&exp.to_lowercase()))
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let must_not_violations: Vec<String> = case.must_not_contain.iter()
                .filter(|term| answer_sentences.contains(&term.to_lowercase()))
                .cloned()
                .collect();

            let passed = missed.is_empty() && must_not_violations.is_empty();

            if verbose {
                if passed {
                    println!("PASS (recall={:.0}% MRR={:.2} P@1={} rank={:?})",
                        recall * 100.0, mrr, p_at_1 as u8, first_hit_rank);
                } else {
                    let mut fail_parts = Vec::new();
                    if !missed.is_empty() {
                        fail_parts.push(format!("missed={:?}", missed));
                    }
                    if !must_not_violations.is_empty() {
                        fail_parts.push(format!("must_not_violated={:?}", must_not_violations));
                    }
                    println!("FAIL (recall={:.0}% MRR={:.2} P@1={} rank={:?}) {}",
                        recall * 100.0, mrr, p_at_1 as u8, first_hit_rank, fail_parts.join(", "));
                }
            }

            eval_results.push(EvalResult {
                query: case.query.clone(), pdf: case.pdf.clone(), label,
                case_type: case.case_type.clone(), difficulty: case.difficulty.clone(),
                recall, partial_score, mrr, precision_at_1: p_at_1,
                answer_rank: first_hit_rank, passed,
                missed, must_not_violations, top_scores,
                tags: case.tags.clone(), notes: case.notes.clone(),
            });
        }
    }

    // Persist embedding cache to disk for next run
    disk_cache.save();

    eval_results
}

// == Eval mode =================================================================

async fn run_eval(
    eval_path: &str,
    model_dir: &Path,
    backend: &dyn RetrievalBackend,
    report_path: Option<&str>,
    json_out: Option<&str>,
    mode: &InferenceMode,
) {
    let content = std::fs::read_to_string(eval_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read eval file: {e}"); std::process::exit(1); });
    let cases: Vec<EvalCase> = serde_json::from_str(&content)
        .unwrap_or_else(|e| { eprintln!("Invalid eval JSON: {e}"); std::process::exit(1); });

    let results = run_eval_cases(&cases, model_dir, backend, true, mode).await;

    // -- Scorecard ------------------------------------------------------------
    print_scorecard(&results, backend.name());

    // -- Save JSON report (--report) ------------------------------------------
    if let Some(path) = report_path {
        let report = build_report(backend.name(), &results);
        save_report(&report, path);
    }

    // -- Save extended JSON output (--json-out) --------------------------------
    if let Some(json_path) = json_out {
        write_json_output(&results, backend.name(), json_path);
    }
}

fn print_scorecard(eval_results: &[EvalResult], backend_name: &str) {
    let summary = compute_summary(eval_results);

    print_banner(&format!("EVAL SCORECARD ({})", backend_name));
    println!("Cases:          {}", summary.total);
    println!("Passed:         {}/{} ({:.0}%)", summary.passed, summary.total,
        if summary.total > 0 { summary.passed as f32 / summary.total as f32 * 100.0 } else { 0.0 });
    println!("Avg recall:     {:.1}%", summary.avg_recall * 100.0);
    println!("MRR:            {:.3}", summary.mrr);
    println!("Precision@1:    {:.1}% ({}/{})", summary.p_at_1 * 100.0,
        eval_results.iter().filter(|r| r.precision_at_1).count(), summary.total);
    println!("Avg partial:    {:.1}%", summary.avg_partial_score * 100.0);

    // -- Breakdown by difficulty -----------------------------------------------
    print_banner("BY DIFFICULTY");
    for diff in &["easy", "medium", "hard"] {
        let subset: Vec<EvalResult> = eval_results.iter()
            .filter(|r| r.difficulty == *diff)
            .cloned()
            .collect();
        if subset.is_empty() { continue; }
        let m = compute_aggregate(&subset);
        println!("  {:<8} {}/{} passed  MRR={:.3}  P@1={:.1}%  partial={:.1}%",
            diff, m.passed, m.count, m.avg_mrr, m.precision_at_1 * 100.0, m.avg_partial_score * 100.0);
    }

    // -- Breakdown by type -----------------------------------------------------
    print_banner("BY TYPE");
    let mut types: Vec<String> = eval_results.iter().map(|r| r.case_type.clone()).collect();
    types.sort();
    types.dedup();
    for t in &types {
        let subset: Vec<EvalResult> = eval_results.iter()
            .filter(|r| r.case_type == *t)
            .cloned()
            .collect();
        if subset.is_empty() { continue; }
        let m = compute_aggregate(&subset);
        println!("  {:<16} {}/{} passed  MRR={:.3}  P@1={:.1}%  partial={:.1}%",
            t, m.passed, m.count, m.avg_mrr, m.precision_at_1 * 100.0, m.avg_partial_score * 100.0);
    }

    // -- Per-PDF grouped results -----------------------------------------------
    print_banner("PER-PDF RESULTS");

    let mut pdf_order: Vec<String> = Vec::new();
    for r in eval_results {
        if !pdf_order.contains(&r.pdf) {
            pdf_order.push(r.pdf.clone());
        }
    }

    for pdf in &pdf_order {
        let pdf_results: Vec<&EvalResult> = eval_results.iter().filter(|r| r.pdf == *pdf).collect();
        let pdf_passed = pdf_results.iter().filter(|r| r.passed).count();
        let pdf_mrr = if pdf_results.is_empty() { 0.0 } else {
            pdf_results.iter().map(|r| r.mrr).sum::<f32>() / pdf_results.len() as f32
        };

        let short_name = std::path::Path::new(pdf)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| pdf.clone());

        println!("\n  {} ({}/{} passed, MRR={:.3})", short_name, pdf_passed, pdf_results.len(), pdf_mrr);
        for r in &pdf_results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            let rank_str = match r.answer_rank {
                Some(rank) => format!("rank={}", rank),
                None => "rank=-".to_string(),
            };
            println!("    {status} [{:<6}] [{:<16}] MRR={:.2} {rank_str}  \"{}\"",
                r.difficulty, r.case_type, r.mrr, r.query);
            if !r.missed.is_empty() {
                println!("         missed: {:?}", r.missed);
            }
            if !r.must_not_violations.is_empty() {
                println!("         must_not_violated: {:?}", r.must_not_violations);
            }
        }
    }

    // -- Score detail dump -----------------------------------------------------
    print_banner("SCORE DETAILS");
    for r in eval_results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        println!("{status} [{:.0}% MRR={:.2} P@1={} rank={:?}] \"{}\" ({})",
            r.recall * 100.0, r.mrr, r.precision_at_1 as u8, r.answer_rank, r.query, r.pdf);
        if !r.missed.is_empty() {
            println!("   missed: {:?}", r.missed);
        }
        if !r.top_scores.is_empty() {
            let scores_str: Vec<String> = r.top_scores.iter()
                .map(|(s, idx)| format!("chunk{}={:.3}", idx, s))
                .collect();
            println!("   top: {}", scores_str.join(", "));
        }
    }
}

fn write_json_output(eval_results: &[EvalResult], backend_name: &str, json_path: &str) {
    let aggregate = compute_aggregate(eval_results);

    // By difficulty
    let mut by_difficulty: HashMap<String, Vec<EvalResult>> = HashMap::new();
    for r in eval_results {
        by_difficulty.entry(r.difficulty.clone()).or_default().push(r.clone());
    }
    let by_difficulty_metrics: HashMap<String, AggregateMetrics> = by_difficulty.iter()
        .map(|(k, v)| (k.clone(), compute_aggregate(v)))
        .collect();

    // By type
    let mut by_type: HashMap<String, Vec<EvalResult>> = HashMap::new();
    for r in eval_results {
        by_type.entry(r.case_type.clone()).or_default().push(r.clone());
    }
    let by_type_metrics: HashMap<String, AggregateMetrics> = by_type.iter()
        .map(|(k, v)| (k.clone(), compute_aggregate(v)))
        .collect();

    // By PDF
    let mut by_pdf_order: Vec<String> = Vec::new();
    for r in eval_results {
        if !by_pdf_order.contains(&r.pdf) {
            by_pdf_order.push(r.pdf.clone());
        }
    }
    let mut by_pdf_metrics: HashMap<String, PdfGroupMetrics> = HashMap::new();
    for pdf in &by_pdf_order {
        let v: Vec<&EvalResult> = eval_results.iter().filter(|r| r.pdf == *pdf).collect();
        let count = v.len();
        let passed = v.iter().filter(|r| r.passed).count();
        let avg_mrr = if count > 0 { v.iter().map(|r| r.mrr).sum::<f32>() / count as f32 } else { 0.0 };
        let results: Vec<PdfCaseSummary> = v.iter().map(|r| PdfCaseSummary {
            query: r.query.clone(),
            difficulty: r.difficulty.clone(),
            case_type: r.case_type.clone(),
            passed: r.passed,
            mrr: r.mrr,
            rank: r.answer_rank,
        }).collect();
        by_pdf_metrics.insert(pdf.clone(), PdfGroupMetrics { count, passed, avg_mrr, results });
    }

    let output = JsonOutput {
        backend: backend_name.to_string(),
        total_cases: eval_results.len(),
        passed: eval_results.iter().filter(|r| r.passed).count(),
        aggregate,
        by_difficulty: by_difficulty_metrics,
        by_type: by_type_metrics,
        by_pdf: by_pdf_metrics,
        results: eval_results.to_vec(),
    };

    match serde_json::to_string_pretty(&output) {
        Ok(json) => {
            match std::fs::write(json_path, &json) {
                Ok(_) => println!("\nJSON results written to: {json_path}"),
                Err(e) => eprintln!("\nFailed to write JSON: {e}"),
            }
        }
        Err(e) => eprintln!("\nFailed to serialize JSON: {e}"),
    }
}

// == Compare mode: --compare backend1,backend2 =================================

async fn run_compare(eval_path: &str, model_dir: &Path, backend_names: &[&str]) {
    assert!(backend_names.len() == 2, "--compare requires exactly two backends separated by comma");

    let content = std::fs::read_to_string(eval_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read eval file: {e}"); std::process::exit(1); });
    let cases: Vec<EvalCase> = serde_json::from_str(&content)
        .unwrap_or_else(|e| { eprintln!("Invalid eval JSON: {e}"); std::process::exit(1); });

    let name_a = backend_names[0];
    let name_b = backend_names[1];
    let backend_a = select_backend(name_a, model_dir);
    let backend_b = select_backend(name_b, model_dir);

    println!("Comparing backends: {} vs {}", backend_a.name(), backend_b.name());
    println!();

    println!("-- Running backend A: {} --", backend_a.name());
    let results_a = run_eval_cases(&cases, model_dir, backend_a.as_ref(), true, &InferenceMode::Balanced).await;

    println!("\n-- Running backend B: {} --", backend_b.name());
    let results_b = run_eval_cases(&cases, model_dir, backend_b.as_ref(), true, &InferenceMode::Balanced).await;

    let summary_a = compute_summary(&results_a);
    let summary_b = compute_summary(&results_b);

    let col_a_name = backend_a.name();
    let col_b_name = backend_b.name();
    let label_width = results_a.iter()
        .map(|r| r.label.len())
        .max()
        .unwrap_or(4)
        .max("TOTAL MRR".len())
        .max("Case".len()) + 2;
    let col_a_width = col_a_name.len().max(10) + 2;
    let col_b_width = col_b_name.len().max(10) + 2;
    let delta_width = 8;

    print_compare_border('+', '+', '+', label_width, col_a_width, col_b_width, delta_width);
    println!("| {:<lw$}| {:<aw$}| {:<bw$}| {:<dw$}|",
        "Case", col_a_name, col_b_name, "Delta",
        lw = label_width, aw = col_a_width, bw = col_b_width, dw = delta_width);
    print_compare_border('+', '+', '+', label_width, col_a_width, col_b_width, delta_width);

    for (ra, rb) in results_a.iter().zip(results_b.iter()) {
        let cell_a = format_rank_cell(ra);
        let cell_b = format_rank_cell(rb);
        let delta = format_rank_delta(ra, rb);

        println!("| {:<lw$}| {:<aw$}| {:<bw$}| {:<dw$}|",
            ra.label, cell_a, cell_b, delta,
            lw = label_width, aw = col_a_width, bw = col_b_width, dw = delta_width);
    }

    print_compare_border('+', '+', '+', label_width, col_a_width, col_b_width, delta_width);

    let mrr_delta = summary_b.mrr - summary_a.mrr;
    let mrr_delta_str = format_f32_delta(mrr_delta);
    println!("| {:<lw$}| {:<aw$}| {:<bw$}| {:<dw$}|",
        "TOTAL MRR",
        format!("{:.2}", summary_a.mrr),
        format!("{:.2}", summary_b.mrr),
        mrr_delta_str,
        lw = label_width, aw = col_a_width, bw = col_b_width, dw = delta_width);

    let p1_delta = summary_b.p_at_1 - summary_a.p_at_1;
    let p1_delta_str = format_f32_delta(p1_delta);
    println!("| {:<lw$}| {:<aw$}| {:<bw$}| {:<dw$}|",
        "TOTAL P@1",
        format!("{:.2}", summary_a.p_at_1),
        format!("{:.2}", summary_b.p_at_1),
        p1_delta_str,
        lw = label_width, aw = col_a_width, bw = col_b_width, dw = delta_width);

    let recall_delta = summary_b.avg_recall - summary_a.avg_recall;
    let recall_delta_str = format_f32_delta(recall_delta);
    println!("| {:<lw$}| {:<aw$}| {:<bw$}| {:<dw$}|",
        "AVG RECALL",
        format!("{:.2}", summary_a.avg_recall),
        format!("{:.2}", summary_b.avg_recall),
        recall_delta_str,
        lw = label_width, aw = col_a_width, bw = col_b_width, dw = delta_width);

    print_compare_border('+', '+', '+', label_width, col_a_width, col_b_width, delta_width);
}

fn print_compare_border(left: char, mid: char, right: char, lw: usize, aw: usize, bw: usize, dw: usize) {
    println!("{left}{}{mid}{}{mid}{}{mid}{}{right}",
        "-".repeat(lw + 1), "-".repeat(aw + 1), "-".repeat(bw + 1), "-".repeat(dw + 1));
}

fn format_rank_cell(r: &EvalResult) -> String {
    if is_negative_type(&r.case_type) {
        if r.passed { "PASS".to_string() } else { "FAIL".to_string() }
    } else {
        match r.answer_rank {
            Some(1) => "P@1".to_string(),
            Some(rank) => format!("rank {}", rank),
            None => "MISS".to_string(),
        }
    }
}

fn format_rank_delta(a: &EvalResult, b: &EvalResult) -> String {
    if is_negative_type(&a.case_type) {
        match (a.passed, b.passed) {
            (true, true) | (false, false) => "  =".to_string(),
            (false, true) => "+FIX".to_string(),
            (true, false) => "LOST".to_string(),
        }
    } else {
        match (a.answer_rank, b.answer_rank) {
            (Some(ra), Some(rb)) if ra == rb => "  =".to_string(),
            (Some(ra), Some(rb)) => {
                let diff = ra as i32 - rb as i32;
                if diff > 0 {
                    format!("+{}", diff)
                } else {
                    format!("{}", diff)
                }
            }
            (None, Some(_)) => "+NEW".to_string(),
            (Some(_), None) => "LOST".to_string(),
            (None, None) => "  =".to_string(),
        }
    }
}

fn format_f32_delta(d: f32) -> String {
    if d.abs() < 0.005 {
        "  =".to_string()
    } else if d > 0.0 {
        format!("+{:.2}", d)
    } else {
        format!("{:.2}", d)
    }
}

// == Diff mode: --diff baseline.json ===========================================

async fn run_diff(
    eval_path: &str,
    baseline_path: &str,
    model_dir: &Path,
    backend: &dyn RetrievalBackend,
    report_path: Option<&str>,
) {
    let baseline_json = std::fs::read_to_string(baseline_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read baseline file: {e}"); std::process::exit(1); });
    let baseline: EvalReport = serde_json::from_str(&baseline_json)
        .unwrap_or_else(|e| { eprintln!("Invalid baseline JSON: {e}"); std::process::exit(1); });

    let content = std::fs::read_to_string(eval_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read eval file: {e}"); std::process::exit(1); });
    let cases: Vec<EvalCase> = serde_json::from_str(&content)
        .unwrap_or_else(|e| { eprintln!("Invalid eval JSON: {e}"); std::process::exit(1); });

    println!("Baseline: {} (backend: {})", baseline_path, baseline.backend);
    println!("Current:  {} (backend: {})", eval_path, backend.name());
    println!();

    let results = run_eval_cases(&cases, model_dir, backend, true, &InferenceMode::Balanced).await;
    let current_summary = compute_summary(&results);

    let baseline_map: HashMap<String, &CaseReport> = baseline.cases.iter()
        .map(|c| (c.query.clone(), c))
        .collect();

    let mut regressions: Vec<DiffEntry> = Vec::new();
    let mut improvements: Vec<DiffEntry> = Vec::new();
    let mut unchanged: usize = 0;

    for r in &results {
        if let Some(base) = baseline_map.get(&r.query) {
            let was_pass = base.passed;
            let now_pass = r.passed;
            let rank_changed = base.answer_rank != r.answer_rank;
            let mrr_changed = (base.mrr - r.mrr).abs() > 0.005;

            if was_pass && !now_pass {
                regressions.push(DiffEntry {
                    query: r.query.clone(),
                    label: r.label.clone(),
                    old_rank: base.answer_rank,
                    new_rank: r.answer_rank,
                    old_mrr: base.mrr,
                    new_mrr: r.mrr,
                    kind: DiffKind::PassToFail,
                });
            } else if !was_pass && now_pass {
                improvements.push(DiffEntry {
                    query: r.query.clone(),
                    label: r.label.clone(),
                    old_rank: base.answer_rank,
                    new_rank: r.answer_rank,
                    old_mrr: base.mrr,
                    new_mrr: r.mrr,
                    kind: DiffKind::FailToPass,
                });
            } else if rank_changed || mrr_changed {
                let entry = DiffEntry {
                    query: r.query.clone(),
                    label: r.label.clone(),
                    old_rank: base.answer_rank,
                    new_rank: r.answer_rank,
                    old_mrr: base.mrr,
                    new_mrr: r.mrr,
                    kind: DiffKind::RankChange,
                };
                if r.mrr < base.mrr {
                    regressions.push(entry);
                } else {
                    improvements.push(entry);
                }
            } else {
                unchanged += 1;
            }
        }
    }

    if !regressions.is_empty() {
        println!("\nREGRESSIONS ({}):", regressions.len());
        for entry in &regressions {
            print!("  X \"{}\"", entry.query);
            match entry.kind {
                DiffKind::PassToFail => {
                    println!(" -- was PASS, now FAIL");
                }
                DiffKind::RankChange => {
                    println!(" -- was {}, now {} (MRR: {:.2} -> {:.2})",
                        format_rank_opt(entry.old_rank),
                        format_rank_opt(entry.new_rank),
                        entry.old_mrr, entry.new_mrr);
                }
                _ => println!(),
            }
        }
    }

    if !improvements.is_empty() {
        println!("\nIMPROVEMENTS ({}):", improvements.len());
        for entry in &improvements {
            print!("  + \"{}\"", entry.query);
            match entry.kind {
                DiffKind::FailToPass => {
                    println!(" -- was FAIL, now PASS");
                }
                DiffKind::RankChange => {
                    println!(" -- was {}, now {} (MRR: {:.2} -> {:.2})",
                        format_rank_opt(entry.old_rank),
                        format_rank_opt(entry.new_rank),
                        entry.old_mrr, entry.new_mrr);
                }
                _ => println!(),
            }
        }
    }

    if regressions.is_empty() && improvements.is_empty() {
        println!("\nNo changes detected ({} cases unchanged).", unchanged);
    } else {
        println!("\nUnchanged: {unchanged}");
    }

    let bs = &baseline.summary;
    let mrr_diff = current_summary.mrr - bs.mrr;
    let p1_diff = current_summary.p_at_1 - bs.p_at_1;

    let mrr_arrow = if mrr_diff.abs() < 0.005 { "=" }
        else if mrr_diff > 0.0 { "UP" } else { "DOWN" };
    let p1_arrow = if p1_diff.abs() < 0.005 { "=" }
        else if p1_diff > 0.0 { "UP" } else { "DOWN" };

    println!("\nNET: MRR {:.2} -> {:.2} ({} {:.2}) | P@1 {:.2} -> {:.2} ({} {:.2})",
        bs.mrr, current_summary.mrr, mrr_arrow, mrr_diff.abs(),
        bs.p_at_1, current_summary.p_at_1, p1_arrow, p1_diff.abs());

    if let Some(path) = report_path {
        let report = build_report(backend.name(), &results);
        save_report(&report, path);
    }
}

#[derive(Debug)]
enum DiffKind {
    PassToFail,
    FailToPass,
    RankChange,
}

#[derive(Debug)]
struct DiffEntry {
    query: String,
    #[allow(dead_code)]
    label: String,
    old_rank: Option<usize>,
    new_rank: Option<usize>,
    old_mrr: f32,
    new_mrr: f32,
    kind: DiffKind,
}

fn format_rank_opt(rank: Option<usize>) -> String {
    match rank {
        Some(1) => "rank 1".to_string(),
        Some(r) => format!("rank {}", r),
        None => "MISS".to_string(),
    }
}

// == Benchmark modes: --benchmark-modes ========================================

fn mode_label(mode: &InferenceMode) -> &'static str {
    match mode {
        InferenceMode::Quick => "Quick",
        InferenceMode::Balanced => "Balanced",
        InferenceMode::Extended => "Extended",
    }
}

async fn run_benchmark_modes(
    eval_path: &str,
    model_dir: &Path,
    backend: &dyn RetrievalBackend,
    report_path: Option<&str>,
) {
    let content = std::fs::read_to_string(eval_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read eval file: {e}"); std::process::exit(1); });
    let cases: Vec<EvalCase> = serde_json::from_str(&content)
        .unwrap_or_else(|e| { eprintln!("Invalid eval JSON: {e}"); std::process::exit(1); });

    let modes = [InferenceMode::Quick, InferenceMode::Balanced, InferenceMode::Extended];
    let mut all_results: Vec<(&'static str, Vec<EvalResult>, SummaryReport)> = Vec::new();

    for mode in &modes {
        let label = mode_label(mode);
        let retrieval_params = pipeline::RetrievalModeParams::from_mode(mode);
        println!("\n{}", "━".repeat(70));
        println!(" RUNNING MODE: {} (top_k={}, candidate_pool={})",
            label, retrieval_params.top_k, retrieval_params.candidate_pool_k);
        println!("{}", "━".repeat(70));

        let results = run_eval_cases(&cases, model_dir, backend, false, mode).await;
        let summary = compute_summary(&results);
        all_results.push((label, results, summary));
    }

    // ── Summary comparison table ──────────────────────────────────────────────
    println!("\n\n{}", "═".repeat(78));
    println!(" INFERENCE MODE BENCHMARK RESULTS");
    println!(" Backend: {} | Cases: {} | Date: {}",
        backend.name(), cases.len(),
        chrono_timestamp());
    println!("{}", "═".repeat(78));

    // Header
    println!("\n{:<22} {:>10} {:>10} {:>10}", "", "Quick", "Balanced", "Extended");
    println!("{}", "─".repeat(56));

    let q = &all_results[0].2;
    let b = &all_results[1].2;
    let e = &all_results[2].2;

    println!("{:<22} {:>9}% {:>9}% {:>9}%",
        "Pass Rate",
        format!("{:.1}", if q.total > 0 { q.passed as f32 / q.total as f32 * 100.0 } else { 0.0 }),
        format!("{:.1}", if b.total > 0 { b.passed as f32 / b.total as f32 * 100.0 } else { 0.0 }),
        format!("{:.1}", if e.total > 0 { e.passed as f32 / e.total as f32 * 100.0 } else { 0.0 }));

    println!("{:<22} {:>10} {:>10} {:>10}",
        "MRR",
        format!("{:.3}", q.mrr),
        format!("{:.3}", b.mrr),
        format!("{:.3}", e.mrr));

    println!("{:<22} {:>9}% {:>9}% {:>9}%",
        "Precision@1",
        format!("{:.1}", q.p_at_1 * 100.0),
        format!("{:.1}", b.p_at_1 * 100.0),
        format!("{:.1}", e.p_at_1 * 100.0));

    println!("{:<22} {:>9}% {:>9}% {:>9}%",
        "Avg Recall",
        format!("{:.1}", q.avg_recall * 100.0),
        format!("{:.1}", b.avg_recall * 100.0),
        format!("{:.1}", e.avg_recall * 100.0));

    println!("{:<22} {:>9}% {:>9}% {:>9}%",
        "Avg Partial Score",
        format!("{:.1}", q.avg_partial_score * 100.0),
        format!("{:.1}", b.avg_partial_score * 100.0),
        format!("{:.1}", e.avg_partial_score * 100.0));

    println!("{:<22} {:>10} {:>10} {:>10}",
        "Passed / Total",
        format!("{}/{}", q.passed, q.total),
        format!("{}/{}", b.passed, b.total),
        format!("{}/{}", e.passed, e.total));

    // ── By difficulty breakdown ───────────────────────────────────────────────
    println!("\n{}", "─".repeat(78));
    println!(" BY DIFFICULTY");
    println!("{}", "─".repeat(78));
    println!("{:<22} {:>10} {:>10} {:>10}", "", "Quick", "Balanced", "Extended");
    println!("{}", "─".repeat(56));

    for diff in &["easy", "medium", "hard"] {
        let metrics: Vec<AggregateMetrics> = all_results.iter().map(|(_, results, _)| {
            let subset: Vec<EvalResult> = results.iter()
                .filter(|r| r.difficulty == *diff)
                .cloned()
                .collect();
            compute_aggregate(&subset)
        }).collect();

        if metrics.iter().all(|m| m.count == 0) { continue; }

        println!("{:<22} {:>10} {:>10} {:>10}",
            format!("{} (pass rate)", diff),
            format!("{}/{}", metrics[0].passed, metrics[0].count),
            format!("{}/{}", metrics[1].passed, metrics[1].count),
            format!("{}/{}", metrics[2].passed, metrics[2].count));

        println!("{:<22} {:>10} {:>10} {:>10}",
            format!("{} (MRR)", diff),
            format!("{:.3}", metrics[0].avg_mrr),
            format!("{:.3}", metrics[1].avg_mrr),
            format!("{:.3}", metrics[2].avg_mrr));
    }

    // ── By case type breakdown ────────────────────────────────────────────────
    println!("\n{}", "─".repeat(78));
    println!(" BY CASE TYPE");
    println!("{}", "─".repeat(78));
    println!("{:<22} {:>10} {:>10} {:>10}", "", "Quick", "Balanced", "Extended");
    println!("{}", "─".repeat(56));

    let mut all_types: Vec<String> = all_results[0].1.iter()
        .map(|r| r.case_type.clone()).collect();
    all_types.sort();
    all_types.dedup();

    for t in &all_types {
        let metrics: Vec<AggregateMetrics> = all_results.iter().map(|(_, results, _)| {
            let subset: Vec<EvalResult> = results.iter()
                .filter(|r| r.case_type == *t)
                .cloned()
                .collect();
            compute_aggregate(&subset)
        }).collect();

        println!("{:<22} {:>10} {:>10} {:>10}",
            t,
            format!("{}/{}", metrics[0].passed, metrics[0].count),
            format!("{}/{}", metrics[1].passed, metrics[1].count),
            format!("{}/{}", metrics[2].passed, metrics[2].count));
    }

    // ── By practice area (PDF) breakdown ──────────────────────────────────────
    println!("\n{}", "─".repeat(78));
    println!(" BY PRACTICE AREA");
    println!("{}", "─".repeat(78));
    println!("{:<22} {:>10} {:>10} {:>10}", "", "Quick", "Balanced", "Extended");
    println!("{}", "─".repeat(56));

    let mut pdf_order: Vec<String> = Vec::new();
    for r in &all_results[0].1 {
        let short = std::path::Path::new(&r.pdf)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| r.pdf.clone());
        if !pdf_order.contains(&short) {
            pdf_order.push(short);
        }
    }

    for pdf_stem in &pdf_order {
        let metrics: Vec<AggregateMetrics> = all_results.iter().map(|(_, results, _)| {
            let subset: Vec<EvalResult> = results.iter()
                .filter(|r| {
                    let stem = std::path::Path::new(&r.pdf)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    stem == *pdf_stem
                })
                .cloned()
                .collect();
            compute_aggregate(&subset)
        }).collect();

        println!("{:<22} {:>10} {:>10} {:>10}",
            pdf_stem,
            format!("{}/{} {:.0}%", metrics[0].passed, metrics[0].count, metrics[0].avg_mrr * 100.0),
            format!("{}/{} {:.0}%", metrics[1].passed, metrics[1].count, metrics[1].avg_mrr * 100.0),
            format!("{}/{} {:.0}%", metrics[2].passed, metrics[2].count, metrics[2].avg_mrr * 100.0));
    }

    // ── Per-case detail (cases that differ across modes) ──────────────────────
    println!("\n{}", "─".repeat(78));
    println!(" CASES WITH MODE-DEPENDENT RESULTS");
    println!("{}", "─".repeat(78));

    let mut any_diff = false;
    for i in 0..all_results[0].1.len() {
        let rq = &all_results[0].1[i];
        let rb = &all_results[1].1[i];
        let re = &all_results[2].1[i];

        if rq.passed != rb.passed || rb.passed != re.passed
            || rq.answer_rank != rb.answer_rank || rb.answer_rank != re.answer_rank
        {
            any_diff = true;
            let q_cell = format_rank_cell(rq);
            let b_cell = format_rank_cell(rb);
            let e_cell = format_rank_cell(re);
            println!("  {:<40} Q:{:<6} B:{:<6} E:{:<6}",
                &rq.label[..rq.label.len().min(40)], q_cell, b_cell, e_cell);
        }
    }
    if !any_diff {
        println!("  (all cases produced identical results across modes)");
    }

    // ── Save report if requested ──────────────────────────────────────────────
    if let Some(path) = report_path {
        // Save extended report with all three modes
        let mut report_data = serde_json::Map::new();
        for (label, results, _summary) in &all_results {
            let mode_report = build_report(&format!("{}-{}", backend.name(), label.to_lowercase()), results);
            report_data.insert(label.to_lowercase().to_string(),
                serde_json::to_value(&mode_report).unwrap_or_default());
        }
        let json = serde_json::to_string_pretty(&report_data)
            .unwrap_or_else(|e| { eprintln!("Serialize error: {e}"); "{}".to_string() });
        std::fs::write(path, json)
            .unwrap_or_else(|e| { eprintln!("Write error: {e}"); });
        println!("\nBenchmark report saved to {path}");
    }

    println!("\n{}", "═".repeat(78));
}

fn chrono_timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Simple date: just show epoch for now
    format!("{}", secs)
}

fn parse_mode(s: &str) -> InferenceMode {
    match s.to_lowercase().as_str() {
        "quick" => InferenceMode::Quick,
        "balanced" => InferenceMode::Balanced,
        "extended" => InferenceMode::Extended,
        other => {
            eprintln!("Unknown mode: '{}'. Available: quick, balanced, extended", other);
            std::process::exit(1);
        }
    }
}

// == Interactive mode ==========================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut pdf_path: Option<String> = None;
    let mut query_text: Option<String> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut skip_llm = false;
    let mut eval_path: Option<String> = None;
    let mut backend_name = "hybrid-bm25-cosine".to_string();
    let mut compare_arg: Option<String> = None;
    let mut diff_path: Option<String> = None;
    let mut report_path: Option<String> = None;
    let mut json_out: Option<String> = None;
    let mut mode_arg: Option<String> = None;
    let mut benchmark_modes = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pdf" => { i += 1; pdf_path = Some(args[i].clone()); }
            "--query" => { i += 1; query_text = Some(args[i].clone()); }
            "--data-dir" => { i += 1; data_dir = Some(PathBuf::from(&args[i])); }
            "--skip-llm" => { skip_llm = true; }
            "--eval" => { i += 1; eval_path = Some(args[i].clone()); }
            "--backend" => { i += 1; backend_name = args[i].clone(); }
            "--compare" => { i += 1; compare_arg = Some(args[i].clone()); }
            "--diff" => { i += 1; diff_path = Some(args[i].clone()); }
            "--report" => { i += 1; report_path = Some(args[i].clone()); }
            "--json-out" => { i += 1; json_out = Some(args[i].clone()); }
            "--mode" => { i += 1; mode_arg = Some(args[i].clone()); }
            "--benchmark-modes" => { benchmark_modes = true; }
            _ => { eprintln!("Unknown argument: {}", args[i]); }
        }
        i += 1;
    }

    let data_dir = data_dir.unwrap_or_else(|| {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("com.justiceai.app")
        }
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| r"C:\temp".into());
            PathBuf::from(appdata).join("com.justiceai.app")
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let data_home = std::env::var("XDG_DATA_HOME")
                .unwrap_or_else(|_| format!("{home}/.local/share"));
            PathBuf::from(data_home).join("com.justiceai.app")
        }
    });
    let model_dir = data_dir.join("models");

    let mode = mode_arg.as_deref().map(parse_mode).unwrap_or(InferenceMode::Balanced);

    // -- Benchmark modes: run all three modes and compare ---------------------
    if benchmark_modes {
        let eval_file = eval_path.unwrap_or_else(|| {
            eprintln!("--benchmark-modes requires --eval <eval.json>");
            std::process::exit(1);
        });
        let backend = select_backend(&backend_name, &model_dir);
        run_benchmark_modes(&eval_file, &model_dir, backend.as_ref(), report_path.as_deref()).await;
        return;
    }

    // -- Compare mode ---------------------------------------------------------
    if let Some(ref compare) = compare_arg {
        let eval_file = eval_path.unwrap_or_else(|| {
            eprintln!("--compare requires --eval <eval.json>");
            std::process::exit(1);
        });
        let backends: Vec<&str> = compare.split(',').collect();
        if backends.len() != 2 {
            eprintln!("--compare requires exactly two backends separated by comma, e.g. --compare hybrid,reranker");
            std::process::exit(1);
        }
        run_compare(&eval_file, &model_dir, &backends).await;
        return;
    }

    // -- Diff mode ------------------------------------------------------------
    if let Some(ref baseline) = diff_path {
        let eval_file = eval_path.unwrap_or_else(|| {
            eprintln!("--diff requires --eval <eval.json>");
            std::process::exit(1);
        });
        let backend = select_backend(&backend_name, &model_dir);
        run_diff(&eval_file, baseline, &model_dir, backend.as_ref(), report_path.as_deref()).await;
        return;
    }

    // -- Eval mode ------------------------------------------------------------
    if let Some(eval_file) = eval_path {
        let backend = select_backend(&backend_name, &model_dir);
        run_eval(&eval_file, &model_dir, backend.as_ref(), report_path.as_deref(), json_out.as_deref(), &mode).await;
        return;
    }

    // -- Interactive mode -----------------------------------------------------
    let backend = select_backend(&backend_name, &model_dir);
    let pdf_path = pdf_path.unwrap_or_else(|| {
        eprintln!("Usage: harness --pdf <path> --query <text> [--data-dir <path>] [--skip-llm] [--backend <name>]");
        eprintln!("       harness --eval <eval.json> [--data-dir <path>] [--backend <name>] [--report out.json] [--json-out <path>]");
        eprintln!("       harness --eval <eval.json> --compare backend1,backend2");
        eprintln!("       harness --eval <eval.json> --diff baseline.json [--report out.json]");
        std::process::exit(1);
    });
    let query = query_text.unwrap_or_else(|| {
        eprintln!("Usage: harness --pdf <path> --query <text>");
        std::process::exit(1);
    });

    let settings = AppSettings::default();
    println!("Backend: {}", backend.name());

    // -- EXTRACTION + CHUNKING ------------------------------------------------
    print_banner("EXTRACTION");
    let (pages, chunks) = match parse_and_chunk(&pdf_path, &settings) {
        Ok(r) => r,
        Err(e) => { eprintln!("Parse error: {e}"); std::process::exit(1); }
    };

    println!("Pages: {}", pages.len());
    for page in pages.iter().take(3) {
        let preview: String = page.text.chars().take(500).collect();
        println!("\n--- Page {} ({} chars) ---", page.page_number, page.text.len());
        println!("{preview}");
        if page.text.len() > 500 { println!("..."); }
    }

    print_banner("CHUNKING");
    println!("Total chunks: {}", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        let preview: String = chunk.text.chars().take(200).collect();
        println!(
            "\n[Chunk {}] page={} tokens={} len={}",
            i, chunk.page_number, chunk.token_count, chunk.text.len()
        );
        println!("{preview}");
        if chunk.text.len() > 200 { println!("..."); }
    }

    // -- RETRIEVAL ------------------------------------------------------------
    print_banner("RETRIEVAL (via backend)");

    let config = RetrievalConfig {
        top_k: settings.top_k,
        candidate_pool_k: (settings.top_k * 6).min(60),
        score_threshold: 0.0,
        mmr_lambda: 0.7,
        expand_keywords: true,
        ..Default::default()
    };

    let scored = match embed_and_retrieve(&chunks, &query, &pdf_path, &model_dir, backend.as_ref(), &config).await {
        Ok(s) => s,
        Err(e) => { eprintln!("Retrieval error: {e}"); std::process::exit(1); }
    };

    for (rank, (score, meta, _)) in scored.iter().enumerate() {
        let preview: String = meta.text.chars().take(150).collect();
        println!(
            "\n[#{} score={:.4}] page={} chunk={}",
            rank + 1, score, meta.page_number, meta.chunk_index
        );
        println!("{preview}");
        if meta.text.len() > 150 { println!("..."); }
    }

    // -- CONTEXT ASSEMBLY -----------------------------------------------------
    let context_parts: Vec<String> = scored
        .iter()
        .enumerate()
        .map(|(i, (_, meta, _))| {
            format!(
                "SOURCE {} -- {}, Page {}:\n\"{}\"",
                i + 1, meta.file_name, meta.page_number, meta.text
            )
        })
        .collect();
    let context = context_parts.join("\n\n---\n\n");

    print_banner("FINAL CONTEXT (sent to LLM)");
    let ctx_preview: String = context.chars().take(3000).collect();
    println!("{ctx_preview}");
    if context.len() > 3000 { println!("\n... [truncated, {} total chars]", context.len()); }

    // -- LLM INFERENCE --------------------------------------------------------
    let gguf_path = model_dir.join("qwen3.gguf");
    if skip_llm {
        println!("\n[--skip-llm flag set, skipping LLM]");
        return;
    }
    if !gguf_path.exists() || gguf_path.metadata().map(|m| m.len()).unwrap_or(0) < pipeline::GGUF_MIN_SIZE {
        println!("\nSkipping LLM (model not found at {})", gguf_path.display());
        return;
    }

    print_banner("LLM RESPONSE");
    println!("Running ask_llm...\n");

    let model_cache: Arc<Mutex<Option<llama_cpp_2::model::LlamaModel>>> =
        Arc::new(Mutex::new(None));
    let history: Vec<(String, String)> = Vec::new();

    let inference_params = pipeline::InferenceParams::from_mode(&InferenceMode::Balanced);
    match pipeline::ask_llm(&query, &context, &history, &model_dir, model_cache, |tok| {
        print!("{tok}");
        use std::io::Write;
        std::io::stdout().flush().ok();
    }, None, inference_params).await {
        Ok(answer) => { println!("\n\n--- Final answer ---\n{answer}"); }
        Err(e) => { eprintln!("LLM error: {e}"); }
    }
}
