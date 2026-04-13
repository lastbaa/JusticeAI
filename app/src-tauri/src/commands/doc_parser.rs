use crate::state::DocumentPage;
use calamine::{open_workbook_auto, Reader};
use mailparse::MailHeaderMap;
use regex::Regex;
use std::io::Read;
use std::path::Path;

const MAX_FILE_SIZE_BYTES: u64 = 40 * 1024 * 1024;
const MAX_TEXT_CHARS: usize = 2_000_000;
const PAGE_CHAR_BUDGET: usize = 7_000;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "txt", "md", "csv", "eml", "html", "htm", "mhtml", "xml", "xlsx", "png", "jpg",
    "jpeg", "tif", "tiff",
];

/// Resolve and validate file type using extension + lightweight magic checks.
/// This blocks accidental parser confusion and simple extension spoofing.
pub fn detect_supported_extension(path: &str) -> Result<String, String> {
    enforce_file_security(path)?;

    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .ok_or_else(|| format!("File has no extension: {path}"))?;

    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("Unsupported file type: {path}"));
    }

    let head = read_head(path, 8)?;
    match ext.as_str() {
        "pdf" if !head.starts_with(b"%PDF") => {
            return Err(format!(
                "File extension is .pdf but content is not a PDF: {path}"
            ));
        }
        "png" if head != [137, 80, 78, 71, 13, 10, 26, 10] => {
            return Err(format!(
                "File extension is .png but header is invalid: {path}"
            ));
        }
        "jpg" | "jpeg" if !head.starts_with(&[0xFF, 0xD8, 0xFF]) => {
            return Err(format!(
                "File extension is .jpg/.jpeg but header is invalid: {path}"
            ));
        }
        "tif" | "tiff" if !(head.starts_with(b"II*\0") || head.starts_with(b"MM\0*")) => {
            return Err(format!(
                "File extension is .tif/.tiff but header is invalid: {path}"
            ));
        }
        _ => {}
    }

    Ok(ext)
}

/// Route document parsing to the appropriate handler based on file extension.
/// Supports pdf, docx, txt, csv, html, eml, xlsx, and image formats (via OCR).
pub fn parse_by_extension(path: &str, ext: &str, model_dir: &std::path::Path) -> Result<Vec<DocumentPage>, String> {
    match ext {
        "pdf" => parse_pdf(path),
        "docx" => parse_docx(path),
        "txt" => parse_plain_text(path),
        "md" => parse_markdown(path),
        "csv" => parse_csv(path),
        "eml" => parse_eml(path),
        "html" | "htm" => parse_html(path),
        "mhtml" => parse_mhtml(path),
        "xml" => parse_xml(path),
        "xlsx" => parse_xlsx(path),
        "png" | "jpg" | "jpeg" | "tif" | "tiff" => parse_image_ocr(path, model_dir),
        _ => Err(format!("Unsupported file extension: {ext}")),
    }
}

/// Score the quality of extracted text to decide if OCR fallback is needed.
/// Returns a value between 0.0 (garbage) and 1.0 (clean text).
fn text_quality_score(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;
    let printable = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
        .count() as f64;

    let ratio = printable / total_chars;

    // Also check for reasonable word patterns
    let words = text.split_whitespace().count();
    let avg_word_len = if words > 0 {
        total_chars / words as f64
    } else {
        0.0
    };

    // Good text: >90% printable, 3-15 avg word length
    let word_score = if avg_word_len > 3.0 && avg_word_len < 15.0 {
        1.0
    } else {
        0.5
    };

    // Penalize text that's mostly punctuation/symbols (failed CMap font decoding).
    // Real text should be at least 25% alphabetic characters.
    let alpha = text.chars().filter(|c| c.is_alphabetic()).count() as f64;
    let alpha_ratio = alpha / total_chars;
    let alpha_score = if alpha_ratio < 0.15 { 0.2 } else if alpha_ratio < 0.25 { 0.6 } else { 1.0 };

    ratio * word_score * alpha_score
}

/// Filter out form fields whose values already appear in the page text.
fn deduplicate_form_fields(
    fields: &[(String, String)],
    page_text: &str,
) -> Vec<(String, String)> {
    fields
        .iter()
        .filter(|(_, value)| {
            let trimmed = value.trim();
            // Keep if value is non-trivial and NOT already in page text
            trimmed.len() > 1 && !page_text.contains(trimmed)
        })
        .cloned()
        .collect()
}

/// Normalize a multi-line form field value into a single line for readability.
/// Replaces newlines with ", " and trims whitespace from each line fragment.
fn normalize_field_value(value: &str) -> String {
    value
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Redact common PII patterns (SSN, EIN, credit card, routing numbers) from a string.
///
/// NOT applied by default — Justice AI runs fully on-device so all data stays local.
/// This function is available for future use if/when document sharing, export, or
/// cloud sync features are implemented. To enable, call `redact_pii()` on field values
/// before displaying citations/excerpts to the user. Do NOT apply to chunks used for
/// retrieval — the LLM needs original values to answer questions accurately.
#[allow(dead_code)]
fn redact_pii(value: &str) -> String {
    let mut result = value.to_string();

    // SSN: XXX-XX-XXXX
    let ssn_re = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    result = ssn_re.replace_all(&result, "***-**-****").to_string();

    // SSN without dashes: 9 digits, but skip invalid prefixes (0xx, 9xx)
    let ssn_re2 = Regex::new(r"\b[1-8]\d{8}\b").unwrap();
    result = ssn_re2.replace_all(&result, "*********").to_string();

    // EIN: XX-XXXXXXX
    let ein_re = Regex::new(r"\b\d{2}-\d{7}\b").unwrap();
    result = ein_re.replace_all(&result, "**-*******").to_string();

    // Credit card: XXXX-XXXX-XXXX-XXXX or XXXXXXXXXXXXXXXX (with optional spaces/dashes)
    let cc_re = Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap();
    result = cc_re.replace_all(&result, "**** **** **** ****").to_string();

    // Routing number: 9 digits starting with 0-3
    let routing_re = Regex::new(r"\b[0-3]\d{8}\b").unwrap();
    result = routing_re.replace_all(&result, "*********").to_string();

    result
}

/// Detect and format table-like content in extracted text.
/// Looks for tab-delimited or consistent-spacing patterns.
fn format_tables(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();

    for line in &lines {
        // Detect tab-delimited lines (common in PDF table extraction)
        if line.matches('\t').count() >= 2 {
            // Format as pipe-delimited table row
            let cells: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
            result.push(format!("| {} |", cells.join(" | ")));
        } else {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}

/// Parse a PDF file into pages of text.
///
/// Pipeline:
/// 1. Race pdf_oxide and pdf-extract in parallel threads.
/// 2. Accept whichever returns first with good quality results.
/// 3. If the PDF has Form XObjects (filled forms), use lopdf to get XObject
///    coordinates and re-interleave filled values next to their template labels.
/// 4. Fall back to plain lopdf extract_text as a last resort.
pub fn parse_pdf(path: &str) -> Result<Vec<DocumentPage>, String> {
    use rayon::prelude::*;

    let mut pages = parse_pdf_parallel(path)?;

    // Post-process: format table-like content (parallel across pages)
    pages.par_iter_mut().for_each(|page| {
        page.text = format_tables(&page.text);
    });

    Ok(pages)
}

/// Try pdf_oxide and pdf-extract in parallel, take whichever succeeds first
/// with good results. Falls back to lopdf synchronously.
fn parse_pdf_parallel(path: &str) -> Result<Vec<DocumentPage>, String> {
    use std::thread;

    // Quick check for password-protected PDFs before launching heavy parsers
    if let Ok(doc) = lopdf::Document::load(path) {
        if doc.is_encrypted() {
            return Err("This PDF is password-protected. Please remove the password and try again.".to_string());
        }
    }

    let path1 = path.to_string();
    let path2 = path.to_string();

    // Launch both parsers in threads
    let handle1 = thread::spawn(move || try_pdf_oxide(&path1));
    let handle2 = thread::spawn(move || pdf_extract::extract_text(&path2));

    // Try pdf_oxide first (usually better)
    if let Ok(result) = handle1.join() {
        if let Some(ref pages) = result {
            if !pages.is_empty() && pages.iter().any(|p| !p.text.trim().is_empty()) {
                // Quality check using text_quality_score
                let quality = pages
                    .iter()
                    .map(|p| text_quality_score(&p.text))
                    .sum::<f64>()
                    / pages.len().max(1) as f64;

                if quality >= 0.5 {
                    let mut pages = result.unwrap();
                    // Still use lopdf for AcroForm and form field re-interleaving
                    if let Ok(doc) = lopdf::Document::load(path) {
                        let improved = reinterleave_form_fields(&doc, &pages);
                        if !improved.is_empty() {
                            pages = improved;
                        }
                        append_acroform_values(&doc, &mut pages);
                    }
                    return Ok(pages);
                } else {
                    log::warn!("pdf_oxide quality score {quality:.2} < 0.5, trying alternatives");
                }
            }
        }
    }

    // Fall back to pdf-extract
    if let Ok(Ok(raw)) = handle2.join() {
        if !raw.trim().is_empty() {
            let mut pages = pdf_extract_pages(&raw);
            if !pages.is_empty() {
                if let Ok(doc) = lopdf::Document::load(path) {
                    // If pdf-extract collapsed multiple pages into one (no form-feeds),
                    // try lopdf per-page extraction to get proper page boundaries.
                    let lopdf_page_count = doc.get_pages().len();
                    if pages.len() == 1 && lopdf_page_count > 1 {
                        let lopdf_pages = extract_lopdf_pages(&doc);
                        let has_content =
                            lopdf_pages.iter().any(|p| !p.text.trim().is_empty());
                        if lopdf_pages.len() > 1 && has_content {
                            pages = lopdf_pages;
                        }
                    }

                    // Try to improve form-field extraction by re-interleaving with coordinates
                    let improved = reinterleave_form_fields(&doc, &pages);
                    if !improved.is_empty() {
                        pages = improved;
                    }

                    // Append AcroForm field values that aren't already in the text.
                    append_acroform_values(&doc, &mut pages);
                }
                return Ok(pages);
            }
        }
    }

    // Final fallback: lopdf (synchronous, since it's the last resort)
    try_lopdf_parse(path)
}

/// Last-resort PDF parsing using only lopdf.
fn try_lopdf_parse(path: &str) -> Result<Vec<DocumentPage>, String> {
    let doc = lopdf::Document::load(path).map_err(|e| format!("PDF load error: {e}"))?;
    let mut pages = extract_lopdf_pages(&doc);
    append_acroform_values(&doc, &mut pages);
    Ok(pages)
}

/// Try extracting text using pdf_oxide. Returns None on any error,
/// or Some(empty vec) if extraction succeeds but produces no content.
fn try_pdf_oxide(path: &str) -> Option<Vec<DocumentPage>> {
    use rayon::prelude::*;

    let mut doc = pdf_oxide::PdfDocument::open(path).ok()?;
    let page_count = doc.page_count().ok()?;
    if page_count == 0 {
        return Some(Vec::new());
    }

    // Step 1: Sequential raw text extraction (PdfDocument is !Sync)
    let raw_texts: Vec<(usize, String)> = (0..page_count)
        .map(|i| {
            let text = doc.extract_text(i).unwrap_or_default();
            (i, text)
        })
        .collect();

    // Step 2: Parallel text cleaning and quality counting
    let pages: Vec<(DocumentPage, usize, usize)> = raw_texts
        .into_par_iter()
        .map(|(i, text)| {
            let cleaned = clean_pdf_text(&text);
            let mut printable = 0usize;
            let mut chars = 0usize;
            for ch in cleaned.chars() {
                chars += 1;
                if is_printable_pdf_char(ch) {
                    printable += 1;
                }
            }
            (
                DocumentPage {
                    page_number: (i + 1) as u32,
                    text: cleaned,
                },
                printable,
                chars,
            )
        })
        .collect();

    let total_printable: usize = pages.iter().map(|(_, p, _)| p).sum();
    let total_chars: usize = pages.iter().map(|(_, _, c)| c).sum();
    let pages: Vec<DocumentPage> = pages.into_iter().map(|(page, _, _)| page).collect();

    // Quality gate: reject if <60% printable (same threshold as pdf-extract path)
    if total_chars > 0 && (total_printable as f32 / total_chars as f32) < 0.60 {
        log::warn!("pdf_oxide extraction rejected: only {total_printable}/{total_chars} printable chars");
        return None;
    }

    // Reject if all pages are empty
    if pages.iter().all(|p| p.text.trim().is_empty()) {
        return None;
    }

    log::info!("pdf_oxide extracted {page_count} pages ({total_printable}/{total_chars} printable)");
    Some(pages)
}

/// Extract text per-page using lopdf.
///
/// lopdf's `Document` is not `Sync` (uses `Rc` internally), so we extract
/// raw text sequentially (fast — just reading the internal object tree) and
/// then run the expensive `clean_pdf_text` post-processing in parallel via
/// rayon.
fn extract_lopdf_pages(doc: &lopdf::Document) -> Vec<DocumentPage> {
    use rayon::prelude::*;

    let page_map = doc.get_pages();
    let mut page_nums: Vec<u32> = page_map.keys().cloned().collect();
    page_nums.sort();

    // Step 1: Sequential raw text extraction (cannot be parallelized — doc is !Sync)
    let raw_pages: Vec<(u32, String)> = page_nums
        .iter()
        .map(|&page_num| {
            let raw = doc.extract_text(&[page_num]).unwrap_or_default();
            (page_num, raw)
        })
        .collect();

    // Step 2: Parallel text cleaning (unicode normalization, ligature replacement,
    // whitespace cleanup — this is the expensive part for large documents)
    raw_pages
        .into_par_iter()
        .map(|(page_num, raw)| DocumentPage {
            page_number: page_num,
            text: clean_pdf_text(&raw),
        })
        .collect()
}

/// Read AcroForm widget annotation values and append them to page text.
///
/// AcroForm fields (the kind created by Adobe Acrobat, IRS forms, etc.) store
/// filled values in annotation dictionaries under the `/V` key. Neither
/// `pdf-extract` nor `lopdf::extract_text` reads these — they only see the
/// page content stream. This function reads `/V` values and appends any that
/// aren't already present in the page text.
fn append_acroform_values(doc: &lopdf::Document, pages: &mut [DocumentPage]) {
    let page_map = doc.get_pages();
    let mut page_nums: Vec<u32> = page_map.keys().cloned().collect();
    page_nums.sort();

    // Also try the global AcroForm /Fields array as a fallback.
    // XFA-based PDFs (like IRS forms) may store field annotations in the
    // document catalog's AcroForm rather than in individual page /Annots.
    let global_fields = extract_global_acroform_fields(doc);
    log::info!("AcroForm global extraction found {} fields", global_fields.len());
    if !global_fields.is_empty() && !pages.is_empty() {
        prepend_form_summary(&global_fields, pages);
        return;
    }

    for (page_idx, &page_num) in page_nums.iter().enumerate() {
        let page_id = match page_map.get(&page_num) {
            Some(id) => *id,
            None => continue,
        };

        // Get the page dictionary
        let page_dict = match doc.get_dictionary(page_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Get /Annots array
        let annots = match page_dict.get(b"Annots") {
            Ok(obj) => match doc.dereference(obj) {
                Ok((_, lopdf::Object::Array(arr))) => arr.clone(),
                _ => match obj {
                    lopdf::Object::Array(arr) => arr.clone(),
                    _ => continue,
                },
            },
            Err(_) => continue,
        };

        // Collect field values with their y-position for sorting
        let mut field_entries: Vec<(String, String, f32)> = Vec::new(); // (label, value, y)

        for annot_ref in &annots {
            let annot_id = match annot_ref {
                lopdf::Object::Reference(id) => id,
                _ => continue,
            };
            let annot_dict = match doc.get_dictionary(*annot_id) {
                Ok(d) => d,
                Err(_) => continue,
            };

            // Only process Widget annotations with text fields (FT = Tx)
            let is_widget = annot_dict
                .get(b"Subtype")
                .ok()
                .and_then(|o| o.as_name().ok())
                .map_or(false, |n| n == b"Widget");
            if !is_widget {
                continue;
            }

            // Check /FT on this widget; if absent, check /Parent's /FT (inherited).
            let local_ft = annot_dict
                .get(b"FT")
                .ok()
                .and_then(|o| o.as_name().ok())
                .map(|n| n.to_vec());
            let effective_ft = local_ft.or_else(|| {
                annot_dict
                    .get(b"Parent")
                    .ok()
                    .and_then(|p| match p {
                        lopdf::Object::Reference(pid) => doc.get_dictionary(*pid).ok(),
                        _ => None,
                    })
                    .and_then(|pd| pd.get(b"FT").ok())
                    .and_then(|o| o.as_name().ok())
                    .map(|n| n.to_vec())
            });
            let is_text = effective_ft.as_deref().map_or(false, |ft| ft == b"Tx" || ft == b"Ch");
            if !is_text {
                continue;
            }

            // Get /V (value)
            let value = match annot_dict.get(b"V") {
                Ok(lopdf::Object::String(bytes, _)) => {
                    String::from_utf8_lossy(bytes).trim().to_string()
                }
                _ => continue,
            };
            if value.is_empty() {
                continue;
            }

            // Get /T (field name) for a readable label
            let label = annot_dict
                .get(b"T")
                .ok()
                .and_then(|o| match o {
                    lopdf::Object::String(bytes, _) => {
                        Some(String::from_utf8_lossy(bytes).to_string())
                    }
                    _ => None,
                })
                .unwrap_or_default();

            // Get /Rect for y-position (for sorting top-to-bottom)
            let y = annot_dict
                .get(b"Rect")
                .ok()
                .and_then(|o| match o {
                    lopdf::Object::Array(arr) if arr.len() >= 4 => {
                        // Rect = [x0, y0, x1, y1] — use y1 (top of field)
                        arr[3]
                            .as_float()
                            .ok()
                            .or_else(|| arr[3].as_i64().ok().map(|i| i as f32))
                    }
                    _ => None,
                })
                .unwrap_or(0.0);

            field_entries.push((label, value, y));
        }

        if field_entries.is_empty() {
            continue;
        }

        // Sort top-to-bottom (descending y = top of page first)
        field_entries.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Collect as (label, value) pairs for the summary
        if page_idx >= pages.len() {
            continue;
        }
        let fields: Vec<(String, String)> = field_entries
            .into_iter()
            .map(|(label, value, _)| (label, value))
            .collect();
        if !fields.is_empty() {
            prepend_form_summary(&fields, &mut pages[page_idx..]);
        }
    }
}

/// Prepend a structured "FORM DATA" summary to the first page so that
/// filled values get their own dense, fact-rich chunk. Without this,
/// form values appended at the end of a huge boilerplate page get lost
/// in embedding space and never surface during retrieval.
/// Heuristic label for a form field value when the PDF field name is generic (f1_01, etc.).
fn infer_field_label(value: &str) -> &'static str {
    let v = value.trim();
    // SSN pattern: 3-2-4 digits or just digits in that range
    if v.len() <= 4 && v.chars().all(|c| c.is_ascii_digit()) {
        // Could be part of SSN or a short numeric field
        return "ID Number";
    }
    // Looks like a full SSN (xxx-xx-xxxx)
    if v.len() == 11
        && v.chars().filter(|c| *c == '-').count() == 2
        && v.replace('-', "").chars().all(|c| c.is_ascii_digit())
    {
        return "Social Security Number";
    }
    // ZIP code
    if (v.len() == 5 || v.len() == 10)
        && v.chars().next().map_or(false, |c| c.is_ascii_digit())
        && v.replace('-', "").chars().all(|c| c.is_ascii_digit())
    {
        return "ZIP Code";
    }
    // State abbreviation (2 uppercase letters)
    if v.len() == 2 && v.chars().all(|c| c.is_ascii_uppercase()) {
        return "State";
    }
    // Contains comma + state pattern → city/state
    if v.contains(',') && v.split(',').count() == 2 {
        let parts: Vec<&str> = v.split(',').collect();
        let after = parts[1].trim();
        if after.len() >= 2 && after.len() <= 12 {
            return "City, State, ZIP";
        }
    }
    // Looks like a street address (starts with digits, has words)
    if v.chars().next().map_or(false, |c| c.is_ascii_digit()) && v.contains(' ') && v.len() > 5 {
        return "Address";
    }
    // Multiple capitalized words → likely a name or business
    let words: Vec<&str> = v.split_whitespace().collect();
    if words.len() >= 2
        && words.len() <= 5
        && words
            .iter()
            .all(|w| w.chars().next().map_or(false, |c| c.is_uppercase()))
    {
        if words.len() <= 3 && words.iter().all(|w| w.len() <= 15) {
            return "Name";
        }
        return "Business/Entity Name";
    }
    "Form Field"
}

fn prepend_form_summary(fields: &[(String, String)], pages: &mut [DocumentPage]) {
    if fields.is_empty() || pages.is_empty() {
        return;
    }

    // Deduplicate: filter out fields whose values already appear verbatim in page text
    let page_text = &pages[0].text;
    let fields = deduplicate_form_fields(fields, page_text);

    let mut lines = Vec::new();

    for (label, value) in &fields {
        // Quality filter: skip garbage form fields (checkbox states, short fragments)
        let v = value.trim();
        if v.is_empty() || v.len() < 2 {
            continue;
        }
        let v_lower = v.to_lowercase();
        if matches!(v_lower.as_str(), "check" | "checked" | "off" | "yes" | "no" | "x" | "true" | "false") {
            continue;
        }
        // Skip standalone 1-2 digit numbers (date fragments, toggle values)
        if v.len() <= 2 && v.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let clean_label = label.replace("[0]", "").replace("[1]", "");
        let normalized_value = normalize_field_value(value);
        // For generic field names (f1_01, etc.), infer a descriptive label from the value
        if clean_label.is_empty()
            || clean_label.starts_with("f1_")
            || clean_label.starts_with("f2_")
        {
            let inferred = infer_field_label(&normalized_value);
            lines.push(format!("{inferred}: {normalized_value}"));
        } else {
            lines.push(format!("{clean_label}: {normalized_value}"));
        }
    }

    if !lines.is_empty() {
        let summary = format!("FILLED FORM DATA:\n{}\n\n", lines.join("\n"));
        pages[0].text = format!("{}{}", summary, pages[0].text);
    }
}

/// Re-interleave filled form values next to their template labels.
///
/// PDFs with filled forms have two layers: template labels and filled values,
/// rendered as separate XObjects. pdf-extract outputs them in stream order
/// (all labels first, then all values). This function:
///
/// 1. Uses lopdf to read XObject coordinates from the content stream
/// 2. Identifies which XObjects are "small" (filled field values) vs "large" (template)
/// 3. Extracts text from small XObjects (simple encoding, works with lopdf)
/// 4. Matches each filled value to the nearest template label by y-coordinate
/// 5. Inserts filled values next to their labels in the template text
/// Extract field values from the document-level AcroForm /Fields array.
///
/// IRS forms and other XFA-based PDFs store field widgets in the document
/// catalog's `/AcroForm` dictionary rather than (or in addition to) page-level
/// `/Annots`. This function walks the AcroForm field tree recursively to find
/// all text field values.
fn extract_global_acroform_fields(doc: &lopdf::Document) -> Vec<(String, String)> {
    let mut results = Vec::new();

    // Get the catalog → AcroForm → Fields
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return results,
    };

    let acroform_obj = match catalog.get(b"AcroForm") {
        Ok(obj) => obj,
        Err(_) => return results,
    };

    let acroform = match doc.dereference(acroform_obj) {
        Ok((_, lopdf::Object::Dictionary(d))) => d,
        _ => match acroform_obj {
            lopdf::Object::Dictionary(d) => d,
            _ => return results,
        },
    };

    let fields = match acroform.get(b"Fields") {
        Ok(lopdf::Object::Array(arr)) => arr.clone(),
        Ok(obj) => match doc.dereference(obj) {
            Ok((_, lopdf::Object::Array(arr))) => arr.clone(),
            _ => return results,
        },
        Err(_) => return results,
    };

    // Walk field tree (no inherited /FT at the root level)
    for field_ref in &fields {
        collect_acroform_field(doc, field_ref, &mut results, None);
    }

    results
}

/// Recursively collect text field values from an AcroForm field node.
/// `inherited_ft` carries the parent's /FT down the tree — many PDF forms
/// (IRS W-9, etc.) only set /FT on the parent node and children inherit it.
fn collect_acroform_field(
    doc: &lopdf::Document,
    obj: &lopdf::Object,
    results: &mut Vec<(String, String)>,
    inherited_ft: Option<&[u8]>,
) {
    let dict = match obj {
        lopdf::Object::Reference(id) => match doc.get_dictionary(*id) {
            Ok(d) => d,
            Err(_) => return,
        },
        lopdf::Object::Dictionary(d) => d,
        _ => return,
    };

    // Resolve this node's /FT (field type), falling back to inherited.
    let local_ft = dict
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| n.to_vec());
    let effective_ft: Option<&[u8]> = local_ft.as_deref().or(inherited_ft);

    // Check for /Kids (intermediate node in field tree)
    if let Ok(kids_obj) = dict.get(b"Kids") {
        let kids_arr = match doc.dereference(kids_obj) {
            Ok((_, lopdf::Object::Array(arr))) => Some(arr.clone()),
            _ => match kids_obj {
                lopdf::Object::Array(arr) => Some(arr.clone()),
                _ => None,
            },
        };
        if let Some(kids) = kids_arr {
            for kid in &kids {
                collect_acroform_field(doc, kid, results, effective_ft);
            }
            // Don't return yet — some parent nodes with /Kids also have /V
        }
    }

    // Check for /V value — accept text fields (Tx) or choice fields (Ch),
    // and also accept when no /FT is set (some forms omit it entirely).
    let is_acceptable = effective_ft.map_or(true, |ft| ft == b"Tx" || ft == b"Ch");
    if !is_acceptable {
        return;
    }

    let value = match dict.get(b"V") {
        Ok(lopdf::Object::String(bytes, _)) => String::from_utf8_lossy(bytes).trim().to_string(),
        _ => return,
    };
    if value.is_empty() {
        return;
    }

    let label = dict
        .get(b"T")
        .ok()
        .and_then(|o| match o {
            lopdf::Object::String(bytes, _) => Some(decode_pdf_string(bytes)),
            _ => None,
        })
        .unwrap_or_default()
        .replace("[0]", "")
        .replace("[1]", "");

    results.push((label, value));
}

fn reinterleave_form_fields(
    doc: &lopdf::Document,
    original_pages: &[DocumentPage],
) -> Vec<DocumentPage> {
    let page_map = doc.get_pages();
    let mut page_nums: Vec<u32> = page_map.keys().cloned().collect();
    page_nums.sort();

    let mut result_pages = Vec::new();

    for (page_idx, &page_num) in page_nums.iter().enumerate() {
        // Default: keep the original page text if reinterleaving fails or is unnecessary
        let fallback = page_idx < original_pages.len();

        let page_id = match page_map.get(&page_num) {
            Some(id) => *id,
            None => {
                if fallback {
                    result_pages.push(original_pages[page_idx].clone());
                }
                continue;
            }
        };

        let content_bytes = match doc.get_page_content(page_id) {
            Ok(b) => b,
            Err(_) => {
                if fallback {
                    result_pages.push(original_pages[page_idx].clone());
                }
                continue;
            }
        };

        let xobjects = collect_xobjects(doc, page_id);
        if xobjects.is_empty() {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        // Get all XObject invocations with their CTM positions
        let ops = scan_content_ops(&content_bytes);
        let mut xobj_entries: Vec<(String, [f32; 6])> = Vec::new(); // (name, ctm)
        let mut ctm_stack: Vec<[f32; 6]> = Vec::new();
        let mut ctm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

        for op in &ops {
            match op {
                ScannedOp::SaveState => ctm_stack.push(ctm),
                ScannedOp::RestoreState => {
                    if let Some(saved) = ctm_stack.pop() {
                        ctm = saved;
                    }
                }
                ScannedOp::ConcatMatrix(m) => {
                    ctm = multiply_matrices(ctm, *m);
                }
                ScannedOp::DoXObject(name) => {
                    xobj_entries.push((name.clone(), ctm));
                }
            }
        }

        if xobj_entries.len() < 2 {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        // Classify XObjects by content stream size: the template is large, fields are small
        let mut sized_entries: Vec<(String, [f32; 6], usize, lopdf::ObjectId)> = Vec::new();
        for (name, entry_ctm) in &xobj_entries {
            if let Some(&obj_id) = xobjects.get(name.as_str()) {
                let size = xobject_content_size(doc, obj_id);
                sized_entries.push((name.clone(), *entry_ctm, size, obj_id));
            }
        }

        if sized_entries.is_empty() {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        // Template = largest XObject; filled fields = the rest
        let max_size = sized_entries.iter().map(|e| e.2).max().unwrap_or(0);
        if max_size < 100 {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        let filled_fields: Vec<_> = sized_entries.iter().filter(|e| e.2 < max_size).collect();

        if filled_fields.is_empty() {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        // Extract text (via lopdf) and position for each filled field.
        // We use lopdf text primarily for matching — the actual text for replacement
        // comes from pdf-extract (which handles font encoding + spacing correctly).
        let mut field_entries: Vec<(String, f32, f32)> = Vec::new(); // (lopdf_text, x, y)
        for (_name, entry_ctm, _size, obj_id) in &filled_fields {
            let text = extract_xobject_text(doc, *obj_id);
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                field_entries.push((trimmed, entry_ctm[4], entry_ctm[5]));
            }
        }

        if field_entries.is_empty() {
            if fallback {
                result_pages.push(original_pages[page_idx].clone());
            }
            continue;
        }

        // Sort by visual position (-y, x) = top-to-bottom, left-to-right.
        // Use a wide y-band (30 PDF points) because filled-value XObjects may be placed
        // at slightly different y than their template labels on the same visual row.
        field_entries.sort_by(|a, b| {
            let ya = (-a.2 / 30.0).round() as i32;
            let yb = (-b.2 / 30.0).round() as i32;
            ya.cmp(&yb)
                .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Get the original page text from pdf-extract
        let original_text = if page_idx < original_pages.len() {
            &original_pages[page_idx].text
        } else {
            // More lopdf pages than pdf-extract pages — can't interleave
            continue;
        };

        // The filled values appear at the END of the pdf-extract output.
        // Find where the values block starts using the first coordinate-sorted
        // entry's lopdf text as an anchor. Then split the original text into
        // template + values, and replace blanks in the template.
        let first_lopdf = &field_entries[0].0;
        let insert_point = original_text.find(first_lopdf.as_str());
        let template_text = match insert_point {
            Some(pos) => &original_text[..pos],
            None => {
                if fallback {
                    result_pages.push(original_pages[page_idx].clone());
                }
                continue;
            }
        };

        // Use lopdf-extracted text for each value, with space recovery.
        // Most form field values are short (names, dates, amounts) and decode
        // correctly. For longer values where lopdf strips spaces, we insert
        // spaces before capital letters that follow lowercase letters.
        let field_values: Vec<String> = field_entries
            .iter()
            .map(|(t, _, _)| recover_spaces(t))
            .collect();

        // Replace each blank field (run of 4+ underscores) with the next value
        let mut interleaved = String::new();
        let mut value_idx = 0;
        let mut i = 0;
        let template_bytes = template_text.as_bytes();
        while i < template_bytes.len() {
            if template_bytes[i] == b'_' {
                let start = i;
                while i < template_bytes.len() && template_bytes[i] == b'_' {
                    i += 1;
                }
                let run_len = i - start;
                if run_len >= 4 && value_idx < field_values.len() {
                    interleaved.push_str(&field_values[value_idx]);
                    value_idx += 1;
                } else {
                    for _ in 0..run_len {
                        interleaved.push('_');
                    }
                }
            } else {
                interleaved.push(template_bytes[i] as char);
                i += 1;
            }
        }

        for val in &field_values[value_idx..] {
            interleaved.push('\n');
            interleaved.push_str(val);
        }

        let cleaned = clean_pdf_text(&interleaved);
        // Quality gate: if reinterleaved text is >30% shorter than original,
        // the split point was wrong and we lost content. Fall back to original
        // text, but prepend extracted field values as a FILLED FORM DATA block
        // so they get retrieval boosting.
        let orig_len = original_text.len();
        if cleaned.len() < orig_len * 7 / 10 {
            log::warn!(
                "reinterleave_form_fields: result ({} chars) is much shorter than original ({} chars), keeping original with form summary",
                cleaned.len(), orig_len,
            );
            if fallback {
                // Build a mini form summary from the extracted field values
                let form_lines: Vec<String> = field_values
                    .iter()
                    .filter(|v| v.trim().len() >= 2)
                    .map(|v| {
                        let normalized = normalize_field_value(v);
                        format!("Form Field: {normalized}")
                    })
                    .collect();
                let mut page = original_pages[page_idx].clone();
                if !form_lines.is_empty() {
                    let summary = format!("FILLED FORM DATA:\n{}\n\n", form_lines.join("\n"));
                    page.text = format!("{}{}", summary, page.text);
                }
                result_pages.push(page);
            }
        } else if !cleaned.trim().is_empty() {
            result_pages.push(DocumentPage {
                page_number: page_num,
                text: cleaned,
            });
        }
    }

    result_pages
}

/// Get the decompressed content size of an XObject stream.
fn xobject_content_size(doc: &lopdf::Document, obj_id: lopdf::ObjectId) -> usize {
    match doc.get_object(obj_id) {
        Ok(lopdf::Object::Stream(ref s)) => s
            .decompressed_content()
            .map(|b| b.len())
            .unwrap_or(s.content.len()),
        _ => 0,
    }
}

// ── Coordinate-aware PDF extraction ──────────────────────────────────────────

// ── Minimal content stream scanner ───────────────────────────────────────────
// We only need to recognize: q, Q, cm (6 numbers), Do (/Name).
// This avoids depending on lopdf's private parser module.

#[derive(Debug)]
enum ScannedOp {
    SaveState,              // q
    RestoreState,           // Q
    ConcatMatrix([f32; 6]), // a b c d e f cm
    DoXObject(String),      // /Name Do
}

/// Scan raw content stream bytes for q/Q/cm/Do operators.
/// Tracks a small operand stack of recent numbers and names to pair with operators.
fn scan_content_ops(bytes: &[u8]) -> Vec<ScannedOp> {
    let mut ops = Vec::new();
    let mut number_stack: Vec<f32> = Vec::new();
    let mut last_name: Option<String> = None;

    let text = String::from_utf8_lossy(bytes);
    let mut chars = text.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        // Skip whitespace
        if ch.is_ascii_whitespace() {
            chars.next();
            continue;
        }

        // PDF comment: skip to end of line
        if ch == '%' {
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if c == '\n' || c == '\r' {
                    break;
                }
            }
            continue;
        }

        // PDF name: /SomeName
        if ch == '/' {
            chars.next(); // consume '/'
            let start = i + 1;
            let mut end = start;
            while let Some(&(j, c)) = chars.peek() {
                if c.is_ascii_whitespace()
                    || c == '/'
                    || c == '['
                    || c == ']'
                    || c == '('
                    || c == ')'
                    || c == '<'
                    || c == '>'
                    || c == '{'
                    || c == '}'
                {
                    break;
                }
                end = j + c.len_utf8();
                chars.next();
            }
            last_name = Some(text[start..end].to_string());
            number_stack.clear(); // name resets number context
            continue;
        }

        // Number (integer or real)
        if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            let start = i;
            chars.next();
            let mut end = start + ch.len_utf8();
            while let Some(&(j, c)) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    end = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(n) = text[start..end].parse::<f32>() {
                number_stack.push(n);
            }
            continue;
        }

        // String literal (...) — skip (we don't need text from page-level stream)
        if ch == '(' {
            chars.next();
            let mut depth = 1;
            let mut prev_backslash = false;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if prev_backslash {
                    prev_backslash = false;
                    continue;
                }
                if c == '\\' {
                    prev_backslash = true;
                    continue;
                }
                if c == '(' {
                    depth += 1;
                }
                if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            continue;
        }

        // Hex string <...> or dict <<...>> — skip
        if ch == '<' {
            chars.next();
            if let Some(&(_, '<')) = chars.peek() {
                // dict — skip until >>
                chars.next();
                let mut depth = 1;
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == '<' {
                        if let Some(&(_, '<')) = chars.peek() {
                            chars.next();
                            depth += 1;
                        }
                    } else if c == '>' {
                        if let Some(&(_, '>')) = chars.peek() {
                            chars.next();
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
            } else {
                // hex string — skip until >
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == '>' {
                        break;
                    }
                }
            }
            continue;
        }

        // Array [...] — skip
        if ch == '[' {
            chars.next();
            let mut depth = 1;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if c == '[' {
                    depth += 1;
                }
                if c == ']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            continue;
        }

        // Alphabetic — this is an operator
        if ch.is_ascii_alphabetic() || ch == '\'' || ch == '"' {
            let start = i;
            chars.next();
            let mut end = start + ch.len_utf8();
            while let Some(&(j, c)) = chars.peek() {
                if c.is_ascii_alphabetic() || c == '*' {
                    end = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let operator = &text[start..end];
            match operator {
                "q" => {
                    ops.push(ScannedOp::SaveState);
                    number_stack.clear();
                }
                "Q" => {
                    ops.push(ScannedOp::RestoreState);
                    number_stack.clear();
                }
                "cm" => {
                    if number_stack.len() >= 6 {
                        let start_idx = number_stack.len() - 6;
                        let m = [
                            number_stack[start_idx],
                            number_stack[start_idx + 1],
                            number_stack[start_idx + 2],
                            number_stack[start_idx + 3],
                            number_stack[start_idx + 4],
                            number_stack[start_idx + 5],
                        ];
                        ops.push(ScannedOp::ConcatMatrix(m));
                    }
                    number_stack.clear();
                }
                "Do" => {
                    if let Some(name) = last_name.take() {
                        ops.push(ScannedOp::DoXObject(name));
                    }
                    number_stack.clear();
                }
                _ => {
                    // Any other operator resets the stacks
                    number_stack.clear();
                    last_name = None;
                }
            }
            continue;
        }

        // Anything else — skip
        chars.next();
    }

    ops
}

/// Collect XObject name→ObjectId mapping from a page's Resources dictionary.
fn collect_xobjects(
    doc: &lopdf::Document,
    page_id: lopdf::ObjectId,
) -> std::collections::HashMap<String, lopdf::ObjectId> {
    let mut map = std::collections::HashMap::new();

    let (res_dict, res_ids) = doc.get_page_resources(page_id);

    fn extract_from_dict(
        doc: &lopdf::Document,
        dict: &lopdf::Dictionary,
        map: &mut std::collections::HashMap<String, lopdf::ObjectId>,
    ) {
        let xobj = match dict.get(b"XObject") {
            Ok(obj) => obj,
            Err(_) => return,
        };
        let xobj_dict = match xobj {
            lopdf::Object::Dictionary(ref d) => d,
            lopdf::Object::Reference(id) => match doc.get_dictionary(*id) {
                Ok(d) => d,
                Err(_) => return,
            },
            _ => return,
        };
        for (name, val) in xobj_dict.iter() {
            if let Ok(id) = val.as_reference() {
                let name_str = String::from_utf8_lossy(name).to_string();
                map.insert(name_str, id);
            }
        }
    }

    if let Some(dict) = res_dict {
        extract_from_dict(doc, dict, &mut map);
    }
    for res_id in res_ids {
        if let Ok(dict) = doc.get_dictionary(res_id) {
            extract_from_dict(doc, dict, &mut map);
        }
    }
    map
}

/// Extract plain text from a Form XObject stream using lopdf's text extraction.
/// Uses `Document::decode_text` for font encoding, scanning the XObject's own
/// content stream for text-showing operators (Tj, TJ, ', ").
fn extract_xobject_text(doc: &lopdf::Document, obj_id: lopdf::ObjectId) -> String {
    let stream = match doc.get_object(obj_id) {
        Ok(lopdf::Object::Stream(ref s)) => s,
        _ => return String::new(),
    };

    // Verify it's a Form XObject
    let is_form = stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| n == b"Form")
        .unwrap_or(false);
    if !is_form {
        return String::new();
    }

    let content_bytes = match stream.decompressed_content() {
        Ok(b) => b,
        Err(_) => stream.content.clone(),
    };

    // Collect font encodings from this XObject's own Resources
    let xobj_fonts = collect_xobject_fonts(doc, &stream.dict);

    // Scan the XObject content stream for text operators
    extract_text_from_content_bytes(&content_bytes, &xobj_fonts)
}

/// Scan raw content stream bytes for text-showing operators and extract text.
fn extract_text_from_content_bytes(
    bytes: &[u8],
    fonts: &std::collections::HashMap<String, String>,
) -> String {
    let mut text = String::new();
    let mut current_font_encoding: Option<&str> = None;
    let mut number_stack: Vec<f32> = Vec::new();

    let raw = String::from_utf8_lossy(bytes);
    let mut chars = raw.char_indices().peekable();
    let mut last_name: Option<String> = None;
    let mut pending_strings: Vec<(Vec<u8>, bool)> = Vec::new(); // (bytes, is_hex)

    while let Some(&(cur_i, ch)) = chars.peek() {
        if ch.is_ascii_whitespace() {
            chars.next();
            continue;
        }

        // Comment
        if ch == '%' {
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if c == '\n' || c == '\r' {
                    break;
                }
            }
            continue;
        }

        // Name
        if ch == '/' {
            chars.next();
            let mut name = String::new();
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_whitespace() || b"/[]()<>{}".contains(&(c as u8)) {
                    break;
                }
                name.push(c);
                chars.next();
            }
            last_name = Some(name);
            continue;
        }

        // Number — track for Td displacement
        if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            let start_i = cur_i;
            chars.next();
            let mut end_i = start_i + ch.len_utf8();
            while let Some(&(j, c)) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    end_i = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(n) = raw[start_i..end_i].parse::<f32>() {
                number_stack.push(n);
            }
            continue;
        }

        // String literal (...)
        if ch == '(' {
            chars.next();
            let mut depth = 1;
            let mut string_bytes = Vec::new();
            let mut prev_backslash = false;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if prev_backslash {
                    prev_backslash = false;
                    match c {
                        'n' => string_bytes.push(b'\n'),
                        'r' => string_bytes.push(b'\r'),
                        't' => string_bytes.push(b'\t'),
                        '\\' => string_bytes.push(b'\\'),
                        '(' => string_bytes.push(b'('),
                        ')' => string_bytes.push(b')'),
                        _ => string_bytes.push(c as u8),
                    }
                    continue;
                }
                if c == '\\' {
                    prev_backslash = true;
                    continue;
                }
                if c == '(' {
                    depth += 1;
                    string_bytes.push(b'(');
                    continue;
                }
                if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    string_bytes.push(b')');
                    continue;
                }
                string_bytes.push(c as u8);
            }
            pending_strings.push((string_bytes, false));
            continue;
        }

        // Hex string <...>
        if ch == '<' {
            chars.next();
            if let Some(&(_, '<')) = chars.peek() {
                // Dict — skip
                chars.next();
                let mut depth = 1;
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == '<' {
                        if let Some(&(_, '<')) = chars.peek() {
                            chars.next();
                            depth += 1;
                        }
                    } else if c == '>' {
                        if let Some(&(_, '>')) = chars.peek() {
                            chars.next();
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
            } else {
                let mut hex = String::new();
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == '>' {
                        break;
                    }
                    if c.is_ascii_hexdigit() {
                        hex.push(c);
                    }
                }
                let hex_bytes: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&hex[i..(i + 2).min(hex.len())], 16).ok())
                    .collect();
                pending_strings.push((hex_bytes, true));
            }
            continue;
        }

        // Array [...]
        if ch == '[' {
            chars.next();
            // For TJ arrays, we need to collect string items
            // Simple approach: collect all strings inside the array
            let mut depth = 1;
            let mut arr_strings: Vec<(Vec<u8>, bool)> = Vec::new();
            while let Some(&(_, c)) = chars.peek() {
                if c == ']' {
                    chars.next();
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                if c == '[' {
                    chars.next();
                    depth += 1;
                    continue;
                }
                if c == '(' {
                    chars.next();
                    let mut sdepth = 1;
                    let mut sbytes = Vec::new();
                    let mut esc = false;
                    while let Some(&(_, sc)) = chars.peek() {
                        chars.next();
                        if esc {
                            esc = false;
                            match sc {
                                'n' => sbytes.push(b'\n'),
                                'r' => sbytes.push(b'\r'),
                                't' => sbytes.push(b'\t'),
                                _ => sbytes.push(sc as u8),
                            };
                            continue;
                        }
                        if sc == '\\' {
                            esc = true;
                            continue;
                        }
                        if sc == '(' {
                            sdepth += 1;
                            sbytes.push(b'(');
                            continue;
                        }
                        if sc == ')' {
                            sdepth -= 1;
                            if sdepth == 0 {
                                break;
                            }
                            sbytes.push(b')');
                            continue;
                        }
                        sbytes.push(sc as u8);
                    }
                    arr_strings.push((sbytes, false));
                    continue;
                }
                if c == '<' {
                    chars.next();
                    if let Some(&(_, '<')) = chars.peek() {
                        // Skip dict inside array (rare)
                        chars.next();
                        let mut ddepth = 1;
                        while let Some(&(_, dc)) = chars.peek() {
                            chars.next();
                            if dc == '<' {
                                if let Some(&(_, '<')) = chars.peek() {
                                    chars.next();
                                    ddepth += 1;
                                }
                            } else if dc == '>' {
                                if let Some(&(_, '>')) = chars.peek() {
                                    chars.next();
                                    ddepth -= 1;
                                    if ddepth == 0 {
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        let mut hex = String::new();
                        while let Some(&(_, hc)) = chars.peek() {
                            chars.next();
                            if hc == '>' {
                                break;
                            }
                            if hc.is_ascii_hexdigit() {
                                hex.push(hc);
                            }
                        }
                        let hbytes: Vec<u8> = (0..hex.len())
                            .step_by(2)
                            .filter_map(|i| {
                                u8::from_str_radix(&hex[i..(i + 2).min(hex.len())], 16).ok()
                            })
                            .collect();
                        arr_strings.push((hbytes, true));
                    }
                    continue;
                }
                chars.next();
            }
            if !arr_strings.is_empty() {
                pending_strings = arr_strings;
            }
            continue;
        }

        // Operator (alphabetic)
        if ch.is_ascii_alphabetic() || ch == '\'' || ch == '"' {
            let mut operator = String::new();
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphabetic() || c == '*' || c == '\'' || c == '"' {
                    operator.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            match operator.as_str() {
                "Tf" => {
                    if let Some(ref name) = last_name {
                        current_font_encoding = fonts.get(name).map(|s| s.as_str());
                    }
                    last_name = None;
                    pending_strings.clear();
                    number_stack.clear();
                }
                "Tj" | "'" => {
                    if let Some((ref bytes, _)) = pending_strings.last() {
                        text.push_str(&lopdf::Document::decode_text(current_font_encoding, bytes));
                    }
                    pending_strings.clear();
                    last_name = None;
                    number_stack.clear();
                }
                "TJ" => {
                    for (ref bytes, _) in &pending_strings {
                        text.push_str(&lopdf::Document::decode_text(current_font_encoding, bytes));
                    }
                    pending_strings.clear();
                    last_name = None;
                    number_stack.clear();
                }
                "Td" | "TD" => {
                    // Td tx ty: move text position by (tx, ty).
                    // If ty is near zero, it's a horizontal move (space between words).
                    // If ty is significantly nonzero, it's a new line.
                    let ty = if number_stack.len() >= 2 {
                        number_stack[number_stack.len() - 1]
                    } else {
                        -1.0 // assume newline if we can't parse
                    };
                    if !text.is_empty() {
                        if ty.abs() < 2.0 {
                            // Small y-displacement: horizontal move = word space
                            if !text.ends_with(' ') && !text.ends_with('\n') {
                                text.push(' ');
                            }
                        } else if !text.ends_with('\n') {
                            text.push('\n');
                        }
                    }
                    number_stack.clear();
                    pending_strings.clear();
                    last_name = None;
                }
                _ => {
                    pending_strings.clear();
                    last_name = None;
                    number_stack.clear();
                }
            }
            continue;
        }

        chars.next();
    }

    text
}

/// Collect font encoding info from an XObject's own Resources dictionary.
fn collect_xobject_fonts(
    doc: &lopdf::Document,
    stream_dict: &lopdf::Dictionary,
) -> std::collections::HashMap<String, String> {
    let mut fonts = std::collections::HashMap::new();

    let resources = match stream_dict.get(b"Resources") {
        Ok(obj) => match obj {
            lopdf::Object::Dictionary(ref d) => d,
            lopdf::Object::Reference(id) => match doc.get_dictionary(*id) {
                Ok(d) => d,
                Err(_) => return fonts,
            },
            _ => return fonts,
        },
        Err(_) => return fonts,
    };

    let font_dict = match resources.get(b"Font") {
        Ok(obj) => match obj {
            lopdf::Object::Dictionary(ref d) => d,
            lopdf::Object::Reference(id) => match doc.get_dictionary(*id) {
                Ok(d) => d,
                Err(_) => return fonts,
            },
            _ => return fonts,
        },
        Err(_) => return fonts,
    };

    for (name, val) in font_dict.iter() {
        let name_str = String::from_utf8_lossy(name).to_string();
        let font = match val {
            lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok(),
            lopdf::Object::Dictionary(ref d) => Some(d),
            _ => None,
        };
        if let Some(font) = font {
            if let Ok(encoding) = font.get(b"Encoding") {
                if let Ok(enc_name) = encoding.as_name() {
                    let enc_str = String::from_utf8_lossy(enc_name).to_string();
                    fonts.insert(name_str, enc_str);
                }
            }
        }
    }
    fonts
}

/// Recover missing word spaces in lopdf-extracted text.
///
/// When lopdf extracts text from Form XObjects, it often strips inter-word
/// spaces because the PDF uses text positioning operators (Td) rather than
/// literal space characters. This heuristic inserts spaces:
/// - Before an uppercase letter preceded by a lowercase letter (`EagleRow` → `Eagle Row`)
/// - Before a digit preceded by a letter or vice versa (`time2pm` → `time 2pm`)
/// - Before `$` preceded by a non-space
/// - After `,` when not followed by a space or digit
fn recover_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 1 {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len() + text.len() / 4);
    result.push(chars[0]);
    for i in 1..chars.len() {
        let prev = chars[i - 1];
        let cur = chars[i];
        let need_space =
            // camelCase: lowercase followed by uppercase
            (prev.is_lowercase() && cur.is_uppercase())
            // digit-to-letter boundary: `18Eagle` → `18 Eagle`
            || (prev.is_ascii_digit() && cur.is_alphabetic())
            // Before $ when preceded by non-whitespace non-$
            || (cur == '$' && !prev.is_whitespace() && prev != '$')
            // After comma when followed by a letter (not digit for decimals)
            || (prev == ',' && cur.is_alphabetic());

        if need_space && !prev.is_whitespace() {
            result.push(' ');
        }
        result.push(cur);
    }
    result
}

/// Multiply two 2D affine transformation matrices (each stored as [a,b,c,d,e,f]).
/// Result = current * new (post-multiplication, as PDF spec requires).
fn multiply_matrices(cur: [f32; 6], new: [f32; 6]) -> [f32; 6] {
    [
        new[0] * cur[0] + new[1] * cur[2],
        new[0] * cur[1] + new[1] * cur[3],
        new[2] * cur[0] + new[3] * cur[2],
        new[2] * cur[1] + new[3] * cur[3],
        new[4] * cur[0] + new[5] * cur[2] + cur[4],
        new[4] * cur[1] + new[5] * cur[3] + cur[5],
    ]
}

/// Split a pdf-extract full-document string (pages separated by \x0c) into
/// DocumentPage entries. Returns an empty Vec if the text looks garbled (too
/// few printable characters — typically means pdf-extract also failed to decode
/// the font encoding and returned garbage).
fn pdf_extract_pages(raw: &str) -> Vec<DocumentPage> {
    let total_chars = raw.chars().count();
    if total_chars == 0 {
        return Vec::new();
    }

    // Quality gate: at least 60 % of characters must be printable
    // (printable = not a control char and not in Unicode Private Use Area).
    let printable = raw
        .chars()
        .filter(|&c| {
            let code = c as u32;
            c == '\n'
                || c == '\t'
                || c == '\x0c'
                || (!c.is_control() && !(0xE000..=0xF8FF).contains(&code) && code < 0xFFF0)
        })
        .count();
    let ratio = printable as f32 / total_chars as f32;
    if ratio < 0.60 {
        return Vec::new();
    }

    // Split on form-feed to get per-page text.
    let mut pages = Vec::new();
    for (i, page_text) in raw.split('\x0c').enumerate() {
        let cleaned = clean_pdf_text(page_text);
        if !cleaned.trim().is_empty() {
            pages.push(DocumentPage {
                page_number: (i + 1) as u32,
                text: cleaned,
            });
        }
    }
    pages
}

/// Returns true if a character is safe to keep in extracted PDF text.
///
/// lopdf commonly returns Unicode Private Use Area codepoints (U+E000–U+F8FF)
/// and raw control bytes for PDFs whose fonts lack a ToUnicode map. To users
/// and the LLM these look like "encrypted" garbage. Strip them here at the
/// source so they never reach the vector store or the model prompt.
fn is_printable_pdf_char(ch: char) -> bool {
    // Structural whitespace we explicitly preserve
    if ch == '\n' || ch == '\t' {
        return true;
    }
    // All other control characters (including \r, \x00 – \x1F, \x7F – \x9F)
    if ch.is_control() {
        return false;
    }
    let code = ch as u32;
    // Unicode Private Use Area — the primary source of PDF font-encoding garbage
    if (0xE000..=0xF8FF).contains(&code) {
        return false;
    }
    // Specials block, surrogates, noncharacters
    if code >= 0xFFF0 {
        return false;
    }
    true
}

/// Decode a PDF string that may be UTF-16BE (with BOM) or plain Latin-1.
fn decode_pdf_string(bytes: &[u8]) -> String {
    // UTF-16BE strings start with BOM: FE FF
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    Some(u16::from_be_bytes([c[0], c[1]]))
                } else {
                    None
                }
            })
            .collect();
        String::from_utf16_lossy(&chars)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

fn clean_pdf_text(text: &str) -> String {
    // ── Pass 1: string-level substitutions ───────────────────────────────────
    // These run before the character loop so every subsequent step sees clean
    // Unicode and plain ASCII where possible.

    // Lopdf identity-H placeholder
    let text = text.replace("?Identity-H Unimplemented?", " ");

    // OpenType / PDF ligatures — very common in professionally typeset legal
    // documents. Without this, "first" may arrive as "ﬁrst" (U+FB01), which
    // looks correct visually but doesn't match the plain-ASCII "fi" that the
    // embedding model was trained on.
    let text = text
        .replace('\u{FB00}', "ff") // ﬀ
        .replace('\u{FB01}', "fi") // ﬁ
        .replace('\u{FB02}', "fl") // ﬂ
        .replace('\u{FB03}', "ffi") // ﬃ
        .replace('\u{FB04}', "ffl") // ﬄ
        .replace('\u{FB05}', "st") // ﬅ
        .replace('\u{FB06}', "st"); // ﬆ

    // Typographic quotes → straight ASCII (keeps tokenisation consistent)
    let text = text
        .replace('\u{2018}', "'") // ' left single
        .replace('\u{2019}', "'") // ' right single / apostrophe
        .replace('\u{201C}', "\"") // " left double
        .replace('\u{201D}', "\"") // " right double
        .replace('\u{201A}', ",") // ‚ single low-9 (misused as comma in some fonts)
        .replace('\u{201E}', "\""); // „ double low-9

    // Dashes and special spaces
    let text = text
        .replace('\u{2013}', "-") // – en dash → hyphen
        .replace('\u{00A0}', " ") // non-breaking space → regular space
        .replace('\u{00AD}', ""); // soft hyphen (invisible) → remove

    // ── Pass 2: character-level whitespace normalization + PUA stripping ─────
    // Preserve newlines, collapse runs of other whitespace to a single space,
    // and silently drop non-printable / Private-Use-Area characters.
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;
    let mut prev_was_newline = false;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' {
            if !prev_was_newline {
                result.push('\n');
                prev_was_newline = true;
                prev_was_space = true;
            }
        } else if ch.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
            prev_was_newline = false;
        } else if is_printable_pdf_char(ch) {
            result.push(ch);
            prev_was_space = false;
            prev_was_newline = false;
        }
        // Non-printable / private-use-area chars are silently dropped.
    }

    // ── Pass 3: post-normalization fixups ────────────────────────────────────

    // Remove hyphenated line breaks: "agree-\nment" → "agreement".
    // Legal PDFs routinely hyphenate long words at the right margin. Keeping
    // the hyphen+newline would split the word across two chunks, poisoning
    // embeddings for both.  We join only when the character immediately after
    // the newline is a letter (avoids touching list items like "- \nItem").
    let result = remove_hyphen_breaks(&result);

    // Collapse runs of 3+ consecutive newlines down to 2.  Some PDFs emit a
    // newline for every line of whitespace between sections; more than two
    // consecutive newlines adds no structural information for the chunker and
    // just inflates token counts.
    let result = collapse_blank_lines(&result);

    result.trim().to_string()
}

/// Join hyphenated line breaks: `word-\nword` → `wordword` (the hyphen was
/// a typographic line-break marker, not a real hyphen).
fn remove_hyphen_breaks(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < len {
        // Pattern: '-' followed by '\n' followed by a letter → join without hyphen.
        if i + 2 < len && chars[i] == '-' && chars[i + 1] == '\n' && chars[i + 2].is_alphabetic() {
            // Drop the hyphen and newline; the next character follows normally.
            i += 2;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Collapse 3 or more consecutive newlines to exactly 2.
fn collapse_blank_lines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut newline_run = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                result.push('\n');
            }
            // 3rd+ newline in a run: silently drop
        } else {
            newline_run = 0;
            result.push(ch);
        }
    }
    result
}

/// Extract text from DOCX with structural awareness.
/// Preserves heading hierarchy by prepending markers.
/// Splits on explicit page breaks <w:br w:type="page"/>
pub fn parse_docx(path: &str) -> Result<Vec<DocumentPage>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("DOCX open error: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("ZIP error: {e}"))?;

    let mut xml_content = String::new();
    {
        let mut doc_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| format!("document.xml not found: {e}"))?;
        doc_xml
            .read_to_string(&mut xml_content)
            .map_err(|e| format!("Read error: {e}"))?;
    }

    let doc =
        roxmltree::Document::parse(&xml_content).map_err(|e| format!("XML parse error: {e}"))?;

    let ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let mut pages: Vec<String> = vec![String::new()];

    // Walk top-level body children (paragraphs, tables) for structure-aware extraction
    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        if node.tag_name().namespace() != Some(ns) {
            continue;
        }
        match node.tag_name().name() {
            "p" => {
                // Detect heading style from paragraph properties
                let heading_level = detect_docx_heading_level(&node, ns);

                // Collect all text runs within this paragraph
                let para_text = collect_paragraph_text(&node, ns);

                if let Some(last) = pages.last_mut() {
                    if !last.is_empty() && !last.ends_with('\n') {
                        last.push('\n');
                    }

                    if !para_text.trim().is_empty() {
                        if let Some(level) = heading_level {
                            // Prepend heading markers
                            let prefix = "#".repeat(level.min(6));
                            last.push_str(&format!("{} {}", prefix, para_text.trim()));
                        } else {
                            last.push_str(&para_text);
                        }
                    }
                }
            }
            "br" => {
                // Check for page break (at top level, outside paragraphs)
                let is_page_break = node
                    .attributes()
                    .any(|a| a.name() == "type" && a.value() == "page");
                if is_page_break {
                    pages.push(String::new());
                } else {
                    if let Some(last) = pages.last_mut() {
                        last.push('\n');
                    }
                }
            }
            _ => {}
        }
    }

    let result: Vec<DocumentPage> = pages
        .into_iter()
        .enumerate()
        .filter(|(_, text)| !text.trim().is_empty())
        .map(|(i, text)| DocumentPage {
            page_number: i as u32 + 1,
            text: text.trim().to_string(),
        })
        .collect();

    if result.is_empty() {
        return Err("No text content found in DOCX".to_string());
    }

    Ok(result)
}

/// Detect the heading level of a DOCX paragraph from its style properties.
/// Returns Some(1) for Heading1, Some(2) for Heading2, etc., or None for body text.
fn detect_docx_heading_level(para_node: &roxmltree::Node, ns: &str) -> Option<usize> {
    // Look for w:pPr > w:pStyle with val like "Heading1", "Heading2", etc.
    for child in para_node.children() {
        if !child.is_element() {
            continue;
        }
        if child.tag_name().namespace() != Some(ns) || child.tag_name().name() != "pPr" {
            continue;
        }
        for prop in child.children() {
            if !prop.is_element() {
                continue;
            }
            if prop.tag_name().namespace() != Some(ns) {
                continue;
            }
            if prop.tag_name().name() == "pStyle" {
                if let Some(val) = prop.attribute((ns, "val")).or_else(|| prop.attribute("val")) {
                    let lower = val.to_lowercase();
                    // Match "heading1", "heading2", etc. and also "title"
                    if lower == "title" {
                        return Some(1);
                    }
                    if lower == "subtitle" {
                        return Some(2);
                    }
                    if lower.starts_with("heading") {
                        if let Some(level_str) = lower.strip_prefix("heading") {
                            if let Ok(level) = level_str.trim().parse::<usize>() {
                                return Some(level);
                            }
                        }
                    }
                }
            }
            // Check for outlineLvl (used by some DOCX generators instead of named styles)
            if prop.tag_name().name() == "outlineLvl" {
                if let Some(val) = prop.attribute((ns, "val")).or_else(|| prop.attribute("val")) {
                    if let Ok(level) = val.parse::<usize>() {
                        return Some(level + 1); // outlineLvl is 0-based
                    }
                }
            }
        }
        // Also check if the paragraph is bold-only (likely a section title):
        // Look for w:rPr > w:b without w:bCs being false
        let has_bold_style = child.children().any(|rpr| {
            rpr.is_element()
                && rpr.tag_name().name() == "rPr"
                && rpr.tag_name().namespace() == Some(ns)
                && rpr.children().any(|b| {
                    b.is_element()
                        && b.tag_name().name() == "b"
                        && b.tag_name().namespace() == Some(ns)
                        && b.attribute((ns, "val")).unwrap_or("true") != "false"
                })
        });
        if has_bold_style {
            // Bold paragraph with no explicit heading style — treat as an informal heading
            // Only if it's short (likely a section title, not a bold body paragraph)
            return None; // We'll check text length below
        }
    }
    None
}

/// Collect all text content from a DOCX paragraph node, handling runs and breaks.
fn collect_paragraph_text(para_node: &roxmltree::Node, ns: &str) -> String {
    let mut text = String::new();
    for descendant in para_node.descendants() {
        if !descendant.is_element() {
            continue;
        }
        if descendant.tag_name().namespace() != Some(ns) {
            continue;
        }
        match descendant.tag_name().name() {
            "t" => {
                if let Some(t) = descendant.text() {
                    text.push_str(t);
                }
            }
            "br" => {
                let is_page_break = descendant
                    .attributes()
                    .any(|a| a.name() == "type" && a.value() == "page");
                if is_page_break {
                    // Page breaks inside paragraphs are handled at the paragraph level
                } else {
                    text.push('\n');
                }
            }
            "tab" => {
                text.push('\t');
            }
            _ => {}
        }
    }

    text
}

fn enforce_file_security(path: &str) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Could not read file metadata for {path}: {e}"))?;

    if !meta.is_file() {
        return Err(format!("Not a regular file: {path}"));
    }
    if meta.len() == 0 {
        return Err(format!("File is empty: {path}"));
    }
    if meta.len() > MAX_FILE_SIZE_BYTES {
        return Err(format!(
            "File is too large ({:.1} MB). Limit is {:.1} MB.",
            meta.len() as f64 / 1_000_000.0,
            MAX_FILE_SIZE_BYTES as f64 / 1_000_000.0
        ));
    }
    Ok(())
}

fn read_head(path: &str, n: usize) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("Open error: {e}"))?;
    let mut buf = vec![0u8; n];
    let bytes_read = file
        .read(&mut buf)
        .map_err(|e| format!("Read error: {e}"))?;
    buf.truncate(bytes_read);
    Ok(buf)
}

fn normalize_text(mut text: String) -> String {
    if text.len() > MAX_TEXT_CHARS {
        let cut = text.floor_char_boundary(MAX_TEXT_CHARS);
        text.truncate(cut);
    }

    text = text.replace('\r', "\n");
    text = text.replace('\t', " ");

    let mut cleaned = String::with_capacity(text.len());
    let mut newline_run = 0usize;
    for ch in text.chars() {
        let code = ch as u32;
        let printable =
            ch == '\n' || (!ch.is_control() && !(0xE000..=0xF8FF).contains(&code) && code < 0xFFF0);
        if !printable {
            continue;
        }

        if ch == '\n' {
            newline_run += 1;
            if newline_run > 2 {
                continue;
            }
        } else {
            newline_run = 0;
        }
        cleaned.push(ch);
    }

    let mut deduped = String::new();
    let mut prev = String::new();
    let mut run = 0usize;
    for line in cleaned.lines() {
        let normalized = line.trim();
        if normalized.is_empty() {
            deduped.push('\n');
            prev.clear();
            run = 0;
            continue;
        }

        if normalized == prev {
            run += 1;
            if run >= 3 {
                continue;
            }
        } else {
            prev = normalized.to_string();
            run = 0;
        }

        deduped.push_str(normalized);
        deduped.push('\n');
    }

    deduped.trim().to_string()
}

fn paginate_text(text: &str) -> Vec<DocumentPage> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    if text.len() <= PAGE_CHAR_BUDGET {
        return vec![DocumentPage {
            page_number: 1,
            text: text.to_string(),
        }];
    }

    let mut pages = Vec::new();
    let mut start = 0usize;
    let mut page_number = 1u32;
    while start < text.len() {
        let mut end = (start + PAGE_CHAR_BUDGET).min(text.len());
        end = text.floor_char_boundary(end);
        if end < text.len() {
            let window = &text[start..end];
            if let Some(idx) = window.rfind('\n') {
                let candidate = start + idx;
                if candidate > start + PAGE_CHAR_BUDGET / 2 {
                    end = candidate;
                }
            }
        }
        if end <= start {
            break;
        }

        let page = text[start..end].trim();
        if !page.is_empty() {
            pages.push(DocumentPage {
                page_number,
                text: page.to_string(),
            });
            page_number += 1;
        }
        start = end;
    }
    pages
}

fn parse_plain_text(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("Text read error: {e}"))?;
    let text = normalize_text(String::from_utf8_lossy(&bytes).into_owned());
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No text content found".to_string());
    }
    Ok(pages)
}

fn parse_markdown(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("Markdown read error: {e}"))?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    // Keep heading words, drop markdown punctuation to reduce token overhead.
    let re_md = Regex::new(r"(?m)^\s{0,3}([#>\-\*\+]\s+|`{3,}.*$)").map_err(|e| e.to_string())?;
    text = re_md.replace_all(&text, "").to_string();
    let text = normalize_text(text);
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No markdown content found".to_string());
    }
    Ok(pages)
}

fn parse_csv(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("CSV parse error: {e}"))?;

    let headers = rdr
        .headers()
        .map_err(|e| format!("CSV headers error: {e}"))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect::<Vec<String>>();

    let mut rows: Vec<String> = Vec::new();
    for (idx, rec) in rdr.records().enumerate() {
        if idx >= 50_000 {
            break;
        }
        let record = rec.map_err(|e| format!("CSV row error: {e}"))?;
        let mut parts = Vec::new();
        for (col_i, val) in record.iter().enumerate().take(200) {
            let cell = val.trim();
            if cell.is_empty() {
                continue;
            }
            let key = headers
                .get(col_i)
                .cloned()
                .unwrap_or_else(|| format!("col_{}", col_i + 1));
            let clipped = cell.chars().take(500).collect::<String>();
            parts.push(format!("{key}: {clipped}"));
        }
        if parts.is_empty() {
            continue;
        }
        rows.push(format!("Row {} | {}", idx + 1, parts.join(" | ")));
    }

    if rows.is_empty() {
        return Err("No usable CSV rows found".to_string());
    }

    let text = normalize_text(rows.join("\n"));
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No CSV content found".to_string());
    }
    Ok(pages)
}

fn html_to_text(html: &str) -> Result<String, String> {
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").map_err(|e| e.to_string())?;
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").map_err(|e| e.to_string())?;
    let re_nav = Regex::new(r"(?is)<nav[^>]*>.*?</nav>").map_err(|e| e.to_string())?;
    let re_footer = Regex::new(r"(?is)<footer[^>]*>.*?</footer>").map_err(|e| e.to_string())?;
    let re_aside = Regex::new(r"(?is)<aside[^>]*>.*?</aside>").map_err(|e| e.to_string())?;
    let no_script = re_script.replace_all(html, " ");
    let no_style = re_style.replace_all(&no_script, " ");
    let no_nav = re_nav.replace_all(&no_style, " ");
    let no_footer = re_footer.replace_all(&no_nav, " ");
    let reduced = re_aside.replace_all(&no_footer, " ");
    Ok(html2text::from_read(reduced.as_bytes(), 120))
}

fn parse_html(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("HTML read error: {e}"))?;
    let text = html_to_text(&String::from_utf8_lossy(&bytes))?;
    let text = normalize_text(text);
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No HTML content found".to_string());
    }
    Ok(pages)
}

fn collect_mail_bodies(
    part: &mailparse::ParsedMail<'_>,
    plain: &mut Vec<String>,
    html: &mut Vec<String>,
) {
    if part.subparts.is_empty() {
        let mime = part.ctype.mimetype.to_ascii_lowercase();
        if mime == "text/plain" {
            if let Ok(body) = part.get_body() {
                plain.push(body);
            }
        } else if mime == "text/html" {
            if let Ok(body) = part.get_body() {
                html.push(body);
            }
        }
        return;
    }
    for sub in &part.subparts {
        collect_mail_bodies(sub, plain, html);
    }
}

fn parse_eml(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("EML read error: {e}"))?;
    let parsed = mailparse::parse_mail(&bytes).map_err(|e| format!("EML parse error: {e}"))?;

    let mut out = String::new();
    for header in ["Subject", "From", "To", "Cc", "Date"] {
        if let Some(v) = parsed.headers.get_first_value(header) {
            out.push_str(&format!("{header}: {}\n", v.trim()));
        }
    }
    out.push('\n');

    let mut plain = Vec::new();
    let mut html = Vec::new();
    collect_mail_bodies(&parsed, &mut plain, &mut html);

    if !plain.is_empty() {
        out.push_str("Email Body:\n");
        out.push_str(&plain.join("\n\n"));
    } else if !html.is_empty() {
        out.push_str("Email Body:\n");
        out.push_str(&html_to_text(&html.join("\n\n"))?);
    }

    let text = normalize_text(out);
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No email content found".to_string());
    }
    Ok(pages)
}

fn parse_mhtml(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("MHTML read error: {e}"))?;
    let parsed = mailparse::parse_mail(&bytes).map_err(|e| format!("MHTML parse error: {e}"))?;

    let mut plain = Vec::new();
    let mut html = Vec::new();
    collect_mail_bodies(&parsed, &mut plain, &mut html);

    let body = if !html.is_empty() {
        html_to_text(&html.join("\n\n"))?
    } else {
        plain.join("\n\n")
    };

    let text = normalize_text(body);
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No MHTML content found".to_string());
    }
    Ok(pages)
}

fn parse_xml(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let raw = std::fs::read_to_string(path).map_err(|e| format!("XML read error: {e}"))?;
    let lower = raw.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Err(
            "XML DOCTYPE/ENTITY declarations are not allowed for security reasons.".to_string(),
        );
    }

    let doc = roxmltree::Document::parse(&raw).map_err(|e| format!("XML parse error: {e}"))?;
    let mut lines = Vec::<String>::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let path = node
            .ancestors()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<&str>>()
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join("/");

        if let Some(text) = node.text().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            let clipped = text.chars().take(500).collect::<String>();
            lines.push(format!("{path}: {clipped}"));
        }

        for attr in node.attributes() {
            let v = attr.value().trim();
            if !v.is_empty() {
                let clipped = v.chars().take(300).collect::<String>();
                lines.push(format!("{path}@{}: {clipped}", attr.name()));
            }
        }

        if lines.len() > 100_000 {
            break;
        }
    }

    if lines.is_empty() {
        return Err("No XML textual content found".to_string());
    }

    let text = normalize_text(lines.join("\n"));
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No XML content found".to_string());
    }
    Ok(pages)
}

fn cell_to_string<T: std::fmt::Display>(v: T) -> String {
    let s = v.to_string();
    s.trim().to_string()
}

fn parse_xlsx(path: &str) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("XLSX open error: {e}"))?;

    let mut lines = Vec::<String>::new();
    let sheets = workbook.sheet_names().to_owned();
    for sheet in sheets {
        let Ok(range) = workbook.worksheet_range(&sheet) else {
            continue;
        };
        let mut header: Vec<String> = Vec::new();

        for (row_idx, row) in range.rows().enumerate() {
            if row_idx >= 10_000 {
                break;
            }
            let values: Vec<String> = row.iter().map(cell_to_string).collect();
            if values.iter().all(|v| v.is_empty()) {
                continue;
            }

            if header.is_empty() {
                header = values;
                continue;
            }

            let mut parts = Vec::new();
            for (i, cell) in values.iter().enumerate().take(200) {
                if cell.is_empty() {
                    continue;
                }
                let key = header
                    .get(i)
                    .filter(|h| !h.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i + 1));
                let clipped = cell.chars().take(500).collect::<String>();
                parts.push(format!("{key}: {clipped}"));
            }
            if !parts.is_empty() {
                lines.push(format!(
                    "Sheet {sheet} | Row {} | {}",
                    row_idx + 1,
                    parts.join(" | ")
                ));
            }
        }
    }

    if lines.is_empty() {
        return Err("No tabular XLSX content found".to_string());
    }

    let text = normalize_text(lines.join("\n"));
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("No XLSX content found".to_string());
    }
    Ok(pages)
}

fn parse_image_ocr(path: &str, model_dir: &std::path::Path) -> Result<Vec<DocumentPage>, String> {
    enforce_file_security(path)?;

    let ocr_dir = model_dir.join("ocr");
    let detection_path = ocr_dir.join("text-detection.rten");
    let recognition_path = ocr_dir.join("text-recognition.rten");

    if !detection_path.exists() || !recognition_path.exists() {
        return Err(
            "OCR models not found. Please run the initial setup to download them.".to_string(),
        );
    }

    let detection_model = rten::Model::load_file(&detection_path)
        .map_err(|e| format!("Failed to load OCR detection model: {e}"))?;
    let recognition_model = rten::Model::load_file(&recognition_path)
        .map_err(|e| format!("Failed to load OCR recognition model: {e}"))?;

    let engine = ocrs::OcrEngine::new(ocrs::OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .map_err(|e| format!("Failed to create OCR engine: {e}"))?;

    let img = image::open(path)
        .map_err(|e| format!("Failed to open image: {e}"))?;
    let rgb = img.into_rgb8();
    let dims = rgb.dimensions();

    let img_source = ocrs::ImageSource::from_bytes(rgb.as_raw(), dims)
        .map_err(|e| format!("Failed to create OCR input: {e}"))?;

    let ocr_input = engine
        .prepare_input(img_source)
        .map_err(|e| format!("Failed to prepare OCR input: {e}"))?;

    let raw_text = engine
        .get_text(&ocr_input)
        .map_err(|e| format!("OCR processing failed: {e}"))?;

    let text = normalize_text(raw_text);
    let pages = paginate_text(&text);
    if pages.is_empty() {
        return Err("OCR produced no text".to_string());
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_real_pdf() {
        // Uses BIWS-Restructuring-1.pdf from the Desktop — a real financial/legal PDF.
        let path = "/Users/liamneild/Desktop/BIWS-Restructuring-1.pdf";
        if !std::path::Path::new(path).exists() {
            eprintln!("Test PDF not found at {path}, skipping.");
            return;
        }
        let pages = parse_pdf(path).expect("parse_pdf should not error");
        assert!(!pages.is_empty(), "should produce at least one page");

        let total_chars: usize = pages.iter().map(|p| p.text.len()).sum();
        eprintln!("Pages: {}, total chars: {}", pages.len(), total_chars);

        for page in pages.iter().take(3) {
            eprintln!(
                "\n--- PAGE {} ({} chars) ---",
                page.page_number,
                page.text.len()
            );
            eprintln!("{}", &page.text[..page.text.len().min(500)]);

            // No private-use-area characters should survive
            for ch in page.text.chars() {
                let code = ch as u32;
                assert!(
                    !(0xE000..=0xF8FF).contains(&code),
                    "PUA character U+{:04X} found on page {} — sanitization failed",
                    code,
                    page.page_number
                );
            }
            // No control characters except newline/tab
            for ch in page.text.chars() {
                if ch == '\n' || ch == '\t' {
                    continue;
                }
                assert!(
                    !ch.is_control(),
                    "Control char U+{:04X} found on page {} — sanitization failed",
                    ch as u32,
                    page.page_number
                );
            }
            // "?Identity-H Unimplemented?" must be gone
            assert!(
                !page.text.contains("?Identity-H Unimplemented?"),
                "lopdf Identity-H placeholder found on page {} — not stripped",
                page.page_number
            );
        }
        eprintln!("\nAll assertions passed.");
    }

    #[test]
    fn test_clean_pdf_text_strips_pua() {
        let raw = "Salary: \u{E001}\u{E002}\u{E003} $85,000 ?Identity-H Unimplemented? per year";
        let cleaned = clean_pdf_text(raw);
        assert!(!cleaned.contains("?Identity-H Unimplemented?"));
        assert!(cleaned.contains("$85,000"));
        for ch in cleaned.chars() {
            let code = ch as u32;
            assert!(
                !(0xE000..=0xF8FF).contains(&code),
                "PUA char U+{:04X} not stripped",
                code
            );
        }
    }

    #[test]
    fn test_ligature_normalization() {
        let raw = "The \u{FB01}rst party shall \u{FB02}y to the meeting. \u{FB03}nal settlement.";
        let cleaned = clean_pdf_text(raw);
        assert!(cleaned.contains("first"), "ﬁ ligature not expanded");
        assert!(cleaned.contains("fly"), "ﬂ ligature not expanded");
        assert!(cleaned.contains("ffinal"), "ﬃ ligature not expanded");
        assert!(!cleaned.contains('\u{FB01}'));
        assert!(!cleaned.contains('\u{FB02}'));
        assert!(!cleaned.contains('\u{FB03}'));
    }

    #[test]
    fn test_smart_quote_normalization() {
        let raw = "\u{201C}Party A\u{201D} agrees and \u{2018}Party B\u{2019} consents.";
        let cleaned = clean_pdf_text(raw);
        assert!(cleaned.contains("\"Party A\""));
        assert!(cleaned.contains("'Party B'"));
    }

    #[test]
    fn test_hyphen_line_break_removal() {
        let raw = "The agree-\nment shall terminate upon written notice.";
        let cleaned = clean_pdf_text(raw);
        assert!(
            cleaned.contains("agreement"),
            "hyphen line break not joined: got {cleaned:?}"
        );
        assert!(!cleaned.contains("-\n"));
    }

    #[test]
    fn test_hyphen_break_not_removed_for_list_items() {
        // A real hyphen at end of line NOT followed by a letter should stay
        let raw = "Items:\n- First item\n- Second item";
        let cleaned = clean_pdf_text(raw);
        assert!(
            cleaned.contains("- First"),
            "list hyphen incorrectly removed"
        );
    }

    #[test]
    fn test_collapse_blank_lines() {
        let raw = "Section 1\n\n\n\n\nSection 2";
        let cleaned = clean_pdf_text(raw);
        // Should have at most 2 consecutive newlines
        assert!(
            !cleaned.contains("\n\n\n"),
            "3+ consecutive newlines not collapsed: got {cleaned:?}"
        );
        assert!(cleaned.contains("Section 1"));
        assert!(cleaned.contains("Section 2"));
    }

    #[test]
    fn test_nonbreaking_space_normalized() {
        let raw = "Party\u{00A0}A agrees.";
        let cleaned = clean_pdf_text(raw);
        assert!(
            cleaned.contains("Party A"),
            "non-breaking space not normalized"
        );
    }

    #[test]
    fn test_soft_hyphen_removed() {
        let raw = "agree\u{00AD}ment";
        let cleaned = clean_pdf_text(raw);
        assert_eq!(
            cleaned, "agreement",
            "soft hyphen not removed: got {cleaned:?}"
        );
    }

    #[test]
    fn test_en_dash_normalized() {
        let raw = "pages 1\u{2013}5 of the agreement";
        let cleaned = clean_pdf_text(raw);
        assert!(
            cleaned.contains("pages 1-5"),
            "en dash not normalized: got {cleaned:?}"
        );
    }

    #[test]
    fn test_parse_plain_text() {
        let path =
            std::env::temp_dir().join(format!("justice_ai_test_{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, "Line one\nLine two\n\nLine three").expect("write temp txt");
        let pages = parse_plain_text(path.to_string_lossy().as_ref()).expect("parse txt");
        assert!(!pages.is_empty());
        assert!(pages[0].text.contains("Line one"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_parse_csv() {
        let path =
            std::env::temp_dir().join(format!("justice_ai_test_{}.csv", uuid::Uuid::new_v4()));
        std::fs::write(&path, "name,amount\nAlice,100\nBob,200\n").expect("write temp csv");
        let pages = parse_csv(path.to_string_lossy().as_ref()).expect("parse csv");
        assert!(!pages.is_empty());
        assert!(pages[0].text.contains("name: Alice"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_xml_rejects_doctype() {
        let path =
            std::env::temp_dir().join(format!("justice_ai_test_{}.xml", uuid::Uuid::new_v4()));
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE foo [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]>
<root><name>&xxe;</name></root>"#;
        std::fs::write(&path, xml).expect("write temp xml");
        let err =
            parse_xml(path.to_string_lossy().as_ref()).expect_err("doctype should be rejected");
        assert!(err.to_lowercase().contains("doctype") || err.to_lowercase().contains("entity"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_parse_html() {
        let path =
            std::env::temp_dir().join(format!("justice_ai_test_{}.html", uuid::Uuid::new_v4()));
        let html = r#"<html><body><nav>menu</nav><h1>Case Summary</h1><p>Material fact.</p></body></html>"#;
        std::fs::write(&path, html).expect("write temp html");
        let pages = parse_html(path.to_string_lossy().as_ref()).expect("parse html");
        assert!(!pages.is_empty());
        assert!(pages[0].text.contains("Case Summary"));
        let _ = std::fs::remove_file(path);
    }
}
