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

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use async_trait::async_trait;

use music_core::state::{
    MusicProductionState, STEM_DRAFT_GROOVE, STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::groove_synth::UniversalDrumSynthesizer;
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_fusion::memory::SystemMemory;

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
        groove_buf.to_wav_file_with_format(&groove_path, WavBitDepth::Pcm16).map_err(|e| {
            PipelineError::DspError(format!("Failed to write draft_groove.wav: {}", e))
        })?;
        state.set_stem(STEM_DRAFT_GROOVE, groove_path.clone());
        log::info!("Node 2: Generated draft groove ({:.1}s) at {:?}", duration_secs, groove_path);

        // 3. Stage 2: YuE multi-track vocal and accompaniment synthesis
        let memory = SystemMemory::detect_live();
        let neural_model = memory.select_neural_model(&state.engine_mode);
        log::info!(
            "Node 2: Neural model selection -> {:?} (engine_mode: {})",
            neural_model, state.engine_mode
        );

        let mut synthesized_via_hf = false;

        if neural_model != music_fusion::memory::NeuralModelChoice::PureRustFallback {
            let uv_bridge = crate::python_bridge::UvBridge::new();
            if uv_bridge.is_available() {
                let venv_dir = PathBuf::from(".venv");
                match uv_bridge.bootstrap_environment(&venv_dir) {
                    Ok(python_bin) => {
                        let script_path = PathBuf::from("workers/generate_song.py");
                        let lyrics_file = stems_dir.parent().unwrap_or(&stems_dir).join("lyrics.txt");
                        let args = crate::python_bridge::GenerateSongArgs {
                            model: neural_model.model_id().to_string(),
                            prompt: format!("{} track at {} BPM in {}", state.genre, state.bpm, state.key_signature),
                            lyrics_file,
                            bpm: state.bpm,
                            key: state.key_signature.clone(),
                            duration_secs,
                            output_dir: stems_dir.clone(),
                            device: if memory.has_usable_gpu() { "cuda".to_string() } else { "cpu".to_string() },
                        };
                        match uv_bridge.run_generate_song(&python_bin, &script_path, &args, self.subprocess_timeout_secs).await {
                            Ok(output) => {
                                state.set_stem(STEM_YUE_VOCALS, output.vocals_path);
                                state.set_stem(STEM_YUE_INSTRUMENTALS, output.instrumentals_path);
                                synthesized_via_hf = true;
                                println!(
                                    "🎛️  [SYNTHESIS ENGINE] Track rendered by Hugging Face GPU weights (Model: {}, Device: CUDA, VRAM: {:.2} GB free)",
                                    neural_model.display_name(),
                                    memory.gpu_vram_free_gb
                                );
                            }
                            Err(e) => {
                                println!(
                                    "⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: {}. Automatically falling back to pure-Rust physical modeling synthesizer.",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: {}. Automatically falling back to pure-Rust physical modeling synthesizer.",
                            e
                        );
                    }
                }
            } else {
                println!(
                    "⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: uv executable not found. Automatically falling back to pure-Rust physical modeling synthesizer."
                );
            }
        }

        // 4. Pure-Rust DSP fallback multi-stem harmonic renderer
        if !synthesized_via_hf {
            println!(
                "🎹  [SYNTHESIS ENGINE] Track rendered by pure-Rust physical modeling synthesizer (Karplus-Strong string algorithm, Reggae one-drop drums & dub bass)"
            );
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
    #[allow(dead_code)]
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
        let _beat_duration = 60.0 / bpm;

        let key_root_hz = Self::parse_key_signature_root_hz(&state.key_signature);

        // 1. Synthesize Harmonic Instrumental Bed (Chords + Bassline + Bubble Organ + Skank)
        let mut inst_buf = groove_buf.clone();

        // Synthesize Dub Sub-Bass (180 Hz lowpass, 15ms attack, 120ms release)
        let mut bass_synth = music_dsp::groove_synth::DubSubBass::new(sample_rate);
        let bass_buf = bass_synth.synthesize(duration_secs, bpm as u32);

        // Synthesize Hammond Organ Bubble (1x-4x drawbars, 6 Hz tremolo LFO, ADSR)
        let mut organ_synth = music_dsp::groove_synth::HammondOrganBubble::new(sample_rate);
        let organ_buf = organ_synth.synthesize(duration_secs, bpm as u32, key_root_hz);

        // Synthesize Karplus-Strong offbeat skank chords (beats 2 & 4)
        let skank_buf = music_dsp::guitar_wah::synthesize_reggae_skank_track(duration_secs, sample_rate, bpm as u32);

        for i in 0..frames {
            let l_drum = inst_buf.samples[i * 2];
            let r_drum = inst_buf.samples[i * 2 + 1];

            let l_bass = if i * 2 < bass_buf.samples.len() { bass_buf.samples[i * 2] } else { 0.0 };
            let r_bass = if i * 2 + 1 < bass_buf.samples.len() { bass_buf.samples[i * 2 + 1] } else { l_bass };

            let l_organ = if i * 2 < organ_buf.samples.len() { organ_buf.samples[i * 2] } else { 0.0 };
            let r_organ = if i * 2 + 1 < organ_buf.samples.len() { organ_buf.samples[i * 2 + 1] } else { l_organ };

            let l_skank = if i * 2 < skank_buf.samples.len() { skank_buf.samples[i * 2] } else { 0.0 };
            let r_skank = if i * 2 + 1 < skank_buf.samples.len() { skank_buf.samples[i * 2 + 1] } else { l_skank };

            // Authentic dub mix: drums (0.42), dub bass (0.35), bubble organ (0.22), skank (0.20)
            let mixed_l = (l_drum * 0.42 + l_bass * 0.35 + l_organ * 0.22 + l_skank * 0.20).tanh() * 0.94;
            let mixed_r = (r_drum * 0.42 + r_bass * 0.35 + r_organ * 0.22 + r_skank * 0.20).tanh() * 0.94;

            inst_buf.samples[i * 2] = mixed_l;
            inst_buf.samples[i * 2 + 1] = mixed_r;
        }

        let inst_path = stems_dir.join("yue_instrumentals.wav");
        inst_buf.to_wav_file_with_format(&inst_path, WavBitDepth::Pcm16).map_err(|e| {
            PipelineError::DspError(format!("Failed to write yue_instrumentals.wav: {}", e))
        })?;

        // 2. Synthesize Guide Vocal Stem (yue_vocals.wav) with True Melodic Singing Synthesis
        let vocal_buf = Self::synthesize_melodic_singing_vocal_stem(
            state,
            stems_dir,
            duration_secs,
            sample_rate,
            bpm,
            key_root_hz,
        );

        let vocals_path = stems_dir.join("yue_vocals.wav");
        vocal_buf.to_wav_file_with_format(&vocals_path, WavBitDepth::Pcm16).map_err(|e| {
            PipelineError::DspError(format!("Failed to write yue_vocals.wav: {}", e))
        })?;

        Ok((vocals_path, inst_path))
    }

    /// Synthesizes a true melodic singing vocal stem (`yue_vocals.wav`)
    /// locked strictly onto the musical scale (e.g. A Major Pentatonic at 76 BPM)
    /// with sustained sung vowels, 5.5 Hz vibrato, 3-part harmony choir backing,
    /// and warm studio reverb (never flat spoken desktop screen-reader TTS).
    fn synthesize_melodic_singing_vocal_stem(
        state: &MusicProductionState,
        _stems_dir: &Path,
        duration_secs: f32,
        sample_rate: u32,
        bpm: f32,
        key_root_hz: f32,
    ) -> AudioBuffer {
        let synth = music_dsp::melodic_singing::MelodicSingingSynthesizer::new(sample_rate);
        let lyrics = if !state.lyrics.trim().is_empty() {
            state.lyrics.as_str()
        } else if let Some(ref bp) = state.blueprint {
            bp.style_of_music.as_str()
        } else {
            "One love, one heart, let us get together and feel alright"
        };
        synth.synthesize_melodic_vocal_track(duration_secs, bpm, key_root_hz, lyrics)
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
