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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spans: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub words: Option<Vec<serde_json::Value>>,
}

/// Whether a page is filled with text spans or with words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Spans,
    Words,
}

/// Extracts structured text spans or words from an already-open document.
///
/// `page` is a 0-based index. When it is omitted, every page is extracted.
/// OCR runs for scanned pages when `use_ocr` is true and models can be found.
/// Does not open or close `doc`.
pub fn extract(
    doc: &mut PdfDocument,
    page: Option<usize>,
    use_ocr: bool,
    granularity: Granularity,
) -> Result<OutputData> {
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
        let page_data =
            match granularity {
                Granularity::Spans => PageData {
                    page_number: page_num,
                    spans: Some(extract_page_spans(doc, page_num, use_ocr).with_context(|| {
                        format!("Failed to extract spans from page {}", page_num)
                    })?),
                    words: None,
                },
                Granularity::Words => PageData {
                    page_number: page_num,
                    spans: None,
                    words: Some(extract_page_words(doc, page_num, use_ocr).with_context(|| {
                        format!("Failed to extract words from page {}", page_num)
                    })?),
                },
            };

        pages.push(page_data);
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

fn extract_page_words(
    doc: &mut PdfDocument,
    page_index: usize,
    use_ocr: bool,
) -> Result<Vec<serde_json::Value>> {
    let words = match doc.extract_words(page_index) {
        Ok(words) => {
            if words_have_text(&words) {
                words
            } else if use_ocr {
                attempt_ocr_words(doc, page_index)?
            } else {
                words
            }
        }
        Err(_) if use_ocr => attempt_ocr_words(doc, page_index)?,
        Err(e) => return Err(e.into()),
    };

    words
        .into_iter()
        .map(|word| serde_json::to_value(&word))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn words_have_text(words: &[pdf_oxide::layout::Word]) -> bool {
    !words.is_empty() && words.iter().any(|word| !word.text.trim().is_empty())
}

/// Groups OCR span glyphs into words on non-whitespace runs.
fn spans_to_words(spans: Vec<pdf_oxide::layout::TextSpan>) -> Vec<pdf_oxide::layout::Word> {
    let mut words = Vec::new();
    for span in spans {
        let mut run = Vec::new();
        for ch in span.to_chars() {
            if ch.char.is_whitespace() {
                push_word(&mut words, &mut run);
            } else {
                run.push(ch);
            }
        }
        push_word(&mut words, &mut run);
    }
    words
}

fn push_word(words: &mut Vec<pdf_oxide::layout::Word>, run: &mut Vec<pdf_oxide::layout::TextChar>) {
    if !run.is_empty() {
        words.push(pdf_oxide::layout::TextBlock::from_chars(std::mem::take(
            run,
        )));
    }
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

/// Result of trying OCR before the caller falls back to native extraction.
enum OcrOutcome {
    NotNeeded,
    Spans(Vec<pdf_oxide::layout::TextSpan>),
    Unavailable,
}

fn attempt_ocr_extraction(
    doc: &mut PdfDocument,
    page_index: usize,
) -> Result<Vec<pdf_oxide::layout::TextSpan>> {
    match run_ocr(doc, page_index)? {
        OcrOutcome::Spans(spans) => Ok(spans),
        OcrOutcome::NotNeeded | OcrOutcome::Unavailable => {
            doc.extract_spans(page_index).map_err(|e| e.into())
        }
    }
}

fn attempt_ocr_words(
    doc: &mut PdfDocument,
    page_index: usize,
) -> Result<Vec<pdf_oxide::layout::Word>> {
    match run_ocr(doc, page_index)? {
        OcrOutcome::Spans(spans) => Ok(spans_to_words(spans)),
        OcrOutcome::NotNeeded | OcrOutcome::Unavailable => {
            doc.extract_words(page_index).map_err(|e| e.into())
        }
    }
}

fn run_ocr(doc: &mut PdfDocument, page_index: usize) -> Result<OcrOutcome> {
    use pdf_oxide::ocr::{needs_ocr, ocr_page_spans, OcrConfig, OcrEngine, OcrExtractOptions};

    if !needs_ocr(doc, page_index)? {
        return Ok(OcrOutcome::NotNeeded);
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
            Ok(spans) => Ok(OcrOutcome::Spans(spans)),
            Err(e) => {
                eprintln!("Warning: OCR extraction failed: {}", e);
                eprintln!("  Falling back to native text extraction");
                Ok(OcrOutcome::Unavailable)
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

        Ok(OcrOutcome::Unavailable)
    }
}
