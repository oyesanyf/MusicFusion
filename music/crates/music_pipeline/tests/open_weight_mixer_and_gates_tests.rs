use std::f32::consts::PI;
use std::fs;
use music_core::schema::StemMixConfig;
use music_core::state::{
    LyricsMode, MusicProductionState, STEM_AMBIENCE_FX, STEM_BACKING_HARMONIES, STEM_BASS,
    STEM_DRUMS, STEM_FINAL_MASTER, STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR,
    STEM_KEYS_SYNTHS, STEM_OTHER, STEM_PERCUSSION_FILLS, STEM_RASPY_TENOR_VOCALS,
    STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::wav::AudioBuffer;
use music_pipeline::nodes::node1_producer_panel::OllamaProducerPanelNode;
use music_pipeline::nodes::node3_vocal_timbre::VocalTimbreFusionNode;
use music_pipeline::nodes::node4_stem_inpainting::VirtuosoStemInpaintingNode;
use music_pipeline::nodes::node5_dsp_judge_gate::DspAndJudgeGateNode;
use music_pipeline::nodes::node6_analog_mastering::AnalogMasteringNode;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

fn make_sine(freq_hz: f32, amp: f32, dur_secs: f32) -> AudioBuffer {
    let sr = 48000u32;
    let frames = (sr as f32 * dur_secs) as usize;
    let mut buf = AudioBuffer::new(sr, 2, dur_secs);
    for f in 0..frames {
        let t = f as f32 / sr as f32;
        let v = (2.0 * PI * freq_hz * t).sin() * amp;
        buf.samples[f * 2] = v;
        buf.samples[f * 2 + 1] = v;
    }
    buf
}

fn rms_stereo(buf: &AudioBuffer) -> (f32, f32) {
    let frames = buf.frame_count().max(1);
    let ch = buf.channels.max(1) as usize;
    let mut sum_l = 0.0f64;
    let mut sum_r = 0.0f64;
    for f in 0..frames {
        let l = buf.samples[f * ch] as f64;
        let r = buf.samples[f * ch + (1 % ch)] as f64;
        sum_l += l * l;
        sum_r += r * r;
    }
    (
        (sum_l / frames as f64).sqrt() as f32,
        (sum_r / frames as f64).sqrt() as f32,
    )
}

#[tokio::test]
async fn test_node1_user_lyrics_preserved_verbatim_and_gates_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let mut state = MusicProductionState::new("song-verbatim-1", "Indie Electronic", 118, "D Minor");
    state.engine_mode = "pure_rust".to_string();
    state.lyrics_mode = LyricsMode::UserProvided;
    state.lyrics = "Midnight rain on copper wire\nNeon reflections never tire".to_string();
    state.style_prompt = "Lush indie electronic track with shimmering synths".to_string();
    state.selected_model = Some("YuE2".to_string());
    state.commercial_only = true;

    let node1 = OllamaProducerPanelNode::new().with_storage_dir(tmp.path());
    node1.execute_node(&mut state).await.expect("Node 1 failed");

    assert_eq!(state.lyrics.matches("Midnight rain on copper wire").count(), 1);
    assert_eq!(state.lyrics.matches("Neon reflections never tire").count(), 1);
    assert!(!state.lyrics.contains("Kingston"));
    assert!(!state.lyrics.contains("One love"));
    assert_ne!(state.selected_model.as_deref(), Some("YuE2"), "Commercial-only guard must override CC-BY-NC YuE2");

    let gate_names: Vec<&str> = state.decision_traces.iter().map(|t| t.gate_name.as_str()).collect();
    assert!(gate_names.contains(&"gate1_model_selection"));
    assert!(gate_names.contains(&"gate2_lyrics_routing"));
    assert!(gate_names.contains(&"gate5_license_export"));
}

#[tokio::test]
async fn test_node1_ai_write_lyrics_uses_topic_and_genre() {
    let tmp = tempfile::tempdir().unwrap();
    let mut state = MusicProductionState::new("song-ai-lyrics-1", "Synthwave", 108, "C Minor");
    state.engine_mode = "pure_rust".to_string();
    state.lyrics_mode = LyricsMode::AiGenerated;
    state.lyrics_topic = "Starlight voyage across Saturn".to_string();
    state.style_prompt = "Dreamy analog synthwave ballad".to_string();
    state.lyrics = String::new();

    let node1 = OllamaProducerPanelNode::new().with_storage_dir(tmp.path());
    node1.execute_node(&mut state).await.expect("Node 1 failed");

    assert!(state.lyrics.contains("[Verse 1]"));
    assert!(state.lyrics.contains("[Chorus]"));
    assert!(!state.lyrics.contains("Trenchtown"));
    assert!(!state.lyrics.contains("Kingston"));
}

#[tokio::test]
async fn test_node3_preserves_neural_vocals_and_ignores_legacy_vocal_ref_wav() {
    let tmp = tempfile::tempdir().unwrap();
    let stems_dir = tmp.path().join("songs/song-n3-clean/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();

    let src_vocal = stems_dir.join("yue_vocals.wav");
    let expected_buf = make_sine(440.0, 0.45, 0.5);
    expected_buf.to_wav_file(&src_vocal).unwrap();

    let forbidden_ref = stems_dir.join("legacy_ref.wav");
    let ref_buf = make_sine(1980.0, 0.90, 0.5);
    ref_buf.to_wav_file(&forbidden_ref).unwrap();

    let mut state = MusicProductionState::new("song-n3-clean", "Pop", 120, "C Major");
    state.rendered_by_neural = true;
    state.vocal_ref_wav = Some(forbidden_ref);
    state.set_stem(STEM_YUE_VOCALS, src_vocal);

    let node3 = VocalTimbreFusionNode::new().with_storage_dir(tmp.path());
    node3.execute_node(&mut state).await.expect("Node 3 failed");

    let out_path = state.get_stem(STEM_RASPY_TENOR_VOCALS).expect("vocal output registered");
    let out_buf = AudioBuffer::from_wav_file(out_path).unwrap();
    let max_diff = out_buf
        .samples
        .iter()
        .zip(expected_buf.samples.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(max_diff < 0.002, "Neural vocal stem must be preserved without legacy ref contamination (diff={})", max_diff);
}

#[tokio::test]
async fn test_node4_preserves_neural_stems_and_populates_8ch_daw_stems() {
    let tmp = tempfile::tempdir().unwrap();
    let stems_dir = tmp.path().join("songs/song-n4-neural/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();

    let d_buf = make_sine(120.0, 0.35, 0.5);
    let b_buf = make_sine(75.0, 0.40, 0.5);
    let g_buf = make_sine(550.0, 0.30, 0.5);
    let o_buf = make_sine(880.0, 0.25, 0.5);
    let v_buf = make_sine(330.0, 0.40, 0.5);

    let p_d = stems_dir.join("drums.wav");
    let p_b = stems_dir.join("bass.wav");
    let p_g = stems_dir.join("guitar.wav");
    let p_o = stems_dir.join("other.wav");
    let p_v = stems_dir.join("raspy_tenor_vocals.wav");

    d_buf.to_wav_file(&p_d).unwrap();
    b_buf.to_wav_file(&p_b).unwrap();
    g_buf.to_wav_file(&p_g).unwrap();
    o_buf.to_wav_file(&p_o).unwrap();
    v_buf.to_wav_file(&p_v).unwrap();

    let mut state = MusicProductionState::new("song-n4-neural", "Orchestral", 96, "G Major");
    state.engine_mode = "pure_rust".to_string();
    state.rendered_by_neural = true;
    state.set_stem(STEM_DRUMS, p_d);
    state.set_stem(STEM_BASS, p_b.clone());
    state.set_stem(STEM_GUITAR, p_g);
    state.set_stem(STEM_OTHER, p_o);
    state.set_stem(STEM_RASPY_TENOR_VOCALS, p_v);
    state.set_stem(STEM_YUE_INSTRUMENTALS, stems_dir.join("other.wav"));

    let node4 = VirtuosoStemInpaintingNode::new().with_storage_dir(tmp.path());
    node4.execute_node(&mut state).await.expect("Node 4 failed");

    for key in [
        STEM_DRUMS,
        STEM_BASS,
        STEM_GUITAR,
        STEM_OTHER,
        STEM_INPAINTED_DRUMS,
        STEM_INPAINTED_GUITAR,
        STEM_BACKING_HARMONIES,
        STEM_KEYS_SYNTHS,
        STEM_PERCUSSION_FILLS,
        STEM_AMBIENCE_FX,
    ] {
        let p = state.get_stem(key).unwrap_or_else(|| panic!("Missing stem {}", key));
        assert!(p.is_file(), "Stem '{}' missing on disk: {:?}", key, p);
    }

    let preserved_bass = AudioBuffer::from_wav_file(state.get_stem(STEM_BASS).unwrap()).unwrap();
    let max_diff = preserved_bass
        .samples
        .iter()
        .zip(b_buf.samples.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(max_diff < 0.002, "Neural bass stem must NOT be overwritten with Reggae DubSubBass");
}

#[tokio::test]
async fn test_daw_mixer_solo_mute_pan_3band_eq_and_non_destructive_bounce() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = PipelineConfig::new()
        .with_storage_dir(tmp.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.5);
    let runner = PipelineRunner::new(cfg).unwrap();

    let mut state = MusicProductionState::new("song-mixer-test", "Jazz", 100, "C Major");
    state.engine_mode = "pure_rust".to_string();
    state.rendered_by_neural = true;

    let stems_dir = tmp.path().join("songs/song-mixer-test/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();
    let p_voc = stems_dir.join("raspy_tenor_vocals.wav");
    let p_gtr = stems_dir.join("inpainted_guitar.wav");
    let p_bas = stems_dir.join("bass.wav");
    make_sine(440.0, 0.5, 0.5).to_wav_file(&p_voc).unwrap();
    make_sine(880.0, 0.5, 0.5).to_wav_file(&p_gtr).unwrap();
    make_sine(80.0, 0.5, 0.5).to_wav_file(&p_bas).unwrap();

    state.set_stem(STEM_RASPY_TENOR_VOCALS, p_voc);
    state.set_stem(STEM_INPAINTED_GUITAR, p_gtr);
    state.set_stem(STEM_BASS, p_bas);

    let node6 = AnalogMasteringNode::new().with_storage_dir(tmp.path());

    // 1. Mute vocals & bass, Pan guitar hard left -> L RMS must vastly exceed R RMS
    let mut pan_mix = StemMixConfig::default();
    if let Some(c) = pan_mix.channels.get_mut("vocals") {
        c.mute = true;
    }
    if let Some(c) = pan_mix.channels.get_mut("bass") {
        c.mute = true;
    }
    if let Some(c) = pan_mix.channels.get_mut("guitar") {
        c.pan = -1.0;
    }
    let mut bp = music_core::schema::SongBlueprint::default_one_love_shining_anthem();
    bp.stem_mix = pan_mix.clone();
    state.blueprint = Some(bp);

    let premaster_left = node6.mix_stems_to_premaster(&state);
    let (l_rms, r_rms) = rms_stereo(&premaster_left);
    assert!(l_rms > 0.05 && r_rms < 0.005, "Hard-left pan must isolate signal to Left channel: L={}, R={}", l_rms, r_rms);

    // 2. Solo bass only with +10 dB low-shelf boost vs 0 dB low-shelf
    let mut solo_flat = StemMixConfig::default();
    if let Some(c) = solo_flat.channels.get_mut("bass") {
        c.solo = true;
        c.eq_low_db = 0.0;
    }
    state.blueprint.as_mut().unwrap().stem_mix = solo_flat;
    let flat_bass = node6.mix_stems_to_premaster(&state);
    let (flat_l, _) = rms_stereo(&flat_bass);

    let mut solo_boost = StemMixConfig::default();
    if let Some(c) = solo_boost.channels.get_mut("bass") {
        c.solo = true;
        c.eq_low_db = 10.0;
    }
    state.blueprint.as_mut().unwrap().stem_mix = solo_boost.clone();
    let boosted_bass = node6.mix_stems_to_premaster(&state);
    let (boost_l, _) = rms_stereo(&boosted_bass);
    assert!(boost_l > flat_l * 1.8, "Low-shelf +10 dB EQ must boost 80 Hz bass energy: flat={}, boost={}", flat_l, boost_l);

    // 3. Execute Node 6 for v1 and save checkpoint, then bounce v1 -> v2 via mix_and_master_song
    node6.execute_node(&mut state).await.expect("Node 6 v1 failed");
    let v1_master = tmp.path().join("songs/song-mixer-test/v1/final_master.wav");
    assert!(v1_master.is_file());
    let v1_bytes = fs::read(&v1_master).unwrap();
    runner.checkpointer().save_checkpoint(&state, "analog_mastering_node").await.unwrap();

    let mut bounce_mix = StemMixConfig::default();
    if let Some(c) = bounce_mix.channels.get_mut("guitar") {
        c.solo = true;
        c.reverb_send = 0.65;
    }
    let v2_state = runner
        .mix_and_master_song("song-mixer-test", 1, bounce_mix)
        .await
        .expect("mix_and_master_song failed");

    assert_eq!(v2_state.version, 2);
    assert_eq!(v2_state.blueprint.as_ref().unwrap().parent_version, Some(1));
    let v2_master = v2_state.get_stem(STEM_FINAL_MASTER).expect("v2 master stem");
    assert!(v2_master.is_file());
    assert_eq!(fs::read(&v1_master).unwrap(), v1_bytes, "v1 final_master.wav must never be overwritten by v2 bounce");
}

#[tokio::test]
async fn test_node5_records_gate3_and_node6_preserves_20khz_on_neural() {
    let tmp = tempfile::tempdir().unwrap();
    let stems_dir = tmp.path().join("songs/song-bw-test/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();

    // 16 kHz high-frequency shimmer signal
    let hi_path = stems_dir.join("inpainted_guitar.wav");
    make_sine(16000.0, 0.5, 0.25).to_wav_file(&hi_path).unwrap();

    let mut state = MusicProductionState::new("song-bw-test", "Pop", 120, "A Major");
    state.engine_mode = "pure_rust".to_string();
    state.rendered_by_neural = true;
    state.set_stem(STEM_INPAINTED_GUITAR, hi_path);

    let node5 = DspAndJudgeGateNode::default();
    node5.execute_node(&mut state).await.expect("Node 5 failed");
    assert!(
        state.decision_traces.iter().any(|t| t.gate_name == "gate3_telemetry_adjudication"),
        "Node 5 must record gate3 decision trace"
    );

    let node6 = AnalogMasteringNode::new().with_storage_dir(tmp.path());
    node6.execute_node(&mut state).await.expect("Node 6 neural failed");
    let neural_out = AudioBuffer::from_wav_file(state.get_stem(STEM_FINAL_MASTER).unwrap()).unwrap();
    let (n_rms, _) = rms_stereo(&neural_out);

    state.version = 2;
    state.rendered_by_neural = false;
    node6.execute_node(&mut state).await.expect("Node 6 dsp failed");
    let dsp_out = AudioBuffer::from_wav_file(state.get_stem(STEM_FINAL_MASTER).unwrap()).unwrap();
    let (d_rms, _) = rms_stereo(&dsp_out);

    assert!(n_rms > d_rms, "Neural 20kHz path ({}) must preserve >12kHz energy vs DSP 12kHz LPF ({})", n_rms, d_rms);
}
