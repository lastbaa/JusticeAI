use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResult {
    pub passed: bool,
    pub assertion_type: AssertionType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    CitationFormat,
    CitationFilename,
    CitationPage,
    NumberExactness,
    Blocklist,
    Hallucination,
    FabricatedEntity,
    Misattribution,
}

/// Common English stopwords used to filter key content words.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
    "being", "have", "has", "had", "do", "does", "did", "will", "would",
    "could", "should", "may", "might", "can", "this", "that", "these",
    "those", "it", "its", "not", "no", "nor", "if", "then", "than",
    "so", "as", "any", "all", "each", "every", "such", "other", "into",
    "upon", "under", "over", "between", "through", "after", "before",
    "shall", "hereby", "thereof", "herein", "pursuant", "notwithstanding",
    "hereinafter", "therein", "thereto", "whereas", "hereunder", "hereof",
    "also", "about", "which", "when", "where", "there", "their", "they",
    "them", "what", "who", "whom", "your", "you", "more", "most", "some",
    "only", "very", "just", "still", "here", "both", "same", "while",
];

/// Extract key content words from text: words > 3 chars that are not stopwords.
/// Uses > 3 (min 4 chars) to retain important legal terms like "tort", "lien",
/// "deed", "writ", "void", "suit", "jury", "fact", "rule".
fn extract_key_words(text: &str) -> HashSet<String> {
    let stopset: HashSet<&str> = STOPWORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3 && !stopset.contains(w))
        .map(|w| w.to_string())
        .collect()
}

/// Verify citations match `[filename, p. N]` format, at least one exists,
/// and optionally that cited filenames appear in `known_files`.
pub fn check_citations(answer: &str, known_files: Option<&[&str]>) -> Vec<AssertionResult> {
    let mut results = Vec::new();
    let cite_re = Regex::new(r"\[([^,\[\]]+),\s*p\.\s*(\d+)\]").unwrap();
    let captures: Vec<_> = cite_re.captures_iter(answer).collect();

    // Check every bracket group that looks citation-like but may be malformed
    let bracket_re = Regex::new(r"\[[^\[\]]{2,}\]").unwrap();
    for m in bracket_re.find_iter(answer) {
        let text = m.as_str();
        let valid = cite_re.is_match(text);
        results.push(AssertionResult {
            passed: valid,
            assertion_type: AssertionType::CitationFormat,
            message: if valid {
                format!("Citation formatted correctly: {text}")
            } else {
                format!("Citation may be incomplete: {text}")
            },
        });
    }

    if let Some(files) = known_files {
        for cap in &captures {
            let cited = cap[1].trim();
            let found = files.iter().any(|f| *f == cited);
            results.push(AssertionResult {
                passed: found,
                assertion_type: AssertionType::CitationFilename,
                message: if found {
                    format!("Source document found: {cited}")
                } else {
                    format!("Referenced document not recognized: {cited}")
                },
            });
        }
    }

    results
}

/// Verify that every expected number/date string appears verbatim in the answer.
pub fn check_number_exactness(answer: &str, expected: &[&str]) -> Vec<AssertionResult> {
    expected
        .iter()
        .map(|num| {
            let found = answer.contains(num);
            AssertionResult {
                passed: found,
                assertion_type: AssertionType::NumberExactness,
                message: if found {
                    format!("Number matches source: {num}")
                } else {
                    format!("Number may differ from source: {num}")
                },
            }
        })
        .collect()
}

/// Fail if any blocked term appears in the answer (case-insensitive).
pub fn check_blocklist(answer: &str, blocked: &[&str]) -> Vec<AssertionResult> {
    let lower = answer.to_lowercase();
    blocked
        .iter()
        .map(|term| {
            let found = lower.contains(&term.to_lowercase());
            AssertionResult {
                passed: !found,
                assertion_type: AssertionType::Blocklist,
                message: if found {
                    format!("Contains restricted term: \"{term}\"")
                } else {
                    format!("No restricted terms found")
                },
            }
        })
        .collect()
}

/// Extract sentences containing numbers, capitalized proper nouns, strong legal
/// qualifiers, or negation+legal patterns from the answer, then verify each
/// appears (substring) in at least one source chunk.
pub fn check_no_hallucination(answer: &str, chunks: &[&str]) -> Vec<AssertionResult> {
    // Original pattern: sentences with numbers or proper nouns
    let claim_re = Regex::new(
        r"(?:^|[.!?]\s+)([A-Z][^.!?]*(?:\d[\d,./%$]+|[A-Z][a-z]{2,})[^.!?]*[.!?])"
    ).unwrap();

    let legal_qualifiers = [
        "perpetual", "unlimited", "exclusive", "irrevocable", "non-refundable",
        "binding", "mandatory", "prohibited", "waived", "forfeited", "void",
        "terminated", "expired", "renewed", "amended", "assignable",
        "non-transferable", "confidential", "material",
    ];

    let negation_re = Regex::new(
        r"\b(no|not|never|none|cannot|shall not|must not|may not)\b"
    ).unwrap();

    // Regex to strip inline citations before matching against source text
    let cite_strip = Regex::new(r"\s*\[[^\]]*\]").unwrap();

    // Split answer into sentences for qualifier/negation checks
    let sentence_split_re = Regex::new(r"(?s)([^.!?\n]+[.!?])").unwrap();
    let all_sentences: Vec<&str> = sentence_split_re
        .find_iter(answer)
        .map(|m| m.as_str().trim())
        .filter(|s| s.len() >= 15)
        .collect();

    let mut results = Vec::new();
    let mut checked_sentences: HashSet<String> = HashSet::new();

    // 1. Original: sentences with numbers or proper nouns (token-level check)
    for cap in claim_re.captures_iter(answer) {
        let sentence = cap[1].trim();
        let clean = cite_strip.replace_all(sentence, "").trim().to_string();
        if clean.len() < 15 {
            continue;
        }
        checked_sentences.insert(clean.clone());

        let token_re = Regex::new(r"\d[\d,./%$]+|[A-Z][a-z]{2,}").unwrap();
        let tokens: Vec<&str> = token_re.find_all(&clean).map(|m| m.as_str()).collect();
        if tokens.is_empty() {
            continue;
        }
        // Use a 60% grounding ratio instead of requiring ALL tokens to match.
        // Minor format differences (date formats, casing) caused false positives
        // when requiring 100% match.
        let grounded_count = tokens.iter()
            .filter(|tok| chunks.iter().any(|c| c.contains(*tok)))
            .count();
        let ratio = grounded_count as f64 / tokens.len() as f64;
        let grounded = ratio >= 0.6;
        results.push(AssertionResult {
            passed: grounded,
            assertion_type: AssertionType::Hallucination,
            message: if grounded {
                format!("Verified in sources: \"{}\"", truncate(&clean, 90))
            } else {
                format!(
                    "Could not fully verify this claim in your documents: \"{}\"",
                    truncate(&clean, 90)
                )
            },
        });
    }

    // 2. Qualitative claims: sentences with strong legal qualifiers
    let chunks_lower: Vec<String> = chunks.iter().map(|c| c.to_lowercase()).collect();

    for sentence in &all_sentences {
        let clean = cite_strip.replace_all(sentence, "").trim().to_string();
        if clean.len() < 15 || checked_sentences.contains(&clean) {
            continue;
        }
        let lower = clean.to_lowercase();
        let has_qualifier = legal_qualifiers.iter().any(|q| lower.contains(q));
        if !has_qualifier {
            continue;
        }
        checked_sentences.insert(clean.clone());

        let key_words = extract_key_words(&clean);
        if key_words.is_empty() {
            continue;
        }
        let grounded_count = key_words.iter()
            .filter(|w| chunks_lower.iter().any(|c| c.contains(w.as_str())))
            .count();
        let ratio = grounded_count as f64 / key_words.len() as f64;
        let passed = ratio >= 0.5;
        results.push(AssertionResult {
            passed,
            assertion_type: AssertionType::Hallucination,
            message: if passed {
                format!("Verified in sources: \"{}\"", truncate(&clean, 90))
            } else {
                format!(
                    "Strong claim not fully supported by your documents: \"{}\"",
                    truncate(&clean, 90)
                )
            },
        });
    }

    // 3. Negation claims: sentences with negation + legal terms
    let legal_context_terms = [
        "contract", "agreement", "clause", "term", "party", "obligation",
        "right", "liability", "warranty", "indemnity", "license", "payment",
        "notice", "termination", "breach", "remedy", "dispute", "claim",
        "damages", "penalty", "consent", "approval", "assignment",
    ];

    for sentence in &all_sentences {
        let clean = cite_strip.replace_all(sentence, "").trim().to_string();
        if clean.len() < 15 || checked_sentences.contains(&clean) {
            continue;
        }
        let lower = clean.to_lowercase();
        let has_negation = negation_re.is_match(&lower);
        let has_legal = legal_context_terms.iter().any(|t| lower.contains(t));
        if !has_negation || !has_legal {
            continue;
        }
        checked_sentences.insert(clean.clone());

        let key_words = extract_key_words(&clean);
        if key_words.is_empty() {
            continue;
        }
        let grounded_count = key_words.iter()
            .filter(|w| chunks_lower.iter().any(|c| c.contains(w.as_str())))
            .count();
        let ratio = grounded_count as f64 / key_words.len() as f64;
        let passed = ratio >= 0.5;
        results.push(AssertionResult {
            passed,
            assertion_type: AssertionType::Hallucination,
            message: if passed {
                format!("Verified in sources: \"{}\"", truncate(&clean, 90))
            } else {
                format!(
                    "This statement could not be confirmed in your documents: \"{}\"",
                    truncate(&clean, 90)
                )
            },
        });
    }

    if results.is_empty() {
        results.push(AssertionResult {
            passed: true,
            assertion_type: AssertionType::Hallucination,
            message: "No specific claims to verify".into(),
        });
    }
    results
}

/// Detect fabricated legal entities: court names, jurisdictions, case parties,
/// case numbers, statute citations, and specific legal claims that don't appear
/// in any source chunk. This is a stronger check than `check_no_hallucination`
/// because it specifically targets the high-risk patterns that LLMs fabricate
/// most often in legal contexts.
pub fn check_fabricated_entities(answer: &str, chunks: &[&str]) -> Vec<AssertionResult> {
    let mut results = Vec::new();
    let all_sources = chunks.join(" ");
    let sources_lower = all_sources.to_lowercase();

    // 1. Court names — "Superior Court of X", "District Court of X", etc.
    let court_re = Regex::new(
        r"(?i)(Superior|District|Circuit|Supreme|Municipal|Family|Probate|Bankruptcy|Appellate|County)\s+Court\s+of\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,3})"
    ).unwrap();
    for cap in court_re.captures_iter(answer) {
        let full = cap[0].trim();
        let full_lower = full.to_lowercase();
        let grounded = sources_lower.contains(&full_lower);
        if !grounded {
            results.push(AssertionResult {
                passed: false,
                assertion_type: AssertionType::FabricatedEntity,
                message: format!("Court name not found in your documents: \"{}\"", truncate(full, 80)),
            });
        }
    }

    // 2. "County of X" — fabricated jurisdictions
    let county_re = Regex::new(r"County\s+of\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,3})").unwrap();
    for cap in county_re.captures_iter(answer) {
        let full = cap[0].trim();
        let full_lower = full.to_lowercase();
        if !sources_lower.contains(&full_lower) {
            results.push(AssertionResult {
                passed: false,
                assertion_type: AssertionType::FabricatedEntity,
                message: format!("Jurisdiction not found in your documents: \"{}\"", truncate(full, 80)),
            });
        }
    }

    // 3. Case numbers — "Case No. X", "Docket No. X"
    let case_no_re = Regex::new(r"(?i)(Case|Docket|Civil Action)\s+No\.\s*[\w-]+").unwrap();
    for m in case_no_re.find_iter(answer) {
        let text = m.as_str();
        let text_lower = text.to_lowercase();
        if !sources_lower.contains(&text_lower) {
            results.push(AssertionResult {
                passed: false,
                assertion_type: AssertionType::FabricatedEntity,
                message: format!("Case number not found in your documents: \"{}\"", text),
            });
        }
    }

    // 4. "plaintiff" / "defendant" with specific names not in sources
    let party_re = Regex::new(
        r"(?i)(?:the\s+)?(plaintiff|defendant|petitioner|respondent|appellant|appellee)(?:'s?\s+|\s+)([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)"
    ).unwrap();
    for cap in party_re.captures_iter(answer) {
        let party_name = cap[2].trim();
        if party_name.len() >= 4 && !sources_lower.contains(&party_name.to_lowercase()) {
            results.push(AssertionResult {
                passed: false,
                assertion_type: AssertionType::FabricatedEntity,
                message: format!("Party name not found in your documents: \"{}\"", party_name),
            });
        }
    }

    // 5. Specific statute citations not in documents (e.g., "§ 1234", "42 U.S.C. § 1983")
    let statute_re = Regex::new(r"\d+\s+(?:U\.?S\.?C\.?|C\.?F\.?R\.?)\s*§\s*\d+").unwrap();
    for m in statute_re.find_iter(answer) {
        let text = m.as_str();
        let text_lower = text.to_lowercase().replace(' ', "");
        let source_norm = sources_lower.replace(' ', "");
        if !source_norm.contains(&text_lower) {
            results.push(AssertionResult {
                passed: false,
                assertion_type: AssertionType::FabricatedEntity,
                message: format!("Statute citation not found in your documents: \"{}\"", text),
            });
        }
    }

    results
}

/// Validate page numbers in citations against known page counts.
/// Returns a list of violations for citations referencing pages beyond a file's
/// actual page count.
pub fn check_citation_pages(
    answer: &str,
    file_page_counts: &HashMap<String, usize>,
) -> Vec<String> {
    let mut violations = Vec::new();
    let cite_re = Regex::new(r"\[([^,\[\]]+),\s*p\.\s*(\d+)\]").unwrap();

    for cap in cite_re.captures_iter(answer) {
        let filename = cap[1].trim();
        let page: usize = match cap[2].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if let Some(&total) = file_page_counts.get(filename) {
            if page > total {
                violations.push(format!(
                    "Citation [{}, p. {}] invalid: file has only {} pages",
                    filename, page, total
                ));
            }
        }
        // If file not in map, skip (can't validate)
    }

    violations
}

/// Check if cited facts actually appear in the cited source file.
/// For each citation and the sentence it appears in, extracts key terms and
/// checks whether they appear in chunks from that specific file. If <30% of
/// key terms are found, flags as potential misattribution.
pub fn check_misattribution(
    answer: &str,
    chunks_by_file: &HashMap<String, Vec<String>>,
    threshold: f64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let cite_re = Regex::new(r"\[([^,\[\]]+),\s*p\.\s*(\d+)\]").unwrap();
    let cite_strip = Regex::new(r"\s*\[[^\]]*\]").unwrap();

    // Neutralize periods inside brackets so "p. N" doesn't cause sentence breaks.
    // Replace each char inside [...] with a space (preserves byte offsets).
    let mut neutralized: Vec<u8> = answer.bytes().collect();
    let mut in_bracket = false;
    for b in neutralized.iter_mut() {
        match *b {
            b'[' => in_bracket = true,
            b']' => in_bracket = false,
            b'.' | b'!' | b'?' if in_bracket => *b = b' ',
            _ => {}
        }
    }
    let neutralized_str = String::from_utf8_lossy(&neutralized);
    let sentence_re = Regex::new(r"(?s)([^.!?\n]+[.!?])").unwrap();

    // Find sentence boundaries in neutralized text, then map back to original
    let sentences: Vec<&str> = sentence_re
        .find_iter(&neutralized_str)
        .filter_map(|m| answer.get(m.start()..m.end()))
        .map(|s| s.trim())
        .collect();

    for sentence in &sentences {
        for cap in cite_re.captures_iter(sentence) {
            let filename = cap[1].trim().to_string();
            let file_chunks = match chunks_by_file.get(&filename) {
                Some(c) => c,
                None => continue, // Can't validate without chunks
            };

            // Strip citations from sentence to get the claim text
            let clean = cite_strip.replace_all(sentence, "").trim().to_string();
            let key_words = extract_key_words(&clean);
            if key_words.is_empty() {
                continue;
            }

            let file_text: String = file_chunks.iter()
                .map(|c| c.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");

            let grounded_count = key_words.iter()
                .filter(|w| file_text.contains(w.as_str()))
                .count();
            let ratio = grounded_count as f64 / key_words.len() as f64;

            if ratio < threshold {
                warnings.push(format!(
                    "Potential misattribution: sentence cites [{}] but only {:.0}% of key terms found in that file: \"{}\"",
                    filename,
                    ratio * 100.0,
                    truncate(&clean, 80)
                ));
            }
        }
    }

    warnings
}

/// Convenience wrapper for `check_misattribution` using the default 0.30 threshold.
pub fn check_misattribution_default(
    answer: &str,
    chunks_by_file: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    check_misattribution(answer, chunks_by_file, 0.30)
}

/// Compute a confidence score (0.0–1.0) for an answer based on grounding signals.
/// Starts at 1.0 and deducts for hedging language, missing citations, short answers,
/// and "not found" disclaimers. Boosts for multiple citations and strong grounding.
pub fn compute_confidence(answer: &str, chunks: &[String]) -> f64 {
    let mut confidence: f64 = 1.0;
    let lower = answer.to_lowercase();

    // Deduct for hedging language
    let hedges = [
        "may be", "possibly", "it appears", "it seems", "might be",
        "could be", "perhaps", "unclear", "uncertain",
    ];
    for hedge in &hedges {
        if lower.contains(hedge) {
            confidence -= 0.05;
        }
    }

    // Citation-based scoring
    let citation_count = answer.matches(", p.").count();
    match citation_count {
        0 => confidence -= 0.25,
        1 => confidence -= 0.05,
        2..=4 => confidence += 0.05,
        _ => confidence += 0.10,
    }

    // Factual density: ratio of sentences with at least one citation
    let sentences: Vec<&str> = answer.split(". ").collect();
    let cited_sentences = sentences.iter()
        .filter(|s| s.contains('[') && s.contains(", p."))
        .count();
    if sentences.len() > 2 {
        let citation_ratio = cited_sentences as f64 / sentences.len() as f64;
        if citation_ratio < 0.3 {
            confidence -= 0.15;
        } else if citation_ratio > 0.6 {
            confidence += 0.05;
        }
    }

    // Content grounding against chunks
    let answer_words: HashSet<&str> = lower.split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();
    if !answer_words.is_empty() && !chunks.is_empty() {
        let chunk_text = chunks.join(" ").to_lowercase();
        let grounded = answer_words.iter().filter(|w| chunk_text.contains(*w)).count();
        let grounding_ratio = grounded as f64 / answer_words.len() as f64;
        if grounding_ratio < 0.5 {
            confidence -= 0.20;
        }
    }

    // Deduct if answer is very short (< 50 chars) for non-simple queries
    if answer.len() < 50 {
        confidence -= 0.1;
    }

    // Deduct if answer contains "not present" or "not found" disclaimers
    if lower.contains("not present in the provided") || lower.contains("not found in the") {
        confidence -= 0.15;
    }

    confidence.clamp(0.0, 1.0)
}

/// Compute a confidence score that blends grading-system output (60%) with
/// heuristic confidence (40%). Use when grading data (known_files, expected
/// terms, first_hit_rank, mode) is available for a more calibrated result.
pub fn compute_confidence_with_grading(
    answer: &str,
    chunks: &[String],
    known_files: &[String],
    expected: &[String],
    first_hit_rank: Option<usize>,
    mode: &str,
) -> f64 {
    let grade = crate::grading::grade_response(answer, chunks, known_files, expected, first_hit_rank, mode);
    let grade_confidence = grade.overall / 100.0;
    let heuristic = compute_confidence(answer, chunks);
    (grade_confidence * 0.6 + heuristic * 0.4).clamp(0.0, 1.0)
}

/// Strip sentences from the answer that contain proper nouns or numbers not
/// grounded in any source chunk. This is a last-resort cleanup that removes
/// individual ungrounded sentences rather than failing the whole response.
/// If all sentences would be removed, returns the original answer unchanged.
///
/// Handles sentences ending with `.\n`, `.\n\n`, and bullet points (`- `, `* `).
pub fn strip_ungrounded_claims(answer: &str, chunks: &[String]) -> String {
    let all_sources = chunks.join(" ");
    let sources_lower = all_sources.to_lowercase();

    // Split into lines first, then split lines into sentences
    // This handles bullet points and newline-separated content
    let mut segments: Vec<String> = Vec::new();
    for line in answer.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            segments.push(String::new()); // preserve paragraph breaks
            continue;
        }

        // Detect bullet prefix
        let (prefix, content) = if trimmed.starts_with("- ") {
            ("- ", &trimmed[2..])
        } else if trimmed.starts_with("* ") {
            ("* ", &trimmed[2..])
        } else {
            ("", trimmed)
        };

        // Split content into sentences within this line
        let sentence_re = Regex::new(r"(?s)([^.!?]+[.!?])").unwrap();
        let line_sentences: Vec<&str> = sentence_re
            .find_iter(content)
            .map(|m| m.as_str())
            .collect();

        if line_sentences.is_empty() {
            // No sentence-ending punctuation; treat whole line as one segment
            segments.push(format!("{}{}", prefix, content));
        } else {
            for (i, s) in line_sentences.iter().enumerate() {
                let p = if i == 0 { prefix } else { "" };
                segments.push(format!("{}{}", p, s.trim()));
            }
            // Trailing text after last sentence
            let last_end = content.rfind(|c: char| c == '.' || c == '!' || c == '?');
            if let Some(pos) = last_end {
                let tail = content[pos + 1..].trim();
                if !tail.is_empty() {
                    segments.push(tail.to_string());
                }
            }
        }
    }

    // Regex for proper nouns (capitalized multi-char words) and numbers
    let key_token_re = Regex::new(r"\b[A-Z][a-z]{2,}\b|\b\d[\d,./%$]+\b").unwrap();
    // Regex to strip inline citations before checking
    let cite_strip_re = Regex::new(r"\s*\[[^\]]*\]").unwrap();

    let original_count = segments.iter().filter(|s| !s.trim().is_empty()).count();
    let mut kept: Vec<String> = Vec::new();

    for segment in &segments {
        let trimmed = segment.trim();

        // Preserve empty lines (paragraph breaks)
        if trimmed.is_empty() {
            kept.push(String::new());
            continue;
        }

        let clean = cite_strip_re.replace_all(trimmed, "");
        let tokens: Vec<&str> = key_token_re.find_iter(&clean).map(|m| m.as_str()).collect();

        // If no key tokens, keep (it's generic/connective text)
        if tokens.is_empty() {
            kept.push(segment.clone());
            continue;
        }

        // Check if >= 60% of key tokens are grounded in sources.
        // This prevents stripping sentences where the core fact is grounded
        // but a name is paraphrased.
        let grounded_count = tokens.iter()
            .filter(|tok| sources_lower.contains(&tok.to_lowercase()))
            .count();
        let grounded_ratio = grounded_count as f64 / tokens.len() as f64;

        if grounded_ratio >= 0.6 {
            kept.push(segment.clone());
        } else {
            log::info!(
                "Stripping ungrounded segment: \"{}\"",
                if trimmed.len() > 80 { &trimmed[..80] } else { trimmed }
            );
        }
    }

    // Count non-empty kept segments
    let kept_count = kept.iter().filter(|s| !s.trim().is_empty()).count();

    // If all segments removed, return original (don't make it worse)
    if kept_count == 0 {
        return answer.to_string();
    }

    // If nothing was stripped, return original to preserve formatting
    if kept_count == original_count {
        return answer.to_string();
    }

    // Remove orphaned bullet markers and trailing empty lines
    let result: Vec<&str> = kept.iter()
        .map(|s| s.as_str())
        .filter(|s| {
            let t = s.trim();
            // Remove lines that are just bullet markers with no content
            t != "-" && t != "*" && t != "- " && t != "* "
        })
        .collect();

    // Join with newlines, collapse multiple blank lines
    let joined = result.join("\n");
    let collapse_re = Regex::new(r"\n{3,}").unwrap();
    collapse_re.replace_all(&joined, "\n\n").trim().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Find a valid char boundary at or before `max` to avoid panicking on multi-byte UTF-8.
        let end = s.char_indices()
            .take_while(|(i, _)| *i <= max)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        format!("{}...", &s[..end])
    }
}

trait FindAll<'t> {
    fn find_all(&self, text: &'t str) -> regex::Matches<'_, 't>;
}
impl<'t> FindAll<'t> for Regex {
    fn find_all(&self, text: &'t str) -> regex::Matches<'_, 't> { self.find_iter(text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── check_no_hallucination tests ─────────────────────────────────────────

    #[test]
    fn hallucination_catches_qualifier_claim() {
        let answer = "The license is perpetual and irrevocable under all circumstances.";
        let chunks = &["The license is granted for a period of 12 months."];
        let results = check_no_hallucination(answer, chunks);
        let has_flag = results.iter().any(|r| !r.passed);
        assert!(has_flag, "Should flag ungrounded qualifier claim");
    }

    #[test]
    fn hallucination_passes_grounded_qualifier() {
        let answer = "The license is exclusive and non-transferable.";
        let chunks = &["Licensor grants an exclusive, non-transferable license to use the software."];
        let results = check_no_hallucination(answer, chunks);
        let has_flag = results.iter().any(|r| !r.passed);
        assert!(!has_flag, "Grounded qualifier claim should pass");
    }

    #[test]
    fn hallucination_catches_negation_claim() {
        let answer = "The contract contains no termination clause whatsoever.";
        let chunks = &["Either party may terminate this agreement with 30 days notice."];
        let results = check_no_hallucination(answer, chunks);
        let has_flag = results.iter().any(|r| !r.passed);
        assert!(has_flag, "Should flag ungrounded negation+legal claim");
    }

    #[test]
    fn hallucination_passes_grounded_negation() {
        let answer = "The agreement shall not be assigned without prior written consent.";
        let chunks = &["This agreement shall not be assigned without prior written consent of the other party."];
        let results = check_no_hallucination(answer, chunks);
        let has_flag = results.iter().any(|r| !r.passed);
        assert!(!has_flag, "Grounded negation claim should pass");
    }

    // ── check_citation_pages tests ──────────────────────────────────────────

    #[test]
    fn citation_page_valid() {
        let answer = "See [contract.pdf, p. 5] for details.";
        let mut pages = HashMap::new();
        pages.insert("contract.pdf".to_string(), 10);
        let violations = check_citation_pages(answer, &pages);
        assert!(violations.is_empty(), "Page 5 of 10 should be valid");
    }

    #[test]
    fn citation_page_exceeds_count() {
        let answer = "See [contract.pdf, p. 50] for details.";
        let mut pages = HashMap::new();
        pages.insert("contract.pdf".to_string(), 10);
        let violations = check_citation_pages(answer, &pages);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("only 10 pages"));
    }

    #[test]
    fn citation_page_unknown_file_skipped() {
        let answer = "See [unknown.pdf, p. 99] for details.";
        let pages = HashMap::new();
        let violations = check_citation_pages(answer, &pages);
        assert!(violations.is_empty(), "Unknown file should be skipped");
    }

    #[test]
    fn citation_page_multiple_violations() {
        let answer = "See [a.pdf, p. 5] and [b.pdf, p. 20] for details.";
        let mut pages = HashMap::new();
        pages.insert("a.pdf".to_string(), 3);
        pages.insert("b.pdf".to_string(), 8);
        let violations = check_citation_pages(answer, &pages);
        assert_eq!(violations.len(), 2);
    }

    // ── check_misattribution tests ──────────────────────────────────────────

    #[test]
    fn misattribution_detected() {
        let answer = "The indemnification clause requires unlimited liability coverage [lease.pdf, p. 3].";
        let mut chunks_by_file = HashMap::new();
        chunks_by_file.insert(
            "lease.pdf".to_string(),
            vec!["Rent is due on the first of each month. Late fees apply after the fifth.".to_string()],
        );
        let warnings = check_misattribution(answer, &chunks_by_file, 0.30);
        assert!(!warnings.is_empty(), "Should flag misattributed content");
    }

    #[test]
    fn misattribution_passes_when_grounded() {
        let answer = "Monthly rent payment is due on the first of each calendar month [lease.pdf, p. 1].";
        let mut chunks_by_file = HashMap::new();
        chunks_by_file.insert(
            "lease.pdf".to_string(),
            vec!["The monthly rent payment is due on the first of each calendar month.".to_string()],
        );
        let warnings = check_misattribution(answer, &chunks_by_file, 0.30);
        assert!(warnings.is_empty(), "Well-grounded citation should pass: {:?}", warnings);
    }

    #[test]
    fn misattribution_skips_unknown_file() {
        let answer = "Details are in [mystery.pdf, p. 1].";
        let chunks_by_file = HashMap::new();
        let warnings = check_misattribution(answer, &chunks_by_file, 0.30);
        assert!(warnings.is_empty(), "Unknown file should be skipped");
    }

    // ── compute_confidence tests ────────────────────────────────────────────

    #[test]
    fn confidence_no_citations_penalized() {
        let answer = "The agreement requires 30 days notice for termination. This is a standard clause in most contracts.";
        let chunks = vec!["The agreement requires 30 days notice for termination.".to_string()];
        let conf = compute_confidence(answer, &chunks);
        assert!(conf < 0.85, "No citations should reduce confidence, got {}", conf);
    }

    #[test]
    fn confidence_well_cited_answer() {
        let answer = "The rent is $2,500 [lease.pdf, p. 1]. Late fees are $100 [lease.pdf, p. 2]. Notice period is 30 days [lease.pdf, p. 3].";
        let chunks = vec![
            "rent is $2,500 per month".to_string(),
            "late fees of $100".to_string(),
            "notice period is 30 days".to_string(),
        ];
        let conf = compute_confidence(answer, &chunks);
        assert!(conf > 0.8, "Well-cited answer should have high confidence, got {}", conf);
    }

    #[test]
    fn confidence_hedging_reduces_score() {
        let answer = "It seems the contract may be void. It appears there might be an issue [doc.pdf, p. 1].";
        let chunks = vec!["contract void issue".to_string()];
        let conf = compute_confidence(answer, &chunks);
        assert!(conf < 0.9, "Hedging should reduce confidence, got {}", conf);
    }

    #[test]
    fn confidence_ungrounded_content_penalized() {
        let answer = "The zygomorphic parameterization of the stochastic eigenvalues indicates divergence [doc.pdf, p. 1].";
        let chunks = vec!["The lease requires 30 days notice.".to_string()];
        let conf = compute_confidence(answer, &chunks);
        assert!(conf < 0.85, "Ungrounded content should reduce confidence, got {}", conf);
    }

    // ── strip_ungrounded_claims tests ───────────────────────────────────────

    #[test]
    fn strip_handles_newline_sentences() {
        let answer = "The rent is $2,500.\nThe landlord is John Smith.\nThe lease expires in December.";
        let chunks = vec!["The rent is $2,500 per month. The lease expires in December 2025.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        assert!(!result.contains("John Smith"), "Should strip ungrounded John Smith sentence");
        assert!(result.contains("$2,500"), "Should keep grounded rent sentence");
    }

    #[test]
    fn strip_preserves_bullet_formatting() {
        let answer = "Key terms:\n- The rent is $2,500.\n- The landlord is John Smith.\n- Payment due monthly.";
        let chunks = vec!["The rent is $2,500. Payment due monthly.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        assert!(!result.contains("John Smith"), "Should strip ungrounded bullet");
        assert!(result.contains("$2,500"), "Should keep grounded bullet");
    }

    #[test]
    fn strip_no_orphan_bullets() {
        let answer = "Items:\n- The landlord is John Smith.\n- End of list.";
        // Only "John Smith" is ungrounded; "End" short token is ignored
        let chunks = vec!["End of list noted.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        // Should not contain orphaned "- " with no content
        assert!(!result.contains("\n- \n"), "Should not leave orphaned bullet markers");
    }

    #[test]
    fn strip_returns_original_when_all_grounded() {
        let answer = "The rent is $2,500.\nPayment is due monthly.";
        let chunks = vec!["The rent is $2,500. Payment is due monthly.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        assert_eq!(result, answer, "Should return original when nothing stripped");
    }

    // ── check_fabricated_entities tests ────────────────────────────────────────

    #[test]
    fn fabricated_court_name_flagged() {
        let answer = "The case is currently in the Superior Court of California, County of San Diego.";
        let chunks = &["This contract is between John Smith and Acme Corp."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed && matches!(r.assertion_type, AssertionType::FabricatedEntity));
        assert!(has_fabrication, "Should flag fabricated court name");
    }

    #[test]
    fn grounded_court_name_passes() {
        let answer = "The case is in the Superior Court of California, County of San Diego.";
        let chunks = &["Filed in the Superior Court of California, County of San Diego on Jan 1, 2024."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed);
        assert!(!has_fabrication, "Court name in source should pass");
    }

    #[test]
    fn fabricated_county_flagged() {
        let answer = "This falls under the jurisdiction of County of Los Angeles.";
        let chunks = &["Agreement signed in New York."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed && matches!(r.assertion_type, AssertionType::FabricatedEntity));
        assert!(has_fabrication, "Should flag fabricated county");
    }

    #[test]
    fn fabricated_case_number_flagged() {
        let answer = "See Case No. 2024-CV-12345 for details.";
        let chunks = &["The lease agreement between the parties."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed);
        assert!(has_fabrication, "Should flag fabricated case number");
    }

    #[test]
    fn grounded_case_number_passes() {
        let answer = "See Case No. 2024-CV-12345 for details.";
        let chunks = &["This matter, Case No. 2024-CV-12345, concerns a property dispute."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed);
        assert!(!has_fabrication, "Case number in source should pass");
    }

    #[test]
    fn fabricated_statute_flagged() {
        let answer = "Under 42 U.S.C. § 1983, the plaintiff may seek relief.";
        let chunks = &["The landlord must return the deposit within 30 days."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed);
        assert!(has_fabrication, "Should flag fabricated statute citation");
    }

    #[test]
    fn no_fabrication_when_answer_is_clean() {
        let answer = "The lease requires **30 days** notice before termination.";
        let chunks = &["The lease requires 30 days written notice before termination by either party."];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed);
        assert!(!has_fabrication, "Clean answer should have no fabrication flags");
    }

    #[test]
    fn hello_hallucination_scenario() {
        let answer = "The case is currently in the Superior Court of California, County of San Diego.\n\
            The case is about a dispute between the plaintiff, the defendant, and the defendant's \
            insurer regarding the defendant's alleged negligence in causing harm to the plaintiff.";
        let chunks = &[
            "BARTENDING SERVICES CONTRACT\nThis agreement is entered into between Party A and Party B.",
            "The contractor agrees to provide bartending services at the specified venue.",
        ];
        let results = check_fabricated_entities(answer, chunks);
        let has_fabrication = results.iter().any(|r| !r.passed && matches!(r.assertion_type, AssertionType::FabricatedEntity));
        assert!(has_fabrication, "Must catch the 'Hello' hallucination scenario — fabricated court/county");
    }

    // ── extract_key_words tests ─────────────────────────────────────────────

    #[test]
    fn extract_key_words_filters_stopwords() {
        let words = extract_key_words("the contract requires unlimited liability coverage");
        assert!(words.contains("contract"));
        assert!(words.contains("requires"));
        assert!(words.contains("unlimited"));
        assert!(words.contains("liability"));
        assert!(words.contains("coverage"));
        assert!(!words.contains("the"), "Should filter stopword 'the'");
    }

    #[test]
    fn extract_key_words_filters_short_words() {
        let words = extract_key_words("the big red fox ran over tort lien");
        assert!(!words.contains("big"), "'big' is <= 3 chars");
        assert!(!words.contains("red"), "'red' is <= 3 chars");
        assert!(!words.contains("fox"), "'fox' is <= 3 chars");
        assert!(!words.contains("ran"), "'ran' is <= 3 chars");
        assert!(!words.contains("over"), "'over' is a stopword");
        // Legal terms with 4 chars should be included
        assert!(words.contains("tort"), "'tort' is > 3 chars and not a stopword");
        assert!(words.contains("lien"), "'lien' is > 3 chars and not a stopword");
    }

    // ── compute_confidence_with_grading tests ─────────────────────────────

    #[test]
    fn confidence_with_grading_blends_scores() {
        let answer = "The rent is $1,500 [lease.pdf, p. 1]. Late fees are $100 [lease.pdf, p. 2].";
        let chunks = vec![
            "The rent is $1,500 per month.".to_string(),
            "Late fees of $100 apply.".to_string(),
        ];
        let known_files = vec!["lease.pdf".to_string()];
        let expected = vec!["rent".to_string(), "$1,500".to_string()];
        let conf = compute_confidence_with_grading(answer, &chunks, &known_files, &expected, Some(1), "normal");
        assert!(conf > 0.0 && conf <= 1.0, "Blended confidence should be in (0,1], got {}", conf);
    }

    #[test]
    fn confidence_with_grading_low_grade_reduces_score() {
        // No citations, no expected terms found, no retrieval hit → grade should be low
        let answer = "Something completely unrelated with no citations.";
        let chunks = vec!["The lease requires 30 days notice.".to_string()];
        let known_files = vec!["lease.pdf".to_string()];
        let expected = vec!["deposit".to_string(), "security".to_string()];
        let conf = compute_confidence_with_grading(answer, &chunks, &known_files, &expected, None, "normal");
        let heuristic = compute_confidence(answer, &chunks);
        // With a low grade, blended should be less than or equal to pure heuristic
        assert!(conf <= heuristic + 0.05, "Low grade should not inflate confidence: blended={}, heuristic={}", conf, heuristic);
    }

    // ── strip_ungrounded_claims softened threshold tests ──────────────────

    #[test]
    fn strip_keeps_sentence_when_most_tokens_grounded() {
        // "Smith" is not in source but "rent" and "$2,500" and "Monthly" are → 3/4 = 75% ≥ 60%
        let answer = "Monthly rent from Smith is $2,500.";
        let chunks = vec!["Monthly rent is $2,500 per the lease agreement.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        assert!(result.contains("$2,500"), "Should keep sentence when >=60% tokens grounded, got: {}", result);
    }

    #[test]
    fn strip_removes_sentence_when_few_tokens_grounded() {
        // Fabricated tokens: "Johannesburg", "Canterbury", "Wellington" — none in source
        // Only grounded token might be none. All 3 proper nouns ungrounded → 0% < 60%
        let answer = "Filed in Johannesburg by Canterbury and Wellington.\nThe rent is $2,500.";
        let chunks = vec!["The rent is $2,500 per month.".to_string()];
        let result = strip_ungrounded_claims(answer, &chunks);
        assert!(!result.contains("Johannesburg"), "Should strip sentence with <60% grounded tokens");
        assert!(result.contains("$2,500"), "Should keep grounded sentence");
    }

    // ── check_misattribution threshold parameter tests ────────────────────

    #[test]
    fn misattribution_custom_threshold_stricter() {
        let answer = "The indemnification clause requires coverage [lease.pdf, p. 3].";
        let mut chunks_by_file = HashMap::new();
        chunks_by_file.insert(
            "lease.pdf".to_string(),
            vec!["Rent is due on the first of each month. Indemnification applies broadly.".to_string()],
        );
        // With a very strict threshold (0.80), even partial grounding should flag
        let warnings_strict = check_misattribution(answer, &chunks_by_file, 0.80);
        // With lenient threshold (0.10), should pass
        let warnings_lenient = check_misattribution(answer, &chunks_by_file, 0.10);
        assert!(
            warnings_strict.len() >= warnings_lenient.len(),
            "Stricter threshold should produce >= warnings: strict={}, lenient={}",
            warnings_strict.len(), warnings_lenient.len()
        );
    }

    #[test]
    fn misattribution_default_wrapper_matches() {
        let answer = "The indemnification clause requires unlimited liability coverage [lease.pdf, p. 3].";
        let mut chunks_by_file = HashMap::new();
        chunks_by_file.insert(
            "lease.pdf".to_string(),
            vec!["Rent is due on the first of each month. Late fees apply after the fifth.".to_string()],
        );
        let direct = check_misattribution(answer, &chunks_by_file, 0.30);
        let default = check_misattribution_default(answer, &chunks_by_file);
        assert_eq!(direct, default, "Default wrapper should match threshold=0.30");
    }
}
