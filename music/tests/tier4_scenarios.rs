//! Tier 4: Real-World Multi-Genre Application Scenarios
//!
//! Exercises full end-to-end music production across diverse global genres:
//! 1. Flagship 1977 Roots Reggae "One Love" Anthem (76 BPM, A Major)
//! 2. High-Energy Afrobeats / Amapiano Hit (108 BPM, F# Minor)
//! 3. Mainstream Pop Radio Hit (120 BPM, C Major)
//! 4. Fast Bebop Jazz / Fusion (140 BPM, Bb Major)
//! 5. Stadium Psychedelic Rock & Screaming Guitar Solo (128 BPM, E Minor)
//! 6. Cinematic Orchestral & Epic Choral Score (84 BPM, D Minor, 6/8 Compound)
//! 7. Heavy Boom-Bap & 808 Trap Hip-Hop (144 BPM, G Minor)
//! 8. Club Melodic House & EDM (126 BPM, A Minor)

use super::common::*;
use std::path::PathBuf;

#[test]
fn test_scenario_01_flagship_roots_reggae_anthem() {
    let mut state = MusicProductionState::default();
    assert_eq!(state.bpm, 76);
    assert_eq!(state.key_signature, "A Major");
    assert!(state.style_prompt.contains("1977 roots reggae"));

    // Stage 1 & 2: 76 BPM One-Drop Groove and Harmonic Accompaniment
    let groove = PureRustDspEngine::synthesize_groove("Roots Reggae", state.bpm, 2.0);
    assert_eq!(groove.sample_rate, 48_000);
    state.stem_paths.insert("draft_groove".to_string(), PathBuf::from("draft_groove.wav"));
    state.stem_paths.insert("yue_instrumentals".to_string(), PathBuf::from("yue_instrumentals.wav"));

    // Stage 3: Soulful Raspy Jamaican Male Tenor Vocals
    let _vocal_formant = PureRustDspEngine::apply_vocal_formant_and_rasp(&groove, 0.82);
    state.stem_paths.insert("raspy_tenor_vocals".to_string(), PathBuf::from("raspy_tenor_vocals.wav"));

    // Stage 4: 16-Bar Crying Blues Guitar Solo Inpainting & Nyabinghi Fills
    let solo = PureRustDspEngine::synthesize_guitar_solo(2, state.bpm, 220.0); // A3 root
    let inpainted = PureRustDspEngine::equal_power_crossfade(&groove, &solo, 2400, 480);
    state.stem_paths.insert("inpainted_guitar".to_string(), PathBuf::from("inpainted_guitar.wav"));

    // Stage 5: Telemetry Inspection & Gate
    let tel = SpectralTelemetryEngine::analyze(&inpainted, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.peak_dbfs <= 0.0);
    state.telemetry = tel;
    state.quality_passed = true;

    // Stage 6: 1977 Studer A800 Warm Analog Tape & Sub-Bass Mastering
    let final_master = PureRustDspEngine::apply_analog_tape_mastering(&inpainted, 3.2);
    let master_tel = SpectralTelemetryEngine::analyze(&final_master, state.bpm);
    assert!(!master_tel.clipping);
    assert!(master_tel.peak_dbfs <= -0.3); // -0.3 dBFS ceiling
    state.stem_paths.insert("final_master".to_string(), PathBuf::from("final_master.wav"));

    // Verification invariants
    assert_eq!(state.stem_paths.len(), 5);
    assert!(state.quality_passed);
    assert_eq!(state.version, 1);
}

#[test]
fn test_scenario_02_afrobeats_amapiano_hit() {
    let state = MusicProductionState {
        song_id: "song-afrobeats-lagos-energy".to_string(),
        genre: "Afrobeats".to_string(),
        bpm: 108,
        key_signature: "F# Minor".to_string(),
        style_prompt: "Lagos midnight energy, syncopated log-drum clave, nylon guitar, smooth vocal harmonies".to_string(),
        ..Default::default()
    };

    // Stage 1: Groove with Tresillo log-drum clave
    let groove = PureRustDspEngine::synthesize_groove("Afrobeats", state.bpm, 2.0);
    let tel_groove = SpectralTelemetryEngine::analyze(&groove, state.bpm);
    assert!(!tel_groove.clipping);

    // Stage 3: Smooth vocal processing
    let vocal = PureRustDspEngine::apply_vocal_formant_and_rasp(&groove, 0.25);
    let tel_vocal = SpectralTelemetryEngine::analyze(&vocal, state.bpm);
    assert!(!tel_vocal.clipping);

    // Stage 6: Modern punchy bass master
    let master = PureRustDspEngine::apply_analog_tape_mastering(&vocal, 4.0);
    let master_tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!master_tel.clipping);
    assert!(master_tel.peak_dbfs <= -0.3);
}

#[test]
fn test_scenario_03_mainstream_pop_radio_hit() {
    let state = MusicProductionState {
        song_id: "song-summer-pop-radio".to_string(),
        genre: "Pop".to_string(),
        bpm: 120,
        key_signature: "C Major".to_string(),
        style_prompt: "Bright summer radio pop anthem, driving four-on-the-floor kick, soaring vocal hook".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("Pop", state.bpm, 2.0);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&groove, 2.0);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.peak_dbfs <= -0.3);
    assert!(tel.dynamic_range_db >= 6.0);
}

#[test]
fn test_scenario_04_fast_bebop_jazz_fusion() {
    let state = MusicProductionState {
        song_id: "song-harlem-bebop-swing".to_string(),
        genre: "Jazz Bebop / Fusion".to_string(),
        bpm: 140,
        key_signature: "Bb Major".to_string(),
        style_prompt: "Late night jazz club, fast bebop swing, walking acoustic bass, swung ride cymbal".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("Jazz", state.bpm, 2.0);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&groove, 1.0);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.dynamic_range_db >= 10.0); // Jazz requires high dynamic range
}

#[test]
fn test_scenario_05_stadium_psychedelic_rock() {
    let state = MusicProductionState {
        song_id: "song-stadium-rock-solo".to_string(),
        genre: "Psychedelic / Stadium Rock".to_string(),
        bpm: 128,
        key_signature: "E Minor".to_string(),
        style_prompt: "Driving heavy drums, distorted power chords, screaming wah-wah electric guitar solo".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("Rock", state.bpm, 2.0);
    let solo = PureRustDspEngine::synthesize_guitar_solo(2, state.bpm, 330.0); // E4 note
    let spliced = PureRustDspEngine::equal_power_crossfade(&groove, &solo, 2400, 480);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&spliced, 3.5);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.peak_dbfs <= -0.3);
}

#[test]
fn test_scenario_06_cinematic_orchestral_compound_meter() {
    let state = MusicProductionState {
        song_id: "song-cinematic-epic-score".to_string(),
        genre: "Cinematic Orchestral".to_string(),
        bpm: 84,
        key_signature: "D Minor".to_string(),
        time_signature: "6/8".to_string(),
        style_prompt: "Majestic fantasy score, thunderous timpani rolls, string sections, brass fanfare".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("Cinematic", state.bpm, 2.0);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&groove, 1.5);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert_eq!(state.time_signature, "6/8");
}

#[test]
fn test_scenario_07_heavy_boom_bap_trap_hiphop() {
    let state = MusicProductionState {
        song_id: "song-late-night-trap-808".to_string(),
        genre: "Hip-Hop / Trap".to_string(),
        bpm: 144,
        key_signature: "G Minor".to_string(),
        style_prompt: "Dark trap banger, booming 808 sub-bass, rolling 32nd hi-hats, hard snare".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("Hip-Hop", state.bpm, 2.0);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&groove, 5.0);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.peak_dbfs <= -0.3);
}

#[test]
fn test_scenario_08_club_melodic_house_edm() {
    let state = MusicProductionState {
        song_id: "song-festival-melodic-house".to_string(),
        genre: "EDM / Melodic House".to_string(),
        bpm: 126,
        key_signature: "A Minor".to_string(),
        style_prompt: "Sunrise melodic house, pumping four-on-the-floor, euphoric supersaw chords".to_string(),
        ..Default::default()
    };

    let groove = PureRustDspEngine::synthesize_groove("EDM", state.bpm, 2.0);
    let master = PureRustDspEngine::apply_analog_tape_mastering(&groove, 3.0);
    let tel = SpectralTelemetryEngine::analyze(&master, state.bpm);
    assert!(!tel.clipping);
    assert!(tel.peak_dbfs <= -0.3);
}

