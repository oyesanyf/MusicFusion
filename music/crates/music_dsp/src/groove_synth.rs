//! crates/music_dsp/src/groove_synth.rs
//!
//! Universal Drum & Rhythm Synthesizer generating authentic 48 kHz stereo PCM audio
//! conditioned on GenreTaxonomy, RhythmGroove, and BPM (40 to 220).
//!
//! Synthesizes authentic drum voices from physical/subtractive models:
//! - Deep pitch-bent 808 sub-bass kick
//! - Punchy acoustic/EDM kick
//! - Dual-layer acoustic snare & concert snare
//! - Wooden cross-stick / Reggae rimshot
//! - Multi-clap burst
//! - Metallic closed & open hi-hats
//! - Amapiano / Afrobeat percussive log drums
//! - Polyrhythmic Nyabinghi hand drums (Funde, Akete, Thunder)
//! - Afrobeat syncopated shaker & clave
//! - Jazz swung ride cymbal with bell ping & sizzle
//! - Orchestral timpani rolls and percussion swells

use std::f32::consts::PI;
use music_core::genre::{GenrePreset, GenreTaxonomy, RhythmGroove};
use crate::wav::AudioBuffer;

pub const SAMPLE_RATE: u32 = 48_000;

/// Fast, deterministic PRNG for pure-Rust DSP (Xorshift32).
#[derive(Debug, Clone)]
pub struct DspRng {
    state: u32,
}

impl DspRng {
    pub fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 0x1337_C0DE } else { seed } }
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Parameters configuring a single drum hit event.
#[derive(Debug, Clone, Copy)]
pub struct DrumHit {
    pub time_secs: f32,
    pub voice: DrumVoice,
    pub velocity: f32, // 0.0 to 1.0
    pub pan: f32,      // -1.0 (left) to 1.0 (right)
}

/// Identifiers for individual drum synthesizer voices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrumVoice {
    KickAcoustic,
    Kick808Sub,
    KickEdmPunch,
    SnareAcoustic,
    SnareRimshot,
    Clap,
    HiHatClosed,
    HiHatOpen,
    HiHatTriplet,
    LogDrumAmapiano,
    NyabinghiFunde,
    NyabinghiAkete,
    NyabinghiThunder,
    Clave,
    Shaker,
    JazzRideCymbal,
    OrchestralTimpani,
    CrashCymbal,
}

/// Universal Drum Synthesizer Engine.
pub struct UniversalDrumSynthesizer;

pub type UniversalDrumSynth = UniversalDrumSynthesizer;

impl UniversalDrumSynthesizer {
    /// Public entry point conforming to PROJECT.md interface contract:
    /// `generate_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer`
    pub fn generate_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer {
        let clamped_bpm = bpm.clamp(40, 220);
        let preset = GenreTaxonomy::find_or_create(genre);
        Self::generate_groove_preset(&preset, Some(clamped_bpm), duration_secs)
    }

    /// Generates groove conditioned on a resolved GenrePreset.
    pub fn generate_groove_preset(preset: &GenrePreset, bpm: Option<u32>, duration_secs: f32) -> AudioBuffer {
        let active_bpm = bpm.unwrap_or(preset.default_bpm).clamp(40, 220);
        Self::generate_groove_rhythm(preset.rhythm_groove, active_bpm, duration_secs)
    }

    /// Generates groove directly from a RhythmGroove pattern foundation.
    pub fn generate_groove_rhythm(rhythm: RhythmGroove, bpm: u32, duration_secs: f32) -> AudioBuffer {
        let clamped_bpm = bpm.clamp(40, 220);
        let mut buf = AudioBuffer::new_stereo(duration_secs);
        let frames = buf.frame_count();
        let total_secs = frames as f32 / SAMPLE_RATE as f32;

        let hits = Self::schedule_pattern(rhythm, clamped_bpm, total_secs);
        Self::render_hits(&hits, &mut buf);
        buf
    }

    /// Schedules rhythmic hit events across the requested duration.
    pub fn schedule_pattern(rhythm: RhythmGroove, bpm: u32, duration_secs: f32) -> Vec<DrumHit> {
        let mut hits = Vec::new();
        let beat_sec = 60.0 / bpm as f32;
        let bar_sec = beat_sec * 4.0;
        let total_bars = (duration_secs / bar_sec).ceil() as usize;

        for bar in 0..total_bars {
            let bar_start = bar as f32 * bar_sec;
            if bar_start >= duration_secs {
                break;
            }

            match rhythm {
                // 1. One-Drop Reggae (76 BPM):
                // Beat 1: Empty or subtle hi-hat.
                // Beat 3: Heavy Kick & Snare Rimshot simultaneously.
                // Shuffle 16th hi-hats, Nyabinghi hand percussion polyrhythms.
                RhythmGroove::OneDrop => {
                    let swing_offset = beat_sec * 0.045; // 16th-note shuffle
                    for b in 0..4 {
                        let beat_t = bar_start + b as f32 * beat_sec;
                        // Hi-Hats: 8th-note shuffle feel
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::HiHatClosed, velocity: 0.65, pan: -0.2 });
                        }
                        let off_t = beat_t + beat_sec * 0.5 + swing_offset;
                        if off_t < duration_secs {
                            hits.push(DrumHit { time_secs: off_t, voice: DrumVoice::HiHatClosed, velocity: 0.50, pan: -0.2 });
                        }
                    }

                    // Beat 3: Authentic One-Drop impact (Kick + Rimshot on beat 3)
                    let beat3_t = bar_start + 2.0 * beat_sec;
                    if beat3_t < duration_secs {
                        hits.push(DrumHit { time_secs: beat3_t, voice: DrumVoice::KickAcoustic, velocity: 0.95, pan: 0.0 });
                        hits.push(DrumHit { time_secs: beat3_t, voice: DrumVoice::SnareRimshot, velocity: 0.90, pan: 0.1 });
                    }

                    // Polyrhythmic Nyabinghi hand drums on syncopated subdivisions
                    let funde_t1 = bar_start + 0.5 * beat_sec;
                    let funde_t2 = bar_start + 2.5 * beat_sec;
                    if funde_t1 < duration_secs {
                        hits.push(DrumHit { time_secs: funde_t1, voice: DrumVoice::NyabinghiFunde, velocity: 0.70, pan: -0.4 });
                    }
                    if funde_t2 < duration_secs {
                        hits.push(DrumHit { time_secs: funde_t2, voice: DrumVoice::NyabinghiFunde, velocity: 0.75, pan: -0.4 });
                    }
                    let akete_t = bar_start + 3.25 * beat_sec;
                    if akete_t < duration_secs {
                        hits.push(DrumHit { time_secs: akete_t, voice: DrumVoice::NyabinghiAkete, velocity: 0.65, pan: 0.4 });
                    }
                    // Occasional Nyabinghi Thunder roll on bar downbeat
                    if bar % 4 == 0 && bar_start < duration_secs {
                        hits.push(DrumHit { time_secs: bar_start, voice: DrumVoice::NyabinghiThunder, velocity: 0.85, pan: 0.0 });
                    }
                }

                // 2. Steppers Reggae:
                // Kick on 1, 2, 3, 4 with rimshot on 3 and dub syncopations
                RhythmGroove::Steppers => {
                    for b in 0..4 {
                        let beat_t = bar_start + b as f32 * beat_sec;
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::KickAcoustic, velocity: 0.88, pan: 0.0 });
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::HiHatClosed, velocity: 0.60, pan: -0.15 });
                        }
                    }
                    let beat3_t = bar_start + 2.0 * beat_sec;
                    if beat3_t < duration_secs {
                        hits.push(DrumHit { time_secs: beat3_t, voice: DrumVoice::SnareRimshot, velocity: 0.92, pan: 0.1 });
                    }
                }

                // 3. Four-on-the-Floor (120–128 BPM - EDM/House/Pop):
                // Kick on 1, 2, 3, 4; Clap on 2 and 4; Open hi-hat on offbeats.
                RhythmGroove::FourOnTheFloor => {
                    for b in 0..4 {
                        let beat_t = bar_start + b as f32 * beat_sec;
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::KickEdmPunch, velocity: 0.95, pan: 0.0 });
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::HiHatClosed, velocity: 0.45, pan: -0.15 });
                        }
                        // Clap on 2 and 4
                        if (b == 1 || b == 3) && beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::Clap, velocity: 0.85, pan: 0.1 });
                        }
                        // Open hi-hat on upbeat (offbeat)
                        let off_t = beat_t + beat_sec * 0.5;
                        if off_t < duration_secs {
                            hits.push(DrumHit { time_secs: off_t, voice: DrumVoice::HiHatOpen, velocity: 0.75, pan: 0.25 });
                        }
                    }
                }

                // 4. Boom-Bap & Trap (70–144 BPM):
                // Trap 808: pitch-bent sub-bass kick, hard snare on 3, rapid hi-hat rolls/triplets.
                RhythmGroove::Trap808 | RhythmGroove::BoomBap => {
                    // Kick pattern (1, 1-and-a, 3-and)
                    let k1 = bar_start;
                    let k2 = bar_start + 1.75 * beat_sec;
                    let k3 = bar_start + 2.5 * beat_sec;
                    if k1 < duration_secs {
                        hits.push(DrumHit { time_secs: k1, voice: DrumVoice::Kick808Sub, velocity: 1.0, pan: 0.0 });
                    }
                    if k2 < duration_secs {
                        hits.push(DrumHit { time_secs: k2, voice: DrumVoice::Kick808Sub, velocity: 0.85, pan: 0.0 });
                    }
                    if k3 < duration_secs {
                        hits.push(DrumHit { time_secs: k3, voice: DrumVoice::Kick808Sub, velocity: 0.90, pan: 0.0 });
                    }
                    // Hard Snare on 3 (half-time trap) or 2 & 4 (boom-bap)
                    let snare_t1 = bar_start + 2.0 * beat_sec;
                    if snare_t1 < duration_secs {
                        hits.push(DrumHit { time_secs: snare_t1, voice: DrumVoice::SnareAcoustic, velocity: 0.95, pan: 0.0 });
                    }
                    if rhythm == RhythmGroove::BoomBap {
                        let snare_t2 = bar_start + 1.0 * beat_sec;
                        let snare_t3 = bar_start + 3.0 * beat_sec;
                        if snare_t2 < duration_secs {
                            hits.push(DrumHit { time_secs: snare_t2, voice: DrumVoice::SnareAcoustic, velocity: 0.90, pan: 0.0 });
                        }
                        if snare_t3 < duration_secs {
                            hits.push(DrumHit { time_secs: snare_t3, voice: DrumVoice::SnareAcoustic, velocity: 0.90, pan: 0.0 });
                        }
                    }
                    // Trap 8th notes and 16th/triplet hi-hat rolls
                    for step in 0..16 {
                        let t = bar_start + step as f32 * (beat_sec / 4.0);
                        if t < duration_secs {
                            let vel = if step % 4 == 0 { 0.75 } else if step % 2 == 0 { 0.55 } else { 0.40 };
                            let voice = if step >= 12 && step <= 14 { DrumVoice::HiHatTriplet } else { DrumVoice::HiHatClosed };
                            hits.push(DrumHit { time_secs: t, voice, velocity: vel, pan: -0.2 });
                        }
                    }
                }

                // 5. Afrobeats & Amapiano & Highlife (108–114 BPM):
                // 3:2 Tresillo clave syncopation, shaker loop, Amapiano percussive log drums.
                RhythmGroove::AfrobeatTresillo | RhythmGroove::AmapianoLogDrum | RhythmGroove::HighlifeSyncopation => {
                    // Shaker 16th-note loop across the bar
                    for s in 0..16 {
                        let st = bar_start + s as f32 * (beat_sec / 4.0);
                        if st < duration_secs {
                            let accent = if s % 4 == 0 || s % 3 == 0 { 0.70 } else { 0.40 };
                            hits.push(DrumHit { time_secs: st, voice: DrumVoice::Shaker, velocity: accent, pan: 0.35 });
                        }
                    }
                    // 3:2 Tresillo Clave: Hits on 0.0, 0.75, 1.5, 2.0, 3.0 beats
                    let clave_steps = [0.0f32, 0.75, 1.5, 2.0, 3.0];
                    for &step in &clave_steps {
                        let ct = bar_start + step * beat_sec;
                        if ct < duration_secs {
                            hits.push(DrumHit { time_secs: ct, voice: DrumVoice::Clave, velocity: 0.80, pan: -0.3 });
                        }
                    }
                    // Amapiano Log-Drum pitch drops
                    let log_steps = [0.0f32, 1.75, 2.5, 3.25];
                    for &step in &log_steps {
                        let lt = bar_start + step * beat_sec;
                        if lt < duration_secs {
                            hits.push(DrumHit { time_secs: lt, voice: DrumVoice::LogDrumAmapiano, velocity: 0.90, pan: 0.0 });
                        }
                    }
                    // Soft kick on beat 1 and 3
                    if bar_start < duration_secs {
                        hits.push(DrumHit { time_secs: bar_start, voice: DrumVoice::KickAcoustic, velocity: 0.85, pan: 0.0 });
                    }
                    let k3 = bar_start + 2.0 * beat_sec;
                    if k3 < duration_secs {
                        hits.push(DrumHit { time_secs: k3, voice: DrumVoice::KickAcoustic, velocity: 0.80, pan: 0.0 });
                    }
                }

                // 6. Rock & Metal (128–180 BPM):
                // Driving acoustic double-kick, heavy backbeat snare on 2 and 4.
                RhythmGroove::RockBackbeat | RhythmGroove::MetalDoubleKick => {
                    let is_double_kick = rhythm == RhythmGroove::MetalDoubleKick;
                    if is_double_kick {
                        // Continuous 16th-note double kick
                        for s in 0..16 {
                            let kt = bar_start + s as f32 * (beat_sec / 4.0);
                            if kt < duration_secs {
                                hits.push(DrumHit { time_secs: kt, voice: DrumVoice::KickAcoustic, velocity: 0.88, pan: (s % 2) as f32 * 0.1 - 0.05 });
                            }
                        }
                    } else {
                        // Standard Rock kick on 1 & 3 (and 3-and)
                        let k1 = bar_start;
                        let k2 = bar_start + 2.0 * beat_sec;
                        let k3 = bar_start + 2.5 * beat_sec;
                        if k1 < duration_secs { hits.push(DrumHit { time_secs: k1, voice: DrumVoice::KickAcoustic, velocity: 0.95, pan: 0.0 }); }
                        if k2 < duration_secs { hits.push(DrumHit { time_secs: k2, voice: DrumVoice::KickAcoustic, velocity: 0.90, pan: 0.0 }); }
                        if k3 < duration_secs { hits.push(DrumHit { time_secs: k3, voice: DrumVoice::KickAcoustic, velocity: 0.75, pan: 0.0 }); }
                    }
                    // Heavy Snare on 2 & 4
                    let s2 = bar_start + 1.0 * beat_sec;
                    let s4 = bar_start + 3.0 * beat_sec;
                    if s2 < duration_secs { hits.push(DrumHit { time_secs: s2, voice: DrumVoice::SnareAcoustic, velocity: 0.98, pan: 0.0 }); }
                    if s4 < duration_secs { hits.push(DrumHit { time_secs: s4, voice: DrumVoice::SnareAcoustic, velocity: 0.98, pan: 0.0 }); }

                    // Driving 8th-note Ride/Cymbal
                    for b in 0..8 {
                        let ct = bar_start + b as f32 * (beat_sec / 2.0);
                        if ct < duration_secs {
                            hits.push(DrumHit { time_secs: ct, voice: DrumVoice::JazzRideCymbal, velocity: 0.70, pan: 0.3 });
                        }
                    }
                    // Crash on downbeat of bar 0 and bar 4
                    if bar % 4 == 0 && bar_start < duration_secs {
                        hits.push(DrumHit { time_secs: bar_start, voice: DrumVoice::CrashCymbal, velocity: 0.90, pan: -0.4 });
                    }
                }

                // 7. Jazz Swing (110–160 BPM):
                // Swung ride cymbal (ding-ding-a-ding triplet feel), feathered kick, ghost snares.
                RhythmGroove::JazzSwingRide => {
                    for b in 0..4 {
                        let beat_t = bar_start + b as f32 * beat_sec;
                        // Feathered soft kick on every beat
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::KickAcoustic, velocity: 0.35, pan: 0.0 });
                        }
                        // Swung Ride Cymbal: beat 1, beat 2 + triplet offbeat, beat 3, beat 4 + triplet offbeat
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::JazzRideCymbal, velocity: 0.75, pan: 0.35 });
                        }
                        if b == 1 || b == 3 {
                            let trip_t = beat_t + beat_sec * (2.0 / 3.0); // 3rd triplet partial
                            if trip_t < duration_secs {
                                hits.push(DrumHit { time_secs: trip_t, voice: DrumVoice::JazzRideCymbal, velocity: 0.60, pan: 0.35 });
                            }
                        }
                        // Hi-hat foot chick on 2 & 4
                        if (b == 1 || b == 3) && beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::HiHatClosed, velocity: 0.50, pan: -0.3 });
                        }
                    }
                }

                // 8. Orchestral Percussion (60–100 BPM, 6/8 compound or 4/4):
                // Timpani rolls, concert snare, heavy crash swells.
                RhythmGroove::CinematicPercussion | RhythmGroove::WaltzThreeFour | RhythmGroove::GospelShuffle => {
                    // Dotted quarter note pulse in 6/8 or 3/4
                    let timpani_hit1 = bar_start;
                    let timpani_hit2 = bar_start + 2.0 * beat_sec;
                    if timpani_hit1 < duration_secs {
                        hits.push(DrumHit { time_secs: timpani_hit1, voice: DrumVoice::OrchestralTimpani, velocity: 0.90, pan: -0.2 });
                    }
                    if timpani_hit2 < duration_secs {
                        hits.push(DrumHit { time_secs: timpani_hit2, voice: DrumVoice::OrchestralTimpani, velocity: 0.85, pan: 0.2 });
                    }
                    // Concert snare roll leading to bar downbeat
                    for r in 0..8 {
                        let roll_t = bar_start + 3.0 * beat_sec + r as f32 * (beat_sec / 8.0);
                        if roll_t < duration_secs {
                            let crescendo = 0.3 + 0.6 * (r as f32 / 8.0);
                            hits.push(DrumHit { time_secs: roll_t, voice: DrumVoice::SnareAcoustic, velocity: crescendo, pan: 0.0 });
                        }
                    }
                }

                // Fallback / Standard backbeat for other grooves (BossaNova, Salsa, etc.):
                _ => {
                    for b in 0..4 {
                        let beat_t = bar_start + b as f32 * beat_sec;
                        if (b == 0 || b == 2) && beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::KickAcoustic, velocity: 0.85, pan: 0.0 });
                        } else if (b == 1 || b == 3) && beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::SnareAcoustic, velocity: 0.85, pan: 0.0 });
                        }
                        if beat_t < duration_secs {
                            hits.push(DrumHit { time_secs: beat_t, voice: DrumVoice::HiHatClosed, velocity: 0.55, pan: -0.1 });
                        }
                    }
                }
            }
        }
        hits
    }

    /// Renders drum hits into the stereo PCM AudioBuffer.
    pub fn render_hits(hits: &[DrumHit], buffer: &mut AudioBuffer) {
        let frames = buffer.frame_count();
        let mut rng = DspRng::new(0xABCD_1234);

        for hit in hits {
            let start_frame = (hit.time_secs * SAMPLE_RATE as f32).round() as usize;
            if start_frame >= frames {
                continue;
            }

            let voice_duration = Self::voice_duration_secs(hit.voice);
            let hit_frames = (voice_duration * SAMPLE_RATE as f32).round() as usize;
            let end_frame = (start_frame + hit_frames).min(frames);

            let pan_l = ((1.0 - hit.pan) * 0.5).sqrt();
            let pan_r = ((1.0 + hit.pan) * 0.5).sqrt();

            for f in start_frame..end_frame {
                let t = (f - start_frame) as f32 / SAMPLE_RATE as f32;
                let sample_val = Self::synthesize_sample(hit.voice, t, hit.velocity, &mut rng);

                buffer.samples[f * 2] = (buffer.samples[f * 2] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
                buffer.samples[f * 2 + 1] = (buffer.samples[f * 2 + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
            }
        }
    }

    pub fn voice_duration_secs(voice: DrumVoice) -> f32 {
        match voice {
            DrumVoice::Kick808Sub => 0.85,
            DrumVoice::KickAcoustic | DrumVoice::KickEdmPunch => 0.35,
            DrumVoice::SnareAcoustic | DrumVoice::SnareRimshot => 0.28,
            DrumVoice::Clap => 0.22,
            DrumVoice::HiHatClosed | DrumVoice::HiHatTriplet => 0.06,
            DrumVoice::HiHatOpen => 0.40,
            DrumVoice::LogDrumAmapiano => 0.45,
            DrumVoice::NyabinghiFunde | DrumVoice::NyabinghiAkete => 0.25,
            DrumVoice::NyabinghiThunder => 0.65,
            DrumVoice::Clave => 0.08,
            DrumVoice::Shaker => 0.07,
            DrumVoice::JazzRideCymbal => 0.60,
            DrumVoice::OrchestralTimpani => 1.50,
            DrumVoice::CrashCymbal => 1.20,
        }
    }

    #[inline]
    pub fn synthesize_sample(voice: DrumVoice, t: f32, _velocity: f32, rng: &mut DspRng) -> f32 {
        match voice {
            // 808 Sub-Bass: Pitch drops 130 Hz -> 42 Hz, long decay, warm saturation
            DrumVoice::Kick808Sub => {
                let pitch = 42.0 + 88.0 * (-t / 0.038).exp();
                let env = (-t / 0.40).exp();
                let click = if t < 0.005 { (2.0 * PI * 2400.0 * t).sin() * 0.3 } else { 0.0 };
                let body = (2.0 * PI * pitch * t).sin();
                (1.3 * (body * env + click)).tanh() * 0.85
            }

            // Punchy EDM / Acoustic Kick: Rapid 180 Hz -> 55 Hz sweep, 220 ms decay
            DrumVoice::KickAcoustic | DrumVoice::KickEdmPunch => {
                let pitch = 55.0 + 125.0 * (-t / 0.022).exp();
                let env = (1.0 - t / 0.35).max(0.0).powi(3);
                let click = if t < 0.004 { rng.next_f32() * 0.4 } else { 0.0 };
                let body = (2.0 * PI * pitch * t).sin();
                ((body + click) * env).clamp(-0.95, 0.95) * 0.88
            }

            // Snare: Dual sines (185 Hz & 330 Hz) + bandpass noise
            DrumVoice::SnareAcoustic => {
                let body_env = (1.0 - t / 0.12).max(0.0).powi(2);
                let body = ((2.0 * PI * 185.0 * t).sin() * 0.6 + (2.0 * PI * 330.0 * t).sin() * 0.4) * body_env;
                let noise_env = (-t / 0.16).exp();
                let noise = rng.next_f32() * noise_env;
                (body * 0.45 + noise * 0.55).clamp(-0.95, 0.95) * 0.82
            }

            // Reggae Rimshot / Cross-Stick: High wooden ring (1350 Hz)
            DrumVoice::SnareRimshot => {
                let env = (-t / 0.045).exp();
                let shell = (2.0 * PI * 1350.0 * t).sin() * env;
                let pop = if t < 0.006 { rng.next_f32() * 0.5 } else { 0.0 };
                (shell * 0.7 + pop * 0.3).clamp(-0.95, 0.95) * 0.85
            }

            // Handclap: 3 pre-bursts (0, 11, 22 ms) + main tail (33 ms)
            DrumVoice::Clap => {
                let is_preburst = (t < 0.008) || (t >= 0.011 && t < 0.019) || (t >= 0.022 && t < 0.030);
                let is_main = t >= 0.033;
                let env = if is_preburst {
                    0.6
                } else if is_main {
                    (-((t - 0.033) / 0.10)).exp()
                } else {
                    0.0
                };
                let n = rng.next_f32();
                (n * env).clamp(-0.95, 0.95) * 0.78
            }

            // Closed Hi-Hat: High metallic noise, ultra-fast decay
            DrumVoice::HiHatClosed | DrumVoice::HiHatTriplet => {
                let env = (-t / 0.035).exp();
                let n = rng.next_f32();
                (n * env).clamp(-0.90, 0.90) * 0.60
            }

            // Open Hi-Hat: Metallic noise with sustained sizzle
            DrumVoice::HiHatOpen => {
                let env = (-t / 0.30).exp();
                let n = rng.next_f32();
                (n * env).clamp(-0.90, 0.90) * 0.65
            }

            // Amapiano Log-Drum: Resonant pitch drop (190 Hz -> 50 Hz) + wooden overtone
            DrumVoice::LogDrumAmapiano => {
                let pitch = 50.0 + 140.0 * (-t / 0.045).exp();
                let env = (-t / 0.28).exp();
                let fund = (2.0 * PI * pitch * t).sin();
                let overtone = (2.0 * PI * pitch * 3.0 * t).sin() * 0.2;
                ((fund + overtone) * env * 1.2).tanh() * 0.90
            }

            // Nyabinghi Funde (heartbeat hand drum: 115 Hz)
            DrumVoice::NyabinghiFunde => {
                let env = (-t / 0.16).exp();
                let slap = if t < 0.008 { rng.next_f32() * 0.3 } else { 0.0 };
                let body = (2.0 * PI * 115.0 * t).sin();
                ((body + slap) * env).clamp(-0.95, 0.95) * 0.80
            }

            // Nyabinghi Akete / Peete (high hand slap: 380 Hz -> 260 Hz)
            DrumVoice::NyabinghiAkete => {
                let pitch = 260.0 + 120.0 * (-t / 0.030).exp();
                let env = (-t / 0.11).exp();
                let body = (2.0 * PI * pitch * t).sin();
                (body * env).clamp(-0.95, 0.95) * 0.75
            }

            // Nyabinghi Thunder (deep bass acoustic drop: 55 Hz)
            DrumVoice::NyabinghiThunder => {
                let env = (-t / 0.45).exp();
                let body = (2.0 * PI * 55.0 * t).sin();
                (body * env * 1.1).tanh() * 0.85
            }

            // Clave: Resonant wooden strike (2200 Hz & 2700 Hz)
            DrumVoice::Clave => {
                let env = (-t / 0.040).exp();
                let ring = (2.0 * PI * 2200.0 * t).sin() * 0.6 + (2.0 * PI * 2700.0 * t).sin() * 0.4;
                (ring * env).clamp(-0.95, 0.95) * 0.75
            }

            // Afrobeat Shaker: Swelling noise packet
            DrumVoice::Shaker => {
                let env = (t / 0.015).min(1.0) * (-t / 0.045).exp();
                let n = rng.next_f32();
                (n * env).clamp(-0.90, 0.90) * 0.55
            }

            // Jazz Swung Ride Cymbal: Bell ping (850 Hz) + high sizzle wash
            DrumVoice::JazzRideCymbal => {
                let env = (-t / 0.45).exp();
                let ping = (2.0 * PI * 850.0 * t).sin() * 0.35 * (-t / 0.08).exp();
                let sizzle = rng.next_f32() * 0.65;
                ((ping + sizzle) * env).clamp(-0.90, 0.90) * 0.65
            }

            // Orchestral Timpani: Resonant low membrane modes (82 Hz, 119 Hz, 160 Hz)
            DrumVoice::OrchestralTimpani => {
                let attack = (t / 0.020).min(1.0);
                let decay = (-t / 1.10).exp();
                let env = attack * decay;
                let m1 = (2.0 * PI * 82.0 * t).sin() * 0.60;
                let m2 = (2.0 * PI * 119.0 * t).sin() * 0.25;
                let m3 = (2.0 * PI * 160.0 * t).sin() * 0.15;
                ((m1 + m2 + m3) * env * 1.15).tanh() * 0.88
            }

            // Crash Cymbal: Dense explosive burst + long metallic wash
            DrumVoice::CrashCymbal => {
                let env = (-t / 0.85).exp();
                let n = rng.next_f32();
                (n * env).clamp(-0.90, 0.90) * 0.70
            }
        }
    }
}

/// Convenience standalone function conforming to PROJECT.md interface contract:
/// `generate_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer`
pub fn generate_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer {
    UniversalDrumSynthesizer::generate_groove(genre, bpm, duration_secs)
}

/// Compatibility alias matching `synthesize_groove`.
pub fn synthesize_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer {
    generate_groove(genre, bpm, duration_secs)
}
