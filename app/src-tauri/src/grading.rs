use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeReport {
    pub retrieval_precision: f64,
    pub factual_grounding: f64,
    pub citation_accuracy: f64,
    pub response_completeness: f64,
    pub response_efficiency: f64,
    pub overall: f64,
    pub grade_letter: String,
    pub details: Vec<String>,
}

/// Score retrieval precision based on the rank of the first relevant hit.
/// rank=1→100, rank=2→75, rank=3→50, rank=4→25, rank=5+→10, None→0.
pub fn grade_retrieval(first_hit_rank: Option<usize>) -> (f64, String) {
    match first_hit_rank {
        Some(1) => (100.0, "First hit at rank 1".into()),
        Some(2) => (75.0, "First hit at rank 2".into()),
        Some(3) => (50.0, "First hit at rank 3".into()),
        Some(4) => (25.0, "First hit at rank 4".into()),
        Some(r) => (10.0, format!("First hit at rank {r} (5+)")),
        None => (0.0, "Relevant chunk not found in results".into()),
    }
}

/// Score factual grounding: extract sentences containing numbers or proper nouns,
/// then verify key tokens appear in at least one source chunk.
pub fn grade_grounding(answer: &str, chunks: &[String]) -> (f64, String) {
    let claim_re = Regex::new(
        r"(?:^|[.!?]\s+)([A-Z][^.!?]*(?:\d[\d,./%$]+|[A-Z][a-z]{2,})[^.!?]*[.!?])",
    )
    .unwrap();
    let token_re = Regex::new(r"\d[\d,./%$]+|[A-Z][a-z]{2,}").unwrap();
    let cite_strip = Regex::new(r"\s*\[[^\]]*\]").unwrap();

    let mut total = 0usize;
    let mut grounded = 0usize;

    for cap in claim_re.captures_iter(answer) {
        let sentence = cap[1].trim();
        let clean = cite_strip.replace_all(sentence, "").trim().to_string();
        if clean.len() < 15 {
            continue;
        }
        let tokens: Vec<&str> = token_re.find_iter(&clean).map(|m| m.as_str()).collect();
        if tokens.is_empty() {
            continue;
        }
        total += 1;
        if tokens.iter().all(|tok| chunks.iter().any(|c| c.contains(tok))) {
            grounded += 1;
        }
    }

    if total == 0 {
        return (100.0, "No extractable factual claims — full score".into());
    }
    let score = (grounded as f64 / total as f64) * 100.0;
    (
        score,
        format!("{grounded}/{total} claims grounded in source chunks"),
    )
}

/// Score citation accuracy: find all `[..., p. N]` patterns, check format and
/// filename against known_files. No citations at all → 0.
pub fn grade_citations(answer: &str, known_files: &[String]) -> (f64, String) {
    let cite_re = Regex::new(r"\[([^,\[\]]+),\s*p\.\s*(\d+)\]").unwrap();
    let bracket_re = Regex::new(r"\[[^\[\]]{2,}\]").unwrap();

    let all_brackets: Vec<_> = bracket_re.find_iter(answer).collect();
    if all_brackets.is_empty() {
        return (0.0, "No citations found (citations are expected)".into());
    }

    let mut valid = 0usize;
    let total = all_brackets.len();

    for m in &all_brackets {
        let text = m.as_str();
        if let Some(cap) = cite_re.captures(text) {
            let cited_file = cap[1].trim();
            // Format is valid; also check filename against known_files
            if known_files.iter().any(|f| f == cited_file) {
                valid += 1;
            }
            // If format is correct but filename unknown, it's still not valid
        }
        // Malformed brackets are not valid
    }

    let score = (valid as f64 / total as f64) * 100.0;
    (score, format!("{valid}/{total} citations valid and matched"))
}

/// Score response completeness: case-insensitive substring search for each
/// expected term. Empty expected list → 100.
pub fn grade_completeness(answer: &str, expected: &[String]) -> (f64, String) {
    if expected.is_empty() {
        return (100.0, "No expected terms specified — full score".into());
    }

    let lower = answer.to_lowercase();
    let found = expected
        .iter()
        .filter(|term| lower.contains(&term.to_lowercase()))
        .count();
    let score = (found as f64 / expected.len() as f64) * 100.0;
    (
        score,
        format!("{found}/{} expected terms found in answer", expected.len()),
    )
}

/// Score response efficiency: deduct for hedging, filler, and excessive length.
pub fn grade_efficiency(answer: &str, mode: &str) -> (f64, String) {
    let lower = answer.to_lowercase();
    let mut score = 100.0f64;
    let mut notes: Vec<String> = Vec::new();

    // Hedging phrases: -5 each, max -30
    let hedging = [
        "may be", "possibly", "it appears", "it seems", "might be", "could be", "perhaps",
    ];
    let hedge_count = hedging.iter().filter(|h| lower.contains(**h)).count();
    let hedge_penalty = (hedge_count as f64 * 5.0).min(30.0);
    if hedge_penalty > 0.0 {
        score -= hedge_penalty;
        notes.push(format!("{hedge_count} hedging phrase(s) (-{hedge_penalty})"));
    }

    // Filler phrases: -10 each, max -30
    let fillers = [
        "i hope this helps",
        "let me explain",
        "as mentioned",
        "in conclusion",
        "to summarize",
    ];
    let filler_count = fillers.iter().filter(|f| lower.contains(**f)).count();
    let filler_penalty = (filler_count as f64 * 10.0).min(30.0);
    if filler_penalty > 0.0 {
        score -= filler_penalty;
        notes.push(format!("{filler_count} filler phrase(s) (-{filler_penalty})"));
    }

    // Extended mode section checking
    if mode == "extended" || mode == "Extended" {
        let expected_sections = ["**Answer**", "**Key Provisions**", "**Analysis**", "**Caveats**"];
        let lower_answer = answer.to_lowercase();
        let mut missing_sections = Vec::new();
        for section in &expected_sections {
            if !lower_answer.contains(&section.to_lowercase().replace("**", "")) {
                missing_sections.push(*section);
            }
        }
        if !missing_sections.is_empty() {
            let penalty = (missing_sections.len() as f64 * 10.0).min(30.0);
            score -= penalty;
            notes.push(format!("Missing sections: {}", missing_sections.join(", ")));
        }
    }

    // Word count penalty
    let word_count = answer.split_whitespace().count();
    let limit = if mode == "quick" { 300 } else { 800 };
    if word_count > limit {
        score -= 20.0;
        notes.push(format!("Word count {word_count} exceeds {limit} limit (-20)"));
    }

    score = score.max(0.0);
    let detail = if notes.is_empty() {
        "No efficiency deductions".into()
    } else {
        notes.join("; ")
    };
    (score, detail)
}

/// Produce a full grade report across all 5 dimensions.
pub fn grade_response(
    answer: &str,
    chunks: &[String],
    known_files: &[String],
    expected: &[String],
    first_hit_rank: Option<usize>,
    mode: &str,
) -> GradeReport {
    let (retrieval_precision, d1) = grade_retrieval(first_hit_rank);
    let (factual_grounding, d2) = grade_grounding(answer, chunks);
    let (citation_accuracy, d3) = grade_citations(answer, known_files);
    let (response_completeness, d4) = grade_completeness(answer, expected);
    let (response_efficiency, d5) = grade_efficiency(answer, mode);

    let overall = retrieval_precision * 0.25
        + factual_grounding * 0.30
        + citation_accuracy * 0.15
        + response_completeness * 0.20
        + response_efficiency * 0.10;

    let grade_letter = if overall >= 90.0 {
        "A"
    } else if overall >= 80.0 {
        "B"
    } else if overall >= 70.0 {
        "C"
    } else if overall >= 60.0 {
        "D"
    } else {
        "F"
    }
    .to_string();

    GradeReport {
        retrieval_precision,
        factual_grounding,
        citation_accuracy,
        response_completeness,
        response_efficiency,
        overall,
        grade_letter,
        details: vec![d1, d2, d3, d4, d5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_rank_scores() {
        assert_eq!(grade_retrieval(Some(1)).0, 100.0);
        assert_eq!(grade_retrieval(Some(2)).0, 75.0);
        assert_eq!(grade_retrieval(Some(3)).0, 50.0);
        assert_eq!(grade_retrieval(Some(4)).0, 25.0);
        assert_eq!(grade_retrieval(Some(5)).0, 10.0);
        assert_eq!(grade_retrieval(Some(99)).0, 10.0);
        assert_eq!(grade_retrieval(None).0, 0.0);
    }

    #[test]
    fn grounding_no_claims() {
        let (score, _) = grade_grounding("Hello world.", &["some chunk".into()]);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn grounding_grounded_claim() {
        // The regex requires `[.!?]\s+` before a claim sentence (not start of string).
        let answer = "Overview provided. The monthly rent amount is $1,500 according to the lease agreement signed by John Smith.";
        let chunks = vec!["The monthly rent amount is $1,500 according to the lease agreement signed by John Smith.".into()];
        let (score, detail) = grade_grounding(answer, &chunks);
        assert!(score > 0.0, "Expected grounded score > 0, got {score}: {detail}");
    }

    #[test]
    fn citations_none_scores_zero() {
        let (score, _) = grade_citations("No citations here.", &["file.pdf".into()]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn citations_valid() {
        let answer = "The rent is $1000 [lease.pdf, p. 3].";
        let known = vec!["lease.pdf".into()];
        let (score, _) = grade_citations(answer, &known);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn citations_unknown_file() {
        let answer = "See [unknown.pdf, p. 1].";
        let known = vec!["lease.pdf".into()];
        let (score, _) = grade_citations(answer, &known);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn completeness_all_found() {
        let answer = "The landlord must return the security deposit within 30 days.";
        let expected = vec!["security deposit".into(), "30 days".into()];
        let (score, _) = grade_completeness(answer, &expected);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn completeness_partial() {
        let answer = "The landlord must return the deposit.";
        let expected = vec!["deposit".into(), "30 days".into()];
        let (score, _) = grade_completeness(answer, &expected);
        assert!((score - 50.0).abs() < 0.01);
    }

    #[test]
    fn completeness_empty_expected() {
        let (score, _) = grade_completeness("anything", &[]);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn efficiency_clean_answer() {
        let (score, _) = grade_efficiency("The rent is $1000 per month.", "quick");
        assert_eq!(score, 100.0);
    }

    #[test]
    fn efficiency_hedging_deduction() {
        let (score, _) = grade_efficiency("It may be that the rent could be $1000.", "quick");
        assert!(score < 100.0);
    }

    #[test]
    fn efficiency_filler_deduction() {
        let (score, _) =
            grade_efficiency("Let me explain the lease terms. I hope this helps.", "quick");
        assert!(score <= 80.0);
    }

    #[test]
    fn efficiency_long_answer_quick_mode() {
        let long = "word ".repeat(301);
        let (score, _) = grade_efficiency(&long, "quick");
        assert!(score <= 80.0);
    }

    #[test]
    fn efficiency_long_answer_normal_mode() {
        // 500 words is fine in normal mode
        let medium = "word ".repeat(500);
        let (score, _) = grade_efficiency(&medium, "normal");
        assert_eq!(score, 100.0);
    }

    #[test]
    fn overall_grade_letter() {
        let report = grade_response(
            "The rent is $1000 [lease.pdf, p. 3].",
            &["The monthly rent is $1000.".into()],
            &["lease.pdf".into()],
            &["rent".into(), "$1000".into()],
            Some(1),
            "normal",
        );
        assert!(
            report.overall > 0.0,
            "Overall score should be positive"
        );
        assert!(
            ["A", "B", "C", "D", "F"].contains(&report.grade_letter.as_str()),
            "Grade letter should be valid"
        );
        assert_eq!(report.details.len(), 5);
    }

    #[test]
    fn floor_at_zero() {
        // Max deductions: 6 hedging (30) + 5 fillers (30) + long (20) = 80, but capped
        let answer = "May be possibly it appears it seems might be could be perhaps. \
                       I hope this helps. Let me explain. As mentioned. In conclusion. To summarize. ".repeat(50);
        let (score, _) = grade_efficiency(&answer, "quick");
        assert!(score >= 0.0);
    }

    // ── Extended mode section checking tests (Gap 28) ───────────────────

    #[test]
    fn efficiency_extended_all_sections_present() {
        let answer = "**Answer**\nThe rent is $1000.\n\n**Key Provisions**\nSection 5.\n\n**Analysis**\nThis is standard.\n\n**Caveats**\nConsult a lawyer.";
        let (score, _) = grade_efficiency(answer, "extended");
        // No section penalty expected
        assert_eq!(score, 100.0);
    }

    #[test]
    fn efficiency_extended_missing_sections() {
        let answer = "**Answer**\nThe rent is $1000.\n\n**Analysis**\nThis is standard.";
        let (score, detail) = grade_efficiency(answer, "extended");
        // Missing Key Provisions and Caveats => -20
        assert!(score < 100.0, "score={score}, detail={detail}");
        assert!(detail.contains("Missing sections"), "detail={detail}");
    }

    #[test]
    fn efficiency_extended_all_missing() {
        let answer = "The rent is $1000 per month.";
        let (score, detail) = grade_efficiency(answer, "extended");
        // Missing all 4 sections => -30 (capped)
        assert!(score <= 70.0, "score={score}, detail={detail}");
    }

    #[test]
    fn efficiency_non_extended_no_section_check() {
        // Non-extended mode should not penalize missing sections
        let answer = "The rent is $1000 per month.";
        let (score, _) = grade_efficiency(answer, "balanced");
        assert_eq!(score, 100.0);
    }
}
