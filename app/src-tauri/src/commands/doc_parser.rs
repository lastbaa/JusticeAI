use crate::state::DocumentPage;
use std::io::Read;

/// Parse a PDF file into pages of text using lopdf
pub fn parse_pdf(path: &str) -> Result<Vec<DocumentPage>, String> {
    let doc = lopdf::Document::load(path).map_err(|e| format!("PDF load error: {e}"))?;
    let page_map = doc.get_pages();

    // Sort pages by page number
    let mut page_nums: Vec<u32> = page_map.keys().cloned().collect();
    page_nums.sort();

    let mut pages = Vec::new();
    for page_num in page_nums {
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        let cleaned = clean_pdf_text(&text);
        pages.push(DocumentPage {
            page_number: page_num,
            text: cleaned,
        });
    }

    Ok(pages)
}

fn clean_pdf_text(text: &str) -> String {
    // Replace multiple whitespace/newlines with single space, trim
    let mut result = String::with_capacity(text.len());
    let mut last_space = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_space {
                result.push(' ');
                last_space = true;
            }
        } else {
            result.push(ch);
            last_space = false;
        }
    }
    result.trim().to_string()
}

/// Parse a DOCX file into pages of text
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

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| format!("XML parse error: {e}"))?;

    let ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let mut pages: Vec<String> = vec![String::new()];
    let mut in_paragraph = false;

    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        if node.tag_name().namespace() != Some(ns) {
            continue;
        }
        match node.tag_name().name() {
            "p" => {
                // Paragraph start — we handle text at the run level
                // Add newline at paragraph end if current page has content
                if !in_paragraph {
                    in_paragraph = true;
                }
                // Add newline after each paragraph when traversal ends
                // We detect paragraph end by checking if parent's next sibling is a new p
                // Simpler: just add newline when we see a new p element at body level
                if let Some(last) = pages.last_mut() {
                    if !last.is_empty() && !last.ends_with('\n') {
                        last.push('\n');
                    }
                }
            }
            "t" => {
                // Text run
                if let Some(text) = node.text() {
                    if let Some(last) = pages.last_mut() {
                        last.push_str(text);
                    }
                }
            }
            "br" => {
                // Check for page break
                let is_page_break = node
                    .attributes()
                    .any(|a| a.name() == "type" && a.value() == "page");
                if is_page_break {
                    pages.push(String::new());
                } else {
                    // Line break
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
