//! `music_dsp`: Pure-Rust 48 kHz Digital Signal Processing (DSP) Engine.
//!
//! Provides authentic, broadcast-grade audio synthesis, physical string modeling,
//! vocal formant/rasp simulation, sample-accurate stem crossfading, pure-Rust RIFF WAV I/O,
//! spectral telemetry analysis, and analog tape mastering with zero external C dependencies.

pub mod acoustic_analyzer;
pub mod crossfader;
pub mod error;
pub mod groove_synth;
pub mod guitar_wah;
pub mod melodic_singing;
pub mod reference_rl_synth;
pub mod rvq;
pub mod tape_mastering;
pub mod telemetry_calc;
pub mod vocal_formant;
pub mod wav;

// Re-exports
pub use acoustic_analyzer::{AcousticAnalyzer, ANTI_MONOPOLY_CEILING, REFERENCE_CAP_LIMIT};
pub use error::DspError;
pub use melodic_singing::{
    MelodicSingingSynthesizer, StudioVocalReverb, MAJOR_PENTATONIC_RATIOS,
};
pub use reference_rl_synth::{ReferenceRlSongOutput, ReferenceRlSynthesizer};
pub use rvq::{
    RvqTokenizer, RvqTokenSequence, DEFAULT_CODEBOOK_DIM, DEFAULT_CODEBOOK_SIZE,
    DEFAULT_RVQ_STAGES,
};
pub use wav::{AudioBuffer, WavBitDepth, WavFormat, WavSpec, DEFAULT_SAMPLE_RATE};

pub use telemetry_calc::{
    calculate_phase_alignment, calculate_snr, calculate_spectral_flatness,
    calculate_telemetry, calculate_tempo_drift, fft_radix2, SpectralTelemetryEngine,
};

pub use groove_synth::{
    generate_groove, synthesize_groove, synthesize_reggae_one_drop_drums_and_dub_bass,
    DrumHit, DrumVoice, DspRng, DubSubBass, HammondOrganBubble,
    UniversalDrumSynth, UniversalDrumSynthesizer,
};

pub use guitar_wah::{
    synthesize_guitar_solo, synthesize_lead_solo, synthesize_reggae_skank_chord,
    synthesize_reggae_skank_track, A_MAJOR_PENTATONIC, ContinuousPhaseAccumulator, GuitarAmpSimulator,
    KARPLUS_FEEDBACK_DAMPING, KarplusStrongGuitar, KarplusStrongString, PinkNoiseGenerator,
    SoloGenerator, SoloNote, WahWahFilter,
};

pub use vocal_formant::{
    apply_vocal_formant_and_rasp, apply_vocal_timbre, BiquadFilter, FormantFilterBank,
    FormantParams, PitchLockVocoder, VocalArchetype, VocalRaspProcessor,
};

pub use crossfader::{
    equal_power_crossfade, splice_stem, CrossfadeCurve, StemCrossfader,
};

pub use tape_mastering::{
    apply_analog_mastering, apply_analog_tape_mastering, AnalogTapeSaturator,
    BrickwallLimiter, ButterworthLowpass12kHz, MasteringConfig, MasteringProfile,
    StereoSpatialProcessor, SubBassWarmthFilter, BRICKWALL_CEILING,
};

pub const SAMPLE_RATE: u32 = 48_000;

pub fn dsp_version() -> &'static str {
    "0.2.0-milestone2"
}
