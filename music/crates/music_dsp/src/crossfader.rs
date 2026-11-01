//! `crossfader`: Sample-Accurate Stem Inpainting Crossfader locked to BPM beat grid
//! with Equal-Power (Sine/Cosine, Square Root) and Linear crossfade curves.

use std::f32::consts::PI;
use crate::wav::AudioBuffer;

pub const SAMPLE_RATE: u32 = 48_000;

/// Crossfade transition curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfadeCurve {
    /// Equal-Power Sine/Cosine: w1 = cos(pi/2 * t), w2 = sin(pi/2 * t).
    /// Guarantees strictly constant total acoustic power (w1^2 + w2^2 = 1.0).
    EqualPowerSine,
    /// Equal-Power Square Root: w1 = sqrt(1 - t), w2 = sqrt(t).
    EqualPowerSqrt,
    /// Linear crossfade: w1 = 1 - t, w2 = t (for strictly correlated/phase-locked stems).
    Linear,
}

impl CrossfadeCurve {
    #[inline]
    pub fn compute_weights(&self, t: f32) -> (f32, f32) {
        let t_norm = t.clamp(0.0, 1.0);
        match self {
            CrossfadeCurve::EqualPowerSine => {
                let theta = t_norm * (PI / 2.0);
                (theta.cos(), theta.sin())
            }
            CrossfadeCurve::EqualPowerSqrt => {
                ((1.0 - t_norm).sqrt(), t_norm.sqrt())
            }
            CrossfadeCurve::Linear => {
                (1.0 - t_norm, t_norm)
            }
        }
    }
}

/// Sample-accurate stem crossfader and inpainting splice engine.
pub struct StemCrossfader;

impl StemCrossfader {
    /// Slices and crossfades an inpainted stem (e.g. guitar solo, drum fill) into a base stem.
    pub fn inpaint_splice(
        base: &AudioBuffer,
        splice: &AudioBuffer,
        start_frame: usize,
        crossfade_frames: usize,
        curve: CrossfadeCurve,
    ) -> AudioBuffer {
        let mut out = base.clone();
        let total_frames = out.frame_count();
        let splice_frames = splice.frame_count();
        let channels = out.channels as usize;

        if start_frame >= total_frames || splice_frames == 0 {
            return out;
        }

        let xfade_len = crossfade_frames.min(splice_frames / 2).max(1);

        for i in 0..splice_frames {
            let target_frame = start_frame + i;
            if target_frame >= total_frames {
                break;
            }

            let (w_base, w_splice) = if i < xfade_len {
                // Entry transition: fade from base to splice
                let t = i as f32 / xfade_len as f32;
                curve.compute_weights(t)
            } else if i >= splice_frames - xfade_len {
                // Exit transition: fade from splice back to base
                let t = (splice_frames - 1 - i) as f32 / xfade_len as f32;
                let (w_exit_splice, w_exit_base) = curve.compute_weights(1.0 - t);
                (w_exit_base, w_exit_splice)
            } else {
                // Sustained body: 100% splice
                (0.0, 1.0)
            };

            for c in 0..channels {
                let s_base = base.samples[target_frame * channels + c];
                let s_splice = if c < splice.channels as usize {
                    splice.samples[i * splice.channels as usize + c]
                } else {
                    splice.samples[i * splice.channels as usize]
                };
                let combined = w_base * s_base + w_splice * s_splice;
                out.samples[target_frame * channels + c] = combined.clamp(-1.0, 1.0);
            }
        }

        out
    }

    /// Slices an inpainted section locked to musical bar boundaries.
    pub fn inpaint_bars(
        base: &AudioBuffer,
        splice: &AudioBuffer,
        start_bar: u32,
        bpm: u32,
        beats_per_bar: u32,
        crossfade_ms: f32,
        curve: CrossfadeCurve,
    ) -> AudioBuffer {
        let samples_per_beat = (60.0 / bpm as f32 * SAMPLE_RATE as f32).round() as usize;
        let samples_per_bar = samples_per_beat * beats_per_bar as usize;
        let start_frame = start_bar as usize * samples_per_bar;
        let xfade_frames = (crossfade_ms / 1000.0 * SAMPLE_RATE as f32).round() as usize;

        Self::inpaint_splice(base, splice, start_frame, xfade_frames, curve)
    }
}

/// Standalone equal-power crossfader function matching `tests/common/mod.rs` line 532.
pub fn equal_power_crossfade(
    base: &AudioBuffer,
    splice: &AudioBuffer,
    start_frame: usize,
    crossfade_frames: usize,
) -> AudioBuffer {
    StemCrossfader::inpaint_splice(
        base,
        splice,
        start_frame,
        crossfade_frames,
        CrossfadeCurve::EqualPowerSqrt,
    )
}

/// BPM grid-locked stem splicing helper.
pub fn splice_stem(
    base: &AudioBuffer,
    splice: &AudioBuffer,
    start_bar: u32,
    bpm: u32,
    beats_per_bar: u32,
) -> AudioBuffer {
    StemCrossfader::inpaint_bars(
        base,
        splice,
        start_bar,
        bpm,
        beats_per_bar,
        15.0, // 15 ms smooth transition
        CrossfadeCurve::EqualPowerSine,
    )
}
