//! Google Magika Content Classifier Router & Binary Filter.
//!
//! # Core Invariants:
//! 1. DO NOT SKIP IMAGES! Images (`png`, `jpeg`, `webp`, `gif`, `bmp`, `tiff`, `svg`, etc.)
//!    must be routed to `CandidateKind::Image` for rich metadata and feature indexing.
//! 2. Text, source code, and documents route to `CandidateKind::Text` for chunking.
//! 3. Compiled binaries (PE, ELF, Mach-O, WASM), archives, and audio/video are filtered out.
//! 4. Files exceeding the configured size cap (default: 50MB) are rejected.
//! 5. 0-byte files are accepted as text with 0 chunks without panic.

use std::path::Path;
use quorum_classifier::{FileCategory, FileIdentity, FileTypeClassifier};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config::IngestConfig;

/// Classified candidate destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateKind {
    /// Text, code, or structured document -> routed to Chunker -> Gemma Embedder.
    Text {
        char_count_estimate: usize,
    },
    /// Image file -> routed to ImageIndexer (dimensions, EXIF, tokens, visual summary).
    Image {
        format_label: String,
        mime_type: String,
    },
}

/// Filter decision with explicit audit rationale.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterDecision {
    /// Valid candidate accepted for pipeline ingestion.
    Accept {
        identity: FileIdentity,
        kind: CandidateKind,
        size_bytes: u64,
        mtime_secs: i64,
    },
    /// Skipped because file size exceeds configured cap.
    SkipExceedsSizeCap {
        size_bytes: u64,
        max_bytes: u64,
    },
    /// Skipped because it is a non-indexable compiled binary, archive, or media stream.
    SkipBinary {
        identity: FileIdentity,
    },
    /// Skipped because file is hidden and skip_hidden is active.
    SkipHidden,
    /// Skipped because symlink following is disabled.
    SkipSymlink,
    /// Skipped due to inaccessible or locked file after retry.
    SkipIoError {
        error: String,
    },
}

/// Content-type classifier router combining Google Magika with QuorumRAG filtering policies.
#[derive(Debug, Clone)]
pub struct ClassifierRouter {
    classifier: FileTypeClassifier,
    config: IngestConfig,
}

impl ClassifierRouter {
    pub fn new(config: IngestConfig) -> Self {
        Self {
            classifier: FileTypeClassifier::new(),
            config,
        }
    }

    /// Evaluates a candidate path on disk and returns the definitive routing decision.
    pub fn evaluate_file(&self, path: &Path) -> FilterDecision {
        // 1. Check symlink policy
        if !self.config.follow_symlinks {
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    return FilterDecision::SkipSymlink;
                }
            }
        }

        // 2. Check hidden file policy
        if self.config.skip_hidden {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with('.') {
                    return FilterDecision::SkipHidden;
                }
            }
        }

        // 3. Inspect metadata with retry for Windows sharing violations
        let metadata = match self.read_metadata_with_retry(path, 3) {
            Ok(m) => m,
            Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
        };

        if !metadata.is_file() {
            return FilterDecision::SkipIoError { error: "Path is not a regular file".into() };
        }

        let size = metadata.len();
        if size > self.config.max_file_size_bytes {
            return FilterDecision::SkipExceedsSizeCap {
                size_bytes: size,
                max_bytes: self.config.max_file_size_bytes,
            };
        }

        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // 4. Content classification via Google Magika (first 4KB inspection)
        let identity = match self.classifier.classify_file(path) {
            Ok(id) => id,
            Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
        };

        // 5. Ingestion Routing Invariant:
        // DO NOT SKIP IMAGES! Images must route to ImageIndexer.
        if identity.category == FileCategory::Image || Self::is_image_mime(&identity.mime_type) || Self::is_image_label(&identity.label) {
            return FilterDecision::Accept {
                identity: identity.clone(),
                kind: CandidateKind::Image {
                    format_label: identity.label,
                    mime_type: identity.mime_type,
                },
                size_bytes: size,
                mtime_secs: mtime,
            };
        }

        // 6. Text / Source Code / Structured Data Routing
        if identity.is_text || matches!(
            identity.category,
            FileCategory::Code | FileCategory::Text | FileCategory::Document | FileCategory::Data
        ) {
            return FilterDecision::Accept {
                identity,
                kind: CandidateKind::Text {
                    char_count_estimate: size as usize,
                },
                size_bytes: size,
                mtime_secs: mtime,
            };
        }

        // 7. Non-indexable compiled binaries (PE, ELF, Mach-O, DEX, WASM, Archives, Audio, Video)
        debug!(
            path = %path.display(),
            label = %identity.label,
            mime = %identity.mime_type,
            category = %identity.category,
            "Filtered out non-indexable binary/media file"
        );
        FilterDecision::SkipBinary { identity }
    }

    /// Fast helper to confirm if a MIME type represents a supported image format.
    pub fn is_image_mime(mime: &str) -> bool {
        matches!(
            mime,
            "image/jpeg"
                | "image/png"
                | "image/webp"
                | "image/gif"
                | "image/bmp"
                | "image/tiff"
                | "image/svg+xml"
                | "image/x-icon"
                | "image/avif"
        )
    }

    /// Fast helper to confirm if a label represents an image format.
    pub fn is_image_label(label: &str) -> bool {
        matches!(
            label,
            "png" | "jpeg" | "jpg" | "webp" | "gif" | "bmp" | "svg" | "tiff"
        )
    }

    /// Windows file-lock retry helper: retries `std::fs::metadata` on sharing violations.
    fn read_metadata_with_retry(&self, path: &Path, retries: usize) -> std::io::Result<std::fs::Metadata> {
        let mut last_err = None;
        for attempt in 0..retries {
            match std::fs::metadata(path) {
                Ok(m) => return Ok(m),
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < retries {
                        std::thread::sleep(std::time::Duration::from_millis(50 * (attempt as u64 + 1)));
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| std::io::Error::other("Unknown error")))
    }
}
