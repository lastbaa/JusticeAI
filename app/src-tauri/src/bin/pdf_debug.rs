//! Temporary debug binary: compare PDF extraction engines on filled_form_simple.pdf
use app_lib::commands::doc_parser;

fn main() {
    let pdf_path = "tests/fixtures/filled_form_simple.pdf";
    let abs_path = std::fs::canonicalize(pdf_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(pdf_path));
    let path_str = abs_path.to_str().unwrap();

    println!("\n{}", "=".repeat(70));
    println!(" PDF DEBUG: {}", path_str);
    println!("{}", "=".repeat(70));

    // 1. Full parse_pdf (combined pipeline)
    println!("\n>>> ENGINE: parse_pdf (full pipeline) <<<");
    match doc_parser::parse_pdf(path_str) {
        Ok(pages) => {
            for p in &pages {
                println!("--- Page {} ---", p.page_number);
                println!("{}", p.text);
            }
            check_keywords("parse_pdf", &pages);
        }
        Err(e) => println!("ERROR: {e}"),
    }

    // 2. pdf_oxide directly
    println!("\n>>> ENGINE: pdf_oxide (raw, no AcroForm) <<<");
    match pdf_oxide_raw(path_str) {
        Some(pages) => {
            for p in &pages {
                println!("--- Page {} ---", p.page_number);
                println!("{}", p.text);
            }
            check_keywords("pdf_oxide", &pages);
        }
        None => println!("pdf_oxide returned None (failed or empty)"),
    }

    // 3. pdf-extract directly
    println!("\n>>> ENGINE: pdf-extract (raw, no AcroForm) <<<");
    match pdf_extract::extract_text(path_str) {
        Ok(raw) => {
            println!("{}", raw);
            check_keywords_str("pdf-extract", &raw);
        }
        Err(e) => println!("ERROR: {e}"),
    }

    // 4. lopdf directly
    println!("\n>>> ENGINE: lopdf extract_text (raw, no AcroForm) <<<");
    match lopdf::Document::load(path_str) {
        Ok(doc) => {
            let page_map = doc.get_pages();
            let mut page_nums: Vec<u32> = page_map.keys().cloned().collect();
            page_nums.sort();
            let mut all_text = String::new();
            for pn in &page_nums {
                let text = doc.extract_text(&[*pn]).unwrap_or_default();
                println!("--- Page {} ---", pn);
                println!("{}", text);
                all_text.push_str(&text);
            }
            check_keywords_str("lopdf", &all_text);

            // 4b. lopdf AcroForm fields
            println!("\n>>> ENGINE: lopdf AcroForm fields <<<");
            dump_acroform(&doc);
        }
        Err(e) => println!("ERROR: {e}"),
    }
}

fn pdf_oxide_raw(path: &str) -> Option<Vec<app_lib::state::DocumentPage>> {
    let mut doc = pdf_oxide::PdfDocument::open(path).ok()?;
    let page_count = doc.page_count().ok()?;
    let mut pages = Vec::new();
    for i in 0..page_count {
        let text = doc.extract_text(i).unwrap_or_default();
        pages.push(app_lib::state::DocumentPage {
            page_number: (i + 1) as u32,
            text,
        });
    }
    Some(pages)
}

fn check_keywords(engine: &str, pages: &[app_lib::state::DocumentPage]) {
    let combined: String = pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    check_keywords_str(engine, &combined);
}

fn check_keywords_str(engine: &str, text: &str) {
    let keywords = ["Maria Garcia", "8,500", "8500", "200", "$8,500", "$200"];
    println!("\n  [{}] Keyword check:", engine);
    for kw in &keywords {
        let found = text.contains(kw);
        println!("    {:>12} => {}", kw, if found { "FOUND" } else { "MISSING" });
    }
}

fn dump_acroform(doc: &lopdf::Document) {
    use lopdf::Object;
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => { println!("  No catalog"); return; }
    };
    let acroform = match catalog.get(b"AcroForm") {
        Ok(obj) => match obj {
            Object::Reference(r) => doc.get_object(*r).ok(),
            Object::Dictionary(_) => Some(obj),
            _ => None,
        },
        Err(_) => { println!("  No AcroForm in catalog"); return; }
    };
    let acroform = match acroform {
        Some(Object::Dictionary(d)) => d,
        _ => { println!("  AcroForm is not a dict"); return; }
    };
    let fields = match acroform.get(b"Fields") {
        Ok(Object::Array(arr)) => arr,
        Ok(Object::Reference(r)) => {
            match doc.get_object(*r) {
                Ok(Object::Array(arr)) => arr,
                _ => { println!("  Fields ref not array"); return; }
            }
        }
        _ => { println!("  No Fields array"); return; }
    };
    println!("  Found {} top-level fields:", fields.len());
    for (i, field_obj) in fields.iter().enumerate() {
        let field_ref = match field_obj {
            Object::Reference(r) => r,
            _ => continue,
        };
        if let Ok(Object::Dictionary(fd)) = doc.get_object(*field_ref) {
            let name = fd.get(b"T")
                .ok()
                .and_then(|o| match o {
                    Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
                    _ => None,
                })
                .unwrap_or_else(|| format!("(unnamed-{})", i));
            let value = fd.get(b"V")
                .map(|o| format!("{:?}", o))
                .unwrap_or_else(|_e| "(no /V)".to_string());
            println!("  Field {:>2}: T={:30} V={}", i, name, value);
        }
    }
}
