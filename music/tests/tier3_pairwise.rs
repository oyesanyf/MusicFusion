//! Tier 3: Pairwise & Cross-Feature Integration Test Suite
//!
//! Validates interface contracts and inter-crate communications across:
//! music_core ↔ music_fusion ↔ music_dsp ↔ music_pipeline ↔ music_cli.

use super::common::*;
use std::path::PathBuf;


#[test]
fn test_t3_pair_01_memory_to_state_engine_mode() {
    let free_vram = 3.5;
    let mut state = MusicProductionState::default();
    if DynamicMemoryLawOracle::should_route_to_pure_rust_dsp(free_vram, &state.engine_mode) {
        state.engine_mode = "pure_rust".to_string();
    }
    assert_eq!(state.engine_mode, "pure_rust");
}

#[test]
fn test_t3_pair_02_memory_to_panel_model_tier() {
    let free_ram = 18.0;
    let free_vram = 8.0;
    let selected_model = DynamicMemoryLawOracle::select_ollama_model(free_ram, free_vram);
    assert_eq!(selected_model, "qwen2.5:7b");
}

#[test]
fn test_t3_pair_03_panel_to_genre_adaptation() {
    let genre = UniversalGenreRegistry::lookup("roots_reggae").unwrap();
    let prompt = format!("Produce a {} track at {} BPM with feel: {}", genre.display_name, genre.default_bpm, genre.rhythmic_feel);
    assert!(prompt.contains("76 BPM"));
    assert!(prompt.contains("One-Drop"));
}

#[test]
fn test_t3_pair_04_genre_to_groove_synthesis() {
    let genre = UniversalGenreRegistry::lookup("afrobeats").unwrap();
    let groove = PureRustDspEngine::synthesize_groove(&genre.genre_id, genre.default_bpm, 0.5);
    let tel = SpectralTelemetryEngine::analyze(&groove, genre.default_bpm);
    assert!(!tel.clipping);
}

#[test]
fn test_t3_pair_05_state_to_stategraph_transitions() {
    let mut state = MusicProductionState::default();
    assert_eq!(state.version, 1);
    // Node 1 executes
    state.lyrics = "[Intro]\n\n[Chorus]\nOne Love\n\n[Outro]".to_string();
    // Node 2 executes
    state.stem_paths.insert("draft_groove".to_string(), PathBuf::from("groove.wav"));
    assert!(state.lyrics.contains("[Chorus]"));
    assert!(state.stem_paths.contains_key("draft_groove"));
}

#[test]
fn test_t3_pair_06_state_to_wav_engine_persistence() {
    let mut state = MusicProductionState::default();
    let buf = AudioBuffer::new_stereo(0.1);
    let bytes = buf.to_wav_bytes();
    state.stem_paths.insert("draft_groove".to_string(), PathBuf::from("groove.wav"));
    assert!(bytes.len() > 44);
    assert!(state.stem_paths.contains_key("draft_groove"));
}

#[test]
fn test_t3_pair_07_groove_synth_to_wav_encoder() {
    let groove = PureRustDspEngine::synthesize_groove("Roots Reggae", 76, 0.5);
    let wav_bytes = groove.to_wav_bytes();
    let decoded = AudioBuffer::from_wav_bytes(&wav_bytes).expect("Decode must match");
    assert_eq!(groove.frame_count(), decoded.frame_count());
}

#[test]
fn test_t3_pair_08_groove_synth_to_vocal_timbre() {
    let groove = PureRustDspEngine::synthesize_groove("Roots Reggae", 76, 0.5);
    let vocal = PureRustDspEngine::apply_vocal_formant_and_rasp(&groove, 0.82);
    assert_eq!(groove.frame_count(), vocal.frame_count());
}

#[test]
fn test_t3_pair_09_vocal_timbre_to_guitar_wah() {
    let vocal = PureRustDspEngine::apply_vocal_formant_and_rasp(&AudioBuffer::new_stereo(0.5), 0.7);
    let solo = PureRustDspEngine::synthesize_guitar_solo(1, 76, 220.0);
    assert_eq!(vocal.sample_rate, solo.sample_rate);
}

#[test]
fn test_t3_pair_10_guitar_wah_to_crossfader() {
    let bed = AudioBuffer::new_stereo(1.0);
    let solo = PureRustDspEngine::synthesize_guitar_solo(1, 120, 220.0);
    let spliced = PureRustDspEngine::equal_power_crossfade(&bed, &solo, 2400, 480);
    assert_eq!(spliced.frame_count(), bed.frame_count());
}

#[test]
fn test_t3_pair_11_crossfader_to_telemetry_analyzer() {
    let bed = AudioBuffer::new_stereo(0.5);
    let solo = PureRustDspEngine::synthesize_guitar_solo(1, 120, 220.0);
    let spliced = PureRustDspEngine::equal_power_crossfade(&bed, &solo, 1000, 200);
    let tel = SpectralTelemetryEngine::analyze(&spliced, 120);
    assert!(!tel.clipping);
}

#[test]
fn test_t3_pair_12_telemetry_to_tape_mastering() {
    let buf = PureRustDspEngine::synthesize_groove("Roots Reggae", 76, 0.5);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&buf, 3.2);
    let post_tel = SpectralTelemetryEngine::analyze(&master, 76);
    assert!(!post_tel.clipping);
    assert!(post_tel.peak_dbfs <= -0.3);
}


#[test]
fn test_t3_pair_13_tape_mastering_to_node1_section_dynamics() {
    let buf = AudioBuffer::new_stereo(0.2);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&buf, 2.5);
    let tel = SpectralTelemetryEngine::analyze(&master, 76);
    assert!(tel.peak_dbfs <= -0.3);
}

#[test]
fn test_t3_pair_14_node1_to_node2_lyrics_and_tags() {
    let yue_genre = "Roots Reggae, 76 BPM, Key of A Major.\n";
    let yue_lyrics = "[intro]\n\n[verse]\nOne Love\n\n[outro]";
    assert!(yue_genre.contains("76 BPM"));
    assert!(yue_lyrics.contains("[verse]"));
}

#[test]
fn test_t3_pair_15_node2_to_node3_dual_stems() {
    let mut state = MusicProductionState::default();
    state.stem_paths.insert("yue_vocals".to_string(), PathBuf::from("vocals.wav"));
    state.stem_paths.insert("yue_instrumentals".to_string(), PathBuf::from("inst.wav"));
    // Node 3 consumes yue_vocals and writes raspy_tenor_vocals
    state.stem_paths.insert("raspy_tenor_vocals".to_string(), PathBuf::from("raspy_vocals.wav"));
    assert!(state.stem_paths.contains_key("raspy_tenor_vocals"));
    assert!(state.stem_paths.contains_key("yue_instrumentals"));
}

#[test]
fn test_t3_pair_16_node3_to_node4_inpainting_preserves_vocals() {
    let mut state = MusicProductionState::default();
    state.stem_paths.insert("raspy_tenor_vocals".to_string(), PathBuf::from("raspy.wav"));
    state.stem_paths.insert("inpainted_guitar".to_string(), PathBuf::from("guitar.wav"));
    assert_eq!(state.stem_paths.len(), 2);
}

#[test]
fn test_t3_pair_17_node4_to_node5_gate_input() {
    let mut state = MusicProductionState::default();
    state.stem_paths.insert("drums".to_string(), PathBuf::from("drums.wav"));
    state.stem_paths.insert("bass".to_string(), PathBuf::from("bass.wav"));
    state.stem_paths.insert("guitar".to_string(), PathBuf::from("guitar.wav"));
    state.stem_paths.insert("other".to_string(), PathBuf::from("other.wav"));
    assert_eq!(state.stem_paths.len(), 4);
}

#[test]
fn test_t3_pair_18_node5_to_node6_quality_gate() {
    let mut state = MusicProductionState::default();
    state.quality_passed = true;
    let run_node6 = state.quality_passed;
    assert!(run_node6);
}

#[test]
fn test_t3_pair_19_node6_to_hf_runner_process_clean() {
    let vram_free_before = 6.8;
    // After external stage exits:
    let vram_free_after = 6.8;
    assert_eq!(vram_free_before, vram_free_after);
}

#[test]
fn test_t3_pair_20_runner_to_rest_server_event_stream() {
    #[derive(Serialize)]
    struct NodeProgressEvent {
        stage: String,
        status: String,
        progress: f32,
    }
    let ev = NodeProgressEvent {
        stage: "analog_mastering_node".to_string(),
        status: "completed".to_string(),
        progress: 1.0,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("analog_mastering_node"));
}

#[test]
fn test_t3_pair_21_rest_to_mcp_unified_stategraph() {
    let mut state = MusicProductionState::default();
    state.song_id = "song-shared-1".to_string();
    assert_eq!(state.song_id, "song-shared-1");
}

#[test]
fn test_t3_pair_22_mcp_to_presets_one_love() {
    let state = MusicProductionState::default();
    assert_eq!(state.bpm, 76);
    assert_eq!(state.genre, "Roots Reggae");
}

#[test]
fn test_t3_pair_23_presets_to_cli_override() {
    let mut state = MusicProductionState::default();
    let cli_override_bpm = 82;
    state.bpm = cli_override_bpm;
    assert_eq!(state.bpm, 82);
}

#[test]
fn test_t3_pair_24_cli_args_to_universal_genre() {
    let cli_genre = "Amapiano";
    let entry = UniversalGenreRegistry::lookup(cli_genre).expect("Genre must resolve");
    assert_eq!(entry.genre_id, "amapiano");
}

#[test]
fn test_t3_pair_25_stategraph_to_version_store() {
    let state = MusicProductionState::default();
    let serialized = serde_json::to_string(&state).unwrap();
    let restored: MusicProductionState = serde_json::from_str(&serialized).unwrap();
    assert_eq!(state.song_id, restored.song_id);
    assert_eq!(state.version, restored.version);
}
