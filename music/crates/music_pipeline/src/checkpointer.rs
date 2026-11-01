//! Checkpoint manager for song versioning, state snapshotting, and WAV stem persistence.

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use music_core::state::MusicProductionState;
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use crate::error::{PipelineError, Result};

pub struct CheckpointManager {
    base_dir: PathBuf,
    sqlite_conn: Option<Arc<Mutex<rusqlite::Connection>>>,
}

impl CheckpointManager {
    /// Creates a file-based checkpointer root at base_dir.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            sqlite_conn: None,
        }
    }

    /// Initializes checkpointer with embedded SQLite metadata persistence.
    pub fn with_sqlite(base_dir: impl Into<PathBuf>, db_name: &str) -> Result<Self> {
        let base = base_dir.into();
        std::fs::create_dir_all(&base).map_err(|e| PipelineError::CheckpointIoError {
            path: base.clone(),
            source: e,
        })?;
        let db_path = base.join(db_name);
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS song_versions (
                song_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                active_node TEXT,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (song_id, version)
            )",
            [],
        ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS stem_records (
                song_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                stem_name TEXT NOT NULL,
                stem_path TEXT NOT NULL,
                PRIMARY KEY (song_id, version, stem_name)
            )",
            [],
        ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        Ok(Self {
            base_dir: base,
            sqlite_conn: Some(Arc::new(Mutex::new(conn))),
        })
    }

    /// Returns the base storage directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Validates path component preventing directory traversal, NT ADS breakouts,
    /// Windows reserved device names, and filesystem quirks.
    pub fn sanitize_path_component(component: &str) -> Result<String> {
        let comp_lower = component.to_ascii_lowercase();
        if component.is_empty()
            || component.contains('/')
            || component.contains('\\')
            || component.contains(':')
            || component.contains('\0')
            || component.contains("..")
            || comp_lower.contains("%2e")
            || comp_lower.contains("%2f")
            || comp_lower.contains("%5c")
            || comp_lower.contains("%00")
            || component.ends_with('.')
            || component.ends_with(' ')
        {
            return Err(PipelineError::SecurityError(format!(
                "Invalid path component: '{}'",
                component
            )));
        }

        // Windows reserved DOS device names (case-insensitive, alone or with any extension)
        const WINDOWS_RESERVED_NAMES: &[&str] = &[
            "con", "prn", "aux", "nul",
            "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
            "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];

        let base_stem = comp_lower.split('.').next().unwrap_or(&comp_lower);
        if WINDOWS_RESERVED_NAMES.contains(&base_stem) {
            return Err(PipelineError::SecurityError(format!(
                "Windows reserved device name rejected: '{}'",
                component
            )));
        }

        let p = Path::new(component);
        for c in p.components() {
            if !matches!(c, Component::Normal(_)) {
                return Err(PipelineError::SecurityError(format!(
                    "Path traversal attempt rejected in component: '{}'",
                    component
                )));
            }
        }

        Ok(component.to_string())
    }

    /// Resolves directory for song version: `./storage/songs/<song_id>/v<version>/`
    pub fn version_dir(&self, song_id: &str, version: u32) -> Result<PathBuf> {
        let clean_id = Self::sanitize_path_component(song_id)?;
        Ok(self.base_dir.join("songs").join(clean_id).join(format!("v{}", version)))
    }

    /// Resolves stems directory: `./storage/songs/<song_id>/v<version>/stems/`
    pub fn stems_dir(&self, song_id: &str, version: u32) -> Result<PathBuf> {
        Ok(self.version_dir(song_id, version)?.join("stems"))
    }

    /// Persists state snapshot to `./storage/songs/<song_id>/v<version>/state.json`.
    pub async fn save_checkpoint(
        &self,
        state: &MusicProductionState,
        active_node: &str,
    ) -> Result<PathBuf> {
        let v_dir = self.version_dir(&state.song_id, state.version)?;
        std::fs::create_dir_all(&v_dir).map_err(|e| PipelineError::CheckpointIoError {
            path: v_dir.clone(),
            source: e,
        })?;

        let state_path = v_dir.join("state.json");
        let json = state.to_json()?;
        std::fs::write(&state_path, &json).map_err(|e| PipelineError::CheckpointIoError {
            path: state_path.clone(),
            source: e,
        })?;

        // SQLite persistence
        if let Some(conn_lock) = &self.sqlite_conn {
            let conn = conn_lock.lock().map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO song_versions (song_id, version, active_node, state_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    state.song_id,
                    state.version,
                    active_node,
                    json,
                    chrono::Utc::now().to_rfc3339()
                ],
            ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
        }

        Ok(state_path)
    }

    /// Loads a specific version state snapshot.
    pub async fn load_checkpoint(&self, song_id: &str, version: u32) -> Result<MusicProductionState> {
        let v_dir = self.version_dir(song_id, version)?;
        let state_path = v_dir.join("state.json");
        let content = std::fs::read_to_string(&state_path).map_err(|e| PipelineError::CheckpointIoError {
            path: state_path,
            source: e,
        })?;
        let state = MusicProductionState::from_json(&content)?;
        Ok(state)
    }

    /// Saves an audio stem buffer directly into the version stems directory.
    pub async fn save_stem(
        &self,
        song_id: &str,
        version: u32,
        stem_name: &str,
        buffer: &AudioBuffer,
    ) -> Result<PathBuf> {
        let s_dir = self.stems_dir(song_id, version)?;
        std::fs::create_dir_all(&s_dir).map_err(|e| PipelineError::CheckpointIoError {
            path: s_dir.clone(),
            source: e,
        })?;

        let clean_name = Self::sanitize_path_component(stem_name)?;
        let stem_path = s_dir.join(format!("{}.wav", clean_name));
        buffer.to_wav_file_with_format(&stem_path, WavBitDepth::Pcm16)
            .map_err(|e| PipelineError::DspError(e.to_string()))?;

        if let Some(conn_lock) = &self.sqlite_conn {
            let conn = conn_lock.lock().map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO stem_records (song_id, version, stem_name, stem_path)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    song_id,
                    version,
                    stem_name,
                    stem_path.to_string_lossy().to_string()
                ],
            ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
        }

        Ok(stem_path)
    }

    /// Loads a specific audio stem buffer from the version stems directory.
    pub async fn load_stem(
        &self,
        song_id: &str,
        version: u32,
        stem_name: &str,
    ) -> Result<AudioBuffer> {
        let s_dir = self.stems_dir(song_id, version)?;
        let clean_name = Self::sanitize_path_component(stem_name)?;
        let stem_path = s_dir.join(format!("{}.wav", clean_name));
        AudioBuffer::from_wav_file(&stem_path).map_err(|e| PipelineError::DspError(e.to_string()))
    }

    /// Lists all persisted versions for a given song_id.
    pub fn list_versions(&self, song_id: &str) -> Result<Vec<u32>> {
        let clean_id = Self::sanitize_path_component(song_id)?;
        if let Some(conn_lock) = &self.sqlite_conn {
            let conn = conn_lock.lock().map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT version FROM song_versions WHERE song_id = ?1 ORDER BY version ASC")
                .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            let rows = stmt.query_map([&clean_id], |row| row.get(0))
                .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            let mut versions = Vec::new();
            for v in rows.flatten() {
                versions.push(v);
            }
            return Ok(versions);
        }

        // Filesystem scan fallback
        let song_dir = self.base_dir.join("songs").join(&clean_id);
        let mut versions = Vec::new();
        if song_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(song_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if let Some(stripped) = file_name.strip_prefix('v') {
                        if let Ok(ver) = stripped.parse::<u32>() {
                            versions.push(ver);
                        }
                    }
                }
            }
        }
        versions.sort_unstable();
        Ok(versions)
    }
}
