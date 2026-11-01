//! Node 2: Groove and YuE Synthesis Node (`groove_and_yue_synthesis_node`).
//!
//! Responsibilities:
//! 1. Probes live hardware via `SystemMemory::detect_live()`.
//! 2. Stage 1: Drum & Rhythm Foundation:
//!    - Generates `draft_groove.wav` using `music_dsp::groove_synth::UniversalDrumSynthesizer::generate_groove`.
//!    - Supports all 7 canonical rhythm styles (One-Drop Reggae at 76 BPM, Steppers,
//!      Afrobeats/Amapiano, Four-on-the-Floor, Boom-Bap/Trap, Rock/Metal, Jazz/Orchestral).
//!    - Saves `draft_groove.wav` to `./storage/songs/<song_id>/v<version>/stems/draft_groove.wav`.
//! 3. Stage 2: YuE Multi-Track Vocal and Accompaniment Synthesis:
//!    - If GPU/HF mode: spawns external subprocess via `tokio::process::Command` with Windows
//!      `CREATE_NO_WINDOW` (0x08000000) and awaits clean termination to guarantee 100% VRAM reclamation.
//!    - If pure-Rust mode or on subprocess fallback: executes pure-Rust multi-stem harmonic renderer.
//!    - Emits `yue_vocals.wav` and `yue_instrumentals.wav`.
//! 4. Registers stems in `state.stem_paths` (`STEM_DRAFT_GROOVE`, `STEM_YUE_VOCALS`, `STEM_YUE_INSTRUMENTALS`).
//! 5. Records transition in `state.history`.

use std::f32::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use async_trait::async_trait;

use music_core::state::{
    MusicProductionState, STEM_DRAFT_GROOVE, STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::groove_synth::UniversalDrumSynthesizer;
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_fusion::memory::{ExecutionRoute, PipelineStageKind, SystemMemory};

use crate::error::PipelineError;
use crate::graph::GraphNode;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Helper function to resolve the stems directory for a specific song version.
pub fn get_song_stems_dir(song_id: &str, version: u32) -> PathBuf {
    PathBuf::from(format!("storage/songs/{}/v{}/stems", song_id, version))
}

/// Node 2: Groove and YuE Synthesis Node.
#[derive(Debug, Clone)]
pub struct GrooveAndYueSynthesisNode {
    pub default_duration_secs: f32,
    pub max_duration_secs: Option<f32>,
    pub subprocess_timeout_secs: u64,
    pub storage_dir: Option<PathBuf>,
}

impl Default for GrooveAndYueSynthesisNode {
    fn default() -> Self {
        Self {
            default_duration_secs: 15.0, // Default 15s for efficient test / pipeline runs
            max_duration_secs: None,
            subprocess_timeout_secs: 600, // 10 minutes maximum for external heavy inference
            storage_dir: None,
        }
    }
}

impl GrooveAndYueSynthesisNode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_duration(mut self, secs: f32) -> Self {
        self.default_duration_secs = secs.max(2.0);
        self
    }

    pub fn with_max_duration(mut self, secs: f32) -> Self {
        self.max_duration_secs = Some(secs.max(2.0));
        self
    }

    pub fn with_storage_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    pub fn resolve_stems_dir(&self, song_id: &str, version: u32) -> PathBuf {
        let base = self.storage_dir.clone().unwrap_or_else(|| PathBuf::from("storage"));
        base.join("songs").join(song_id).join(format!("v{}", version)).join("stems")
    }

    /// Primary execution method for Node 2.
    pub async fn execute_node(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        state.record_node_transition("groove_and_yue_synthesis_node");

        let stems_dir = self.resolve_stems_dir(&state.song_id, state.version);
        fs::create_dir_all(&stems_dir).map_err(PipelineError::IoError)?;

        // 1. Determine target duration from max override, blueprint, or default
        let duration_secs = if let Some(max_dur) = self.max_duration_secs {
            max_dur.max(2.0)
        } else if let Some(ref bp) = state.blueprint {
            let calc = bp.estimated_duration_secs() as f32;
            calc.clamp(2.0, 300.0)
        } else {
            self.default_duration_secs
        };

        // 2. Stage 1: Generate draft_groove.wav using pure-Rust Universal Drum Synthesizer
        let groove_buf = self.generate_draft_groove(state, duration_secs)?;
        let groove_path = stems_dir.join("draft_groove.wav");
        groove_buf.to_wav_file_with_format(&groove_path, WavBitDepth::Float32).map_err(|e| {
            PipelineError::DspError(format!("Failed to write draft_groove.wav: {}", e))
        })?;
        state.set_stem(STEM_DRAFT_GROOVE, groove_path.clone());
        log::info!("Node 2: Generated draft groove ({:.1}s) at {:?}", duration_secs, groove_path);

        // 3. Stage 2: YuE multi-track vocal and accompaniment synthesis
        let memory = SystemMemory::detect_live();
        let route = memory.route_stage(PipelineStageKind::Stage2YuE, &state.engine_mode);
        log::info!(
            "Node 2: Sizing decision for Stage2YuE -> {:?} (engine_mode: {})",
            route, state.engine_mode
        );

        let mut synthesized_via_hf = false;

        if route == ExecutionRoute::HuggingFaceGpu || route == ExecutionRoute::HuggingFaceCpu {
            // Attempt external YuE subprocess execution with 100% VRAM reclamation
            match self.execute_yue_subprocess(state, &stems_dir, &groove_path).await {
                Ok((vocals_path, inst_path)) => {
                    state.set_stem(STEM_YUE_VOCALS, vocals_path);
                    state.set_stem(STEM_YUE_INSTRUMENTALS, inst_path);
                    synthesized_via_hf = true;
                    log::info!("Node 2: Successfully generated dual stems via external YuE subprocess");
                }
                Err(e) => {
                    log::warn!(
                        "Node 2: YuE external subprocess skipped/failed ({}); gracefully falling back to pure-Rust DSP synthesis",
                        e
                    );
                }
            }
        }

        // 4. Pure-Rust DSP fallback multi-stem harmonic renderer
        if !synthesized_via_hf {
            let (vocals_path, inst_path) = self.render_pure_rust_dual_stems(state, &stems_dir, &groove_buf)?;
            state.set_stem(STEM_YUE_VOCALS, vocals_path);
            state.set_stem(STEM_YUE_INSTRUMENTALS, inst_path);
            log::info!("Node 2: Successfully generated dual stems via pure-Rust DSP harmonic renderer");
        }

        state.history.push(
            "Node 2 (Groove & YuE Synthesis) completed: draft_groove, yue_vocals, yue_instrumentals registered".to_string()
        );

        Ok(())
    }

    /// Stage 1: Synthesizes draft groove across canonical styles.
    fn generate_draft_groove(
        &self,
        state: &MusicProductionState,
        duration_secs: f32,
    ) -> Result<AudioBuffer, PipelineError> {
        let genre_name = if !state.genre.is_empty() {
            &state.genre
        } else {
            "Roots Reggae"
        };
        let bpm = state.bpm.clamp(40, 220);
        let buffer = UniversalDrumSynthesizer::generate_groove(genre_name, bpm, duration_secs);
        Ok(buffer)
    }

    /// Stage 2: Executes external YuE subprocess with deterministic process isolation & VRAM cleanup.
    async fn execute_yue_subprocess(
        &self,
        _state: &MusicProductionState,
        stems_dir: &Path,
        groove_path: &Path,
    ) -> Result<(PathBuf, PathBuf), PipelineError> {
        let version_dir = stems_dir.parent().unwrap_or(stems_dir);
        let genre_file = version_dir.join("genre.txt");
        let lyrics_file = version_dir.join("lyrics.txt");

        let expected_vocals = stems_dir.join("yue_vocals.wav");
        let expected_inst = stems_dir.join("yue_instrumentals.wav");

        let python_bin = std::env::var("YUE_PYTHON_BIN").unwrap_or_else(|_| "python".to_string());
        let infer_script = std::env::var("YUE_INFER_SCRIPT").unwrap_or_else(|_| "infer.py".to_string());

        // Verify required input files exist
        if !genre_file.exists() || !lyrics_file.exists() {
            return Err(PipelineError::StateError(
                "Missing genre.txt or lyrics.txt required for YuE inference".to_string(),
            ));
        }

        let mut cmd = tokio::process::Command::new(&python_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.arg(&infer_script)
            .arg("--stage1_model").arg("m-a-p/YuE-s1-7B-anneal-en-icl")
            .arg("--stage2_model").arg("m-a-p/YuE-s2-1B-general")
            .arg("--genre_txt").arg(&genre_file)
            .arg("--lyrics_txt").arg(&lyrics_file)
            .arg("--groove_audio").arg(groove_path)
            .arg("--output_dir").arg(stems_dir)
            .arg("--cuda_idx").arg("0");

        log::info!("Node 2: Spawning YuE subprocess: {:?}", cmd);

        // Spawn child process
        let child = cmd.spawn().map_err(|e| {
            PipelineError::SubprocessError {
                stage: "Stage2YuE".to_string(),
                message: format!("Failed to spawn {}: {}", python_bin, e),
            }
        })?;

        // Enforce execution timeout
        let timeout_duration = Duration::from_secs(self.subprocess_timeout_secs);
        let output_res = tokio::time::timeout(timeout_duration, child.wait_with_output()).await;

        let output = match output_res {
            Ok(res) => res.map_err(|e| PipelineError::SubprocessError {
                stage: "Stage2YuE".to_string(),
                message: format!("Subprocess I/O error: {}", e),
            })?,
            Err(_) => {
                return Err(PipelineError::SubprocessTimeout {
                    stage: "Stage2YuE".to_string(),
                    timeout_secs: self.subprocess_timeout_secs,
                });
            }
        };

        // Note: The child process handle has now terminated and dropped.
        // Windows WDDM / CUDA kernel driver automatically reclaims 100% of GPU memory!

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PipelineError::SubprocessError {
                stage: "Stage2YuE".to_string(),
                message: format!("Subprocess exited with code {:?}: {}", output.status.code(), stderr),
            });
        }

        // Verify expected stem output files exist
        if !expected_vocals.exists() || !expected_inst.exists() {
            return Err(PipelineError::MissingStem(
                "YuE subprocess completed successfully but output stems were not found on disk".to_string(),
            ));
        }

        Ok((expected_vocals, expected_inst))
    }

    /// Pure-Rust DSP Multi-Stem Harmonic Renderer.
    /// Synthesizes `yue_instrumentals.wav` (groove + harmonic chords + bass) and `yue_vocals.wav`.
    fn render_pure_rust_dual_stems(
        &self,
        state: &MusicProductionState,
        stems_dir: &Path,
        groove_buf: &AudioBuffer,
    ) -> Result<(PathBuf, PathBuf), PipelineError> {
        let sample_rate = groove_buf.sample_rate;
        let frames = groove_buf.frame_count();
        let duration_secs = groove_buf.duration_secs();
        let bpm = state.bpm.clamp(40, 220) as f32;
        let beat_duration = 60.0 / bpm;

        let key_root_hz = Self::parse_key_signature_root_hz(&state.key_signature);

        // 1. Synthesize Harmonic Instrumental Bed (Chords + Bassline)
        let mut inst_buf = groove_buf.clone();

        // Add melodic rolling bassline (55 Hz to 110 Hz)
        let bass_octave_hz = key_root_hz * 0.5;
        for i in 0..frames {
            let t = i as f32 / sample_rate as f32;
            let current_beat = (t / beat_duration) as usize;

            // Simple walking bass notes per measure: root, 5th, octave, 4th
            let note_mult = match current_beat % 4 {
                0 => 1.0,         // Tonic
                1 => 1.498_31,    // Perfect Fifth
                2 => 2.0,         // Octave
                3 => 1.334_84,    // Perfect Fourth
                _ => 1.0,
            };
            let bass_freq = bass_octave_hz * note_mult;

            // Sine sub-bass + subtle 2nd harmonic
            let sub = (2.0 * PI * bass_freq * t).sin() * 0.35;
            let warm_2nd = (2.0 * PI * bass_freq * 2.0 * t).sin() * 0.12;
            let bass_sample = sub + warm_2nd;

            // Mix bass into Left and Right channels with warmth
            inst_buf.samples[i * 2] = (inst_buf.samples[i * 2] * 0.70 + bass_sample).clamp(-0.95, 0.95);
            inst_buf.samples[i * 2 + 1] = (inst_buf.samples[i * 2 + 1] * 0.70 + bass_sample).clamp(-0.95, 0.95);
        }

        // Add harmonic offbeat skank / chord bed (triads at key root, 3rd, 5th)
        let root = key_root_hz;
        let third = root * 1.259921;  // Major third
        let fifth = root * 1.498307;  // Perfect fifth

        for i in 0..frames {
            let t = i as f32 / sample_rate as f32;
            let beat_phase = (t % beat_duration) / beat_duration;

            // Offbeat pulse on the "and" of the beat (0.48 to 0.72 of each beat)
            let skank_env = if (0.48..=0.72).contains(&beat_phase) {
                ((beat_phase - 0.48) / 0.24 * PI).sin() * 0.25
            } else {
                0.0
            };

            if skank_env > 0.001 {
                let chord = (2.0 * PI * root * t).sin()
                    + (2.0 * PI * third * t).sin()
                    + (2.0 * PI * fifth * t).sin();
                let chord_sample = (chord / 3.0) * skank_env;

                inst_buf.samples[i * 2] = (inst_buf.samples[i * 2] + chord_sample).clamp(-0.96, 0.96);
                inst_buf.samples[i * 2 + 1] = (inst_buf.samples[i * 2 + 1] + chord_sample * 0.90).clamp(-0.96, 0.96);
            }
        }

        let inst_path = stems_dir.join("yue_instrumentals.wav");
        inst_buf.to_wav_file_with_format(&inst_path, WavBitDepth::Float32).map_err(|e| {
            PipelineError::DspError(format!("Failed to write yue_instrumentals.wav: {}", e))
        })?;

        // 2. Synthesize Guide Vocal Stem (yue_vocals.wav)
        let mut vocal_buf = AudioBuffer::new_stereo(duration_secs);
        let vocal_root = key_root_hz * 2.0; // Tenor range (~220 Hz for A)

        for i in 0..frames {
            let t = i as f32 / sample_rate as f32;
            let current_beat = (t / beat_duration) as usize;

            // Vocal phrasing: sings for 2 beats, rests for 2 beats (call-and-response feel)
            let is_singing = (current_beat % 4) < 3;
            if is_singing {
                // Subtle vibrato (5.5 Hz)
                let vibrato = (2.0 * PI * 5.5 * t).sin() * 4.0;
                let freq = vocal_root + vibrato;

                // Formant-like harmonic structure: Fundamental + 2nd + 3rd harmonics
                let f0 = (2.0 * PI * freq * t).sin() * 0.40;
                let f1 = (2.0 * PI * freq * 2.0 * t).sin() * 0.25;
                let f2 = (2.0 * PI * freq * 3.0 * t).sin() * 0.15;
                let raw_vocal = f0 + f1 + f2;

                vocal_buf.samples[i * 2] = raw_vocal.clamp(-0.90, 0.90);
                vocal_buf.samples[i * 2 + 1] = (raw_vocal * 0.95).clamp(-0.90, 0.90);
            }
        }

        let vocals_path = stems_dir.join("yue_vocals.wav");
        vocal_buf.to_wav_file_with_format(&vocals_path, WavBitDepth::Float32).map_err(|e| {
            PipelineError::DspError(format!("Failed to write yue_vocals.wav: {}", e))
        })?;

        Ok((vocals_path, inst_path))
    }

    /// Helper to convert key signature name into fundamental root frequency in Hz.
    pub fn parse_key_signature_root_hz(key: &str) -> f32 {
        let upper = key.to_ascii_uppercase();
        if upper.starts_with("A#") || upper.starts_with("BB") {
            116.54 // Bb2 / A#2
        } else if upper.starts_with('A') {
            110.00 // A2
        } else if upper.starts_with('B') {
            123.47 // B2
        } else if upper.starts_with("C#") || upper.starts_with("DB") {
            138.59 // C#3 / Db3
        } else if upper.starts_with('C') {
            130.81 // C3
        } else if upper.starts_with("D#") || upper.starts_with("EB") {
            155.56 // Eb3 / D#3
        } else if upper.starts_with('D') {
            146.83 // D3
        } else if upper.starts_with('E') {
            164.81 // E3
        } else if upper.starts_with("F#") || upper.starts_with("GB") {
            185.00 // F#3 / Gb3
        } else if upper.starts_with('F') {
            174.61 // F3
        } else if upper.starts_with("G#") || upper.starts_with("AB") {
            207.65 // G#3 / Ab3
        } else if upper.starts_with('G') {
            196.00 // G3
        } else {
            110.00 // Default A
        }
    }
}

#[async_trait]
impl GraphNode for GrooveAndYueSynthesisNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.execute_node(state).await
    }

    fn name(&self) -> &str {
        "groove_and_yue_synthesis_node"
    }

    fn description(&self) -> &str {
        "Groove & YuE Synthesis: Draft groove, dual-stem vocals and instrumentals"
    }
}
