use regex::Regex;
use serde::{Deserialize, Serialize};

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
    NumberExactness,
    Blocklist,
    Hallucination,
    FabricatedEntity,
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
                format!("Valid citation format: {text}")
            } else {
                format!("Malformed citation: {text}")
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
                    format!("Cited file exists: {cited}")
                } else {
                    format!("Cited file not in known documents: {cited}")
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
                    format!("Number appears verbatim: {num}")
                } else {
                    format!("Expected number missing or altered: {num}")
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
                    format!("Blocked term detected: \"{term}\"")
                } else {
                    format!("Blocked term absent: \"{term}\"")
                },
            }
        })
        .collect()
}

/// Extract sentences containing numbers or capitalized proper nouns from the
/// answer, then verify each appears (substring) in at least one source chunk.
pub fn check_no_hallucination(answer: &str, chunks: &[&str]) -> Vec<AssertionResult> {
    let claim_re = Regex::new(
        r"(?:^|[.!?]\s+)([A-Z][^.!?]*(?:\d[\d,./%$]+|[A-Z][a-z]{2,})[^.!?]*[.!?])"
    ).unwrap();

    let mut results = Vec::new();
    for cap in claim_re.captures_iter(answer) {
        let sentence = cap[1].trim();
        // Strip inline citations before matching against source text
        let cite_strip = Regex::new(r"\s*\[[^\]]*\]").unwrap();
        let clean = cite_strip.replace_all(sentence, "").trim().to_string();
        if clean.len() < 15 {
            continue;
        }
        // Extract key tokens (numbers, capitalized words 3+ chars) and check
        // that each token appears in at least one chunk.
        let token_re = Regex::new(r"\d[\d,./%$]+|[A-Z][a-z]{2,}").unwrap();
        let tokens: Vec<&str> = token_re.find_all(&clean).map(|m| m.as_str()).collect();
        if tokens.is_empty() {
            continue;
        }
        let grounded = tokens.iter().all(|tok| chunks.iter().any(|c| c.contains(tok)));
        results.push(AssertionResult {
            passed: grounded,
            assertion_type: AssertionType::Hallucination,
            message: if grounded {
                format!("Claim grounded in sources: \"{}\"", truncate(&clean, 80))
            } else {
                let missing: Vec<&&str> = tokens
                    .iter()
                    .filter(|t| !chunks.iter().any(|c| c.contains(**t)))
                    .collect();
                format!(
                    "Possible hallucination — tokens {:?} not in sources: \"{}\"",
                    missing,
                    truncate(&clean, 80)
                )
            },
        });
    }
    if results.is_empty() {
        results.push(AssertionResult {
            passed: true,
            assertion_type: AssertionType::Hallucination,
            message: "No falsifiable claims extracted (nothing to check)".into(),
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
                message: format!("Court name not found in source documents: \"{}\"", truncate(full, 80)),
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
                message: format!("Jurisdiction not found in source documents: \"{}\"", truncate(full, 80)),
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
                message: format!("Case number not found in source documents: \"{}\"", text),
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
                message: format!("Party name not found in source documents: \"{}\"", party_name),
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
                message: format!("Statute citation not found in source documents: \"{}\"", text),
            });
        }
    }

    results
}

/// Compute a confidence score (0.0–1.0) for an answer based on grounding signals.
/// Starts at 1.0 and deducts for hedging language, missing citations, short answers,
/// and "not found" disclaimers. Boosts for multiple citations (good grounding).
pub fn compute_confidence(answer: &str, _chunks: &[String]) -> f64 {
    let mut confidence: f64 = 1.0;

    // Deduct for hedging language
    let hedges = [
        "may be", "possibly", "it appears", "it seems", "might",
        "could be", "perhaps", "unclear", "uncertain",
    ];
    let lower = answer.to_lowercase();
    for hedge in &hedges {
        if lower.contains(hedge) {
            confidence -= 0.05;
        }
    }

    // Deduct if no citations found
    if !answer.contains('[') || !answer.contains(", p.") {
        confidence -= 0.2;
    }

    // Deduct if answer is very short (< 50 chars) for non-simple queries
    if answer.len() < 50 {
        confidence -= 0.1;
    }

    // Deduct if answer contains "not present" or "not found" disclaimers
    if lower.contains("not present in the provided") || lower.contains("not found in the") {
        confidence -= 0.15;
    }

    // Boost if multiple citations found (good grounding)
    let citation_count = answer.matches(", p.").count();
    if citation_count >= 3 {
        confidence += 0.1;
    }

    confidence.clamp(0.0, 1.0)
}

/// Strip sentences from the answer that contain proper nouns or numbers not
/// grounded in any source chunk. This is a last-resort cleanup that removes
/// individual ungrounded sentences rather than failing the whole response.
/// If all sentences would be removed, returns the original answer unchanged.
pub fn strip_ungrounded_claims(answer: &str, chunks: &[String]) -> String {
    let all_sources = chunks.join(" ");
    let sources_lower = all_sources.to_lowercase();

    // Split into sentences (rough: split on `. `, `! `, `? `, or newline)
    let sentence_re = Regex::new(r"(?s)([^.!?\n]+[.!?])").unwrap();
    let sentences: Vec<&str> = sentence_re
        .find_iter(answer)
        .map(|m| m.as_str())
        .collect();

    if sentences.is_empty() {
        return answer.to_string();
    }

    // Regex for proper nouns (capitalized multi-char words) and numbers
    let key_token_re = Regex::new(r"\b[A-Z][a-z]{2,}\b|\b\d[\d,./%$]+\b").unwrap();
    // Regex to strip inline citations before checking
    let cite_strip_re = Regex::new(r"\s*\[[^\]]*\]").unwrap();

    let mut kept: Vec<&str> = Vec::new();
    for sentence in &sentences {
        let clean = cite_strip_re.replace_all(sentence, "");
        let tokens: Vec<&str> = key_token_re.find_iter(&clean).map(|m| m.as_str()).collect();

        // If no key tokens, keep the sentence (it's generic/connective text)
        if tokens.is_empty() {
            kept.push(sentence);
            continue;
        }

        // Check if all key tokens are grounded in sources
        let grounded = tokens.iter().all(|tok| {
            sources_lower.contains(&tok.to_lowercase())
        });

        if grounded {
            kept.push(sentence);
        } else {
            log::info!(
                "Stripping ungrounded sentence: \"{}\"",
                if sentence.len() > 80 { &sentence[..80] } else { sentence }
            );
        }
    }

    // If all sentences removed, return original (don't make it worse)
    if kept.is_empty() {
        return answer.to_string();
    }

    // If nothing was stripped, return original to preserve formatting
    if kept.len() == sentences.len() {
        return answer.to_string();
    }

    kept.join(" ")
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
        // This is the exact scenario from the bug: user says "Hello", model fabricates
        // court details that don't exist in any document.
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
}
