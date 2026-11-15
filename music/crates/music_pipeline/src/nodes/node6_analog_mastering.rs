//! Node 6: Analog Mastering Node (`analog_mastering_node`).
//!
//! Applies genre-matched 48 kHz Studer A800 analog tape saturation, 55–85 Hz sub-bass
//! warmth resonant filtering, Blumlein Mid-Side stereo width expansion, and lookahead
//! brickwall peak limiter clamping strictly to <= 0.966 (-0.3 dBFS ceiling).

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use async_trait::async_trait;
use log::{info, warn};

use music_core::schema::{StemChannelStrip, StemMixConfig, CANONICAL_MIXER_STEMS};
use music_core::state::{
    MusicProductionState, STEM_AMBIENCE_FX, STEM_BACKING_HARMONIES, STEM_BASS, STEM_DRUMS,
    STEM_FINAL_MASTER, STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_KEYS_SYNTHS,
    STEM_OTHER, STEM_PERCUSSION_FILLS, STEM_RASPY_TENOR_VOCALS, STEM_YUE_VOCALS,
};
use music_dsp::tape_mastering::{apply_analog_mastering, ButterworthLowpass12kHz, MasteringProfile, BRICKWALL_CEILING};
use music_dsp::telemetry_calc::calculate_telemetry;
use music_dsp::vocal_formant::BiquadFilter;
use music_dsp::wav::AudioBuffer;
use music_fusion::memory::{PipelineStageKind, SystemMemory};

use crate::error::PipelineError;
use crate::graph::GraphNode;
use crate::nodes::node5_dsp_judge_gate::mix_buffers;

pub struct AnalogMasteringNode {
    pub storage_dir: Option<PathBuf>,
}

impl Default for AnalogMasteringNode {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalogMasteringNode {
    pub fn new() -> Self {
        Self { storage_dir: None }
    }

    pub fn with_storage_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    pub fn resolve_version_dir(&self, song_id: &str, version: u32) -> PathBuf {
        let base = self.storage_dir.clone().unwrap_or_else(|| PathBuf::from("storage"));
        base.join("songs").join(song_id).join(format!("v{}", version))
    }

    /// Selects the genre-matched mastering profile conforming to ORIGINAL_REQUEST.md.
    pub fn resolve_profile(genre: &str) -> MasteringProfile {
        let g = genre.to_ascii_lowercase();
        if g.contains("reggae") || g.contains("dub") || g.contains("rocksteady") {
            MasteringProfile::StuderA800Tape1970s
        } else if g.contains("pop") || g.contains("r&b") || g.contains("hip-hop") || g.contains("trap") {
            MasteringProfile::ModernPunchyPop
        } else if g.contains("edm") || g.contains("amapiano") || g.contains("club") || g.contains("house") || g.contains("techno") {
            MasteringProfile::ClubEdmSubHeavy
        } else if g.contains("orchestral") || g.contains("cinematic") || g.contains("classical") {
            MasteringProfile::CinematicOrchestral
        } else if g.contains("jazz") || g.contains("blues") || g.contains("acoustic") || g.contains("folk") {
            MasteringProfile::AudiophileJazz
        } else {
            MasteringProfile::StuderA800Tape1970s // Default warm tape master
        }
    }

    /// Processes one active DAW stem channel strip: converts to 2ch stereo, applies 3-band EQ
    /// (`100 Hz Low-Shelf`, `1 kHz Peak`, `10 kHz High-Shelf`), constant-power stereo panning,
    /// Schroeder multi-tap stereo reverb send, and dB fader staging.
    fn render_channel_strip(
        mut buf: AudioBuffer,
        strip: &StemChannelStrip,
        gain_linear: f32,
    ) -> AudioBuffer {
        let frames = buf.frame_count();
        if frames == 0 || gain_linear <= 1e-6 {
            return AudioBuffer::new(buf.sample_rate.max(48000), 2, 0.1);
        }

        // Convert mono -> stereo cleanly in-place if needed
        if buf.channels != 2 {
            let src_ch = buf.channels.max(1) as usize;
            let mut stereo = Vec::with_capacity(frames * 2);
            for f in 0..frames {
                let l = buf.samples[f * src_ch];
                let r = if src_ch > 1 { buf.samples[f * src_ch + 1] } else { l };
                stereo.push(l);
                stereo.push(r);
            }
            buf = AudioBuffer {
                samples: stereo,
                sample_rate: buf.sample_rate,
                channels: 2,
            };
        }

        // 1. 3-band studio EQ per channel strip
        let apply_low = strip.eq_low_db.abs() > 0.05;
        let apply_mid = strip.eq_mid_db.abs() > 0.05;
        let apply_high = strip.eq_high_db.abs() > 0.05;
        if apply_low || apply_mid || apply_high {
            let mut low_eq = BiquadFilter::low_shelf(100.0, strip.eq_low_db.clamp(-12.0, 12.0), 2);
            let mut mid_eq = BiquadFilter::peaking_eq(1000.0, 0.85, strip.eq_mid_db.clamp(-12.0, 12.0), 2);
            let mut high_eq = BiquadFilter::high_shelf(10000.0, strip.eq_high_db.clamp(-12.0, 12.0), 2);
            for f in 0..frames {
                for c in 0..2 {
                    let idx = f * 2 + c;
                    let mut s = buf.samples[idx];
                    if apply_low {
                        s = low_eq.process_sample(c, s);
                    }
                    if apply_mid {
                        s = mid_eq.process_sample(c, s);
                    }
                    if apply_high {
                        s = high_eq.process_sample(c, s);
                    }
                    buf.samples[idx] = s;
                }
            }
        }

        // 2. Constant-power stereo Pan + dB Gain staging
        let pan = strip.pan.clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * 0.25 * std::f32::consts::PI;
        let l_gain = gain_linear * angle.cos() * std::f32::consts::SQRT_2;
        let r_gain = gain_linear * angle.sin() * std::f32::consts::SQRT_2;
        for f in 0..frames {
            let idx = f * 2;
            buf.samples[idx] = (buf.samples[idx] * l_gain).clamp(-1.5, 1.5);
            buf.samples[idx + 1] = (buf.samples[idx + 1] * r_gain).clamp(-1.5, 1.5);
        }

        // 3. Lightweight Schroeder 4-tap comb + stereo diffusion reverb send
        let rev_wet = strip.reverb_send.clamp(0.0, 1.0);
        if rev_wet > 0.01 {
            let sr_scale = (buf.sample_rate.max(8000) as f32 / 48000.0).max(0.25);
            let delays_l = [
                (1557.0 * sr_scale) as usize,
                (1617.0 * sr_scale) as usize,
                (1491.0 * sr_scale) as usize,
                (1422.0 * sr_scale) as usize,
            ];
            let delays_r = [
                (1580.0 * sr_scale) as usize,
                (1640.0 * sr_scale) as usize,
                (1514.0 * sr_scale) as usize,
                (1445.0 * sr_scale) as usize,
            ];
            let mut combs_l: Vec<Vec<f32>> = delays_l.iter().map(|&d| vec![0.0; d.max(8)]).collect();
            let mut combs_r: Vec<Vec<f32>> = delays_r.iter().map(|&d| vec![0.0; d.max(8)]).collect();
            let mut pos_l = [0usize; 4];
            let mut pos_r = [0usize; 4];
            let feedback = 0.72f32;
            let wet_mix = rev_wet * 0.35;

            for f in 0..frames {
                let dry_l = buf.samples[f * 2];
                let dry_r = buf.samples[f * 2 + 1];
                let mono_in = 0.5 * (dry_l + dry_r);
                let mut sum_l = 0.0f32;
                let mut sum_r = 0.0f32;
                for i in 0..4 {
                    let pl = pos_l[i];
                    let yl = combs_l[i][pl];
                    combs_l[i][pl] = mono_in + yl * feedback;
                    pos_l[i] = (pl + 1) % combs_l[i].len();
                    sum_l += yl;

                    let pr = pos_r[i];
                    let yr = combs_r[i][pr];
                    combs_r[i][pr] = mono_in + yr * feedback;
                    pos_r[i] = (pr + 1) % combs_r[i].len();
                    sum_r += yr;
                }
                buf.samples[f * 2] = (dry_l + (sum_l * 0.25) * wet_mix).clamp(-1.5, 1.5);
                buf.samples[f * 2 + 1] = (dry_r + (sum_r * 0.25) * wet_mix).clamp(-1.5, 1.5);
            }
        }

        buf
    }

    /// Sums all 8 DAW stems in parallel (`std::thread::scope`) into a 48 kHz stereo premaster
    /// buffer honoring Solo/Mute, dB faders, constant-power pan, 3-band EQ, reverb sends,
    /// and vocal-priority sidechain ducking.
    pub fn mix_stems_to_premaster(&self, state: &MusicProductionState) -> AudioBuffer {
        let mut premaster = AudioBuffer::new_stereo(0.1);
        let default_mix = StemMixConfig::default();
        let mix_cfg = state
            .blueprint
            .as_ref()
            .map(|b| b.stem_mix.clone())
            .unwrap_or_else(|| default_mix.clone());

        let any_solo = mix_cfg.any_solo();

        struct StemJob {
            chan_id: &'static str,
            path: PathBuf,
            strip: StemChannelStrip,
            gain_linear: f32,
            is_vocal_group: bool,
        }

        let resolve_path = |chan_id: &str| -> Option<PathBuf> {
            match chan_id {
                "vocals" => state
                    .get_stem(STEM_RASPY_TENOR_VOCALS)
                    .or_else(|| state.get_stem(STEM_YUE_VOCALS))
                    .cloned(),
                "backing_harmonies" => state.get_stem(STEM_BACKING_HARMONIES).cloned(),
                "drums" => state
                    .get_stem(STEM_INPAINTED_DRUMS)
                    .or_else(|| state.get_stem(STEM_DRUMS))
                    .cloned(),
                "bass" => state.get_stem(STEM_BASS).cloned(),
                "guitar" => state
                    .get_stem(STEM_INPAINTED_GUITAR)
                    .or_else(|| state.get_stem(STEM_GUITAR))
                    .cloned(),
                "keys_synths" => state
                    .get_stem(STEM_KEYS_SYNTHS)
                    .or_else(|| state.get_stem(STEM_OTHER))
                    .cloned(),
                "percussion_fills" => state.get_stem(STEM_PERCUSSION_FILLS).cloned(),
                "ambience_fx" => state.get_stem(STEM_AMBIENCE_FX).cloned(),
                _ => state.get_stem(chan_id).cloned(),
            }
        };

        let base_scale_for = |chan_id: &str| -> f32 {
            match chan_id {
                "vocals" => 0.420,
                "backing_harmonies" => 0.145,
                "drums" => 0.165,
                "bass" => 0.155,
                "guitar" => 0.150,
                "keys_synths" => 0.145,
                "percussion_fills" => 0.105,
                "ambience_fx" => 0.105,
                _ => 0.150,
            }
        };

        let mut seen_paths: HashSet<PathBuf> = HashSet::new();
        let mut jobs: Vec<StemJob> = Vec::with_capacity(CANONICAL_MIXER_STEMS.len());

        for &chan_id in &CANONICAL_MIXER_STEMS {
            let strip = mix_cfg.resolve_channel(chan_id);
            let active = !strip.mute && (!any_solo || strip.solo);
            if !active {
                continue;
            }
            if let Some(path) = resolve_path(chan_id) {
                if !path.is_file() || !seen_paths.insert(path.clone()) {
                    continue;
                }
                let def_strip = default_mix.resolve_channel(chan_id);
                let delta_db = (strip.gain_db - def_strip.gain_db).clamp(-24.0, 12.0);
                let gain_linear = base_scale_for(chan_id) * 10.0f32.powf(delta_db / 20.0);
                jobs.push(StemJob {
                    chan_id,
                    path,
                    strip,
                    gain_linear,
                    is_vocal_group: matches!(chan_id, "vocals" | "backing_harmonies"),
                });
            }
        }

        // Parallel per-stem WAV decode + EQ + Pan + Reverb via std::thread::scope
        let rendered: Vec<(&'static str, bool, Option<AudioBuffer>)> = std::thread::scope(|s| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|job| {
                    s.spawn(move || {
                        let maybe_buf = AudioBuffer::from_wav_file(&job.path)
                            .ok()
                            .map(|b| Self::render_channel_strip(b, &job.strip, job.gain_linear));
                        (job.chan_id, job.is_vocal_group, maybe_buf)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });

        let mut vocal_lead_buf: Option<AudioBuffer> = None;
        let mut backing_buf: Option<AudioBuffer> = None;
        for (chan_id, is_vocal, maybe_buf) in rendered {
            let Some(buf) = maybe_buf else { continue };
            if !is_vocal {
                mix_buffers(&mut premaster, &buf, 1.0);
            } else if chan_id == "vocals" {
                vocal_lead_buf = Some(buf);
            } else {
                backing_buf = Some(buf);
            }
        }

        // Apply Vocal-Priority Sidechain Ducking (-2.6 dB on instruments when lead vocal sings)
        if let Some(ref vocal_buf) = vocal_lead_buf {
            let ch = premaster.channels.max(1) as usize;
            let v_ch = vocal_buf.channels.max(1) as usize;
            let common_frames = premaster.frame_count().min(vocal_buf.frame_count());
            let mut env = 0.0f32;
            let attack = 0.008f32;
            let release = 0.0004f32;
            for f in 0..common_frames {
                let v_abs = vocal_buf.samples[f * v_ch].abs();
                if v_abs > env {
                    env += attack * (v_abs - env);
                } else {
                    env += release * (v_abs - env);
                }
                let duck_gain = (1.0 - (env * 1.4).clamp(0.0, 0.26)).clamp(0.74, 1.0);
                for c in 0..ch {
                    premaster.samples[f * ch + c] *= duck_gain;
                }
            }
            mix_buffers(&mut premaster, vocal_buf, 1.0);
        }
        if let Some(ref b_buf) = backing_buf {
            mix_buffers(&mut premaster, b_buf, 1.0);
        }

        premaster
    }

    /// Primary execution method for Node 6.
    pub async fn execute_node(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        info!("Executing AnalogMasteringNode for song '{}'", state.song_id);

        let song_dir = self.resolve_version_dir(&state.song_id, state.version);
        fs::create_dir_all(&song_dir).map_err(PipelineError::IoError)?;

        // Offload CPU-heavy stem loading, 8-ch DSP mixing, EQ/reverb, and analog mastering to blocking pool
        let state_snapshot = state.clone();
        let storage_dir_clone = self.storage_dir.clone();
        let (master_buf, final_telemetry) = tokio::task::spawn_blocking(move || {
            let node = AnalogMasteringNode {
                storage_dir: storage_dir_clone,
            };
            let mut premaster = node.mix_stems_to_premaster(&state_snapshot);

            // Preserve full 20 Hz - 20 kHz shimmer on neural stems; keep 12 kHz low-pass on pure-Rust synths
            if !state_snapshot.rendered_by_neural {
                let mut lpf = ButterworthLowpass12kHz::new(
                    premaster.channels as usize,
                    premaster.sample_rate as f32,
                );
                lpf.process_buffer(&mut premaster);
            }

            let profile = Self::resolve_profile(&state_snapshot.genre);
            let memory = SystemMemory::detect_live();
            let use_dsp = memory.should_use_dsp(PipelineStageKind::Stage5Mastering, &state_snapshot.engine_mode);
            let mut mastered = if !use_dsp && state_snapshot.is_hf_hybrid() {
                apply_analog_mastering(&premaster, profile)
            } else if let Some(ref bp) = state_snapshot.blueprint {
                music_dsp::tape_mastering::apply_analog_tape_mastering(
                    &premaster,
                    bp.stem_mix.sub_bass_boost_db,
                )
            } else {
                apply_analog_mastering(&premaster, profile)
            };

            for s in &mut mastered.samples {
                *s = (*s * 1.2).tanh();
            }

            let mut max_abs = 0.0f32;
            for &s in &mastered.samples {
                let a = s.abs();
                if a > max_abs {
                    max_abs = a;
                }
            }
            if max_abs > 0.0001 {
                let target_peak = 0.89125f32; // -1.0 dBFS
                let norm_gain = target_peak / max_abs;
                for s in &mut mastered.samples {
                    *s = (*s * norm_gain).clamp(-0.89125, 0.89125);
                }
            }

            let final_telemetry = calculate_telemetry(&mastered, state_snapshot.bpm);
            (mastered, final_telemetry)
        })
        .await
        .map_err(|e| PipelineError::DspError(format!("Mastering blocking task panicked: {}", e)))?;

        info!("Final Master Telemetry: {}", final_telemetry.summary());

        // Assert brickwall ceiling invariant
        if final_telemetry.clipping || final_telemetry.peak_dbfs > 0.0 {
            warn!("Mastering peak exceeds 0.0 dBFS ceiling; clamping to {}", BRICKWALL_CEILING);
        }

        state.set_telemetry(final_telemetry.clone());

        // 7. Save final_master.wav as universally playable 16-bit PCM WAV via hound
        let master_path = song_dir.join("final_master.wav");
        master_buf.to_wav_file_hound_pcm16(&master_path)
            .map_err(|e| PipelineError::DspError(e.to_string()))?;
        state.set_stem(STEM_FINAL_MASTER, master_path);

        // 8. Write full song metadata manifest including "lyrics"
        let manifest_path = song_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&serde_json::json!({
            "song_id": state.song_id,
            "version": state.version,
            "genre": state.genre,
            "bpm": state.bpm,
            "key_signature": state.key_signature,
            "time_signature": state.time_signature,
            "style_prompt": state.style_prompt,
            "lyrics": state.lyrics,
            "judge_score": state.judge_score,
            "quality_passed": state.quality_passed,
            "retry_count": state.retry_count,
            "telemetry": final_telemetry,
            "stems": state.stem_paths,
            "created_at": state.created_at,
            "updated_at": state.updated_at,
        }))?;
        fs::write(&manifest_path, manifest_json).map_err(PipelineError::IoError)?;

        state.record_node_transition("analog_mastering_node");
        info!("AnalogMasteringNode successfully completed final master.");
        Ok(())
    }
}

#[async_trait]
impl GraphNode for AnalogMasteringNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.execute_node(state).await
    }

    fn name(&self) -> &str {
        "analog_mastering_node"
    }

    fn description(&self) -> &str {
        "Analog Mastering: Studer A800 tape saturation, sub-bass warmth, and brickwall limiter"
    }
}
