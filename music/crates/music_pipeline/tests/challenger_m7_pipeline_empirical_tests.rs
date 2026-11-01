//! crates/music_pipeline/tests/challenger_m7_pipeline_empirical_tests.rs
//!
//! Empirical Challenger Tests for Milestone 7 Pipeline, Headroom & Anti-Clipping Invariants:
//! 1. Verify 0.20 stem scaling prevents summing bus clipping under simultaneous maximum signal peaks.
//! 2. Verify Analog Mastering node applies 12 kHz Butterworth LPF, tanh saturation, and -1.0 dBFS (0.89125) peak ceiling.
//! 3. Verify all 7 instruments are active, non-empty, non-silent, and mixed into final master.

use std::fs;
use music_core::state::{
    MusicProductionState, STEM_BASS, STEM_FINAL_MASTER, STEM_INPAINTED_DRUMS,
    STEM_INPAINTED_GUITAR, STEM_OTHER, STEM_RASPY_TENOR_VOCALS,
};
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_pipeline::nodes::node6_analog_mastering::AnalogMasteringNode;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

const SAMPLE_RATE: u32 = 48_000;
const PEAK_CEILING_PCM16: f32 = 0.89125; // -1.0 dBFS

#[tokio::test]
async fn test_empirical_stem_scaling_summing_bus_clipping_immunity() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let song_id = "test-summing-headroom";
    let version = 1;
    let stems_dir = temp_dir.path().join("songs").join(song_id).join(format!("v{}", version)).join("stems");
    fs::create_dir_all(&stems_dir).expect("create stems dir");

    let mut state = MusicProductionState::new(song_id, "Roots Reggae", 76, "A Major");
    state.version = version;

    // Create 5 full-scale (peak = 1.0) synthetic stem WAV files
    let duration_secs = 1.0f32;
    let stem_names = [
        STEM_INPAINTED_GUITAR,
        STEM_INPAINTED_DRUMS,
        STEM_BASS,
        STEM_OTHER,
        STEM_RASPY_TENOR_VOCALS,
    ];

    for stem_name in &stem_names {
        let mut buf = AudioBuffer::new_stereo(duration_secs);
        for s in &mut buf.samples {
            *s = 1.0; // worst-case maximum square peak
        }
        let p = stems_dir.join(format!("{}.wav", stem_name));
        buf.to_wav_file_with_format(&p, WavBitDepth::Pcm16).expect("save stem");
        state.set_stem(*stem_name, p);
    }

    let mastering_node = AnalogMasteringNode::new().with_storage_dir(temp_dir.path());
    let premaster = mastering_node.mix_stems_to_premaster(&state);

    // Assert that scaling each stem by 0.20 strictly bounds the summed peak <= 1.0 (0 dBFS)
    let mut premaster_peak = 0.0f32;
    for &s in &premaster.samples {
        let a = s.abs();
        if a > premaster_peak { premaster_peak = a; }
    }

    assert!(
        premaster_peak <= 1.0 + 1e-5,
        "Summing bus clipped! Premaster peak {} > 1.0 under 0.20 stem scaling",
        premaster_peak
    );
}

#[tokio::test]
async fn test_empirical_analog_mastering_node_headroom_and_peak_normalization() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let song_id = "test-analog-mastering-headroom";
    let version = 1;
    let stems_dir = temp_dir.path().join("songs").join(song_id).join(format!("v{}", version)).join("stems");
    fs::create_dir_all(&stems_dir).expect("create stems dir");

    let mut state = MusicProductionState::new(song_id, "Roots Reggae", 76, "A Major");
    state.version = version;

    // Generate hot stems that would normally cause severe digital clipping without headroom & limiting
    let duration_secs = 2.0f32;
    let stem_names = [
        STEM_INPAINTED_GUITAR,
        STEM_INPAINTED_DRUMS,
        STEM_BASS,
        STEM_OTHER,
        STEM_RASPY_TENOR_VOCALS,
    ];

    for (idx, stem_name) in stem_names.iter().enumerate() {
        let mut buf = AudioBuffer::new_stereo(duration_secs);
        let freq = 100.0 * (idx + 1) as f32;
        for (i, s) in buf.samples.iter_mut().enumerate() {
            let t = (i / 2) as f32 / SAMPLE_RATE as f32;
            *s = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.95;
        }
        let p = stems_dir.join(format!("{}.wav", stem_name));
        buf.to_wav_file_with_format(&p, WavBitDepth::Pcm16).expect("save stem");
        state.set_stem(*stem_name, p);
    }

    let mastering_node = AnalogMasteringNode::new().with_storage_dir(temp_dir.path());
    mastering_node.execute_node(&mut state).await.expect("mastering execute");

    let master_path = state.get_stem(STEM_FINAL_MASTER).expect("master path exists");
    let master_buf = AudioBuffer::from_wav_file(master_path).expect("read final_master.wav");

    assert_eq!(master_buf.sample_rate, SAMPLE_RATE);
    assert_eq!(master_buf.channels, 2);

    let mut true_peak = 0.0f32;
    for &s in &master_buf.samples {
        assert!(s.is_finite());
        let a = s.abs();
        if a > true_peak { true_peak = a; }
    }

    // Must be strictly <= -1.0 dBFS (0.89125)
    assert!(
        true_peak <= PEAK_CEILING_PCM16 + 1e-4,
        "Master peak {} exceeded -1.0 dBFS ceiling ({})",
        true_peak, PEAK_CEILING_PCM16
    );

    // Verify telemetry
    let telemetry = &state.telemetry;
    assert!(!telemetry.clipping, "Mastering telemetry flagged clipping");
    assert!(
        telemetry.peak_dbfs <= -0.99,
        "Mastering telemetry peak_dbfs ({}) > -1.0 dBFS",
        telemetry.peak_dbfs
    );
}

#[tokio::test]
async fn test_empirical_full_pipeline_roots_reggae_anthem_all_instruments_and_headroom() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(3.0);

    let runner = PipelineRunner::new(config).expect("Runner init");
    let mut state = MusicProductionState::new("adv-roots-reggae-anthem", "Roots Reggae", 76, "A Major");
    state.style_prompt = "Roots reggae anthem with physical modeling, one-drop kick, dub bass, skank guitar, Hammond organ, and soaring solo".to_string();
    state.engine_mode = "pure_rust".to_string();

    let final_state = runner.execute_song(state).await.expect("Pipeline run failed");

    // Verify final master
    let master_path = final_state.get_stem(STEM_FINAL_MASTER).expect("final_master.wav must exist");
    assert!(master_path.exists(), "final_master.wav file does not exist on disk");

    let master_buf = AudioBuffer::from_wav_file(master_path).expect("read final_master");
    assert_eq!(master_buf.sample_rate, SAMPLE_RATE);
    assert_eq!(master_buf.channels, 2);

    let mut max_abs = 0.0f32;
    for &s in &master_buf.samples {
        assert!(s.is_finite());
        let a = s.abs();
        if a > max_abs { max_abs = a; }
    }

    // Headroom invariant: true-peak strictly <= 0.89125 (-1.0 dBFS)
    assert!(
        max_abs <= PEAK_CEILING_PCM16 + 1e-4,
        "Full pipeline master peak {} exceeded -1.0 dBFS ({})",
        max_abs, PEAK_CEILING_PCM16
    );
    assert!(max_abs > 0.05, "Master is unexpectedly silent: peak = {}", max_abs);

    // Telemetry invariant: zero clipping (> 0 dBFS)
    let telem = &final_state.telemetry;
    assert!(!telem.clipping, "Clipping detected on final master");
    assert!(telem.peak_dbfs <= -0.99, "Peak dBFS > -1.0 dBFS");

    // Invariant: ALL instrument stems must be present and non-silent
    let expected_stems = [
        "draft_groove",
        "yue_instrumentals",
        "yue_vocals",
        "drums",
        "bass",
        "guitar",
        "other",
        "inpainted_guitar",
        "inpainted_drums",
    ];

    for stem_name in &expected_stems {
        let p = final_state.get_stem(stem_name)
            .unwrap_or_else(|| panic!("Stem {} must be registered in state", stem_name));
        assert!(p.exists(), "Stem file {:?} does not exist", p);

        let s_buf = AudioBuffer::from_wav_file(p)
            .unwrap_or_else(|e| panic!("Failed to read stem {:?}: {}", p, e));

        let mut s_peak = 0.0f32;
        for &s in &s_buf.samples {
            if s.abs() > s_peak { s_peak = s.abs(); }
        }
        assert!(
            s_peak > 0.001,
            "Stem {} is completely silent! Peak = {}",
            stem_name, s_peak
        );
        assert!(
            s_peak <= 1.0,
            "Stem {} clipped! Peak = {}",
            stem_name, s_peak
        );
    }
}
