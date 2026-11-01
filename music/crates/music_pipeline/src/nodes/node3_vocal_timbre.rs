//! Node 3: Vocal Timbre Fusion Node (`vocal_timbre_fusion_node`).
//!
//! Responsibilities:
//! 1. Probes live hardware via `SystemMemory::detect_live()`.
//! 2. Consumes guide vocal stem (`yue_vocals.wav` or fallback).
//! 3. Stage 3: Vocal Timbre Conversion:
//!    - If GPU/HF mode: attempts out-of-process `Seed-VC` (`Plachtaa/Seed-VC`, `f0_condition=true`)
//!      with Windows `CREATE_NO_WINDOW` (0x08000000) and awaits process exit for 100% VRAM release.
//!    - If pure-Rust mode or fallback: executes pure-Rust formant & vocal fold rasp processor.
//! 4. Archetype matching across universal styles:
//!    - Jamaican Tenor (1977 Roots Reggae, F1=500, F2=1500, F3=2600 Hz, rasp=0.82)
//!    - Soulful R&B Soprano (Pop/Dance/Soul, F1=750, F2=1850, F3=2900 Hz, rasp=0.15)
//!    - Gritty Rock Baritone (Rock/Metal, F1=400, F2=1200, F3=2400 Hz, rasp=0.75)
//!    - Warm Acoustic Crooner (Jazz/Acoustic, F1=450, F2=1350, F3=2500 Hz, rasp=0.20)
//!    - Operatic Choral (Classical/Gospel, F1=600, F2=1600, F3=2800 Hz, rasp=0.05)
//!    - Rap Cadence (Hip-Hop/Trap, F1=550, F2=1450, F3=2550 Hz, rasp=0.40)
//! 5. Pitch-Lock Vocoder:
//!    - Invokes `PitchLockVocoder::align_and_pitch_lock` to snap fundamental pitch to key signature
//!      and preserve vocal phrasing alignment within <= 5 ms (<= 240 samples at 48 kHz).
//! 6. Optional Audio Conditioning:
//!    - Imprints `vocal_ref_wav` audio conditioning when provided.
//! 7. Headroom & Zero Digital Clipping:
//!    - Enforces soft ceiling clamp [-0.98, 0.98].
//! 8. Saves `raspy_tenor_vocals.wav` to `./storage/songs/<song_id>/v<version>/stems/raspy_tenor_vocals.wav`.
//! 9. Registers stem in `state.stem_paths` (`STEM_RASPY_TENOR_VOCALS`) and updates history.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use async_trait::async_trait;

use music_core::state::{
    MusicProductionState, STEM_DRAFT_GROOVE, STEM_RASPY_TENOR_VOCALS, STEM_YUE_VOCALS,
};
use music_dsp::vocal_formant::{
    apply_vocal_timbre, PitchLockVocoder, VocalArchetype,
};
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

/// Node 3: Vocal Timbre Fusion Node.
#[derive(Debug, Clone, Default)]
pub struct VocalTimbreFusionNode {
    pub default_rasp_override: Option<f32>,
    pub subprocess_timeout_secs: u64,
    pub storage_dir: Option<PathBuf>,
}

impl VocalTimbreFusionNode {
    pub fn new() -> Self {
        Self {
            default_rasp_override: None,
            subprocess_timeout_secs: 300,
            storage_dir: None,
        }
    }

    pub fn with_rasp(mut self, rasp: f32) -> Self {
        self.default_rasp_override = Some(rasp.clamp(0.0, 1.0));
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

    /// Primary execution method for Node 3.
    pub async fn execute_node(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        state.record_node_transition("vocal_timbre_fusion_node");

        let stems_dir = self.resolve_stems_dir(&state.song_id, state.version);
        fs::create_dir_all(&stems_dir).map_err(PipelineError::IoError)?;

        let output_vocal_path = stems_dir.join("raspy_tenor_vocals.wav");

        // 1. Locate source guide vocal stem
        let source_vocal_path = if let Some(p) = state.get_stem(STEM_YUE_VOCALS) {
            p.clone()
        } else if let Some(p) = state.get_stem(STEM_DRAFT_GROOVE) {
            log::warn!("Node 3: yue_vocals not registered; using draft_groove as source bed");
            p.clone()
        } else {
            // Generate synthetic fallback buffer if no stem exists
            let fallback_path = stems_dir.join("temp_guide_vocal.wav");
            let buf = AudioBuffer::new_stereo(2.0);
            buf.to_wav_file_with_format(&fallback_path, WavBitDepth::Pcm16).map_err(|e| {
                PipelineError::DspError(format!("Failed to write temp guide vocal: {}", e))
            })?;
            fallback_path
        };

        // 2. Hardware sizing decision for Stage 3 Voice Conversion
        let memory = SystemMemory::detect_live();
        let route = memory.route_stage(PipelineStageKind::Stage3VoiceConversion, &state.engine_mode);
        log::info!(
            "Node 3: Sizing decision for Stage3VoiceConversion -> {:?} (engine_mode: {})",
            route, state.engine_mode
        );

        let mut synthesized_via_hf = false;

        // 3. Attempt external Seed-VC subprocess execution if GPU/HF mode
        if route == ExecutionRoute::HuggingFaceGpu || route == ExecutionRoute::HuggingFaceCpu {
            match self.execute_seed_vc_subprocess(state, &source_vocal_path, &output_vocal_path).await {
                Ok(_) => {
                    synthesized_via_hf = true;
                    log::info!("Node 3: Successfully generated vocal timbre via Seed-VC subprocess");
                }
                Err(e) => {
                    log::warn!(
                        "Node 3: Seed-VC subprocess skipped/failed ({}); falling back to pure-Rust DSP vocal modeling",
                        e
                    );
                }
            }
        }

        // 4. Pure-Rust DSP Vocal Formant, Rasp & Pitch-Lock Processor
        if !synthesized_via_hf {
            self.render_pure_rust_vocal_timbre(state, &source_vocal_path, &output_vocal_path)?;
            log::info!("Node 3: Successfully generated vocal timbre via pure-Rust DSP engine");
        }

        // 5. Register output stem in state
        state.set_stem(STEM_RASPY_TENOR_VOCALS, output_vocal_path.clone());
        state.history.push(format!(
            "Node 3 (Vocal Timbre Fusion) completed: registered '{}'",
            STEM_RASPY_TENOR_VOCALS
        ));

        Ok(())
    }

    /// External Seed-VC subprocess execution with deterministic VRAM reclamation.
    async fn execute_seed_vc_subprocess(
        &self,
        state: &MusicProductionState,
        source_vocal_path: &Path,
        output_vocal_path: &Path,
    ) -> Result<(), PipelineError> {
        let python_bin = std::env::var("SEED_VC_PYTHON_BIN").unwrap_or_else(|_| "python".to_string());
        let script = std::env::var("SEED_VC_SCRIPT").unwrap_or_else(|_| "seed_vc_infer.py".to_string());

        let mut cmd = tokio::process::Command::new(&python_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.arg(&script)
            .arg("--source").arg(source_vocal_path)
            .arg("--output").arg(output_vocal_path)
            .arg("--f0_condition").arg("true");

        if let Some(ref ref_wav) = state.vocal_ref_wav {
            if ref_wav.exists() {
                cmd.arg("--ref").arg(ref_wav);
            }
        }

        log::info!("Node 3: Spawning Seed-VC subprocess: {:?}", cmd);

        let child = cmd.spawn().map_err(|e| PipelineError::SubprocessError {
            stage: "Stage3VoiceConversion".to_string(),
            message: format!("Failed to spawn {}: {}", python_bin, e),
        })?;

        let timeout_duration = Duration::from_secs(self.subprocess_timeout_secs);
        let output_res = tokio::time::timeout(timeout_duration, child.wait_with_output()).await;

        let output = match output_res {
            Ok(res) => res.map_err(|e| PipelineError::SubprocessError {
                stage: "Stage3VoiceConversion".to_string(),
                message: format!("Seed-VC subprocess I/O error: {}", e),
            })?,
            Err(_) => {
                return Err(PipelineError::SubprocessTimeout {
                    stage: "Stage3VoiceConversion".to_string(),
                    timeout_secs: self.subprocess_timeout_secs,
                });
            }
        };

        // Note: Child process handle dropped. 100% VRAM freed immediately!

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PipelineError::SubprocessError {
                stage: "Stage3VoiceConversion".to_string(),
                message: format!("Seed-VC exited with code {:?}: {}", output.status.code(), stderr),
            });
        }

        if !output_vocal_path.exists() {
            return Err(PipelineError::MissingStem(
                "Seed-VC completed but output WAV file was not created on disk".to_string(),
            ));
        }

        Ok(())
    }

    /// Pure-Rust DSP Vocal Formant & Rasp Processor with Pitch-Lock Vocoder.
    fn render_pure_rust_vocal_timbre(
        &self,
        state: &MusicProductionState,
        source_vocal_path: &Path,
        output_vocal_path: &Path,
    ) -> Result<(), PipelineError> {
        // 1. Read input audio buffer
        let input_buf = AudioBuffer::from_wav_file(source_vocal_path).map_err(|e| {
            PipelineError::DspError(format!("Failed to load source vocal WAV {:?}: {}", source_vocal_path, e))
        })?;

        // 2. Select universal vocal archetype based on genre and blueprint
        let archetype = Self::select_vocal_archetype(&state.genre, state);
        let params = archetype.formant_parameters();

        // 3. Determine rasp amount
        let rasp = if let Some(custom) = self.default_rasp_override {
            custom
        } else if let Some(ref bp) = state.blueprint {
            bp.stem_mix.vocal_rasp_intensity
        } else {
            params.default_rasp
        };

        // 4. Apply resonant formant filter bank and non-linear rasp distortion
        let formant_and_rasp_buf = apply_vocal_timbre(&input_buf, rasp, 0.0);

        // 5. Apply Pitch-Lock Vocoder & transient phase alignment (<= 5 ms alignment error)
        let key_root_hz = crate::nodes::node2_groove_yue::GrooveAndYueSynthesisNode::parse_key_signature_root_hz(
            &state.key_signature,
        ) * 2.0; // Tenor octave
        let pitch_locked = PitchLockVocoder::align_and_pitch_lock(&formant_and_rasp_buf, state.bpm, key_root_hz);

        // 6. Optional vocal_ref_wav audio conditioning
        let mut final_buf = pitch_locked;
        if let Some(ref ref_path) = state.vocal_ref_wav {
            if ref_path.exists() {
                if let Ok(ref_buf) = AudioBuffer::from_wav_file(ref_path) {
                    log::info!("Node 3: Applying audio conditioning from reference WAV: {:?}", ref_path);
                    // Blend 10% subtle harmonic spectral reference coloring
                    let mix_frames = final_buf.frame_count().min(ref_buf.frame_count());
                    for f in 0..mix_frames {
                        for c in 0..final_buf.channels as usize {
                            let idx_f = f * final_buf.channels as usize + c;
                            let idx_r = f * ref_buf.channels as usize + (c % ref_buf.channels as usize);
                            let blended = final_buf.samples[idx_f] * 0.90 + ref_buf.samples[idx_r] * 0.10;
                            final_buf.samples[idx_f] = blended.clamp(-0.98, 0.98);
                        }
                    }
                }
            }
        }

        // 7. Verify zero clipping and enforce safe broadcast headroom [-0.98, 0.98]
        final_buf.clamp(-0.98, 0.98);

        // 8. Write to disk
        final_buf.to_wav_file_with_format(output_vocal_path, WavBitDepth::Pcm16).map_err(|e| {
            PipelineError::DspError(format!("Failed to write raspy_tenor_vocals.wav: {}", e))
        })?;

        Ok(())
    }

    /// Selects universal vocal archetype based on genre name and blueprint hints.
    pub fn select_vocal_archetype(genre: &str, state: &MusicProductionState) -> VocalArchetype {
        let g = genre.to_ascii_lowercase();

        // 1. Check if state style prompt or blueprint explicitly asks for raspy Jamaican tenor
        if g.contains("reggae") || g.contains("dub") || g.contains("roots")
            || state.style_prompt.to_ascii_lowercase().contains("jamaican")
            || state.style_prompt.to_ascii_lowercase().contains("marley")
        {
            return VocalArchetype::JamaicanTenor;
        }

        // 2. Check other universal genres
        if g.contains("pop") || g.contains("r&b") || g.contains("rnb") || g.contains("soul") || g.contains("dance") {
            VocalArchetype::SoulfulRnbSoprano
        } else if g.contains("rock") || g.contains("metal") || g.contains("punk") || g.contains("grunge") {
            VocalArchetype::GrittyRockBaritone
        } else if g.contains("jazz") || g.contains("crooner") || g.contains("acoustic") || g.contains("blues") || g.contains("folk") {
            VocalArchetype::WarmAcousticCrooner
        } else if g.contains("classical") || g.contains("opera") || g.contains("choral") || g.contains("gospel") {
            VocalArchetype::OperaticChoral
        } else if g.contains("hip-hop") || g.contains("hiphop") || g.contains("trap") || g.contains("rap") || g.contains("boom-bap") {
            VocalArchetype::RapCadence
        } else {
            VocalArchetype::JamaicanTenor // Default canonical flagship preset
        }
    }
}

#[async_trait]
impl GraphNode for VocalTimbreFusionNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.execute_node(state).await
    }

    fn name(&self) -> &str {
        "vocal_timbre_fusion_node"
    }

    fn description(&self) -> &str {
        "Vocal Timbre Fusion: Formant shaping, vocal fold rasp, and pitch-lock vocoder"
    }
}
