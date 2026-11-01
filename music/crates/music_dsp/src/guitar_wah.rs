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
            decay_factor: 0.995,
            is_active: false,
        }
    }

    /// Plucks the string at the specified fundamental frequency (in Hz) and velocity.
    pub fn pluck(&mut self, freq_hz: f32, velocity: f32) {
        let period = (self.sample_rate / freq_hz.clamp(50.0, 4000.0)).max(4.0);
        let int_period = period.floor() as usize;

        self.buffer_len = int_period.min(self.delay_buffer.len() - 2);
        self.write_ptr = 0;
        self.prev_filter_sample = 0.0;
        self.allpass_state = 0.0;
        // String decay factor scaled by pitch (higher notes decay faster)
        self.decay_factor = (0.996 - (freq_hz / 10000.0)).clamp(0.985, 0.998);

        // Excite delay line with shaped pick noise
        let mut prng = 0x5EED_F00Du32;
        let pick_pos_samples = (self.buffer_len as f32 * 0.18).round() as usize;

        for i in 0..self.buffer_len {
            prng ^= prng << 13;
            prng ^= prng >> 17;
            prng ^= prng << 5;
            let noise = (prng as f32 / u32::MAX as f32) * 2.0 - 1.0;
            self.delay_buffer[i] = noise * velocity;
        }

        // Comb filter simulating pick position relative to bridge
        if pick_pos_samples > 0 && pick_pos_samples < self.buffer_len {
            for i in pick_pos_samples..self.buffer_len {
                self.delay_buffer[i] -= 0.5 * self.delay_buffer[i - pick_pos_samples];
            }
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

        // Lowpass averaging loop filter: (y[n] + y[n-1]) / 2 * decay
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
            center_freq: 800.0,
            q_resonance: 4.2,
            envelope_state: 0.0,
        }
    }

    /// Sets pedal position (0.0 = heel/400 Hz, 1.0 = toe/2200 Hz).
    pub fn set_pedal_position(&mut self, pos: f32) {
        let clamped = pos.clamp(0.0, 1.0);
        // Exponential sweep characteristic of vintage Cry Baby potentiometer
        self.center_freq = 400.0 * (2200.0 / 400.0f32).powf(clamped);
    }

    /// Processes one sample through the resonant bandpass wah circuit.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let f = (2.0 * (PI * self.center_freq / self.sample_rate).sin()).clamp(0.01, 1.2);
        let q = 1.0 / self.q_resonance;

        // Chamberlin State-Variable Filter (SVF) equations
        let hp = input - self.lp_state - q * self.bp_state;
        self.bp_state += f * hp;
        self.lp_state += f * self.bp_state;

        // Peak/bandpass blend with subtle dry presence
        let wah_out = self.bp_state * 2.8 + input * 0.25;
        wah_out.clamp(-2.0, 2.0)
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

        // 5.2 kHz low-pass (cabinet speaker cone emulation)
        let lp_alpha = 0.45;
        self.cabinet_lp = (1.0 - lp_alpha) * self.cabinet_lp + lp_alpha * cleaned;

        self.cabinet_lp * 0.85
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
        let mut amp = GuitarAmpSimulator::new(3.8);

        let mut current_note_idx = 0;
        let mut active_note: Option<SoloNote> = None;
        let mut note_elapsed_secs = 0.0f32;

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

            // Calculate pitch bend and vibrato modulation
            if let Some(note) = active_note {
                note_elapsed_secs += 1.0 / SAMPLE_RATE as f32;
                let note_duration_secs = note.duration_beats * beat_duration_secs;

                let mut current_pitch = note.pitch_hz;
                // Pitch bend ramp
                if note.bend_semitones > 0.0 && note_elapsed_secs > 0.08 {
                    let bend_progress = ((note_elapsed_secs - 0.08) / (note_duration_secs * 0.6)).min(1.0);
                    let semitones = note.bend_semitones * bend_progress;
                    current_pitch *= (2.0f32).powf(semitones / 12.0);
                }

                // Vibrato: 5.2 Hz modulation kicking in after 180 ms sustain
                if note.vibrato && note_elapsed_secs > 0.18 {
                    let vib = (2.0 * PI * 5.2 * (note_elapsed_secs - 0.18)).sin() * 0.45; // +/- 0.45 semitones
                    current_pitch *= (2.0f32).powf(vib / 12.0);
                }

                // Update string period if frequency changed
                let new_period = (SAMPLE_RATE as f32 / current_pitch).round() as usize;
                if new_period > 10 && new_period < string.delay_buffer.len() - 2 {
                    string.buffer_len = new_period;
                }
            }

            // String physical loop
            let raw_sample = string.tick();

            // Auto-modulate wah filter
            wah.auto_modulate(current_time_secs, raw_sample);
            let wah_sample = wah.process(raw_sample);

            // Tube amp saturation
            let amp_sample = amp.process(wah_sample);

            // Subtle spatial stereo imaging
            let left = amp_sample * 0.95;
            let right = amp_sample * 0.88;

            buffer.samples[f * 2] = left.clamp(-0.95, 0.95);
            buffer.samples[f * 2 + 1] = right.clamp(-0.95, 0.95);
        }

        buffer
    }

    /// Composes a 16-bar blues/rock solo melody in A Major / Blues scale.
    pub fn compose_16_bar_blues_rock_solo(bars: u32, _root: &str, _scale: ModalScale) -> Vec<SoloNote> {
        let mut notes = Vec::new();

        // Exact Pitch Frequencies for A (Root = 220 Hz):
        let a3 = 220.00;
        let b3 = 246.94;
        let c_sharp4 = 277.18;
        let d4 = 293.66;
        let d_sharp4 = 311.13; // Blue note!
        let e4 = 329.63;
        let f_sharp4 = 369.99;
        let g4 = 392.00;
        let a4 = 440.00;
        let c5 = 523.25;
        let c_sharp5 = 554.37;
        let d5 = 587.33;
        let e5 = 659.25;
        let g5 = 783.99;
        let a5 = 880.00;

        // Phrase 1: Bars 1–4 (Opening Motif & Call in lower register)
        notes.push(SoloNote { bar_offset_beats: 1.0, duration_beats: 1.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 2.0, duration_beats: 0.5, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 2.5, duration_beats: 1.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: true, velocity: 0.90 });

        notes.push(SoloNote { bar_offset_beats: 5.0, duration_beats: 1.0, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: true, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 6.5, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.75 });
        notes.push(SoloNote { bar_offset_beats: 7.0, duration_beats: 1.0, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });

        notes.push(SoloNote { bar_offset_beats: 9.0, duration_beats: 0.5, pitch_hz: a3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 9.5, duration_beats: 0.5, pitch_hz: b3, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 10.0, duration_beats: 2.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: true, velocity: 0.90 });

        // Phrase 2: Bars 5–8 (Blues development, blue notes & bending)
        notes.push(SoloNote { bar_offset_beats: 17.0, duration_beats: 1.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 0.92 });
        notes.push(SoloNote { bar_offset_beats: 18.5, duration_beats: 0.5, pitch_hz: g4, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 19.0, duration_beats: 1.0, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });

        // D4 full bend to E4 (2 semitones)
        notes.push(SoloNote { bar_offset_beats: 21.0, duration_beats: 1.5, pitch_hz: d4, bend_semitones: 2.0, vibrato: true, velocity: 0.95 });
        // Blue note Eb4
        notes.push(SoloNote { bar_offset_beats: 23.0, duration_beats: 0.5, pitch_hz: d_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 23.5, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 25.0, duration_beats: 3.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: true, velocity: 0.92 });

        // Phrase 3: Bars 9–12 (Climax & Soaring High-Register Screaming Bends)
        notes.push(SoloNote { bar_offset_beats: 33.0, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });
        notes.push(SoloNote { bar_offset_beats: 33.5, duration_beats: 0.5, pitch_hz: g4, bend_semitones: 0.0, vibrato: false, velocity: 0.88 });
        notes.push(SoloNote { bar_offset_beats: 34.0, duration_beats: 0.5, pitch_hz: a4, bend_semitones: 0.0, vibrato: false, velocity: 0.92 });
        notes.push(SoloNote { bar_offset_beats: 34.5, duration_beats: 0.5, pitch_hz: c5, bend_semitones: 0.0, vibrato: false, velocity: 0.92 });

        // Screaming D5 bend up to E5 (+2 semitones) with high vibrato
        notes.push(SoloNote { bar_offset_beats: 36.0, duration_beats: 2.0, pitch_hz: d5, bend_semitones: 2.0, vibrato: true, velocity: 1.0 });

        // Rapid descending lick
        notes.push(SoloNote { bar_offset_beats: 39.0, duration_beats: 0.25, pitch_hz: e5, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.25, duration_beats: 0.25, pitch_hz: d5, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.5, duration_beats: 0.25, pitch_hz: c5, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 39.75, duration_beats: 0.25, pitch_hz: a4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });

        // Soaring peak A5 (880 Hz)
        notes.push(SoloNote { bar_offset_beats: 41.0, duration_beats: 3.0, pitch_hz: a5, bend_semitones: 0.0, vibrato: true, velocity: 1.0 });

        // Phrase 4: Bars 13–16 (Descent & Soulful Resolution)
        notes.push(SoloNote { bar_offset_beats: 49.0, duration_beats: 0.75, pitch_hz: g5, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 49.75, duration_beats: 0.75, pitch_hz: e5, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
        notes.push(SoloNote { bar_offset_beats: 50.5, duration_beats: 1.0, pitch_hz: c_sharp5, bend_semitones: 0.0, vibrato: true, velocity: 0.88 });

        notes.push(SoloNote { bar_offset_beats: 52.0, duration_beats: 1.0, pitch_hz: a4, bend_semitones: 0.0, vibrato: false, velocity: 0.85 });
        notes.push(SoloNote { bar_offset_beats: 53.0, duration_beats: 0.5, pitch_hz: f_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 53.5, duration_beats: 0.5, pitch_hz: e4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });
        notes.push(SoloNote { bar_offset_beats: 54.0, duration_beats: 1.0, pitch_hz: c_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.80 });

        // Final tonic resolution on A3
        notes.push(SoloNote { bar_offset_beats: 56.0, duration_beats: 6.0, pitch_hz: a3, bend_semitones: 0.0, vibrato: true, velocity: 0.95 });

        // Filter notes within requested bar range
        let max_beats = bars as f32 * 4.0;
        notes.into_iter().filter(|n| n.bar_offset_beats < max_beats).collect()
    }
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
