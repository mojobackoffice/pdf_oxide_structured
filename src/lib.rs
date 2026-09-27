mod extract;

use extract::{extract, Granularity};
use magnus::{function, Error, Object, RString, Ruby};
use pdf_oxide::PdfDocument;
use std::path::Path;

/// Returns the CLI JSON envelope for a PDF at `path`.
///
/// `page` is a 0-based index. `nil` extracts every page.
fn extract_path_json(ruby: &Ruby, path: String, page: Option<i64>) -> Result<String, Error> {
    extract_path(ruby, path, page, Granularity::Spans)
}

/// Returns the CLI JSON envelope of words for a PDF at `path`.
///
/// `page` is a 0-based index. `nil` extracts every page.
fn extract_words_path_json(ruby: &Ruby, path: String, page: Option<i64>) -> Result<String, Error> {
    extract_path(ruby, path, page, Granularity::Words)
}

/// Opens `path` and serializes spans or words.
fn extract_path(
    ruby: &Ruby,
    path: String,
    page: Option<i64>,
    granularity: Granularity,
) -> Result<String, Error> {
    let page = page_index(ruby, page)?;
    let mut doc = PdfDocument::open(Path::new(&path))
        .map_err(|error| Error::new(ruby.exception_runtime_error(), error.to_string()))?;
    extract_document(ruby, &mut doc, page, granularity)
}

/// Returns the CLI JSON envelope for PDF `bytes`.
///
/// `page` is a 0-based index. `nil` extracts every page.
fn extract_bytes_json(ruby: &Ruby, bytes: RString, page: Option<i64>) -> Result<String, Error> {
    extract_bytes(ruby, bytes, page, Granularity::Spans)
}

/// Returns the CLI JSON envelope of words for PDF `bytes`.
///
/// `page` is a 0-based index. `nil` extracts every page.
fn extract_words_bytes_json(
    ruby: &Ruby,
    bytes: RString,
    page: Option<i64>,
) -> Result<String, Error> {
    extract_bytes(ruby, bytes, page, Granularity::Words)
}

/// Opens `bytes` and serializes spans or words.
fn extract_bytes(
    ruby: &Ruby,
    bytes: RString,
    page: Option<i64>,
    granularity: Granularity,
) -> Result<String, Error> {
    let page = page_index(ruby, page)?;
    // Copied immediately so Ruby cannot move the string while the slice is held.
    let bytes = unsafe { bytes.as_slice() }.to_vec();
    let mut doc = PdfDocument::from_bytes(bytes)
        .map_err(|error| Error::new(ruby.exception_runtime_error(), error.to_string()))?;
    extract_document(ruby, &mut doc, page, granularity)
}

/// Converts a Ruby page index into a 0-based page, or every page when omitted.
fn page_index(ruby: &Ruby, page: Option<i64>) -> Result<Option<usize>, Error> {
    match page {
        None => Ok(None),
        Some(index) if index >= 0 => Ok(Some(index as usize)),
        Some(index) => Err(Error::new(
            ruby.exception_arg_error(),
            format!("page index {index} is out of bounds"),
        )),
    }
}

/// Runs span or word extraction and serializes the CLI envelope.
fn extract_document(
    ruby: &Ruby,
    doc: &mut PdfDocument,
    page: Option<usize>,
    granularity: Granularity,
) -> Result<String, Error> {
    let output = extract(doc, page, true, granularity)
        .map_err(|error| Error::new(ruby.exception_runtime_error(), error.to_string()))?;

    serde_json::to_string(&output)
        .map_err(|error| Error::new(ruby.exception_runtime_error(), error.to_string()))
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("PdfOxideStructured")?;
    module.define_singleton_method("extract_path_json", function!(extract_path_json, 2))?;
    module.define_singleton_method("extract_bytes_json", function!(extract_bytes_json, 2))?;
    module.define_singleton_method(
        "extract_words_path_json",
        function!(extract_words_path_json, 2),
    )?;
    module.define_singleton_method(
        "extract_words_bytes_json",
        function!(extract_words_bytes_json, 2),
    )?;
    Ok(())
}
