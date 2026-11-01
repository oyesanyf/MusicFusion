use std::path::PathBuf;
use music_core::{
    GenreCategory, GenreTaxonomy, MusicProductionState,
    RhythmGroove, SongBlueprint, StemTelemetry,
    STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER, STEM_GUITAR,
    STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER, STEM_RASPY_TENOR_VOCALS,
    STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};

// ============================================================================
// 1. BOUNDARY TEMPOS (40 BPM, 220 BPM, AND OUTSIDE LIMITS)
// ============================================================================

#[test]
fn test_stress_tempo_boundary_minimum_40_bpm() {
    let state = MusicProductionState::new("boundary-min-40", "Ambient Chill", 40, "C Minor");
    assert_eq!(state.bpm, 40);
    assert_eq!(state.time_signature, "4/4");

    // Check blueprint duration calculation with min 40 BPM
    let mut bp = SongBlueprint::default_one_love_shining_anthem();
    bp.bpm = 40.0;
    let dur = bp.estimated_duration_secs();
    // 52 bars * 4 beats * (60 / 40) = 312 seconds
    assert!((dur - 312.0).abs() < 1e-4);
}

#[test]
fn test_stress_tempo_boundary_maximum_220_bpm() {
    let state = MusicProductionState::new("boundary-max-220", "Fast Drum & Bass", 220, "F# Minor");
    assert_eq!(state.bpm, 220);

    let mut bp = SongBlueprint::default_one_love_shining_anthem();
    bp.bpm = 220.0;
    let dur = bp.estimated_duration_secs();
    // 52 bars * 4 beats * (60 / 220) = 56.7272... seconds
    assert!((dur - (208.0 * 60.0 / 220.0)).abs() < 1e-4);
}

#[test]
fn test_stress_tempo_extreme_low_and_zero_protection() {
    // 0 BPM: state accepts u32
    let state_zero = MusicProductionState::new("bpm-0", "Drone", 0, "C");
    assert_eq!(state_zero.bpm, 0);

    // SongBlueprint estimated_duration_secs protects against division by zero using .max(40.0)
    let mut bp = SongBlueprint::default_one_love_shining_anthem();
    bp.bpm = 0.0;
    let dur_zero = bp.estimated_duration_secs();
    assert!(!dur_zero.is_nan());
    assert!(!dur_zero.is_infinite());
    // Should clamp to 40.0 BPM floor
    assert!((dur_zero - 312.0).abs() < 1e-4);

    // Negative BPM protection
    bp.bpm = -50.0;
    let dur_neg = bp.estimated_duration_secs();
    assert!(!dur_neg.is_nan());
    assert!((dur_neg - 312.0).abs() < 1e-4);
}

#[test]
fn test_stress_tempo_extreme_high_and_overflow() {
    // 999 BPM
    let state_high = MusicProductionState::new("bpm-999", "Speedcore", 999, "A Minor");
    assert_eq!(state_high.bpm, 999);

    // u32::MAX BPM
    let state_max = MusicProductionState::new("bpm-max", "Extratone", u32::MAX, "C");
    assert_eq!(state_max.bpm, u32::MAX);

    let mut bp = SongBlueprint::default_one_love_shining_anthem();
    bp.bpm = 10_000.0;
    let dur_fast = bp.estimated_duration_secs();
    assert!(dur_fast > 0.0);
    assert!(!dur_fast.is_nan());
}

// ============================================================================
// 2. UNUSUAL / CUSTOM HYBRID GENRE PROMPTS & INTELLIGENT FALLBACK
// ============================================================================

#[test]
fn test_stress_hybrid_genre_cyberpunk_celtic_drone_metal() {
    let p = GenreTaxonomy::find_or_create("Cyberpunk Celtic Drone Metal");
    // "cyber" matches PopElectronic preset template
    assert_eq!(p.category, GenreCategory::PopElectronic);
    assert_eq!(p.name, "Cyberpunk Celtic Drone Metal");
    assert_eq!(p.id, "cyberpunk_celtic_drone_metal");
    assert!(p.default_bpm >= 40 && p.default_bpm <= 220);
    assert!(!p.recommended_instrumentation.is_empty());
    assert!(!p.vocal_archetype.is_empty());
    assert!(!p.mastering_profile.is_empty());
}

#[test]
fn test_stress_hybrid_genre_afro_cuban_jazz_fusion() {
    let p = GenreTaxonomy::find_or_create("Afro-Cuban Jazz Fusion");
    // "jazz" matches the canonical jazz preset (which also covers fusion)
    assert_eq!(p.category, GenreCategory::JazzFusion);
    assert_eq!(p.id, "jazz");
    assert_eq!(p.default_bpm, 180);
    assert!(!p.style_tags.is_empty());
    assert!(!p.recommended_instrumentation.is_empty());
}

#[test]
fn test_stress_unusual_unknown_strings_fallback() {
    // Completely unrecognized hybrid/novel style
    let p1 = GenreTaxonomy::find_or_create("Xylophone Polka Fusion 3000");
    assert_eq!(p1.category, GenreCategory::ExperimentalHybrid);
    assert_eq!(p1.default_bpm, 120);
    assert_eq!(p1.default_time_signature, "4/4");
    assert_eq!(p1.default_key, "C Major");
    assert_eq!(p1.rhythm_groove, RhythmGroove::FourOnTheFloor);
    assert_eq!(p1.id, "xylophone_polka_fusion_3000");

    // Special characters, symbols, unicode
    let p2 = GenreTaxonomy::find_or_create("!@#$%^&*()_+~{}|:\"<>?");
    assert_eq!(p2.category, GenreCategory::ExperimentalHybrid);
    assert!(!p2.id.contains('!'));
    assert!(!p2.id.contains('@'));

    // Non-Latin UTF-8 (Japanese Katakana, Cyrillic, Emoji) without matching Latin keywords
    let p3 = GenreTaxonomy::find_or_create("日本語シティーポップ 🎵 1982");
    // Falls back gracefully to ExperimentalHybrid without panic
    assert_eq!(p3.category, GenreCategory::ExperimentalHybrid);
    assert_eq!(p3.name, "日本語シティーポップ 🎵 1982");

    // Latin City Pop query matches PopElectronic
    let p3_latin = GenreTaxonomy::find_or_create("Japanese City Pop 1982");
    assert_eq!(p3_latin.category, GenreCategory::PopElectronic);

    // Empty and whitespace-only queries resolve to the flagship preset (1977 Roots Reggae Anthem)
    let p4 = GenreTaxonomy::find_or_create("");
    assert_eq!(p4.category, GenreCategory::RootsReggaeDub);
    assert_eq!(p4.default_bpm, 76);

    let p5 = GenreTaxonomy::find_or_create("   \t\n  ");
    assert_eq!(p5.category, GenreCategory::RootsReggaeDub);
    assert_eq!(p5.default_bpm, 76);
}

#[test]
fn test_stress_all_20_registered_presets_validity() {
    let presets = GenreTaxonomy::all_presets();
    assert_eq!(presets.len(), 20, "Must contain exactly 20 canonical presets");

    for p in &presets {
        assert!(!p.id.is_empty(), "Preset id cannot be empty");
        assert!(!p.name.is_empty(), "Preset name cannot be empty");
        assert!(!p.subgenres.is_empty(), "Preset subgenres cannot be empty");
        assert!(p.default_bpm >= 40 && p.default_bpm <= 220, "BPM out of bounds: {}", p.default_bpm);
        assert!(p.bpm_range.0 <= p.bpm_range.1, "Invalid BPM range");
        assert!(!p.default_time_signature.is_empty(), "Time signature missing");
        assert!(!p.default_key.is_empty(), "Default key missing");
        assert!(!p.description.is_empty(), "Description missing");
        assert!(!p.style_tags.is_empty(), "Style tags empty");
        assert!(!p.recommended_instrumentation.is_empty(), "Instrumentation empty");
        assert!(!p.vocal_archetype.is_empty(), "Vocal archetype empty");
        assert!(!p.mastering_profile.is_empty(), "Mastering profile empty");

        // Verify lookup resolves correctly for each preset by id and by name
        let resolved_id = GenreTaxonomy::find_or_create(&p.id);
        assert_eq!(resolved_id.id, p.id);
        let resolved_name = GenreTaxonomy::find_or_create(&p.name);
        assert_eq!(resolved_name.id, p.id);
    }
}

// ============================================================================
// 3. STEM MUTATION SEQUENCES
// ============================================================================

#[test]
fn test_stress_stem_mutations_lifecycle() {
    let mut state = MusicProductionState::new("stem-test", "Roots Reggae", 76, "A Major");
    assert_eq!(state.stem_paths.len(), 0);

    let all_11_stems = [
        STEM_DRAFT_GROOVE,
        STEM_YUE_VOCALS,
        STEM_YUE_INSTRUMENTALS,
        STEM_RASPY_TENOR_VOCALS,
        STEM_DRUMS,
        STEM_BASS,
        STEM_GUITAR,
        STEM_OTHER,
        STEM_INPAINTED_GUITAR,
        STEM_INPAINTED_DRUMS,
        STEM_FINAL_MASTER,
    ];

    // Assert initially none exist
    assert!(!state.has_required_stems(&all_11_stems));
    for stem in all_11_stems {
        assert!(!state.has_stem(stem));
        assert!(state.get_stem(stem).is_none());
    }

    // Empty required list returns true (vacuously true)
    assert!(state.has_required_stems(&[]));

    // Register stems sequentially
    for (i, stem) in all_11_stems.iter().enumerate() {
        let fake_path = PathBuf::from(format!("storage/{}.wav", stem));
        state.set_stem(*stem, fake_path.clone());

        assert!(state.has_stem(stem));
        assert_eq!(state.get_stem(stem), Some(&fake_path));
        assert_eq!(state.stem_paths.len(), i + 1);
    }

    // Now all 11 stems exist
    assert!(state.has_required_stems(&all_11_stems));

    // Test alias record_stem overwrites cleanly
    let new_path = PathBuf::from("storage/new_final_master_v2.wav");
    state.record_stem(STEM_FINAL_MASTER, new_path.clone());
    assert_eq!(state.get_stem(STEM_FINAL_MASTER), Some(&new_path));
    assert_eq!(state.stem_paths.len(), 11);

    // Test partial required stems check
    let core_stems = [STEM_DRUMS, STEM_BASS, STEM_FINAL_MASTER];
    assert!(state.has_required_stems(&core_stems));

    // Remove one stem and check required stems fails
    let removed = state.remove_stem(STEM_DRUMS);
    assert_eq!(removed, Some(PathBuf::from("storage/drums.wav")));
    assert!(!state.has_stem(STEM_DRUMS));
    assert!(!state.has_required_stems(&all_11_stems));
    assert!(!state.has_required_stems(&core_stems));
    assert_eq!(state.stem_paths.len(), 10);

    // Removing non-existent stem returns None
    assert_eq!(state.remove_stem("non_existent_stem"), None);
}

// ============================================================================
// 4. SERDE JSON ROUNDTRIPS WITH CORRUPTED / MISSING OPTIONAL FIELDS
// ============================================================================

#[test]
fn test_stress_serde_roundtrip_complete() {
    let mut state = MusicProductionState::default_reggae_anthem();
    state.set_stem(STEM_DRAFT_GROOVE, "storage/draft.wav");
    state.set_stem(STEM_FINAL_MASTER, "storage/master.wav");
    state.vocal_ref_wav = Some(PathBuf::from("refs/bob_marley_vocal.wav"));
    state.master_ref_wav = Some(PathBuf::from("refs/exodus_tape.wav"));
    state.record_node_transition("node1_producer_panel");
    state.record_node_transition("node2_groove_yue");

    let json = state.to_json().expect("Serialization failed");
    let restored = MusicProductionState::from_json(&json).expect("Deserialization failed");

    assert_eq!(restored.song_id, state.song_id);
    assert_eq!(restored.version, state.version);
    assert_eq!(restored.engine_mode, state.engine_mode);
    assert_eq!(restored.bpm, state.bpm);
    assert_eq!(restored.key_signature, state.key_signature);
    assert_eq!(restored.vocal_ref_wav, state.vocal_ref_wav);
    assert_eq!(restored.master_ref_wav, state.master_ref_wav);
    assert_eq!(restored.stem_paths.len(), 2);
    assert_eq!(restored.active_node, Some("node2_groove_yue".to_string()));
    assert_eq!(restored.history.len(), state.history.len());
    assert!(restored.blueprint.is_some());
}

#[test]
fn test_stress_serde_deserialization_with_missing_optional_fields() {
    // Minimal JSON containing only the essential fields, with optional fields missing
    let json_minimal = r#"{
        "song_id": "minimal-001",
        "version": 1,
        "engine_mode": "auto",
        "style_prompt": "Roots Reggae",
        "lyrics": "[intro]",
        "bpm": 76,
        "key_signature": "A Major",
        "stem_paths": {},
        "telemetry": {
            "peak_dbfs": -0.5,
            "rms_dbfs": -14.0,
            "dynamic_range_db": 12.0,
            "tempo_drift_score": 0.002,
            "clipping": false,
            "snr_db": 92.0,
            "spectral_flatness": 0.18,
            "headroom_db": 0.5,
            "duration_secs": 0.0,
            "sample_rate": 48000,
            "channels": 2,
            "phase_alignment_score": 0.98,
            "vocal_headroom_db": 3.5
        },
        "judge_score": 0.0,
        "quality_passed": false,
        "retry_count": 0,
        "genre": "Roots Reggae",
        "time_signature": "4/4",
        "vocal_ref_wav": null,
        "master_ref_wav": null,
        "blueprint": null,
        "active_node": null,
        "history": [],
        "created_at": "2026-11-01T00:00:00Z",
        "updated_at": "2026-11-01T00:00:00Z"
    }"#;

    let res = MusicProductionState::from_json(json_minimal);
    assert!(res.is_ok(), "Should successfully deserialize when optional fields are null");
    let state = res.unwrap();
    assert_eq!(state.song_id, "minimal-001");
    assert!(state.vocal_ref_wav.is_none());
    assert!(state.master_ref_wav.is_none());
    assert!(state.blueprint.is_none());
    assert!(state.active_node.is_none());
}

#[test]
fn test_stress_serde_corrupted_json_rejection() {
    // 1. Incomplete/truncated JSON
    let truncated = r#"{"song_id": "bad", "bpm": 76"#;
    assert!(MusicProductionState::from_json(truncated).is_err());

    // 2. Syntax error: trailing comma / unmatched braces
    let syntax_err = r#"{"song_id": "bad", "bpm": 76,}"#;
    assert!(MusicProductionState::from_json(syntax_err).is_err());

    // 3. Type mismatch: string for bpm where integer expected
    let type_err = r#"{
        "song_id": "type-err",
        "version": 1,
        "engine_mode": "auto",
        "style_prompt": "",
        "lyrics": "",
        "bpm": "seventy-six",
        "key_signature": "A Major",
        "stem_paths": {},
        "telemetry": {},
        "judge_score": 0.0,
        "quality_passed": false,
        "retry_count": 0,
        "genre": "Reggae",
        "time_signature": "4/4",
        "history": [],
        "created_at": "",
        "updated_at": ""
    }"#;
    assert!(MusicProductionState::from_json(type_err).is_err());

    // 4. Missing required field (song_id)
    let missing_field = r#"{
        "version": 1,
        "bpm": 76
    }"#;
    assert!(MusicProductionState::from_json(missing_field).is_err());
}

// ============================================================================
// 5. STEM TELEMETRY VALIDATION WARNINGS, CLIPPING & BROADCAST READINESS
// ============================================================================

#[test]
fn test_stress_telemetry_nominal_broadcast_ready() {
    let tel = StemTelemetry::nominal();
    assert!(tel.is_broadcast_ready());
    assert!(tel.is_passing());
    assert!(tel.has_headroom(0.5));
    assert!(tel.validation_warnings().is_empty());
    assert!(!tel.summary().is_empty());
}

#[test]
fn test_stress_telemetry_clipping_and_headroom_violation() {
    let mut tel = StemTelemetry::default();
    tel.clipping = true;
    tel.peak_dbfs = 0.8;
    assert!(!tel.is_broadcast_ready());

    let warnings = tel.validation_warnings();
    assert_eq!(warnings.len(), 2);
    assert!(warnings[0].contains("Digital clipping detected"));
    assert!(warnings[1].contains("Peak exceeds 0.0 dBFS ceiling"));
}

#[test]
fn test_stress_telemetry_dynamic_range_over_compression() {
    let mut tel = StemTelemetry::default();
    tel.dynamic_range_db = 2.5; // less than 4.0 dB
    assert!(!tel.is_broadcast_ready());

    let warnings = tel.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("Dynamic range is excessively compressed")));
}

#[test]
fn test_stress_telemetry_tempo_drift_threshold() {
    let mut tel = StemTelemetry::default();
    tel.tempo_drift_score = 0.06; // >= 0.05
    assert!(!tel.is_broadcast_ready());

    let warnings = tel.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("Significant tempo drift detected")));
}

#[test]
fn test_stress_telemetry_hot_rms_loudness() {
    let mut tel = StemTelemetry::default();
    tel.rms_dbfs = -3.5; // hotter than -6.0 dBFS
    assert!(!tel.is_broadcast_ready());

    let warnings = tel.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("RMS loudness is dangerously hot")));
}

#[test]
fn test_stress_telemetry_phase_cancellation_risk() {
    let mut tel = StemTelemetry::default();
    tel.phase_alignment_score = 0.55; // < 0.70
    assert!(!tel.is_broadcast_ready());

    let warnings = tel.validation_warnings();
    assert!(warnings.iter().any(|w| w.contains("Phase cancellation warning")));
}

#[test]
fn test_stress_telemetry_all_violations_simultaneously() {
    let tel = StemTelemetry {
        peak_dbfs: 1.2,
        rms_dbfs: -2.0,
        dynamic_range_db: 1.5,
        tempo_drift_score: 0.12,
        clipping: true,
        snr_db: 40.0,
        spectral_flatness: 0.8,
        headroom_db: -1.2,
        duration_secs: 120.0,
        sample_rate: 48000,
        channels: 2,
        phase_alignment_score: 0.40,
        vocal_headroom_db: -2.0,
    };

    assert!(!tel.is_broadcast_ready());
    let warnings = tel.validation_warnings();
    // Should trigger all 6 warning conditions
    assert_eq!(warnings.len(), 6, "Expected 6 distinct warnings, got: {:?}", warnings);
}

#[test]
fn test_stress_telemetry_exact_boundary_conditions() {
    // Exact 0.0 dBFS peak without clipping flag
    let mut tel_ceiling = StemTelemetry::default();
    tel_ceiling.peak_dbfs = 0.0;
    tel_ceiling.clipping = false;
    assert!(tel_ceiling.is_broadcast_ready());
    assert!(tel_ceiling.validation_warnings().is_empty());

    // 0.0001 dBFS above ceiling
    tel_ceiling.peak_dbfs = 0.0001;
    assert!(!tel_ceiling.is_broadcast_ready());
    assert!(tel_ceiling.validation_warnings().iter().any(|w| w.contains("Peak exceeds")));

    // Exact -6.0 dBFS RMS
    let mut tel_rms = StemTelemetry::default();
    tel_rms.rms_dbfs = -6.0;
    assert!(tel_rms.is_broadcast_ready());

    tel_rms.rms_dbfs = -5.999;
    assert!(!tel_rms.is_broadcast_ready());

    // Exact 0.05 drift score
    let mut tel_drift = StemTelemetry::default();
    tel_drift.tempo_drift_score = 0.0499;
    assert!(tel_drift.is_broadcast_ready());

    tel_drift.tempo_drift_score = 0.05;
    assert!(!tel_drift.is_broadcast_ready());

    // Exact 0.70 phase score
    let mut tel_phase = StemTelemetry::default();
    tel_phase.phase_alignment_score = 0.70;
    assert!(tel_phase.is_broadcast_ready());

    tel_phase.phase_alignment_score = 0.699;
    assert!(!tel_phase.is_broadcast_ready());
}
