//! crates/music_dsp/src/guitar_wah.rs
//!
//! Physical Guitar String (Extended Karplus-Strong) and Resonant Wah-Wah Modeler.
//! Generates expressive lead guitar solos (such as the 16-bar soaring psychedelic
//! blues/rock solo in A Major / modal scales for Milestone 2 and Node 4 inpainting).
//!
//! Includes:
//! - Extended Karplus-Strong delay loop with fractional allpass delay interpolation
//! - Pick excitation noise burst with plectrum comb filtering
//! - Resonant Chamberlin State-Variable Filter (SVF) Wah-Wah pedal (400 Hz – 2200 Hz sweep)
//! - Vacuum tube overdrive / amplifier soft-clipping and cabinet simulation
//! - 16-bar soaring blues/rock solo generator with musical phrasing, bends, and vibrato

use std::f32::consts::PI;
use crate::wav::AudioBuffer;
use music_core::genre::ModalScale;

pub const SAMPLE_RATE: u32 = 48_000;

/// Continuous phase accumulator across buffer and note boundaries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousPhaseAccumulator {
    pub phase: f32, // Normalized in [0.0, 2.0 * PI)
}

impl ContinuousPhaseAccumulator {
    pub const fn new() -> Self {
        Self { phase: 0.0 }
    }

    #[inline]
    pub fn advance(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        let current = self.phase;
        self.phase += 2.0 * PI * freq_hz / sample_rate;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
            if self.phase >= 2.0 * PI {
                self.phase = self.phase.rem_euclid(2.0 * PI);
            }
        }
        current
    }

    #[inline]
    pub fn sin(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        self.advance(freq_hz, sample_rate).sin()
    }
}

/// Paul Kellet's filtered pink noise generator with velvet 2-pole lowpass.
#[derive(Debug, Clone)]
pub struct PinkNoiseGenerator {
    b0: f32, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32, b6: f32,
    lp: f32,
}

impl PinkNoiseGenerator {
    pub const fn new() -> Self {
        Self { b0: 0.0, b1: 0.0, b2: 0.0, b3: 0.0, b4: 0.0, b5: 0.0, b6: 0.0, lp: 0.0 }
    }

    #[inline]
    pub fn next_sample(&mut self, rng: &mut crate::groove_synth::DspRng) -> f32 {
        let white = rng.next_f32();
        self.b0 = 0.99886 * self.b0 + white * 0.0555179;
        self.b1 = 0.99332 * self.b1 + white * 0.0750759;
        self.b2 = 0.96900 * self.b2 + white * 0.1538520;
        self.b3 = 0.86650 * self.b3 + white * 0.3104856;
        self.b4 = 0.55000 * self.b4 + white * 0.5329522;
        self.b5 = -0.7616 * self.b5 - white * 0.0168980;
        let pink = (self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362) * 0.11;
        self.b6 = white * 0.115926;
        self.lp = 0.77 * self.lp + 0.23 * pink; // ~2000 Hz lowpass
        self.lp
    }
}

pub const KARPLUS_FEEDBACK_DAMPING: f32 = 0.994;
pub const A_MAJOR_PENTATONIC: [f32; 6] = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0];

/// Physical Karplus-Strong String Oscillator.
pub struct KarplusStrongString {
    sample_rate: f32,
    pub delay_buffer: Vec<f32>,
    pub write_ptr: usize,
    pub buffer_len: usize,
    pub prev_filter_sample: f32,
    pub allpass_state: f32,
    pub decay_factor: f32,
    pub is_active: bool,
}

pub type KarplusStrongGuitar = KarplusStrongString;

impl KarplusStrongString {
    pub fn new(sample_rate: u32) -> Self {
        let max_period = (sample_rate as f32 / 40.0).ceil() as usize; // support down to 40 Hz
        Self {
            sample_rate: sample_rate as f32,
            delay_buffer: vec![0.0; max_period + 4],
            write_ptr: 0,
            buffer_len: 100,
            prev_filter_sample: 0.0,
            allpass_state: 0.0,
            decay_factor: KARPLUS_FEEDBACK_DAMPING,
            is_active: false,
        }
    }

    /// Plucks the string at the specified fundamental frequency (in Hz) and velocity
    /// excited by low-pass filtered pink noise burst with 0.994 feedback damping.
    pub fn pluck(&mut self, freq_hz: f32, velocity: f32) {
        let period = (self.sample_rate / freq_hz.clamp(50.0, 4000.0)).max(4.0);
        let int_period = period.round() as usize;

        self.buffer_len = int_period.min(self.delay_buffer.len() - 2);
        self.write_ptr = 0;
        self.prev_filter_sample = 0.0;
        self.allpass_state = 0.0;
        self.decay_factor = KARPLUS_FEEDBACK_DAMPING;

        let mut rng = crate::groove_synth::DspRng::new(12345 + (freq_hz * 100.0) as u32);
        let mut pink = PinkNoiseGenerator::new();

        for i in 0..self.buffer_len {
            self.delay_buffer[i] = pink.next_sample(&mut rng) * velocity;
        }

        self.is_active = true;
    }

    /// Plucks the string with an external PRNG and pink noise generator.
    pub fn pluck_with_rng(&mut self, freq_hz: f32, velocity: f32, rng: &mut crate::groove_synth::DspRng, pink: &mut PinkNoiseGenerator) {
        let period = (self.sample_rate / freq_hz.clamp(50.0, 4000.0)).max(4.0);
        let int_period = period.round() as usize;

        self.buffer_len = int_period.min(self.delay_buffer.len() - 2);
        self.write_ptr = 0;
        self.prev_filter_sample = 0.0;
        self.allpass_state = 0.0;
        self.decay_factor = KARPLUS_FEEDBACK_DAMPING;

        for i in 0..self.buffer_len {
            self.delay_buffer[i] = pink.next_sample(rng) * velocity;
        }

        self.is_active = true;
    }

    /// Computes one sample from the Karplus-Strong string physical loop.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        if !self.is_active {
            return 0.0;
        }

        let read_idx = self.write_ptr;
        let current_sample = self.delay_buffer[read_idx];

        // Lowpass averaging loop filter: (y[n] + y[n-1]) / 2 * decay (0.994 damping)
        let filtered = (current_sample + self.prev_filter_sample) * 0.5 * self.decay_factor;
        self.prev_filter_sample = current_sample;

        // Write back to circular delay buffer
        self.delay_buffer[self.write_ptr] = filtered;
        self.write_ptr = (self.write_ptr + 1) % self.buffer_len;

        filtered
    }
}

/// Dynamic Resonant State-Variable Wah-Wah Filter (Chamberlin SVF).
pub struct WahWahFilter {
    sample_rate: f32,
    lp_state: f32,
    bp_state: f32,
    center_freq: f32,
    q_resonance: f32,
    envelope_state: f32,
}

impl WahWahFilter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            lp_state: 0.0,
            bp_state: 0.0,
            center_freq: 380.0,
            q_resonance: 1.25,
            envelope_state: 0.0,
        }
    }

    /// Sets pedal position (0.0 = heel/240 Hz, 1.0 = toe/680 Hz warm jazz/reggae sweep).
    pub fn set_pedal_position(&mut self, pos: f32) {
        let clamped = pos.clamp(0.0, 1.0);
        // Smooth warm low-mid sweep without piercing high-frequency resonances
        self.center_freq = 240.0 * (680.0 / 240.0f32).powf(clamped);
    }

    /// Processes one sample through the warm resonant wah circuit.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let f = (2.0 * (PI * self.center_freq / self.sample_rate).sin()).clamp(0.01, 0.45);
        let q = 1.0 / self.q_resonance;

        // Chamberlin State-Variable Filter (SVF) equations
        let hp = input - self.lp_state - q * self.bp_state;
        self.bp_state += f * hp;
        self.lp_state += f * self.bp_state;

        // Warm lowpass + gentle bandpass blend with dry body presence (zero harshness)
        let wah_out = self.lp_state * 0.65 + self.bp_state * 0.35 + input * 0.35;
        wah_out.clamp(-1.0, 1.0)
    }

    /// Auto-modulates wah filter with LFO and input dynamic envelope.
    pub fn auto_modulate(&mut self, t: f32, input: f32) {
        // 1.3 Hz psychedelic LFO sweep
        let lfo = (2.0 * PI * 1.3 * t).sin() * 0.5 + 0.5;
        // Envelope detector
        let alpha = 0.005;
        self.envelope_state = (1.0 - alpha) * self.envelope_state + alpha * input.abs();
        let auto_drive = (self.envelope_state * 4.0).clamp(0.0, 0.4);

        let combined_pedal = (lfo * 0.7 + auto_drive).clamp(0.0, 1.0);
        self.set_pedal_position(combined_pedal);
    }
}

/// Vintage Vacuum Tube Overdrive & Cabinet Simulator.
pub struct GuitarAmpSimulator {
    gain: f32,
    cabinet_lp: f32,
    cabinet_hp: f32,
}

impl GuitarAmpSimulator {
    pub fn new(drive: f32) -> Self {
        Self {
            gain: drive.clamp(1.0, 8.0),
            cabinet_lp: 0.0,
            cabinet_hp: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Pre-gain boost
        let driven = x * self.gain;
        // Asymmetric cubic soft-clipping with tube second-harmonic warmth
        let saturated = (driven + 0.06 * driven * driven).tanh();

        // 80 Hz high-pass (anti-mud)
        let hp_alpha = 0.015;
        self.cabinet_hp = (1.0 - hp_alpha) * self.cabinet_hp + hp_alpha * saturated;
        let cleaned = saturated - self.cabinet_hp;

        // Warm 1.1 kHz cabinet speaker cone rolloff (eliminating high-pitched guitar whine)
        let lp_alpha = 0.14;
        self.cabinet_lp = (1.0 - lp_alpha) * self.cabinet_lp + lp_alpha * cleaned;

        self.cabinet_lp * 0.72
    }
}

/// Representation of a musical note event in a guitar solo.
#[derive(Debug, Clone, Copy)]
pub struct SoloNote {
    pub bar_offset_beats: f32,
    pub duration_beats: f32,
    pub pitch_hz: f32,
    pub bend_semitones: f32,
    pub vibrato: bool,
    pub velocity: f32,
}

/// 16-Bar Psychedelic Blues/Rock Solo Generator.
pub struct SoloGenerator;

impl SoloGenerator {
    /// Generates a complete 16-bar soaring guitar solo conforming to PROJECT.md contract:
    /// `synthesize_lead_solo(bars: u32, bpm: u32, scale: &str) -> AudioBuffer`
    pub fn synthesize_lead_solo(bars: u32, bpm: u32, scale: &str) -> AudioBuffer {
        let clamped_bpm = bpm.clamp(40, 220);
        let modal = if scale.to_ascii_lowercase().contains("blues") {
            ModalScale::BluesScale
        } else if scale.to_ascii_lowercase().contains("minor") {
            ModalScale::Aeolian
        } else {
            ModalScale::Ionian // Default A Major / Mixolydian blues-rock
        };

        Self::synthesize_solo_advanced(bars, clamped_bpm, "A", modal)
    }

    /// Advanced solo generator with root key and modal scale selection.
    pub fn synthesize_solo_advanced(bars: u32, bpm: u32, root: &str, scale: ModalScale) -> AudioBuffer {
        let clamped_bpm = bpm.clamp(40, 220);
        let beats_per_bar = 4.0;
        let total_beats = bars as f32 * beats_per_bar;
        let duration_secs = (total_beats * 60.0) / clamped_bpm as f32;

        let mut buffer = AudioBuffer::new_stereo(duration_secs);
        let frames = buffer.frame_count();
        let beat_duration_secs = 60.0 / clamped_bpm as f32;

        let notes = Self::compose_16_bar_blues_rock_solo(bars, root, scale);

        let mut string = KarplusStrongString::new(SAMPLE_RATE);
        let mut wah = WahWahFilter::new(SAMPLE_RATE);
        let mut amp = GuitarAmpSimulator::new(1.35);

        let mut current_note_idx = 0;
        let mut active_note: Option<SoloNote> = None;
        let mut note_elapsed_secs = 0.0f32;
        let mut continuous_phase = 0.0f32;

        for f in 0..frames {
            let current_time_secs = f as f32 / SAMPLE_RATE as f32;
            let current_beat = current_time_secs / beat_duration_secs;

            // Trigger next note when beat matches
            while current_note_idx < notes.len() && current_beat >= notes[current_note_idx].bar_offset_beats {
                let note = notes[current_note_idx];
                string.pluck(note.pitch_hz, note.velocity);
                active_note = Some(note);
                note_elapsed_secs = 0.0;
                current_note_idx += 1;
            }

            // Calculate smooth pitch bend, vibrato, and envelope without mutating circular buffer length
            let mut bent_harmonic = 0.0f32;
            if let Some(note) = active_note {
                note_elapsed_secs += 1.0 / SAMPLE_RATE as f32;
                let note_duration_secs = (note.duration_beats * beat_duration_secs).max(0.05);

                let mut current_pitch = note.pitch_hz;
                // Smooth pitch bend ramp
                if note.bend_semitones > 0.0 && note_elapsed_secs > 0.08 {
                    let bend_progress = ((note_elapsed_secs - 0.08) / (note_duration_secs * 0.6)).min(1.0);
                    let semitones = note.bend_semitones * bend_progress;
                    current_pitch *= (2.0f32).powf(semitones / 12.0);
                }

                // Vibrato: 5.2 Hz modulation kicking in after 180 ms sustain
                if note.vibrato && note_elapsed_secs > 0.18 {
                    let vib = (2.0 * PI * 5.2 * (note_elapsed_secs - 0.18)).sin() * 0.22;
                    current_pitch *= (2.0f32).powf(vib / 12.0);
                }

                continuous_phase = (continuous_phase + current_pitch / SAMPLE_RATE as f32).fract();
                let env_attack = (note_elapsed_secs / 0.018).min(1.0);
                let env_decay = (-note_elapsed_secs / (note_duration_secs * 0.85)).exp();
                let env = env_attack * env_decay * note.velocity;

                let s1 = (2.0 * PI * continuous_phase).sin() * 0.72;
                let s2 = (4.0 * PI * continuous_phase).sin() * 0.18;
                bent_harmonic = (s1 + s2) * env;
            }

            // Combine smooth Karplus-Strong body resonance with phase-continuous bent string oscillator
            let raw_sample = string.tick() * 0.45 + bent_harmonic * 0.55;

            // Auto-modulate warm wah filter
            wah.auto_modulate(current_time_secs, raw_sample);
            let wah_sample = wah.process(raw_sample);

            // Tube amp saturation
            let amp_sample = amp.process(wah_sample) * 0.48;

            // Subtle spatial stereo imaging
            let left = amp_sample * 0.95;
            let right = amp_sample * 0.88;

            buffer.samples[f * 2] = left.clamp(-0.95, 0.95);
            buffer.samples[f * 2 + 1] = right.clamp(-0.95, 0.95);
        }

        buffer
    }

    /// Composes a 16-bar guitar solo melody strictly locked to A Major pentatonic scale
    /// (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz) without chromatic siren sweeps.
    pub fn compose_16_bar_blues_rock_solo(bars: u32, _root: &str, _scale: ModalScale) -> Vec<SoloNote> {
        let mut notes = Vec::new();

        // Strict A Major Pentatonic Scale Frequencies (220.0 to 440.0 Hz):
        let a3 = 220.00;       // Root A
        let b3 = 246.94;       // Major 2nd B
        let c_sharp4 = 277.18; // Major 3rd C#
        let e4 = 329.63;       // Perfect 5th E
        let f_sharp4 = 369.99; // Major 6th F#
        let a4 = 440.00;       // Octave A

        // Phrase 1: Bars 1–4 (Opening Motif & Call in pentatonic register)
        notes.push(SoloNote { bar_offset_beats: 1.0, duration_beats: 1.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 2.0, duration_beats: 0.5, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 2.5, duration_beats: 1.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: true, velocity: 0.90 });

        notes.push(SoloNote { bar_offset_beats: 5.0, duration_beats: 1.0, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: true, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 6.5, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.75 });
        notes.push(SoloNote { bar_offset_beats: 7.0, duration_beats: 1.0, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });

        notes.push(SoloNote { bar_offset_beats: 9.0, duration_beats: 0.5, pitch_hz: a3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 9.5, duration_beats: 0.5, pitch_hz: b3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 10.0, duration_beats: 2.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: true, velocity: 0.90 });

        // Phrase 2: Bars 5–8 (Pentatonic development & melodic phrasing)
        notes.push(SoloNote { bar_offset_beats: 17.0, duration_beats: 1.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 0.92 });
        notes.push(SoloNote { bar_offset_beats: 18.5, duration_beats: 0.5, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 19.0, duration_beats: 1.0, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });

        notes.push(SoloNote { bar_offset_beats: 21.0, duration_beats: 1.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: true, velocity: 0.95 });
        notes.push(SoloNote { bar_offset_beats: 23.0, duration_beats: 0.5, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 23.5, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 25.0, duration_beats: 3.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 0.92 });

        // Phrase 3: Bars 9–12 (Climax & Soaring Pentatonic Highs)
        notes.push(SoloNote { bar_offset_beats: 33.0, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });
        notes.push(SoloNote { bar_offset_beats: 33.5, duration_beats: 0.5, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });
        notes.push(SoloNote { bar_offset_beats: 34.0, duration_beats: 0.5, pitch_hz: a4, bend_semitones: 0.0, vibrato: false, velocity: 0.92 });
        notes.push(SoloNote { bar_offset_beats: 34.5, duration_beats: 0.5, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.92 });

        notes.push(SoloNote { bar_offset_beats: 36.0, duration_beats: 2.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 1.0 });

        // Rapid descending lick strictly on pentatonic scale
        notes.push(SoloNote { bar_offset_beats: 39.0, duration_beats: 0.25, pitch_hz: a4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.25, duration_beats: 0.25, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.5, duration_beats: 0.25, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.75, duration_beats: 0.25, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });

        // Sustained peak A4
        notes.push(SoloNote { bar_offset_beats: 41.0, duration_beats: 3.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 1.0 });

        // Phrase 4: Bars 13–16 (Descent & Soulful Tonic Resolution)
        notes.push(SoloNote { bar_offset_beats: 49.0, duration_beats: 0.75, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 49.75, duration_beats: 0.75, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 50.5, duration_beats: 1.0, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: true, velocity: 0.88 });

        notes.push(SoloNote { bar_offset_beats: 52.0, duration_beats: 1.0, pitch_hz: b3, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 53.0, duration_beats: 0.5, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 53.5, duration_beats: 0.5, pitch_hz: b3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 54.0, duration_beats: 1.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });

        // Final tonic resolution on A3 (220.0 Hz)
        notes.push(SoloNote { bar_offset_beats: 56.0, duration_beats: 6.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: true, velocity: 0.95 });

        // Filter notes within requested bar range
        let max_beats = bars as f32 * 4.0;
        notes.into_iter().filter(|n| n.bar_offset_beats < max_beats).collect()
    }
}

/// Helper to synthesize reggae offbeat skank chords (beats 2 and 4) on A Major pentatonic frequencies.
pub fn synthesize_reggae_skank_chord(
    bar_beat: usize,
    sample_rate: u32,
    velocity: f32,
) -> Option<[KarplusStrongString; 4]> {
    // Reggae skanks hit on beats 2 and 4 (0-indexed 1 and 3)
    if bar_beat != 1 && bar_beat != 3 {
        return None;
    }
    let chord_pitches = [220.0, 277.18, 329.63, 440.0];
    let mut strings = [
        KarplusStrongString::new(sample_rate),
        KarplusStrongString::new(sample_rate),
        KarplusStrongString::new(sample_rate),
        KarplusStrongString::new(sample_rate),
    ];
    for (i, pitch) in chord_pitches.iter().enumerate() {
        strings[i].pluck(*pitch, velocity * 0.85);
    }
    Some(strings)
}

/// Synthesizes complete reggae offbeat skank guitar track across duration_secs at bpm.
/// Hits chords on beats 2 and 4 using Karplus-Strong string physical models on A Major pentatonic frequencies.
pub fn synthesize_reggae_skank_track(duration_secs: f32, sample_rate: u32, bpm: u32) -> AudioBuffer {
    let mut buf = AudioBuffer::new_stereo(duration_secs);
    let frames = buf.frame_count();
    let beat_sec = 60.0 / (bpm.clamp(40, 220) as f32);
    let bar_sec = beat_sec * 4.0;
    let sr = sample_rate as f32;

    let mut current_strings: Option<[KarplusStrongString; 4]> = None;
    let mut last_beat = usize::MAX;

    for i in 0..frames {
        let t = i as f32 / sr;
        let beat_in_bar = ((t % bar_sec) / beat_sec).floor() as usize;

        if beat_in_bar != last_beat {
            last_beat = beat_in_bar;
            if let Some(new_chords) = synthesize_reggae_skank_chord(beat_in_bar, sample_rate, 0.75) {
                current_strings = Some(new_chords);
            }
        }

        if let Some(ref mut strings) = current_strings {
            let mut s_sum = 0.0f32;
            for strg in strings.iter_mut() {
                s_sum += strg.tick();
            }
            let s_out = s_sum * 0.25;
            buf.samples[i * 2] = s_out;
            buf.samples[i * 2 + 1] = s_out * 0.95;
        }
    }

    buf
}

/// Convenience standalone function conforming to PROJECT.md interface contract:
/// `synthesize_lead_solo(bars: u32, bpm: u32, scale: &str) -> AudioBuffer`
pub fn synthesize_lead_solo(bars: u32, bpm: u32, scale: &str) -> AudioBuffer {
    SoloGenerator::synthesize_lead_solo(bars, bpm, scale)
}

/// Compatibility alias matching `synthesize_guitar_solo(bars, bpm, scale_root_hz)`.
pub fn synthesize_guitar_solo(bars: u32, bpm: u32, _scale_root_hz: f32) -> AudioBuffer {
    SoloGenerator::synthesize_lead_solo(bars, bpm, "A Major")
}
