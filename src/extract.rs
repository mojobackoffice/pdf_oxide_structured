use anyhow::{Context, Result};
use pdf_oxide::PdfDocument;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputData {
    pub file: String,
    pub page_count: usize,
    pub pages: Vec<PageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageData {
    pub page_number: usize,
    pub spans: Vec<serde_json::Value>,
}

/// Extracts structured text spans from an already-open document.
///
/// `page` is a 0-based index. When it is omitted, every page is extracted.
/// OCR runs for scanned pages when `use_ocr` is true and models can be found.
/// Does not open or close `doc`.
pub fn extract(doc: &mut PdfDocument, page: Option<usize>, use_ocr: bool) -> Result<OutputData> {
    let page_count = doc.page_count().context("Failed to get page count")?;

    let pages_to_extract: Vec<usize> = if let Some(page) = page {
        if page >= page_count {
            anyhow::bail!(
                "Page index {} is out of bounds. Document has {} pages (0-indexed: 0-{})",
                page,
                page_count,
                page_count.saturating_sub(1)
            );
        }
        vec![page]
    } else {
        (0..page_count).collect()
    };

    let mut pages = Vec::new();
    for page_num in pages_to_extract {
        let spans = extract_page_spans(doc, page_num, use_ocr)
            .with_context(|| format!("Failed to extract spans from page {}", page_num))?;

        pages.push(PageData {
            page_number: page_num,
            spans,
        });
    }

    Ok(OutputData {
        file: String::new(),
        page_count,
        pages,
    })
}

fn extract_page_spans(
    doc: &mut PdfDocument,
    page_index: usize,
    use_ocr: bool,
) -> Result<Vec<serde_json::Value>> {
    let spans = match doc.extract_spans(page_index) {
        Ok(spans) => {
            if !spans.is_empty() && spans.iter().any(|s| !s.text.trim().is_empty()) {
                spans
            } else if use_ocr {
                attempt_ocr_extraction(doc, page_index)?
            } else {
                spans
            }
        }
        Err(_) if use_ocr => attempt_ocr_extraction(doc, page_index)?,
        Err(e) => return Err(e.into()),
    };

    spans
        .into_iter()
        .map(|span| serde_json::to_value(&span))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Locates OCR models, preferring `PDF_OXIDE_MODEL_DIR`, then the CLI fallbacks.
fn model_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Ok(dir) = std::env::var("PDF_OXIDE_MODEL_DIR") {
        locations.push(PathBuf::from(dir));
    }
    locations.push(PathBuf::from("models"));
    if let Some(home) = dirs::home_dir() {
        locations.push(home.join(".pdf-extract/models"));
    }
    locations.push(PathBuf::from("/usr/local/share/pdf-extract/models"));
    locations
}

fn attempt_ocr_extraction(
    doc: &mut PdfDocument,
    page_index: usize,
) -> Result<Vec<pdf_oxide::layout::TextSpan>> {
    use pdf_oxide::ocr::{needs_ocr, ocr_page_spans, OcrConfig, OcrEngine, OcrExtractOptions};

    if !needs_ocr(doc, page_index)? {
        return doc.extract_spans(page_index).map_err(|e| e.into());
    }

    let mut det_model = None;
    let mut rec_model = None;
    let mut dict_file = None;

    for location in model_locations() {
        let det = location.join("det.onnx");
        let rec = location.join("rec.onnx");
        let dict_candidates = [
            location.join("dict.txt"),
            location.join("en_dict.txt"),
            location.join("cn_dict.txt"),
            location.join("ch_dict.txt"),
        ];
        let dict = dict_candidates.into_iter().find(|p| p.exists());

        if det.exists() && rec.exists() && dict.is_some() {
            det_model = Some(det);
            rec_model = Some(rec);
            dict_file = dict;
            break;
        }
    }

    if let (Some(det), Some(rec), Some(dict)) = (det_model, rec_model, dict_file) {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(
            det.to_str().unwrap(),
            rec.to_str().unwrap(),
            dict.to_str().unwrap(),
            config,
        )?;

        let options = OcrExtractOptions::default();
        match ocr_page_spans(doc, page_index, &engine, &options) {
            Ok(spans) => Ok(spans),
            Err(e) => {
                eprintln!("Warning: OCR extraction failed: {}", e);
                eprintln!("  Falling back to native text extraction");
                doc.extract_spans(page_index).map_err(|e| e.into())
            }
        }
    } else {
        eprintln!(
            "Warning: OCR models not found. Falling back to native text extraction for page {}",
            page_index
        );
        eprintln!("  Place OCR models (det.onnx, rec.onnx, dict.txt or en_dict.txt) in one of:");
        if let Ok(env_dir) = std::env::var("PDF_OXIDE_MODEL_DIR") {
            eprintln!("    - {} (from PDF_OXIDE_MODEL_DIR)", env_dir);
        }
        eprintln!("    - ./models/");
        if let Some(home) = dirs::home_dir() {
            eprintln!("    - {}", home.join(".pdf-extract/models").display());
        }
        eprintln!("    - /usr/local/share/pdf-extract/models/");

        doc.extract_spans(page_index).map_err(|e| e.into())
    }
}
