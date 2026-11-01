# Milestone 1 Exploration & Specification Report: `crates/music_core`

**Author**: Explorer Subagent (`explorer_m1_1`)  
**Mission**: Plan the detailed implementation of `crates/music_core` for Milestone 1.  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m1_1\`  
**Target Crate**: `crates/music_core/`  
**Status**: Complete & Ready for Implementation  

---

## 1. Observation

### 1.1 Existing Filesystem & Crate Layout
Direct inspection of `d:\harfile\music` confirmed:
- `Cargo.toml` exists at workspace root declaring 5 members:
  ```toml
  members = [
      "crates/music_core",
      "crates/music_fusion",
      "crates/music_dsp",
      "crates/music_pipeline",
      "crates/music_cli",
  ]
  ```
- `crates/music_core/Cargo.toml` exists (19 lines, 443 bytes).
- `crates/music_core/src/schema.rs` exists (467 lines, 23,558 bytes) defining:
  - `PipelineModelStack` (lines 5–48)
  - `StemMixConfig` (lines 51–90)
  - `SongSection` (lines 93–122)
  - `SongBlueprint` (lines 125–466) with `default_one_love_shining_anthem()` and `default_rise_up_together_anthem()`.
- **Missing Files in `crates/music_core`**:
  - `crates/music_core/src/lib.rs` DOES NOT EXIST.
  - `crates/music_core/src/state.rs` DOES NOT EXIST.
  - `crates/music_core/src/telemetry.rs` DOES NOT EXIST.
  - `crates/music_core/src/genre.rs` DOES NOT EXIST.
  - `crates/music_core/src/blueprint.rs` DOES NOT EXIST.
  - `crates/music_core/src/error.rs` DOES NOT EXIST.
  - `crates/music_core/tests/` DOES NOT EXIST.

### 1.2 Verbatim Cargo Build Failure
Running `cargo check` in `d:\harfile\music` reports:
```
error: failed to load manifest for workspace member `D:\harfile\music\crates\music_core`
referenced by workspace at `D:\harfile\music\Cargo.toml`

Caused by:
  failed to parse manifest at `D:\harfile\music\crates\music_core\Cargo.toml`

Caused by:
  no targets specified in the manifest
  either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present
```

### 1.3 `crates/music_core/Cargo.toml` Current Contents
Inspecting lines 1–19 of `crates/music_core/Cargo.toml`:
```toml
[package]
name = "music_core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
rusqlite = { workspace = true }
chrono = { workspace = true }
sysinfo = { workspace = true }
uuid = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }
regex = { workspace = true }
reqwest = { workspace = true }
```
Observation: `sysinfo` and `reqwest` are heavyweight dependencies belonging to `music_fusion` (which probes hardware and calls Ollama HTTP APIs). `music_core` is the domain model crate and only requires serde, serde_json, uuid, chrono, thiserror, anyhow, and regex. Streamlining `music_core` dependencies keeps compilation ultra-fast.

---

## 2. Logic Chain

1. **Root Dependency Invariant**: `crates/music_core` is the foundational dependency for all other crates in the workspace (`music_fusion`, `music_dsp`, `music_pipeline`, `music_cli`). None of the other crates can compile or declare their interface contracts until `music_core` provides the canonical structs:
   - `MusicProductionState` (state container passed through LangGraph and REST endpoints).
   - `StemTelemetry` (audio inspection metrics passed from DSP to Judge gate).
   - `GenreTaxonomy` / `GenrePreset` (style prompts, BPM ranges, and scales passed to Ollama panel and DSP synthesizers).
   - `SongBlueprint` / `SongSection` / `StemMixConfig` (structural and mix parameters).

2. **13+ Field State Specification**: `ORIGINAL_REQUEST.md` specifies `MusicProductionState` with 13 mandatory fields:
   - `song_id: String`
   - `version: u32`
   - `engine_mode: String` (`"auto"`, `"pure_rust"`, `"hf_hybrid"`)
   - `style_prompt: String`
   - `lyrics: String`
   - `bpm: u32` (default 76)
   - `key_signature: String` (default `"A Major"`)
   - `vocal_ref_wav: Option<PathBuf>`
   - `master_ref_wav: Option<PathBuf>`
   - `stem_paths: HashMap<String, PathBuf>`
   - `telemetry: StemTelemetry`
   - `judge_score: f32`
   - `quality_passed: bool`
   - `retry_count: u8`
   To support universal multi-genre adaptability, persistent versioning, and pipeline tracking, we extend this to 19 fields by adding:
   - `genre: String` (primary genre name)
   - `time_signature: String` (`"4/4"`, `"3/4"`, `"6/8"`, etc.)
   - `blueprint: Option<SongBlueprint>` (full bar-by-bar arrangement)
   - `active_node: Option<String>` (current pipeline stage)
   - `history: Vec<String>` (audit log of node transitions)
   - `created_at: String` / `updated_at: String` (RFC 3339 timestamps)

3. **Audio Telemetry Invariant**: `StemTelemetry` must capture waveform health so Node 5 (`dsp_and_judge_gate_node`) can decide whether to pass to Node 6 (`analog_mastering_node`) or trigger a conditional retry. It must contain:
   - `peak_dbfs: f32`, `rms_dbfs: f32`, `dynamic_range_db: f32`, `tempo_drift_score: f32`, `clipping: bool`, `snr_db: f32`, `spectral_flatness: f32`, `headroom_db: f32`, `duration_secs: f32`, `sample_rate: u32`, `channels: u16`.
   - Methods `is_broadcast_ready() -> bool` and `validation_warnings() -> Vec<String>` provide immediate deterministic gating.

4. **Universal Genre Taxonomy Invariant**: The follow-up request explicitly mandates that this engine cannot be hardcoded only to reggae, but must support "all types of music known to man" while retaining the 1977 reggae anthem as a primary preset.
   - We define `GenreCategory` (11 categories: `RootsReggaeDub`, `AfrobeatHighlifeAmapiano`, `RnbSoulGospel`, `HipHopTrap`, `BluesRockMetal`, `JazzFusion`, `ClassicalCinematic`, `PopElectronic`, `LatinCaribbean`, `FolkCountryWorld`, `ExperimentalHybrid`).
   - We define `ModalScale` (16 modal scales: `Ionian`, `Dorian`, `Phrygian`, `Lydian`, `Mixolydian`, `Aeolian`, `Locrian`, `HarmonicMinor`, `MelodicMinor`, `MajorPentatonic`, `MinorPentatonic`, `BluesScale`, `ArabicHijaz`, `JapaneseHirajoshi`, `SpanishGypsy`, `RagaBhairav`).
   - We define `RhythmGroove` (16 rhythmic groove patterns: `OneDrop`, `Steppers`, `FourOnTheFloor`, `BoomBap`, `Trap808`, `AfrobeatTresillo`, `AmapianoLogDrum`, `HighlifeSyncopation`, `RockBackbeat`, `MetalDoubleKick`, `JazzSwingRide`, `BossaNovaClave`, `SalsaCascara`, `CinematicPercussion`, `WaltzThreeFour`, `GospelShuffle`).
   - We define `GenrePreset` containing tempo range, default BPM, default key, modal scale, groove pattern, YuE style tags, instrumentation, vocal archetype, mastering profile, and stem mix config.
   - We implement `GenreTaxonomy::find_or_create(query: &str) -> GenrePreset`: If an exact or fuzzy preset matches, it returns it; if an unknown hybrid is queried (e.g., "Future Glitch Cyberpunk"), it parses keywords and creates a fully valid `GenrePreset` without ever returning empty or panicking.

5. **Blueprint Integration**: `schema.rs` already contains rich implementations for `SongBlueprint`, `SongSection`, `StemMixConfig`, and `PipelineModelStack`. `crates/music_core/src/blueprint.rs` should re-export these types, satisfying `PROJECT.md` layout without duplicating or destabilizing `schema.rs`.

---

## 3. Caveats

1. **Read-Only Scope**: As an Explorer agent, we do not modify files in `crates/music_core/` directly. All implementations below are provided for Worker 1 to copy and commit.
2. **Audio Buffer Processing**: `crates/music_core` defines the data structures (`StemTelemetry`, `StemMixConfig`) and validation logic, but the actual 48 kHz WAV decoding/encoding and FFT/spectral math are implemented in `crates/music_dsp` (Milestone 2).
3. **Hardware / Ollama Probing**: Hardware detection (`SystemMemory::detect_live()`) and HTTP communication with Ollama live in `crates/music_fusion` (also part of Milestone 1, handled in parallel by `explorer_m1_2`).

---

## 4. Conclusion & Complete Implementation Specification

Below are the exact, complete, production-ready Rust source code specifications for all files required to implement `crates/music_core`.

### 4.1 `crates/music_core/Cargo.toml`
```toml
[package]
name = "music_core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
regex = { workspace = true }
```

---

### 4.2 `crates/music_core/src/error.rs`
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicCoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("State validation error: {0}")]
    ValidationError(String),

    #[error("Genre not found in taxonomy: {0}")]
    GenreNotFound(String),

    #[error("Missing required stem: {0}")]
    MissingStem(String),

    #[error("Invalid BPM {0}: must be between 30 and 300")]
    InvalidBpm(u32),
}

pub type Result<T> = std::result::Result<T, MusicCoreError>;
```

---

### 4.3 `crates/music_core/src/telemetry.rs`
```rust
use serde::{Deserialize, Serialize};

/// Audio telemetry metrics calculated from waveform analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemTelemetry {
    /// Peak amplitude in dBFS (0.0 dBFS is digital maximum)
    pub peak_dbfs: f32,
    /// Root-Mean-Square loudness in dBFS
    pub rms_dbfs: f32,
    /// Dynamic range in dB (difference between peak and noise floor / quiet passages)
    pub dynamic_range_db: f32,
    /// Tempo drift score (0.0 = perfect grid lock, >0.05 = noticeable drift)
    pub tempo_drift_score: f32,
    /// True if any sample reaches or exceeds 0.0 dBFS
    pub clipping: bool,
    /// Signal-to-noise ratio in dB
    pub snr_db: f32,
    /// Spectral flatness measure (0.0 = pure tone, 1.0 = white noise)
    pub spectral_flatness: f32,
    /// True peak headroom in dB (distance below 0.0 dBFS)
    pub headroom_db: f32,
    /// Duration of audio in seconds
    pub duration_secs: f32,
    /// Sampling rate in Hz (standard: 48000 Hz)
    pub sample_rate: u32,
    /// Channel count (1 = mono, 2 = stereo)
    pub channels: u16,
}

impl Default for StemTelemetry {
    fn default() -> Self {
        Self {
            peak_dbfs: -0.5,
            rms_dbfs: -14.0,
            dynamic_range_db: 12.0,
            tempo_drift_score: 0.002,
            clipping: false,
            snr_db: 92.0,
            spectral_flatness: 0.18,
            headroom_db: 0.5,
            duration_secs: 0.0,
            sample_rate: 48_000,
            channels: 2,
        }
    }
}

impl StemTelemetry {
    pub fn new_broadcast_standard() -> Self {
        Self::default()
    }

    /// Evaluates whether the audio stem passes studio digital quality inspection.
    pub fn is_broadcast_ready(&self) -> bool {
        !self.clipping
            && self.peak_dbfs <= 0.0
            && self.dynamic_range_db >= 6.0
            && self.tempo_drift_score < 0.05
            && self.rms_dbfs <= -6.0
    }

    /// Checks if headroom is at or above required threshold in dB.
    pub fn has_headroom(&self, min_headroom_db: f32) -> bool {
        self.headroom_db >= min_headroom_db
    }

    /// Generates human-readable inspection warnings if anomalies are detected.
    pub fn validation_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.clipping {
            warnings.push(format!(
                "Digital clipping detected: peak dBFS is {:.2}",
                self.peak_dbfs
            ));
        }
        if self.peak_dbfs > 0.0 {
            warnings.push(format!(
                "Peak exceeds 0.0 dBFS ceiling: {:.2} dBFS",
                self.peak_dbfs
            ));
        }
        if self.dynamic_range_db < 4.0 {
            warnings.push(format!(
                "Dynamic range is excessively compressed: {:.2} dB",
                self.dynamic_range_db
            ));
        }
        if self.tempo_drift_score >= 0.05 {
            warnings.push(format!(
                "Significant tempo drift detected: drift score {:.4} exceeds 0.05 threshold",
                self.tempo_drift_score
            ));
        }
        if self.rms_dbfs > -6.0 {
            warnings.push(format!(
                "RMS loudness is dangerously hot: {:.2} dBFS",
                self.rms_dbfs
            ));
        }
        warnings
    }

    /// Diagnostic summary string for logging and UI telemetry readout.
    pub fn summary(&self) -> String {
        format!(
            "Peak: {:.2} dBFS | RMS: {:.2} dBFS | DynRange: {:.2} dB | Drift: {:.4} | Clip: {} | Headroom: {:.2} dB | SNR: {:.1} dB",
            self.peak_dbfs,
            self.rms_dbfs,
            self.dynamic_range_db,
            self.tempo_drift_score,
            self.clipping,
            self.headroom_db,
            self.snr_db
        )
    }
}
```

---

### 4.4 `crates/music_core/src/state.rs`
```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{MusicCoreError, Result};
use crate::schema::SongBlueprint;
use crate::telemetry::StemTelemetry;

/// Engine execution mode for song generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    Auto,
    PureRust,
    HfHybrid,
}

impl Default for EngineMode {
    fn default() -> Self {
        EngineMode::Auto
    }
}

impl EngineMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineMode::Auto => "auto",
            EngineMode::PureRust => "pure_rust",
            EngineMode::HfHybrid => "hf_hybrid",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_ascii_lowercase().trim() {
            "pure_rust" | "purerust" | "pure-rust" | "dsp" => EngineMode::PureRust,
            "hf_hybrid" | "hfhybrid" | "hf-hybrid" | "gpu" | "huggingface" => EngineMode::HfHybrid,
            _ => EngineMode::Auto,
        }
    }
}

// Canonical Stem Key Constants
pub const STEM_DRAFT_GROOVE: &str = "draft_groove";
pub const STEM_YUE_VOCALS: &str = "yue_vocals";
pub const STEM_YUE_INSTRUMENTALS: &str = "yue_instrumentals";
pub const STEM_RASPY_TENOR_VOCALS: &str = "raspy_tenor_vocals";
pub const STEM_DRUMS: &str = "drums";
pub const STEM_BASS: &str = "bass";
pub const STEM_GUITAR: &str = "guitar";
pub const STEM_OTHER: &str = "other";
pub const STEM_INPAINTED_GUITAR: &str = "inpainted_guitar";
pub const STEM_INPAINTED_DRUMS: &str = "inpainted_drums";
pub const STEM_FINAL_MASTER: &str = "final_master";

/// Complete, versioned State container flowing through the 6-stage LangGraph StateGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicProductionState {
    /// 1. Unique song identifier
    pub song_id: String,
    /// 2. Incremental version number (starts at 1)
    pub version: u32,
    /// 3. Execution engine mode: "auto", "pure_rust", or "hf_hybrid"
    pub engine_mode: String,
    /// 4. Style prompt describing genre, instrumentation, mood, production era
    pub style_prompt: String,
    /// 5. Full song lyrics with structural tags ([Intro], [Verse 1], [Chorus], etc.)
    pub lyrics: String,
    /// 6. Target Tempo in Beats Per Minute
    pub bpm: u32,
    /// 7. Musical key signature (e.g. "A Major", "G Minor", "C Major")
    pub key_signature: String,
    /// 8. Optional path to vocal reference WAV for timbre cloning / Seed-VC conditioning
    pub vocal_ref_wav: Option<PathBuf>,
    /// 9. Optional path to master reference WAV for Matchering EQ profiling
    pub master_ref_wav: Option<PathBuf>,
    /// 10. Map of isolated audio stem paths
    pub stem_paths: HashMap<String, PathBuf>,
    /// 11. Audio spectral and waveform telemetry inspection
    pub telemetry: StemTelemetry,
    /// 12. Multi-LLM Ollama Judge composite score (0.0 to 10.0 or 0.0 to 1.0)
    pub judge_score: f32,
    /// 13. Quality gate flag: true if Judge score >= threshold and telemetry clean
    pub quality_passed: bool,
    /// 14. Count of retry iterations attempted when quality gate fails
    pub retry_count: u8,

    // Universal Multi-Genre & Pipeline Tracking Extensions
    /// 15. Primary musical genre name
    pub genre: String,
    /// 16. Musical time signature (e.g. "4/4", "3/4", "6/8")
    pub time_signature: String,
    /// 17. Complete song blueprint with bar-by-bar arrangement, chords, mix parameters
    pub blueprint: Option<SongBlueprint>,
    /// 18. Current or last active pipeline node name
    pub active_node: Option<String>,
    /// 19. Audit log of node transitions and execution events
    pub history: Vec<String>,
    /// 20. ISO 8601 creation timestamp
    pub created_at: String,
    /// 21. ISO 8601 last update timestamp
    pub updated_at: String,
}

impl Default for MusicProductionState {
    fn default() -> Self {
        Self::default_reggae_anthem()
    }
}

impl MusicProductionState {
    /// Creates a new state container with basic metadata.
    pub fn new(song_id: &str, genre: &str, bpm: u32, key_signature: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            song_id: song_id.to_string(),
            version: 1,
            engine_mode: "auto".to_string(),
            style_prompt: String::new(),
            lyrics: String::new(),
            bpm,
            key_signature: key_signature.to_string(),
            vocal_ref_wav: None,
            master_ref_wav: None,
            stem_paths: HashMap::new(),
            telemetry: StemTelemetry::default(),
            judge_score: 0.0,
            quality_passed: false,
            retry_count: 0,
            genre: genre.to_string(),
            time_signature: "4/4".to_string(),
            blueprint: None,
            active_node: None,
            history: vec![format!("Initialized song {} at {}", song_id, now)],
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Pre-loaded flagship preset: 1977 Roots Reggae "One Love Shining" Anthem at 76 BPM in A Major.
    pub fn default_reggae_anthem() -> Self {
        let bp = SongBlueprint::default_one_love_shining_anthem();
        let lyrics = bp.to_yue_lyrics_txt();
        let style = bp.style_of_music.clone();
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            song_id: bp.song_id.clone(),
            version: bp.version,
            engine_mode: "auto".to_string(),
            style_prompt: style,
            lyrics,
            bpm: bp.bpm.round() as u32,
            key_signature: bp.key_signature.clone(),
            vocal_ref_wav: None,
            master_ref_wav: None,
            stem_paths: HashMap::new(),
            telemetry: StemTelemetry::default(),
            judge_score: 0.0,
            quality_passed: false,
            retry_count: 0,
            genre: "Roots Reggae".to_string(),
            time_signature: bp.time_signature.clone(),
            blueprint: Some(bp),
            active_node: None,
            history: vec![format!("Loaded 1977 Roots Reggae Anthem preset at {}", now)],
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Creates a state directly from a `SongBlueprint`.
    pub fn from_blueprint(bp: &SongBlueprint) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            song_id: bp.song_id.clone(),
            version: bp.version,
            engine_mode: "auto".to_string(),
            style_prompt: bp.style_of_music.clone(),
            lyrics: bp.to_yue_lyrics_txt(),
            bpm: bp.bpm.round() as u32,
            key_signature: bp.key_signature.clone(),
            vocal_ref_wav: None,
            master_ref_wav: None,
            stem_paths: HashMap::new(),
            telemetry: StemTelemetry::default(),
            judge_score: 0.0,
            quality_passed: false,
            retry_count: 0,
            genre: bp.style_of_music.split(',').next().unwrap_or("General").trim().to_string(),
            time_signature: bp.time_signature.clone(),
            blueprint: Some(bp.clone()),
            active_node: None,
            history: vec![format!("Created state from blueprint {} v{} at {}", bp.song_id, bp.version, now)],
            created_at: bp.created_at.clone(),
            updated_at: now,
        }
    }

    /// Advances the song version number and refreshes timestamp.
    pub fn advance_version(&mut self) {
        self.version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.history.push(format!("Advanced to version {} at {}", self.version, self.updated_at));
    }

    /// Registers or updates an audio stem file path.
    pub fn set_stem(&mut self, stem_name: impl Into<String>, path: impl Into<PathBuf>) {
        let name = stem_name.into();
        let p = path.into();
        self.stem_paths.insert(name.clone(), p);
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.history.push(format!("Registered stem '{}'", name));
    }

    /// Retrieves an audio stem file path by name.
    pub fn get_stem(&self, stem_name: &str) -> Option<&PathBuf> {
        self.stem_paths.get(stem_name)
    }

    /// Checks if a named stem is present in the state.
    pub fn has_stem(&self, stem_name: &str) -> bool {
        self.stem_paths.contains_key(stem_name)
    }

    /// Removes an audio stem file path.
    pub fn remove_stem(&mut self, stem_name: &str) -> Option<PathBuf> {
        self.stem_paths.remove(stem_name)
    }

    /// Checks if all specified stem keys exist.
    pub fn has_required_stems(&self, stems: &[&str]) -> bool {
        stems.iter().all(|s| self.stem_paths.contains_key(*s))
    }

    /// Updates telemetry metrics and checks for clipping.
    pub fn set_telemetry(&mut self, telemetry: StemTelemetry) {
        self.telemetry = telemetry;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Updates Ollama judge evaluation score and pass status.
    pub fn set_judge_result(&mut self, score: f32, passed: bool) {
        self.judge_score = score;
        self.quality_passed = passed;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.history.push(format!("Judge score: {:.2}, passed: {}", score, passed));
    }

    /// Increments retry count and returns new value.
    pub fn increment_retry(&mut self) -> u8 {
        self.retry_count = self.retry_count.saturating_add(1);
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.history.push(format!("Incremented retry_count to {}", self.retry_count));
        self.retry_count
    }

    /// Resets retry count to 0.
    pub fn reset_retry(&mut self) {
        self.retry_count = 0;
    }

    /// Returns true if retry count is below the maximum allowed retries.
    pub fn can_retry(&self, max_retries: u8) -> bool {
        self.retry_count < max_retries
    }

    /// Records entry into a pipeline graph node.
    pub fn record_node_transition(&mut self, node_name: &str) {
        self.active_node = Some(node_name.to_string());
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.history.push(format!("Entered node '{}' at {}", node_name, self.updated_at));
    }

    /// Sets the engine mode.
    pub fn set_engine_mode(&mut self, mode: &str) {
        self.engine_mode = EngineMode::from_str_loose(mode).as_str().to_string();
    }

    /// Returns true if current engine mode prefers pure-Rust synthesis.
    pub fn is_pure_rust(&self) -> bool {
        EngineMode::from_str_loose(&self.engine_mode) == EngineMode::PureRust
    }

    /// Returns true if current engine mode prefers Hugging Face GPU checkpoints.
    pub fn is_hf_hybrid(&self) -> bool {
        EngineMode::from_str_loose(&self.engine_mode) == EngineMode::HfHybrid
    }

    /// Serializes state to pretty JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(MusicCoreError::from)
    }

    /// Deserializes state from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(MusicCoreError::from)
    }

    /// Saves state to disk file.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads state from disk file.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }
}
```

---

### 4.5 `crates/music_core/src/genre.rs`
```rust
use serde::{Deserialize, Serialize};
use crate::schema::StemMixConfig;

/// Broad genre family categories spanning world music.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenreCategory {
    RootsReggaeDub,
    AfrobeatHighlifeAmapiano,
    RnbSoulGospel,
    HipHopTrap,
    BluesRockMetal,
    JazzFusion,
    ClassicalCinematic,
    PopElectronic,
    LatinCaribbean,
    FolkCountryWorld,
    ExperimentalHybrid,
}

impl GenreCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            GenreCategory::RootsReggaeDub => "Roots Reggae & Dub",
            GenreCategory::AfrobeatHighlifeAmapiano => "Afrobeats, Highlife & Amapiano",
            GenreCategory::RnbSoulGospel => "R&B, Soul & Gospel",
            GenreCategory::HipHopTrap => "Hip-Hop & Trap",
            GenreCategory::BluesRockMetal => "Blues, Rock & Metal",
            GenreCategory::JazzFusion => "Jazz & Fusion",
            GenreCategory::ClassicalCinematic => "Classical & Cinematic",
            GenreCategory::PopElectronic => "Pop & Electronic",
            GenreCategory::LatinCaribbean => "Latin & Caribbean",
            GenreCategory::FolkCountryWorld => "Folk, Country & World",
            GenreCategory::ExperimentalHybrid => "Experimental & Hybrid",
        }
    }
}

/// Musical modal scales across Western and global traditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalScale {
    Ionian,             // Major
    Dorian,             // Minor with natural 6th
    Phrygian,           // Minor with flat 2nd
    Lydian,             // Major with raised 4th
    Mixolydian,         // Major with flat 7th
    Aeolian,            // Natural Minor
    Locrian,            // Diminished
    HarmonicMinor,      // Minor with raised 7th
    MelodicMinor,       // Minor with raised 6th and 7th
    MajorPentatonic,    // 1 2 3 5 6
    MinorPentatonic,    // 1 b3 4 5 b7
    BluesScale,         // Minor Pentatonic + b5 blue note
    ArabicHijaz,        // Phrygian Dominant (1 b2 3 4 5 b6 b7)
    JapaneseHirajoshi,  // 1 2 b3 5 b6
    SpanishGypsy,       // Phrygian Dominant with harmonic flares
    RagaBhairav,        // Indian Classical morning raga
}

impl ModalScale {
    pub fn name(&self) -> &'static str {
        match self {
            ModalScale::Ionian => "Major (Ionian)",
            ModalScale::Dorian => "Dorian",
            ModalScale::Phrygian => "Phrygian",
            ModalScale::Lydian => "Lydian",
            ModalScale::Mixolydian => "Mixolydian",
            ModalScale::Aeolian => "Natural Minor (Aeolian)",
            ModalScale::Locrian => "Locrian",
            ModalScale::HarmonicMinor => "Harmonic Minor",
            ModalScale::MelodicMinor => "Melodic Minor",
            ModalScale::MajorPentatonic => "Major Pentatonic",
            ModalScale::MinorPentatonic => "Minor Pentatonic",
            ModalScale::BluesScale => "Blues Scale",
            ModalScale::ArabicHijaz => "Arabic Maqam Hijaz",
            ModalScale::JapaneseHirajoshi => "Japanese Hirajoshi",
            ModalScale::SpanishGypsy => "Spanish Gypsy (Phrygian Dominant)",
            ModalScale::RagaBhairav => "Raga Bhairav",
        }
    }
}

/// Rhythmic groove foundation patterns for drum and rhythm synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RhythmGroove {
    OneDrop,            // Reggae: kick & snare on beat 3, 16th-note hi-hat, offbeat skank
    Steppers,           // Reggae: kick on 1, 2, 3, 4 with syncopated dub accents
    FourOnTheFloor,     // House/EDM: kick on 1, 2, 3, 4, offbeat open hat
    BoomBap,            // Hip-Hop: swung 16th hats, kick on 1 and offbeats, snare on 2 and 4
    Trap808,            // Trap: pitch-sliding 808 sub kick, rapid 32nd triplet hi-hat rolls
    AfrobeatTresillo,   // Afrobeats: 3:2 Tresillo syncopation, polyrhythmic shekere & congas
    AmapianoLogDrum,    // Amapiano: syncopated percussive bass log-drums, shaker shuffle
    HighlifeSyncopation,// Highlife: interlocking acoustic guitars, two-finger groove, clave bell
    RockBackbeat,       // Rock: heavy kick on 1 & 3, snare on 2 & 4, driving 8th-note cymbals
    MetalDoubleKick,    // Metal: rapid double-kick patterns, blast beats, tight palm-mute gallop
    JazzSwingRide,      // Jazz: triplet swing ride cymbal ding-ding-a-ding, brushed snare comping
    BossaNovaClave,     // Bossa Nova: nylon string syncopated chords, gentle rimshot clave
    SalsaCascara,       // Salsa: 2:3 or 3:2 son clave, cascara timbales, piano montuno
    CinematicPercussion,// Orchestral: heavy taiko hits, orchestral bass drum, timpani rolls
    WaltzThreeFour,     // 3/4 time: strong 1, light 2 and 3 waltz pulse
    GospelShuffle,      // Gospel: 6/8 or 12/8 swung triplet shuffle, tambourine on 2 and 4
}

/// Complete configuration preset for a musical genre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenrePreset {
    pub id: String,
    pub name: String,
    pub category: GenreCategory,
    pub subgenres: Vec<String>,
    pub default_bpm: u32,
    pub bpm_range: (u32, u32),
    pub default_time_signature: String,
    pub default_key: String,
    pub recommended_scale: ModalScale,
    pub rhythm_groove: RhythmGroove,
    pub description: String,
    pub style_tags: Vec<String>,
    pub recommended_instrumentation: Vec<String>,
    pub vocal_archetype: String,
    pub mastering_profile: String,
    pub stem_mix: StemMixConfig,
}

/// Universal taxonomy of all human musical genres and presets.
pub struct GenreTaxonomy;

impl GenreTaxonomy {
    /// Returns the comprehensive list of 20+ world music presets.
    pub fn all_presets() -> Vec<GenrePreset> {
        vec![
            Self::flagship_reggae_anthem(),
            Self::afrobeats_hit(),
            Self::amapiano_groove(),
            Self::highlife_golden(),
            Self::classic_soul_rnb(),
            Self::traditional_gospel(),
            Self::boom_bap_hip_hop(),
            Self::modern_trap(),
            Self::delta_blues(),
            Self::psychedelic_rock(),
            Self::heavy_metal(),
            Self::bebop_jazz(),
            Self::cinematic_orchestral(),
            Self::modern_pop(),
            Self::club_house_edm(),
            Self::latin_salsa(),
            Self::bossa_nova(),
            Self::americana_country(),
            Self::arabic_maqam_fusion(),
            Self::ambient_chillout(),
        ]
    }

    /// Primary Flagship Preset: 1977 Roots Reggae "One Love" Anthem at 76 BPM in A Major.
    pub fn flagship_reggae_anthem() -> GenrePreset {
        GenrePreset {
            id: "roots_reggae_1977".to_string(),
            name: "1977 Roots Reggae Anthem".to_string(),
            category: GenreCategory::RootsReggaeDub,
            subgenres: vec![
                "Roots Reggae".to_string(),
                "Dub".to_string(),
                "Rocksteady".to_string(),
                "Nyabinghi".to_string(),
            ],
            default_bpm: 76,
            bpm_range: (68, 84),
            default_time_signature: "4/4".to_string(),
            default_key: "A Major".to_string(),
            recommended_scale: ModalScale::Mixolydian,
            rhythm_groove: RhythmGroove::OneDrop,
            description: "1977 roots reggae anthem with soulful raspy Jamaican male tenor vocals, hypnotic one drop drumming, virtuosic blues-rock guitar solo, call-and-response gospel choir, and warm analog tape production.".to_string(),
            style_tags: vec![
                "1977 roots reggae anthem".to_string(),
                "soulful raspy Jamaican male tenor vocals".to_string(),
                "spiritual and uplifting".to_string(),
                "76 BPM".to_string(),
                "hypnotic one drop master drumming".to_string(),
                "complex polyrhythmic snare rolls".to_string(),
                "deep sub bass".to_string(),
                "virtuosic soaring psychedelic blues rock electric guitar solos".to_string(),
                "warm analog tape production".to_string(),
                "lush gospel backing choir".to_string(),
            ],
            recommended_instrumentation: vec![
                "One-drop drum kit with rimshot snare".to_string(),
                "Fender Jazz Bass with flatwound strings".to_string(),
                "Fender Stratocaster electric guitar with CryBaby wah".to_string(),
                "Hammond B3 organ with Leslie rotating speaker".to_string(),
                "Nyabinghi hand drums (funde and kete)".to_string(),
                "Brass horn section (tenor sax, trumpet, trombone)".to_string(),
                "Female gospel backing choir".to_string(),
            ],
            vocal_archetype: "Soulful Raspy Jamaican Male Tenor".to_string(),
            mastering_profile: "1977 Studer A800 2-Inch Analog Tape Reference".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn afrobeats_hit() -> GenrePreset {
        GenrePreset {
            id: "afrobeats".to_string(),
            name: "Modern Afrobeats Hit".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Afrobeats".to_string(), "Afro-Fusion".to_string(), "Afro-Pop".to_string()],
            default_bpm: 105,
            bpm_range: (96, 115),
            default_time_signature: "4/4".to_string(),
            default_key: "F# Minor".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::AfrobeatTresillo,
            description: "Contemporary West African Afrobeats with infectious 3:2 syncopation, warm basslines, bright marimba/electric piano chords, and vocal hooks.".to_string(),
            style_tags: vec![
                "modern afrobeats hit".to_string(),
                "infectious 3:2 syncopation".to_string(),
                "warm punchy 808 bass".to_string(),
                "clean muted electric guitar riffs".to_string(),
                "shaker rolls and conga fills".to_string(),
            ],
            recommended_instrumentation: vec![
                "Syncopated kick and snare rim".to_string(),
                "Shekere and shaker loop".to_string(),
                "Sub bass".to_string(),
                "Clean stratocaster riffs".to_string(),
                "Bright Rhodes/Marimba".to_string(),
            ],
            vocal_archetype: "Smooth West African Melodic Tenor/Baritone".to_string(),
            mastering_profile: "Modern Punchy Streaming Pop Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.0,
                gospel_choir_db: -2.0,
                drums_db: 1.5,
                dub_bass_db: 2.0,
                guitar_db: 0.5,
                organ_brass_db: -1.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 2.8,
                vocal_rasp_intensity: 0.20,
                stereo_width: 1.15,
                target_lufs: -12.0,
            },
        }
    }

    pub fn amapiano_groove() -> GenrePreset {
        GenrePreset {
            id: "amapiano".to_string(),
            name: "South African Amapiano".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Amapiano".to_string(), "Kwaito".to_string(), "Deep House".to_string()],
            default_bpm: 113,
            bpm_range: (108, 118),
            default_time_signature: "4/4".to_string(),
            default_key: "C Minor".to_string(),
            recommended_scale: ModalScale::Aeolian,
            rhythm_groove: RhythmGroove::AmapianoLogDrum,
            description: "Deep South African Amapiano featuring syncopated pitch-bent percussive log drums, jazzy Rhodes piano chords, shaker shuffle, and soulful toplines.".to_string(),
            style_tags: vec![
                "south african amapiano".to_string(),
                "pitch bent sub log drums".to_string(),
                "jazzy rhodes piano chords".to_string(),
                "laid-back shaker shuffle".to_string(),
                "soulful vocal chants".to_string(),
            ],
            recommended_instrumentation: vec![
                "Log drum synthesizer".to_string(),
                "Fender Rhodes electric piano".to_string(),
                "Shaker shuffle".to_string(),
                "Congas and rimshot".to_string(),
            ],
            vocal_archetype: "Soulful Chanted Topline with Harmonies".to_string(),
            mastering_profile: "Club Sound System Deep Sub Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: 0.0,
                drums_db: 1.8,
                dub_bass_db: 3.5,
                guitar_db: -2.0,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.25,
                sub_bass_boost_db: 4.5,
                vocal_rasp_intensity: 0.15,
                stereo_width: 1.20,
                target_lufs: -11.0,
            },
        }
    }

    pub fn highlife_golden() -> GenrePreset {
        GenrePreset {
            id: "highlife".to_string(),
            name: "Classic Highlife".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Highlife".to_string(), "Guitar Band Highlife".to_string(), "Palm-Wine".to_string()],
            default_bpm: 118,
            bpm_range: (110, 126),
            default_time_signature: "4/4".to_string(),
            default_key: "G Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::HighlifeSyncopation,
            description: "Joyful Ghanaian & Nigerian Highlife with interlocking dual clean electric guitars, brass fanfare, clave bell, and harmonized vocals.".to_string(),
            style_tags: vec![
                "classic highlife".to_string(),
                "interlocking guitar melodies".to_string(),
                "brass fanfare".to_string(),
                "clave bell groove".to_string(),
                "harmonized joyful vocals".to_string(),
            ],
            recommended_instrumentation: vec![
                "Twin clean electric guitars".to_string(),
                "Brass horns".to_string(),
                "Bass guitar".to_string(),
                "Clave bell and congas".to_string(),
            ],
            vocal_archetype: "Harmonized Melodic Tenor Duo".to_string(),
            mastering_profile: "Vintage Warm Brass Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn classic_soul_rnb() -> GenrePreset {
        GenrePreset {
            id: "soul_rnb".to_string(),
            name: "Classic Soul & R&B".to_string(),
            category: GenreCategory::RnbSoulGospel,
            subgenres: vec!["Soul".to_string(), "Motown".to_string(), "Neo-Soul".to_string(), "R&B".to_string()],
            default_bpm: 84,
            bpm_range: (70, 96),
            default_time_signature: "4/4".to_string(),
            default_key: "E Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::GospelShuffle,
            description: "Deep, heartfelt 1960s/70s soul with expressive vocal delivery, lush strings, vibrant horns, rhythmic bass, and tambourine groove.".to_string(),
            style_tags: vec![
                "classic deep soul".to_string(),
                "passionate vocal delivery".to_string(),
                "lush orchestral strings".to_string(),
                "punchy brass swells".to_string(),
                "motown bassline".to_string(),
            ],
            recommended_instrumentation: vec![
                "Drums with tambourine".to_string(),
                "P-Bass".to_string(),
                "Wurlitzer/Rhodes".to_string(),
                "Horn section".to_string(),
                "String quartet".to_string(),
            ],
            vocal_archetype: "Passionate Gritty Soul Belter".to_string(),
            mastering_profile: "1960s Motown Analog Tape Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn traditional_gospel() -> GenrePreset {
        GenrePreset {
            id: "gospel".to_string(),
            name: "Uplifting Gospel Anthem".to_string(),
            category: GenreCategory::RnbSoulGospel,
            subgenres: vec!["Gospel".to_string(), "Praise & Worship".to_string(), "Spirituals".to_string()],
            default_bpm: 72,
            bpm_range: (60, 88),
            default_time_signature: "6/8".to_string(),
            default_key: "Ab Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::GospelShuffle,
            description: "Soaring, inspirational gospel anthem featuring mass choir call-and-response, acoustic grand piano, Hammond B3 organ swells, and dramatic crescendos.".to_string(),
            style_tags: vec![
                "uplifting gospel anthem".to_string(),
                "mass choir harmonies".to_string(),
                "hammond b3 organ swells".to_string(),
                "grand piano flourishes".to_string(),
                "dynamic crescendo".to_string(),
            ],
            recommended_instrumentation: vec![
                "Grand piano".to_string(),
                "Hammond B3 organ".to_string(),
                "Drums with marching snare accents".to_string(),
                "Bass guitar".to_string(),
                "Mass choir".to_string(),
            ],
            vocal_archetype: "Powerful Gospel Soloist & Choir".to_string(),
            mastering_profile: "Dynamic High-Headroom Choral Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.0,
                gospel_choir_db: 3.0,
                drums_db: 0.0,
                dub_bass_db: 1.0,
                guitar_db: -1.0,
                organ_brass_db: 1.5,
                analog_tape_warmth: 0.65,
                tape_echo_wet: 0.30,
                sub_bass_boost_db: 2.0,
                vocal_rasp_intensity: 0.50,
                stereo_width: 1.30,
                target_lufs: -14.0,
            },
        }
    }

    pub fn boom_bap_hip_hop() -> GenrePreset {
        GenrePreset {
            id: "boom_bap".to_string(),
            name: "90s Golden Era Boom-Bap".to_string(),
            category: GenreCategory::HipHopTrap,
            subgenres: vec!["Boom-Bap".to_string(), "East Coast Hip-Hop".to_string(), "Lo-Fi Hip-Hop".to_string()],
            default_bpm: 92,
            bpm_range: (84, 98),
            default_time_signature: "4/4".to_string(),
            default_key: "D Minor".to_string(),
            recommended_scale: ModalScale::MinorPentatonic,
            rhythm_groove: RhythmGroove::BoomBap,
            description: "Gritty 1990s New York boom-bap with crisp chopped vinyl drum breaks, filtered jazz upright bass, vinyl crackle, and rhyming cadence.".to_string(),
            style_tags: vec![
                "90s boom-bap hip hop".to_string(),
                "hard hitting drum break".to_string(),
                "filtered jazz sample".to_string(),
                "upright bass chop".to_string(),
                "rhythmic lyric flow".to_string(),
            ],
            recommended_instrumentation: vec![
                "Sampled acoustic drum break".to_string(),
                "Upright acoustic bass".to_string(),
                "Chopped jazz piano/horns".to_string(),
                "Turntable scratch accents".to_string(),
            ],
            vocal_archetype: "Crisp Articulate Rapping Cadence".to_string(),
            mastering_profile: "Punchy Vinyl & Tape Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 3.0,
                gospel_choir_db: -6.0,
                drums_db: 2.5,
                dub_bass_db: 2.0,
                guitar_db: -1.0,
                organ_brass_db: -1.0,
                analog_tape_warmth: 0.85,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 3.0,
                vocal_rasp_intensity: 0.40,
                stereo_width: 1.05,
                target_lufs: -11.5,
            },
        }
    }

    pub fn modern_trap() -> GenrePreset {
        GenrePreset {
            id: "trap".to_string(),
            name: "Modern 808 Trap".to_string(),
            category: GenreCategory::HipHopTrap,
            subgenres: vec!["Trap".to_string(), "Drill".to_string(), "Dark Trap".to_string()],
            default_bpm: 140,
            bpm_range: (130, 160),
            default_time_signature: "4/4".to_string(),
            default_key: "F Minor".to_string(),
            recommended_scale: ModalScale::Phrygian,
            rhythm_groove: RhythmGroove::Trap808,
            description: "Dark, hard-hitting Atlanta trap with sliding 808 sub-bass, rapid-fire triplet hi-hat rolls, atmospheric bells, and aggressive melodic cadence.".to_string(),
            style_tags: vec![
                "modern trap hit".to_string(),
                "distorted 808 bass slides".to_string(),
                "triplet hi-hat rolls".to_string(),
                "minor key bells and synths".to_string(),
            ],
            recommended_instrumentation: vec![
                "808 Sub-bass".to_string(),
                "Trap drum kit with snappy snare".to_string(),
                "Glockenspiel/Bells synth".to_string(),
                "Reverse reverb pads".to_string(),
            ],
            vocal_archetype: "Autotuned Melodic Trap Flow".to_string(),
            mastering_profile: "Loud Modern Commercial Trap Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.5,
                gospel_choir_db: -4.0,
                drums_db: 2.0,
                dub_bass_db: 4.0,
                guitar_db: -3.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 5.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.25,
                target_lufs: -9.0,
            },
        }
    }

    pub fn delta_blues() -> GenrePreset {
        GenrePreset {
            id: "delta_blues".to_string(),
            name: "Delta & Electric Blues".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Delta Blues".to_string(), "Chicago Blues".to_string(), "Blues Rock".to_string()],
            default_bpm: 78,
            bpm_range: (65, 96),
            default_time_signature: "4/4".to_string(),
            default_key: "E Blues".to_string(),
            recommended_scale: ModalScale::BluesScale,
            rhythm_groove: RhythmGroove::RockBackbeat,
            description: "Raw, soulful American blues with overdriven valve-amp electric guitar bends, crying harmonica, walking bass, and gravelly vocals.".to_string(),
            style_tags: vec![
                "raw electric blues".to_string(),
                "soaring guitar bends".to_string(),
                "valve amplifier saturation".to_string(),
                "blues harp harmonica".to_string(),
                "gravelly emotional vocal".to_string(),
            ],
            recommended_instrumentation: vec![
                "Gibson Les Paul / Stratocaster through tube amp".to_string(),
                "Blues harp (harmonica)".to_string(),
                "Upright or electric bass".to_string(),
                "Minimal drum kit".to_string(),
            ],
            vocal_archetype: "Raw Gravelly Blues Baritone/Tenor".to_string(),
            mastering_profile: "1950s Tube Amp & Tape Saturation Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn psychedelic_rock() -> GenrePreset {
        GenrePreset {
            id: "psychedelic_rock".to_string(),
            name: "1970s Stadium Psychedelic Rock".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Psychedelic Rock".to_string(), "Classic Rock".to_string(), "Progressive Rock".to_string()],
            default_bpm: 116,
            bpm_range: (95, 132),
            default_time_signature: "4/4".to_string(),
            default_key: "D Major".to_string(),
            recommended_scale: ModalScale::Mixolydian,
            rhythm_groove: RhythmGroove::RockBackbeat,
            description: "Expansive 1970s stadium psychedelic rock with virtuosic soaring guitar solos, phaser and wah effects, driving bass, and soaring rock vocals.".to_string(),
            style_tags: vec![
                "1970s stadium rock".to_string(),
                "virtuosic electric guitar solo".to_string(),
                "crybaby wah pedal".to_string(),
                "heavy drum fills".to_string(),
                "soaring vocal belts".to_string(),
            ],
            recommended_instrumentation: vec![
                "Lead electric guitar with overdrive and delay".to_string(),
                "Rhythm electric guitar".to_string(),
                "Rickenbacker / Precision bass".to_string(),
                "Rock drum kit with heavy crash cymbals".to_string(),
                "Mellotron / analog synth pads".to_string(),
            ],
            vocal_archetype: "Soaring Rock Tenor with Natural Grit".to_string(),
            mastering_profile: "1970s Stadium Analog Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -1.0,
                drums_db: 2.0,
                dub_bass_db: 1.5,
                guitar_db: 3.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.85,
                tape_echo_wet: 0.35,
                sub_bass_boost_db: 2.0,
                vocal_rasp_intensity: 0.70,
                stereo_width: 1.25,
                target_lufs: -12.0,
            },
        }
    }

    pub fn heavy_metal() -> GenrePreset {
        GenrePreset {
            id: "heavy_metal".to_string(),
            name: "Symphonic Heavy Metal".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Heavy Metal".to_string(), "Thrash Metal".to_string(), "Symphonic Metal".to_string()],
            default_bpm: 138,
            bpm_range: (110, 180),
            default_time_signature: "4/4".to_string(),
            default_key: "E Minor".to_string(),
            recommended_scale: ModalScale::HarmonicMinor,
            rhythm_groove: RhythmGroove::MetalDoubleKick,
            description: "High-octane heavy metal featuring thunderous double-kick drumming, palm-muted high-gain guitar riffs, sweeping neo-classical solos, and operatic rock vocals.".to_string(),
            style_tags: vec![
                "symphonic heavy metal".to_string(),
                "rapid double kick drums".to_string(),
                "high gain palm muted riffs".to_string(),
                "sweeping arpeggio guitar solos".to_string(),
                "operatic vocal range".to_string(),
            ],
            recommended_instrumentation: vec![
                "Down-tuned high-gain guitars".to_string(),
                "Heavy 5-string bass".to_string(),
                "Double-bass drum kit".to_string(),
                "Orchestral choir and strings".to_string(),
            ],
            vocal_archetype: "Powerful Operatic Rock Tenor or Aggressive Grunt".to_string(),
            mastering_profile: "High-Energy Wall of Sound Metal Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: 0.5,
                drums_db: 2.5,
                dub_bass_db: 1.5,
                guitar_db: 3.5,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 2.5,
                vocal_rasp_intensity: 0.85,
                stereo_width: 1.30,
                target_lufs: -10.0,
            },
        }
    }

    pub fn bebop_jazz() -> GenrePreset {
        GenrePreset {
            id: "jazz".to_string(),
            name: "Modal Jazz & Bebop".to_string(),
            category: GenreCategory::JazzFusion,
            subgenres: vec!["Bebop".to_string(), "Modal Jazz".to_string(), "Hard Bop".to_string(), "Fusion".to_string()],
            default_bpm: 180,
            bpm_range: (120, 240),
            default_time_signature: "4/4".to_string(),
            default_key: "Bb Major".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::JazzSwingRide,
            description: "Sophisticated jazz featuring complex harmonic substitutions, swinging triplet ride cymbal, acoustic walking upright bass, and virtuoso saxophone/trumpet improvisation.".to_string(),
            style_tags: vec![
                "acoustic modal jazz".to_string(),
                "swinging ride cymbal".to_string(),
                "walking upright bass".to_string(),
                "virtuosic saxophone solo".to_string(),
                "rich extended jazz chords".to_string(),
            ],
            recommended_instrumentation: vec![
                "Tenor saxophone".to_string(),
                "Acoustic upright bass".to_string(),
                "Steinway grand piano".to_string(),
                "Jazz drum kit with coated heads and brushes".to_string(),
            ],
            vocal_archetype: "Warm Intimate Jazz Crooner or Scat Soloist".to_string(),
            mastering_profile: "Audiophile High-Dynamic Range Natural Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: -6.0,
                drums_db: 0.0,
                dub_bass_db: 0.5,
                guitar_db: 0.0,
                organ_brass_db: 1.5,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.10,
                sub_bass_boost_db: 1.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.15,
                target_lufs: -16.0,
            },
        }
    }

    pub fn cinematic_orchestral() -> GenrePreset {
        GenrePreset {
            id: "cinematic".to_string(),
            name: "Epic Cinematic Orchestral Score".to_string(),
            category: GenreCategory::ClassicalCinematic,
            subgenres: vec!["Cinematic Score".to_string(), "Orchestral".to_string(), "Film Trailer".to_string()],
            default_bpm: 80,
            bpm_range: (50, 140),
            default_time_signature: "4/4".to_string(),
            default_key: "D Minor".to_string(),
            recommended_scale: ModalScale::HarmonicMinor,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Grand orchestral soundtrack with massive brass swells, sweeping string ensembles, thundering Taiko percussion, French horn themes, and choral chants.".to_string(),
            style_tags: vec![
                "epic cinematic score".to_string(),
                "sweeping symphonic strings".to_string(),
                "massive brass horns".to_string(),
                "thunderous taiko drums".to_string(),
                "choral chanting".to_string(),
            ],
            recommended_instrumentation: vec![
                "Full symphonic string section (violins, violas, cellos, basses)".to_string(),
                "French horn section & tubas".to_string(),
                "Taiko drums and orchestral bass drum".to_string(),
                "Timpani and concert cymbals".to_string(),
            ],
            vocal_archetype: "Epic Operatic Soloist / Choral Ensemble".to_string(),
            mastering_profile: "Ultra-Wide Dynamic Cinematic Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: 2.5,
                drums_db: 2.0,
                dub_bass_db: 2.5,
                guitar_db: -4.0,
                organ_brass_db: 3.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.35,
                sub_bass_boost_db: 3.5,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.40,
                target_lufs: -15.0,
            },
        }
    }

    pub fn modern_pop() -> GenrePreset {
        GenrePreset {
            id: "pop".to_string(),
            name: "Global Radio Pop Hit".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["Dance-Pop".to_string(), "Synthpop".to_string(), "Electropop".to_string()],
            default_bpm: 122,
            bpm_range: (112, 130),
            default_time_signature: "4/4".to_string(),
            default_key: "C Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: "Radio-ready dance-pop anthem with shimmering synthesizers, irresistible melodic earworm vocal hooks, punchy drums, and pristine modern production.".to_string(),
            style_tags: vec![
                "global radio pop hit".to_string(),
                "catchy vocal earworm".to_string(),
                "shimmering synth pads".to_string(),
                "driving four on the floor beat".to_string(),
            ],
            recommended_instrumentation: vec![
                "Synthesizer plucks and leads".to_string(),
                "Electronic drum samples".to_string(),
                "Sidechained sub-bass".to_string(),
                "Acoustic guitar strums in verses".to_string(),
            ],
            vocal_archetype: "Pristine Modern Pop Soprano/Tenor".to_string(),
            mastering_profile: "Bright Punchy Commercial Streaming Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.5,
                gospel_choir_db: -1.0,
                drums_db: 1.5,
                dub_bass_db: 2.0,
                guitar_db: 0.0,
                organ_brass_db: 0.0,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 2.5,
                vocal_rasp_intensity: 0.05,
                stereo_width: 1.20,
                target_lufs: -10.5,
            },
        }
    }

    pub fn club_house_edm() -> GenrePreset {
        GenrePreset {
            id: "house_edm".to_string(),
            name: "Peak-Time House & EDM".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["House".to_string(), "Tech House".to_string(), "EDM".to_string(), "Synthwave".to_string()],
            default_bpm: 126,
            bpm_range: (124, 132),
            default_time_signature: "4/4".to_string(),
            default_key: "G Minor".to_string(),
            recommended_scale: ModalScale::Aeolian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: "Peak-time club house music driven by a pumping 4/4 kick, rolling basslines, classic 909 open hi-hats, uplifting piano stabs, and soulful vocal chops.".to_string(),
            style_tags: vec![
                "peak time club house".to_string(),
                "pumping four on the floor kick".to_string(),
                "rolling bassline".to_string(),
                "classic 909 open hi-hat".to_string(),
                "uplifting house piano chords".to_string(),
            ],
            recommended_instrumentation: vec![
                "909 Drum machine".to_string(),
                "Analog saw-wave synth bass".to_string(),
                "Korg M1 piano stabs".to_string(),
                "White noise risers and crash cymbals".to_string(),
            ],
            vocal_archetype: "Soulful Powerful House Diva Topline".to_string(),
            mastering_profile: "Club Sound System Maximum Energy Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -0.5,
                drums_db: 2.5,
                dub_bass_db: 3.5,
                guitar_db: -4.0,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.25,
                sub_bass_boost_db: 4.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.25,
                target_lufs: -9.5,
            },
        }
    }

    pub fn latin_salsa() -> GenrePreset {
        GenrePreset {
            id: "salsa".to_string(),
            name: "Classic Fania-Style Salsa".to_string(),
            category: GenreCategory::LatinCaribbean,
            subgenres: vec!["Salsa Dura".to_string(), "Mambo".to_string(), "Son Cubano".to_string()],
            default_bpm: 186,
            bpm_range: (160, 210),
            default_time_signature: "4/4".to_string(),
            default_key: "A Minor".to_string(),
            recommended_scale: ModalScale::SpanishGypsy,
            rhythm_groove: RhythmGroove::SalsaCascara,
            description: "Electrifying New York/Puerto Rican salsa dura with syncopated piano montuno, punchy horn section, driving timbales cascara, baby bass tumbao, and sonero vocals.".to_string(),
            style_tags: vec![
                "classic salsa dura".to_string(),
                "piano montuno".to_string(),
                "punchy horn section".to_string(),
                "timbales cascara and bongo bell".to_string(),
                "sonero call and response".to_string(),
            ],
            recommended_instrumentation: vec![
                "Timbales, congas, and bongo".to_string(),
                "Baby bass (Ampeg electric upright)".to_string(),
                "Acoustic grand piano".to_string(),
                "Trombone and trumpet section".to_string(),
            ],
            vocal_archetype: "Dynamic Improvising Sonero Vocalist".to_string(),
            mastering_profile: "Bright Dynamic Latin Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn bossa_nova() -> GenrePreset {
        GenrePreset {
            id: "bossa_nova".to_string(),
            name: "Intimate Brazilian Bossa Nova".to_string(),
            category: GenreCategory::LatinCaribbean,
            subgenres: vec!["Bossa Nova".to_string(), "Samba".to_string(), "MPB".to_string()],
            default_bpm: 128,
            bpm_range: (115, 140),
            default_time_signature: "2/4".to_string(),
            default_key: "F Major".to_string(),
            recommended_scale: ModalScale::Lydian,
            rhythm_groove: RhythmGroove::BossaNovaClave,
            description: "Gentle, intimate Brazilian Bossa Nova with nylon-string classical guitar syncopation, whispering romantic vocals, soft brush drumming, and flute flourishes.".to_string(),
            style_tags: vec![
                "intimate bossa nova".to_string(),
                "nylon-string guitar syncopation".to_string(),
                "soft brushed snare".to_string(),
                "gentle whispering vocals".to_string(),
                "flute and piano embellishments".to_string(),
            ],
            recommended_instrumentation: vec![
                "Nylon-string acoustic guitar".to_string(),
                "Upright acoustic bass".to_string(),
                "Brushed snare and gentle rimshot".to_string(),
                "Concert flute".to_string(),
            ],
            vocal_archetype: "Intimate Whispering Soft Baritone/Alto".to_string(),
            mastering_profile: "Intimate Audiophile Acoustic Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -8.0,
                drums_db: -1.0,
                dub_bass_db: 0.0,
                guitar_db: 2.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 1.0,
                vocal_rasp_intensity: 0.05,
                stereo_width: 1.10,
                target_lufs: -15.0,
            },
        }
    }

    pub fn americana_country() -> GenrePreset {
        GenrePreset {
            id: "americana".to_string(),
            name: "Acoustic Bluegrass & Americana".to_string(),
            category: GenreCategory::FolkCountryWorld,
            subgenres: vec!["Americana".to_string(), "Bluegrass".to_string(), "Country".to_string(), "Folk".to_string()],
            default_bpm: 104,
            bpm_range: (80, 130),
            default_time_signature: "4/4".to_string(),
            default_key: "G Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::HighlifeSyncopation,
            description: "Heartfelt acoustic Americana featuring fingerpicked Martin dreadnought guitar, bluegrass fiddle, weeping pedal steel guitar, and authentic harmony singing.".to_string(),
            style_tags: vec![
                "acoustic americana bluegrass".to_string(),
                "fingerpicked acoustic guitar".to_string(),
                "bluegrass fiddle solo".to_string(),
                "weeping pedal steel guitar".to_string(),
                "two-part vocal harmony".to_string(),
            ],
            recommended_instrumentation: vec![
                "Acoustic guitar (steel string)".to_string(),
                "Fiddle / Violin".to_string(),
                "Pedal steel guitar".to_string(),
                "Upright bass".to_string(),
                "Banjo / Mandolin".to_string(),
            ],
            vocal_archetype: "Warm Honest Acoustic Storyteller".to_string(),
            mastering_profile: "Natural Room Acoustic Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn arabic_maqam_fusion() -> GenrePreset {
        GenrePreset {
            id: "arabic_maqam".to_string(),
            name: "Middle Eastern Maqam Fusion".to_string(),
            category: GenreCategory::FolkCountryWorld,
            subgenres: vec!["Arabic Maqam".to_string(), "Oriental Fusion".to_string(), "Sufi".to_string()],
            default_bpm: 96,
            bpm_range: (80, 120),
            default_time_signature: "4/4".to_string(),
            default_key: "D Hijaz".to_string(),
            recommended_scale: ModalScale::ArabicHijaz,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Mystical Middle Eastern orchestral fusion with virtuosic Oud melodies, Ney flute, microtonal string runs, Darbuka / Riq percussion, and emotional vocal ornamentation.".to_string(),
            style_tags: vec![
                "middle eastern maqam fusion".to_string(),
                "virtuosic oud solos".to_string(),
                "breathy ney flute".to_string(),
                "darbuka and riq frame drums".to_string(),
                "microtonal vocal ornamentation".to_string(),
            ],
            recommended_instrumentation: vec![
                "Oud (Middle Eastern lute)".to_string(),
                "Ney flute".to_string(),
                "Qanun (plucked zither)".to_string(),
                "Darbuka & Riq percussion".to_string(),
                "Orchestral strings in unison".to_string(),
            ],
            vocal_archetype: "Expressive Microtonal Arabic Vocalist".to_string(),
            mastering_profile: "Rich Spatial Acoustic Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn ambient_chillout() -> GenrePreset {
        GenrePreset {
            id: "ambient".to_string(),
            name: "Deep Atmospheric Ambient".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["Ambient".to_string(), "Chillout".to_string(), "Drone".to_string(), "Space Music".to_string()],
            default_bpm: 60,
            bpm_range: (40, 80),
            default_time_signature: "4/4".to_string(),
            default_key: "C# Minor".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Meditative, immersive soundscape with lush analog synthesizer drones, gentle sub-bass pulses, ethereal vocal textures, and vast spatial reverbs.".to_string(),
            style_tags: vec![
                "deep atmospheric ambient".to_string(),
                "lush analog synthesizer drones".to_string(),
                "gentle sub bass pulse".to_string(),
                "ethereal vocal choir texture".to_string(),
                "vast spatial reverb".to_string(),
            ],
            recommended_instrumentation: vec![
                "Analog polyphonic synthesizers (Prophet, Juno)".to_string(),
                "Bowed electric guitar / E-Bow".to_string(),
                "Sub-bass drone oscillator".to_string(),
                "Reverb-drenched vocal pads".to_string(),
            ],
            vocal_archetype: "Ethereal Wordless Ambient Vocalist".to_string(),
            mastering_profile: "Expansive Immersive Reverb Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 0.0,
                gospel_choir_db: 2.0,
                drums_db: -4.0,
                dub_bass_db: 2.5,
                guitar_db: 0.5,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.70,
                tape_echo_wet: 0.50,
                sub_bass_boost_db: 3.5,
                vocal_rasp_intensity: 0.0,
                stereo_width: 1.45,
                target_lufs: -18.0,
            },
        }
    }

    /// Case-insensitive search across presets by id, name, or subgenre.
    pub fn find_preset(query: &str) -> Option<GenrePreset> {
        let q = query.trim().to_lowercase();
        let presets = Self::all_presets();

        // 1. Direct ID match
        if let Some(p) = presets.iter().find(|p| p.id.to_lowercase() == q) {
            return Some(p.clone());
        }
        // 2. Direct name match
        if let Some(p) = presets.iter().find(|p| p.name.to_lowercase() == q) {
            return Some(p.clone());
        }
        // 3. Subgenre match
        if let Some(p) = presets.iter().find(|p| {
            p.subgenres.iter().any(|s| s.to_lowercase() == q)
        }) {
            return Some(p.clone());
        }
        // 4. Substring contains match
        if let Some(p) = presets.iter().find(|p| {
            p.name.to_lowercase().contains(&q) || q.contains(&p.id.to_lowercase())
        }) {
            return Some(p.clone());
        }

        None
    }

    /// Intelligent fallback: Finds an existing preset, or dynamically constructs a complete, valid
    /// `GenrePreset` for ANY genre name known to humanity (e.g. "K-Pop", "Celtic Punk", "Flamenco Trap").
    pub fn find_or_create(query: &str) -> GenrePreset {
        if let Some(preset) = Self::find_preset(query) {
            return preset;
        }

        let q = query.trim().to_lowercase();
        let slug = q.replace(|c: char| !c.is_alphanumeric(), "_");

        // Keyword heuristics for auto-constructing presets
        if q.contains("reggae") || q.contains("dub") || q.contains("rasta") || q.contains("ska") {
            let mut p = Self::flagship_reggae_anthem();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("afro") || q.contains("amapiano") || q.contains("highlife") || q.contains("juju") {
            let mut p = Self::afrobeats_hit();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("soul") || q.contains("r&b") || q.contains("gospel") || q.contains("funk") {
            let mut p = Self::classic_soul_rnb();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("hip") || q.contains("hop") || q.contains("rap") || q.contains("trap") || q.contains("drill") {
            let mut p = Self::boom_bap_hip_hop();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("metal") || q.contains("rock") || q.contains("punk") || q.contains("grunge") {
            let mut p = Self::psychedelic_rock();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("jazz") || q.contains("swing") || q.contains("bebop") || q.contains("fusion") {
            let mut p = Self::bebop_jazz();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("classic") || q.contains("orchestr") || q.contains("cinematic") || q.contains("symphon") {
            let mut p = Self::cinematic_orchestral();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("house") || q.contains("edm") || q.contains("techno") || q.contains("dance") || q.contains("trance") {
            let mut p = Self::club_house_edm();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("latin") || q.contains("salsa") || q.contains("bossa") || q.contains("cumbia") || q.contains("reggaeton") {
            let mut p = Self::latin_salsa();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("country") || q.contains("folk") || q.contains("bluegrass") || q.contains("acoustic") {
            let mut p = Self::americana_country();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }

        // Generic intelligent fallback for any experimental or novel genre prompt
        GenrePreset {
            id: slug,
            name: query.to_string(),
            category: GenreCategory::ExperimentalHybrid,
            subgenres: vec![query.to_string()],
            default_bpm: 120,
            bpm_range: (60, 180),
            default_time_signature: "4/4".to_string(),
            default_key: "C Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: format!("Universal production profile for {}.", query),
            style_tags: vec![query.to_string(), "120 BPM".to_string(), "4/4 time".to_string()],
            recommended_instrumentation: vec![
                "Drums and percussion".to_string(),
                "Bass".to_string(),
                "Lead instrument".to_string(),
                "Harmony chords".to_string(),
            ],
            vocal_archetype: "Versatile Melodic Vocalist".to_string(),
            mastering_profile: "Balanced Dynamic Full-Spectrum Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }
}
```

---

### 4.6 `crates/music_core/src/blueprint.rs`
```rust
//! Re-exports of `SongBlueprint`, `SongSection`, `StemMixConfig`, and `PipelineModelStack`
//! from `schema.rs` to satisfy the project layout specification.

pub use crate::schema::{PipelineModelStack, SongBlueprint, SongSection, StemMixConfig};
```

---

### 4.7 `crates/music_core/src/lib.rs`
```rust
//! `music_core`: Core domain data structures, universal genre taxonomy, state management,
//! audio telemetry inspection, and song blueprints for the AI Music Producer Agent.

pub mod blueprint;
pub mod error;
pub mod genre;
pub mod schema;
pub mod state;
pub mod telemetry;

pub use blueprint::*;
pub use error::{MusicCoreError, Result};
pub use genre::{GenreCategory, GenrePreset, GenreTaxonomy, ModalScale, RhythmGroove};
pub use schema::{PipelineModelStack, SongBlueprint, SongSection, StemMixConfig};
pub use state::{
    EngineMode, MusicProductionState, STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER, STEM_RASPY_TENOR_VOCALS,
    STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
pub use telemetry::StemTelemetry;
```

---

### 4.8 `crates/music_core/tests/core_tests.rs` (Unit & Integration Tests)
```rust
use std::path::PathBuf;
use music_core::{
    EngineMode, GenreCategory, GenreTaxonomy, ModalScale, MusicProductionState, RhythmGroove,
    SongBlueprint, StemTelemetry, STEM_DRUMS, STEM_FINAL_MASTER,
};

#[test]
fn test_serde_music_production_state_roundtrip() {
    let mut state = MusicProductionState::new("test-song-01", "Afrobeats", 105, "F# Minor");
    state.set_stem(STEM_DRUMS, PathBuf::from("storage/drums.wav"));
    state.set_stem(STEM_FINAL_MASTER, PathBuf::from("storage/final_master.wav"));
    state.lyrics = "[intro]\nFeel the groove\n[verse]\nUnder the sun".to_string();
    state.style_prompt = "Modern Afrobeats, 105 BPM, warm 808 bass".to_string();
    state.judge_score = 8.5;
    state.quality_passed = true;
    state.retry_count = 1;

    let json = state.to_json().expect("Serialization to JSON failed");
    assert!(json.contains("test-song-01"));
    assert!(json.contains("Afrobeats"));

    let restored = MusicProductionState::from_json(&json).expect("Deserialization failed");
    assert_eq!(restored.song_id, "test-song-01");
    assert_eq!(restored.bpm, 105);
    assert_eq!(restored.key_signature, "F# Minor");
    assert_eq!(restored.judge_score, 8.5);
    assert!(restored.quality_passed);
    assert_eq!(restored.retry_count, 1);
    assert!(restored.has_stem(STEM_DRUMS));
    assert!(restored.has_stem(STEM_FINAL_MASTER));
}

#[test]
fn test_default_reggae_anthem_state() {
    let state = MusicProductionState::default_reggae_anthem();
    assert_eq!(state.song_id, "song-one-love-shining-1977");
    assert_eq!(state.bpm, 76);
    assert_eq!(state.key_signature, "A Major");
    assert_eq!(state.genre, "Roots Reggae");
    assert_eq!(state.engine_mode, "auto");
    assert_eq!(state.retry_count, 0);
    assert!(!state.lyrics.is_empty());
    assert!(state.style_prompt.contains("1977 roots reggae anthem"));
    assert!(state.blueprint.is_some());
}

#[test]
fn test_stem_mutation_and_versioning() {
    let mut state = MusicProductionState::new("v-song", "Rock", 120, "E Minor");
    assert_eq!(state.version, 1);
    state.advance_version();
    assert_eq!(state.version, 2);

    state.set_stem("guitar", "storage/guitar.wav");
    assert!(state.has_stem("guitar"));
    assert_eq!(
        state.get_stem("guitar"),
        Some(&PathBuf::from("storage/guitar.wav"))
    );

    assert_eq!(state.increment_retry(), 1);
    assert!(state.can_retry(2));
    assert_eq!(state.increment_retry(), 2);
    assert!(!state.can_retry(2));
}

#[test]
fn test_telemetry_broadcast_readiness() {
    let clean = StemTelemetry::default();
    assert!(clean.is_broadcast_ready());
    assert!(clean.validation_warnings().is_empty());

    let mut clipped = StemTelemetry::default();
    clipped.clipping = true;
    clipped.peak_dbfs = 0.5;
    assert!(!clipped.is_broadcast_ready());
    let warnings = clipped.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("clipping")));

    let mut drifting = StemTelemetry::default();
    drifting.tempo_drift_score = 0.08;
    assert!(!drifting.is_broadcast_ready());
    assert!(drifting.validation_warnings().iter().any(|w| w.contains("tempo drift")));
}

#[test]
fn test_genre_taxonomy_coverage() {
    let presets = GenreTaxonomy::all_presets();
    assert!(presets.len() >= 20);

    // Verify key categories represented
    assert!(presets.iter().any(|p| p.category == GenreCategory::RootsReggaeDub));
    assert!(presets.iter().any(|p| p.category == GenreCategory::AfrobeatHighlifeAmapiano));
    assert!(presets.iter().any(|p| p.category == GenreCategory::RnbSoulGospel));
    assert!(presets.iter().any(|p| p.category == GenreCategory::HipHopTrap));
    assert!(presets.iter().any(|p| p.category == GenreCategory::BluesRockMetal));
    assert!(presets.iter().any(|p| p.category == GenreCategory::JazzFusion));
    assert!(presets.iter().any(|p| p.category == GenreCategory::ClassicalCinematic));
    assert!(presets.iter().any(|p| p.category == GenreCategory::PopElectronic));
    assert!(presets.iter().any(|p| p.category == GenreCategory::LatinCaribbean));
    assert!(presets.iter().any(|p| p.category == GenreCategory::FolkCountryWorld));
}

#[test]
fn test_genre_lookup_and_intelligent_fallback() {
    let reggae = GenreTaxonomy::find_or_create("Roots Reggae");
    assert_eq!(reggae.id, "roots_reggae_1977");
    assert_eq!(reggae.default_bpm, 76);

    let afrobeats = GenreTaxonomy::find_or_create("Afrobeats");
    assert_eq!(afrobeats.id, "afrobeats");
    assert_eq!(afrobeats.default_bpm, 105);

    let amapiano = GenreTaxonomy::find_or_create("Amapiano");
    assert_eq!(amapiano.id, "amapiano");
    assert_eq!(amapiano.rhythm_groove, RhythmGroove::AmapianoLogDrum);

    // Dynamic unknown genre fallback
    let cyberpunk = GenreTaxonomy::find_or_create("Glitch Cyberpunk Synth");
    assert_eq!(cyberpunk.category, GenreCategory::PopElectronic); // matched 'synth'
    assert_eq!(cyberpunk.default_bpm, 126);

    let unknown = GenreTaxonomy::find_or_create("Xylophone Polka Fusion 3000");
    assert_eq!(unknown.category, GenreCategory::ExperimentalHybrid);
    assert_eq!(unknown.default_bpm, 120);
}

#[test]
fn test_blueprint_schema_calculations() {
    let bp = SongBlueprint::default_one_love_shining_anthem();
    assert_eq!(bp.total_bars(), 52);

    let duration = bp.estimated_duration_secs();
    // 52 bars * 4 beats * (60 / 76) ≈ 164.21 seconds
    assert!((duration - 164.21).abs() < 1.0);

    let genre_txt = bp.to_yue_genre_txt();
    assert!(genre_txt.contains("76 BPM"));
    assert!(genre_txt.contains("Key of A Major"));
    assert!(genre_txt.ends_with(".\n"));

    let lyrics_txt = bp.to_yue_lyrics_txt();
    assert!(lyrics_txt.contains("[intro]"));
    assert!(lyrics_txt.contains("[verse]"));
    assert!(lyrics_txt.contains("[chorus]"));
    assert!(lyrics_txt.contains("[guitar solo]"));
}
```

---

## 5. Verification Method

Once Worker 1 applies the code specifications detailed in Section 4:

1. **Clean Workspace Target Verification**:
   Execute:
   ```powershell
   cargo check -p music_core
   ```
   **Expected Outcome**: Compiles with zero errors and zero warnings.

2. **Full Unit Test Verification**:
   Execute:
   ```powershell
   cargo test -p music_core -- --nocapture
   ```
   **Expected Outcome**: All 7 integration and unit tests pass (`test_serde_music_production_state_roundtrip`, `test_default_reggae_anthem_state`, `test_stem_mutation_and_versioning`, `test_telemetry_broadcast_readiness`, `test_genre_taxonomy_coverage`, `test_genre_lookup_and_intelligent_fallback`, `test_blueprint_schema_calculations`).

3. **Downstream Interface Contract Verification**:
   Confirm that downstream crates (`music_fusion`, `music_dsp`, `music_pipeline`) can import:
   ```rust
   use music_core::{
       MusicProductionState, StemTelemetry, GenreTaxonomy, GenrePreset,
       SongBlueprint, StemMixConfig, EngineMode
   };
   ```
   without type conflicts or missing fields.

4. **Invalidation Conditions**:
   - Any omission of the 13 required fields in `MusicProductionState` invalidates the specification.
   - Any failure of `GenreTaxonomy::find_or_create` to return a valid preset for arbitrary user input invalidates the universal multi-genre requirement.
   - Any modification to `crates/music_core/src/schema.rs` that breaks existing `default_one_love_shining_anthem()` or `default_rise_up_together_anthem()` invalidates the reggae preset contract.
