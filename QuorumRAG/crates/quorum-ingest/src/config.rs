//! Configuration structs for crawler, watcher, chunker, reindexer, and drift remediation.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration parameters governing crawler, watcher, and ingestion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Designated root directories or drive roots to crawl and watch.
    pub watch_paths: Vec<PathBuf>,

    /// Maximum file size permitted for ingestion in bytes (default: 50MB).
    pub max_file_size_bytes: u64,

    /// Minimum file size in bytes (default: 1; 0-byte files produce 0 chunks).
    pub min_file_size_bytes: u64,

    /// Whether to respect .gitignore rules (default: true).
    pub git_ignore: bool,

    /// Whether to respect .ignore rules (default: true).
    pub ignore_files: bool,

    /// Whether to respect global gitignore files (default: true).
    pub git_global: bool,

    /// Whether to skip hidden files and hidden directories (default: true).
    pub skip_hidden: bool,

    /// Whether to follow filesystem symlinks (default: false, prevents traversal loops).
    pub follow_symlinks: bool,

    /// Restrict traversal to the same filesystem / volume (default: false).
    pub same_file_system: bool,

    /// Number of worker threads for directory walking (0 = auto / CPU core count).
    pub crawler_threads: usize,

    /// Watcher debouncing duration in milliseconds (default: 300ms).
    pub debounce_duration_ms: u64,

    /// Internal channel buffer capacity for discovered files (default: 4096).
    pub channel_buffer_size: usize,

    /// Micro-batch size for storage writes (default: 64 chunks).
    pub micro_batch_size: usize,

    /// Micro-batch accumulation timeout in milliseconds (default: 500ms).
    pub batch_timeout_ms: u64,

    /// Access tier policy directory mapping rules: (Path prefix, access_tier 0..3).
    pub access_tier_rules: Vec<(PathBuf, u8)>,

    /// Optional automatic drift healing schedule in seconds.
    pub auto_drift_heal_interval_secs: Option<u64>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            max_file_size_bytes: 50 * 1024 * 1024, // 50MB
            min_file_size_bytes: 1,
            git_ignore: true,
            ignore_files: true,
            git_global: true,
            skip_hidden: true,
            follow_symlinks: false,
            same_file_system: false,
            crawler_threads: 0,
            debounce_duration_ms: 300,
            channel_buffer_size: 4096,
            micro_batch_size: 64,
            batch_timeout_ms: 500,
            access_tier_rules: Vec::new(),
            auto_drift_heal_interval_secs: None,
        }
    }
}

impl IngestConfig {
    pub fn debounce_duration(&self) -> Duration {
        Duration::from_millis(self.debounce_duration_ms)
    }

    pub fn batch_timeout(&self) -> Duration {
        Duration::from_millis(self.batch_timeout_ms)
    }

    /// Resolve access tier for a path based on longest matching prefix in access_tier_rules.
    pub fn resolve_access_tier(&self, path: &std::path::Path) -> u8 {
        let mut best_tier = 0u8;
        let mut longest_match = 0;

        for (prefix, tier) in &self.access_tier_rules {
            if path.starts_with(prefix) {
                let match_len = prefix.as_os_str().len();
                if match_len > longest_match {
                    longest_match = match_len;
                    best_tier = *tier;
                }
            }
        }

        best_tier
    }

    /// Discovers all mounted fixed physical drives on the host system (excluding CD-ROMs like `F:\`,
    /// removable drives, network shares, and virtual cloud mounts like `Google Drive`).
    pub fn discover_physical_drives() -> Vec<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
                fn GetVolumeInformationW(
                    lpRootPathName: *const u16,
                    lpVolumeNameBuffer: *mut u16,
                    nVolumeNameSize: u32,
                    lpVolumeSerialNumber: *mut u32,
                    lpMaximumComponentLength: *mut u32,
                    lpFileSystemFlags: *mut u32,
                    lpFileSystemNameBuffer: *mut u16,
                    nFileSystemNameSize: u32,
                ) -> i32;
            }

            const DRIVE_FIXED: u32 = 3;
            let mut data_drives = Vec::new();
            let mut os_drives = Vec::new();

            for letter in b'C'..=b'Z' {
                let root_str = format!("{}:\\", letter as char);
                let wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();

                let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
                if drive_type != DRIVE_FIXED {
                    continue;
                }

                let mut vol_name = [0u16; 261];
                let mut fs_name = [0u16; 261];
                let ok = unsafe {
                    GetVolumeInformationW(
                        wide.as_ptr(),
                        vol_name.as_mut_ptr(),
                        vol_name.len() as u32,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        fs_name.as_mut_ptr(),
                        fs_name.len() as u32,
                    )
                };
                if ok == 0 {
                    continue;
                }

                let vol_len = vol_name.iter().position(|&c| c == 0).unwrap_or(0);
                let fs_len = fs_name.iter().position(|&c| c == 0).unwrap_or(0);
                let vol_label = String::from_utf16_lossy(&vol_name[..vol_len]).to_ascii_lowercase();
                let fs_label = String::from_utf16_lossy(&fs_name[..fs_len]).to_ascii_uppercase();

                // Only include genuine local physical filesystems (NTFS / ReFS), excluding virtual cloud drives
                if !matches!(fs_label.as_str(), "NTFS" | "REFS") {
                    continue;
                }
                if vol_label.contains("google drive")
                    || vol_label.contains("onedrive")
                    || vol_label.contains("dropbox")
                {
                    continue;
                }

                let path = PathBuf::from(&root_str);
                if std::fs::read_dir(&path).is_ok() {
                    if letter == b'C' {
                        os_drives.push(path);
                    } else {
                        data_drives.push(path);
                    }
                }
            }

            data_drives.extend(os_drives);
            if data_drives.is_empty() {
                if let Ok(cwd) = std::env::current_dir() {
                    data_drives.push(cwd);
                }
            }
            data_drives
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut drives = Vec::new();
            for candidate in &["/mnt", "/Volumes", "/home", "/Users"] {
                let p = PathBuf::from(candidate);
                if p.exists() && std::fs::read_dir(&p).is_ok() {
                    drives.push(p);
                }
            }
            if drives.is_empty() {
                drives.push(PathBuf::from("/"));
            }
            drives
        }
    }
}

/// Configuration parameters for text chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Target chunk size in characters (default: 500, matching test_ingestion_dedup.rs).
    pub target_chunk_size: usize,
    /// Overlap in characters between adjacent chunks (default: 100).
    pub chunk_overlap: usize,
    /// Minimum chunk length to avoid tiny trailing fragments (default: 40).
    pub min_chunk_size: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            target_chunk_size: 500,
            chunk_overlap: 100,
            min_chunk_size: 40,
        }
    }
}

/// Reindexing execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexOptions {
    /// Target root paths to reindex. If empty, uses configured daemon watch directories.
    pub target_paths: Vec<PathBuf>,
    /// Full rebuild mode: bypasses mtime and Level-1 BLAKE3 catalogs, re-verifying all files.
    pub full_rebuild: bool,
    /// Force re-embedding all chunks even if Level-2 chunk BLAKE3 cache matches.
    pub force_reembed: bool,
    /// Micro-batch size for storage writes (default: 64).
    pub batch_size: usize,
    /// Number of worker threads for parallel file walking (0 = auto).
    pub num_threads: usize,
    /// Dry run mode: reports files and chunks that would be changed without writing to DB.
    pub dry_run: bool,
}

impl Default for ReindexOptions {
    fn default() -> Self {
        Self {
            target_paths: Vec::new(),
            full_rebuild: false,
            force_reembed: false,
            batch_size: 128,
            num_threads: 0,
            dry_run: false,
        }
    }
}

/// Categories of drift that can be detected and remediated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Filesystem <-> Database Merkle state drift (untracked or deleted files).
    MerkleState,
    /// LanceDB Vector <-> Tantivy BM25 hybrid index desync.
    HybridIndexDesync,
    /// Obsolete contract hash, dimension, or non-unit L2 norm.
    VectorContract,
    /// ACL access_tier policy mismatch with directory configuration.
    AclPolicy,
    /// Storage fragmentation, tombstones, and unmerged segments.
    StorageCompaction,
}

/// Drift options configuring which drifts to fix and execution mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftFixOptions {
    /// Specific drift types to address. Empty means fix ALL 5 types.
    pub target_drifts: Vec<DriftType>,
    /// Dry run mode: detect and report drift anomalies without modifying storage.
    pub dry_run: bool,
    /// Directory configuration mapping (path prefix -> AccessTier) for ACL drift.
    pub access_tier_rules: Vec<(PathBuf, u8)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_physical_drives_excludes_non_physical() {
        let drives = IngestConfig::discover_physical_drives();
        assert!(!drives.is_empty(), "Must discover at least one physical drive");

        let drive_strings: Vec<String> = drives
            .iter()
            .map(|p| p.to_string_lossy().to_ascii_uppercase())
            .collect();

        #[cfg(target_os = "windows")]
        {
            assert!(
                !drive_strings.contains(&"F:\\".to_string()),
                "F:\\ is not a physical drive (CD-ROM) and must be excluded; got {:?}",
                drive_strings
            );
            assert!(
                !drive_strings.contains(&"G:\\".to_string()),
                "G:\\ is a virtual Google Drive mount and must be excluded; got {:?}",
                drive_strings
            );
            assert!(
                drive_strings.contains(&"D:\\".to_string()) || drive_strings.contains(&"C:\\".to_string()),
                "Must discover physical NTFS drives (D:\\ and/or C:\\); got {:?}",
                drive_strings
            );
        }
    }
}

