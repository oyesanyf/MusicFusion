//! Strongly-typed error definitions for digital signal processing and audio I/O.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DspError {
    #[error("I/O error during audio processing: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid WAV file header: {0}")]
    InvalidWavHeader(String),

    #[error("Unsupported WAV format: format tag {format_tag}, bits per sample {bits_per_sample}")]
    UnsupportedFormat { format_tag: u16, bits_per_sample: u16 },

    #[error("Mismatched sample rates: expected {expected} Hz, got {actual} Hz")]
    SampleRateMismatch { expected: u32, actual: u32 },

    #[error("Mismatched channel counts: buffer A has {channels_a}, buffer B has {channels_b}")]
    ChannelMismatch { channels_a: u16, channels_b: u16 },

    #[error("Buffer parameter error: {0}")]
    ParameterError(String),
}
