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
    CitationPresence,
    CitationFilename,
    NumberExactness,
    Blocklist,
    Hallucination,
}

/// Verify citations match `[filename, p. N]` format, at least one exists,
/// and optionally that cited filenames appear in `known_files`.
pub fn check_citations(answer: &str, known_files: Option<&[&str]>) -> Vec<AssertionResult> {
    let mut results = Vec::new();
    let cite_re = Regex::new(r"\[([^,\[\]]+),\s*p\.\s*(\d+)\]").unwrap();
    let captures: Vec<_> = cite_re.captures_iter(answer).collect();

    results.push(AssertionResult {
        passed: !captures.is_empty(),
        assertion_type: AssertionType::CitationPresence,
        message: if captures.is_empty() {
            "No citations found in answer".into()
        } else {
            format!("Found {} citation(s)", captures.len())
        },
    });

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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

trait FindAll<'t> {
    fn find_all(&self, text: &'t str) -> regex::Matches<'_, 't>;
}
impl<'t> FindAll<'t> for Regex {
    fn find_all(&self, text: &'t str) -> regex::Matches<'_, 't> { self.find_iter(text) }
}
