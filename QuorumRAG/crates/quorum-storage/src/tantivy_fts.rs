//! Co-located Tantivy BM25 full-text search index.
//!
//! # Invariant: Absolute Arrow Isolation
//! All types and internal Tantivy dependencies are strictly `pub(crate)`.

use std::fs::create_dir_all;
use std::ops::Bound;
use std::path::Path;
use std::sync::Mutex;

use quorum_types::{DocumentChunk, ScoredChunk};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument, Term};

use crate::error::StorageError;

/// Tantivy inverted index providing BM25 keyword search with access-tier filtering.
pub(crate) struct TantivyFtsIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter<TantivyDocument>>,
    field_chunk_id: Field,
    field_document_id: Field,
    field_file_path: Field,
    field_file_path_text: Field,
    field_line_start: Field,
    field_line_end: Field,
    field_blake3_hash: Field,
    field_access_tier: Field,
    field_text_content: Field,
    field_metadata_json: Field,
    field_vector_bytes: Field,
}

impl TantivyFtsIndex {
    /// Opens or creates the Tantivy index at `path`.
    pub(crate) fn open(path: &Path) -> Result<Self, StorageError> {
        create_dir_all(path).map_err(|e| StorageError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut builder = Schema::builder();
        let field_chunk_id = builder.add_text_field("chunk_id", STRING | STORED);
        let field_document_id = builder.add_text_field("document_id", STRING | STORED);
        let field_file_path = builder.add_text_field("file_path", STRING | STORED);
        let field_file_path_text = builder.add_text_field("file_path_text", TEXT);
        let field_line_start = builder.add_u64_field("line_start", FAST | STORED);
        let field_line_end = builder.add_u64_field("line_end", FAST | STORED);
        let field_blake3_hash = builder.add_text_field("blake3_hash", STRING | STORED);
        let field_access_tier = builder.add_u64_field("access_tier", FAST | STORED | INDEXED);
        let field_text_content = builder.add_text_field("text_content", TEXT | STORED);
        let field_metadata_json = builder.add_text_field("metadata_json", STRING | STORED);
        let field_vector_bytes = builder.add_bytes_field("vector_bytes", STORED);
        let schema = builder.build();

        let index = Index::open_in_dir(path)
            .or_else(|_| Index::create_in_dir(path, schema))
            .map_err(|e| StorageError::Tantivy(e.to_string()))?;

        let writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .map_err(|e| StorageError::Tantivy(format!("Failed to create Tantivy writer: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .map_err(|e| StorageError::Tantivy(format!("Failed to create Tantivy reader: {}", e)))?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            field_chunk_id,
            field_document_id,
            field_file_path,
            field_file_path_text,
            field_line_start,
            field_line_end,
            field_blake3_hash,
            field_access_tier,
            field_text_content,
            field_metadata_json,
            field_vector_bytes,
        })
    }

    /// Indexes a slice of chunks into the Tantivy BM25 inverted index.
    pub(crate) fn index_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy index writer mutex poisoned".into()))?;

        let mut count = 0;
        for c in chunks {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.field_chunk_id, &c.chunk_id);
            doc.add_text(self.field_document_id, &c.document_id);
            doc.add_text(self.field_file_path, &c.file_path);
            doc.add_text(self.field_file_path_text, &c.file_path);
            doc.add_u64(self.field_line_start, c.line_start as u64);
            doc.add_u64(self.field_line_end, c.line_end as u64);
            doc.add_text(self.field_blake3_hash, &c.blake3_hash);
            doc.add_u64(self.field_access_tier, c.access_tier as u64);
            doc.add_text(self.field_text_content, &c.text_content);
            doc.add_text(self.field_metadata_json, &c.metadata_json);

            let mut vec_bytes = Vec::with_capacity(c.vector.len() * 4);
            for &f in &c.vector {
                vec_bytes.extend_from_slice(&f.to_le_bytes());
            }
            doc.add_bytes(self.field_vector_bytes, &vec_bytes);

            writer
                .add_document(doc)
                .map_err(|e| StorageError::Tantivy(format!("Add document error: {}", e)))?;
            count += 1;
        }

        commit_with_retry(&mut writer)?;

        self.reader
            .reload()
            .map_err(|e| StorageError::Tantivy(format!("Tantivy reload error: {}", e)))?;

        Ok(count)
    }

    /// Executes BM25 full-text search with `access_tier <= max_tier` filtering.
    pub(crate) fn search(
        &self,
        query_text: &str,
        max_tier: u8,
        top_k: usize,
    ) -> Result<Vec<ScoredChunk>, StorageError> {
        let trimmed = query_text.trim();
        if trimmed.is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let mut searcher = self.reader.searcher();
        if searcher.num_docs() == 0 {
            let _ = self.reader.reload();
            searcher = self.reader.searcher();
        }
        if searcher.num_docs() == 0 {
            return Ok(Vec::new());
        }
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.field_text_content, self.field_file_path_text],
        );

        // Parse query with fallback if query has syntax chars
        let parsed_query = query_parser.parse_query(trimmed).or_else(|_| {
            let sanitized: String = trimmed
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                .collect();
            let terms: Vec<&str> = sanitized.split_whitespace().collect();
            if terms.is_empty() {
                query_parser.parse_query("*")
            } else {
                query_parser.parse_query(&terms.join(" "))
            }
        });

        let text_query = match parsed_query {
            Ok(q) => q,
            Err(_) => return Ok(Vec::new()),
        };

        let tier_range = RangeQuery::new(
            Bound::Included(Term::from_field_u64(self.field_access_tier, 0)),
            Bound::Included(Term::from_field_u64(self.field_access_tier, max_tier as u64)),
        );

        let combined = BooleanQuery::intersection(vec![text_query, Box::new(tier_range)]);
        let num_docs = searcher.num_docs() as usize;
        let effective_limit = top_k.min(num_docs.max(1));
        let top_docs = searcher
            .search(&combined, &TopDocs::with_limit(effective_limit).order_by_score())
            .map_err(|e| StorageError::Tantivy(format!("Search execution failed: {}", e)))?;

        let mut scored_chunks = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| StorageError::Tantivy(format!("Failed to retrieve document: {}", e)))?;

            let chunk = self.doc_to_chunk(&doc);
            scored_chunks.push(ScoredChunk::new(chunk, score, "local", false));
        }

        scored_chunks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
        });

        Ok(scored_chunks)
    }

    /// Checks if a chunk hash already exists in the Tantivy index.
    pub(crate) fn has_chunk_hash(&self, blake3_hash: &str) -> Result<bool, StorageError> {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.field_blake3_hash, blake3_hash);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let hits = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| StorageError::Tantivy(e.to_string()))?;
        Ok(!hits.is_empty())
    }

    /// Deletes all indexed documents matching the specified `file_path`.
    pub(crate) fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
        let term = Term::from_field_text(self.field_file_path, file_path);
        let query = TermQuery::new(term.clone(), IndexRecordOption::Basic);
        let searcher = self.reader.searcher();
        let count = searcher
            .search(&query, &Count)
            .map_err(|e| StorageError::Tantivy(format!("Count matching documents failed: {}", e)))?;

        if count == 0 {
            return Ok(0);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        let _ = writer.delete_term(term);
        commit_with_retry(&mut writer)?;

        self.reader
            .reload()
            .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;

        Ok(count)
    }

    /// Merges Tantivy index segments.
    pub(crate) fn compact(&self) -> Result<(), StorageError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        commit_with_retry(&mut writer)?;

        self.reader
            .reload()
            .map_err(|e| StorageError::Tantivy(format!("Reload error: {}", e)))?;

        Ok(())
    }

    /// Converts a retrieved TantivyDocument into a domain DocumentChunk.
    pub(crate) fn doc_to_chunk(&self, doc: &TantivyDocument) -> DocumentChunk {
        let chunk_id = doc
            .get_first(self.field_chunk_id)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let document_id = doc
            .get_first(self.field_document_id)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let file_path = doc
            .get_first(self.field_file_path)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let line_start = doc
            .get_first(self.field_line_start)
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        let line_end = doc
            .get_first(self.field_line_end)
            .and_then(|v| v.as_u64())
            .unwrap_or(line_start as u64) as u32;
        let blake3_hash = doc
            .get_first(self.field_blake3_hash)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let access_tier = doc
            .get_first(self.field_access_tier)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        let text_content = doc
            .get_first(self.field_text_content)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let metadata_json = doc
            .get_first(self.field_metadata_json)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let vector = if let Some(bytes) = doc.get_first(self.field_vector_bytes).and_then(|v| v.as_bytes()) {
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect()
        } else {
            Vec::new()
        };

        DocumentChunk {
            chunk_id,
            document_id,
            file_path,
            line_start,
            line_end,
            blake3_hash,
            access_tier,
            vector,
            text_content,
            metadata_json,
        }
    }

    /// Scans all stored chunks in Tantivy.
    pub(crate) fn scan_all_chunks(&self) -> Result<Vec<DocumentChunk>, StorageError> {
        let searcher = self.reader.searcher();
        let query = tantivy::query::AllQuery;
        let num_docs = searcher.num_docs() as usize;
        if num_docs == 0 {
            return Ok(Vec::new());
        }
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(num_docs.max(1)).order_by_score())
            .map_err(|e| StorageError::Tantivy(format!("Scan search failed: {}", e)))?;

        let mut chunks = Vec::with_capacity(top_docs.len());
        for (_score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| StorageError::Tantivy(format!("Failed to retrieve document: {}", e)))?;
            chunks.push(self.doc_to_chunk(&doc));
        }
        Ok(chunks)
    }

    /// Deletes a specific chunk by its chunk_id.
    pub(crate) fn delete_by_chunk_id(&self, chunk_id: &str) -> Result<usize, StorageError> {
        let term = Term::from_field_text(self.field_chunk_id, chunk_id);
        let query = TermQuery::new(term.clone(), IndexRecordOption::Basic);
        let searcher = self.reader.searcher();
        let count = searcher
            .search(&query, &Count)
            .map_err(|e| StorageError::Tantivy(format!("Count matching documents failed: {}", e)))?;

        if count == 0 {
            return Ok(0);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        let _ = writer.delete_term(term);
        commit_with_retry(&mut writer)?;

        self.reader
            .reload()
            .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;

        Ok(count)
    }
}

/// Helper that retries Tantivy commit operations when encountering transient Windows file locks.
fn commit_with_retry(writer: &mut IndexWriter<TantivyDocument>) -> Result<(), StorageError> {
    let mut attempts = 0;
    loop {
        match writer.commit() {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempts += 1;
                if attempts >= 10 {
                    return Err(StorageError::Tantivy(format!("Tantivy commit error: {}", e)));
                }
                std::thread::sleep(std::time::Duration::from_millis(15 * attempts));
            }
        }
    }
}
