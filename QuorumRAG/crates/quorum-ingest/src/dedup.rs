//! Two-Level BLAKE3 Deduplication Engine for QuorumRAG.
//!
//! Level 1: Full-file BLAKE3 hash catalog mapping `(file_path, file_blake3, mtime, size)`.
//!          Detects renames and moves, and skips unchanged files entirely with zero re-embeddings.
//! Level 2: Chunk-level BLAKE3 hash cache mapping `chunk_blake3 -> Vec<f32>` ensuring
//!          identical text chunks across distinct files or edits reuse existing embeddings.

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

/// Catalog entry recording metadata and content hashes for a tracked file on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileCatalogEntry {
    /// Canonical or relative path of the file.
    pub file_path: PathBuf,
    /// 64-character lowercase hex BLAKE3 hash of the entire file content.
    pub file_blake3: String,
    /// Last modified timestamp in seconds since Unix epoch.
    pub mtime_secs: i64,
    /// File size in bytes.
    pub size_bytes: u64,
    /// List of Level-2 chunk BLAKE3 hashes generated from this file.
    pub chunk_hashes: Vec<String>,
}

/// Action to be taken for a file evaluated against the Level-1 catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupAction {
    /// File has identical mtime and size, or identical BLAKE3 content hash.
    /// Action: Skip chunking and embedding entirely.
    Unchanged {
        file_blake3: String,
        chunk_hashes: Vec<String>,
    },
    /// File content hash matches an existing entry under a different path.
    /// Action: Rename/Move detected. Update catalog path, zero new embeddings!
    MovedOrRenamed {
        old_path: PathBuf,
        new_path: PathBuf,
        file_blake3: String,
        chunk_hashes: Vec<String>,
    },
    /// File is either new or its content BLAKE3 hash has changed.
    /// Action: Proceed to chunking; chunk vectors will be checked against Level-2 cache.
    ModifiedOrNew {
        file_blake3: String,
        is_new: bool,
    },
}

/// Level-1 File Deduplication Catalog with fast-path metadata checking and rename detection.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Level1DedupCatalog {
    /// Primary index: file_blake3 -> FileCatalogEntry
    entries: HashMap<String, FileCatalogEntry>,
    /// Secondary index: file_path -> file_blake3
    path_to_hash: HashMap<PathBuf, String>,
}

impl Level1DedupCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load catalog from a JSON file on disk.
    pub fn load_from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let catalog = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(catalog)
    }

    /// Persist catalog atomically to a JSON file on disk.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp_path = path.with_extension("tmp");
        {
            let f = File::create(&temp_path)?;
            serde_json::to_writer_pretty(f, self)
                .map_err(io::Error::other)?;
        }
        std::fs::rename(temp_path, path)?;
        Ok(())
    }

    /// Compute 64-char lowercase hex BLAKE3 hash of a file via streaming reader.
    pub fn hash_file_stream(path: impl AsRef<Path>) -> io::Result<String> {
        let f = File::open(path)?;
        let mut reader = BufReader::with_capacity(64 * 1024, f);
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Check a file's status against the catalog before reading content.
    ///
    /// 1. Fast Path: If path, mtime, and size match existing entry -> Unchanged (0 disk read).
    /// 2. Move/Rename Detection: If content hash exists under another path -> MovedOrRenamed.
    /// 3. Otherwise -> ModifiedOrNew.
    pub fn evaluate_file(
        &mut self,
        path: &Path,
        current_mtime: i64,
        current_size: u64,
    ) -> io::Result<DedupAction> {
        // Step 1: Check if path exists in secondary index
        if let Some(existing_hash) = self.path_to_hash.get(path) {
            if let Some(entry) = self.entries.get(existing_hash) {
                if entry.mtime_secs == current_mtime && entry.size_bytes == current_size {
                    // FAST PATH: mtime and size unchanged, skip reading file!
                    return Ok(DedupAction::Unchanged {
                        file_blake3: existing_hash.clone(),
                        chunk_hashes: entry.chunk_hashes.clone(),
                    });
                }
            }
        }

        // Step 2: Content may have changed or file is moved/new. Compute BLAKE3 hash.
        let file_b3 = Self::hash_file_stream(path)?;

        // Check if content hash already exists in catalog
        if let Some(entry) = self.entries.get_mut(&file_b3) {
            if entry.file_path != path {
                // RENAME OR MOVE DETECTED!
                let old_path = entry.file_path.clone();
                let chunk_hashes = entry.chunk_hashes.clone();

                // Update indices
                self.path_to_hash.remove(&old_path);
                self.path_to_hash.insert(path.to_path_buf(), file_b3.clone());
                entry.file_path = path.to_path_buf();
                entry.mtime_secs = current_mtime;
                entry.size_bytes = current_size;

                return Ok(DedupAction::MovedOrRenamed {
                    old_path,
                    new_path: path.to_path_buf(),
                    file_blake3: file_b3,
                    chunk_hashes,
                });
            } else {
                // Content identical, but mtime or size metadata changed (e.g. touched)
                entry.mtime_secs = current_mtime;
                entry.size_bytes = current_size;
                return Ok(DedupAction::Unchanged {
                    file_blake3: file_b3,
                    chunk_hashes: entry.chunk_hashes.clone(),
                });
            }
        }

        // Step 3: Truly new or modified content
        let is_new = !self.path_to_hash.contains_key(path);
        Ok(DedupAction::ModifiedOrNew {
            file_blake3: file_b3,
            is_new,
        })
    }

    /// Register or update an entry in the catalog after chunking/indexing.
    pub fn register_entry(&mut self, entry: FileCatalogEntry) {
        self.path_to_hash
            .insert(entry.file_path.clone(), entry.file_blake3.clone());
        self.entries.insert(entry.file_blake3.clone(), entry);
    }

    /// Remove a file from the catalog when deleted on disk.
    pub fn remove_file(&mut self, path: &Path) -> Option<FileCatalogEntry> {
        if let Some(hash) = self.path_to_hash.remove(path) {
            self.entries.remove(&hash)
        } else {
            None
        }
    }

    pub fn get_file_entry(&self, path: &Path) -> Option<&FileCatalogEntry> {
        self.path_to_hash
            .get(path)
            .and_then(|hash| self.entries.get(hash))
    }

    pub fn get_all_tracked_files(&self) -> Vec<PathBuf> {
        self.path_to_hash.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Statistics for Level-2 Chunk Cache telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub capacity: usize,
    pub current_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }
}

/// Level-2 Chunk Cache mapping `chunk_blake3 -> Vec<f32>` (Gemma 768d embedding).
///
/// Thread-safe bounded LRU cache preventing duplicate neural embeddings across
/// distinct files or within single-line edits.
pub struct Level2ChunkCache {
    inner: Arc<RwLock<Level2ChunkCacheInner>>,
}

struct Level2ChunkCacheInner {
    capacity: usize,
    map: HashMap<String, (Vec<f32>, usize)>,
    order: VecDeque<String>,
    hits: u64,
    misses: u64,
}

impl Level2ChunkCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Level2ChunkCacheInner {
                capacity: capacity.max(128),
                map: HashMap::with_capacity(capacity.min(65536)),
                order: VecDeque::with_capacity(capacity.min(65536)),
                hits: 0,
                misses: 0,
            })),
        }
    }

    /// Retrieve cached embedding vector if present.
    pub fn get(&self, chunk_blake3: &str) -> Option<Vec<f32>> {
        let mut inner = self.inner.write().ok()?;
        if let Some(vec) = inner.map.get(chunk_blake3).map(|(v, _)| v.clone()) {
            inner.hits += 1;
            Some(vec)
        } else {
            inner.misses += 1;
            None
        }
    }

    /// Insert a newly computed embedding vector into the cache.
    pub fn insert(&self, chunk_blake3: String, vector: Vec<f32>) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        if inner.map.contains_key(&chunk_blake3) {
            return;
        }

        // Evict if at capacity
        while inner.map.len() >= inner.capacity && !inner.order.is_empty() {
            if let Some(oldest) = inner.order.pop_front() {
                inner.map.remove(&oldest);
            }
        }

        let order_len = inner.order.len();
        inner.order.push_back(chunk_blake3.clone());
        inner.map.insert(chunk_blake3, (vector, order_len));
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.read().unwrap();
        CacheStats {
            hits: inner.hits,
            misses: inner.misses,
            capacity: inner.capacity,
            current_size: inner.map.len(),
        }
    }
}

/// Unified Two-Level Deduplication Engine combining Level-1 File Catalog
/// and Level-2 Chunk Vector Cache with thread-safe synchronization.
#[derive(Clone)]
pub struct TwoLevelDedupEngine {
    catalog: Arc<RwLock<Level1DedupCatalog>>,
    cache: Arc<Level2ChunkCache>,
}

impl Default for TwoLevelDedupEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TwoLevelDedupEngine {
    pub fn new() -> Self {
        Self {
            catalog: Arc::new(RwLock::new(Level1DedupCatalog::new())),
            cache: Arc::new(Level2ChunkCache::new(50_000)),
        }
    }

    pub fn catalog(&self) -> &Arc<RwLock<Level1DedupCatalog>> {
        &self.catalog
    }

    pub fn cache(&self) -> &Arc<Level2ChunkCache> {
        &self.cache
    }

    /// Evaluates a file against the Level-1 catalog.
    pub fn evaluate_file(
        &self,
        path: &Path,
        current_mtime: i64,
        current_size: u64,
    ) -> io::Result<DedupAction> {
        let mut catalog = self.catalog.write().map_err(|_| {
            io::Error::other("Level 1 catalog lock poisoned")
        })?;
        catalog.evaluate_file(path, current_mtime, current_size)
    }

    /// Registers a newly indexed file into the Level-1 catalog.
    pub fn register_entry(&self, entry: FileCatalogEntry) {
        if let Ok(mut catalog) = self.catalog.write() {
            catalog.register_entry(entry);
        }
    }

    /// Removes a file from the catalog when deleted from disk.
    pub fn remove_file(&self, path: &Path) -> Option<FileCatalogEntry> {
        let mut catalog = self.catalog.write().ok()?;
        catalog.remove_file(path)
    }

    pub fn get_file_entry(&self, path: &Path) -> Option<FileCatalogEntry> {
        let catalog = self.catalog.read().ok()?;
        catalog.get_file_entry(path).cloned()
    }

    pub fn get_all_tracked_files(&self) -> Vec<PathBuf> {
        let catalog = self.catalog.read().ok();
        catalog.map(|c| c.get_all_tracked_files()).unwrap_or_default()
    }
}
