//! Unit and integration test suite for the 6 sequential fusion nodes.

use std::fs;
use music_core::state::{
    MusicProductionState, STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER,
    STEM_RASPY_TENOR_VOCALS, STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::tape_mastering::MasteringProfile;
use music_dsp::vocal_formant::VocalArchetype;
use music_dsp::wav::AudioBuffer;
use music_pipeline::nodes::node1_producer_panel::OllamaProducerPanelNode;
use music_pipeline::nodes::node2_groove_yue::GrooveAndYueSynthesisNode;
use music_pipeline::nodes::node3_vocal_timbre::VocalTimbreFusionNode;
use music_pipeline::nodes::node4_stem_inpainting::VirtuosoStemInpaintingNode;
use music_pipeline::nodes::node5_dsp_judge_gate::{
    judge_gate_condition, DspAndJudgeGateNode, ROUTE_MASTERING, ROUTE_RETRY_GROOVE, ROUTE_RETRY_STEM,
};
use music_pipeline::nodes::node6_analog_mastering::AnalogMasteringNode;

#[tokio::test]
async fn test_node1_enforce_structural_bracket_tags_auto_insert() {
    let raw = "Walking through the valley of hope\nLove is the answer to every question";
    let formatted = OllamaProducerPanelNode::enforce_structural_bracket_tags("", raw, "Roots Reggae");
    assert!(formatted.contains("[Intro]"));
    assert!(formatted.contains("[Verse 1]"));
    assert!(formatted.contains("[Chorus]"));
    assert!(formatted.contains("[Outro]"));
}

#[tokio::test]
async fn test_node1_strip_think_tags() {
    let raw = "<think>Analyzing genre: 1977 roots reggae\nNeed one drop</think>[Intro]\n\n[Verse 1]\nOne Love\n\n[Outro]";
    let formatted = OllamaProducerPanelNode::enforce_structural_bracket_tags(raw, "", "Roots Reggae");
    assert!(!formatted.contains("<think>"));
    assert!(!formatted.contains("</think>"));
    assert!(formatted.contains("[Intro]"));
    assert!(formatted.contains("[Verse 1]"));
    assert!(formatted.contains("[Outro]"));
}

#[tokio::test]
async fn test_node1_execution_pure_rust_blueprint_emission() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n1", "Roots Reggae", 76, "A Major");
    state.style_prompt = "1977 roots reggae anthem, spiritual and uplifting".to_string();
    state.engine_mode = "pure_rust".to_string();

    let node1 = OllamaProducerPanelNode::new().with_storage_dir(temp_dir.path());
    let res = node1.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 1 execution failed: {:?}", res);

    assert!(state.blueprint.is_some());
    assert!(state.lyrics.contains("[Intro]"));
    assert!(state.lyrics.contains("[Outro]"));
    assert_eq!(state.bpm, 76);

    let genre_file = temp_dir.path().join("songs/test-song-n1/v1/genre.txt");
    let lyrics_file = temp_dir.path().join("songs/test-song-n1/v1/lyrics.txt");
    assert!(genre_file.exists());
    assert!(lyrics_file.exists());
}

#[tokio::test]
async fn test_node2_draft_groove_and_dual_stems_generation() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n2", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    let node2 = GrooveAndYueSynthesisNode::new()
        .with_duration(2.0)
        .with_storage_dir(temp_dir.path());
    let res = node2.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 2 execution failed: {:?}", res);

    assert!(state.has_stem(STEM_DRAFT_GROOVE));
    assert!(state.has_stem(STEM_YUE_VOCALS));
    assert!(state.has_stem(STEM_YUE_INSTRUMENTALS));

    let groove_p = state.get_stem(STEM_DRAFT_GROOVE).unwrap();
    let vocals_p = state.get_stem(STEM_YUE_VOCALS).unwrap();
    let inst_p = state.get_stem(STEM_YUE_INSTRUMENTALS).unwrap();

    assert!(groove_p.exists());
    assert!(vocals_p.exists());
    assert!(inst_p.exists());

    let buf_groove = AudioBuffer::from_wav_file(groove_p).unwrap();
    assert_eq!(buf_groove.sample_rate, 48_000);
    assert!(buf_groove.duration_secs() >= 1.9);
}

#[tokio::test]
async fn test_node3_vocal_archetype_selection() {
    let s_reggae = MusicProductionState::new("s1", "Roots Reggae", 76, "A Major");
    assert_eq!(VocalTimbreFusionNode::select_vocal_archetype("Roots Reggae", &s_reggae), VocalArchetype::JamaicanTenor);

    let s_pop = MusicProductionState::new("s2", "Pop", 120, "C Major");
    assert_eq!(VocalTimbreFusionNode::select_vocal_archetype("Pop", &s_pop), VocalArchetype::SoulfulRnbSoprano);

    let s_rock = MusicProductionState::new("s3", "Rock", 128, "E Minor");
    assert_eq!(VocalTimbreFusionNode::select_vocal_archetype("Rock", &s_rock), VocalArchetype::GrittyRockBaritone);
}

#[tokio::test]
async fn test_node3_execution_pure_rust_vocal_timbre() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n3", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    // Prepare temp yue_vocals.wav
    let stems_dir = temp_dir.path().join("songs/test-song-n3/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();
    let temp_vocal = stems_dir.join("yue_vocals.wav");
    let test_buf = AudioBuffer::new_stereo(2.0);
    test_buf.to_wav_file(&temp_vocal).unwrap();
    state.set_stem(STEM_YUE_VOCALS, temp_vocal);

    let node3 = VocalTimbreFusionNode::new()
        .with_rasp(0.82)
        .with_storage_dir(temp_dir.path());
    let res = node3.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 3 execution failed: {:?}", res);

    assert!(state.has_stem(STEM_RASPY_TENOR_VOCALS));
    let raspy_p = state.get_stem(STEM_RASPY_TENOR_VOCALS).unwrap();
    assert!(raspy_p.exists());

    let out_buf = AudioBuffer::from_wav_file(raspy_p).unwrap();
    assert_eq!(out_buf.sample_rate, 48_000);
    for s in &out_buf.samples {
        assert!(s.abs() <= 0.98, "Sample clipped: {}", s);
    }
}

#[tokio::test]
async fn test_node4_stem_separation_and_solo_inpainting() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n4", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    // Prepare temp yue_instrumentals.wav
    let stems_dir = temp_dir.path().join("songs/test-song-n4/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();
    let temp_inst = stems_dir.join("yue_instrumentals.wav");
    let test_buf = AudioBuffer::new_stereo(4.0);
    test_buf.to_wav_file(&temp_inst).unwrap();
    state.set_stem(STEM_YUE_INSTRUMENTALS, temp_inst);

    let node4 = VirtuosoStemInpaintingNode::new().with_storage_dir(temp_dir.path());
    let res = node4.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 4 execution failed: {:?}", res);

    assert!(state.has_stem(STEM_DRUMS));
    assert!(state.has_stem(STEM_BASS));
    assert!(state.has_stem(STEM_GUITAR));
    assert!(state.has_stem(STEM_OTHER));
    assert!(state.has_stem(STEM_INPAINTED_GUITAR));
    assert!(state.has_stem(STEM_INPAINTED_DRUMS));

    let solo_buf = AudioBuffer::from_wav_file(state.get_stem(STEM_INPAINTED_GUITAR).unwrap()).unwrap();
    assert_eq!(solo_buf.sample_rate, 48_000);
    assert!(solo_buf.frame_count() > 0);
}

#[tokio::test]
async fn test_node5_dsp_judge_gate_evaluation() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n5", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    // Create realistic non-clipping stems
    let stems_dir = temp_dir.path().join("songs/test-song-n5/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();

    let p_vocal = stems_dir.join("raspy_tenor_vocals.wav");
    let p_guitar = stems_dir.join("inpainted_guitar.wav");
    let p_drums = stems_dir.join("inpainted_drums.wav");

    let buf = AudioBuffer::new_stereo(2.0);
    buf.to_wav_file(&p_vocal).unwrap();
    buf.to_wav_file(&p_guitar).unwrap();
    buf.to_wav_file(&p_drums).unwrap();

    state.set_stem(STEM_RASPY_TENOR_VOCALS, p_vocal);
    state.set_stem(STEM_INPAINTED_GUITAR, p_guitar);
    state.set_stem(STEM_INPAINTED_DRUMS, p_drums);

    let node5 = DspAndJudgeGateNode::default();
    let res = node5.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 5 execution failed: {:?}", res);

    assert!(state.judge_score >= 0.0 && state.judge_score <= 10.0);
    assert!(!state.telemetry.clipping);
}

#[tokio::test]
async fn test_node5_conditional_edge_retry_routing() {
    // 1. Success condition -> routes to mastering
    let mut state_ok = MusicProductionState::default();
    state_ok.quality_passed = true;
    assert_eq!(judge_gate_condition(&state_ok), ROUTE_MASTERING);

    // 2. Retry groove when tempo drift > 0.50
    let mut state_drift = MusicProductionState::default();
    state_drift.quality_passed = false;
    state_drift.retry_count = 0;
    state_drift.telemetry.tempo_drift_score = 0.65;
    assert_eq!(judge_gate_condition(&state_drift), ROUTE_RETRY_GROOVE);

    // 3. Retry stem inpainting when score low or stem issues
    let mut state_stem = MusicProductionState::default();
    state_stem.quality_passed = false;
    state_stem.retry_count = 1;
    state_stem.telemetry.tempo_drift_score = 0.10;
    assert_eq!(judge_gate_condition(&state_stem), ROUTE_RETRY_STEM);

    // 4. Retry exhausted (retry_count >= 2) -> advances to mastering to prevent loop
    let mut state_exhausted = MusicProductionState::default();
    state_exhausted.quality_passed = false;
    state_exhausted.retry_count = 2;
    assert_eq!(judge_gate_condition(&state_exhausted), ROUTE_MASTERING);
}

#[tokio::test]
async fn test_node6_mastering_profiles_and_brickwall_ceiling() {
    assert_eq!(AnalogMasteringNode::resolve_profile("Roots Reggae"), MasteringProfile::StuderA800Tape1970s);
    assert_eq!(AnalogMasteringNode::resolve_profile("Pop"), MasteringProfile::ModernPunchyPop);
    assert_eq!(AnalogMasteringNode::resolve_profile("Amapiano"), MasteringProfile::ClubEdmSubHeavy);
    assert_eq!(AnalogMasteringNode::resolve_profile("Cinematic Orchestral"), MasteringProfile::CinematicOrchestral);
    assert_eq!(AnalogMasteringNode::resolve_profile("Jazz"), MasteringProfile::AudiophileJazz);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut state = MusicProductionState::new("test-song-n6", "Roots Reggae", 76, "A Major");

    let stems_dir = temp_dir.path().join("songs/test-song-n6/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();

    let p_vocal = stems_dir.join("raspy_tenor_vocals.wav");
    let p_guitar = stems_dir.join("inpainted_guitar.wav");
    let p_drums = stems_dir.join("inpainted_drums.wav");

    let buf = AudioBuffer::new_stereo(2.0);
    buf.to_wav_file(&p_vocal).unwrap();
    buf.to_wav_file(&p_guitar).unwrap();
    buf.to_wav_file(&p_drums).unwrap();

    state.set_stem(STEM_RASPY_TENOR_VOCALS, p_vocal);
    state.set_stem(STEM_INPAINTED_GUITAR, p_guitar);
    state.set_stem(STEM_INPAINTED_DRUMS, p_drums);

    let node6 = AnalogMasteringNode::new().with_storage_dir(temp_dir.path());
    let res = node6.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 6 execution failed: {:?}", res);

    assert!(state.has_stem(STEM_FINAL_MASTER));
    let master_path = state.get_stem(STEM_FINAL_MASTER).unwrap();
    assert!(master_path.exists());

    let master_buf = AudioBuffer::from_wav_file(master_path).unwrap();
    assert_eq!(master_buf.sample_rate, 48_000);
    assert_eq!(master_buf.channels, 2);

    // True peak limiter assertion: <= 0.966 (-0.3 dBFS)
    for s in &master_buf.samples {
        assert!(s.abs() <= 0.966, "True peak limiter ceiling violated: {}", s);
    }

    let manifest_path = temp_dir.path().join("songs/test-song-n6/v1/manifest.json");
    assert!(manifest_path.exists());
}
