use serde::{Deserialize, Serialize};

/// Audio telemetry metrics calculated from waveform and spectral analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemTelemetry {
    /// Peak amplitude in dBFS (0.0 dBFS is digital maximum)
    pub peak_dbfs: f32,
    /// Root-Mean-Square loudness in dBFS
    pub rms_dbfs: f32,
    /// Dynamic range in dB (difference between peak and noise floor / quiet passages)
    pub dynamic_range_db: f32,
    /// Tempo drift score (0.0 = perfect grid lock, >0.05 = noticeable drift)
    pub tempo_drift_score: f32,
    /// True if any sample reaches or exceeds 0.0 dBFS
    pub clipping: bool,
    /// Signal-to-noise ratio in dB
    pub snr_db: f32,
    /// Spectral flatness measure (0.0 = pure tone, 1.0 = white noise)
    pub spectral_flatness: f32,
    /// True peak headroom in dB (distance below 0.0 dBFS)
    pub headroom_db: f32,
    /// Duration of audio in seconds
    pub duration_secs: f32,
    /// Sampling rate in Hz (standard: 48000 Hz)
    pub sample_rate: u32,
    /// Channel count (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Phase alignment score (1.0 = perfectly in phase, <0.7 = phase cancellation risk)
    pub phase_alignment_score: f32,
    /// Vocal-to-instrumental headroom in dB
    pub vocal_headroom_db: f32,
}

impl Default for StemTelemetry {
    fn default() -> Self {
        Self {
            peak_dbfs: -0.5,
            rms_dbfs: -14.0,
            dynamic_range_db: 12.0,
            tempo_drift_score: 0.002,
            clipping: false,
            snr_db: 92.0,
            spectral_flatness: 0.18,
            headroom_db: 0.5,
            duration_secs: 0.0,
            sample_rate: 48_000,
            channels: 2,
            phase_alignment_score: 0.98,
            vocal_headroom_db: 3.5,
        }
    }
}

impl StemTelemetry {
    /// Standard studio nominal telemetry profile.
    pub fn nominal() -> Self {
        Self::default()
    }

    /// Broadcast-ready standard telemetry profile.
    pub fn new_broadcast_standard() -> Self {
        Self::default()
    }

    /// Evaluates whether the audio stem passes studio digital quality inspection.
    pub fn is_broadcast_ready(&self) -> bool {
        !self.clipping
            && self.peak_dbfs <= 0.0
            && self.dynamic_range_db >= 6.0
            && self.tempo_drift_score < 0.05
            && self.rms_dbfs <= -6.0
            && self.phase_alignment_score >= 0.70
    }

    /// Alias for is_broadcast_ready.
    pub fn is_passing(&self) -> bool {
        self.is_broadcast_ready()
    }

    /// Checks if headroom is at or above required threshold in dB.
    pub fn has_headroom(&self, min_headroom_db: f32) -> bool {
        self.headroom_db >= min_headroom_db
    }

    /// Generates human-readable inspection warnings if anomalies are detected.
    pub fn validation_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.clipping {
            warnings.push(format!(
                "Digital clipping detected: peak dBFS is {:.2}",
                self.peak_dbfs
            ));
        }
        if self.peak_dbfs > 0.0 {
            warnings.push(format!(
                "Peak exceeds 0.0 dBFS ceiling: {:.2} dBFS",
                self.peak_dbfs
            ));
        }
        if self.dynamic_range_db < 4.0 {
            warnings.push(format!(
                "Dynamic range is excessively compressed: {:.2} dB",
                self.dynamic_range_db
            ));
        }
        if self.tempo_drift_score >= 0.05 {
            warnings.push(format!(
                "Significant tempo drift detected: drift score {:.4} exceeds 0.05 threshold",
                self.tempo_drift_score
            ));
        }
        if self.rms_dbfs > -6.0 {
            warnings.push(format!(
                "RMS loudness is dangerously hot: {:.2} dBFS",
                self.rms_dbfs
            ));
        }
        if self.phase_alignment_score < 0.70 {
            warnings.push(format!(
                "Phase cancellation warning: phase alignment score is {:.2}",
                self.phase_alignment_score
            ));
        }
        warnings
    }

    /// Diagnostic summary string for logging and UI telemetry readout.
    pub fn summary(&self) -> String {
        format!(
            "Peak: {:.2} dBFS | RMS: {:.2} dBFS | DynRange: {:.2} dB | Drift: {:.4} | Clip: {} | Headroom: {:.2} dB | SNR: {:.1} dB | Phase: {:.2}",
            self.peak_dbfs,
            self.rms_dbfs,
            self.dynamic_range_db,
            self.tempo_drift_score,
            self.clipping,
            self.headroom_db,
            self.snr_db,
            self.phase_alignment_score
        )
    }
}
