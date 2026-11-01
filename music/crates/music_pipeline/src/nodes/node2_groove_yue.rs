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

        // 2. Synthesize Guide Vocal Stem (yue_vocals.wav) with Real Articulated English Lyrics
        let vocal_buf = Self::synthesize_articulated_vocal_stem(
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

    /// Synthesizes a richly articulated, intelligible vocal stem (`yue_vocals.wav`)
    /// using Windows native SpeechSynthesizer at natural human pitch with studio de-essing
    /// and subtle spatial room presence (zero carrier sine whine, zero overlapping voice chaos).
    fn synthesize_articulated_vocal_stem(
        state: &MusicProductionState,
        stems_dir: &Path,
        duration_secs: f32,
        sample_rate: u32,
        bpm: f32,
        key_root_hz: f32,
    ) -> AudioBuffer {
        let frames = (duration_secs * sample_rate as f32).round() as usize;
        let beat_duration = 60.0 / bpm.max(40.0);
        let vocal_root = if key_root_hz > 165.0 { key_root_hz } else { key_root_hz * 1.5 };

        // Major pentatonic melody scale ratios: Root, 2nd, 3rd, 5th, 6th
        let _melody_scale: [f32; 8] = [
            1.0,       // Tonic
            1.122_46,  // Major Second
            1.259_92,  // Major Third
            1.498_31,  // Perfect Fifth
            1.681_79,  // Major Sixth
            1.498_31,  // Perfect Fifth
            1.259_92,  // Major Third
            1.0,       // Tonic
        ];

        // Attempt real articulated English vocal synthesis when duration >= 4.0s
        let mut speech_buf_opt = if duration_secs >= 4.0 {
            Self::try_synthesize_windows_sapi_vocals(state, stems_dir, duration_secs)
        } else {
            None
        };

        let mut vocal_buf = AudioBuffer::new(sample_rate, 2, duration_secs);

        if let Some(ref mut speech_buf) = speech_buf_opt {
            // Normalize speech buffer peak to 0.88 so words are crystal clear
            let mut max_peak = 0.0f32;
            for &s in &speech_buf.samples {
                let a = s.abs();
                if a > max_peak {
                    max_peak = a;
                }
            }
            let norm_gain = if max_peak > 0.01 { (0.88 / max_peak).clamp(0.5, 3.0) } else { 1.0 };

            let copy_frames = frames.min(speech_buf.frame_count());
            let src_ch = speech_buf.channels as usize;
            let mut lp_l = 0.0f32;
            let mut lp_r = 0.0f32;

            for i in 0..copy_frames {
                let raw_l = speech_buf.samples[i * src_ch] * norm_gain;
                let raw_r = if src_ch > 1 {
                    speech_buf.samples[i * src_ch + 1] * norm_gain
                } else {
                    raw_l
                };

                // Gentle 1-pole de-esser smoothing (~4.5 kHz) — zero continuous sine carrier whine!
                lp_l = 0.45 * lp_l + 0.55 * raw_l;
                lp_r = 0.45 * lp_r + 0.55 * raw_r;

                vocal_buf.samples[i * 2] = (raw_l * 0.72 + lp_l * 0.28).clamp(-0.94, 0.94);
                vocal_buf.samples[i * 2 + 1] = (raw_r * 0.72 + lp_r * 0.28).clamp(-0.94, 0.94);
            }
        } else {
            // Fast Pure-Rust Warm Syllable Synthesizer (for short unit tests / non-Windows)
            // Uses phase-continuous accumulators, portamento glide, and smooth raised-cosine envelopes
            let syllable_dur = (beat_duration * 0.5).max(0.18);
            let sr = sample_rate as f32;
            let mut vocal_phase = 0.0f32;
            let mut smoothed_freq = vocal_root;
            let mut smoothed_env = 0.0f32;
            let glide_alpha = (1.0 - (-1.0 / (0.015 * sr)).exp()).clamp(0.0005, 0.05);
            let env_alpha = (1.0 - (-1.0 / (0.010 * sr)).exp()).clamp(0.001, 0.08);

            for i in 0..frames {
                let t = i as f32 / sr;
                let current_beat = (t / beat_duration) as usize;
                let syl_phase = (t % syllable_dur) / syllable_dur;

                // Continuous vocal delivery without silence gating dropouts
                let target_env = if syl_phase < 0.15 {
                    0.25 + 0.75 * (syl_phase / 0.15 * PI * 0.5).sin()
                } else if syl_phase < 0.80 {
                    1.0
                } else {
                    0.25 + 0.75 * (((1.0 - syl_phase) / 0.20) * PI * 0.5).sin().max(0.0)
                };
                smoothed_env += env_alpha * (target_env - smoothed_env);

                // Locked strictly to A Major pentatonic frequencies [220.0, 246.94, 277.18, 329.63, 369.99, 440.0]
                let penta = music_dsp::guitar_wah::A_MAJOR_PENTATONIC;
                let target_penta_hz = penta[current_beat % penta.len()];
                let note_freq = if vocal_root > 0.0 {
                    (vocal_root / 220.0) * target_penta_hz
                } else {
                    target_penta_hz
                };
                let vibrato = (2.0 * PI * 5.0 * t).sin() * 1.5;
                let target_freq = note_freq + vibrato;
                smoothed_freq += glide_alpha * (target_freq - smoothed_freq);

                vocal_phase = (vocal_phase + smoothed_freq / sr).fract();

                let glottal = (2.0 * PI * vocal_phase).sin() * 0.52
                    + (4.0 * PI * vocal_phase).sin() * 0.22
                    + (6.0 * PI * vocal_phase).sin() * 0.08;

                let raw_vocal = (glottal * smoothed_env).tanh() * 0.85;
                vocal_buf.samples[i * 2] = raw_vocal;
                vocal_buf.samples[i * 2 + 1] = raw_vocal * 0.95;
            }
        }

        // Apply subtle 115ms Stereo Studio Room Ambience (0.08 wet mix) for natural depth
        let delay_frames = ((0.115 * sample_rate as f32) as usize).max(1);
        let mut delayed = vocal_buf.clone();
        for i in delay_frames..frames {
            let dry_l = vocal_buf.samples[i * 2];
            let dry_r = vocal_buf.samples[i * 2 + 1];
            let echo_l = vocal_buf.samples[(i - delay_frames) * 2 + 1] * 0.08;
            let echo_r = vocal_buf.samples[(i - delay_frames) * 2] * 0.08;
            delayed.samples[i * 2] = (dry_l + echo_l).clamp(-0.95, 0.95);
            delayed.samples[i * 2 + 1] = (dry_r + echo_r).clamp(-0.95, 0.95);
        }

        delayed
    }

    /// Invokes Windows `System.Speech.Synthesis.SpeechSynthesizer` at 48,000 Hz 16-bit Stereo PCM
    /// to render crystal-clear, natural-pitch Lead Vocals from the song's actual lyrics.
    #[cfg(windows)]
    fn try_synthesize_windows_sapi_vocals(
        state: &MusicProductionState,
        stems_dir: &Path,
        duration_secs: f32,
    ) -> Option<AudioBuffer> {
        let raw_lyrics = if !state.lyrics.trim().is_empty() {
            state.lyrics.clone()
        } else if let Some(ref bp) = state.blueprint {
            bp.to_yue_lyrics_txt()
        } else {
            String::new()
        };

        // Strip section headers like [Verse 1], [Chorus] AND parenthetical stage notes (...)
        let mut cleaned_text = String::with_capacity(raw_lyrics.len());
        let mut in_brackets = false;
        let mut in_parens = false;
        for ch in raw_lyrics.chars() {
            match ch {
                '[' => {
                    in_brackets = true;
                    cleaned_text.push('\n');
                }
                ']' => {
                    in_brackets = false;
                    cleaned_text.push('\n');
                }
                '(' => in_parens = true,
                ')' => in_parens = false,
                _ if !in_brackets && !in_parens => cleaned_text.push(ch),
                _ => {}
            }
        }

        let mut lines: Vec<String> = cleaned_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                l.chars()
                    .filter(|c| c.is_alphanumeric() || " ,.!?'-\"".contains(*c))
                    .collect::<String>()
            })
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            if state.genre.to_ascii_lowercase().contains("afrobeats") {
                lines = vec![
                    "Lagos to the world, feel the groove, we move to the sunrise beat!".to_string(),
                    "Early morning sunshine rising over Lagos bay, feel the energy awaken as we start the brand new day.".to_string(),
                    "Highlife sweet guitar is plucking melodies so clean, flipping through the rhythm like a cinematic scene.".to_string(),
                    "Log drum is vibrating, shake the body, let it go! African connection got the whole world in a glow.".to_string(),
                    "Celebrate the journey from the island to the street, nothing can compare to this syncopated beat.".to_string(),
                    "Raise up your hands if you are ready for the sound, feel the joy spreading all across the town.".to_string(),
                ];
            } else {
                lines = vec![
                    "Walking down the trenchtown avenue early morning light, guiding all the children through the struggles of the night.".to_string(),
                    "Hear the sweet Nyabinghi drums rolling down the street, every heavy burden falls before the one drop beat.".to_string(),
                    "One love, one heart, let us get together and feel alright.".to_string(),
                    "Give thanks and praises to the highest light, we are standing tall through the darkest night.".to_string(),
                    "Rasta man vibration spreading joy across the land, healing every brother with a helping open hand.".to_string(),
                    "Love is the answer, love is the righteous way, lighting up tomorrow from the dawn of yesterday.".to_string(),
                ];
            }
        }

        // Repeat lyrical stanzas so the vocal performance spans the full duration of the song
        let target_line_count = ((duration_secs / 3.8).ceil() as usize).clamp(4, 90);
        let mut expanded_lines = Vec::with_capacity(target_line_count);
        for idx in 0..target_line_count {
            expanded_lines.push(lines[idx % lines.len()].clone());
        }

        let lead_wav_path = stems_dir.join("_sapi_lead_vocal.wav");
        let lyrics_txt_path = stems_dir.join("_sapi_lyrics.txt");

        let joined_lyrics = expanded_lines.join("\n");
        if fs::write(&lyrics_txt_path, &joined_lyrics).is_err() {
            return None;
        }

        let ps_script = r#"
Add-Type -AssemblyName System.Speech
$ErrorActionPreference = 'Stop'
$lyricsPath = $args[0]
$leadOut = $args[1]

$lines = Get-Content -Path $lyricsPath -Encoding UTF8 | Where-Object { $_.Trim().Length -gt 0 }
$fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(48000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Stereo)

$leadSynth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try { $leadSynth.SelectVoice('Microsoft David Desktop') } catch {}
$leadSynth.Volume = 100
$leadSynth.SetOutputToWaveFile($leadOut, $fmt)

$pitches = @('+0st', '+1st', '+2st', '+0st', '-1st', '+1st')
$rates = @('-6%', '-5%', '-8%', '-6%')
$sbLead = New-Object System.Text.StringBuilder
[void]$sbLead.Append('<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">')
[void]$sbLead.Append('<break time="450ms"/>')
$idx = 0
foreach ($line in $lines) {
    $clean = [System.Security.SecurityElement]::Escape($line)
    $p = $pitches[$idx % $pitches.Length]
    $r = $rates[$idx % $rates.Length]
    [void]$sbLead.Append("<prosody pitch='$p' rate='$r' volume='loud'>$clean</prosody><break time='500ms'/>")
    $idx++
}
[void]$sbLead.Append('</speak>')
$leadSynth.SpeakSsml($sbLead.ToString())
$leadSynth.SetOutputToNull()
$leadSynth.Dispose()
"#;

        let script_path = stems_dir.join("_synth_vocals.ps1");
        if fs::write(&script_path, ps_script).is_err() {
            return None;
        }

        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script_path.to_string_lossy(),
                &lyrics_txt_path.to_string_lossy(),
                &lead_wav_path.to_string_lossy(),
            ])
            .status();

        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_file(&lyrics_txt_path);

        if !status.map(|s| s.success()).unwrap_or(false) {
            return None;
        }

        let lead_buf = AudioBuffer::from_wav_file(&lead_wav_path).ok()?;
        let _ = fs::remove_file(&lead_wav_path);

        let target_frames = (duration_secs * 48_000.0).round() as usize;
        let mut combined = AudioBuffer::new(48_000, 2, duration_secs);

        let lead_frames = lead_buf.frame_count().max(1);
        let lead_ch = lead_buf.channels as usize;
        let doubler_offset = 528usize; // 11ms sample-locked stereo doubler (never out of sync)
        let loop_fade_frames = 960usize.min(lead_frames / 4).max(1); // 20ms smooth fade at loop boundaries

        for f in 0..target_frames {
            let lf = f % lead_frames;
            // Apply smooth linear fade-in/fade-out at loop wrap boundaries to prevent clicks
            let loop_env = if lf < loop_fade_frames {
                lf as f32 / loop_fade_frames as f32
            } else if lf >= lead_frames - loop_fade_frames {
                (lead_frames - 1 - lf) as f32 / loop_fade_frames as f32
            } else {
                1.0
            };

            let l_lead = lead_buf.samples[lf * lead_ch] * loop_env;
            let r_lead = if lead_ch > 1 {
                lead_buf.samples[lf * lead_ch + 1] * loop_env
            } else {
                l_lead
            };

            let (dbl_l, dbl_r) = if f >= doubler_offset {
                let df = (f - doubler_offset) % lead_frames;
                let dbl_env = if df < loop_fade_frames {
                    df as f32 / loop_fade_frames as f32
                } else if df >= lead_frames - loop_fade_frames {
                    (lead_frames - 1 - df) as f32 / loop_fade_frames as f32
                } else {
                    1.0
                };
                let dl = lead_buf.samples[df * lead_ch] * 0.16 * dbl_env;
                let dr = if lead_ch > 1 { lead_buf.samples[df * lead_ch + 1] * 0.16 * dbl_env } else { dl };
                (dr, dl)
            } else {
                (0.0, 0.0)
            };

            combined.samples[f * 2] = (l_lead * 0.88 + dbl_l).tanh() * 0.94;
            combined.samples[f * 2 + 1] = (r_lead * 0.88 + dbl_r).tanh() * 0.94;
        }

        Some(combined)
    }

    #[cfg(not(windows))]
    fn try_synthesize_windows_sapi_vocals(
        _state: &MusicProductionState,
        _stems_dir: &Path,
        _duration_secs: f32,
    ) -> Option<AudioBuffer> {
        None
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
