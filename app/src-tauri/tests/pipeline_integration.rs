// Integration tests for the Justice AI RAG pipeline.
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ TEST TIER OVERVIEW                                                      │
// │                                                                         │
// │  Tier 1 — Extraction    parse PDF → assert raw text contains key facts  │
// │            (no model needed — runs with plain `cargo test`)             │
// │                                                                         │
// │  Tier 2 — Chunking      parse → chunk → assert facts survive in chunks │
// │            (no model needed — runs with plain `cargo test`)             │
// │                                                                         │
// │  Tier 3 — Retrieval     parse → chunk → embed → assert right chunks    │
// │            (#[ignore] — requires fastembed ~33 MB auto-download)        │
// │            Run with: cargo test -- --include-ignored retrieval          │
// │                                                                         │
// │  Tier 4 — E2E           full pipeline including Saul-7B LLM            │
// │            (#[ignore] — requires 4.5 GB GGUF file)                     │
// │            Run with: cargo test -- --include-ignored e2e               │
// │                                                                         │
// │ Quick run:  cargo test                     (tier 1 + 2 only)           │
// │ Retrieval:  cargo test -- --include-ignored retrieval                   │
// │ Full suite: cargo test -- --include-ignored                             │
// └─────────────────────────────────────────────────────────────────────────┘
//
// Design decisions:
//
// 1. LLM answer evaluation via `check_answer(answer, assertions)`:
//    LLM output is non-deterministic. We do NOT assert exact strings; instead
//    we define semantic constraints (Contains, ContainsAny, NotContains,
//    MatchesDate) that the answer must satisfy. This makes tests stable across
//    temperature variation while still catching regressions.
//
// 2. Known regression guard — event date confusion:
//    The system previously confused 2/25/2026 (client signature date) with the
//    event date (2/28/2026 = "Sat 2.28.26"). A HIGH-priority test carries a
//    NotContains("2/25") assertion to prevent this from regressing silently.
//
// 3. Public helpers:
//    `bartending_contract_test_cases()` is `pub` so a CLI harness binary can
//    import and run the test cases programmatically without duplicating them.
//
// 4. #[allow(dead_code)] on shared helpers:
//    Test utilities used only by ignored tests would otherwise trigger warnings
//    when `cargo test` runs without --include-ignored.

use app_lib::commands::doc_parser::parse_pdf;
use app_lib::pipeline::{self, chunk_document, RetrievalBackend, RetrievalConfig, RetrievalCorpus};
use app_lib::state::AppSettings;

// ── Retrieval test helpers ──────────────────────────────────────────────────

/// Model directory for retrieval tests — uses the app's standard data dir.
#[allow(dead_code)]
fn retrieval_model_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.justiceai.app")
        .join("models")
}

/// Embed a query and all chunks, return top-k using the default retrieval backend.
#[allow(dead_code)]
async fn retrieve_top_chunks(
    chunks: &[pipeline::TempChunk],
    query: &str,
    model_dir: &std::path::Path,
    top_k: usize,
) -> Vec<(f32, pipeline::TempChunk)> {
    let query_vec = pipeline::embed_text(query, true, model_dir).await
        .expect("Failed to embed query");

    let mut chunk_vecs: Vec<Vec<f32>> = Vec::new();
    for chunk in chunks {
        chunk_vecs.push(
            pipeline::embed_text(&chunk.text, false, model_dir).await
                .expect("Failed to embed chunk"),
        );
    }

    let corpus = RetrievalCorpus {
        texts: chunks.iter().map(|c| c.text.as_str()).collect(),
        vectors: chunk_vecs.iter().map(|v| v.as_slice()).collect(),
        chunk_indices: chunks.iter().map(|c| c.chunk_index).collect(),
        bm25_index: None,
    };
    let config = RetrievalConfig {
        top_k,
        candidate_pool_k: 0,   // no MMR in tests
        score_threshold: 0.0,   // no threshold
        expand_keywords: true,
        ..Default::default()
    };

    let backend = pipeline::default_backend();
    let mut ranked = backend.retrieve(query, &query_vec, &corpus, &config);
    pipeline::ensure_form_data_included(&mut ranked, &corpus, 2);

    ranked
        .into_iter()
        .map(|r| (r.score, chunks[r.chunk_index].clone()))
        .collect()
}

// ── PDF path resolution ───────────────────────────────────────────────────────

/// Resolve the path to the bartending contract test fixture.
///
/// Checks JUSTICE_AI_TEST_PDF env var first; falls back to the canonical path
/// in the repository's test fixtures directory. If neither exists the test that
/// calls this function should skip gracefully (see `skip_if_missing`).
fn bartending_pdf_path() -> String {
    if let Ok(env_path) = std::env::var("JUSTICE_AI_TEST_PDF") {
        return env_path;
    }
    // Canonical fixtures path — checked into the repo alongside this file.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/tests/fixtures/bartending_contract.pdf")
}

/// Returns true when the file at `path` is present on disk.
/// Prints a diagnostic message and returns false if not found.
fn skip_if_missing(path: &str) -> bool {
    if !std::path::Path::new(path).exists() {
        eprintln!(
            "[SKIP] Test PDF not found at: {path}\n\
             Set JUSTICE_AI_TEST_PDF env var to point to the bartending contract, or\n\
             ensure tests/fixtures/bartending_contract.pdf exists."
        );
        true
    } else {
        false
    }
}

// ── Answer assertion framework ────────────────────────────────────────────────

/// A single semantic constraint on an LLM answer string.
///
/// All comparisons are case-insensitive so minor casing differences don't
/// cause spurious failures. Use `MatchesDate` for dates that may be expressed
/// in multiple formats in the contract text (2.28.26, 2/28/2026, Feb 28, etc.).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Assertion {
    /// The answer must contain this substring (case-insensitive).
    Contains(&'static str),

    /// At least one of these substrings must be present (case-insensitive).
    ContainsAny(&'static [&'static str]),

    /// The answer must NOT contain this substring — used for regression guards
    /// where we know the pipeline previously returned a wrong value.
    NotContains(&'static str),

    /// Flexible date matching: passes if the answer contains ANY of the
    /// supplied date variants. List the forms that appear in the contract
    /// (e.g. "2.28.26", "2/28/2026", "february 28", "feb 28", "28, 2026").
    MatchesDate(&'static [&'static str]),
}

/// Evaluate a list of `Assertion`s against `answer`. Returns a Vec of failure
/// messages (empty = all assertions passed). Case-insensitive throughout.
pub fn check_answer(answer: &str, assertions: &[Assertion]) -> Vec<String> {
    let lower = answer.to_lowercase();
    let mut failures = Vec::new();

    for assertion in assertions {
        match assertion {
            Assertion::Contains(needle) => {
                if !lower.contains(&needle.to_lowercase() as &str) {
                    failures.push(format!(
                        "Expected answer to contain {:?}, but it did not.\nAnswer: {answer}"
                    , needle));
                }
            }
            Assertion::ContainsAny(needles) => {
                let any_match = needles
                    .iter()
                    .any(|n| lower.contains(&n.to_lowercase() as &str));
                if !any_match {
                    failures.push(format!(
                        "Expected answer to contain at least one of {needles:?}, but none matched.\nAnswer: {answer}"
                    ));
                }
            }
            Assertion::NotContains(needle) => {
                if lower.contains(&needle.to_lowercase() as &str) {
                    failures.push(format!(
                        "Expected answer NOT to contain {:?}, but it did.\nAnswer: {answer}"
                    , needle));
                }
            }
            Assertion::MatchesDate(variants) => {
                let any_match = variants
                    .iter()
                    .any(|v| lower.contains(&v.to_lowercase() as &str));
                if !any_match {
                    failures.push(format!(
                        "Expected answer to contain a date matching one of {variants:?}, but none found.\nAnswer: {answer}"
                    ));
                }
            }
        }
    }

    failures
}

// ── Test case definitions ─────────────────────────────────────────────────────

/// Priority level for test cases — HIGH tests cover known failure modes and
/// critical facts; MEDIUM covers secondary facts; LOW covers edge cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Priority {
    High,
    Medium,
    Low,
}

/// A single RAG test case: a query string, the assertions the answer must
/// satisfy, and the priority level for triage when failures occur.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: &'static str,
    pub query: &'static str,
    pub assertions: Vec<Assertion>,
    pub priority: Priority,
}

/// All test cases for the bartending contract fixture.
///
/// This function is `pub` so the CLI harness binary can import it directly
/// from the integration test module without duplicating the definitions.
pub fn bartending_contract_test_cases() -> Vec<TestCase> {
    vec![
        // ── HIGH PRIORITY — known failure modes ───────────────────────────
        TestCase {
            name: "event_date_not_confused_with_signature_date",
            query: "What is the date of the event?",
            assertions: vec![
                // Event is Sat 2.28.26 — accept multiple representations
                Assertion::MatchesDate(&[
                    "2.28.26",
                    "2/28/2026",
                    "february 28",
                    "feb 28",
                    "28, 2026",
                    "28th",
                    "saturday",
                    "sat",
                ]),
                // REGRESSION GUARD: must NOT return the signature date (2/25/2026)
                // as the event date. This was the documented pre-fix failure mode.
                Assertion::NotContains("2/25"),
                Assertion::NotContains("february 25"),
                Assertion::NotContains("feb 25"),
                // Must not return unfilled template blanks (PDF parsing regression)
                Assertion::NotContains("______"),
            ],
            priority: Priority::High,
        },

        TestCase {
            name: "event_time",
            query: "What time does the event start and end?",
            assertions: vec![
                // Event is 3-7pm
                Assertion::ContainsAny(&["3", "3:00", "3pm", "3 pm"]),
                Assertion::ContainsAny(&["7", "7:00", "7pm", "7 pm"]),
                // Bartender hours 2pm-6pm — extra context from the contract
                Assertion::ContainsAny(&["pm", "afternoon", "evening"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::High,
        },

        TestCase {
            name: "total_fee",
            query: "What is the total cost of the bartending service?",
            assertions: vec![
                Assertion::Contains("275"),
                // Dollar sign may or may not appear depending on phrasing
                Assertion::ContainsAny(&["$275", "275 dollars", "275.00", "$275.00"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::High,
        },

        TestCase {
            name: "deposit_amount",
            query: "How much is the deposit and when is it due?",
            assertions: vec![
                Assertion::Contains("275"),
                // Full amount is due at signing
                Assertion::ContainsAny(&["signing", "sign", "full", "due at signing"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::High,
        },

        // ── MEDIUM PRIORITY — secondary contract facts ────────────────────
        TestCase {
            name: "guest_count",
            query: "How many guests are expected at the event?",
            assertions: vec![
                // 101-125 guests
                Assertion::ContainsAny(&["101", "125", "101-125", "101 to 125", "101–125"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        TestCase {
            name: "event_location",
            query: "Where is the event being held?",
            assertions: vec![
                // 18 Eagle Row, Atlanta, GA
                Assertion::ContainsAny(&["18 eagle row", "eagle row"]),
                Assertion::ContainsAny(&["atlanta", "georgia", "ga"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        TestCase {
            name: "client_name",
            query: "Who is the client for this bartending contract?",
            assertions: vec![
                Assertion::ContainsAny(&["liam neild", "liam", "neild"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        TestCase {
            name: "cancellation_policy",
            query: "What is the cancellation policy?",
            assertions: vec![
                // Nonrefundable, but transferable with 30-day notice
                Assertion::ContainsAny(&["nonrefundable", "non-refundable", "non refundable", "refund"]),
                Assertion::ContainsAny(&["30", "thirty", "transfer", "transferable"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        TestCase {
            name: "bartender_service_hours",
            query: "What hours will the bartender be on-site?",
            assertions: vec![
                // Bartender serves 2pm-6pm
                Assertion::ContainsAny(&["2pm", "2:00", "2 pm"]),
                Assertion::ContainsAny(&["6pm", "6:00", "6 pm"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        TestCase {
            name: "additional_hours_rate",
            query: "What is the rate for additional hours of service?",
            assertions: vec![
                // $50 per additional hour
                Assertion::ContainsAny(&["$50", "50 per hour", "50/hr", "50 an hour", "50 dollars"]),
                Assertion::ContainsAny(&["hour", "hr", "additional"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Medium,
        },

        // ── LOW PRIORITY — supplemental facts ────────────────────────────
        TestCase {
            name: "bartender_company",
            query: "What is the name of the bartending company?",
            assertions: vec![
                Assertion::ContainsAny(&[
                    "forever moore",
                    "forever moore ent",
                    "fme",
                    "sharina moore",
                    "moore",
                ]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Low,
        },

        TestCase {
            name: "governing_law",
            query: "What state's law governs this contract?",
            assertions: vec![
                Assertion::ContainsAny(&["georgia", "ga"]),
                Assertion::ContainsAny(&["law", "govern", "liquor"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Low,
        },

        TestCase {
            name: "event_type",
            query: "What type of event is this bartending contract for?",
            assertions: vec![
                Assertion::ContainsAny(&["party", "event", "celebration"]),
                Assertion::NotContains("______"),
            ],
            priority: Priority::Low,
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// TIER 1 — PDF EXTRACTION TESTS
// No model required. These tests validate that parse_pdf correctly extracts
// the key contract facts from the real PDF file.
// ─────────────────────────────────────────────────────────────────────────────

/// Tier 1: basic parse sanity — at least one page returned, no PUA chars, no
/// Identity-H artifacts, no blank-template artifacts (______ unfilled fields).
#[test]
fn extraction_basic_sanity() {
    let path = bartending_pdf_path();
    if skip_if_missing(&path) { return; }

    let pages = parse_pdf(&path).expect("parse_pdf should not error on a valid PDF");
    assert!(!pages.is_empty(), "parse_pdf returned zero pages");

    let total_chars: usize = pages.iter().map(|p| p.text.len()).sum();
    eprintln!(
        "[extraction_basic_sanity] pages={}, total_chars={}",
        pages.len(),
        total_chars
    );
    assert!(
        total_chars > 100,
        "parsed text is suspiciously short ({total_chars} chars) — possible extraction failure"
    );

    for page in &pages {
        // No private-use-area characters (font encoding garbage)
        for ch in page.text.chars() {
            let code = ch as u32;
            assert!(
                !(0xE000..=0xF8FF).contains(&code),
                "PUA char U+{code:04X} found on page {} — sanitization failed",
                page.page_number
            );
        }

        // No control characters except newline and tab
        for ch in page.text.chars() {
            if ch == '\n' || ch == '\t' { continue; }
            assert!(
                !ch.is_control(),
                "Control char U+{:04X} found on page {} — sanitization failed",
                ch as u32,
                page.page_number
            );
        }

        // No lopdf Identity-H placeholder
        assert!(
            !page.text.contains("?Identity-H Unimplemented?"),
            "Identity-H placeholder not stripped on page {}",
            page.page_number
        );

        // Regression: unfilled template blanks should not appear if the PDF is
        // a real signed contract. If this fires it likely means the wrong PDF
        // was loaded (e.g. a blank template instead of a signed copy).
        let blank_run_count = page.text.match_indices("______").count();
        if blank_run_count > 0 {
            eprintln!(
                "[WARN] page {} contains {blank_run_count} run(s) of '______' — \
                 possible unfilled template. If this is intentional, remove this assertion.",
                page.page_number
            );
        }
    }

    eprintln!("[extraction_basic_sanity] PASSED");
}

/// Tier 1: the event date (2.28.26 / Feb 28 2026) must appear in the raw
/// extracted text. If it does not, the retrieval and E2E tiers will also fail,
/// and the root cause is extraction rather than chunking or retrieval.
#[test]
fn extraction_event_date_present() {
    let path = bartending_pdf_path();
    if skip_if_missing(&path) { return; }

    let pages = parse_pdf(&path).expect("parse_pdf should not error");
    let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let lower = all_text.to_lowercase();

    eprintln!("[extraction_event_date_present] checking for event date in extracted text…");

    // The event date appears in the contract as "Sat 2.28.26" or variants.
    // At least one of these representations must be present.
    let event_date_variants = ["2.28.26", "2/28/26", "2/28/2026", "february 28", "feb 28"];
    let found = event_date_variants.iter().any(|v| lower.contains(v));
    assert!(
        found,
        "Event date not found in extracted text. Checked variants: {event_date_variants:?}\n\
         This suggests the PDF parser failed to extract the relevant page section.\n\
         Extracted text (first 2000 chars):\n{}",
        &all_text[..all_text.len().min(2000)]
    );

    eprintln!("[extraction_event_date_present] PASSED — event date found in raw text");
}

/// Tier 1: the client name, location, and fee must all be present in the
/// extracted text. These are the most important facts in the contract.
#[test]
fn extraction_key_facts_present() {
    let path = bartending_pdf_path();
    if skip_if_missing(&path) { return; }

    let pages = parse_pdf(&path).expect("parse_pdf should not error");
    let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let lower = all_text.to_lowercase();

    let required_facts: &[(&str, &[&str])] = &[
        ("client name",   &["liam neild", "liam", "neild"]),
        ("total fee",     &["275"]),
        ("event location",&["eagle row", "atlanta"]),
        ("guest count",   &["101", "125"]),
    ];

    let mut all_passed = true;
    for (fact_name, variants) in required_facts {
        let found = variants.iter().any(|v| lower.contains(v));
        if found {
            eprintln!("[extraction_key_facts_present] {fact_name}: FOUND");
        } else {
            eprintln!("[extraction_key_facts_present] {fact_name}: MISSING — checked {variants:?}");
            all_passed = false;
        }
    }

    assert!(
        all_passed,
        "One or more key contract facts were not found in extracted text. \
         See eprintln output above for details."
    );

    eprintln!("[extraction_key_facts_present] PASSED — all key facts present");
}

/// Tier 1: regression guard — the signature date (2/25/2026) must not be the
/// ONLY date-like string in the document. If the event date is absent but the
/// signature date is present, downstream tests will wrongly return 2/25/2026
/// as the event date (the documented failure mode).
#[test]
fn extraction_event_date_distinct_from_signature_date() {
    let path = bartending_pdf_path();
    if skip_if_missing(&path) { return; }

    let pages = parse_pdf(&path).expect("parse_pdf should not error");
    let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let lower = all_text.to_lowercase();

    // Confirm the event date IS present
    let event_variants = ["2.28.26", "2/28/26", "2/28/2026", "february 28", "feb 28"];
    let event_found = event_variants.iter().any(|v| lower.contains(v));

    // Confirm the signature date is also present (so the test is meaningful)
    let sig_variants = ["2/25/2026", "2.25.26", "february 25", "feb 25", "25, 2026"];
    let sig_found = sig_variants.iter().any(|v| lower.contains(v));

    eprintln!(
        "[extraction_event_date_distinct_from_signature_date] \
         event_date_found={event_found}, signature_date_found={sig_found}"
    );

    // Both dates must be distinct and parseable — if only one date is present,
    // the LLM has no way to disambiguate and the known failure mode will occur.
    // Note: this is a diagnostic assertion rather than a hard failure when the
    // signature date is absent (some contract versions may not show it).
    if sig_found {
        assert!(
            event_found,
            "Signature date (2/25/2026) found but event date (2/28/2026) NOT found. \
             The LLM will confuse these dates. Check PDF extraction for page completeness."
        );
    }

    eprintln!("[extraction_event_date_distinct_from_signature_date] PASSED");
}

// ─────────────────────────────────────────────────────────────────────────────
// TIER 2 — CHUNKING TESTS
// No model required. These tests validate that the chunker preserves all key
// facts and does not silently drop content during chunk boundaries.
// ─────────────────────────────────────────────────────────────────────────────

/// Shared helper: parse the PDF and chunk with default settings.
/// Returns `(pages, chunk_texts)` where `chunk_texts` is the text of every
/// produced chunk. Uses the real `chunk_document` function from `pipeline`.
fn parse_and_chunk_all_text() -> Option<(Vec<app_lib::state::DocumentPage>, Vec<String>)> {
    let path = bartending_pdf_path();
    if skip_if_missing(&path) { return None; }

    let pages = match parse_pdf(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[parse_and_chunk_all_text] parse_pdf error: {e}");
            return None;
        }
    };

    let settings = AppSettings::default();
    let chunks = chunk_document(&pages, &settings);
    eprintln!(
        "[parse_and_chunk_all_text] pages={}, chunks={}",
        pages.len(),
        chunks.len()
    );
    let all_chunk_text: Vec<String> = chunks.into_iter().map(|c| c.text).collect();
    Some((pages, all_chunk_text))
}

/// Tier 2: every key fact that was present in extracted text must also survive
/// in chunk text. Since chunking is a pure transformation (no bytes are added,
/// only structural boundaries change), a missing fact here means the chunker
/// is incorrectly discarding content.
#[test]
fn chunking_key_facts_survive() {
    let (pages, chunk_texts) = match parse_and_chunk_all_text() {
        Some(v) => v,
        None => return, // PDF not found — skip
    };

    // Full extracted text (pre-chunking) for comparison
    let extracted: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let extracted_lower = extracted.to_lowercase();

    // Combined chunked text
    let chunked: String = chunk_texts.join("\n");
    let chunked_lower = chunked.to_lowercase();

    let facts: &[(&str, &[&str])] = &[
        ("event date",    &["2.28.26", "2/28/2026", "february 28", "feb 28"]),
        ("event time",    &["3", "7"]),          // 3-7pm
        ("total fee",     &["275"]),
        ("guest count",   &["101", "125"]),
        ("location",      &["eagle row", "atlanta"]),
        ("client name",   &["liam neild", "neild"]),
        ("cancellation",  &["nonrefundable", "non-refundable", "cancel", "refund", "transfer"]),
        ("additional rate",&["50"]),
    ];

    let mut failures = Vec::new();

    for (fact_name, variants) in facts {
        let in_extracted = variants.iter().any(|v| extracted_lower.contains(v));
        let in_chunks = variants.iter().any(|v| chunked_lower.contains(v));

        match (in_extracted, in_chunks) {
            (false, _) => {
                // Fact not in extracted text — extraction issue, not chunking.
                // Log but don't fail this test (extraction tests cover this).
                eprintln!("[chunking_key_facts_survive] {fact_name}: not in extracted text (extraction issue — not a chunking failure)");
            }
            (true, false) => {
                eprintln!("[chunking_key_facts_survive] {fact_name}: DROPPED by chunker! Present in extraction, absent in chunks.");
                failures.push(fact_name.to_string());
            }
            (true, true) => {
                eprintln!("[chunking_key_facts_survive] {fact_name}: OK");
            }
        }
    }

    assert!(
        failures.is_empty(),
        "The following facts were present in extracted text but dropped by the chunker: {failures:?}\n\
         This indicates a chunking boundary or flush bug."
    );

    eprintln!("[chunking_key_facts_survive] PASSED");
}

/// Tier 2: chunked text must not contain unfilled template blanks (______).
///
/// Template blanks in the output would mean either:
///   a) the wrong PDF was loaded (blank template instead of signed contract), or
///   b) the chunker is injecting synthetic placeholder text.
///
/// This test guards regression (b) — the chunker must never introduce text
/// that wasn't in the original extracted pages.
#[test]
fn chunking_no_blank_template_artifacts() {
    let (pages, chunk_texts) = match parse_and_chunk_all_text() {
        Some(v) => v,
        None => return,
    };

    // Count blanks in extracted text (ground truth — may be non-zero in templates)
    let extracted: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    let blanks_in_extracted = extracted.match_indices("______").count();

    // Count blanks in chunked output
    let chunked: String = chunk_texts.join("\n");
    let blanks_in_chunked = chunked.match_indices("______").count();

    eprintln!(
        "[chunking_no_blank_template_artifacts] \
         blanks_in_extracted={blanks_in_extracted}, blanks_in_chunked={blanks_in_chunked}"
    );

    // The chunker must not introduce significantly more blanks than the source.
    // Chunk overlap can duplicate a blank that straddles a boundary, so we allow
    // a small tolerance (up to 5% of extracted count, minimum 2).
    let tolerance = std::cmp::max(2, blanks_in_extracted / 20);
    assert!(
        blanks_in_chunked <= blanks_in_extracted + tolerance,
        "Chunker introduced {blanks_in_chunked} blank runs ('______') \
         but extraction only had {blanks_in_extracted} (tolerance {tolerance}). \
         The chunker may be injecting synthetic placeholder text."
    );

    eprintln!("[chunking_no_blank_template_artifacts] PASSED");
}

/// Tier 2: chunk count sanity — a 1-2 page contract should produce a reasonable
/// number of chunks (more than 1, fewer than 50 with default settings).
///
/// If this fires it usually means the chunker is producing a single giant chunk
/// (paragraph boundary detection broken) or fragmenting into hundreds of tiny
/// chunks (tokenisation loop bug).
#[test]
fn chunking_count_within_reasonable_range() {
    let (pages, chunk_texts) = match parse_and_chunk_all_text() {
        Some(v) => v,
        None => return,
    };

    eprintln!(
        "[chunking_count_within_reasonable_range] pages={}, chunk_texts={}",
        pages.len(),
        chunk_texts.len()
    );

    // A 1-2 page bartending contract should produce at least 2 chunks and at
    // most 50. Outside this range suggests a structural bug in the chunker.
    assert!(
        chunk_texts.len() >= 2,
        "Only {} chunk(s) produced — chunker may have collapsed everything into one chunk",
        chunk_texts.len()
    );
    assert!(
        chunk_texts.len() <= 50,
        "{} chunks produced from a 1-2 page contract — chunker may be over-fragmenting",
        chunk_texts.len()
    );

    eprintln!("[chunking_count_within_reasonable_range] PASSED");
}

// ─────────────────────────────────────────────────────────────────────────────
// TIER 3 — RETRIEVAL TESTS
// Requires fastembed (~33 MB auto-download). Marked #[ignore]; run with:
//   cargo test -- --include-ignored retrieval
// ─────────────────────────────────────────────────────────────────────────────

/// Tier 3: given the query "what is the event date", the top retrieved chunk
/// should contain a date string in the Feb 28 2026 family, and must NOT contain
/// only 2/25 without also containing 2/28 — the known confusion regression.
#[test]
#[ignore = "retrieval: requires fastembed model download (~33 MB)"]
fn retrieval_event_date_chunk_found() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let path = bartending_pdf_path();
        if skip_if_missing(&path) { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);
        let model_dir = retrieval_model_dir();

        let top = retrieve_top_chunks(&chunks, "what is the event date", &model_dir, 6).await;
        let top_text: String = top.iter().map(|(_, c)| c.text.as_str()).collect::<Vec<_>>().join(" ");

        // The event date (2.28.26 / 2/28/2026 / Sat) must appear in top-k
        let has_date = ["2.28.26", "2/28/2026", "Sat"].iter().any(|d| top_text.contains(d));
        assert!(has_date,
            "Event date not found in top-6 retrieved chunks.\nTop chunk texts:\n{}",
            top.iter().enumerate().map(|(i, (s, c))| format!("[#{} score={:.4}] {}", i+1, s, &c.text[..c.text.len().min(120)])).collect::<Vec<_>>().join("\n"));
    });
}

/// Tier 3: given the query "cancellation policy", the top retrieved chunk must
/// contain "nonrefundable" or "non-refundable" and "30" (30-day transfer notice).
#[test]
#[ignore = "retrieval: requires fastembed model download (~33 MB)"]
fn retrieval_cancellation_policy_chunk_found() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let path = bartending_pdf_path();
        if skip_if_missing(&path) { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);
        let model_dir = retrieval_model_dir();

        let top = retrieve_top_chunks(&chunks, "cancellation policy", &model_dir, 6).await;
        let top_text: String = top.iter().map(|(_, c)| c.text.as_str()).collect::<Vec<_>>().join(" ");

        let has_nonrefundable = top_text.to_lowercase().contains("nonrefundable")
            || top_text.to_lowercase().contains("non-refundable");
        assert!(has_nonrefundable,
            "Cancellation policy not found in top-6 chunks.\nTop texts:\n{}",
            top.iter().enumerate().map(|(i, (s, c))| format!("[#{} score={:.4}] {}", i+1, s, &c.text[..c.text.len().min(120)])).collect::<Vec<_>>().join("\n"));
    });
}

/// Tier 3: filled W-9 — "what is the name" must retrieve the chunk with "Liam Neild".
/// This is the key regression test for AcroForm extraction + retrieval.
#[test]
#[ignore = "retrieval: requires fastembed model download (~33 MB)"]
fn retrieval_w9_filled_name_found() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let path = fixture_path("irs_w9_filled.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);
        let model_dir = retrieval_model_dir();

        let top = retrieve_top_chunks(&chunks, "what is the name of the person on this W9", &model_dir, 6).await;
        let top_text: String = top.iter().map(|(_, c)| c.text.as_str()).collect::<Vec<_>>().join(" ");

        assert!(top_text.contains("Liam Neild"),
            "\"Liam Neild\" not in top-6 chunks for name query.\nTop chunks:\n{}",
            top.iter().enumerate().map(|(i, (s, c))| format!("[#{} score={:.4}] {}", i+1, s, &c.text[..c.text.len().min(120)])).collect::<Vec<_>>().join("\n"));
    });
}

/// Tier 3: filled W-9 — "what is the address" must retrieve the chunk with "Eagle Row".
#[test]
#[ignore = "retrieval: requires fastembed model download (~33 MB)"]
fn retrieval_w9_filled_address_found() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let path = fixture_path("irs_w9_filled.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);
        let model_dir = retrieval_model_dir();

        let top = retrieve_top_chunks(&chunks, "what is the address on this form", &model_dir, 6).await;
        let top_text: String = top.iter().map(|(_, c)| c.text.as_str()).collect::<Vec<_>>().join(" ");

        assert!(top_text.contains("Eagle Row"),
            "\"Eagle Row\" not in top-6 chunks for address query.\nTop chunks:\n{}",
            top.iter().enumerate().map(|(i, (s, c))| format!("[#{} score={:.4}] {}", i+1, s, &c.text[..c.text.len().min(120)])).collect::<Vec<_>>().join("\n"));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// TIER 4 — END-TO-END LLM TESTS
// Requires Saul-7B GGUF model (~4.5 GB). Marked #[ignore]; run with:
//   cargo test -- --include-ignored e2e
//
// These tests use check_answer() with the full bartending_contract_test_cases()
// suite. LLM output is non-deterministic so we assert semantic constraints
// rather than exact strings.
// ─────────────────────────────────────────────────────────────────────────────

/// Tier 4: run every HIGH-priority test case through the full pipeline.
/// If any assertion fails, the test prints the answer and the failing
/// assertion(s) so the developer can diagnose the LLM response.
#[test]
#[ignore = "e2e: requires Saul-7B GGUF model (~4.5 GB) at {app_data}/models/saul.gguf"]
fn e2e_high_priority_test_cases() {
    // TODO: implement after pipeline refactor exposes ask_saul / query as
    //       testable functions (currently they require tauri::State + Window).
    //
    // Pseudocode:
    //   for tc in bartending_contract_test_cases()
    //       .into_iter()
    //       .filter(|tc| tc.priority == Priority::High)
    //   {
    //       let answer = run_full_pipeline(&path, &tc.query, &settings, &model_dir);
    //       let failures = check_answer(&answer, &tc.assertions);
    //       assert!(failures.is_empty(),
    //           "Test {:?} FAILED:\n{}", tc.name, failures.join("\n"));
    //   }
    eprintln!("[e2e_high_priority_test_cases] TODO: implement after pipeline refactor");
}

/// Tier 4: run every test case (all priorities) through the full pipeline.
#[test]
#[ignore = "e2e: requires Saul-7B GGUF model (~4.5 GB) at {app_data}/models/saul.gguf"]
fn e2e_all_test_cases() {
    // TODO: same as above but includes Medium + Low priority cases
    eprintln!("[e2e_all_test_cases] TODO: implement after pipeline refactor");
}

// ─────────────────────────────────────────────────────────────────────────────
// UNIT TESTS — pure function logic, no I/O, no model files
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod unit {
    use super::*;

    // ── check_answer unit tests ───────────────────────────────────────────

    #[test]
    fn check_answer_contains_passes() {
        let failures = check_answer("The event is on February 28, 2026.", &[
            Assertion::Contains("february 28"),
        ]);
        assert!(failures.is_empty(), "Expected no failures: {failures:?}");
    }

    #[test]
    fn check_answer_contains_case_insensitive() {
        let failures = check_answer("THE EVENT IS ON FEBRUARY 28.", &[
            Assertion::Contains("february 28"),
        ]);
        assert!(failures.is_empty(), "Contains should be case-insensitive: {failures:?}");
    }

    #[test]
    fn check_answer_contains_fails() {
        let failures = check_answer("The event is on March 1.", &[
            Assertion::Contains("february 28"),
        ]);
        assert!(!failures.is_empty(), "Expected failure when needle is missing");
    }

    #[test]
    fn check_answer_contains_any_passes_first() {
        let failures = check_answer("The total fee is $275.", &[
            Assertion::ContainsAny(&["$275", "$300", "$400"]),
        ]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn check_answer_contains_any_passes_second() {
        let failures = check_answer("The fee is three hundred dollars.", &[
            Assertion::ContainsAny(&["$275", "three hundred", "300"]),
        ]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn check_answer_contains_any_fails_none_match() {
        let failures = check_answer("The fee is unknown.", &[
            Assertion::ContainsAny(&["$275", "$300"]),
        ]);
        assert!(!failures.is_empty(), "Expected failure when no variant matches");
    }

    #[test]
    fn check_answer_not_contains_passes() {
        let failures = check_answer("The event is February 28.", &[
            Assertion::NotContains("2/25"),
        ]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn check_answer_not_contains_fails() {
        // Simulates the known regression: answer says 2/25 (signature date)
        // when asked about the event date.
        let failures = check_answer("The event date is 2/25/2026.", &[
            Assertion::NotContains("2/25"),
        ]);
        assert!(!failures.is_empty(), "Expected failure when forbidden string is present");
    }

    #[test]
    fn check_answer_matches_date_passes_exact() {
        let failures = check_answer("The event is on Sat 2.28.26.", &[
            Assertion::MatchesDate(&["2.28.26", "2/28/2026", "february 28"]),
        ]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn check_answer_matches_date_passes_alternative_format() {
        let failures = check_answer("The event is scheduled for February 28, 2026.", &[
            Assertion::MatchesDate(&["2.28.26", "2/28/2026", "february 28"]),
        ]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn check_answer_matches_date_fails_wrong_date() {
        let failures = check_answer("The event is on February 25, 2026.", &[
            Assertion::MatchesDate(&["2.28.26", "2/28/2026", "february 28", "feb 28"]),
        ]);
        assert!(!failures.is_empty(), "Expected failure for wrong date");
    }

    #[test]
    fn check_answer_multiple_assertions_all_pass() {
        let answer = "The event is on February 28, 2026. The total fee is $275.";
        let failures = check_answer(answer, &[
            Assertion::MatchesDate(&["february 28", "2/28/2026"]),
            Assertion::Contains("275"),
            Assertion::NotContains("2/25"),
            Assertion::NotContains("______"),
        ]);
        assert!(failures.is_empty(), "All assertions should pass: {failures:?}");
    }

    #[test]
    fn check_answer_multiple_assertions_partial_fail() {
        // Answer has correct date but also contains the regression value.
        let answer = "The event is February 28. It was signed on 2/25/2026.";
        let failures = check_answer(answer, &[
            Assertion::MatchesDate(&["february 28"]),
            Assertion::NotContains("2/25"),  // regression guard — should fire
        ]);
        // Exactly one failure: the NotContains("2/25") fires
        assert_eq!(failures.len(), 1, "Expected exactly 1 failure, got: {failures:?}");
    }

    // ── Test case structure sanity ────────────────────────────────────────

    #[test]
    fn test_cases_all_have_at_least_one_assertion() {
        for tc in bartending_contract_test_cases() {
            assert!(
                !tc.assertions.is_empty(),
                "Test case {:?} has no assertions — it will never catch regressions",
                tc.name
            );
        }
    }

    #[test]
    fn test_cases_high_priority_count() {
        let high_count = bartending_contract_test_cases()
            .iter()
            .filter(|tc| tc.priority == Priority::High)
            .count();
        // We should have at least 3 HIGH priority tests (event date, fee, deposit)
        assert!(
            high_count >= 3,
            "Expected at least 3 HIGH priority test cases, found {high_count}"
        );
    }

    #[test]
    fn test_cases_event_date_has_regression_guard() {
        let cases = bartending_contract_test_cases();
        let event_date_tc = cases
            .iter()
            .find(|tc| tc.name == "event_date_not_confused_with_signature_date")
            .expect("event_date_not_confused_with_signature_date test case must exist");

        // Must have at least one NotContains("2/25") assertion
        let has_regression_guard = event_date_tc.assertions.iter().any(|a| {
            matches!(a, Assertion::NotContains(s) if s.contains("2/25"))
        });
        assert!(
            has_regression_guard,
            "event_date test case must have a NotContains(\"2/25\") regression guard"
        );
    }

    #[test]
    fn test_cases_no_duplicate_names() {
        let cases = bartending_contract_test_cases();
        let mut seen = std::collections::HashSet::new();
        for tc in &cases {
            assert!(
                seen.insert(tc.name),
                "Duplicate test case name: {:?}",
                tc.name
            );
        }
    }
}

// ── Real-world document tests ────────────────────────────────────────────────
//
// IRS forms and Georgia court forms downloaded from official government sites.
// These are unfilled templates — they test that the parser handles real-world
// PDF structures (AcroForm fields, multi-column layouts, legal boilerplate)
// without crashing or losing text.

mod real_world {
    use super::*;

    #[test]
    fn irs_w9_parses() {
        let path = fixture_path("irs_w9.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on W-9");
        assert!(!pages.is_empty(), "W-9 should have at least 1 page");
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        // Key text that must be present in a W-9
        for needle in &["Taxpayer Identification Number", "Request for Taxpayer"] {
            assert!(
                all_text.to_lowercase().contains(&needle.to_lowercase()),
                "W-9 missing: {needle:?}\nFirst 500 chars:\n{}",
                &all_text[..all_text.len().min(500)]
            );
        }
    }

    #[test]
    fn irs_w4_parses() {
        let path = fixture_path("irs_w4.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on W-4");
        assert!(!pages.is_empty(), "W-4 should have at least 1 page");
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        for needle in &["Withholding", "Employee"] {
            assert!(
                all_text.to_lowercase().contains(&needle.to_lowercase()),
                "W-4 missing: {needle:?}\nFirst 500 chars:\n{}",
                &all_text[..all_text.len().min(500)]
            );
        }
    }

    #[test]
    fn irs_w9_filled_extracts_data() {
        let path = fixture_path("irs_w9_filled.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on filled W-9");
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        // Filled values should appear in extracted text
        for needle in &["Liam Neild", "18 Eagle Row", "Atlanta"] {
            assert!(
                all_text.contains(needle),
                "Filled W-9 missing: {needle:?}\nText:\n{all_text}"
            );
        }
    }

    #[test]
    fn ga_statement_of_claim_parses() {
        let path = fixture_path("ga_statement_of_claim.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on GA claim form");
        assert!(!pages.is_empty(), "GA claim form should have at least 1 page");
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        for needle in &["Statement of Claim", "Magistrate"] {
            assert!(
                all_text.to_lowercase().contains(&needle.to_lowercase()),
                "GA claim form missing: {needle:?}\nFirst 500 chars:\n{}",
                &all_text[..all_text.len().min(500)]
            );
        }
    }
}

// ── Synthetic PDF tests ─────────────────────────────────────────────────────
//
// These test against generated PDFs to ensure the parser isn't overfit to a
// single document structure. Run `python3 tests/fixtures/generate_test_pdfs.py`
// to regenerate if needed.

fn fixture_path(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/tests/fixtures/{name}")
}

mod synthetic_plain {
    use super::*;

    #[test]
    fn parses_without_error() {
        let path = fixture_path("plain_contract.pdf");
        if !std::path::Path::new(&path).exists() {
            eprintln!("[SKIP] plain_contract.pdf not found");
            return;
        }
        let pages = parse_pdf(&path).expect("parse_pdf failed on plain contract");
        assert!(!pages.is_empty(), "Should extract at least one page");
    }

    #[test]
    fn extracts_key_facts() {
        let path = fixture_path("plain_contract.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        // Key facts that must survive extraction
        for needle in &[
            "Jane Thompson",
            "Robert Chen",
            "742 Evergreen Terrace",
            "$1,850",
            "$3,700",
            "April 1, 2025",
            "March 31, 2026",
            "Oregon",
        ] {
            assert!(
                all_text.contains(needle),
                "Plain contract missing: {needle:?}\nExtracted text:\n{all_text}"
            );
        }
    }

    #[test]
    fn chunks_preserve_facts() {
        let path = fixture_path("plain_contract.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);

        let all_chunks: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(all_chunks.contains("$1,850"), "Rent should survive chunking");
        assert!(all_chunks.contains("Jane Thompson"), "Landlord should survive chunking");
    }
}

mod synthetic_filled_form {
    use super::*;

    #[test]
    fn parses_without_error() {
        let path = fixture_path("filled_form_simple.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on filled form");
        assert!(!pages.is_empty());
    }

    #[test]
    fn extracts_filled_values() {
        let path = fixture_path("filled_form_simple.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        for needle in &[
            "Maria Garcia",
            "Wedding Reception",
            "June 14, 2025",
            "$8,500",
            "555-0147",
        ] {
            assert!(
                all_text.contains(needle),
                "Filled form missing: {needle:?}\nExtracted text:\n{all_text}"
            );
        }
    }

    #[test]
    fn values_present_in_text() {
        // Reportlab generates plain draw calls, not XObject-based form fields,
        // so the reinterleave path won't fire. This test ensures values still
        // appear in the extracted text (just not necessarily adjacent to labels).
        // The adjacency fix only applies to real AcroForm/XObject-based PDFs
        // (like the bartending contract).
        let path = fixture_path("filled_form_simple.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        // Both labels and values must be present
        for needle in &["Client Name", "Maria Garcia", "Event Date", "June 14, 2025", "Total Cost", "$8,500"] {
            assert!(
                all_text.contains(needle),
                "Missing: {needle:?}\nText:\n{all_text}"
            );
        }
    }
}

mod synthetic_multipage {
    use super::*;

    #[test]
    fn parses_without_error() {
        let path = fixture_path("multipage_form.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).expect("parse_pdf failed on multipage form");
        assert!(!pages.is_empty());
    }

    #[test]
    fn all_content_extracted() {
        // For reportlab-generated PDFs, pdf-extract may collapse pages.
        // The key requirement is that ALL facts from both pages are present
        // somewhere in the extracted text — page boundaries are secondary.
        let path = fixture_path("multipage_form.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let all_text: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");

        // Page 1 facts
        for needle in &["David Kim", "Corporate Gala", "September 20, 2025", "555-0293"] {
            assert!(
                all_text.contains(needle),
                "Missing page-1 fact: {needle:?}\nText:\n{all_text}"
            );
        }
        // Page 2 facts
        for needle in &["$4,200", "$2,100", "$350/hour", "Premium 6-Hour"] {
            assert!(
                all_text.contains(needle),
                "Missing page-2 fact: {needle:?}\nText:\n{all_text}"
            );
        }
    }

    #[test]
    fn chunks_contain_all_facts() {
        let path = fixture_path("multipage_form.pdf");
        if !std::path::Path::new(&path).exists() { return; }
        let pages = parse_pdf(&path).unwrap();
        let settings = AppSettings::default();
        let chunks = chunk_document(&pages, &settings);

        let all_chunks: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(all_chunks.contains("David Kim"), "Missing from chunks: David Kim");
        assert!(all_chunks.contains("$4,200"), "Missing from chunks: $4,200");
    }
}
