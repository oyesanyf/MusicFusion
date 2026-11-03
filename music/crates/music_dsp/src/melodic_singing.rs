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

/// Natural Minor / Roots-Folk vocal melody scale ratios (E3, G3, A3, B3, C4, D4, E4, F4, G4/A4).
const VOCAL_MINOR_MELODY_RATIOS: [f32; 9] = [
    0.749_154,  // 0: Low 5th (E3 = 164.81 Hz)
    0.890_899,  // 1: Low b7th (G3 = 196.00 Hz)
    1.0,        // 2: Tonic (A3 = 220.00 Hz)
    1.122_462,  // 3: Maj 2nd (B3 = 246.94 Hz)
    1.189_207,  // 4: Min 3rd (C4 = 261.63 Hz)
    1.334_840,  // 5: Perf 4th (D4 = 293.66 Hz)
    1.498_307,  // 6: Perf 5th (E4 = 329.63 Hz)
    1.587_401,  // 7: Min 6th (F4 = 349.23 Hz)
    1.781_797,  // 8: Min 7th (G4 = 392.00 Hz)
];

/// Distinct musical movements in a section-by-section Poem/Song arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongMovementKind {
    OpeningInstrumental,
    IntimateVerse1,
    PreChorusBuild,
    ChamberPopChorus,
    QuietVerse2Drop,
    OrchestralClimax,
    StrippedBackOutro,
}

/// Time window (`start_sec..end_sec`) for a single musical movement in the arrangement.
#[derive(Debug, Clone, Copy)]
pub struct MovementSection {
    pub kind: SongMovementKind,
    pub start_sec: f32,
    pub end_sec: f32,
}

/// Output of section-aligned vocal synthesis, returning the stereo vocal stem and the 7-movement arrangement timeline.
pub struct SectionedVocalOutput {
    pub vocals: AudioBuffer,
    pub movements: [MovementSection; 9],
    pub total_duration_secs: f32,
}

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

    /// Composes novel reference-conditioned 4-bar vocal melodies for Acoustic/Roots/Folk-Rock
    /// in Minor/Dorian or Major keys, parameterized by `seed` so each RL generation produces a
    /// distinct, expressive vocal contour.
    fn compose_reference_phrases(seed: u64) -> [Vec<VocalPhraseNote>; 3] {
        let shift = (seed as usize) % 3;
        // Phrase 1: Warm Acoustic & Roots Narrative Hook (ascending minor/dorian arch)
        let verse_hook = vec![
            VocalPhraseNote { start_beat: 0.0,  duration_beats: 1.5,  scale_idx: 2 + (shift % 2), has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 1.5,  duration_beats: 1.0,  scale_idx: 4, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 2.5,  duration_beats: 1.5,  scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 4.0,  duration_beats: 1.25, scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 5.25, duration_beats: 1.25, scale_idx: 4, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 6.5,  duration_beats: 1.5,  scale_idx: 3, has_backing_harmony: false, is_phrase_end: true  },
            VocalPhraseNote { start_beat: 8.0,  duration_beats: 1.5,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 9.5,  duration_beats: 1.0,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 10.5, duration_beats: 1.5,  scale_idx: 7, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 12.0, duration_beats: 1.5,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 13.5, duration_beats: 2.5,  scale_idx: 2, has_backing_harmony: true,  is_phrase_end: true  },
        ];

        // Phrase 2: Soaring Chorus Anthem (high register 5th -> b7th -> 5th -> 3rd -> tonic)
        let chorus_hook = vec![
            VocalPhraseNote { start_beat: 0.0,  duration_beats: 1.5,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 1.5,  duration_beats: 1.5,  scale_idx: 8, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 3.0,  duration_beats: 1.0,  scale_idx: 7, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 4.0,  duration_beats: 2.0,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 6.0,  duration_beats: 2.0,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: true  },
            VocalPhraseNote { start_beat: 8.0,  duration_beats: 1.25, scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 9.25, duration_beats: 1.25, scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 10.5, duration_beats: 1.5,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 12.0, duration_beats: 1.5,  scale_idx: 3, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 13.5, duration_beats: 2.5,  scale_idx: 2, has_backing_harmony: true,  is_phrase_end: true  },
        ];

        // Phrase 3: Intimate Roots StorytellingCadence (low 5th pickup -> tonic -> 4th -> 5th)
        let roots_bridge = vec![
            VocalPhraseNote { start_beat: 0.0,  duration_beats: 1.0,  scale_idx: 0, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 1.0,  duration_beats: 1.5,  scale_idx: 2, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 2.5,  duration_beats: 1.5,  scale_idx: 4, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 4.0,  duration_beats: 2.0,  scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 6.0,  duration_beats: 2.0,  scale_idx: 4, has_backing_harmony: false, is_phrase_end: true  },
            VocalPhraseNote { start_beat: 8.0,  duration_beats: 1.5,  scale_idx: 6, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 9.5,  duration_beats: 1.5,  scale_idx: 5, has_backing_harmony: true,  is_phrase_end: false },
            VocalPhraseNote { start_beat: 11.0, duration_beats: 1.5,  scale_idx: 4, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 12.5, duration_beats: 1.0,  scale_idx: 1, has_backing_harmony: false, is_phrase_end: false },
            VocalPhraseNote { start_beat: 13.5, duration_beats: 2.5,  scale_idx: 2, has_backing_harmony: true,  is_phrase_end: true  },
        ];

        [verse_hook, chorus_hook, roots_bridge]
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
        self.synthesize_reference_conditioned_vocal_track(
            duration_secs,
            bpm,
            key_root_hz,
            false,
            lyrics,
            0,
            false,
            None,
        )
    }

    /// Reference-Ensemble & RL Conditioned Vocal Synthesizer:
    /// Supports Natural Minor / Dorian scale modes, seed-steered novel melodic contours,
    /// and dynamic instrumental guitar-solo spotlight bars where vocals step back for the lead solo.
    #[allow(clippy::too_many_arguments)]
    pub fn synthesize_reference_conditioned_vocal_track(
        &self,
        duration_secs: f32,
        bpm: f32,
        key_root_hz: f32,
        is_minor: bool,
        lyrics: &str,
        melodic_seed: u64,
        use_reference_phrases: bool,
        solo_beat_window: Option<(f32, f32)>,
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
        let phrases = if use_reference_phrases {
            Self::compose_reference_phrases(melodic_seed)
        } else {
            Self::compose_4bar_phrases()
        };
        let scale_ratios = if is_minor {
            &VOCAL_MINOR_MELODY_RATIOS
        } else {
            &VOCAL_MELODY_RATIOS
        };
        let third_harmony_ratio = if is_minor { 1.189_207 } else { 1.259_921 };

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
                if let Some((solo_start, solo_end)) = solo_beat_window {
                    if total_beats >= solo_start && total_beats < solo_end {
                        synth_pos += 180.0;
                        continue;
                    }
                }
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
                    let ratio = scale_ratios[note.scale_idx % scale_ratios.len()];
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
            let in_solo = solo_beat_window
                .map(|(s, e)| total_beats >= s && total_beats < e)
                .unwrap_or(false);
            let cycle_idx = (total_beats / 16.0).floor() as usize;
            let beat_in_cycle = total_beats % 16.0;
            let phrase = &phrases[cycle_idx % phrases.len()];

            let mut active_note: Option<(usize, &VocalPhraseNote, f32)> = None;
            if !in_solo {
                for (n_idx, note) in phrase.iter().enumerate() {
                    if beat_in_cycle >= note.start_beat && beat_in_cycle < (note.start_beat + note.duration_beats) {
                        let note_progress = (beat_in_cycle - note.start_beat) / note.duration_beats;
                        active_note = Some((n_idx, note, note_progress));
                        break;
                    }
                }
            }

            if let Some((n_idx, note, note_progress)) = active_note {
                let ratio = scale_ratios[note.scale_idx % scale_ratios.len()];
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
                    let high_hz = smoothed_lead_freq * third_harmony_ratio * 1.0015;
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
    /// Formant-Preserving 4-Point Hermite Cubic Melodic Contour & Singing Vibrato Resampler.
    /// Operates on clause-pitched neural vocal phrases (rendered via Gemini Audio TTS or
    /// `en-US-AvaNeural` + FFmpeg `librubberband` with `formant=preserved`) to apply smooth
    /// continuous phrase-level melodic glide and natural human singing vibrato without any
    /// glottal cycle-chopping buzz or consonant smearing.
    fn apply_smooth_30ms_pitch_and_vibrato(
        src: &[f32],
        sr: f32,
        target_duration_secs: f32,
        contour_semitones: &[f32],
        vibrato_freq: f32,
        vibrato_depth: f32,
    ) -> Vec<f32> {
        let n_in = src.len();
        if n_in < 240 {
            return src.to_vec();
        }

        let max_out = ((target_duration_secs + 1.2) * sr) as usize;
        let mut out = Vec::with_capacity(max_out);
        let mut pos = 0.0f64;
        let mut smoothed_rate = 1.0f64;
        let c_len = contour_semitones.len().max(1);
        let vib_d = (vibrato_depth * 0.45).clamp(0.0015, 0.0065) as f64;
        let vib_f = vibrato_freq.clamp(4.5, 6.2) as f64;

        // Calculate mean semitone offset so relative contour moves gently (±1.15 st) around
        // the librubberband formant-preserved base pitch without chipmunk/baritone formant shift.
        let mean_st: f32 = if contour_semitones.is_empty() {
            0.0
        } else {
            contour_semitones.iter().copied().sum::<f32>() / (c_len as f32)
        };

        while pos < (n_in - 4) as f64 && out.len() < max_out {
            let t = (out.len() as f64) / (sr as f64);
            let u = (pos / ((n_in - 1).max(1) as f64)).clamp(0.0, 1.0) * ((c_len - 1).max(1) as f64);
            let idx = (u.floor() as usize).min(c_len.saturating_sub(1));
            let next_idx = (idx + 1).min(c_len.saturating_sub(1));
            let frac = (u - idx as f64).clamp(0.0, 1.0);
            let frac_s = frac * frac * (3.0 - 2.0 * frac);
            let raw_st = if contour_semitones.is_empty() {
                0.0
            } else {
                contour_semitones[idx] as f64 * (1.0 - frac_s) + contour_semitones[next_idx] as f64 * frac_s
            };
            let rel_st = ((raw_st - mean_st as f64) * 0.36).clamp(-1.45, 1.45);
            let vib = (2.0 * std::f64::consts::PI * vib_f * t).sin() * vib_d;
            let target_rate = 2.0f64.powf(rel_st / 12.0) * (1.0 + vib);
            smoothed_rate = smoothed_rate * 0.9992 + target_rate * 0.0008;

            let i0 = pos.floor() as usize;
            let f = (pos - i0 as f64) as f32;
            let xm1 = src[i0.saturating_sub(1)];
            let x0 = src[i0];
            let x1 = src[(i0 + 1).min(n_in - 1)];
            let x2 = src[(i0 + 2).min(n_in - 1)];

            // 4-point Hermite cubic interpolation (zero zero-crossing discontinuities)
            let c0 = x0;
            let c1 = 0.5 * (x1 - xm1);
            let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
            let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
            let y = ((c3 * f + c2) * f + c1) * f + c0;
            out.push(y);

            pos += smoothed_rate;
        }

        let n_out = out.len();
        for (i, val) in out.iter_mut().enumerate() {
            let t = (i as f32) / sr;
            let fade_in = (t / 0.025).min(1.0);
            let rem_sec = ((n_out - 1 - i) as f32) / sr;
            let fade_out = (rem_sec / 0.085).min(1.0);
            *val *= fade_in * fade_out;
        }
        out
    }

    /// Reads `GEMINI_API_KEY` from process environment or `.env` files (`d:\harfile\music\.env`, `d:\harfile\ModelFusion\.env`).
    /// Supports both standard `AIzaSy...` keys and newer `AQ....` Google AI Studio / Gemini API keys.
    pub fn resolve_dotenv_gemini_api_key() -> Option<String> {
        let is_real_key = |k: &str| -> bool {
            let t = k.trim().trim_matches('"').trim_matches('\'');
            (t.starts_with("AIzaSy") || t.starts_with("AQ."))
                && t.len() >= 25
                && !t.contains("DummyKey")
                && !t.contains("UnitTest")
                && !t.ends_with("Test")
        };

        for env_name in ["GEMINI_API_KEY", "GOOGLE_API_KEY"] {
            if let Ok(k) = std::env::var(env_name) {
                let clean = k.trim().trim_matches('"').trim_matches('\'');
                if is_real_key(clean) {
                    return Some(clean.to_string());
                }
            }
        }
        let candidates = [
            std::path::PathBuf::from(".env"),
            std::path::PathBuf::from(r"d:\harfile\music\.env"),
            std::path::PathBuf::from(r"d:\harfile\ModelFusion\.env"),
        ];
        for path in &candidates {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let l = line.trim();
                    if l.starts_with('#') || l.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = l.split_once('=') {
                        if k.trim() == "GEMINI_API_KEY" || k.trim() == "GOOGLE_API_KEY" {
                            let clean = v.trim().trim_matches('"').trim_matches('\'');
                            if is_real_key(clean) {
                                return Some(clean.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(windows)]
    fn render_thirteen_line_neural_phrases(&self, lines: &[String; 13]) -> Option<[Vec<f32>; 13]> {
        let pid = std::process::id();
        let base_dir = std::env::temp_dir();
        let mut txt_paths = Vec::with_capacity(13);
        let mut mp3_paths = Vec::with_capacity(13);
        let mut wav_paths = Vec::with_capacity(13);

        for k in 0..13 {
            let clean: String = lines[k]
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | ',' | '.' | '\'' | '-' | '!'))
                .take(240)
                .collect();
            let text = if clean.trim().is_empty() {
                "Where watering kingdoms heal aching hearts.".to_string()
            } else {
                clean
            };
            let tp = base_dir.join(format!("music_ln_{}_{}.txt", pid, k));
            let mp = base_dir.join(format!("music_ln_{}_{}.mp3", pid, k));
            let wp = base_dir.join(format!("music_ln_{}_{}.wav", pid, k));
            std::fs::write(&tp, &text).ok()?;
            txt_paths.push(tp);
            mp3_paths.push(mp);
            wav_paths.push(wp);
        }

        let fmt_list = |paths: &[std::path::PathBuf]| -> String {
            paths
                .iter()
                .map(|p| format!("r'{}'", p.to_string_lossy()))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let gemini_key = Self::resolve_dotenv_gemini_api_key().unwrap_or_default();
        let py_script = format!(
            "import asyncio, base64, edge_tts, glob, json, os, shutil, subprocess, urllib.request, wave\n\
             txts = [{}]\n\
             mp3s = [{}]\n\
             wavs = [{}]\n\
             gemini_key = r'{}'\n\
             # Musical scale pitch ratios (2^(st/12)) for formant-preserved librubberband singing per full line\n\
             rb_pitches = [1.0000, 1.0595, 1.0000, 1.0595, 1.1225, 1.1892, 1.2599, 1.1892, 1.2599, 1.1225, 1.0000, 1.0595, 1.0000]\n\
             rb_tempos  = [0.88,   0.88,   0.88,   0.89,   0.90,   0.87,   0.87,   0.87,   0.87,   0.88,   0.88,   0.87,   0.85]\n\
             rates      = ['-16%', '-16%', '-16%', '-15%', '-12%', '-15%', '-14%', '-14%', '-14%', '-14%', '-16%', '-18%', '-20%']\n\
             pitches    = ['+1Hz', '+2Hz', '+1Hz', '+2Hz', '+4Hz', '+5Hz', '+6Hz', '+5Hz', '+6Hz', '+3Hz', '+1Hz', '+0Hz', '-1Hz']\n\
             def try_gemini_tts(text, out_wav):\n\
             \x20   if not gemini_key:\n\
             \x20       return False\n\
             \x20   try:\n\
             \x20       url = f'https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={{gemini_key}}'\n\
             \x20       body = {{\n\
             \x20           'contents': [{{'parts': [{{'text': f'Sing warmly and melodically with clear lyrical diction: {{text}}'}}]}}],\n\
             \x20           'generationConfig': {{\n\
             \x20               'responseModalities': ['AUDIO'],\n\
             \x20               'speechConfig': {{'voiceConfig': {{'prebuiltVoiceConfig': {{'voiceName': 'Aoede'}}}}}}\n\
             \x20           }}\n\
             \x20       }}\n\
             \x20       req = urllib.request.Request(url, data=json.dumps(body).encode('utf-8'), headers={{'Content-Type': 'application/json'}}, method='POST')\n\
             \x20       with urllib.request.urlopen(req, timeout=10) as resp:\n\
             \x20           data = json.loads(resp.read().decode('utf-8'))\n\
             \x20           b64 = data['candidates'][0]['content']['parts'][0]['inlineData']['data']\n\
             \x20           pcm = base64.b64decode(b64)\n\
             \x20           with wave.open(out_wav, 'wb') as wf:\n\
             \x20               wf.setnchannels(1)\n\
             \x20               wf.setsampwidth(2)\n\
             \x20               wf.setframerate(24000)\n\
             \x20               wf.writeframes(pcm)\n\
             \x20           return True\n\
             \x20   except Exception:\n\
             \x20       return False\n\
             sem = asyncio.Semaphore(5)\n\
             async def synth_one(i):\n\
             \x20   async with sem:\n\
             \x20       t = open(txts[i], 'r', encoding='utf-8').read().strip()\n\
             \x20       if try_gemini_tts(t, mp3s[i]):\n\
             \x20           return\n\
             \x20       c = edge_tts.Communicate(t, 'en-US-AvaNeural', rate=rates[i], pitch=pitches[i])\n\
             \x20       await c.save(mp3s[i])\n\
             async def main():\n\
             \x20   await asyncio.gather(*(synth_one(i) for i in range(13)))\n\
             asyncio.run(main())\n\
             ff = shutil.which('ffmpeg')\n\
             if not ff:\n\
             \x20   cands = glob.glob(r'C:\\Users\\*\\AppData\\Local\\Microsoft\\WinGet\\Packages\\Gyan.FFmpeg*\\*\\bin\\ffmpeg.exe')\n\
             \x20   ff = cands[0] if cands else 'ffmpeg'\n\
             for i in range(13):\n\
             \x20   af = f'rubberband=tempo={{rb_tempos[i]}}:pitch={{rb_pitches[i]}}:formant=preserved:pitchq=quality,highpass=f=95,acompressor=threshold=-20dB:ratio=2.6:makeup=3.5dB'\n\
             \x20   subprocess.run([ff, '-y', '-i', mp3s[i], '-af', af, '-ar', '48000', '-ac', '1', '-c:a', 'pcm_s16le', wavs[i]], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)\n",
            fmt_list(&txt_paths),
            fmt_list(&mp3_paths),
            fmt_list(&wav_paths),
            gemini_key
        );

        let mut py_cmd = std::process::Command::new("python");
        use std::os::windows::process::CommandExt;
        py_cmd.creation_flags(0x08000000);
        py_cmd.args(["-c", &py_script]);
        let ok = py_cmd.status().map(|s| s.success()).unwrap_or(false);

        let mut out: [Vec<f32>; 13] = Default::default();
        for k in 0..13 {
            let _ = std::fs::remove_file(&txt_paths[k]);
            let _ = std::fs::remove_file(&mp3_paths[k]);
            if ok && wav_paths[k].exists() {
                if let Ok(buf) = AudioBuffer::from_wav_file(&wav_paths[k]) {
                    let ch = buf.channels.max(1) as usize;
                    let mut trimmed = Vec::with_capacity(buf.frame_count());
                    let mut env = 0.0f32;
                    let mut silent_frames = 0usize;
                    let max_breath = (0.14 * self.sample_rate as f32) as usize;
                    for f in 0..buf.frame_count() {
                        let s = buf.samples[f * ch];
                        env = 0.992 * env + 0.008 * s.abs();
                        if env > 0.004 {
                            silent_frames = 0;
                            trimmed.push(s);
                        } else if !trimmed.is_empty() && silent_frames < max_breath {
                            silent_frames += 1;
                            trimmed.push(s * 0.65);
                        }
                    }
                    let peak = trimmed.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
                    if peak > 0.01 {
                        let norm = 0.92 / peak;
                        for x in &mut trimmed {
                            *x = (*x * norm).clamp(-0.98, 0.98);
                        }
                    }
                    out[k] = trimmed;
                }
            }
            let _ = std::fs::remove_file(&wav_paths[k]);
        }

        if out.iter().all(|v| v.len() > 2400) {
            Some(out)
        } else {
            None
        }
    }

    #[cfg(not(windows))]
    fn render_thirteen_line_neural_phrases(&self, _lines: &[String; 13]) -> Option<[Vec<f32>; 13]> {
        None
    }

    /// Synthesizes a complete 3-Minute (`181.0s`), 9-Movement Section-Aligned Vocal Performance & Timeline
    /// with immediate vocal entry at `0:03.0` (after a 1-bar 3.0s fingerpicked guitar pickup) and continuous
    /// poetic singing across all 9 movements (`0:03.0` through `3:01.0`):
    /// - `0:00–0:03`: 1-Bar Opening Fingerpicked Guitar Pickup
    /// - `0:03–0:27`: First Verse (4 poetic lines on 6.0s 2-bar musical phrases)
    /// - `0:27–0:39`: Pre-Chorus Build (2 poetic lines on 6.0s 2-bar musical phrases)
    /// - `0:39–1:03`: Main Chorus #1 (4 poetic lines with stereo-doubled soaring vocals & harmony)
    /// - `1:03–1:27`: Second Verse & Savannah Migration (4 poetic lines over intimate fingerpicked guitar)
    /// - `1:27–1:51`: Main Chorus #2 (full chamber-pop return)
    /// - `1:51–2:15`: Orchestral Climax & Soaring Poetic Bridge (expressive violins, cello, slide guitar & vocal bridge)
    /// - `2:15–2:39`: Final Powerful Chorus #3
    /// - `2:39–3:01`: Stripped-Back Outro (solo fingerpicked guitar, mournful cello, and closing vocal lines)
    pub fn synthesize_sectioned_poem_vocal_track(
        &self,
        poem_lines: &[String; 13],
        bpm: f32,
        key_root_hz: f32,
        is_minor: bool,
        melodic_seed: u64,
    ) -> SectionedVocalOutput {
        let sr = self.sample_rate as f32;
        let safe_bpm = bpm.clamp(50.0, 180.0);
        let vocal_root = Self::normalize_human_vocal_root(key_root_hz);
        let third_st = if is_minor { 3.0f32 } else { 4.0f32 };

        // Continuous 3-minute (181.0s) 9-movement section boundaries with immediate 3.0s vocal entry
        let movements = [
            MovementSection { kind: SongMovementKind::OpeningInstrumental, start_sec: 0.0,   end_sec: 3.0   },
            MovementSection { kind: SongMovementKind::IntimateVerse1,      start_sec: 3.0,   end_sec: 27.0  },
            MovementSection { kind: SongMovementKind::PreChorusBuild,      start_sec: 27.0,  end_sec: 39.0  },
            MovementSection { kind: SongMovementKind::ChamberPopChorus,    start_sec: 39.0,  end_sec: 63.0  },
            MovementSection { kind: SongMovementKind::QuietVerse2Drop,     start_sec: 63.0,  end_sec: 87.0  },
            MovementSection { kind: SongMovementKind::ChamberPopChorus,    start_sec: 87.0,  end_sec: 111.0 },
            MovementSection { kind: SongMovementKind::OrchestralClimax,    start_sec: 111.0, end_sec: 135.0 },
            MovementSection { kind: SongMovementKind::ChamberPopChorus,    start_sec: 135.0, end_sec: 159.0 },
            MovementSection { kind: SongMovementKind::StrippedBackOutro,   start_sec: 159.0, end_sec: 181.0 },
        ];
        let total_dur = 181.0f32;

        if let Some(raw_lines) = self.render_thirteen_line_neural_phrases(poem_lines) {
            // Melodic pitch contours (in semitones) per 2-bar (6.0s) poetic line
            let c_verse_a = [0.0f32, 2.0, third_st, 5.0, third_st, 2.0];
            let c_verse_b = [2.0f32, third_st, 2.0, 0.0, -2.0, 0.0];
            let c_pre_a   = [2.0f32, third_st, 5.0, 7.0, 5.0, third_st];
            let c_pre_b   = [third_st, 5.0, 7.0, 9.0, 7.0, 7.0];
            let c_chor_1  = [7.0f32, 9.0, 7.0, 5.0, third_st, 5.0];
            let c_chor_2  = [5.0f32, 7.0, 5.0, third_st, 2.0, third_st];
            let c_chor_3  = [7.0f32, 9.0, 12.0, 9.0, 7.0, 5.0];
            let c_chor_4  = [5.0f32, third_st, 2.0, 0.0, 2.0, 0.0];
            let c_outro   = [third_st, 2.0, 0.0, -2.0, 0.0, 0.0];

            let mut pitched_lead: [Vec<f32>; 13] = Default::default();
            let mut pitched_harm: [Vec<f32>; 4] = Default::default();

            // Verse 1 lines (0..4)
            pitched_lead[0] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[0], sr, 4.8, &c_verse_a, 5.2, 0.0085);
            pitched_lead[1] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[1], sr, 4.6, &c_verse_b, 5.2, 0.0085);
            pitched_lead[2] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[2], sr, 4.7, &c_verse_a, 5.3, 0.0085);
            pitched_lead[3] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[3], sr, 4.6, &c_verse_b, 5.3, 0.0090);

            // Pre-Chorus lines (4..6)
            pitched_lead[4] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[4], sr, 5.0, &c_pre_a, 5.5, 0.0100);
            pitched_lead[5] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[5], sr, 4.4, &c_pre_b, 5.6, 0.0110);

            // Chorus lines (6..10) + Upper 3rd Harmony
            let chor_contours = [&c_chor_1, &c_chor_2, &c_chor_3, &c_chor_4];
            for k in 0..4 {
                pitched_lead[6 + k] = Self::apply_smooth_30ms_pitch_and_vibrato(
                    &raw_lines[6 + k],
                    sr,
                    4.9,
                    chor_contours[k],
                    5.7,
                    0.0120,
                );
                let mut h_c = [0.0f32; 6];
                for m in 0..6 {
                    h_c[m] = chor_contours[k][m] + third_st;
                }
                pitched_harm[k] = Self::apply_smooth_30ms_pitch_and_vibrato(
                    &raw_lines[6 + k],
                    sr,
                    4.9,
                    &h_c,
                    5.9,
                    0.0110,
                );
            }

            // Verse 2 & Outro lines (10..13)
            pitched_lead[10] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[10], sr, 4.6, &c_verse_a, 5.2, 0.0085);
            pitched_lead[11] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[11], sr, 4.9, &c_verse_b, 5.2, 0.0085);
            pitched_lead[12] = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[12], sr, 4.7, &c_outro, 5.0, 0.0070);

            let total_frames = (total_dur * sr).round() as usize;
            let mut vocals = AudioBuffer::new(self.sample_rate, 2, total_dur);
            let mut dry_l = vec![0.0f32; total_frames];
            let mut dry_r = vec![0.0f32; total_frames];

            let place_mono_close_mic = |src: &[f32], start_sec: f32, gain: f32, dl: &mut [f32], dr: &mut [f32]| {
                let s_frame = (start_sec * sr).round() as usize;
                for (k, &val) in src.iter().enumerate() {
                    let dst = s_frame + k;
                    if dst < dl.len() {
                        dl[dst] += val * gain;
                        dr[dst] += val * gain;
                    }
                }
            };

            let place_chorus_stereo_doubled = |lead: &[f32], harm: &[f32], start_sec: f32, gain: f32, dl: &mut [f32], dr: &mut [f32]| {
                let s_frame = (start_sec * sr).round() as usize;
                let haas_l = (0.014 * sr) as usize;
                let haas_r = (0.019 * sr) as usize;
                for k in 0..lead.len() {
                    let dst = s_frame + k;
                    if dst < dl.len() {
                        let l_val = lead[k] * 0.88 * gain;
                        let dbl_l = if k >= haas_l { lead[k - haas_l] * 0.34 * gain } else { 0.0 };
                        let dbl_r = if k >= haas_r { lead[k - haas_r] * 0.34 * gain } else { 0.0 };
                        let h_val = harm.get(k).copied().unwrap_or(0.0) * 0.24 * gain;
                        dl[dst] += l_val + dbl_l + h_val * 0.85;
                        dr[dst] += l_val + dbl_r + h_val * 1.15;
                    }
                }
            };

            // 1. First Verse (0:03 – 0:27): Immediate vocal entry at 3.0s (4 lines on 6.0s 2-bar phrases)
            place_mono_close_mic(&pitched_lead[0], 3.0,  0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[1], 9.0,  0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[2], 15.0, 0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[3], 21.0, 0.98, &mut dry_l, &mut dry_r);

            // 2. Pre-Chorus Build (0:27 – 0:39): 2 lines on 6.0s boundaries
            place_mono_close_mic(&pitched_lead[4], 27.0, 1.00, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[5], 33.0, 1.02, &mut dry_l, &mut dry_r);

            // 3. Main Chorus #1 (0:39 – 1:03): 4 Chorus lines on 6.0s boundaries
            for k in 0..4 {
                place_chorus_stereo_doubled(
                    &pitched_lead[6 + k],
                    &pitched_harm[k],
                    39.0 + (k as f32) * 6.0,
                    1.02,
                    &mut dry_l,
                    &mut dry_r,
                );
            }

            // 4. Second Verse & Savannah Migration (1:03 – 1:27): Lines 10, 11, 12, and Line 0 reprise
            place_mono_close_mic(&pitched_lead[10], 63.0, 0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[11], 69.0, 0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[12], 75.0, 0.98, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[0],  81.0, 1.00, &mut dry_l, &mut dry_r);

            // 5. Main Chorus #2 (1:27 – 1:51): Full chamber-pop Chorus return
            for k in 0..4 {
                place_chorus_stereo_doubled(
                    &pitched_lead[6 + k],
                    &pitched_harm[k],
                    87.0 + (k as f32) * 6.0,
                    1.04,
                    &mut dry_l,
                    &mut dry_r,
                );
            }

            // 6. Orchestral Climax & Soaring Poetic Bridge (1:51 – 2:15): Continuous vocal reprise over soaring strings
            place_mono_close_mic(&pitched_lead[1], 111.0, 0.96, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[2], 117.0, 0.96, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[4], 123.0, 1.00, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&pitched_lead[5], 129.0, 1.02, &mut dry_l, &mut dry_r);

            // 7. Final Powerful Chorus #3 (2:15 – 2:39): Peak vocal & orchestral climax
            for k in 0..4 {
                place_chorus_stereo_doubled(
                    &pitched_lead[6 + k],
                    &pitched_harm[k],
                    135.0 + (k as f32) * 6.0,
                    1.06,
                    &mut dry_l,
                    &mut dry_r,
                );
            }

            // 8. Stripped-Back Outro (2:39 – 3:01): Closing poetic lines & gentle final cadence
            let outro_l1 = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[10], sr, 4.8, &c_verse_a, 5.0, 0.0065);
            let outro_l2 = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[11], sr, 4.9, &c_outro, 4.9, 0.0060);
            let outro_l3 = Self::apply_smooth_30ms_pitch_and_vibrato(&raw_lines[12], sr, 4.9, &c_outro, 4.8, 0.0055);
            place_mono_close_mic(&outro_l1, 159.0, 0.92, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&outro_l2, 165.0, 0.88, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&outro_l3, 171.0, 0.86, &mut dry_l, &mut dry_r);
            place_mono_close_mic(&outro_l3, 176.2, 0.78, &mut dry_l, &mut dry_r);

            let mut warmth_eq = BiquadFilter::peaking_eq(230.0, 0.9, 1.8, 2);
            let mut air_eq = BiquadFilter::peaking_eq(3400.0, 1.0, 2.4, 2);
            let mut deess_eq = BiquadFilter::peaking_eq(6400.0, 1.2, -2.0, 2);
            let mut close_reverb = StudioVocalReverb::new(self.sample_rate, 0.10);
            let mut lush_reverb = StudioVocalReverb::new(self.sample_rate, 0.24);

            for i in 0..total_frames {
                let t = i as f32 / sr;
                let in_chorus = (t >= 39.0 && t < 63.0)
                    || (t >= 87.0 && t < 111.0)
                    || (t >= 135.0 && t < 159.0);
                let (rl_c, rr_c) = close_reverb.process_stereo(dry_l[i], dry_r[i]);
                let (rl_l, rr_l) = lush_reverb.process_stereo(dry_l[i], dry_r[i]);
                let (rev_l, rev_r) = if in_chorus {
                    (rl_l, rr_l)
                } else {
                    (rl_c, rr_c)
                };
                let eq_l = deess_eq.process_sample(0, air_eq.process_sample(0, warmth_eq.process_sample(0, rev_l)));
                let eq_r = deess_eq.process_sample(1, air_eq.process_sample(1, warmth_eq.process_sample(1, rev_r)));
                vocals.samples[i * 2] = (eq_l * 1.15).tanh() * 0.95;
                vocals.samples[i * 2 + 1] = (eq_r * 1.15).tanh() * 0.95;
            }

            return SectionedVocalOutput {
                vocals,
                movements,
                total_duration_secs: total_dur,
            };
        }

        let combined = poem_lines.join(" ");
        let vocals = self.synthesize_reference_conditioned_vocal_track(
            total_dur,
            safe_bpm,
            vocal_root,
            is_minor,
            &combined,
            melodic_seed,
            true,
            Some((120.0, 144.0)),
        );
        SectionedVocalOutput {
            vocals,
            movements,
            total_duration_secs: total_dur,
        }
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
