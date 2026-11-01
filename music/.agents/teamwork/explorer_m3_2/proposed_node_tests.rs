//! crates/music_pipeline/tests/node_tests.rs
//!
//! Unit and integration test suite for Nodes 1, 2, and 3.

use std::fs;
use std::path::PathBuf;
use music_core::schema::SongBlueprint;
use music_core::state::{
    MusicProductionState, STEM_DRAFT_GROOVE, STEM_RASPY_TENOR_VOCALS, STEM_YUE_INSTRUMENTALS,
    STEM_YUE_VOCALS,
};
use music_dsp::vocal_formant::VocalArchetype;
use music_dsp::wav::AudioBuffer;
use music_pipeline::nodes::node1_producer_panel::OllamaProducerPanelNode;
use music_pipeline::nodes::node2_groove_yue::GrooveAndYueSynthesisNode;
use music_pipeline::nodes::node3_vocal_timbre::VocalTimbreFusionNode;

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
    let mut state = MusicProductionState::new("test-song-n1", "Roots Reggae", 76, "A Major");
    state.style_prompt = "1977 roots reggae anthem, spiritual and uplifting".to_string();

    let node1 = OllamaProducerPanelNode::new();
    let res = node1.execute_node(&mut state).await;
    assert!(res.is_ok(), "Node 1 execution failed: {:?}", res);

    assert!(state.blueprint.is_some());
    assert!(state.lyrics.contains("[Intro]"));
    assert!(state.lyrics.contains("[Outro]"));
    assert_eq!(state.bpm, 76);

    let genre_file = PathBuf::from("storage/songs/test-song-n1/v1/genre.txt");
    let lyrics_file = PathBuf::from("storage/songs/test-song-n1/v1/lyrics.txt");
    assert!(genre_file.exists());
    assert!(lyrics_file.exists());

    // Teardown
    let _ = fs::remove_dir_all("storage/songs/test-song-n1");
}

#[tokio::test]
async fn test_node2_draft_groove_and_dual_stems_generation() {
    let mut state = MusicProductionState::new("test-song-n2", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    let node2 = GrooveAndYueSynthesisNode::new().with_duration(2.0);
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

    // Teardown
    let _ = fs::remove_dir_all("storage/songs/test-song-n2");
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
    let mut state = MusicProductionState::new("test-song-n3", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    // Prepare temp yue_vocals.wav
    let stems_dir = PathBuf::from("storage/songs/test-song-n3/v1/stems");
    fs::create_dir_all(&stems_dir).unwrap();
    let temp_vocal = stems_dir.join("yue_vocals.wav");
    let test_buf = AudioBuffer::new_stereo(2.0);
    test_buf.to_wav_file(&temp_vocal).unwrap();
    state.set_stem(STEM_YUE_VOCALS, temp_vocal);

    let node3 = VocalTimbreFusionNode::new().with_rasp(0.82);
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

    // Teardown
    let _ = fs::remove_dir_all("storage/songs/test-song-n3");
}
