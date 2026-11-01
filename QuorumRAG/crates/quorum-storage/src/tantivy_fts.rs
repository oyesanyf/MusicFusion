//! Co-located Tantivy BM25 full-text search index.
//!
//! # Invariant: Absolute Arrow Isolation
//! All types and internal Tantivy dependencies are strictly `pub(crate)`.

use std::collections::BTreeSet;
use std::fs::create_dir_all;
use std::ops::Bound;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use quorum_types::{DocumentChunk, ScoredChunk};
use tantivy::collector::{Count, TopDocs};
use tantivy::merge_policy::NoMergePolicy;
use tantivy::query::{BooleanQuery, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::*;
use tantivy::index::SegmentId;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument, Term};

use crate::error::StorageError;

/// Tantivy inverted index providing BM25 keyword search with access-tier filtering.
pub(crate) struct TantivyFtsIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter<TantivyDocument>>,
    last_commit_opstamp: AtomicU64,
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
    /// Helper to instantiate a new IndexWriter configured with NoMergePolicy.
    fn create_writer_inner(index: &Index) -> Result<IndexWriter<TantivyDocument>, StorageError> {
        let writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .map_err(|e| StorageError::Tantivy(format!("Failed to create Tantivy writer: {}", e)))?;
        writer.set_merge_policy(Box::new(NoMergePolicy));
        Ok(writer)
    }

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

        let writer = Self::create_writer_inner(&index)?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .map_err(|e| StorageError::Tantivy(format!("Failed to create Tantivy reader: {}", e)))?;

        let last_commit_opstamp = AtomicU64::new(
            index.load_metas().map(|m| m.opstamp).unwrap_or(0),
        );

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            last_commit_opstamp,
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

    /// Converts a domain DocumentChunk into a TantivyDocument.
    fn chunk_to_doc(&self, c: &DocumentChunk) -> TantivyDocument {
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
        doc
    }

    /// Indexes a slice of chunks into the Tantivy BM25 inverted index.
    ///
    /// Implements full batch-replay recovery: if the underlying Tantivy IndexWriter
    /// is killed or panics during `add_document` or `commit`, a fresh writer is
    /// instantiated and all chunks in the batch are replayed from chunk 0.
    pub(crate) fn index_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy index writer mutex poisoned".into()))?;

        let mut recovery_attempts = 0;
        const MAX_RECOVERIES: usize = 3;

        'batch_attempt: loop {
            let mut crash_occurred = false;
            for c in chunks {
                let doc = self.chunk_to_doc(c);
                if let Err(e) = writer.add_document(doc) {
                    let msg = e.to_string();
                    if msg.contains("index writer was killed") || msg.contains("panicked") {
                        tracing::warn!(
                            "Tantivy writer was killed during add_document; re-creating and replaying batch: {}",
                            msg
                        );
                        recovery_attempts += 1;
                        if recovery_attempts > MAX_RECOVERIES {
                            return Err(StorageError::Tantivy(format!(
                                "Exceeded maximum writer recoveries ({}) during add_document: {}",
                                MAX_RECOVERIES, msg
                            )));
                        }
                        *writer = Self::create_writer_inner(&self.index)?;
                        crash_occurred = true;
                        break;
                    } else {
                        return Err(StorageError::Tantivy(format!("Add document error: {}", e)));
                    }
                }
            }

            if crash_occurred {
                // Replay the full batch from index 0 on the fresh writer
                continue 'batch_attempt;
            }

            match commit_with_retry(&mut writer) {
                Ok(op) => {
                    self.last_commit_opstamp.store(op, Ordering::Release);
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("index writer was killed") || msg.contains("panicked") {
                        tracing::warn!(
                            "Tantivy writer killed during commit; re-creating and replaying batch: {}",
                            msg
                        );
                        recovery_attempts += 1;
                        if recovery_attempts > MAX_RECOVERIES {
                            return Err(StorageError::Tantivy(format!(
                                "Exceeded maximum writer recoveries ({}) during commit: {}",
                                MAX_RECOVERIES, msg
                            )));
                        }
                        *writer = Self::create_writer_inner(&self.index)?;
                        // Replay the full batch from index 0 on the fresh writer
                        continue 'batch_attempt;
                    } else {
                        return Err(e);
                    }
                }
            }

            break 'batch_attempt;
        }

        drop(writer);
        self.reload_with_retry()?;

        Ok(chunks.len())
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

        self.reload_with_retry()?;
        let searcher = self.reader.searcher();
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
        self.reload_with_retry()?;
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
        self.reload_with_retry()?;
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

        let _ = writer.delete_term(term.clone());
        match commit_with_retry(&mut writer) {
            Ok(op) => {
                self.last_commit_opstamp.store(op, Ordering::Release);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("index writer was killed") || msg.contains("panicked") {
                    tracing::warn!("Tantivy writer killed during delete; recreating: {}", msg);
                    *writer = Self::create_writer_inner(&self.index)?;
                    let _ = writer.delete_term(term);
                    let op = commit_with_retry(&mut writer)?;
                    self.last_commit_opstamp.store(op, Ordering::Release);
                } else {
                    return Err(e);
                }
            }
        }

        drop(writer);
        self.reload_with_retry()?;

        Ok(count)
    }

    /// Merges Tantivy index segments to reclaim deleted space and consolidate inverted indexes.
    pub(crate) fn compact(&self) -> Result<(), StorageError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        // 1. Commit any pending uncommitted additions/deletions before merging
        match commit_with_retry(&mut writer) {
            Ok(op) => {
                self.last_commit_opstamp.store(op, Ordering::Release);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("index writer was killed") || msg.contains("panicked") {
                    tracing::warn!("Tantivy writer killed during compact pre-commit; recreating: {}", msg);
                    *writer = Self::create_writer_inner(&self.index)?;
                    let op = commit_with_retry(&mut writer)?;
                    self.last_commit_opstamp.store(op, Ordering::Release);
                } else {
                    return Err(e);
                }
            }
        }

        // 2. Reload reader so searcher reflects the latest commit state
        self.reload_with_retry()?;

        // 3. Enumerate all active segment IDs currently in the index
        let segment_ids: Vec<SegmentId> = self
            .reader
            .searcher()
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect();

        // 4. Consolidate segments if multiple segments exist
        if segment_ids.len() >= 2 {
            tracing::info!(
                segment_count = segment_ids.len(),
                "Consolidating Tantivy segments during compaction"
            );
            match writer.merge(&segment_ids).wait() {
                Ok(_merged) => {
                    let op = commit_with_retry(&mut writer)?;
                    self.last_commit_opstamp.store(op, Ordering::Release);
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("index writer was killed") || msg.contains("panicked") {
                        tracing::warn!("Tantivy writer killed during merge; recreating: {}", msg);
                        *writer = Self::create_writer_inner(&self.index)?;
                        let op = commit_with_retry(&mut writer)?;
                        self.last_commit_opstamp.store(op, Ordering::Release);
                    } else {
                        return Err(StorageError::Tantivy(format!("Segment merge error: {}", e)));
                    }
                }
            }
        }

        drop(writer);

        // 5. Final reader reload so searchers immediately pick up consolidated state
        self.reload_with_retry()?;

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
        self.reload_with_retry()?;
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
        self.reload_with_retry()?;
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

        let _ = writer.delete_term(term.clone());
        match commit_with_retry(&mut writer) {
            Ok(op) => {
                self.last_commit_opstamp.store(op, Ordering::Release);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("index writer was killed") || msg.contains("panicked") {
                    tracing::warn!("Tantivy writer killed during delete; recreating: {}", msg);
                    *writer = Self::create_writer_inner(&self.index)?;
                    let _ = writer.delete_term(term);
                    let op = commit_with_retry(&mut writer)?;
                    self.last_commit_opstamp.store(op, Ordering::Release);
                } else {
                    return Err(e);
                }
            }
        }

        drop(writer);
        self.reload_with_retry()?;

        Ok(count)
    }

    /// Clears all indexed documents from the Tantivy index.
    #[allow(dead_code)]
    pub(crate) fn clear(&self) -> Result<(), StorageError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        if let Err(e) = writer.delete_all_documents() {
            let msg = e.to_string();
            if msg.contains("index writer was killed") || msg.contains("panicked") {
                tracing::warn!("Tantivy writer killed during clear; recreating: {}", msg);
                *writer = Self::create_writer_inner(&self.index)?;
                let _ = writer.delete_all_documents();
            }
        }
        match commit_with_retry(&mut writer) {
            Ok(op) => {
                self.last_commit_opstamp.store(op, Ordering::Release);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("index writer was killed") || msg.contains("panicked") {
                    tracing::warn!("Tantivy writer killed during commit in clear; recreating: {}", msg);
                    *writer = Self::create_writer_inner(&self.index)?;
                    let _ = writer.delete_all_documents();
                    let op = commit_with_retry(&mut writer)?;
                    self.last_commit_opstamp.store(op, Ordering::Release);
                } else {
                    return Err(e);
                }
            }
        }

        drop(writer);
        self.reload_with_retry()?;

        Ok(())
    }

    /// Reloads the Tantivy reader, retrying on transient Windows file lock errors
    /// and ensuring the reader is synchronized to at least the last committed opstamp.
    fn reload_with_retry(&self) -> Result<(), StorageError> {
        let min_opstamp = self.last_commit_opstamp.load(Ordering::Acquire);
        for attempt in 1..=200 {
            if let Err(e) = self.reader.reload() {
                if attempt >= 200 {
                    return Err(StorageError::Tantivy(format!("Tantivy reload error: {}", e)));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }

            if min_opstamp > 0 {
                if let Ok(meta) = self.index.load_metas() {
                    if meta.opstamp >= min_opstamp {
                        let disk_segments: BTreeSet<(SegmentId, Option<u64>)> = meta
                            .segments
                            .iter()
                            .map(|s| (s.id(), s.delete_opstamp()))
                            .collect();
                        let searcher = self.reader.searcher();
                        let searcher_segments: BTreeSet<(SegmentId, Option<u64>)> = searcher
                            .segment_readers()
                            .iter()
                            .map(|r| (r.segment_id(), r.delete_opstamp()))
                            .collect();
                        if disk_segments == searcher_segments {
                            return Ok(());
                        }
                    }
                }
                if attempt >= 200 {
                    return Err(StorageError::Tantivy(format!(
                        "Tantivy reader reload failed to synchronize with commit opstamp {} after {} attempts",
                        min_opstamp, attempt
                    )));
                }
                let backoff_ms = if attempt <= 20 {
                    2
                } else if attempt <= 50 {
                    5
                } else if attempt <= 100 {
                    10
                } else {
                    20
                };
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
            } else {
                return Ok(());
            }
        }
        Ok(())
    }
}

/// Helper that retries Tantivy commit operations when encountering transient Windows file locks.
fn commit_with_retry(writer: &mut IndexWriter<TantivyDocument>) -> Result<u64, StorageError> {
    let mut attempts = 0;
    loop {
        match writer.commit() {
            Ok(opstamp) => return Ok(opstamp),
            Err(e) => {
                attempts += 1;
                if attempts >= 10 {
                    return Err(StorageError::Tantivy(format!("Tantivy commit error: {}", e)));
                }
                std::thread::sleep(std::time::Duration::from_millis(20 * attempts));
            }
        }
    }
}
