//! 48 kHz Pure-Rust AudioBuffer and RIFF/WAVE Reader/Writer.
//!
//! Supports 16-bit PCM, 24-bit PCM, and 32-bit IEEE float WAV formats
//! without any external C library dependencies.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::DspError;

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Bit depth format for WAV encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WavBitDepth {
    Pcm16,
    Pcm24,
    Float32,
}

/// Alias for WavBitDepth matching format specifications.
pub type WavFormat = WavBitDepth;

/// Specification metadata for WAV files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WavSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: WavBitDepth,
}

/// 48 kHz Stereo / Mono Audio Buffer for Pure-Rust DSP Processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>, // Interleaved: L, R, L, R... for stereo; L0, L1... for mono
}

impl AudioBuffer {
    /// Constructs a new silent stereo buffer for the given duration in seconds at 48 kHz.
    pub fn new_stereo(duration_secs: f32) -> Self {
        Self::new(DEFAULT_SAMPLE_RATE, 2, duration_secs)
    }

    /// Constructs a new silent mono buffer for the given duration in seconds at 48 kHz.
    pub fn new_mono(duration_secs: f32) -> Self {
        Self::new(DEFAULT_SAMPLE_RATE, 1, duration_secs)
    }

    /// Constructs a new silent audio buffer with specified sample rate, channel count, and duration.
    pub fn new(sample_rate: u32, channels: u16, duration_secs: f32) -> Self {
        let frame_count = (duration_secs * sample_rate as f32).max(0.0).round() as usize;
        let total_samples = frame_count * channels as usize;
        Self {
            sample_rate,
            channels,
            samples: vec![0.0; total_samples],
        }
    }

    /// Constructs an AudioBuffer from raw samples, validating channel alignment.
    pub fn from_samples(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, DspError> {
        if channels == 0 {
            return Err(DspError::ParameterError("Channel count must be > 0".to_string()));
        }
        if sample_rate == 0 {
            return Err(DspError::ParameterError("Sample rate must be > 0".to_string()));
        }
        if samples.len() % channels as usize != 0 {
            return Err(DspError::ParameterError(format!(
                "Sample count {} is not aligned with channel count {}",
                samples.len(),
                channels
            )));
        }
        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// Number of frames (independent of channel count).
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f32 {
        if self.sample_rate == 0 || self.channels == 0 {
            0.0
        } else {
            self.frame_count() as f32 / self.sample_rate as f32
        }
    }

    /// True if the buffer contains zero samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Retrieves sample value at specific frame and channel index.
    pub fn channel_sample(&self, frame: usize, channel: usize) -> Option<f32> {
        if channel >= self.channels as usize || frame >= self.frame_count() {
            None
        } else {
            Some(self.samples[frame * self.channels as usize + channel])
        }
    }

    /// Sets sample value at specific frame and channel index.
    pub fn set_channel_sample(&mut self, frame: usize, channel: usize, val: f32) -> bool {
        if channel >= self.channels as usize || frame >= self.frame_count() {
            false
        } else {
            self.samples[frame * self.channels as usize + channel] = val;
            true
        }
    }

    /// Mixes another audio buffer into `self` with a scalar gain multiplier.
    pub fn mix(&mut self, other: &AudioBuffer, gain: f32) -> Result<(), DspError> {
        if self.sample_rate != other.sample_rate {
            return Err(DspError::SampleRateMismatch {
                expected: self.sample_rate,
                actual: other.sample_rate,
            });
        }

        let mix_frames = self.frame_count().min(other.frame_count());

        if self.channels == other.channels {
            for frame in 0..mix_frames {
                for c in 0..self.channels as usize {
                    let idx_self = frame * self.channels as usize + c;
                    let idx_other = frame * other.channels as usize + c;
                    self.samples[idx_self] += other.samples[idx_other] * gain;
                }
            }
        } else if self.channels == 2 && other.channels == 1 {
            // Mono into Stereo: add mono signal equally to Left and Right
            for frame in 0..mix_frames {
                let mono_s = other.samples[frame] * gain;
                self.samples[frame * 2] += mono_s;
                self.samples[frame * 2 + 1] += mono_s;
            }
        } else if self.channels == 1 && other.channels == 2 {
            // Stereo into Mono: sum L + R scaled by 0.5
            for frame in 0..mix_frames {
                let stereo_s = 0.5 * (other.samples[frame * 2] + other.samples[frame * 2 + 1]) * gain;
                self.samples[frame] += stereo_s;
            }
        } else {
            return Err(DspError::ChannelMismatch {
                channels_a: self.channels,
                channels_b: other.channels,
            });
        }

        Ok(())
    }

    /// Multiplies all samples in the buffer by a linear gain factor.
    pub fn scale_gain(&mut self, gain: f32) {
        for s in &mut self.samples {
            *s *= gain;
        }
    }

    /// Multiplies all samples in the buffer by a gain specified in decibels (dB).
    pub fn scale_db(&mut self, gain_db: f32) {
        let linear_gain = 10.0f32.powf(gain_db / 20.0);
        self.scale_gain(linear_gain);
    }

    /// Normalizes audio buffer so its peak amplitude reaches target_peak_dbfs.
    /// Returns the linear gain factor applied.
    pub fn normalize(&mut self, target_peak_dbfs: f32) -> f32 {
        let max_abs = self.peak_level();

        if max_abs <= 1e-9 {
            return 1.0; // Silent buffer, no scaling
        }

        let target_linear = 10.0f32.powf(target_peak_dbfs / 20.0);
        let factor = target_linear / max_abs;
        self.scale_gain(factor);
        factor
    }

    /// Clamps all sample values between min and max.
    pub fn clamp(&mut self, min: f32, max: f32) {
        for s in &mut self.samples {
            *s = s.clamp(min, max);
        }
    }

    /// Standard broadcast safety clamp between -1.0 and 1.0.
    pub fn clamp_broadcast(&mut self) {
        self.clamp(-1.0, 1.0);
    }

    /// Calculates absolute peak amplitude of the buffer.
    pub fn peak_level(&self) -> f32 {
        let mut max_abs = 0.0f32;
        for &s in &self.samples {
            let abs_s = s.abs();
            if abs_s > max_abs {
                max_abs = abs_s;
            }
        }
        max_abs
    }

    /// Calculates Root-Mean-Square (RMS) amplitude of the buffer.
    pub fn rms_level(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sum_sq = 0.0f64;
        for &s in &self.samples {
            sum_sq += (s as f64).powi(2);
        }
        (sum_sq / self.samples.len() as f64).sqrt() as f32
    }

    /// Extracts a sub-buffer slice from start_frame for frame_count frames.
    pub fn slice(&self, start_frame: usize, frame_count: usize) -> AudioBuffer {
        let total_frames = self.frame_count();
        if start_frame >= total_frames || frame_count == 0 {
            return AudioBuffer {
                sample_rate: self.sample_rate,
                channels: self.channels,
                samples: Vec::new(),
            };
        }

        let actual_frames = frame_count.min(total_frames - start_frame);
        let start_sample = start_frame * self.channels as usize;
        let end_sample = (start_frame + actual_frames) * self.channels as usize;

        AudioBuffer {
            sample_rate: self.sample_rate,
            channels: self.channels,
            samples: self.samples[start_sample..end_sample].to_vec(),
        }
    }

    /// Linear interpolation resampler for arbitrary sample rates.
    pub fn resample_linear(&self, target_sample_rate: u32) -> AudioBuffer {
        if self.sample_rate == target_sample_rate || self.is_empty() {
            return self.clone();
        }

        let ratio = self.sample_rate as f64 / target_sample_rate as f64;
        let target_frames = ((self.frame_count() as f64) / ratio).round() as usize;
        let ch = self.channels as usize;
        let mut target_samples = Vec::with_capacity(target_frames * ch);

        for i in 0..target_frames {
            let src_pos = i as f64 * ratio;
            let src_idx = src_pos.floor() as usize;
            let frac = (src_pos - src_idx as f64) as f32;

            for c in 0..ch {
                let s0 = self.channel_sample(src_idx, c).unwrap_or(0.0);
                let s1 = self.channel_sample(src_idx + 1, c).unwrap_or(s0);
                target_samples.push(s0 * (1.0 - frac) + s1 * frac);
            }
        }

        AudioBuffer {
            sample_rate: target_sample_rate,
            channels: self.channels,
            samples: target_samples,
        }
    }

    /// Converts stereo buffer to mono by averaging channels.
    pub fn to_mono(&self) -> AudioBuffer {
        if self.channels == 1 {
            return self.clone();
        }
        let frames = self.frame_count();
        let mut mono_samples = Vec::with_capacity(frames);
        for f in 0..frames {
            let mut sum = 0.0f32;
            for c in 0..self.channels as usize {
                sum += self.samples[f * self.channels as usize + c];
            }
            mono_samples.push(sum / self.channels as f32);
        }
        AudioBuffer {
            sample_rate: self.sample_rate,
            channels: 1,
            samples: mono_samples,
        }
    }

    /// Converts mono buffer to stereo by duplicating channel.
    pub fn to_stereo(&self) -> AudioBuffer {
        if self.channels == 2 {
            return self.clone();
        }
        let frames = self.frame_count();
        let mut stereo_samples = Vec::with_capacity(frames * 2);
        for f in 0..frames {
            let s = self.samples[f];
            stereo_samples.push(s);
            stereo_samples.push(s);
        }
        AudioBuffer {
            sample_rate: self.sample_rate,
            channels: 2,
            samples: stereo_samples,
        }
    }

    // =========================================================================
    // Pure-Rust RIFF/WAVE Encoder & Decoder
    // =========================================================================

    /// Encodes AudioBuffer into standard 32-bit IEEE float RIFF WAV byte vector.
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        self.to_wav_bytes_with_format(WavBitDepth::Float32)
    }

    /// Encodes AudioBuffer into RIFF WAV byte vector with specified bit depth.
    pub fn to_wav_bytes_with_format(&self, bit_depth: WavBitDepth) -> Vec<u8> {
        let num_channels = self.channels;
        let sample_rate = self.sample_rate;

        let (audio_format, bits_per_sample, bytes_per_sample) = match bit_depth {
            WavBitDepth::Pcm16 => (1u16, 16u16, 2usize),
            WavBitDepth::Pcm24 => (1u16, 24u16, 3usize),
            WavBitDepth::Float32 => (3u16, 32u16, 4usize),
        };

        let block_align = num_channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;
        let data_size = (self.samples.len() * bytes_per_sample) as u32;
        let file_size = 36u32 + data_size;

        let mut bytes = Vec::with_capacity((file_size + 8) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size
        bytes.extend_from_slice(&audio_format.to_le_bytes());
        bytes.extend_from_slice(&num_channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());

        match bit_depth {
            WavBitDepth::Float32 => {
                for &sample in &self.samples {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
            }
            WavBitDepth::Pcm16 => {
                for &sample in &self.samples {
                    let clamped = sample.clamp(-1.0, 0.999969);
                    let val = (clamped * 32767.0).round() as i16;
                    bytes.extend_from_slice(&val.to_le_bytes());
                }
            }
            WavBitDepth::Pcm24 => {
                for &sample in &self.samples {
                    let clamped = sample.clamp(-1.0, 0.99999988);
                    let val = (clamped * 8_388_607.0).round() as i32;
                    let le = val.to_le_bytes();
                    bytes.push(le[0]);
                    bytes.push(le[1]);
                    bytes.push(le[2]);
                }
            }
        }

        bytes
    }

    /// Decodes RIFF WAV byte slice into AudioBuffer. Supports 16-bit, 24-bit PCM and 32-bit float.
    pub fn from_wav_bytes(bytes: &[u8]) -> Result<Self, DspError> {
        if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(DspError::InvalidWavHeader("Missing RIFF/WAVE signature".to_string()));
        }

        let mut offset = 12;
        let mut audio_format = 1u16;
        let mut channels = 2u16;
        let mut sample_rate = DEFAULT_SAMPLE_RATE;
        let mut bits_per_sample = 32u16;
        let mut data_offset = 0;
        let mut data_size = 0;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes(
                bytes[offset + 4..offset + 8]
                    .try_into()
                    .map_err(|_| DspError::InvalidWavHeader("Corrupted chunk size".to_string()))?,
            ) as usize;
            offset += 8;

            if chunk_id == b"fmt " && chunk_size >= 16 {
                if offset + 16 > bytes.len() {
                    return Err(DspError::InvalidWavHeader("Truncated fmt chunk".to_string()));
                }
                audio_format = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
                channels = u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into().unwrap());
                sample_rate = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
                bits_per_sample = u16::from_le_bytes(bytes[offset + 14..offset + 16].try_into().unwrap());
            } else if chunk_id == b"data" {
                data_offset = offset;
                data_size = chunk_size.min(bytes.len().saturating_sub(offset));
                break;
            }
            offset += chunk_size;
        }

        if data_offset == 0 {
            return Err(DspError::InvalidWavHeader("Missing data chunk".to_string()));
        }

        let mut samples = Vec::new();
        if audio_format == 3 && bits_per_sample == 32 {
            let count = data_size / 4;
            samples.reserve(count);
            for i in 0..count {
                let start = data_offset + i * 4;
                if start + 4 <= bytes.len() {
                    let val = f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
                    samples.push(val);
                }
            }
        } else if audio_format == 1 && bits_per_sample == 16 {
            let count = data_size / 2;
            samples.reserve(count);
            for i in 0..count {
                let start = data_offset + i * 2;
                if start + 2 <= bytes.len() {
                    let val = i16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
                    samples.push(val as f32 / 32768.0);
                }
            }
        } else if audio_format == 1 && bits_per_sample == 24 {
            let count = data_size / 3;
            samples.reserve(count);
            for i in 0..count {
                let start = data_offset + i * 3;
                if start + 3 <= bytes.len() {
                    let b0 = bytes[start] as u32;
                    let b1 = bytes[start + 1] as u32;
                    let b2 = bytes[start + 2] as u32;
                    let raw = b0 | (b1 << 8) | (b2 << 16);
                    let signed = if (raw & 0x0080_0000) != 0 {
                        (raw | 0xFF00_0000) as i32
                    } else {
                        raw as i32
                    };
                    samples.push(signed as f32 / 8_388_608.0);
                }
            }
        } else {
            return Err(DspError::UnsupportedFormat {
                format_tag: audio_format,
                bits_per_sample,
            });
        }

        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// Writes AudioBuffer directly to disk file in WAV format.
    pub fn to_wav_file<P: AsRef<Path>>(&self, path: P) -> Result<(), DspError> {
        self.to_wav_file_with_format(path, WavBitDepth::Float32)
    }

    /// Writes AudioBuffer to disk file with specified bit depth.
    pub fn to_wav_file_with_format<P: AsRef<Path>>(&self, path: P, bit_depth: WavBitDepth) -> Result<(), DspError> {
        let bytes = self.to_wav_bytes_with_format(bit_depth);
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Reads AudioBuffer directly from a WAV file on disk.
    pub fn from_wav_file<P: AsRef<Path>>(path: P) -> Result<Self, DspError> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::from_wav_bytes(&bytes)
    }
}
