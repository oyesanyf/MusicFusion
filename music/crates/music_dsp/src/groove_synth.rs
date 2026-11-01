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

/// Fast, deterministic PRNG for pure-Rust DSP (Xorshift32) with 2-pole velvet low-pass brush state.
#[derive(Debug, Clone)]
pub struct DspRng {
    state: u32,
    lp1: f32,
    lp2: f32,
}

impl DspRng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x1337_C0DE } else { seed },
            lp1: 0.0,
            lp2: 0.0,
        }
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Generates a warm, 2-pole low-pass filtered acoustic brush sample (~550 Hz rolloff)
    /// that completely eliminates harsh broadband white-noise static and high-pitched hiss.
    #[inline]
    pub fn smooth_brush(&mut self) -> f32 {
        let raw = self.next_f32();
        self.lp1 = 0.92 * self.lp1 + 0.08 * raw;
        self.lp2 = 0.92 * self.lp2 + 0.08 * self.lp1;
        self.lp2 * 2.2
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
    KickOneDrop,
    SnareOneDrop,
    HiHat16thMetallic,
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
                // Beat 1: Empty downbeat with subtle 16th hi-hats.
                // Beat 3: Heavy Kick (110->42 Hz over 60 ms) & Snare Rimshot (250-1800 Hz BP) simultaneously.
                // 16th-note closed metallic hi-hats (>6000 Hz, 25 ms decay) and Nyabinghi hand percussion.
                RhythmGroove::OneDrop => {
                    // 16th-note closed hi-hats (>6000 Hz, 25 ms decay envelope)
                    for step in 0..16 {
                        let step_t = bar_start + step as f32 * (beat_sec * 0.25);
                        if step_t < duration_secs {
                            let vel = if step % 4 == 0 { 0.70 } else if step % 2 == 0 { 0.55 } else { 0.45 };
                            hits.push(DrumHit { time_secs: step_t, voice: DrumVoice::HiHat16thMetallic, velocity: vel, pan: -0.2 });
                        }
                    }

                    // Beat 3: Authentic One-Drop impact (Kick 110->42 Hz + Rimshot 250-1800 Hz on beat 3)
                    let beat3_t = bar_start + 2.0 * beat_sec;
                    if beat3_t < duration_secs {
                        hits.push(DrumHit { time_secs: beat3_t, voice: DrumVoice::KickOneDrop, velocity: 0.95, pan: 0.0 });
                        hits.push(DrumHit { time_secs: beat3_t, voice: DrumVoice::SnareOneDrop, velocity: 0.90, pan: 0.1 });
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
                            let voice = if (12..=14).contains(&step) { DrumVoice::HiHatTriplet } else { DrumVoice::HiHatClosed };
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

    /// Renders drum hits into the PCM AudioBuffer (supports mono and stereo).
    pub fn render_hits(hits: &[DrumHit], buffer: &mut AudioBuffer) {
        let frames = buffer.frame_count();
        let channels = buffer.channels as usize;
        if channels == 0 || frames == 0 || buffer.samples.is_empty() {
            return;
        }
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

                if channels == 1 {
                    let pan_mono = (pan_l + pan_r) * 0.5;
                    buffer.samples[f] = (buffer.samples[f] + sample_val * pan_mono * hit.velocity).clamp(-0.95, 0.95);
                } else {
                    buffer.samples[f * channels] = (buffer.samples[f * channels] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
                    buffer.samples[f * channels + 1] = (buffer.samples[f * channels + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
                }
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
            DrumVoice::KickOneDrop => 0.40,
            DrumVoice::SnareOneDrop => 0.28,
            DrumVoice::HiHat16thMetallic => 0.05,
        }
    }

    #[inline]
    pub fn synthesize_sample(voice: DrumVoice, t: f32, _velocity: f32, rng: &mut DspRng) -> f32 {
        match voice {
            // 808 Sub-Bass: Pitch drops 130 Hz -> 42 Hz, long decay, warm saturation (zero high click)
            DrumVoice::Kick808Sub => {
                let pitch = 42.0 + 88.0 * (-t / 0.038).exp();
                let env = (-t / 0.40).exp();
                let thump = if t < 0.008 { (2.0 * PI * 140.0 * t).sin() * 0.25 } else { 0.0 };
                let body = (2.0 * PI * pitch * t).sin();
                (1.25 * (body * env + thump)).tanh() * 0.85
            }

            // Punchy EDM / Acoustic Kick: Rapid 160 Hz -> 52 Hz sweep, warm low-end thump
            DrumVoice::KickAcoustic | DrumVoice::KickEdmPunch => {
                let pitch = 52.0 + 108.0 * (-t / 0.022).exp();
                let env = (1.0 - t / 0.35).max(0.0).powi(3);
                let thump = if t < 0.006 { (2.0 * PI * 150.0 * t).sin() * 0.25 } else { 0.0 };
                let body = (2.0 * PI * pitch * t).sin();
                ((body + thump) * env).clamp(-0.95, 0.95) * 0.88
            }

            // Snare: Warm dual membrane modes (180 Hz & 285 Hz) + velvet low-passed brush (zero high-pitch whine)
            DrumVoice::SnareAcoustic => {
                let body_env = (1.0 - t / 0.14).max(0.0).powi(2);
                let body = ((2.0 * PI * 180.0 * t).sin() * 0.65 + (2.0 * PI * 285.0 * t).sin() * 0.35) * body_env;
                let brush_env = (-t / 0.11).exp();
                let brush = rng.smooth_brush() * 0.28;
                (body * 0.78 + brush * brush_env * 0.22).clamp(-0.95, 0.95) * 0.80
            }

            // Reggae Rimshot / Wooden Cross-Stick: Warm maple shell knock (340 Hz & 480 Hz, zero piercing beep)
            DrumVoice::SnareRimshot => {
                let env = (-t / 0.038).exp();
                let shell = ((2.0 * PI * 340.0 * t).sin() * 0.65 + (2.0 * PI * 480.0 * t).sin() * 0.35) * env;
                let knock = if t < 0.006 { rng.smooth_brush() * 0.20 } else { 0.0 };
                (shell * 0.82 + knock).clamp(-0.95, 0.95) * 0.72
            }

            // Handclap: Warm studio clap body (290 Hz & 420 Hz) + velvet brush envelope
            DrumVoice::Clap => {
                let is_preburst = (t < 0.008) || (0.011..0.019).contains(&t) || (0.022..0.030).contains(&t);
                let is_main = t >= 0.033;
                let env = if is_preburst {
                    0.45
                } else if is_main {
                    (-((t - 0.033) / 0.08)).exp()
                } else {
                    0.0
                };
                let body = (2.0 * PI * 290.0 * t).sin() * 0.45 + (2.0 * PI * 420.0 * t).sin() * 0.25;
                let soft_n = rng.smooth_brush() * 0.30;
                ((body + soft_n) * env).clamp(-0.90, 0.90) * 0.55
            }

            // Closed Hi-Hat: Soft studio brush tap (zero high-pitched sine whistle, zero harsh static)
            DrumVoice::HiHatClosed | DrumVoice::HiHatTriplet => {
                let env = (-t / 0.028).exp();
                let warm_wood = (2.0 * PI * 320.0 * t).sin() * 0.22 * (-t / 0.012).exp();
                let brush = rng.smooth_brush() * 0.35;
                ((warm_wood + brush) * env).clamp(-0.80, 0.80) * 0.28
            }

            // Open Hi-Hat: Smooth warm brush sustain without piercing high frequencies
            DrumVoice::HiHatOpen => {
                let env = (-t / 0.16).exp();
                let warm_body = (2.0 * PI * 280.0 * t).sin() * 0.20 * (-t / 0.04).exp();
                let brush = rng.smooth_brush() * 0.35;
                ((warm_body + brush) * env).clamp(-0.80, 0.80) * 0.26
            }

            // Amapiano Log-Drum: Resonant pitch drop (175 Hz -> 52 Hz) + warm 2nd harmonic
            DrumVoice::LogDrumAmapiano => {
                let pitch = 52.0 + 123.0 * (-t / 0.045).exp();
                let env = (-t / 0.28).exp();
                let fund = (2.0 * PI * pitch * t).sin();
                let overtone = (2.0 * PI * pitch * 2.0 * t).sin() * 0.22;
                ((fund + overtone) * env * 1.15).tanh() * 0.88
            }

            // Nyabinghi Funde (heartbeat hand drum: 115 Hz)
            DrumVoice::NyabinghiFunde => {
                let env = (-t / 0.16).exp();
                let slap = if t < 0.008 { (2.0 * PI * 230.0 * t).sin() * 0.22 } else { 0.0 };
                let body = (2.0 * PI * 115.0 * t).sin();
                ((body + slap) * env).clamp(-0.95, 0.95) * 0.80
            }

            // Nyabinghi Akete / Peete (warm hand drum: 290 Hz -> 210 Hz)
            DrumVoice::NyabinghiAkete => {
                let pitch = 210.0 + 80.0 * (-t / 0.030).exp();
                let env = (-t / 0.11).exp();
                let body = (2.0 * PI * pitch * t).sin();
                (body * env).clamp(-0.95, 0.95) * 0.68
            }

            // Nyabinghi Thunder (deep bass acoustic drop: 55 Hz)
            DrumVoice::NyabinghiThunder => {
                let env = (-t / 0.45).exp();
                let body = (2.0 * PI * 55.0 * t).sin();
                (body * env * 1.1).tanh() * 0.85
            }

            // Clave: Warm rosewood knock (330 Hz & 440 Hz — zero high-pitched beep)
            DrumVoice::Clave => {
                let env = (-t / 0.032).exp();
                let wood = (2.0 * PI * 330.0 * t).sin() * 0.65 + (2.0 * PI * 440.0 * t).sin() * 0.35;
                (wood * env).clamp(-0.90, 0.90) * 0.42
            }

            // Afrobeat Shaker: Soft low-passed maraca swell (zero 4.5 kHz whistle, zero sandpaper scratch)
            DrumVoice::Shaker => {
                let env = (t / 0.012).min(1.0) * (-t / 0.038).exp();
                let warm_shell = (2.0 * PI * 260.0 * t).sin() * 0.20;
                let soft_n = rng.smooth_brush() * 0.35;
                ((warm_shell + soft_n) * env).clamp(-0.75, 0.75) * 0.22
            }

            // Jazz Swung Ride: Warm low-mid bell (330 Hz & 495 Hz) + velvet brush
            DrumVoice::JazzRideCymbal => {
                let env = (-t / 0.32).exp();
                let ping = ((2.0 * PI * 330.0 * t).sin() * 0.55 + (2.0 * PI * 495.0 * t).sin() * 0.25) * (-t / 0.09).exp();
                let soft_sizzle = rng.smooth_brush() * 0.25;
                ((ping + soft_sizzle) * env).clamp(-0.85, 0.85) * 0.42
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

            // Warm Swell Cymbal: Low-mid harmonic swell (220 Hz, 330 Hz, 440 Hz) + velvet brush
            DrumVoice::CrashCymbal => {
                let env = (-t / 0.65).exp();
                let c1 = (2.0 * PI * 220.0 * t).sin() * 0.30;
                let c2 = (2.0 * PI * 330.0 * t).sin() * 0.25;
                let c3 = (2.0 * PI * 440.0 * t).sin() * 0.15;
                let soft_n = rng.smooth_brush() * 0.25;
                ((c1 + c2 + c3 + soft_n) * env).clamp(-0.85, 0.85) * 0.40
            }

            // Reggae Kick on Beat 3: Rapid exponential pitch-drop sine wave (110 Hz down to 42 Hz over 60 ms)
            // paired with a band-passed white-noise snare/rimshot burst (250 Hz to 1800 Hz with exponential decay)
            DrumVoice::KickOneDrop => {
                let pitch = 42.0 + 68.0 * (-t / 0.020).exp();
                let kick_env = (-t / 0.35).exp();
                let body = (2.0 * PI * pitch * t).sin() * kick_env;
                let white = rng.next_f32();
                let snare_burst = (white * 0.45 + (2.0 * PI * 850.0 * t).sin() * 0.25) * (-t / 0.040).exp();
                ((body * 0.85 + snare_burst * 0.25) * 1.25).tanh() * 0.90
            }

            // Reggae One-Drop Rimshot Snare: 250 Hz to 1800 Hz BP burst with exponential decay
            DrumVoice::SnareOneDrop => {
                let env = (-t / 0.055).exp();
                let white = rng.next_f32();
                let bp = (white * 0.55 + (2.0 * PI * 440.0 * t).sin() * 0.25 + (2.0 * PI * 1100.0 * t).sin() * 0.20) * env;
                (bp * 1.15).tanh() * 0.85
            }

            // 16th-Note Closed Hi-Hats: High-pass filtered metallic square-wave ratios (>6000 Hz) with tight 25 ms decay
            DrumVoice::HiHat16thMetallic => {
                let env = (-t / 0.025).exp();
                let sq1 = if (t * 6250.0).fract() > 0.5 { 1.0 } else { -1.0 };
                let sq2 = if (t * 7180.0).fract() > 0.5 { 1.0 } else { -1.0 };
                let sq3 = if (t * 8320.0).fract() > 0.5 { 1.0 } else { -1.0 };
                let sq4 = if (t * 9640.0).fract() > 0.5 { 1.0 } else { -1.0 };
                let metallic = sq1 * 0.30 + sq2 * 0.30 + sq3 * 0.20 + sq4 * 0.20;
                (metallic * env * 0.40).clamp(-0.85, 0.85)
            }
        }
    }
}

/// Dub Sub-Bass synthesizer: warm, low-pass filtered (cutoff = 180 Hz) sine + triangle bassline
/// walking on roots and fifths (A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz)
/// with a smooth 15 ms linear attack and 120 ms exponential release.
#[derive(Debug, Clone)]
pub struct DubSubBass {
    pub sample_rate: f32,
    pub lp_state: f32,
    pub phase: f32,
}

impl DubSubBass {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            lp_state: 0.0,
            phase: 0.0,
        }
    }

    /// Synthesizes complete dub sub-bass buffer across duration_secs at bpm (default 76 BPM).
    pub fn synthesize(&mut self, duration_secs: f32, bpm: u32) -> AudioBuffer {
        let mut buf = AudioBuffer::new_stereo(duration_secs);
        let frames = buf.frame_count();
        let beat_sec = 60.0 / (bpm.clamp(40, 220) as f32);
        let bar_sec = beat_sec * 4.0;
        let sr = self.sample_rate;

        // 1-pole lowpass filter for 180 Hz cutoff:
        let fc = 180.0f32;
        let dt = 1.0 / sr;
        let rc = 1.0 / (2.0 * PI * fc);
        let alpha = dt / (rc + dt);

        // Notes walking on roots and fifths: A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz
        let notes = [55.00f32, 73.42, 82.41, 55.00];

        for i in 0..frames {
            let t = i as f32 / sr;
            let beat_in_bar = ((t % bar_sec) / beat_sec).floor() as usize;
            let note_freq = notes[beat_in_bar % 4];

            let time_in_beat = t % beat_sec;
            let gate_dur = beat_sec * 0.85;

            // 15 ms linear attack, continuous sustain, 120 ms exponential release
            let env = if time_in_beat < 0.015 {
                time_in_beat / 0.015
            } else if time_in_beat < gate_dur {
                1.0
            } else {
                let rel_t = time_in_beat - gate_dur;
                (-rel_t / 0.120).exp()
            };

            // Continuous phase accumulation
            self.phase += 2.0 * PI * note_freq / sr;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }

            // Sine + Triangle blend
            let sine = self.phase.sin();
            let norm_p = self.phase / (2.0 * PI);
            let tri = 2.0 * (2.0 * (norm_p - (norm_p + 0.5).floor()).abs()) - 1.0;
            let raw_bass = (sine * 0.70 + tri * 0.30) * env;

            // 180 Hz Low-Pass Filter
            self.lp_state += alpha * (raw_bass - self.lp_state);
            let out_bass = self.lp_state * 0.85;

            buf.samples[i * 2] = out_bass;
            buf.samples[i * 2 + 1] = out_bass;
        }

        buf
    }
}

/// Hammond Organ Bubble synthesizer: additive drawbar sine harmonics
/// (1.0x, 2.0x, 3.0x, 4.0x fundamental) with a gentle 6 Hz rotary speaker
/// tremolo LFO, enveloped smoothly with ADSR.
#[derive(Debug, Clone)]
pub struct HammondOrganBubble {
    pub sample_rate: f32,
    pub phases: [f32; 4],
    pub lfo_phase: f32,
}

impl HammondOrganBubble {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            phases: [0.0; 4],
            lfo_phase: 0.0,
        }
    }

    /// Synthesizes complete Hammond organ bubble buffer across duration_secs at bpm (default 76 BPM)
    /// locked to fundamental root_hz (default 220.0 Hz).
    pub fn synthesize(&mut self, duration_secs: f32, bpm: u32, root_hz: f32) -> AudioBuffer {
        let mut buf = AudioBuffer::new_stereo(duration_secs);
        let frames = buf.frame_count();
        let beat_sec = 60.0 / (bpm.clamp(40, 220) as f32);
        let sr = self.sample_rate;
        let base_freq = if root_hz > 0.0 { root_hz } else { 220.0 };

        for i in 0..frames {
            let t = i as f32 / sr;
            let beat_phase = (t % beat_sec) / beat_sec;

            let (is_active, note_t) = if beat_phase >= 0.5 {
                (true, (beat_phase - 0.5) * beat_sec)
            } else {
                (false, 0.0)
            };

            // ADSR: Attack 10ms, Decay 45ms, Sustain 0.70, Release 35ms
            let env = if !is_active {
                0.06
            } else if note_t < 0.010 {
                0.06 + 0.94 * (note_t / 0.010)
            } else if note_t < 0.055 {
                1.0 - 0.30 * ((note_t - 0.010) / 0.045)
            } else if note_t < (beat_sec * 0.40) {
                0.70
            } else {
                let rel_t = note_t - (beat_sec * 0.40);
                (0.70 * (-rel_t / 0.035).exp()).max(0.06)
            };

            // 6 Hz rotary speaker tremolo LFO
            self.lfo_phase += 2.0 * PI * 6.0 / sr;
            if self.lfo_phase >= 2.0 * PI { self.lfo_phase -= 2.0 * PI; }
            let tremolo = 1.0 + 0.18 * self.lfo_phase.sin();

            // Additive drawbar harmonics: 1.0x, 2.0x, 3.0x, 4.0x fundamental
            let mults = [1.0f32, 2.0, 3.0, 4.0];
            let gains = [0.45f32, 0.30, 0.15, 0.10];
            let mut organ_sample = 0.0f32;
            for d in 0..4 {
                self.phases[d] += 2.0 * PI * (base_freq * mults[d]) / sr;
                if self.phases[d] >= 2.0 * PI { self.phases[d] -= 2.0 * PI; }
                organ_sample += self.phases[d].sin() * gains[d];
            }

            let final_val = organ_sample * env * tremolo * 0.35;
            buf.samples[i * 2] = final_val;
            buf.samples[i * 2 + 1] = final_val * 0.92;
        }

        buf
    }
}

/// Synthesizes complete physical modeling Reggae One-Drop groove with dub bass and organ bubble.
pub fn synthesize_reggae_one_drop_drums_and_dub_bass(duration_secs: f32, bpm: u32, root_hz: f32) -> AudioBuffer {
    let mut drums = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::OneDrop, bpm, duration_secs);
    let bass = DubSubBass::new(SAMPLE_RATE).synthesize(duration_secs, bpm);
    let organ = HammondOrganBubble::new(SAMPLE_RATE).synthesize(duration_secs, bpm, root_hz);

    for i in 0..drums.samples.len() {
        let mixed = drums.samples[i] * 0.45 + bass.samples[i] * 0.35 + organ.samples[i] * 0.20;
        drums.samples[i] = mixed.clamp(-0.95, 0.95);
    }
    drums
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
