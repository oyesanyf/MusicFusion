use std::path::PathBuf;
use music_core::{
    DecisionMode, EngineMode, GenreCategory, GenreTaxonomy, LyricsMode, ModalScale,
    MusicProductionState, RhythmGroove, SongBlueprint, StemChannelStrip, StemMixConfig,
    StemTelemetry, TypedDecisionTrace, STEM_DRUMS, STEM_FINAL_MASTER,
};

#[test]
fn test_serde_music_production_state_roundtrip() {
    let mut state = MusicProductionState::new("test-song-01", "Afrobeats", 105, "F# Minor");
    state.set_stem(STEM_DRUMS, PathBuf::from("storage/drums.wav"));
    state.set_stem(STEM_FINAL_MASTER, PathBuf::from("storage/final_master.wav"));
    state.lyrics = "[intro]\nFeel the groove\n[verse]\nUnder the sun".to_string();
    state.style_prompt = "Modern Afrobeats, 105 BPM, warm 808 bass".to_string();
    state.judge_score = 8.5;
    state.quality_passed = true;
    state.retry_count = 1;

    let json = state.to_json().expect("Serialization to JSON failed");
    assert!(json.contains("test-song-01"));
    assert!(json.contains("Afrobeats"));

    let restored = MusicProductionState::from_json(&json).expect("Deserialization failed");
    assert_eq!(restored.song_id, "test-song-01");
    assert_eq!(restored.bpm, 105);
    assert_eq!(restored.key_signature, "F# Minor");
    assert_eq!(restored.judge_score, 8.5);
    assert!(restored.quality_passed);
    assert_eq!(restored.retry_count, 1);
    assert!(restored.has_stem(STEM_DRUMS));
    assert!(restored.has_stem(STEM_FINAL_MASTER));
}

#[test]
fn test_default_reggae_anthem_state() {
    let state = MusicProductionState::default_reggae_anthem();
    assert_eq!(state.song_id, "song-one-love-shining-1977");
    assert_eq!(state.bpm, 76);
    assert_eq!(state.key_signature, "A Major");
    assert_eq!(state.genre, "Roots Reggae");
    assert_eq!(state.engine_mode, EngineMode::Auto.as_str());
    assert_eq!(state.retry_count, 0);
    assert!(!state.lyrics.is_empty());
    assert!(state.style_prompt.contains("1977 roots reggae anthem"));
    assert!(state.blueprint.is_some());
}

#[test]
fn test_stem_mutation_and_versioning() {
    let mut state = MusicProductionState::new("v-song", "Rock", 120, "E Minor");
    assert_eq!(state.version, 1);
    state.advance_version();
    assert_eq!(state.version, 2);

    state.set_stem("guitar", "storage/guitar.wav");
    assert!(state.has_stem("guitar"));
    assert_eq!(
        state.get_stem("guitar"),
        Some(&PathBuf::from("storage/guitar.wav"))
    );

    assert_eq!(state.increment_retry(), 1);
    assert!(state.can_retry(2));
    assert_eq!(state.increment_retry(), 2);
    assert!(!state.can_retry(2));
}

#[test]
fn test_telemetry_broadcast_readiness() {
    let clean = StemTelemetry::default();
    assert!(clean.is_broadcast_ready());
    assert!(clean.validation_warnings().is_empty());

    let mut clipped = StemTelemetry::default();
    clipped.clipping = true;
    clipped.peak_dbfs = 0.5;
    assert!(!clipped.is_broadcast_ready());
    let warnings = clipped.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("clipping")));

    let mut drifting = StemTelemetry::default();
    drifting.tempo_drift_score = 0.08;
    assert!(!drifting.is_broadcast_ready());
    assert!(drifting.validation_warnings().iter().any(|w| w.contains("tempo drift")));
}

#[test]
fn test_genre_taxonomy_coverage() {
    let presets = GenreTaxonomy::all_presets();
    assert!(presets.len() >= 20);

    // Verify key categories represented
    assert!(presets.iter().any(|p| p.category == GenreCategory::RootsReggaeDub));
    assert!(presets.iter().any(|p| p.category == GenreCategory::AfrobeatHighlifeAmapiano));
    assert!(presets.iter().any(|p| p.category == GenreCategory::RnbSoulGospel));
    assert!(presets.iter().any(|p| p.category == GenreCategory::HipHopTrap));
    assert!(presets.iter().any(|p| p.category == GenreCategory::BluesRockMetal));
    assert!(presets.iter().any(|p| p.category == GenreCategory::JazzFusion));
    assert!(presets.iter().any(|p| p.category == GenreCategory::ClassicalCinematic));
    assert!(presets.iter().any(|p| p.category == GenreCategory::PopElectronic));
    assert!(presets.iter().any(|p| p.category == GenreCategory::LatinCaribbean));
    assert!(presets.iter().any(|p| p.category == GenreCategory::FolkCountryWorld));
}

#[test]
fn test_genre_lookup_and_intelligent_fallback() {
    let reggae = GenreTaxonomy::find_or_create("Roots Reggae");
    assert_eq!(reggae.id, "roots_reggae_1977");
    assert_eq!(reggae.default_bpm, 76);
    assert_eq!(reggae.recommended_scale, ModalScale::Mixolydian);

    let afrobeats = GenreTaxonomy::find_or_create("Afrobeats");
    assert_eq!(afrobeats.id, "afrobeats");
    assert_eq!(afrobeats.default_bpm, 105);

    let amapiano = GenreTaxonomy::find_or_create("Amapiano");
    assert_eq!(amapiano.id, "amapiano");
    assert_eq!(amapiano.rhythm_groove, RhythmGroove::AmapianoLogDrum);

    // Dynamic unknown genre fallback
    let cyberpunk = GenreTaxonomy::find_or_create("Glitch Cyberpunk Synth");
    assert_eq!(cyberpunk.category, GenreCategory::PopElectronic); // matched 'synth'
    assert_eq!(cyberpunk.default_bpm, 126);

    let unknown = GenreTaxonomy::find_or_create("Xylophone Polka Fusion 3000");
    assert_eq!(unknown.category, GenreCategory::ExperimentalHybrid);
    assert_eq!(unknown.default_bpm, 120);
}

#[test]
fn test_blueprint_schema_calculations() {
    let bp = SongBlueprint::default_one_love_shining_anthem();
    assert_eq!(bp.total_bars(), 52);

    let duration = bp.estimated_duration_secs();
    // 52 bars * 4 beats * (60 / 76) ≈ 164.21 seconds
    assert!((duration - 164.21).abs() < 1.0);

    let genre_txt = bp.to_yue_genre_txt();
    assert!(genre_txt.contains("76 BPM"));
    assert!(genre_txt.contains("Key of A Major"));
    assert!(genre_txt.ends_with(".\n"));

    let lyrics_txt = bp.to_yue_lyrics_txt();
    assert!(lyrics_txt.contains("[intro]"));
    assert!(lyrics_txt.contains("[verse]"));
    assert!(lyrics_txt.contains("[chorus]"));
    assert!(lyrics_txt.contains("[guitar solo]"));
}

#[test]
fn test_stem_channel_strip_and_decision_trace_roundtrip() {
    let mut state = MusicProductionState::new("nimble-mixer-01", "Jazz Fusion", 124, "Bb Major");
    state.lyrics_mode = LyricsMode::AiGenerated;
    state.lyrics_topic = "Rainy neon midnight train ride".to_string();
    state.decision_mode = DecisionMode::FastDeterministic;
    state.decision_model = Some("bespokelabs/Bespoke-Nimble-9B".to_string());
    state.commercial_only = true;
    state.record_decision_trace(TypedDecisionTrace {
        gate_name: "gate1_preflight_router".to_string(),
        schema_kind: "enum".to_string(),
        backend_used: "fast_deterministic".to_string(),
        selected_choice: "ACE-Step/acestep-v15-turbo".to_string(),
        probabilities: vec![
            ("ACE-Step/acestep-v15-turbo".to_string(), 0.84),
            ("ASLP-lab/DiffRhythm2".to_string(), 0.16),
        ],
        confidence: 0.84,
        entropy: 0.44,
        hitl_gate_triggered: false,
        latency_ms: 1,
        license_tier: "MIT".to_string(),
    });

    let mut mix = StemMixConfig::default();
    mix.set_channel(
        "raspy_tenor_vocals",
        StemChannelStrip {
            gain_db: 3.5,
            pan: -0.15,
            mute: false,
            solo: true,
            eq_low_db: -2.0,
            eq_mid_db: 3.0,
            eq_high_db: 4.5,
            reverb_send: 0.28,
        },
    );
    let sanitized = mix.resolve_channel("vocals");
    assert_eq!(sanitized.gain_db, 3.5);
    assert_eq!(sanitized.pan, -0.15);
    assert!(sanitized.solo);
    assert!(mix.any_solo());
    assert_eq!(mix.lead_vocal_db, 3.5);

    let json = state.to_json().expect("serialize");
    let restored = MusicProductionState::from_json(&json).expect("deserialize");
    assert_eq!(restored.lyrics_mode, LyricsMode::AiGenerated);
    assert_eq!(restored.decision_mode, DecisionMode::FastDeterministic);
    assert_eq!(restored.decision_traces.len(), 1);
    assert!(restored.commercial_only);
}
