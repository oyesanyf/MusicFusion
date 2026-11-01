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
    /// 12. Multi-LLM Ollama Judge composite score (0.0 to 10.0)
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

    /// Registers a stem (alias for set_stem).
    pub fn record_stem(&mut self, stem_name: &str, path: PathBuf) {
        self.set_stem(stem_name, path);
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
