//! Full End-to-End Pipeline Execution Tests for Milestone 3.

use music_core::state::{
    MusicProductionState, STEM_DRAFT_GROOVE, STEM_FINAL_MASTER, STEM_INPAINTED_DRUMS,
    STEM_INPAINTED_GUITAR, STEM_RASPY_TENOR_VOCALS, STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::wav::AudioBuffer;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

#[tokio::test]
async fn test_full_6_node_pipeline_execution_roots_reggae() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(4.0);

    let runner = PipelineRunner::new(config).expect("Runner initialization");

    let mut state = MusicProductionState::new(
        "reggae-flagship-e2e",
        "Roots Reggae",
        76,
        "A Major",
    );
    state.style_prompt = "1977 roots reggae anthem, spiritual and uplifting, 76 BPM".to_string();
    state.engine_mode = "pure_rust".to_string();

    let final_state = runner.execute_song(state).await.expect("execute_song");

    // 1. Verify Blueprint and Lyrics
    assert!(final_state.blueprint.is_some());
    assert!(final_state.lyrics.contains("[Intro]"));
    assert!(final_state.lyrics.contains("[Outro]"));
    assert_eq!(final_state.bpm, 76);

    // 2. Verify all primary stems exist and are registered
    let required_stems = [
        STEM_DRAFT_GROOVE,
        STEM_YUE_VOCALS,
        STEM_YUE_INSTRUMENTALS,
        STEM_RASPY_TENOR_VOCALS,
        STEM_INPAINTED_GUITAR,
        STEM_INPAINTED_DRUMS,
        STEM_FINAL_MASTER,
    ];

    for stem in &required_stems {
        assert!(final_state.has_stem(stem), "Missing stem registration: {}", stem);
        let path = final_state.get_stem(stem).unwrap();
        assert!(path.exists(), "Stem file does not exist on disk: {:?}", path);
    }

    // 3. Verify Final Master audio characteristics
    let master_path = final_state.get_stem(STEM_FINAL_MASTER).unwrap();
    let master_buf = AudioBuffer::from_wav_file(master_path).expect("Read final_master.wav");

    assert_eq!(master_buf.sample_rate, 48_000);
    assert_eq!(master_buf.channels, 2);
    assert!(master_buf.duration_secs() > 1.0);

    // Assert Studer A800 true peak limiter ceiling: true peak <= 0.966 (-0.3 dBFS)
    for s in &master_buf.samples {
        assert!(s.abs() <= 0.966, "Master clipped above -0.3 dBFS ceiling: {}", s);
    }

    // 4. Verify checkpointer and SQLite persistence
    let versions = runner.checkpointer().list_versions("reggae-flagship-e2e").expect("list_versions");
    assert_eq!(versions, vec![1]);

    let loaded = runner.checkpointer().load_checkpoint("reggae-flagship-e2e", 1).await.expect("load v1");
    assert_eq!(loaded.song_id, "reggae-flagship-e2e");
    assert!(loaded.has_stem(STEM_FINAL_MASTER));
}

#[tokio::test]
async fn test_full_6_node_pipeline_execution_afrobeats() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(4.0);

    let runner = PipelineRunner::new(config).expect("Runner initialization");

    let mut state = MusicProductionState::new(
        "afro-e2e",
        "Afrobeats",
        108,
        "F# Minor",
    );
    state.style_prompt = "Energetic Afrobeats with Amapiano log drums and syncopated clave".to_string();
    state.engine_mode = "pure_rust".to_string();

    let final_state = runner.execute_song(state).await.expect("execute_song");

    assert!(final_state.has_stem(STEM_FINAL_MASTER));
    let master_path = final_state.get_stem(STEM_FINAL_MASTER).unwrap();
    let master_buf = AudioBuffer::from_wav_file(master_path).expect("Read final_master.wav");
    assert_eq!(master_buf.sample_rate, 48_000);
    assert!(!final_state.telemetry.clipping);
}

#[tokio::test]
async fn test_targeted_stem_regeneration() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(4.0);

    let runner = PipelineRunner::new(config).expect("Runner initialization");

    let mut state = MusicProductionState::new("regen-test", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    let _v1 = runner.execute_song(state).await.expect("initial execute");

    // Regenerate stem (advances version from v1 to v2)
    let v2 = runner.regenerate_stem("regen-test", 1, "guitar").await.expect("regenerate_stem");
    assert_eq!(v2.version, 2);
    assert!(v2.has_stem(STEM_INPAINTED_GUITAR));
    assert!(v2.has_stem(STEM_FINAL_MASTER));

    let versions = runner.checkpointer().list_versions("regen-test").expect("list_versions");
    assert_eq!(versions, vec![1, 2]);
}

#[tokio::test]
async fn test_targeted_master_track() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(4.0);

    let runner = PipelineRunner::new(config).expect("Runner initialization");

    let mut state = MusicProductionState::new("master-test", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    let _v1 = runner.execute_song(state).await.expect("initial execute");

    // Master track only
    let mastered = runner.master_track("master-test", 1).await.expect("master_track");
    assert!(mastered.has_stem(STEM_FINAL_MASTER));
}

#[tokio::test]
async fn test_song_iteration_branching() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(4.0);

    let runner = PipelineRunner::new(config).expect("Runner initialization");

    let mut state = MusicProductionState::new("iter-test", "Roots Reggae", 76, "A Major");
    state.engine_mode = "pure_rust".to_string();

    let _v1 = runner.execute_song(state).await.expect("initial execute");

    // Iterate to v2 with prompt update
    let v2 = runner.iterate_song(
        "iter-test",
        1,
        Some("Deeper dub space echo and heavier sub-bass"),
    ).await.expect("iterate_song");

    assert_eq!(v2.version, 2);
    assert_eq!(v2.style_prompt, "Deeper dub space echo and heavier sub-bass");
    assert!(v2.has_stem(STEM_FINAL_MASTER));

    let versions = runner.checkpointer().list_versions("iter-test").expect("list_versions");
    assert_eq!(versions, vec![1, 2]);
}
