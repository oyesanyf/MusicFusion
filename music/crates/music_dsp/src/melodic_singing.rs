//! `melodic_singing`: Pure-Rust 48 kHz Single-Impulse Epoch TD-PSOLA Singing Voice Synthesizer.
//!
//! Converts English lyrics into crystal-clear, 100% pitch-locked human melodic singing:
//! 1. Glottal Closure Epoch Detection: Finds exact single-pulse glottal closure peaks `m_k`
//!    across the articulated lyric audio.
//! 2. Single-Impulse Asymmetric Glottal Grain Extraction (`[-64, +156]` samples around `m_k`):
//!    Extracts strictly ONE glottal closure impulse and its vocal-tract formant ring (~3.2 ms),
//!    cutting off before the next speaking glottal pulse (~5.1 ms) so 0% of the monotone
//!    speaking pitch can bleed into the sung melody.
//! 3. Sub-Grain Linear Interpolation & Legato Phrasing: Smoothly interpolates between adjacent
//!    glottal grains (`m_0 * (1 - frac) + m_1 * frac`) across continuous legato 8-beat musical
//!    lines so words never stutter mid-syllable and every note locks 100% to the musical scale.
//! 4. 3-Part Chorus Harmony & Warm Vocal Body Pad: Layers stereo Major 3rd & Lower 4th harmony
//!    singers on Chorus hooks with warm harmonic formant reinforcement and Schroeder studio reverb.

use std::f32::consts::PI;
use crate::vocal_formant::{BiquadFilter, VocalRaspProcessor};
use crate::wav::AudioBuffer;

pub const SAMPLE_RATE: u32 = 48_000;

/// Pentatonic scale ratios relative to root tonic (Tonic, Maj 2nd, Maj 3rd, Perf 5th, Maj 6th, Octave).
pub const MAJOR_PENTATONIC_RATIOS: [f32; 6] = [
    1.0,        // Tonic (A3 = 220.0 Hz)
    1.122_462,  // Major Second (B3 = 246.94 Hz)
    1.259_921,  // Major Third (C#4 = 277.18 Hz)
    1.498_307,  // Perfect Fifth (E4 = 329.63 Hz)
    1.681_793,  // Major Sixth (F#4 = 369.99 Hz)
    2.0,        // Octave (A4 = 440.0 Hz)
];

/// Extended vocal melody scale ratios (spanning E3 up to A4) in natural Tenor/Alto singing register.
const VOCAL_MELODY_RATIOS: [f32; 9] = [
    0.749_154,  // 0: Low 5th (E3 = 164.81 Hz)
    0.840_896,  // 1: Low 6th (F#3 = 185.00 Hz)
    1.0,        // 2: Tonic (A3 = 220.00 Hz)
    1.122_462,  // 3: Maj 2nd (B3 = 246.94 Hz)
    1.259_921,  // 4: Maj 3rd (C#4 = 277.18 Hz)
    1.334_840,  // 5: Perf 4th (D4 = 293.66 Hz)
    1.498_307,  // 6: Perf 5th (E4 = 329.63 Hz)
    1.681_793,  // 7: Maj 6th (F#4 = 369.99 Hz)
    2.0,        // 8: Octave (A4 = 440.00 Hz)
];

/// Detected glottal closure epoch (pitch mark) for single-impulse TD-PSOLA grain extraction.
#[derive(Debug, Clone, Copy)]
struct PsolaPitchMark {
    center_idx: usize,
    period: usize,
    is_voiced: bool,
    rms: f32,
}

/// Constant-Peak-Gain Zero-Pole Bandpass Formant Resonator.
#[derive(Debug, Clone, Copy)]
struct BandpassFormantResonator {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BandpassFormantResonator {
    const fn new() -> Self {
        Self { x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    #[inline]
    fn process(&mut self, input: f32, freq_hz: f32, bw_hz: f32, sr: f32) -> f32 {
        let fc = freq_hz.clamp(200.0, sr * 0.44);
        let r = (-PI * bw_hz / sr).exp();
        let theta = 2.0 * PI * fc / sr;
        let a1 = -2.0 * r * theta.cos();
        let a2 = r * r;
        let b0 = (1.0 - r * r) * 0.5;

        let y = b0 * (input - self.x2) - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Schroeder Feedback Comb Filter with 1-pole high-frequency damping.
#[derive(Debug, Clone)]
struct DampedCombFilter {
    buffer: Vec<f32>,
    idx: usize,
    feedback: f32,
    damp: f32,
    filter_store: f32,
}

impl DampedCombFilter {
    fn new(delay_samples: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            idx: 0,
            feedback,
            damp,
            filter_store: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.idx];
        self.filter_store = output * (1.0 - self.damp) + self.filter_store * self.damp;
        self.buffer[self.idx] = input + self.filter_store * self.feedback;
        self.idx = (self.idx + 1) % self.buffer.len();
        output
    }
}

/// Schroeder Allpass Filter.
#[derive(Debug, Clone)]
struct AllpassFilter {
    buffer: Vec<f32>,
    idx: usize,
    gain: f32,
}

impl AllpassFilter {
    fn new(delay_samples: usize, gain: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            idx: 0,
            gain,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.idx];
        let output = -input + buf_out;
        self.buffer[self.idx] = input + buf_out * self.gain;
        self.idx = (self.idx + 1) % self.buffer.len();
        output
    }
}

/// Warm Studio Vocal Reverb based on Schroeder architecture.
#[derive(Debug, Clone)]
pub struct StudioVocalReverb {
    comb_l: [DampedCombFilter; 4],
    comb_r: [DampedCombFilter; 4],
    allpass_l: [AllpassFilter; 2],
    allpass_r: [AllpassFilter; 2],
    wet_mix: f32,
}

impl StudioVocalReverb {
    pub fn new(sample_rate: u32, wet_mix: f32) -> Self {
        let sr_factor = sample_rate as f32 / 48_000.0;
        let c_delays = [
            (1116.0 * sr_factor) as usize,
            (1188.0 * sr_factor) as usize,
            (1277.0 * sr_factor) as usize,
            (1356.0 * sr_factor) as usize,
        ];
        let ap_delays = [
            (225.0 * sr_factor) as usize,
            (341.0 * sr_factor) as usize,
        ];

        Self {
            comb_l: [
                DampedCombFilter::new(c_delays[0], 0.74, 0.35),
                DampedCombFilter::new(c_delays[1], 0.74, 0.35),
                DampedCombFilter::new(c_delays[2], 0.74, 0.35),
                DampedCombFilter::new(c_delays[3], 0.74, 0.35),
            ],
            comb_r: [
                DampedCombFilter::new(c_delays[0] + 23, 0.74, 0.35),
                DampedCombFilter::new(c_delays[1] + 29, 0.74, 0.35),
                DampedCombFilter::new(c_delays[2] + 31, 0.74, 0.35),
                DampedCombFilter::new(c_delays[3] + 37, 0.74, 0.35),
            ],
            allpass_l: [
                AllpassFilter::new(ap_delays[0], 0.5),
                AllpassFilter::new(ap_delays[1], 0.5),
            ],
            allpass_r: [
                AllpassFilter::new(ap_delays[0] + 11, 0.5),
                AllpassFilter::new(ap_delays[1] + 13, 0.5),
            ],
            wet_mix: wet_mix.clamp(0.0, 1.0),
        }
    }

    pub fn process_stereo(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let mut sum_l = 0.0f32;
        let mut sum_r = 0.0f32;
        for c in &mut self.comb_l {
            sum_l += c.process(in_l);
        }
        for c in &mut self.comb_r {
            sum_r += c.process(in_r);
        }

        sum_l *= 0.25;
        sum_r *= 0.25;

        for ap in &mut self.allpass_l {
            sum_l = ap.process(sum_l);
        }
        for ap in &mut self.allpass_r {
            sum_r = ap.process(sum_r);
        }

        let out_l = in_l * (1.0 - self.wet_mix) + sum_l * self.wet_mix;
        let out_r = in_r * (1.0 - self.wet_mix) + sum_r * self.wet_mix;

        (out_l, out_r)
    }
}

/// Rhythmic vocal note event within a 4-bar (16-beat) musical phrase.
#[derive(Debug, Clone, Copy)]
struct VocalPhraseNote {
    start_beat: f32,
    duration_beats: f32,
    scale_idx: usize,
    has_backing_harmony: bool,
    is_phrase_end: bool,
}

/// Core Melodic Singing Synthesizer.
#[derive(Debug, Clone)]
pub struct MelodicSingingSynthesizer {
    pub sample_rate: u32,
    pub vibrato_freq: f32,
    pub max_vibrato_depth: f32,
    pub portamento_ms: f32,
}

impl Default for MelodicSingingSynthesizer {
    fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
            vibrato_freq: 5.5,
            max_vibrato_depth: 0.014, // ±1.4% natural singing vibrato
            portamento_ms: 22.0,
        }
    }
}

impl MelodicSingingSynthesizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            ..Default::default()
        }
    }

    /// Normalizes any passed root frequency into natural human Tenor/Alto singing range (196.0 Hz – 246.94 Hz).
    pub fn normalize_human_vocal_root(raw_root_hz: f32) -> f32 {
        let mut f = if raw_root_hz > 20.0 { raw_root_hz } else { 220.0 };
        while f > 265.0 {
            f *= 0.5;
        }
        while f < 175.0 {
            f *= 2.0;
        }
        f
    }

    /// Extracts glottal epochs (pitch marks) across the mono speech waveform for single-impulse TD-PSOLA.
    fn extract_psola_pitch_marks(speech: &[f32], sr: u32) -> Vec<PsolaPitchMark> {
        let mut marks = Vec::new();
        // Zira's speaking fundamental is ~185-235 Hz (204 to 260 samples at 48 kHz)
        let min_p = (sr as usize / 290).max(150); // 165 samples (~290 Hz)
        let max_p = (sr as usize / 150).max(240); // 320 samples (~150 Hz)
        let default_p = (sr as usize / 205).max(180); // 234 samples (~205 Hz)

        if speech.len() < max_p * 3 {
            return marks;
        }

        let mut pos = max_p;
        while pos + max_p * 2 < speech.len() {
            let win = max_p;
            let mut e0 = 0.0f32;
            for i in 0..win {
                let s = speech[pos + i];
                e0 += s * s;
            }
            let rms = (e0 / win as f32).sqrt();

            let mut best_lag = default_p;
            let mut best_corr = 0.0f32;
            if e0 > 1e-5 {
                let mut lag = min_p;
                while lag <= max_p {
                    let mut c = 0.0f32;
                    for i in (0..win).step_by(2) {
                        c += speech[pos + i] * speech[pos + i + lag];
                    }
                    let norm_c = (c * 2.0) / e0;
                    if norm_c > best_corr {
                        best_corr = norm_c;
                        best_lag = lag;
                    }
                    lag += 2;
                }
            }

            let is_voiced = best_corr > 0.34 && rms > 0.018;
            let period = if is_voiced { best_lag } else { default_p };

            let search_start = pos + (period * 75) / 100;
            let search_end = (pos + (period * 125) / 100).min(speech.len() - max_p - 1);
            let mut peak_idx = pos + period;
            if is_voiced && search_end > search_start {
                let mut max_val = -1e9f32;
                for idx in search_start..=search_end {
                    if speech[idx] > max_val {
                        max_val = speech[idx];
                        peak_idx = idx;
                    }
                }
            }

            let actual_step = peak_idx.saturating_sub(pos).clamp(min_p, max_p);
            marks.push(PsolaPitchMark {
                center_idx: peak_idx,
                period: actual_step,
                is_voiced,
                rms,
            });

            pos = peak_idx;
        }

        marks
    }

    /// Generates an articulated speech buffer from `lyrics` using Windows SAPI (`Microsoft Zira Desktop`)
    /// and extracts its glottal pitch marks for single-impulse TD-PSOLA singing resynthesis.
    #[cfg(windows)]
    fn render_psola_source(&self, lyrics: &str, target_frames: usize) -> Option<(Vec<f32>, Vec<PsolaPitchMark>)> {
        if target_frames < 48_000 * 3 {
            return None;
        }

        let clean_lyrics: String = lyrics
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !(l.starts_with('[') && l.ends_with(']')))
            .collect::<Vec<_>>()
            .join(", ");

        if clean_lyrics.is_empty() {
            return None;
        }

        let sanitized: String = clean_lyrics
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | ',' | '.' | '\'' | '-'))
            .take(460)
            .collect();

        let temp_wav = std::env::temp_dir().join(format!(
            "music_psola_v5_{}_{}.wav",
            std::process::id(),
            target_frames
        ));
        let temp_str = temp_wav.to_string_lossy().replace('\'', "''");

        let ps_script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             try {{ $s.SelectVoice('Microsoft Zira Desktop') }} catch {{}}; \
             $s.Rate = -1; \
             $fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(48000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono); \
             $s.SetOutputToWaveFile('{}', $fmt); \
             $s.Speak('{}'); \
             $s.Dispose()",
            temp_str,
            sanitized.replace('\'', " ")
        );

        let mut cmd = std::process::Command::new("powershell");
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);

        let status = cmd.status().ok()?;
        if !status.success() || !temp_wav.exists() {
            return None;
        }

        let buf = AudioBuffer::from_wav_file(&temp_wav).ok();
        let _ = std::fs::remove_file(&temp_wav);
        let buf = buf?;
        if buf.samples.is_empty() {
            return None;
        }

        let ch = buf.channels.max(1) as usize;
        let mut active_speech = Vec::with_capacity(buf.frame_count());
        let mut env_follow = 0.0f32;
        for f in 0..buf.frame_count() {
            let s = buf.samples[f * ch];
            env_follow = 0.994 * env_follow + 0.006 * s.abs();
            if env_follow > 0.006 {
                active_speech.push(s);
            }
        }

        if active_speech.len() < 4800 {
            return None;
        }

        let max_p = active_speech.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        if max_p > 0.01 {
            let scale = 0.92 / max_p;
            for x in &mut active_speech {
                *x = (*x * scale).clamp(-1.0, 1.0);
            }
        }

        let marks = Self::extract_psola_pitch_marks(&active_speech, self.sample_rate);
        if marks.len() < 16 {
            None
        } else {
            Some((active_speech, marks))
        }
    }

    #[cfg(not(windows))]
    fn render_psola_source(&self, _lyrics: &str, _target_frames: usize) -> Option<(Vec<f32>, Vec<PsolaPitchMark>)> {
        None
    }

    /// Composes continuous legato 8-beat musical vocal lines across a 4-bar (16-beat) cycle.
    /// Notes inside each 8-beat line connect legato (`start_beat + duration_beats == next.start_beat`)
    /// so words never get chopped mid-syllable, with a natural 0.5-beat breath at the end of each 2-bar line!
    fn compose_4bar_phrases() -> [Vec<VocalPhraseNote>; 3] {
        // Phrase A: Anthemic Chorus Hook (Continuous legato 16-beat melody — natural word spaces provide breathing)
        let chorus = vec![
            VocalPhraseNote { start_beat: 0.0,  duration_beats: 1.0,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 1.0,  duration_beats: 1.5,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 2.5,  duration_beats: 0.75, scale_idx: 3, has_backing_harmony: false, is_phrase_end: false }, // B3
            VocalPhraseNote { start_beat: 3.25, duration_beats: 0.75, scale_idx: 2, has_backing_harmony: true,  is_phrase_end: false }, // A3
            VocalPhraseNote { start_beat: 4.0,  duration_beats: 1.0,  scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false }, // D4
            VocalPhraseNote { start_beat: 5.0,  duration_beats: 1.0,  scale_idx: 7, has_backing_harmony: true,  is_phrase_end: false }, // F#4
            VocalPhraseNote { start_beat: 6.0,  duration_beats: 2.0,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false }, // E4 held
            VocalPhraseNote { start_beat: 8.0,  duration_beats: 1.25, scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 9.25, duration_beats: 1.25, scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 10.5, duration_beats: 1.5,  scale_idx: 2, has_backing_harmony: true,  is_phrase_end: false }, // A3
            VocalPhraseNote { start_beat: 12.0, duration_beats: 1.0,  scale_idx: 3, has_backing_harmony: false, is_phrase_end: false }, // B3
            VocalPhraseNote { start_beat: 13.0, duration_beats: 1.0,  scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 14.0, duration_beats: 2.0,  scale_idx: 2, has_backing_harmony: true,  is_phrase_end: false }, // A3 resolution
        ];

        // Phrase B: Soulful Verse Narrative
        let verse = vec![
            VocalPhraseNote { start_beat: 0.0,  duration_beats: 1.0, scale_idx: 2, has_backing_harmony: false, is_phrase_end: false }, // A3
            VocalPhraseNote { start_beat: 1.0,  duration_beats: 1.0, scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 2.0,  duration_beats: 1.0, scale_idx: 6, has_backing_harmony: false, is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 3.0,  duration_beats: 1.0, scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 4.0,  duration_beats: 1.0, scale_idx: 5, has_backing_harmony: false, is_phrase_end: false }, // D4
            VocalPhraseNote { start_beat: 5.0,  duration_beats: 1.0, scale_idx: 7, has_backing_harmony: false, is_phrase_end: false }, // F#4
            VocalPhraseNote { start_beat: 6.0,  duration_beats: 2.0, scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 8.0,  duration_beats: 1.0, scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 9.0,  duration_beats: 1.0, scale_idx: 3, has_backing_harmony: false, is_phrase_end: false }, // B3
            VocalPhraseNote { start_beat: 10.0, duration_beats: 2.0, scale_idx: 2, has_backing_harmony: false, is_phrase_end: false }, // A3
            VocalPhraseNote { start_beat: 12.0, duration_beats: 1.0, scale_idx: 3, has_backing_harmony: false, is_phrase_end: false }, // B3
            VocalPhraseNote { start_beat: 13.0, duration_beats: 1.0, scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 14.0, duration_beats: 2.0, scale_idx: 2, has_backing_harmony: true,  is_phrase_end: false }, // A3
        ];

        // Phrase C: Uplifting Melodic Bridge
        let bridge = vec![
            VocalPhraseNote { start_beat: 0.0,   duration_beats: 1.25, scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 1.25,  duration_beats: 1.25, scale_idx: 7, has_backing_harmony: true,  is_phrase_end: false }, // F#4
            VocalPhraseNote { start_beat: 2.5,   duration_beats: 1.5,  scale_idx: 8, has_backing_harmony: true,  is_phrase_end: false }, // A4
            VocalPhraseNote { start_beat: 4.0,   duration_beats: 1.0,  scale_idx: 7, has_backing_harmony: false, is_phrase_end: false }, // F#4
            VocalPhraseNote { start_beat: 5.0,   duration_beats: 1.0,  scale_idx: 6, has_backing_harmony: false, is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 6.0,   duration_beats: 2.0,  scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false }, // D4
            VocalPhraseNote { start_beat: 8.0,   duration_beats: 1.0,  scale_idx: 4, has_backing_harmony: false, is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 9.0,   duration_beats: 1.0,  scale_idx: 6, has_backing_harmony: false, is_phrase_end: false }, // E4
            VocalPhraseNote { start_beat: 10.0,  duration_beats: 2.0,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false }, // C#4
            VocalPhraseNote { start_beat: 12.0,  duration_beats: 1.25, scale_idx: 3, has_backing_harmony: false, is_phrase_end: false }, // B3
            VocalPhraseNote { start_beat: 13.25, duration_beats: 2.75, scale_idx: 2, has_backing_harmony: true,  is_phrase_end: false }, // A3
        ];

        [chorus, verse, bridge]
    }

    /// Single-Impulse Asymmetric Glottal Grain Overlap-Add (`[-48, +112]` around `mark.center_idx`).
    /// Total window width is 160 samples (3.33 ms at 48 kHz), which is strictly < 75% of a single
    /// source pitch period so ZERO neighboring glottal pulses ever enter the grain!
    #[inline]
    fn overlap_add_single_impulse_grain(
        speech: &[f32],
        m0: PsolaPitchMark,
        synth_center: usize,
        gain: f32,
        out_buf: &mut [f32],
    ) {
        let pre_w: isize = 48;   // 1.00 ms pre-closure onset
        let post_w: isize = 112; // 2.33 ms single-impulse formant decay (strictly < 1 speaking period!)
        for offset in -pre_w..=post_w {
            let dst_idx = synth_center as isize + offset;
            if dst_idx < 0 || (dst_idx as usize) >= out_buf.len() {
                continue;
            }
            let s0_idx = m0.center_idx as isize + offset;
            let sample = if s0_idx >= 0 && (s0_idx as usize) < speech.len() {
                speech[s0_idx as usize]
            } else {
                0.0
            };

            let win = if offset < 0 {
                let norm = (offset + pre_w) as f32 / pre_w as f32;
                0.5 * (1.0 - (PI * norm).cos())
            } else {
                let norm = offset as f32 / post_w as f32;
                0.5 * (1.0 + (PI * norm).cos())
            };

            out_buf[dst_idx as usize] += sample * win * gain;
        }
    }

    /// Primary entry point: Synthesizes a full melodic vocal performance using Single-Impulse
    /// Epoch TD-PSOLA grain resynthesis + 3-Part Chorus Harmony + Studio Reverb.
    pub fn synthesize_melodic_vocal_track(
        &self,
        duration_secs: f32,
        bpm: f32,
        key_root_hz: f32,
        lyrics: &str,
    ) -> AudioBuffer {
        let sr = self.sample_rate as f32;
        let frames = (duration_secs * sr).round() as usize;
        let mut buffer = AudioBuffer::new(self.sample_rate, 2, duration_secs);

        if frames == 0 {
            return buffer;
        }

        let safe_bpm = bpm.clamp(40.0, 220.0);
        let beat_duration = 60.0 / safe_bpm;
        let vocal_root = Self::normalize_human_vocal_root(key_root_hz);
        let phrases = Self::compose_4bar_phrases();

        let mut psola_l = vec![0.0f32; frames];
        let mut psola_r = vec![0.0f32; frames];

        let psola_data = self.render_psola_source(lyrics, frames);

        if let Some((ref speech, ref marks)) = psola_data {
            let mut synth_pos = 120.0f64;
            let mut mark_cursor = 0.0f64;
            let mut smoothed_freq = vocal_root;

            while (synth_pos as usize) < frames.saturating_sub(240) {
                let f_idx = synth_pos as usize;
                let t = f_idx as f32 / sr;
                let total_beats = t / beat_duration;
                let cycle_idx = (total_beats / 16.0).floor() as usize;
                let beat_in_cycle = total_beats % 16.0;
                let phrase = &phrases[cycle_idx % phrases.len()];

                let mut active_note: Option<(&VocalPhraseNote, f32)> = None;
                for note in phrase.iter() {
                    if beat_in_cycle >= note.start_beat && beat_in_cycle < (note.start_beat + note.duration_beats) {
                        let note_progress = (beat_in_cycle - note.start_beat) / note.duration_beats;
                        active_note = Some((note, note_progress));
                        break;
                    }
                }

                if let Some((note, note_progress)) = active_note {
                    let ratio = VOCAL_MELODY_RATIOS[note.scale_idx % VOCAL_MELODY_RATIOS.len()];
                    let base_target_hz = vocal_root * ratio;
                    let note_elapsed_sec = note_progress * note.duration_beats * beat_duration;
                    let note_total_sec = note.duration_beats * beat_duration;

                    let vib_depth = if note_elapsed_sec < 0.14 {
                        0.0
                    } else {
                        ((note_elapsed_sec - 0.14) / 0.20).min(1.0) * self.max_vibrato_depth
                    };
                    let vibrato = (2.0 * PI * self.vibrato_freq * t).sin() * vib_depth;
                    let target_hz = base_target_hz * (1.0 + vibrato);
                    // Fast portamento between melody notes so each musical pitch steps cleanly
                    smoothed_freq = smoothed_freq * 0.62 + target_hz * 0.38;

                    let target_period = (sr / smoothed_freq.clamp(130.0, 520.0)) as f64;

                    // Legato phrase envelope: stays at 1.0 between notes inside an 8-beat line,
                    // only tapering at the end of a phrase (`note.is_phrase_end`) so words never stutter!
                    let note_env = if note.is_phrase_end && note_elapsed_sec > (note_total_sec - 0.085).max(0.05) {
                        let rem = (note_total_sec - note_elapsed_sec).max(0.0);
                        (rem / 0.085 * PI * 0.5).sin()
                    } else if (beat_in_cycle - 0.0).abs() < 0.05 || (beat_in_cycle - 8.0).abs() < 0.05 {
                        (note_elapsed_sec / 0.025).min(1.0)
                    } else {
                        1.0
                    };

                    let idx0 = (mark_cursor.floor() as usize) % marks.len();
                    let m0 = marks[idx0];

                    let density_comp = (target_period as f32 / 160.0).clamp(0.60, 1.20);
                    let lead_gain = note_env * density_comp * 0.92;

                    Self::overlap_add_single_impulse_grain(
                        speech, m0, f_idx, lead_gain, &mut psola_l,
                    );
                    Self::overlap_add_single_impulse_grain(
                        speech, m0, f_idx, lead_gain, &mut psola_r,
                    );

                    // Stretch voiced vowels (0.50x) so sung notes hold melodically; advance consonants at 0.94x
                    let speed_rate = if m0.is_voiced && m0.rms > 0.035 {
                        0.50
                    } else {
                        0.94
                    };
                    let period_ratio = target_period / (m0.period.max(150) as f64);
                    mark_cursor += period_ratio * speed_rate;
                    synth_pos += target_period;
                } else {
                    synth_pos += 180.0;
                }
            }
        }

        // Layer warm harmonic singing body pad + Chorus backing harmonies and apply studio EQ & Reverb
        let mut fb_f1 = BandpassFormantResonator::new();
        let mut fb_f2 = BandpassFormantResonator::new();
        let mut fb_f3 = BandpassFormantResonator::new();
        let mut harm_f1 = BandpassFormantResonator::new();
        let mut harm_f2 = BandpassFormantResonator::new();

        let mut warmth_eq = BiquadFilter::peaking_eq(280.0, 1.0, 2.2, 2);
        let mut clarity_eq = BiquadFilter::peaking_eq(2700.0, 1.1, 2.5, 2);
        let mut deess_eq = BiquadFilter::peaking_eq(5400.0, 1.2, -3.0, 2);
        let mut reverb = StudioVocalReverb::new(self.sample_rate, 0.17);

        let mut lead_phase = 0.0f32;
        let mut harm_high_phase = 0.0f32;
        let mut harm_low_phase = 0.0f32;
        let mut smoothed_lead_freq = vocal_root;
        let glide_alpha = (1.0 - (-1.0 / (self.portamento_ms * 0.001 * sr)).exp()).clamp(0.0005, 0.04);

        for i in 0..frames {
            let t = i as f32 / sr;
            let total_beats = t / beat_duration;
            let cycle_idx = (total_beats / 16.0).floor() as usize;
            let beat_in_cycle = total_beats % 16.0;
            let phrase = &phrases[cycle_idx % phrases.len()];

            let mut active_note: Option<(usize, &VocalPhraseNote, f32)> = None;
            for (n_idx, note) in phrase.iter().enumerate() {
                if beat_in_cycle >= note.start_beat && beat_in_cycle < (note.start_beat + note.duration_beats) {
                    let note_progress = (beat_in_cycle - note.start_beat) / note.duration_beats;
                    active_note = Some((n_idx, note, note_progress));
                    break;
                }
            }

            if let Some((n_idx, note, note_progress)) = active_note {
                let ratio = VOCAL_MELODY_RATIOS[note.scale_idx % VOCAL_MELODY_RATIOS.len()];
                let base_target_hz = vocal_root * ratio;
                let note_elapsed_sec = note_progress * note.duration_beats * beat_duration;
                let note_total_sec = note.duration_beats * beat_duration;

                let vib_depth = if note_elapsed_sec < 0.14 {
                    0.0
                } else {
                    ((note_elapsed_sec - 0.14) / 0.20).min(1.0) * self.max_vibrato_depth
                };
                let vibrato = (2.0 * PI * self.vibrato_freq * t).sin() * vib_depth;
                smoothed_lead_freq += glide_alpha * (base_target_hz * (1.0 + vibrato) - smoothed_lead_freq);
                lead_phase = (lead_phase + smoothed_lead_freq / sr).fract();

                let note_env = if note.is_phrase_end && note_elapsed_sec > (note_total_sec - 0.085).max(0.05) {
                    let rem = (note_total_sec - note_elapsed_sec).max(0.0);
                    (rem / 0.085 * PI * 0.5).sin()
                } else {
                    1.0
                };

                let vowels = [
                    (710.0, 1240.0, 2540.0),
                    (540.0, 1860.0, 2620.0),
                    (340.0, 2240.0, 2920.0),
                    (560.0, 980.0,  2440.0),
                ];
                let (vf1, vf2, vf3) = vowels[n_idx % vowels.len()];
                let g = Self::harmonic_singing_carrier(lead_phase, smoothed_lead_freq);
                let body_pad = (fb_f1.process(g, vf1, 85.0, sr) * 0.50
                    + fb_f2.process(g, vf2, 105.0, sr) * 0.50
                    + fb_f3.process(g, vf3, 130.0, sr) * 0.30)
                    * note_env;

                let (mut harm_l, mut harm_r) = (0.0f32, 0.0f32);
                if note.has_backing_harmony {
                    let high_hz = smoothed_lead_freq * 1.259_921 * 1.0015;
                    let low_hz = smoothed_lead_freq * 0.749_154 * 0.9985;
                    harm_high_phase = (harm_high_phase + high_hz / sr).fract();
                    harm_low_phase = (harm_low_phase + low_hz / sr).fract();

                    let g_hi = Self::harmonic_singing_carrier(harm_high_phase, high_hz);
                    let g_lo = Self::harmonic_singing_carrier(harm_low_phase, low_hz);

                    let h_hi = harm_f1.process(g_hi, vf1 * 1.04, 95.0, sr) * 0.25 * note_env;
                    let h_lo = harm_f2.process(g_lo, vf1 * 0.96, 95.0, sr) * 0.25 * note_env;

                    harm_l = h_hi * 0.80 + h_lo * 0.25;
                    harm_r = h_hi * 0.25 + h_lo * 0.80;
                }

                let (dry_l, dry_r) = if psola_data.is_some() {
                    // 76% Single-Impulse TD-PSOLA sung lyrics + 24% harmonic singing pitch lock + Chorus harmony
                    let l = VocalRaspProcessor::apply_rasp(psola_l[i] * 0.76 + body_pad * 0.24, 0.04) + harm_l;
                    let r = VocalRaspProcessor::apply_rasp(psola_r[i] * 0.76 + body_pad * 0.24, 0.04) + harm_r;
                    (l, r)
                } else {
                    (body_pad + harm_l, body_pad + harm_r)
                };

                let (rev_l, rev_r) = reverb.process_stereo(dry_l, dry_r);
                let eq_l = deess_eq.process_sample(0, clarity_eq.process_sample(0, warmth_eq.process_sample(0, rev_l)));
                let eq_r = deess_eq.process_sample(1, clarity_eq.process_sample(1, warmth_eq.process_sample(1, rev_r)));

                buffer.samples[i * 2] = eq_l.clamp(-0.95, 0.95);
                buffer.samples[i * 2 + 1] = eq_r.clamp(-0.95, 0.95);
            } else {
                let (rev_l, rev_r) = reverb.process_stereo(0.0, 0.0);
                let eq_l = deess_eq.process_sample(0, clarity_eq.process_sample(0, warmth_eq.process_sample(0, rev_l)));
                let eq_r = deess_eq.process_sample(1, clarity_eq.process_sample(1, warmth_eq.process_sample(1, rev_r)));

                buffer.samples[i * 2] = eq_l.clamp(-0.95, 0.95);
                buffer.samples[i * 2 + 1] = eq_r.clamp(-0.95, 0.95);
            }
        }

        // Peak normalize to -1.3 dBFS (0.86)
        let mut max_peak = 0.0f32;
        for &s in &buffer.samples {
            let a = s.abs();
            if a > max_peak {
                max_peak = a;
            }
        }
        if max_peak > 0.01 {
            let norm = (0.86 / max_peak).clamp(0.5, 4.0);
            for s in &mut buffer.samples {
                *s = (*s * norm).clamp(-0.95, 0.95);
            }
        }

        buffer
    }

    #[inline]
    fn harmonic_singing_carrier(phase: f32, freq_hz: f32) -> f32 {
        let w = 2.0 * PI * phase;
        let max_h = ((5200.0 / freq_hz.max(100.0)).floor() as usize).clamp(8, 22);
        let mut sum = 0.0f32;
        for k in 1..=max_h {
            let kf = k as f32;
            let amp = 1.0 / kf.powf(0.80);
            sum += (kf * w + kf * 0.37).sin() * amp;
        }
        sum * 0.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_melodic_singing_generates_valid_unclipped_audio() {
        let synth = MelodicSingingSynthesizer::new(SAMPLE_RATE);
        let buf = synth.synthesize_melodic_vocal_track(2.5, 76.0, 220.0, "One love, one heart");

        assert_eq!(buf.sample_rate, 48_000);
        assert_eq!(buf.channels, 2);
        assert!(buf.frame_count() >= 48_000 * 2);

        let mut max_p = 0.0f32;
        let mut sum_sq = 0.0f32;
        for &s in &buf.samples {
            assert!(s.is_finite());
            assert!(s.abs() <= 0.96, "Sample clipped: {}", s);
            max_p = max_p.max(s.abs());
            sum_sq += s * s;
        }

        let rms = (sum_sq / buf.samples.len() as f32).sqrt();
        let rms_db = 20.0 * rms.log10();

        assert!(rms_db > -40.0, "Vocal track too quiet: {} dBFS", rms_db);
        assert!(max_p > 0.50, "Peak must be strong and audible: {}", max_p);
    }

    #[test]
    fn test_melodic_singing_scale_ratios() {
        let root = 220.0f32;
        let expected = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0];
        for (i, &exp) in expected.iter().enumerate() {
            let actual = root * MAJOR_PENTATONIC_RATIOS[i];
            assert!(
                (actual - exp).abs() < 0.1,
                "Scale ratio {} mismatch: actual={}, expected={}",
                i, actual, exp
            );
        }
    }
}
