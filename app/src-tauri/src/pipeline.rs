//! Core RAG pipeline logic — no Tauri dependencies.
//!
//! This module contains all pure pipeline functions extracted from `commands/rag.rs`:
//! embedding, chunking, LLM inference, and retrieval helpers. Tauri command handlers
//! remain in `commands/rag.rs` and call into this module.

use crate::state::{AppSettings, ChunkMetadata, DocumentPage, InferenceMode, Jurisdiction, JurisdictionLevel, RagState};
use llama_cpp_2::llama_backend::LlamaBackend;
use regex::Regex;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const SCORE_THRESHOLD: f32 = 0.20;
pub const GGUF_MIN_SIZE: u64 = 4_000_000_000;

pub const SAUL_GGUF_URL: &str = "https://huggingface.co/MaziyarPanahi/Saul-Instruct-v1-GGUF/resolve/main/Saul-Instruct-v1.Q4_K_M.gguf";

/// Rules-only system prompt — document context goes in the user turn so Llama 2
/// pays full attention to it (system-prompt content is under-weighted by the model).
pub const RULES_PROMPT: &str = "\
You are Justice AI, a legal research assistant specializing in US federal and state law.\n\n\
Rules (follow exactly):\n\
- Cite every factual claim inline as [filename, p. N] immediately after the claim — never group citations at the end.\n\
- State all numbers, dates, dollar amounts, and figures EXACTLY as written in the source — never round or paraphrase.\n\
- Form fields: PDFs may store template labels (e.g. \"Event Date: ______\") and filled values in separate SOURCE chunks. \
Match each value to its nearest field label across all SOURCE chunks before answering. \
Never report a bare value without identifying which label it belongs to.\n\
- Multiple dates: when several dates appear, use the field label (e.g. \"Event Date\", \"Signature Date\", \"Date of Birth\") \
to determine which date answers the question. Prefer the labeled match over proximity in the text.\n\
- State each fact once only. Do not restart or repeat a list you have already written.\n\
- If the answer is not present in the excerpts, say exactly: \"I could not find information about this in your loaded documents.\"\n\
- When no excerpts are provided, answer from your knowledge of US law; note when answers may vary by state or when a licensed attorney should be consulted.\n\
- Never fabricate case citations, statutes, or facts. Do not give specific legal advice.\n\n\
Format:\n\
- Begin with a direct one-sentence answer, then elaborate.\n\
- Use **bold** for key legal terms, parties, dates, and dollar amounts.\n\
- Use bullet points (- ) for lists of multiple items or findings.\n\
- Use ### headers only to separate distinct topics in longer answers.\n\
- Keep paragraphs to 2-3 sentences.\n\
- No pleasantries, preambles, or sign-offs.";

// ── Inference Mode Params ────────────────────────────────────────────────────

pub struct InferenceParams {
    pub max_new_tokens: usize,
    pub temperature: f32,
    pub system_prompt_suffix: &'static str,
}

impl InferenceParams {
    /// Context window budget notes (Saul-7B: n_ctx = 4096):
    ///   prompt_tokens ≈ (sys_prompt + context + question + overhead) / 2.5
    ///   gen_tokens = 4096 - prompt_tokens
    /// Quick:    ~2000 prompt → ~2000 gen headroom (256 used)
    /// Balanced: ~2900 prompt → ~1200 gen headroom (1024 used)
    /// Extended: ~2900 prompt → ~1200 gen headroom (1024 used, but 10 sources)
    /// A runtime cap in ask_saul ensures we never overshoot.
    pub fn from_mode(mode: &InferenceMode) -> Self {
        match mode {
            InferenceMode::Quick => Self {
                max_new_tokens: 256,
                temperature: 0.3,
                system_prompt_suffix: "\nAnswer in 2-3 concise bullet points. Focus on the single most relevant fact. Be brief.",
            },
            InferenceMode::Balanced => Self {
                max_new_tokens: 1024,
                temperature: 0.35,
                system_prompt_suffix: "",
            },
            InferenceMode::Extended => Self {
                max_new_tokens: 1024,
                temperature: 0.2,
                system_prompt_suffix: "\nThink step by step. Cite each claim with page number. Provide comprehensive analysis.",
            },
        }
    }
}

pub struct RetrievalModeParams {
    pub top_k: usize,
    pub candidate_pool_k: usize,
    pub max_context_chars_jur: usize,
    pub max_context_chars_no_jur: usize,
}

impl RetrievalModeParams {
    /// Budget = (4096 - max_new_tokens - ~600 sys/overhead) * 2.5 chars/token.
    /// Quick:    (4096-256-600)*2.5 ≈ 8100 → use 3200/3500 (fast, intentionally small)
    /// Balanced: (4096-1024-600)*2.5 ≈ 6180 → use 5800/6200
    /// Extended: (4096-1024-600)*2.5 ≈ 6180 → use 5800/6200 (same gen budget, more sources)
    pub fn from_mode(mode: &InferenceMode) -> Self {
        match mode {
            InferenceMode::Quick => Self {
                top_k: 3,
                candidate_pool_k: 30,
                max_context_chars_jur: 3_200,
                max_context_chars_no_jur: 3_500,
            },
            InferenceMode::Balanced => Self {
                top_k: 6,
                candidate_pool_k: 60,
                max_context_chars_jur: 5_800,
                max_context_chars_no_jur: 6_200,
            },
            InferenceMode::Extended => Self {
                top_k: 10,
                candidate_pool_k: 80,
                max_context_chars_jur: 5_800,
                max_context_chars_no_jur: 6_200,
            },
        }
    }
}

// ── Jurisdiction Detection ────────────────────────────────────────────────────

pub struct DetectionResult {
    pub jurisdiction: Jurisdiction,
    pub confidence: f32,
    pub signal: String,
}

struct JurisdictionPattern {
    regex: Regex,
    level: JurisdictionLevel,
    state: Option<&'static str>,
    weight: f32,
    signal: &'static str,
}

fn jurisdiction_patterns() -> &'static Vec<JurisdictionPattern> {
    static PATTERNS: OnceLock<Vec<JurisdictionPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let mut patterns = Vec::new();

        // Federal statute citations (weight 0.4)
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"\d+\s+U\.?S\.?C\.?\s*§").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.4, signal: "Federal statute citation (U.S.C.)",
        });
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"\d+\s+C\.?F\.?R\.?\s*§").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.35, signal: "Federal regulation citation (C.F.R.)",
        });

        // Federal court names (weight 0.35)
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"(?i)U\.?S\.?\s+District\s+Court").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.35, signal: "U.S. District Court",
        });
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"(?i)United\s+States\s+Bankruptcy\s+Court").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.35, signal: "U.S. Bankruptcy Court",
        });
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"(?i)Supreme\s+Court\s+of\s+the\s+United\s+States").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.4, signal: "Supreme Court of the United States",
        });

        // Federal agencies (weight 0.1)
        patterns.push(JurisdictionPattern {
            regex: Regex::new(r"(?i)\b(EEOC|SEC|FTC|EPA|NLRB|OSHA|IRS|DOJ|FBI|ATF|DEA|HHS|HUD|DOL|CFPB)\b").unwrap(),
            level: JurisdictionLevel::Federal,
            state: None, weight: 0.1, signal: "Federal agency reference",
        });

        // State statute citations (weight 0.4)
        let state_statutes: Vec<(&str, &str, &str)> = vec![
            ("Alabama", r"(?i)Ala\.\s+Code\s*§", "Alabama Code citation"),
            ("Alaska", r"(?i)Alaska\s+Stat\.\s*§", "Alaska Statute citation"),
            ("Arizona", r"(?i)Ariz\.\s+Rev\.\s+Stat\.\s*§", "Arizona Revised Statute citation"),
            ("Arkansas", r"(?i)Ark\.\s+Code\s+Ann\.\s*§", "Arkansas Code citation"),
            ("California", r"(?i)Cal\.\s+\w+\.?\s+Code\s*§", "California Code citation"),
            ("Colorado", r"(?i)Colo\.\s+Rev\.\s+Stat\.\s*§", "Colorado Revised Statute citation"),
            ("Connecticut", r"(?i)Conn\.\s+Gen\.\s+Stat\.\s*§", "Connecticut General Statute citation"),
            ("Delaware", r"(?i)Del\.\s+Code\s+(Ann\.\s+)?tit\.", "Delaware Code citation"),
            ("Florida", r"(?i)Fla\.\s+Stat\.\s*§", "Florida Statute citation"),
            ("Georgia", r"(?i)Ga\.\s+Code\s+Ann\.\s*§", "Georgia Code citation"),
            ("Hawaii", r"(?i)Haw\.\s+Rev\.\s+Stat\.\s*§", "Hawaii Revised Statute citation"),
            ("Idaho", r"(?i)Idaho\s+Code\s*§", "Idaho Code citation"),
            ("Illinois", r"(?i)\d+\s+ILCS\s+\d+", "Illinois Compiled Statute citation"),
            ("Indiana", r"(?i)Ind\.\s+Code\s*§", "Indiana Code citation"),
            ("Iowa", r"(?i)Iowa\s+Code\s*§", "Iowa Code citation"),
            ("Kansas", r"(?i)Kan\.\s+Stat\.\s+Ann\.\s*§", "Kansas Statute citation"),
            ("Kentucky", r"(?i)Ky\.\s+Rev\.\s+Stat\.\s+Ann\.\s*§", "Kentucky Revised Statute citation"),
            ("Louisiana", r"(?i)La\.\s+(Rev\.\s+Stat\.|Civ\.\s+Code)\s*(Ann\.\s*)?§?", "Louisiana statute citation"),
            ("Maine", r"(?i)Me\.\s+Rev\.\s+Stat\.\s+(Ann\.\s+)?tit\.", "Maine Revised Statute citation"),
            ("Maryland", r"(?i)Md\.\s+Code\s+Ann\.", "Maryland Code citation"),
            ("Massachusetts", r"(?i)Mass\.\s+Gen\.\s+Laws\s+ch\.", "Massachusetts General Laws citation"),
            ("Michigan", r"(?i)Mich\.\s+Comp\.\s+Laws\s*§", "Michigan Compiled Laws citation"),
            ("Minnesota", r"(?i)Minn\.\s+Stat\.\s*§", "Minnesota Statute citation"),
            ("Mississippi", r"(?i)Miss\.\s+Code\s+Ann\.\s*§", "Mississippi Code citation"),
            ("Missouri", r"(?i)Mo\.\s+(Ann\.\s+)?Stat\.\s*§", "Missouri Statute citation"),
            ("Montana", r"(?i)Mont\.\s+Code\s+Ann\.\s*§", "Montana Code citation"),
            ("Nebraska", r"(?i)Neb\.\s+Rev\.\s+Stat\.\s*§", "Nebraska Revised Statute citation"),
            ("Nevada", r"(?i)Nev\.\s+Rev\.\s+Stat\.\s*§", "Nevada Revised Statute citation"),
            ("New Hampshire", r"(?i)N\.H\.\s+Rev\.\s+Stat\.\s+Ann\.\s*§", "New Hampshire Revised Statute citation"),
            ("New Jersey", r"(?i)N\.J\.\s+Stat\.\s+Ann\.\s*§", "New Jersey Statute citation"),
            ("New Mexico", r"(?i)N\.M\.\s+Stat\.\s+Ann\.\s*§", "New Mexico Statute citation"),
            ("New York", r"(?i)N\.Y\.\s+[\w.]+(\s+[\w.&]+)*\s+Law\s*§", "New York Law citation"),
            ("North Carolina", r"(?i)N\.C\.\s+Gen\.\s+Stat\.\s*§", "North Carolina General Statute citation"),
            ("North Dakota", r"(?i)N\.D\.\s+Cent\.\s+Code\s*§", "North Dakota Century Code citation"),
            ("Ohio", r"(?i)Ohio\s+Rev\.\s+Code\s+(Ann\.\s*)?§", "Ohio Revised Code citation"),
            ("Oklahoma", r"(?i)Okla\.\s+Stat\.\s+tit\.", "Oklahoma Statute citation"),
            ("Oregon", r"(?i)Or\.\s+Rev\.\s+Stat\.\s*§", "Oregon Revised Statute citation"),
            ("Pennsylvania", r"(?i)\d+\s+Pa\.\s+Cons\.\s+Stat\.\s*§", "Pennsylvania Consolidated Statute citation"),
            ("Rhode Island", r"(?i)R\.I\.\s+Gen\.\s+Laws\s*§", "Rhode Island General Laws citation"),
            ("South Carolina", r"(?i)S\.C\.\s+Code\s+Ann\.\s*§", "South Carolina Code citation"),
            ("South Dakota", r"(?i)S\.D\.\s+Codified\s+Laws\s*§", "South Dakota Codified Laws citation"),
            ("Tennessee", r"(?i)Tenn\.\s+Code\s+Ann\.\s*§", "Tennessee Code citation"),
            ("Texas", r"(?i)Tex\.\s+\w+\.?\s+(&\s+\w+\.\s+)?Code\s*(Ann\.\s*)?§", "Texas Code citation"),
            ("Utah", r"(?i)Utah\s+Code\s+Ann\.\s*§", "Utah Code citation"),
            ("Vermont", r"(?i)Vt\.\s+Stat\.\s+Ann\.\s+tit\.", "Vermont Statute citation"),
            ("Virginia", r"(?i)Va\.\s+Code\s+Ann\.\s*§", "Virginia Code citation"),
            ("Washington", r"(?i)Wash\.\s+Rev\.\s+Code\s*§", "Washington Revised Code citation"),
            ("West Virginia", r"(?i)W\.\s*Va\.\s+Code\s*§", "West Virginia Code citation"),
            ("Wisconsin", r"(?i)Wis\.\s+Stat\.\s*§", "Wisconsin Statute citation"),
            ("Wyoming", r"(?i)Wyo\.\s+Stat\.\s+Ann\.\s*§", "Wyoming Statute citation"),
        ];
        for (state, re, signal) in state_statutes {
            if let Ok(regex) = Regex::new(re) {
                patterns.push(JurisdictionPattern {
                    regex, level: JurisdictionLevel::State,
                    state: Some(state), weight: 0.4, signal,
                });
            }
        }

        // State court names (weight 0.35)
        let state_courts: Vec<(&str, &str, &str)> = vec![
            ("California", r"(?i)Superior\s+Court\s+of\s+(the\s+State\s+of\s+)?California", "California Superior Court"),
            ("California", r"(?i)Court\s+of\s+Appeal.*California", "California Court of Appeal"),
            ("New York", r"(?i)Supreme\s+Court\s+of\s+the\s+State\s+of\s+New\s+York", "New York Supreme Court"),
            ("New York", r"(?i)New\s+York\s+Supreme\s+Court", "New York Supreme Court"),
            ("Texas", r"(?i)District\s+Court\s+of\s+.*Texas", "Texas District Court"),
            ("Texas", r"(?i)Texas\s+Court\s+of\s+(Criminal\s+)?Appeals", "Texas Court of Appeals"),
            ("Florida", r"(?i)Circuit\s+Court.*Florida", "Florida Circuit Court"),
            ("Illinois", r"(?i)Circuit\s+Court\s+of\s+.*Illinois", "Illinois Circuit Court"),
            ("Pennsylvania", r"(?i)Court\s+of\s+Common\s+Pleas.*Pennsylvania", "Pennsylvania Court of Common Pleas"),
            ("Ohio", r"(?i)Court\s+of\s+Common\s+Pleas.*Ohio", "Ohio Court of Common Pleas"),
            ("Georgia", r"(?i)Superior\s+Court\s+of\s+.*Georgia", "Georgia Superior Court"),
            ("Michigan", r"(?i)Circuit\s+Court.*Michigan", "Michigan Circuit Court"),
            ("New Jersey", r"(?i)Superior\s+Court\s+of\s+New\s+Jersey", "New Jersey Superior Court"),
            ("Virginia", r"(?i)Circuit\s+Court\s+of\s+.*Virginia", "Virginia Circuit Court"),
            ("Massachusetts", r"(?i)(Superior|District)\s+Court.*Massachusetts", "Massachusetts Court"),
            ("Washington", r"(?i)Superior\s+Court\s+of\s+.*Washington", "Washington Superior Court"),
            ("Colorado", r"(?i)District\s+Court.*Colorado", "Colorado District Court"),
            ("Arizona", r"(?i)Superior\s+Court\s+of\s+.*Arizona", "Arizona Superior Court"),
            ("Maryland", r"(?i)Circuit\s+Court\s+.*Maryland", "Maryland Circuit Court"),
            ("North Carolina", r"(?i)Superior\s+Court\s+of\s+.*North\s+Carolina", "North Carolina Superior Court"),
        ];
        for (state, re, signal) in state_courts {
            if let Ok(regex) = Regex::new(re) {
                patterns.push(JurisdictionPattern {
                    regex, level: JurisdictionLevel::State,
                    state: Some(state), weight: 0.35, signal,
                });
            }
        }

        // State in headers (weight 0.15) — matches "State of [X]" in first part of text
        let states_list = [
            "Alabama", "Alaska", "Arizona", "Arkansas", "California", "Colorado",
            "Connecticut", "Delaware", "Florida", "Georgia", "Hawaii", "Idaho",
            "Illinois", "Indiana", "Iowa", "Kansas", "Kentucky", "Louisiana",
            "Maine", "Maryland", "Massachusetts", "Michigan", "Minnesota",
            "Mississippi", "Missouri", "Montana", "Nebraska", "Nevada",
            "New Hampshire", "New Jersey", "New Mexico", "New York",
            "North Carolina", "North Dakota", "Ohio", "Oklahoma", "Oregon",
            "Pennsylvania", "Rhode Island", "South Carolina", "South Dakota",
            "Tennessee", "Texas", "Utah", "Vermont", "Virginia", "Washington",
            "West Virginia", "Wisconsin", "Wyoming",
        ];
        for state_name in &states_list {
            let re = format!(r"(?i)State\s+of\s+{}", regex::escape(state_name));
            if let Ok(regex) = Regex::new(&re) {
                patterns.push(JurisdictionPattern {
                    regex, level: JurisdictionLevel::State,
                    state: Some(state_name), weight: 0.15,
                    signal: "State name in document header",
                });
            }
        }

        patterns
    })
}

/// County extraction regex — matches "[Name] County" patterns.
fn county_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\s+County").unwrap())
}

/// Detect jurisdiction from document text. Scans for legal citations, court names,
/// and state references. Returns the highest-confidence match or None.
pub fn detect_jurisdiction(text: &str) -> Option<DetectionResult> {
    // Only scan the first 10,000 chars for performance
    let scan_text = if text.len() > 10_000 { &text[..10_000] } else { text };

    let patterns = jurisdiction_patterns();

    // Score each match
    let mut best: Option<DetectionResult> = None;

    for pat in patterns.iter() {
        if pat.regex.is_match(scan_text) {
            let confidence = pat.weight;
            if confidence >= 0.3 || best.is_none() {
                let is_better = match &best {
                    Some(b) => confidence > b.confidence,
                    None => true,
                };
                if is_better {
                    // Try to extract county if state-level
                    let county = if pat.level == JurisdictionLevel::State {
                        county_regex()
                            .captures(scan_text)
                            .map(|c| format!("{} County", &c[1]))
                    } else {
                        None
                    };

                    let level = if county.is_some() {
                        JurisdictionLevel::County
                    } else {
                        pat.level.clone()
                    };

                    best = Some(DetectionResult {
                        jurisdiction: Jurisdiction {
                            level,
                            state: pat.state.map(String::from),
                            county,
                        },
                        confidence,
                        signal: pat.signal.to_string(),
                    });
                }
            }
        }
    }

    // Only return if confidence threshold met
    best.filter(|r| r.confidence >= 0.3)
}

/// Generate a prompt fragment with jurisdiction-specific rules for the LLM.
pub fn jurisdiction_prompt_fragment(j: &Jurisdiction) -> String {
    match j.level {
        JurisdictionLevel::Federal => {
            "Jurisdiction: Federal law applies.\n\
             - Federal courts apply federal procedural law. In diversity cases, apply state substantive law (Erie doctrine).\n\
             - Cite federal statutes as [Title] U.S.C. § [Section] and regulations as [Title] C.F.R. § [Section].\n\
             - If documents reference both federal and state law, explain which applies and why.".to_string()
        }
        JurisdictionLevel::State => {
            let state_name = j.state.as_deref().unwrap_or("the relevant state");
            let state_specific = match state_name {
                "California" => "\n- California can be MORE restrictive than federal in: minimum wage, environmental rules, consumer protection, data privacy (CCPA/CPRA), tenant rights.\n\
                                 - Cite California statutes as Cal. [Code Name] Code § [Section].",
                "New York" => "\n- New York can be MORE restrictive than federal in: consumer protection, employment law, rent stabilization, financial regulation.\n\
                               - Cite New York statutes as N.Y. [Law Name] Law § [Section].",
                "Texas" => "\n- Texas follows federal minimum standards in most areas. Notable exceptions: strong property rights, community property rules, specific oil & gas law.\n\
                            - Cite Texas statutes as Tex. [Code Name] Code § [Section].",
                "Florida" => "\n- Florida has no state income tax. Notably stronger homestead protections than most states.\n\
                              - Cite Florida statutes as Fla. Stat. § [Section].",
                "Illinois" => "\n- Illinois can be MORE restrictive than federal in: biometric privacy (BIPA), consumer fraud, employment law.\n\
                               - Cite Illinois statutes as [Chapter] ILCS [Act]/[Section].",
                "Pennsylvania" => "\n- Cite Pennsylvania statutes as [Title] Pa. Cons. Stat. § [Section].",
                "Ohio" => "\n- Cite Ohio statutes as Ohio Rev. Code § [Section].",
                "Georgia" => "\n- Cite Georgia statutes as Ga. Code Ann. § [Section].",
                "New Jersey" => "\n- New Jersey has strong consumer protection and employment laws (LAD, CEPA).\n\
                                 - Cite New Jersey statutes as N.J. Stat. Ann. § [Section].",
                "Virginia" => "\n- Cite Virginia statutes as Va. Code Ann. § [Section].",
                "Massachusetts" => "\n- Massachusetts can be MORE restrictive in: employment law, consumer protection (Chapter 93A), healthcare.\n\
                                    - Cite Massachusetts statutes as Mass. Gen. Laws ch. [Chapter], § [Section].",
                "Washington" => "\n- Washington has no state income tax. Strong employee protections and consumer privacy laws.\n\
                                 - Cite Washington statutes as Wash. Rev. Code § [Section].",
                "Colorado" => "\n- Cite Colorado statutes as Colo. Rev. Stat. § [Section].",
                "Michigan" => "\n- Cite Michigan statutes as Mich. Comp. Laws § [Section].",
                "Arizona" => "\n- Cite Arizona statutes as Ariz. Rev. Stat. § [Section].",
                "Maryland" => "\n- Cite Maryland statutes as Md. Code Ann., [Article] § [Section].",
                "North Carolina" => "\n- Cite North Carolina statutes as N.C. Gen. Stat. § [Section].",
                _ => "",
            };
            format!(
                "Jurisdiction: {state_name} state law applies.\n\
                 - Federal law overrides conflicting {state_name} law (Supremacy Clause).\n\
                 - {state_name} can add requirements beyond federal minimums unless federally preempted.\n\
                 - If both federal and {state_name} law apply, state which governs and why.{state_specific}"
            )
        }
        JurisdictionLevel::County => {
            let county = j.county.as_deref().unwrap_or("the local county");
            let state_name = j.state.as_deref().unwrap_or("the state");
            format!(
                "Jurisdiction: {county}, {state_name} local law applies.\n\
                 - Federal law > state law > county ordinances in case of conflict.\n\
                 - {county} can add requirements beyond {state_name} state minimums unless the state has expressly preempted the area.\n\
                 - Cite local ordinances by name and number when available."
            )
        }
    }
}

// ── Singletons ─────────────────────────────────────────────────────────────────

// fastembed TextEmbedding: ~22 MB ONNX, downloaded to model_dir/fastembed/ on first use.
static EMBED_MODEL: OnceLock<Arc<Mutex<Option<fastembed::TextEmbedding>>>> = OnceLock::new();

// llama.cpp backend stored as Option so init failures don't poison the OnceLock.
static LLAMA_BACKEND: OnceLock<Option<LlamaBackend>> = OnceLock::new();

pub fn get_llama_backend() -> Result<&'static LlamaBackend, String> {
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
pub fn validate_gguf(path: &std::path::Path) -> Result<(), String> {
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

// ── Embedding ─────────────────────────────────────────────────────────────────

/// Embed a text string using the fastembed BGE-small-en-v1.5 model.
/// When `is_query` is true, a retrieval prefix is prepended for asymmetric search.
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

// ── LLM via llama-cpp-2 ───────────────────────────────────────────────────────

/// Format prior conversation turns as labeled text for the model.
pub fn format_history(history: &[(String, String)]) -> String {
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

/// Run LLM inference on Saul-7B-Instruct with the given question, retrieved context, and chat history.
pub async fn ask_saul(
    user_question: &str,
    context: &str,
    history: &[(String, String)],
    model_dir: &Path,
    model_cache: Arc<Mutex<Option<llama_cpp_2::model::LlamaModel>>>,
    on_token: impl Fn(String) + Send + 'static,
    jurisdiction: Option<&Jurisdiction>,
    inference_params: InferenceParams,
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

    // Inject jurisdiction-specific rules into the system prompt when available.
    let j_fragment = jurisdiction.map(jurisdiction_prompt_fragment).unwrap_or_default();
    let mode_suffix = inference_params.system_prompt_suffix;
    let sys_prompt = match (j_fragment.is_empty(), mode_suffix.is_empty()) {
        (true, true) => RULES_PROMPT.to_string(),
        (false, true) => format!("{RULES_PROMPT}\n\n{j_fragment}"),
        (true, false) => format!("{RULES_PROMPT}{mode_suffix}"),
        (false, false) => format!("{RULES_PROMPT}\n\n{j_fragment}{mode_suffix}"),
    };

    // "Answer:" is placed AFTER [/INST] (in the assistant turn), not before it.
    let prompt = format!("[INST] <<SYS>>\n{sys_prompt}\n<</SYS>>\n\n{user_content} [/INST]");

    tokio::task::spawn_blocking(move || {
        // Get (or lazily initialize) the global llama.cpp backend.
        let backend = get_llama_backend()?;

        // Validate GGUF magic bytes before loading.
        validate_gguf(&gguf_path)?;

        // Lock model cache; load from disk on first call only
        let mut model_guard = model_cache
            .lock()
            .map_err(|e| format!("Model mutex poisoned: {e}"))?;

        if model_guard.is_none() {
            log::info!("Loading Saul model from disk (first query)…");
            // Try GPU-accelerated first (Metal on macOS, Vulkan on Linux/Windows).
            // If GPU loading fails (e.g. no Vulkan driver), fall back to CPU-only.
            let model_params_gpu = LlamaModelParams::default().with_n_gpu_layers(100);
            let model = match LlamaModel::load_from_file(backend, &gguf_path, &model_params_gpu) {
                Ok(m) => {
                    log::info!("Saul model loaded with GPU acceleration.");
                    m
                }
                Err(gpu_err) => {
                    log::warn!("GPU model load failed ({gpu_err}), retrying with CPU-only…");
                    let model_params_cpu = LlamaModelParams::default().with_n_gpu_layers(0);
                    LlamaModel::load_from_file(backend, &gguf_path, &model_params_cpu)
                        .map_err(|e| format!("Failed to load Saul model (CPU fallback): {e}"))?
                }
            };
            *model_guard = Some(model);
            log::info!("Saul model loaded and cached.");
        }

        let model = model_guard.as_ref()
            .ok_or_else(|| "Saul model unavailable after initialization".to_string())?;

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

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(64, 1.1, 0.0, 0.0),
            LlamaSampler::min_p(0.05, 1),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::temp(inference_params.temperature),
            LlamaSampler::dist(42),
        ]);
        let mut response = String::new();
        let mut pos = n_tokens;
        // Cap generation to what actually fits in the context window.
        let available_gen = (n_ctx_size as usize).saturating_sub(n_tokens);
        let max_new_tokens = inference_params.max_new_tokens.min(available_gen);
        if max_new_tokens < inference_params.max_new_tokens {
            log::info!(
                "Generation capped to {} tokens (prompt used {}/{} tokens).",
                max_new_tokens, n_tokens, n_ctx_size
            );
        }

        for _ in 0..max_new_tokens {
            if pos >= n_ctx_size as usize {
                log::warn!("Generation stopped: reached context window limit ({n_ctx_size} tokens).");
                break;
            }

            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            let output_bytes = model
                .token_to_piece_bytes(token, 128, false, None)
                .map_err(|e| format!("Token decode error: {e}"))?;
            let token_piece = String::from_utf8_lossy(&output_bytes).into_owned();
            on_token(token_piece.clone());
            response.push_str(&token_piece);

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

        let answer = answer.trim().to_string();

        // Strip conversational filler from the tail of the response
        let answer = strip_trailing_filler(&answer);

        // Truncate incomplete trailing sentence if generation hit token limit
        let answer = truncate_incomplete_sentence(&answer);

        Ok(answer)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Post-processing helpers ──────────────────────────────────────────────────

const FILLER_PATTERNS: &[&str] = &[
    "I hope this helps",
    "Let me know if you have",
    "Please let me know if",
    "If you have any further",
    "Feel free to ask",
    "Is there anything else",
    "Please note that this is not legal advice",
    "Please consult a licensed attorney",
    "I recommend consulting",
];

/// Strip conversational filler from the tail of the response.
/// Only removes if the pattern appears in the last ~200 chars (sign-off position).
fn strip_trailing_filler(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.len() < 10 {
        return trimmed.to_string();
    }
    let tail_start = trimmed.len().saturating_sub(200);
    let tail = &trimmed[tail_start..];
    let lower_tail = tail.to_lowercase();

    for pattern in FILLER_PATTERNS {
        if let Some(pos) = lower_tail.find(&pattern.to_lowercase()) {
            let cut = tail_start + pos;
            let result = trimmed[..cut].trim_end().trim_end_matches(&['.', ',', ';', ' '][..]);
            if !result.is_empty() {
                return result.to_string();
            }
        }
    }
    trimmed.to_string()
}

/// Truncate incomplete trailing sentence when generation hits the token limit.
/// Only trims if keeping >50% of the response and it doesn't end with sentence punctuation.
fn truncate_incomplete_sentence(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }

    let last_char = trimmed.chars().last().unwrap();
    if matches!(last_char, '.' | '!' | '?' | ')' | ']') {
        return trimmed.to_string();
    }

    // Find last sentence boundary
    let boundary = trimmed.rfind(|c: char| matches!(c, '.' | '!' | '?'));
    if let Some(pos) = boundary {
        if pos > trimmed.len() / 2 {
            return trimmed[..=pos].to_string();
        }
    }
    trimmed.to_string()
}

// ── Chunking ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TempChunk {
    pub id: String,
    pub page_number: u32,
    pub chunk_index: usize,
    pub text: String,
    pub token_count: usize,
}

#[derive(PartialEq)]
pub enum FragKind { Normal, ParagraphBreak }

pub struct SentenceFrag<'a> { pub text: &'a str, pub kind: FragKind }

/// Split `text` into sub-slices each at most `max_bytes` bytes long,
/// always cutting at a valid UTF-8 char boundary so no character is mangled.
pub fn split_at_char_boundaries(text: &str, max_bytes: usize) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let raw_end = (start + max_bytes).min(text.len());
        let end = text.floor_char_boundary(raw_end);
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

/// Split parsed document pages into overlapping text chunks suitable for embedding.
/// Respects `settings.chunkSize` and `settings.chunkOverlap` for window sizing.
pub fn chunk_document(pages: &[DocumentPage], settings: &AppSettings) -> Vec<TempChunk> {
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
                let is_orphan = is_section_header(frag.text);

                if is_orphan {
                    if !current.is_empty() {
                        flush(&current, &mut global_idx, &mut chunks, page.page_number);
                        current.clear();
                        sentence_buf.clear();
                    }
                    pending_header = Some(match pending_header.take() {
                        Some(existing) => format!("{existing}\n{}", frag.text),
                        None => frag.text.to_string(),
                    });
                    continue;
                }

                if !current.is_empty() {
                    flush(&current, &mut global_idx, &mut chunks, page.page_number);
                    current.clear();
                    sentence_buf.clear();
                }
                if let Some(h) = pending_header.take() {
                    current.push_str(&h);
                    current.push('\n');
                }
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

        if !chunks.is_empty() {
            let avg_tokens = chunks.iter().map(|c| c.token_count).sum::<usize>()
                / chunks.len();
            let file_name_hint = "document";
            log::debug!(
                "pipeline: chunked '{}' into {} chunks (avg {} tokens)",
                file_name_hint, chunks.len(), avg_tokens
            );
        }
    }

    chunks
}

/// Heuristic check whether a line looks like a section header (e.g. numbered headings, ALL-CAPS titles).
pub fn is_section_header(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || t.len() >= 80 { return false; }
    if t.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        if let Some(dot_pos) = t.find('.') {
            if t[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let after = t[dot_pos + 1..].trim();
                if after.len() <= 40
                    && !after.ends_with('.')
                    && !after.ends_with('!')
                    && !after.ends_with('?') {
                    return true;
                }
            }
        }
    }
    if t.ends_with('.') || t.ends_with('!') || t.ends_with('?') { return false; }
    let u = t.to_uppercase();
    if u.starts_with("SECTION") || u.starts_with("ARTICLE") || u.starts_with("WHEREAS")
        || u.starts_with("NOW THEREFORE") || u.starts_with("SCHEDULE")
        || u.starts_with("EXHIBIT") || u.starts_with("ANNEX") {
        return true;
    }
    if t.len() >= 6
        && t.chars().any(|c| c.is_alphabetic())
        && t.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
        && t.split_whitespace().count() <= 8 {
        return true;
    }
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

/// Split text into sentence-level fragments for fine-grained chunking.
pub fn split_sentences(text: &str) -> Vec<SentenceFrag<'_>> {
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
                if newline_count >= 2 { next_para_break = true; }
                start = j;
                i = j;
            } else {
                i += 1;
            }
        } else if b == b'\n' {
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

// ── BM25 ─────────────────────────────────────────────────────────────────────

/// Tokenize text into lowercase alphanumeric terms (≥2 chars).
pub fn bm25_tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect()
}

/// Precomputed BM25 corpus statistics for a set of chunks.
pub struct Bm25Index {
    /// Number of documents containing each term.
    doc_freq: std::collections::HashMap<String, usize>,
    /// Total number of documents.
    n_docs: usize,
    /// Average document length (in tokens).
    avg_dl: f32,
    /// Per-document token counts (parallel to the chunk slice).
    doc_lens: Vec<usize>,
}

impl Bm25Index {
    /// Build the index from chunk texts.
    pub fn build(texts: &[&str]) -> Self {
        let mut doc_freq: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut doc_lens = Vec::with_capacity(texts.len());
        let mut total_tokens = 0usize;

        for text in texts {
            let tokens = bm25_tokenize(text);
            doc_lens.push(tokens.len());
            total_tokens += tokens.len();

            let unique: std::collections::HashSet<&str> =
                tokens.iter().map(|s| s.as_str()).collect();
            for term in unique {
                *doc_freq.entry(term.to_string()).or_insert(0) += 1;
            }
        }

        let n_docs = texts.len();
        let avg_dl = if n_docs > 0 {
            total_tokens as f32 / n_docs as f32
        } else {
            1.0
        };

        Bm25Index { doc_freq, n_docs, avg_dl, doc_lens }
    }

    /// Score a single document against a query. Returns BM25 score.
    /// `doc_idx` is the index into the original texts slice.
    /// `doc_text` is the document text (re-tokenized per query for TF).
    pub fn score(&self, query_terms: &[String], doc_text: &str, doc_idx: usize) -> f32 {
        const K1: f32 = 1.2;
        const B: f32 = 0.75;

        let doc_tokens = bm25_tokenize(doc_text);
        let dl = self.doc_lens[doc_idx] as f32;

        // Count term frequencies in this document.
        let mut tf: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for t in &doc_tokens {
            *tf.entry(t.as_str()).or_insert(0) += 1;
        }

        let mut score = 0.0f32;
        for qt in query_terms {
            let n = *self.doc_freq.get(qt.as_str()).unwrap_or(&0) as f32;
            let idf = ((self.n_docs as f32 - n + 0.5) / (n + 0.5) + 1.0).ln();
            let idf = idf.max(0.0); // clamp negative IDF for very common terms

            let term_tf = *tf.get(qt.as_str()).unwrap_or(&0) as f32;
            let tf_norm = (term_tf * (K1 + 1.0))
                / (term_tf + K1 * (1.0 - B + B * dl / self.avg_dl));

            score += idf * tf_norm;
        }
        score
    }

    /// Score all documents against query terms, returning scores in index order.
    pub fn score_all(&self, query_terms: &[String], texts: &[&str]) -> Vec<f32> {
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| self.score(query_terms, text, i))
            .collect()
    }
}

/// Compute hybrid scores: `alpha * cosine + (1-alpha) * normalized_bm25`.
/// `cosine_scores` and `bm25_scores` must be parallel arrays.
pub fn hybrid_scores(cosine_scores: &[f32], bm25_scores: &[f32], alpha: f32) -> Vec<f32> {
    // Normalize BM25 scores to [0, 1] so they're on the same scale as cosine.
    let max_bm25 = bm25_scores.iter().cloned().fold(0.0f32, f32::max);
    let norm = if max_bm25 > 0.0 { max_bm25 } else { 1.0 };

    cosine_scores
        .iter()
        .zip(bm25_scores.iter())
        .map(|(&cos, &bm25)| alpha * cos + (1.0 - alpha) * (bm25 / norm))
        .collect()
}

/// Like `hybrid_scores` but with form-data awareness.
/// Chunks whose text starts with "FILLED FORM DATA" get a configurable boost.
pub fn hybrid_scores_with_boost(
    cosine_scores: &[f32],
    bm25_scores: &[f32],
    chunk_texts: &[&str],
    alpha: f32,
    form_boost: f32,
) -> Vec<f32> {
    let max_bm25 = bm25_scores.iter().cloned().fold(0.0f32, f32::max);
    let norm = if max_bm25 > 0.0 { max_bm25 } else { 1.0 };

    cosine_scores
        .iter()
        .zip(bm25_scores.iter())
        .zip(chunk_texts.iter())
        .map(|((&cos, &bm25), &text)| {
            let base = alpha * cos + (1.0 - alpha) * (bm25 / norm);
            if text.starts_with("FILLED FORM DATA") {
                (base + form_boost).min(1.0)
            } else {
                base
            }
        })
        .collect()
}

/// Reciprocal Rank Fusion: merge multiple ranked lists by rank position.
/// Each score list is sorted independently; the fused score for item `i` is:
///   `sum over lists: 1 / (k + rank_of_i_in_list)`
/// where `k` is a smoothing constant (standard: 60).
/// This avoids score normalization issues and is robust across heterogeneous scorers.
pub fn rrf_scores(score_lists: &[Vec<f32>], chunk_texts: &[&str], form_boost: f32) -> Vec<f32> {
    const K: f32 = 60.0;
    let n = score_lists[0].len();
    let mut fused = vec![0.0f32; n];

    for scores in score_lists {
        // Rank by descending score.
        let mut ranked: Vec<(usize, f32)> = scores.iter().cloned().enumerate().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (rank, &(idx, _)) in ranked.iter().enumerate() {
            fused[idx] += 1.0 / (K + rank as f32 + 1.0);
        }
    }

    // Apply form-data boost on top of fused scores.
    for (i, &text) in chunk_texts.iter().enumerate() {
        if text.starts_with("FILLED FORM DATA") {
            fused[i] += form_boost;
        }
    }

    fused
}

// ── Retrieval helpers ─────────────────────────────────────────────────────────

/// Expand query keywords with common legal/employment synonyms.
pub fn expand_keywords(keywords: &std::collections::HashSet<String>) -> std::collections::HashSet<String> {
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
pub fn mmr_select(
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

// ── Pluggable Retrieval Backend ───────────────────────────────────────────────

/// A scored result referencing a corpus item by index.
pub struct ScoredResult {
    pub score: f32,
    /// Index into the `RetrievalCorpus` arrays.
    pub chunk_index: usize,
}

/// Borrowed corpus of chunk texts and embedding vectors.
pub struct RetrievalCorpus<'a> {
    pub texts: Vec<&'a str>,
    pub vectors: Vec<&'a [f32]>,
}

/// Knobs for a retrieval pass.
pub struct RetrievalConfig {
    pub top_k: usize,
    /// How many top candidates to feed into MMR. 0 = skip MMR, return raw top-k.
    pub candidate_pool_k: usize,
    /// Minimum hybrid score to include. 0.0 = no threshold.
    pub score_threshold: f32,
    /// MMR lambda (0.0–1.0). Ignored when `candidate_pool_k == 0`.
    pub mmr_lambda: f32,
    /// Whether to expand query keywords with legal synonyms.
    pub expand_keywords: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            top_k: 6,
            candidate_pool_k: 36,
            score_threshold: SCORE_THRESHOLD,
            mmr_lambda: 0.7,
            expand_keywords: true,
        }
    }
}

/// Pluggable retrieval backend. Implementations score and rank corpus chunks
/// for a given query. Embedding happens upstream; this trait is pure CPU math.
pub trait RetrievalBackend {
    fn retrieve(
        &self,
        query_text: &str,
        query_vector: &[f32],
        corpus: &RetrievalCorpus<'_>,
        config: &RetrievalConfig,
    ) -> Vec<ScoredResult>;

    fn name(&self) -> &str;
}

/// Post-process retrieval results: ensure any "FILLED FORM DATA" chunks are
/// always included. These tiny chunks contain the actual user-specific values
/// and are always relevant when the user asks about document content.
pub fn ensure_form_data_included(
    results: &mut Vec<ScoredResult>,
    corpus: &RetrievalCorpus<'_>,
    max_extra: usize,
) {
    let already: std::collections::HashSet<usize> =
        results.iter().map(|r| r.chunk_index).collect();
    let mut added = 0;
    for (i, text) in corpus.texts.iter().enumerate() {
        if added >= max_extra { break; }
        if already.contains(&i) { continue; }
        if text.starts_with("FILLED FORM DATA") {
            // Insert at position 1 (after the top result) so it's prominent
            // but doesn't displace the best semantic match.
            let insert_pos = 1.min(results.len());
            results.insert(insert_pos, ScoredResult {
                score: 0.5, // neutral score
                chunk_index: i,
            });
            added += 1;
        }
    }
}

// ── Default backend: hybrid BM25 + cosine ────────────────────────────────────

pub struct HybridBm25Cosine {
    pub alpha: f32,
    pub form_boost: f32,
}

impl Default for HybridBm25Cosine {
    fn default() -> Self {
        Self { alpha: 0.5, form_boost: 0.15 }
    }
}

/// Return the default retrieval backend (`HybridBm25Cosine` with standard parameters).
pub fn default_backend() -> HybridBm25Cosine {
    HybridBm25Cosine::default()
}

/// Stop words filtered out during keyword extraction.
const STOP_WORDS: &[&str] = &[
    "a","an","the","is","are","was","were","be","been","being","have","has","had",
    "do","does","did","will","would","could","should","may","might","shall","can",
    "i","me","my","we","our","you","your","he","she","it","they","what","which",
    "who","this","that","these","those","of","in","on","at","by","for","with",
    "about","as","into","to","from","and","but","or","not","any","all","some",
    "how","when","where","why","there","find","show","tell","explain","give",
    "please","provide","describe",
];

/// Extract meaningful keywords from query text, optionally expanding with synonyms.
pub fn extract_query_keywords(query: &str, expand: bool) -> std::collections::HashSet<String> {
    let stop: std::collections::HashSet<&str> = STOP_WORDS.iter().cloned().collect();
    let base: std::collections::HashSet<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !stop.contains(*w))
        .map(|w| w.to_string())
        .collect();
    if expand { expand_keywords(&base) } else { base }
}

impl RetrievalBackend for HybridBm25Cosine {
    fn retrieve(
        &self,
        query_text: &str,
        query_vector: &[f32],
        corpus: &RetrievalCorpus<'_>,
        config: &RetrievalConfig,
    ) -> Vec<ScoredResult> {
        if corpus.texts.is_empty() {
            return vec![];
        }

        // 1. BM25 scoring
        let bm25_index = Bm25Index::build(&corpus.texts);
        let mut query_terms = bm25_tokenize(&query_text.to_lowercase());
        if config.expand_keywords {
            let keywords = extract_query_keywords(query_text, true);
            for kw in &keywords {
                if !query_terms.contains(kw) {
                    query_terms.push(kw.clone());
                }
            }
        }
        let bm25_scores = bm25_index.score_all(&query_terms, &corpus.texts);

        // 2. Cosine scoring
        let cosine_scores: Vec<f32> = corpus.vectors
            .iter()
            .map(|v| RagState::cosine_similarity(query_vector, v))
            .collect();

        // 3. Reciprocal Rank Fusion — merge BM25 and cosine by rank position.
        // RRF is more robust than linear blending because it doesn't require
        // score normalization and handles heterogeneous score distributions well.
        let hybrid = rrf_scores(
            &[cosine_scores, bm25_scores], &corpus.texts, self.form_boost,
        );

        // 4. Sort by fused score descending
        let mut indexed: Vec<(usize, f32)> = hybrid.into_iter().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 5. Threshold filter
        let above: Vec<(usize, f32)> = if config.score_threshold > 0.0 {
            let filtered: Vec<_> = indexed.iter()
                .filter(|(_, s)| *s >= config.score_threshold)
                .cloned()
                .collect();
            if filtered.is_empty() { indexed.clone() } else { filtered }
        } else {
            indexed
        };

        // 6. MMR diversity selection (if candidate_pool_k > 0)
        if config.candidate_pool_k > 0 {
            let pool_size = config.candidate_pool_k.min(above.len());
            let pool: Vec<(f32, ChunkMetadata, Vec<f32>)> = above[..pool_size]
                .iter()
                .map(|&(idx, score)| {
                    let meta = ChunkMetadata {
                        id: idx.to_string(),
                        document_id: String::new(),
                        file_name: String::new(),
                        file_path: String::new(),
                        page_number: 0,
                        chunk_index: idx,
                        text: corpus.texts[idx].to_string(),
                        token_count: 0,
                    };
                    (score, meta, corpus.vectors[idx].to_vec())
                })
                .collect();

            let mmr = mmr_select(pool, config.top_k, config.mmr_lambda);
            mmr.into_iter()
                .map(|(score, meta)| ScoredResult { score, chunk_index: meta.chunk_index })
                .collect()
        } else {
            // No MMR — raw top-k
            above.into_iter()
                .take(config.top_k)
                .map(|(idx, score)| ScoredResult { score, chunk_index: idx })
                .collect()
        }
    }

    fn name(&self) -> &str {
        "hybrid-bm25-cosine"
    }
}

// ── Reranker backend ─────────────────────────────────────────────────────────

// Singleton for the cross-encoder reranker model (~38 MB ONNX, downloaded on first use).
static RERANK_MODEL: OnceLock<Arc<Mutex<Option<fastembed::TextRerank>>>> = OnceLock::new();

/// Two-stage retrieval: cheap first-pass (BM25+cosine) → cross-encoder rerank.
/// Uses JINA Reranker v1 Turbo (~38 MB) via fastembed's TextRerank.
pub struct RerankerBackend {
    /// First-pass retrieval backend.
    pub first_pass: HybridBm25Cosine,
    /// How many candidates the first pass returns for reranking.
    pub first_pass_k: usize,
    /// Cache directory for the reranker ONNX model.
    pub cache_dir: std::path::PathBuf,
}

impl RerankerBackend {
    pub fn new(cache_dir: std::path::PathBuf) -> Self {
        Self {
            first_pass: HybridBm25Cosine::default(),
            // For small corpora (≤500 chunks), reranking all of them is fast (~100ms).
            // Set high so the reranker sees everything by default.
            first_pass_k: 500,
            cache_dir,
        }
    }
}

impl RetrievalBackend for RerankerBackend {
    fn retrieve(
        &self,
        query_text: &str,
        query_vector: &[f32],
        corpus: &RetrievalCorpus<'_>,
        config: &RetrievalConfig,
    ) -> Vec<ScoredResult> {
        if corpus.texts.is_empty() {
            return vec![];
        }

        // Stage 1: cheap first-pass retrieval to narrow candidates.
        let mut first_pass_config = RetrievalConfig {
            top_k: self.first_pass_k,
            candidate_pool_k: 0, // no MMR in first pass
            score_threshold: 0.0,
            expand_keywords: config.expand_keywords,
            mmr_lambda: config.mmr_lambda,
        };
        // If corpus is small enough, skip first pass and rerank everything.
        if corpus.texts.len() <= self.first_pass_k {
            first_pass_config.top_k = corpus.texts.len();
        }
        let candidates = self.first_pass.retrieve(query_text, query_vector, corpus, &first_pass_config);

        if candidates.is_empty() {
            return vec![];
        }

        // Stage 2: cross-encoder reranking.
        let docs: Vec<&str> = candidates.iter()
            .map(|r| corpus.texts[r.chunk_index])
            .collect();
        let candidate_indices: Vec<usize> = candidates.iter().map(|r| r.chunk_index).collect();

        match rerank_with_model(query_text, &docs, &self.cache_dir) {
            Ok(reranked) => {
                reranked.into_iter()
                    .take(config.top_k)
                    .map(|rr| ScoredResult {
                        score: rr.score,
                        chunk_index: candidate_indices[rr.index],
                    })
                    .collect()
            }
            Err(e) => {
                log::warn!("Reranker failed, falling back to first-pass results: {e}");
                // Graceful fallback: return first-pass results truncated to top_k.
                candidates.into_iter().take(config.top_k).collect()
            }
        }
    }

    fn name(&self) -> &str {
        "reranker-jina-v1-turbo"
    }
}

/// Internal: run the reranker model (lazy-loaded singleton).
fn rerank_with_model(
    query: &str,
    documents: &[&str],
    cache_dir: &std::path::Path,
) -> Result<Vec<fastembed::RerankResult>, String> {
    use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

    let model_arc = RERANK_MODEL.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut guard = model_arc.lock().map_err(|e| format!("Rerank model mutex poisoned: {e}"))?;

    let rerank_cache = cache_dir.join("fastembed-reranker");
    if guard.is_none() {
        std::fs::create_dir_all(&rerank_cache)
            .map_err(|e| format!("Cannot create reranker cache dir: {e}"))?;
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::JINARerankerV1TurboEn)
                .with_cache_dir(rerank_cache)
                .with_show_download_progress(false),
        )
        .map_err(|e| format!("Failed to initialize reranker model: {e}"))?;
        *guard = Some(model);
    }

    let model = guard.as_ref()
        .ok_or_else(|| "Reranker model unavailable after initialization".to_string())?;

    let doc_vec: Vec<&str> = documents.to_vec();
    model
        .rerank(query, doc_vec, false, None)
        .map_err(|e| format!("Rerank inference failed: {e}"))
}

// ── Unit tests ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppSettings;

    fn default_settings() -> AppSettings {
        AppSettings {
            chunk_size: 500,
            chunk_overlap: 50,
            top_k: 6,
            theme: "dark".to_string(),
            jurisdiction: None,
            inference_mode: InferenceMode::default(),
        }
    }

    fn make_page(text: &str) -> DocumentPage {
        DocumentPage { page_number: 1, text: text.to_string() }
    }

    // ── InferenceParams / RetrievalModeParams ──────────────────────────────

    #[test]
    fn inference_params_quick() {
        let p = InferenceParams::from_mode(&InferenceMode::Quick);
        assert_eq!(p.max_new_tokens, 256);
        assert!((p.temperature - 0.3).abs() < 0.01);
        assert!(!p.system_prompt_suffix.is_empty());
    }

    #[test]
    fn inference_params_balanced() {
        let p = InferenceParams::from_mode(&InferenceMode::Balanced);
        assert_eq!(p.max_new_tokens, 1024);
        assert!((p.temperature - 0.35).abs() < 0.01);
        assert!(p.system_prompt_suffix.is_empty());
    }

    #[test]
    fn inference_params_extended() {
        let p = InferenceParams::from_mode(&InferenceMode::Extended);
        assert_eq!(p.max_new_tokens, 1024);
        assert!((p.temperature - 0.2).abs() < 0.01);
        assert!(!p.system_prompt_suffix.is_empty());
    }

    #[test]
    fn retrieval_params_scale_with_mode() {
        let q = RetrievalModeParams::from_mode(&InferenceMode::Quick);
        let b = RetrievalModeParams::from_mode(&InferenceMode::Balanced);
        let e = RetrievalModeParams::from_mode(&InferenceMode::Extended);

        // top_k increases across modes
        assert!(q.top_k < b.top_k);
        assert!(b.top_k < e.top_k);

        // candidate pool scales with top_k
        assert!(q.candidate_pool_k <= b.candidate_pool_k);
        assert!(b.candidate_pool_k <= e.candidate_pool_k);

        // Quick has a tighter context budget
        assert!(q.max_context_chars_no_jur < b.max_context_chars_no_jur);

        // Extended has at least as much context as Balanced
        assert!(e.max_context_chars_no_jur >= b.max_context_chars_no_jur);

        // Jurisdiction always reduces budget
        assert!(q.max_context_chars_jur < q.max_context_chars_no_jur);
        assert!(b.max_context_chars_jur < b.max_context_chars_no_jur);
        assert!(e.max_context_chars_jur < e.max_context_chars_no_jur);
    }

    #[test]
    fn inference_mode_serde_backward_compat() {
        // Old settings.json without inferenceMode should deserialize to Balanced
        let json = r#"{"chunkSize":1000,"chunkOverlap":150,"topK":6,"theme":"dark"}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.inference_mode, InferenceMode::Balanced);
    }

    #[test]
    fn inference_mode_serde_round_trip() {
        let settings = AppSettings {
            inference_mode: InferenceMode::Extended,
            ..default_settings()
        };
        let json = serde_json::to_string(&settings).unwrap();
        // AppSettings uses camelCase, InferenceMode uses snake_case values
        assert!(json.contains("\"inferenceMode\":\"extended\""));
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.inference_mode, InferenceMode::Extended);
    }

    // ── is_section_header ──────────────────────────────────────────────────

    #[test]
    fn header_all_caps_bartending() {
        assert!(is_section_header("BARTENDING SERVICES"));
    }

    #[test]
    fn header_all_caps_governing_law() {
        assert!(is_section_header("GOVERNING LAW"));
    }

    #[test]
    fn header_rejects_content_sentence() {
        assert!(!is_section_header("The party agrees to pay $275."));
    }

    #[test]
    fn header_rejects_event_date_line() {
        // "Event Date: Sat 2.28.26" — contains a colon, mixed case, should not be a header
        assert!(!is_section_header("Event Date: Sat 2.28.26"));
    }

    // ── chunk_document — filled form data ──────────────────────────────────

    #[test]
    fn chunk_short_doc_preserves_filled_data() {
        let text = "Client: Liam Neild. Event Date: Sat 2.28.26. Amount: $275 as signing bonus.";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        let all_text: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(all_text.contains("2.28.26") || all_text.contains("Sat"), "Date missing: {all_text}");
        assert!(all_text.contains("$275"), "Amount missing: {all_text}");
    }

    #[test]
    fn chunk_bartending_contract_pattern() {
        // Simulates the actual extracted text pattern from the bartending contract
        let text = "Client Name: _______ Event: _______\nEvent Date: ________ Event Time: 3-7pm\n\nLiam Neild Party williamaneild@gmail.com Sat 2.28.26 3-7pm 101-125 $275 as signing\n2/28/2026 2/25/2026";
        let pages = vec![make_page(text)];
        let chunks = chunk_document(&pages, &default_settings());
        let all_text: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(all_text.contains("Sat 2.28.26") || all_text.contains("2.28.26"),
            "Date 'Sat 2.28.26' missing from chunks: {all_text}");
        assert!(all_text.contains("$275"), "Amount '$275' missing from chunks: {all_text}");
    }

    // ── mmr_select ─────────────────────────────────────────────────────────

    fn make_chunk_meta(id: &str) -> ChunkMetadata {
        ChunkMetadata {
            id: id.to_string(),
            document_id: "doc1".to_string(),
            file_name: "test.pdf".to_string(),
            file_path: "/tmp/test.pdf".to_string(),
            page_number: 1,
            chunk_index: 0,
            text: id.to_string(),
            token_count: 10,
        }
    }

    #[test]
    fn mmr_returns_top_k() {
        let candidates: Vec<(f32, ChunkMetadata, Vec<f32>)> = (0..10)
            .map(|i| {
                let score = 1.0 - i as f32 * 0.05;
                let vec = vec![score, 0.0, 0.0];
                (score, make_chunk_meta(&format!("chunk{i}")), vec)
            })
            .collect();
        let result = mmr_select(candidates, 4, 0.7);
        assert_eq!(result.len(), 4, "Expected exactly 4 results");
    }

    #[test]
    fn mmr_diversifies_near_duplicate_chunks() {
        // chunk_a and chunk_b are nearly identical (cosine ~1.0)
        // chunk_c is diverse
        let chunk_a = (0.9f32, make_chunk_meta("a"), vec![1.0f32, 0.0, 0.0]);
        let chunk_b = (0.85f32, make_chunk_meta("b"), vec![0.99f32, 0.01, 0.0]);
        let chunk_c = (0.7f32, make_chunk_meta("c"), vec![0.0f32, 1.0, 0.0]);

        let result = mmr_select(vec![chunk_a, chunk_b, chunk_c], 2, 0.7);
        assert_eq!(result.len(), 2);
        let ids: Vec<&str> = result.iter().map(|(_, m)| m.id.as_str()).collect();
        // chunk_a should be selected first (highest score), then chunk_c (diverse)
        // chunk_b should lose to chunk_c because chunk_b is near-duplicate of chunk_a
        assert!(ids.contains(&"a"), "chunk_a should be selected (highest score)");
        assert!(ids.contains(&"c"), "chunk_c should be selected over near-duplicate chunk_b");
        assert!(!ids.contains(&"b"), "chunk_b (near-duplicate) should be penalised by MMR");
    }

    // ── split_sentences ────────────────────────────────────────────────────

    #[test]
    fn split_basic_sentences() {
        let text = "First sentence. Second sentence. Third sentence.";
        let frags = split_sentences(text);
        assert!(frags.len() >= 2, "Expected multiple sentence fragments");
        let texts: Vec<&str> = frags.iter().map(|f| f.text).collect();
        assert!(texts.iter().any(|t| t.contains("First")));
        assert!(texts.iter().any(|t| t.contains("Second")));
    }

    #[test]
    fn split_does_not_split_on_mr_abbreviation() {
        let text = "Mr. Smith signed the contract. The terms are clear.";
        let frags = split_sentences(text);
        // Should not split at "Mr." — so "Mr. Smith signed the contract." is one fragment
        let has_full_sentence = frags.iter().any(|f| f.text.contains("Mr.") && f.text.contains("Smith"));
        assert!(has_full_sentence, "Split on 'Mr.' abbreviation — should not split here. Frags: {:?}",
            frags.iter().map(|f| f.text).collect::<Vec<_>>());
    }

    // ── BM25 ───────────────────────────────────────────────────────────────

    #[test]
    fn bm25_tokenize_basic() {
        let tokens = super::bm25_tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Single-char words filtered out
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn bm25_exact_match_scores_higher() {
        let texts = vec![
            "Liam Neild 18 Eagle Row Atlanta GA 30339",
            "The requester must provide form W-9 to the payee for tax purposes.",
            "Section references are to the Internal Revenue Code unless otherwise noted.",
        ];
        let index = Bm25Index::build(&texts);
        let query = super::bm25_tokenize("liam neild name");
        let s0 = index.score(&query, texts[0], 0);
        let s1 = index.score(&query, texts[1], 1);
        let s2 = index.score(&query, texts[2], 2);
        assert!(s0 > s1, "Chunk with 'Liam Neild' should score higher: {s0} vs {s1}");
        assert!(s0 > s2, "Chunk with 'Liam Neild' should score higher: {s0} vs {s2}");
    }

    #[test]
    fn hybrid_scores_blend() {
        let cosine = vec![0.8, 0.3, 0.5];
        let bm25 = vec![0.0, 2.0, 1.0];
        let hybrid = super::hybrid_scores(&cosine, &bm25, 0.5);
        // chunk 0: 0.5*0.8 + 0.5*(0/2) = 0.4
        // chunk 1: 0.5*0.3 + 0.5*(2/2) = 0.65
        // chunk 2: 0.5*0.5 + 0.5*(1/2) = 0.5
        assert!((hybrid[0] - 0.4).abs() < 0.01);
        assert!((hybrid[1] - 0.65).abs() < 0.01);
        assert!((hybrid[2] - 0.5).abs() < 0.01);
        // BM25 match now boosts chunk 1 above chunk 0
        assert!(hybrid[1] > hybrid[0]);
    }

    #[test]
    fn rrf_fuses_rankings_correctly() {
        // 5 items so rank spread is big enough for RRF to differentiate.
        // cosine ranks: 0 > 2 > 4 > 3 > 1
        let cosine = vec![0.9, 0.1, 0.7, 0.2, 0.5];
        // bm25 ranks: 1 > 2 > 3 > 4 > 0
        let bm25   = vec![0.0, 5.0, 4.0, 3.0, 1.0];
        let texts   = vec!["a", "b", "c", "d", "e"];

        let fused = super::rrf_scores(&[cosine, bm25], &texts, 0.0);

        // chunk2: rank 2 in cosine + rank 2 in bm25 → best combined
        // chunk0: rank 1 in cosine + rank 5 in bm25
        // chunk1: rank 5 in cosine + rank 1 in bm25
        assert!(fused[2] > fused[0], "chunk2 should beat chunk0: {:.5} vs {:.5}", fused[2], fused[0]);
        assert!(fused[2] > fused[1], "chunk2 should beat chunk1: {:.5} vs {:.5}", fused[2], fused[1]);
        // chunk0 and chunk1 have symmetric ranks (1+5 vs 5+1) → should be equal
        assert!((fused[0] - fused[1]).abs() < 0.001,
            "chunk0 and chunk1 should tie (symmetric ranks): {:.5} vs {:.5}", fused[0], fused[1]);
    }

    #[test]
    fn rrf_form_boost_applies() {
        let cosine = vec![0.5, 0.5];
        let bm25 = vec![1.0, 1.0];
        let texts = vec!["FILLED FORM DATA: name=Liam", "Regular chunk text"];

        let fused = super::rrf_scores(&[cosine, bm25], &texts, 0.15);
        assert!(fused[0] > fused[1], "Form data chunk should get boosted");
    }

    // ── RetrievalBackend ───────────────────────────────────────────────────

    #[test]
    fn backend_retrieve_returns_top_k() {
        let texts = vec![
            "The contract states the salary is $50,000 per year.",
            "Liam Neild 18 Eagle Row Atlanta GA 30339",
            "Section references are to the Internal Revenue Code.",
        ];
        let v0 = vec![1.0, 0.0, 0.0];
        let v1 = vec![0.0, 1.0, 0.0];
        let v2 = vec![0.5, 0.5, 0.0];
        let query_vec = vec![0.9, 0.1, 0.0]; // close to v0

        let corpus = RetrievalCorpus {
            texts: texts.iter().map(|s| *s).collect(),
            vectors: vec![v0.as_slice(), v1.as_slice(), v2.as_slice()],
        };
        let config = RetrievalConfig {
            top_k: 2,
            candidate_pool_k: 0, // no MMR
            score_threshold: 0.0,
            expand_keywords: false,
            ..Default::default()
        };
        let backend = HybridBm25Cosine::default();
        let results = backend.retrieve("salary contract", &query_vec, &corpus, &config);

        assert_eq!(results.len(), 2);
        // First result should be chunk 0 (high cosine + BM25 match on "salary" and "contract")
        assert_eq!(results[0].chunk_index, 0);
    }

    #[test]
    fn backend_retrieve_with_mmr_reduces_duplicates() {
        // MMR should return fewer results than top_k when corpus is tiny,
        // and should prefer diverse chunks over near-duplicates when possible.
        let texts = vec![
            "The salary is fifty thousand dollars per year.",
            "The salary is 50000 dollars annually.",
            "The office is located at 123 Main Street.",
        ];
        let v0 = vec![1.0, 0.0, 0.0];
        let v1 = vec![0.99, 0.01, 0.0]; // near-duplicate of v0
        let v2 = vec![0.0, 1.0, 0.0];   // diverse
        let query_vec = vec![0.95, 0.05, 0.0];

        let corpus = RetrievalCorpus {
            texts: texts.iter().map(|s| *s).collect(),
            vectors: vec![v0.as_slice(), v1.as_slice(), v2.as_slice()],
        };
        // With MMR
        let config_mmr = RetrievalConfig {
            top_k: 3,
            candidate_pool_k: 3,
            score_threshold: 0.0,
            mmr_lambda: 0.7,
            expand_keywords: false,
        };
        // Without MMR
        let config_raw = RetrievalConfig {
            top_k: 3,
            candidate_pool_k: 0,
            score_threshold: 0.0,
            mmr_lambda: 0.7,
            expand_keywords: false,
        };
        let backend = HybridBm25Cosine::default();
        let with_mmr = backend.retrieve("salary", &query_vec, &corpus, &config_mmr);
        let without_mmr = backend.retrieve("salary", &query_vec, &corpus, &config_raw);

        // Both should return all 3 chunks (corpus is tiny)
        assert_eq!(with_mmr.len(), 3);
        assert_eq!(without_mmr.len(), 3);
        // MMR should reorder: the diverse chunk (2) should rank higher than
        // without MMR, where the near-duplicate (1) stays in its raw position
        let mmr_rank_of_2 = with_mmr.iter().position(|r| r.chunk_index == 2).unwrap();
        let raw_rank_of_2 = without_mmr.iter().position(|r| r.chunk_index == 2).unwrap();
        assert!(mmr_rank_of_2 <= raw_rank_of_2,
            "MMR should promote diverse chunk 2: mmr_rank={mmr_rank_of_2} raw_rank={raw_rank_of_2}");
    }

    #[test]
    fn backend_name() {
        assert_eq!(default_backend().name(), "hybrid-bm25-cosine");
    }

    #[test]
    fn extract_query_keywords_filters_stopwords() {
        let kw = extract_query_keywords("What is the person's name?", false);
        assert!(!kw.contains("what"));
        assert!(!kw.contains("the"));
        assert!(kw.contains("person"));
        assert!(kw.contains("name"));
    }

    #[test]
    fn extract_query_keywords_expands() {
        let kw = extract_query_keywords("salary contract", true);
        assert!(kw.contains("salary"));
        assert!(kw.contains("compensation"), "Should expand 'salary' to include 'compensation'");
    }

    // ── format_history ─────────────────────────────────────────────────────

    #[test]
    fn format_history_capped_at_4_turns() {
        let history: Vec<(String, String)> = (0..8)
            .map(|i| (format!("user{i}"), format!("assistant{i}")))
            .collect();
        let result = format_history(&history);
        assert!(!result.contains("user0"), "Turn 0 should be excluded");
        assert!(!result.contains("user3"), "Turn 3 should be excluded");
        assert!(result.contains("user4"), "Turn 4 should be included");
        assert!(result.contains("user7"), "Turn 7 should be included");
    }

    // ── Jurisdiction Detection ────────────────────────────────────────────

    #[test]
    fn detect_federal_usc_citation() {
        let text = "Pursuant to 42 U.S.C. § 1983, the plaintiff alleges deprivation of civil rights.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect federal jurisdiction from U.S.C. citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
        assert!(r.confidence >= 0.3);
    }

    #[test]
    fn detect_federal_cfr_citation() {
        let text = "The regulation at 29 C.F.R. § 1910.134 requires employers to provide respiratory protection.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect federal jurisdiction from C.F.R. citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
    }

    #[test]
    fn detect_federal_district_court() {
        let text = "IN THE U.S. DISTRICT COURT FOR THE NORTHERN DISTRICT OF CALIFORNIA\nCase No. 3:24-cv-01234";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect federal jurisdiction from U.S. District Court");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
    }

    #[test]
    fn detect_california_statute() {
        let text = "Under Cal. Civ. Code § 1942.5, a landlord may not retaliate against a tenant.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect California jurisdiction from statute citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("California"));
    }

    #[test]
    fn detect_new_york_statute() {
        let text = "N.Y. Gen. Bus. Law § 349 prohibits deceptive business practices.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect New York jurisdiction from statute citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("New York"));
    }

    #[test]
    fn detect_texas_statute() {
        let text = "Tex. Bus. & Com. Code § 17.46 defines deceptive trade practices.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Texas jurisdiction from statute citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Texas"));
    }

    #[test]
    fn detect_florida_statute() {
        let text = "Fla. Stat. § 768.81 governs the allocation of damages in negligence cases.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Florida jurisdiction from statute citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Florida"));
    }

    #[test]
    fn detect_illinois_statute() {
        let text = "Violations of 815 ILCS 505 may result in civil penalties under the Consumer Fraud Act.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Illinois jurisdiction from ILCS citation");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Illinois"));
    }

    #[test]
    fn detect_pennsylvania_statute() {
        let text = "18 Pa. Cons. Stat. § 3921 defines theft by unlawful taking.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Pennsylvania jurisdiction");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Pennsylvania"));
    }

    #[test]
    fn detect_georgia_code() {
        let text = "Ga. Code Ann. § 51-12-5.1 governs punitive damages in Georgia.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Georgia jurisdiction");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Georgia"));
    }

    #[test]
    fn detect_ohio_code() {
        let text = "Ohio Rev. Code § 2307.71 defines products liability.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Ohio jurisdiction");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Ohio"));
    }

    #[test]
    fn detect_california_superior_court() {
        let text = "FILED IN THE SUPERIOR COURT OF CALIFORNIA\nCOUNTY OF LOS ANGELES\nCase No. BC-123456";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect California jurisdiction from court name");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("California"));
    }

    #[test]
    fn detect_new_york_supreme_court() {
        let text = "SUPREME COURT OF THE STATE OF NEW YORK\nCOUNTY OF KINGS\nIndex No. 500001/2024";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect New York jurisdiction from court name");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("New York"));
    }

    #[test]
    fn detect_state_of_header() {
        // "State of" header alone has weight 0.15 (below 0.3 threshold).
        // Combine with a statute citation to pass threshold.
        let text = "STATE OF MICHIGAN\nIN THE CIRCUIT COURT FOR THE COUNTY OF WAYNE\nMich. Comp. Laws § 600.2911";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should detect Michigan from statute + header");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Michigan"));
    }

    #[test]
    fn detect_county_extraction() {
        let text = "FILED IN THE SUPERIOR COURT OF CALIFORNIA\nCOUNTY OF LOS ANGELES\nCase No. BC-123456\nPursuant to Cal. Civ. Code § 1942.5";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        let r = result.unwrap();
        // Should detect county because both state statute and county name present
        assert_eq!(r.jurisdiction.state.as_deref(), Some("California"));
        assert!(r.jurisdiction.county.is_some(), "Should extract county name");
    }

    #[test]
    fn detect_no_jurisdiction_in_generic_text() {
        let text = "This is a general document about business operations. \
                     The company provides consulting services to clients in various locations.";
        let result = detect_jurisdiction(text);
        assert!(result.is_none(), "Should not detect jurisdiction from generic text");
    }

    #[test]
    fn detect_no_jurisdiction_in_non_legal_text() {
        let text = "Today's weather forecast calls for sunny skies. \
                     The temperature will reach 75 degrees Fahrenheit by mid-afternoon.";
        let result = detect_jurisdiction(text);
        assert!(result.is_none(), "Should not detect jurisdiction from non-legal text");
    }

    #[test]
    fn detect_federal_over_agency_alone() {
        // Federal agency alone has low weight (0.1), should NOT pass threshold
        let text = "The EEOC investigates claims of workplace discrimination.";
        let result = detect_jurisdiction(text);
        assert!(result.is_none(), "Agency alone (weight 0.1) should not pass 0.3 threshold");
    }

    #[test]
    fn detect_federal_agency_with_statute() {
        // Federal agency + U.S.C. citation should detect federal
        let text = "The EEOC enforces Title VII under 42 U.S.C. § 2000e.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
    }

    #[test]
    fn detect_prefers_higher_weight_statute_over_header() {
        // Both a state header (0.15) and a statute (0.4) — statute should win
        let text = "State of California\n\nPursuant to Cal. Civ. Code § 1942.5, the tenant has rights.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("California"));
        assert!(r.confidence >= 0.4, "Statute weight should be the winning confidence");
    }

    #[test]
    fn detect_colorado_statute() {
        let text = "Colo. Rev. Stat. § 38-12-104 governs security deposit returns.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Colorado"));
    }

    #[test]
    fn detect_new_jersey_statute() {
        let text = "N.J. Stat. Ann. § 2A:15-97 addresses comparative negligence.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("New Jersey"));
    }

    #[test]
    fn detect_virginia_code() {
        let text = "Va. Code Ann. § 8.01-581.15 covers medical malpractice claims.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Virginia"));
    }

    #[test]
    fn detect_washington_code() {
        let text = "Wash. Rev. Code § 59.18.230 requires landlords to return deposits.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Washington"));
    }

    #[test]
    fn detect_massachusetts_statute() {
        let text = "Under Mass. Gen. Laws ch. 93A, the consumer protection act applies.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Massachusetts"));
    }

    #[test]
    fn detect_michigan_compiled_laws() {
        let text = "Mich. Comp. Laws § 600.2911 allows recovery for defamation.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Michigan"));
    }

    #[test]
    fn detect_minnesota_statute() {
        let text = "Minn. Stat. § 504B.178 governs tenant rights to withhold rent.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Minnesota"));
    }

    #[test]
    fn detect_oregon_statute() {
        let text = "Or. Rev. Stat. § 90.100 defines terms for residential landlord-tenant law.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Oregon"));
    }

    #[test]
    fn detect_maryland_code() {
        let text = "Md. Code Ann., Real Property § 8-203 covers security deposits.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Maryland"));
    }

    #[test]
    fn detect_north_carolina_statute() {
        let text = "N.C. Gen. Stat. § 42-25.9 regulates security deposit handling.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("North Carolina"));
    }

    #[test]
    fn detect_wisconsin_statute() {
        let text = "Wis. Stat. § 704.28 addresses tenant deposit refunds.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Wisconsin"));
    }

    #[test]
    fn detect_arizona_statute() {
        let text = "Ariz. Rev. Stat. § 33-1321 sets rules for residential leases.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Arizona"));
    }

    #[test]
    fn detect_indiana_code() {
        let text = "Ind. Code § 32-31-3-12 requires return of security deposits within 45 days.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Indiana"));
    }

    #[test]
    fn detect_tennessee_code() {
        let text = "Tenn. Code Ann. § 66-28-301 governs landlord obligations.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.state.as_deref(), Some("Tennessee"));
    }

    #[test]
    fn detect_us_supreme_court() {
        let text = "The Supreme Court of the United States held in Brown v. Board of Education.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.level, JurisdictionLevel::Federal);
    }

    #[test]
    fn detect_bankruptcy_court() {
        let text = "UNITED STATES BANKRUPTCY COURT\nSOUTHERN DISTRICT OF NEW YORK\nIn re: Debtor Corp.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().jurisdiction.level, JurisdictionLevel::Federal);
    }

    // ── Jurisdiction Prompt Fragment ──────────────────────────────────────

    #[test]
    fn prompt_fragment_federal() {
        let j = Jurisdiction { level: JurisdictionLevel::Federal, state: None, county: None };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("Federal law applies"), "Federal fragment should declare federal law");
        assert!(frag.contains("Erie doctrine"), "Federal fragment should mention Erie");
        assert!(frag.contains("U.S.C."), "Federal fragment should mention U.S.C. citation format");
        assert!(frag.contains("C.F.R."), "Federal fragment should mention C.F.R. citation format");
    }

    #[test]
    fn prompt_fragment_california() {
        let j = Jurisdiction {
            level: JurisdictionLevel::State,
            state: Some("California".to_string()),
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("California state law applies"), "Should declare California");
        assert!(frag.contains("Supremacy Clause"), "Should mention Supremacy Clause");
        assert!(frag.contains("CCPA"), "California fragment should mention CCPA");
        assert!(frag.contains("Cal."), "Should include California citation format");
    }

    #[test]
    fn prompt_fragment_new_york() {
        let j = Jurisdiction {
            level: JurisdictionLevel::State,
            state: Some("New York".to_string()),
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("New York state law applies"));
        assert!(frag.contains("N.Y."), "Should include NY citation format");
        assert!(frag.contains("rent stabilization"), "Should mention NY-specific areas");
    }

    #[test]
    fn prompt_fragment_texas() {
        let j = Jurisdiction {
            level: JurisdictionLevel::State,
            state: Some("Texas".to_string()),
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("Texas state law applies"));
        assert!(frag.contains("Tex."), "Should include Texas citation format");
        assert!(frag.contains("community property"), "Should mention TX-specific areas");
    }

    #[test]
    fn prompt_fragment_illinois() {
        let j = Jurisdiction {
            level: JurisdictionLevel::State,
            state: Some("Illinois".to_string()),
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("Illinois state law applies"));
        assert!(frag.contains("BIPA"), "Should mention IL biometric privacy");
        assert!(frag.contains("ILCS"), "Should include Illinois citation format");
    }

    #[test]
    fn prompt_fragment_generic_state() {
        let j = Jurisdiction {
            level: JurisdictionLevel::State,
            state: Some("Wyoming".to_string()),
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("Wyoming state law applies"));
        assert!(frag.contains("Supremacy Clause"));
        // No Wyoming-specific notes, so fragment should still have the generic rules
        assert!(frag.contains("which governs and why"));
    }

    #[test]
    fn prompt_fragment_county() {
        let j = Jurisdiction {
            level: JurisdictionLevel::County,
            state: Some("California".to_string()),
            county: Some("Los Angeles County".to_string()),
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("Los Angeles County, California"));
        assert!(frag.contains("Federal law > state law > county ordinances"));
        assert!(frag.contains("preempted"));
    }

    #[test]
    fn prompt_fragment_county_no_details() {
        let j = Jurisdiction {
            level: JurisdictionLevel::County,
            state: None,
            county: None,
        };
        let frag = jurisdiction_prompt_fragment(&j);
        assert!(frag.contains("the local county"));
        assert!(frag.contains("the state"));
    }

    // ── A/B: Detection correctness on realistic document excerpts ─────────

    #[test]
    fn ab_california_lease_excerpt() {
        let text = r#"RESIDENTIAL LEASE AGREEMENT

This Lease Agreement is entered into as of January 15, 2024, by and between
John Smith ("Landlord") and Jane Doe ("Tenant").

PROPERTY: 123 Main Street, Apt 4B, Los Angeles, CA 90001

GOVERNING LAW: This agreement shall be governed by Cal. Civ. Code § 1940 et seq.
and the laws of the State of California.

SECURITY DEPOSIT: Pursuant to Cal. Civ. Code § 1950.5, the security deposit
shall not exceed two months' rent for an unfurnished unit."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "California lease should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("California"));
        assert!(r.confidence >= 0.4, "Statute citation should give high confidence");
    }

    #[test]
    fn ab_federal_complaint_excerpt() {
        let text = r#"IN THE UNITED STATES DISTRICT COURT
FOR THE SOUTHERN DISTRICT OF NEW YORK

Civil Action No. 1:24-cv-00567

COMPLAINT

Plaintiff brings this action pursuant to 42 U.S.C. § 1983 and 28 U.S.C. § 1331
for deprivation of rights under color of state law.

JURISDICTION AND VENUE
This Court has subject matter jurisdiction under 28 U.S.C. § 1331."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Federal complaint should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
        assert!(r.confidence >= 0.35);
    }

    #[test]
    fn ab_new_york_contract_excerpt() {
        let text = r#"EMPLOYMENT AGREEMENT

This Agreement is governed by and construed in accordance with the laws of
the State of New York, without regard to conflict of law principles.

The parties agree to submit to the exclusive jurisdiction of the
Supreme Court of the State of New York, County of New York.

N.Y. Lab. Law § 198-c requires timely payment of wages."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "NY contract should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("New York"));
    }

    #[test]
    fn ab_texas_oil_gas_lease_excerpt() {
        let text = r#"OIL AND GAS LEASE

This Lease is made and entered into in the State of Texas.

LESSEE shall comply with all applicable provisions of Tex. Nat. Res. Code § 91.
Disputes shall be resolved in the District Court of Harris County, Texas."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Texas lease should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Texas"));
    }

    #[test]
    fn ab_georgia_statement_of_claim() {
        let text = r#"IN THE SUPERIOR COURT OF FULTON COUNTY
STATE OF GEORGIA

CIVIL ACTION FILE NO. 2024-CV-12345

COMPLAINT

Plaintiff files this Complaint pursuant to Ga. Code Ann. § 9-11-8."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Georgia statement should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Georgia"));
    }

    #[test]
    fn ab_nda_no_jurisdiction() {
        let text = r#"MUTUAL NON-DISCLOSURE AGREEMENT

This Agreement is made between Company A and Company B.
Both parties agree to keep all shared information confidential.
Neither party shall disclose any proprietary information to third parties.
This agreement shall remain in effect for a period of two years."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_none(), "Generic NDA with no legal citations should return None");
    }

    #[test]
    fn ab_irs_w9_form() {
        let text = r#"Form W-9 (Rev. October 2018)
Department of the Treasury
Internal Revenue Service

Request for Taxpayer Identification Number and Certification

Name: John Smith
Business name: Smith Consulting LLC
Federal tax classification: Limited liability company
Address: 456 Oak Ave, Suite 200, Denver, CO 80202"#;

        let result = detect_jurisdiction(text);
        // IRS is a federal agency (weight 0.1) — should NOT pass threshold alone
        assert!(result.is_none(), "IRS form alone (no statute) should not pass 0.3 threshold");
    }

    #[test]
    fn ab_florida_personal_injury() {
        let text = r#"IN THE CIRCUIT COURT OF THE ELEVENTH JUDICIAL CIRCUIT
IN AND FOR MIAMI-DADE COUNTY, FLORIDA

CASE NO. 2024-CA-001234

COMPLAINT FOR DAMAGES

Plaintiff brings this action under Fla. Stat. § 768.81 for comparative negligence."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Florida PI complaint should be detected");
        let r = result.unwrap();
        assert_eq!(r.jurisdiction.state.as_deref(), Some("Florida"));
    }

    #[test]
    fn ab_mixed_federal_and_state_prefers_stronger_signal() {
        // Federal USC citation (0.4) should beat a State-of header (0.15)
        let text = r#"State of California
Department of Insurance

Pursuant to 15 U.S.C. § 1011 (McCarran-Ferguson Act), the federal government
defers to state insurance regulation."#;

        let result = detect_jurisdiction(text);
        assert!(result.is_some());
        let r = result.unwrap();
        // The U.S.C. citation has weight 0.4 vs "State of California" at 0.15
        assert_eq!(r.jurisdiction.level, JurisdictionLevel::Federal);
    }

    // ── Determinism: same input always produces same output ────────────────

    #[test]
    fn detection_is_deterministic() {
        let texts = [
            "Cal. Civ. Code § 1942.5 protects tenant rights.",
            "42 U.S.C. § 1983 provides for civil rights claims.",
            "N.Y. Gen. Bus. Law § 349 prohibits deceptive practices.",
            "Nothing legal here, just a recipe for chocolate cake.",
        ];
        // Run each 5 times and verify identical results
        for text in &texts {
            let first = detect_jurisdiction(text);
            for _ in 0..5 {
                let again = detect_jurisdiction(text);
                match (&first, &again) {
                    (None, None) => {}
                    (Some(a), Some(b)) => {
                        assert_eq!(a.jurisdiction.level, b.jurisdiction.level);
                        assert_eq!(a.jurisdiction.state, b.jurisdiction.state);
                        assert_eq!(a.confidence, b.confidence);
                    }
                    _ => panic!("Detection should be deterministic for: {text}"),
                }
            }
        }
    }

    #[test]
    fn prompt_fragment_is_deterministic() {
        let jurisdictions = [
            Jurisdiction { level: JurisdictionLevel::Federal, state: None, county: None },
            Jurisdiction { level: JurisdictionLevel::State, state: Some("California".to_string()), county: None },
            Jurisdiction { level: JurisdictionLevel::County, state: Some("Texas".to_string()), county: Some("Harris County".to_string()) },
        ];
        for j in &jurisdictions {
            let first = jurisdiction_prompt_fragment(j);
            for _ in 0..5 {
                assert_eq!(first, jurisdiction_prompt_fragment(j), "Prompt fragment should be deterministic");
            }
        }
    }

    // ── Edge cases ────────────────────────────────────────────────────────

    #[test]
    fn detect_empty_text() {
        assert!(detect_jurisdiction("").is_none());
    }

    #[test]
    fn detect_very_short_text() {
        assert!(detect_jurisdiction("Hi").is_none());
    }

    #[test]
    fn detect_handles_unicode() {
        let text = "§ 1983 — Deprivation of rights under 42 U.S.C. § 1983.";
        let result = detect_jurisdiction(text);
        assert!(result.is_some(), "Should handle section symbols in text");
    }

    #[test]
    fn detect_long_text_only_scans_first_10k() {
        // Put citation at position > 10,000 — should NOT be detected
        let padding = "A ".repeat(6000); // 12,000 chars
        let text = format!("{padding}Cal. Civ. Code § 1942.5");
        let result = detect_jurisdiction(&text);
        assert!(result.is_none(), "Should not scan beyond first 10,000 chars");
    }

    #[test]
    fn detect_citation_within_10k() {
        // Put citation at position < 10,000 — should be detected
        let padding = "A ".repeat(2000); // 4,000 chars
        let text = format!("{padding}Cal. Civ. Code § 1942.5");
        let result = detect_jurisdiction(&text);
        assert!(result.is_some(), "Should detect citation within first 10,000 chars");
    }
}
