use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Source-grounded entity extraction with strict character interval offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extraction {
    pub extraction_class: String,
    pub extraction_text: String,
    pub char_interval: Option<(usize, usize)>,
    pub attributes: BTreeMap<String, String>,
    pub source_chunk_id: Option<String>,
    pub source_file_path: Option<String>,
}

impl Extraction {
    pub fn new(
        extraction_class: impl Into<String>,
        extraction_text: impl Into<String>,
    ) -> Self {
        Self {
            extraction_class: extraction_class.into(),
            extraction_text: extraction_text.into(),
            char_interval: None,
            attributes: BTreeMap::new(),
            source_chunk_id: None,
            source_file_path: None,
        }
    }

    pub fn with_source(
        mut self,
        chunk_id: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        self.source_chunk_id = Some(chunk_id.into());
        self.source_file_path = Some(file_path.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn with_interval(mut self, start: usize, end: usize) -> Self {
        self.char_interval = Some((start, end));
        self
    }
}

/// Verbatim text grounding function:
/// Searches `source_text` for verbatim substring match of `extraction.extraction_text`.
/// If found at byte index `start`, sets `extraction.char_interval = Some((start, start + len))` and returns true.
/// If not found via exact match, attempts case-insensitive fallback:
/// if found and boundaries align, sets `char_interval` and returns true.
/// Otherwise, sets `extraction.char_interval = None` and returns false.
pub fn ground_extraction(extraction: &mut Extraction, source_text: &str) -> bool {
    if extraction.extraction_text.is_empty() {
        extraction.char_interval = None;
        return false;
    }

    // 1. Verbatim exact byte match
    if let Some(start) = source_text.find(&extraction.extraction_text) {
        let end = start + extraction.extraction_text.len();
        extraction.char_interval = Some((start, end));
        return true;
    }

    // 2. Case-insensitive substring fallback
    let source_lower = source_text.to_lowercase();
    let target_lower = extraction.extraction_text.to_lowercase();
    if let Some(start) = source_lower.find(&target_lower) {
        let end = start + target_lower.len();
        if source_text.is_char_boundary(start) && source_text.is_char_boundary(end) {
            extraction.char_interval = Some((start, end));
            return true;
        }
    }

    extraction.char_interval = None;
    false
}

/// Hallucination filter: strictly retains only items where `extraction.char_interval.is_some()`.
pub fn filter_grounded(extractions: Vec<Extraction>) -> Vec<Extraction> {
    extractions
        .into_iter()
        .filter(|e| e.char_interval.is_some())
        .collect()
}
