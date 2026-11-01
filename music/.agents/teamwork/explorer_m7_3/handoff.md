# Technical Exploration & Architecture Mapping Report: Requirement R4 & Verification

**Agent**: `explorer_m7_3` (Sample Songs & Test Explorer)  
**Date**: 2026-11-01  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m7_3\`  
**Target Milestone**: Milestone 7 (Sample Songs Regeneration & Full Test Suite Audit)

---

## 1. Observation

### 1.1 Layout and Status of `storage/songs/`
Direct inspection of `d:\harfile\music\storage\songs/` reveals four song directories, each containing `v1` and `v2` subdirectories:

1. **`roots_reggae_anthem/`**:
   - `v1/`: Contains `final_master.wav` (size: 20,534,444 bytes, duration: 106.95s, peak_dbfs: -0.518 dBFS), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` directory containing 14 files (10 standard stems plus 4 SAPI temporary artifacts: `_sapi_choir.wav`, `_sapi_lead.wav`, `_sapi_lyrics.txt`, `_sapi_script.ps1`).
   - `v1/manifest.json`: Does **not** contain a `"lyrics"` key.
   - `v2/`: Contains `final_master.wav` (size: 14,178,504 bytes, duration: 73.846s, peak_dbfs: -1.829 dBFS, BPM: 78), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` containing the 10 standard stem files.
   - `v2/manifest.json`: Does **not** contain a `"lyrics"` key.

2. **`song-one-love-shining-1977/`**:
   - `v1/`: Contains `final_master.wav` (size: 14,551,624 bytes, duration: 75.789s, peak_dbfs: -0.300 dBFS, BPM: 76), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` (10 standard stems). Manifest contains `"lyrics"` key with multi-section lyrics.
   - `v2/`: Contains `final_master.wav` (size: 14,178,504 bytes, duration: 73.846s, peak_dbfs: -0.300 dBFS, BPM: 78), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` (10 standard stems). Manifest contains `"lyrics"`.

3. **`song_afrobeats_002/`**:
   - `v1/`: Contains `final_master.wav` (size: 19,900,844 bytes, duration: 103.65s, peak_dbfs: -0.558 dBFS, BPM: 104), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` containing 14 files (10 standard stems plus 4 SAPI artifacts). Manifest does not contain `"lyrics"`.
   - `v2/`: Contains `final_master.wav` (size: 10,240,044 bytes, duration: 53.333s, peak_dbfs: -0.300 dBFS, BPM: 108), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` (10 standard stems). Manifest contains `"lyrics"`.

4. **`song_reggae_anthem_001/`**:
   - `v1/`: Contains `final_master.wav` (size: 14,551,624 bytes, duration: 75.789s, peak_dbfs: -0.300 dBFS, BPM: 76), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` (10 standard stems). In `v1/manifest.json`, line 9 specifies `"song_id": "roots_reggae_anthem"` and stem paths point to `storage\songs\roots_reggae_anthem\v1\stems\...` instead of `song_reggae_anthem_001`.
   - `v2/`: Contains `final_master.wav` (size: 14,178,504 bytes, duration: 73.846s, peak_dbfs: -0.300 dBFS, BPM: 78), `genre.txt`, `lyrics.txt`, `manifest.json`, `state.json`, and `stems/` (10 standard stems). In `v2/manifest.json`, line 9 also specifies `"song_id": "roots_reggae_anthem"` and points to `roots_reggae_anthem` stems.

### 1.2 The 10 Standard Stems Required per Song Version
In each `v1/stems/` and `v2/stems/` folder, the 10 standard stems defined in `music_core::state` are:
1. `bass.wav` (`STEM_BASS`)
2. `draft_groove.wav` (`STEM_DRAFT_GROOVE`)
3. `drums.wav` (`STEM_DRUMS`)
4. `guitar.wav` (`STEM_GUITAR`)
5. `inpainted_drums.wav` (`STEM_INPAINTED_DRUMS`)
6. `inpainted_guitar.wav` (`STEM_INPAINTED_GUITAR`)
7. `other.wav` (`STEM_OTHER`)
8. `raspy_tenor_vocals.wav` (`STEM_RASPY_TENOR_VOCALS`)
9. `yue_instrumentals.wav` (`STEM_YUE_INSTRUMENTALS`)
10. `yue_vocals.wav` (`STEM_YUE_VOCALS`)
Plus the top-level master:
11. `final_master.wav` (`STEM_FINAL_MASTER`)

### 1.3 How `--create-reggae-anthem` Works & Invocation Chain
1. **Entry Point** (`crates/music_cli/src/main.rs:46-65`):
   ```rust
   if args.create_reggae_anthem {
       println!("🇯🇲 Generating 1977 Roots Reggae 'One Love' Anthem...");
       let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
       let state = presets::flagship_reggae_anthem_state_with_id(
           Some(song_id),
           args.bpm,
           args.key.as_deref(),
           args.pure_rust,
       );
       let result = runner.execute_song(state).await?;
       println!("✨ Completed Anthem generation: song_id={}, version={}", result.song_id, result.version);
       if let Some(master) = result.stem_paths.get("final_master") {
           println!("🔊 Final Master: {}", master.display());
       }
   ...
   ```
2. **Preset Construction** (`crates/music_cli/src/presets.rs:18-41`):
   Calls `MusicProductionState::default_reggae_anthem()`, overrides `state.song_id` and `bp.song_id`, applies BPM and Key overrides, and sets `state.engine_mode = "pure_rust"` if requested.
3. **Pipeline Execution** (`crates/music_pipeline/src/runner.rs:72-130`):
   Compiles and executes the 6-stage `StateGraph`:
   - Stage 1: `OllamaProducerPanelNode`
   - Stage 2: `GrooveAndYueSynthesisNode` (generates `draft_groove.wav`, `yue_vocals.wav`, `yue_instrumentals.wav`)
   - Stage 3: `VocalTimbreFusionNode` (generates `raspy_tenor_vocals.wav`)
   - Stage 4: `VirtuosoStemInpaintingNode` (separates accompaniment into `drums.wav`, `bass.wav`, `guitar.wav`, `other.wav` and inplants `inpainted_guitar.wav` and `inpainted_drums.wav`)
   - Stage 5: `DspAndJudgeGateNode` (computes composite telemetry, evaluates quality gate)
   - Stage 6: `AnalogMasteringNode` (mixes stems into premaster, applies tape saturation and brickwall limiter, writes `final_master.wav` and `manifest.json`)

### 1.4 Spectral and Clipping Inspector Location & Mechanism
- **Inspector Implementation** (`crates/music_dsp/src/telemetry_calc.rs:287-345`):
  `calculate_telemetry(buffer: &AudioBuffer, expected_bpm: u32) -> StemTelemetry`
  - Computes `peak_dbfs`: `20.0 * max_abs.log10()`
  - Computes `clipping`: `true` if any sample `abs_val >= 1.0`
  - Computes `rms_dbfs`: `20.0 * (sum_sq / N).sqrt().log10()`
  - Computes `dynamic_range_db`: `(peak_dbfs - rms_dbfs).abs()`
  - Computes `tempo_drift_score`: Half-wave rectified onset energy autocorrelation compared against expected BPM 16th-note grid
  - Computes `spectral_flatness`: 1024-point Hann-windowed Radix-2 FFT computing Wiener entropy (ratio of geometric to arithmetic mean of power spectra)
  - Computes `snr_db`: Energy ratio of top 20th percentile active frames vs 10th percentile noise floor
  - Computes `phase_alignment_score`: Normalized stereo cross-correlation `(sum_lr / sqrt(sum_ll * sum_rr) + 1.0) / 2.0`
- **Gate Evaluation** (`crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs:175-197`):
  - Penalty applied if `telemetry.clipping` (-4.0) or `telemetry.peak_dbfs > 0.0` (-2.5).
  - Gate pass requires: `!telemetry.clipping && telemetry.peak_dbfs <= 0.0 && telemetry.tempo_drift_score <= 0.30 && score >= 7.5`.

### 1.5 Current Test Suite Audit Results
Commands executed directly across all workspace packages:

1. `cargo test -p music_core`:
   - Result: **27 passed, 0 failed** (adversarial_stress_tests: 20, core_tests: 7).
2. `cargo test -p music_dsp`:
   - Result: **28 passed, 1 failed**:
     - `test_adversarial_wav_roundtrip_all_formats_extreme_values` in `adversarial_dsp_stress_tests.rs:440` panicked with:
       `Diff 1 exceeds tolerance 0.00013051851`.
       Verbatim observation in `crates/music_dsp/src/wav.rs:371-387`: An automatic 4ms edge fade-in multiplies frame 0 by 0.0, zeroing out the initial sample (1.0 -> 0.0) during serialization.
3. `cargo test -p music_fusion`:
   - Result: **43 passed, 0 failed** (unittests: 4, adversarial_fusion: 17, challenger_m6_tts: 10, fusion_tests: 12).
4. `cargo test -p music_pipeline`:
   - Result: **60 passed, 0 failed** (adversarial_pipeline: 13, challenger_stress: 12, graph_tests: 17, node_tests: 13, pipeline_e2e: 5).
5. `cargo test -p music_cli`:
   - Result: **16 passed, 1 failed**:
     - `cli_and_rest_tests.rs`: 7 passed.
     - `m4_adversarial_challenge_tests.rs`: 13 passed.
     - `challenger_m6_2_empirical_tests.rs`: 3 passed.
     - `tier5_adversarial_challenge_suite.rs`: 8 passed.
     - `challenger_tier5_2_empirical_stress_tests.rs`: 4 passed, 1 failed:
       `test_disk_song_artifacts_roots_reggae_and_one_love` panicked at line 209:
       `assertion failed: (v1_master_buf.duration_secs() - expected_dur_v1).abs() < 1.0`
       (because `roots_reggae_anthem/v1` has duration 106.95s, while test asserts `expected_dur_v1 = 75.78s`).
6. `cargo test --manifest-path tests/Cargo.toml` (`music_e2e_tests`):
   - Result: **191 passed, 0 failed** (tier1: 75, tier2: 30, tier3: 25, tier4: 8, e2e: 53).
7. `python tests/validate_disk_songs.py`:
   - Result: **FAILED**:
     `AssertionError: manifest.json missing 'lyrics' key in storage/songs\roots_reggae_anthem\v1`
     Verbatim observation in `crates/music_pipeline/src/nodes/node6_analog_mastering.rs:238-253`: The JSON serializer omits `"lyrics": state.lyrics`.

---

## 2. Logic Chain

1. **Why `test_disk_song_artifacts_roots_reggae_and_one_love` failed**:
   - `challenger_tier5_2_empirical_stress_tests.rs:161` sets `targets = [("roots_reggae_anthem", 75.78, 73.84), ("song-one-love-shining-1977", 75.78, 73.84)]`.
   - In 48 kHz stereo 16-bit PCM, 24 bars at 76 BPM is 96 beats = `96 * (60 / 76) = 75.78947` seconds = 14,551,624 bytes.
   - `song-one-love-shining-1977/v1/final_master.wav` is exactly 14,551,624 bytes (75.789s), which passes the assertion.
   - However, `roots_reggae_anthem/v1/final_master.wav` was previously generated as 20,534,444 bytes (106.95s).
   - Because `(106.95 - 75.78).abs() = 31.17 > 1.0`, line 209 panicked.
   - Regenerating `roots_reggae_anthem/v1` with the canonical 24-bar arrangement (75.789s) will resolve this failure directly.

2. **Why `validate_disk_songs.py` failed**:
   - `tests/validate_disk_songs.py:208` asserts `"lyrics" in manifest`.
   - `crates/music_pipeline/src/nodes/node6_analog_mastering.rs:238` constructs `manifest.json` with `song_id`, `version`, `genre`, `bpm`, `key_signature`, `time_signature`, `style_prompt`, `judge_score`, `quality_passed`, `retry_count`, `telemetry`, `stems`, `created_at`, `updated_at`, but omitted `"lyrics"`.
   - Adding `"lyrics": state.lyrics` into `node6_analog_mastering.rs:246` ensures every generated `manifest.json` satisfies `validate_disk_songs.py`.

3. **Why `test_adversarial_wav_roundtrip_all_formats_extreme_values` failed**:
   - In `crates/music_dsp/src/wav.rs:371-387`, `edge_fade_frames` applies a 4ms linear fade-in and fade-out to all buffers during WAV serialization.
   - For an adversarial roundtrip test with sample[0] = 1.0, the fade-in scales sample[0] to `1.0 * (0 / fade_frames) = 0.0`.
   - Decoded WAV sample[0] becomes 0.0, yielding `(1.0 - 0.0) = 1.0 > 0.0001305`, failing the test.
   - Removing the disruptive edge-fade in `wav.rs` or applying edge fades only in mastering pre-normalization restores lossless PCM serialization.

4. **Why the new -1.0 dBFS (0.891) normalization and 0.20 stem headroom will pass existing tests**:
   - In `tape_mastering.rs`, `BRICKWALL_CEILING` is currently `0.966` (-0.30 dBFS).
   - Existing tests assert:
     - `assert!(peak <= 0.96605)` (e.g. `challenger_tier5_2_empirical_stress_tests.rs:43, 216`)
     - `assert!(s.abs() <= 0.966)` (e.g. `tests/tier1_features.rs:587`, `node_tests.rs:317`)
     - `assert!(master_tel.peak_dbfs <= -0.3)` (e.g. `tests/tier4_scenarios.rs:49`)
     - `assert!(tel.peak_dbfs <= 0.0)` (e.g. `tests/e2e_tests.rs:99`)
     - `assert!((tel.peak_dbfs - (-1.0)).abs() < 0.05)` (e.g. `dsp_tests.rs:121`)
   - Normalizing to -1.0 dBFS (peak 0.89125) produces a peak of 0.891, which is `< 0.966` and `<= -0.30 dBFS`.
   - Therefore, updating the master peak normalization to -1.0 dBFS (0.891) and scaling stems by 0.20 preserves 100% compliance with both the new requirement R2 and all existing ceiling assertions.

---

## 3. Caveats

1. **Python / uv Subprocess Environment**:
   - Offline / dry-run test execution on developer machines without Hugging Face models will route through pure-Rust fallback. This is the intended graceful fallback behavior defined in R3 and `SystemMemory::route_stage`.
2. **Windows Speech Synthesis (SAPI) Temporary Files**:
   - `_sapi_*.wav`, `_sapi_script.ps1`, and `_sapi_lyrics.txt` are created when `System.Speech` is invoked. If kept in `stems/`, `validate_disk_songs.py` ignores them as long as the 10 canonical stems are present. However, clean regenerations should clean up temporary SAPI scripts.
3. **`song_reggae_anthem_001` Metadata**:
   - In `storage/songs/song_reggae_anthem_001/v1` and `v2`, previous runs wrote `roots_reggae_anthem` into the `song_id` field of `manifest.json`. During regeneration, `--song-id song_reggae_anthem_001` must be passed to ensure `manifest.json` and `state.json` have matching `song_id`.

---

## 4. Conclusion

1. **Current State**:
   - The workspace test harness across `music_core`, `music_fusion`, `music_pipeline`, and `music_e2e_tests` is overwhelmingly green (321 passing tests).
   - Only two test failures exist in the current codebase:
     1. `music_dsp`: `test_adversarial_wav_roundtrip_all_formats_extreme_values` caused by 4ms edge-fading in `wav.rs`.
     2. `music_cli`: `test_disk_song_artifacts_roots_reggae_and_one_love` caused by duration discrepancy in `roots_reggae_anthem/v1` (106.95s vs expected 75.78s).
   - `validate_disk_songs.py` fails on missing `"lyrics"` in `roots_reggae_anthem/v1/manifest.json`.
2. **Required Pipeline Changes (for Worker)**:
   - In `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`:
     1. Add `"lyrics": state.lyrics` into `manifest.json`.
     2. Scale each individual stem by `0.20` (-14 dBFS RMS headroom) before summing into `inst_bus`.
     3. Apply 12 kHz Biquad 2nd-order Butterworth low-pass filtering, `f32::tanh(sample * 1.2)` tape saturation, and peak normalization to `-1.0 dBFS` (0.89125).
   - In `crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs`:
     1. Scale each individual stem by `0.20` in `assemble_composite_mix`.
   - In `crates/music_dsp/src/wav.rs`:
     1. Remove disruptive `edge_fade_frames` from `to_wav_bytes_with_format` so PCM roundtrips are bit-accurate.
3. **Sample Songs Regeneration Matrix**:
   - Regenerate all 4 sample songs (v1 and v2) via the CLI pipeline with the new pure-Rust physical modeling engine (Karplus-Strong, Reggae One-Drop kick pitch drop 110Hz->42Hz, 6kHz+ metallic hi-hats, 180Hz LP dub sub-bass, Hammond organ drawbars + 6Hz rotary LFO).
   - Ensure v1 durations are 75.78s (for 76 BPM songs) and v2 durations are 73.84s (for 78 BPM songs), or 103.65s / 53.33s for Afrobeats.

---

## 5. Verification Method

### Step 1: Core DSP & Serialization Verification
Run the unit and adversarial tests for `music_dsp`:
```powershell
cargo test -p music_dsp
```
*Expected Result*: All tests pass, including `test_adversarial_wav_roundtrip_all_formats_extreme_values`.

### Step 2: Regenerate All 4 Stored Sample Songs (v1 and v2)
Execute the CLI pipeline for each sample song to write updated stems and `final_master.wav`:

1. **`roots_reggae_anthem`**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --song-id roots_reggae_anthem --bpm 76 --key "A Major" --pure-rust --no-browser
   cargo run -p music_cli -- --iterate "Boost guitar wah and sub-bass warmth" --song-id roots_reggae_anthem --version 1 --bpm 78 --pure-rust --no-browser
   ```

2. **`song-one-love-shining-1977`**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --song-id song-one-love-shining-1977 --bpm 76 --key "A Major" --pure-rust --no-browser
   cargo run -p music_cli -- --iterate "Boost guitar solo wah & sustain in [Guitar Solo]" --song-id song-one-love-shining-1977 --version 1 --bpm 78 --pure-rust --no-browser
   ```

3. **`song_afrobeats_002`**:
   ```powershell
   cargo run -p music_cli -- --create-hit --genre afrobeats --song-id song_afrobeats_002 --bpm 104 --key "C Major" --pure-rust --no-browser
   cargo run -p music_cli -- --iterate "Boost Amapiano log-drum sub-bass +3.5dB, add saxophone horn counter-melody, and accelerate groove to 108 BPM" --song-id song_afrobeats_002 --version 1 --bpm 108 --pure-rust --no-browser
   ```

4. **`song_reggae_anthem_001`**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --song-id song_reggae_anthem_001 --bpm 76 --key "A Major" --pure-rust --no-browser
   cargo run -p music_cli -- --iterate "Boost guitar solo wah and enhance sub-bass warmth" --song-id song_reggae_anthem_001 --version 1 --bpm 78 --pure-rust --no-browser
   ```

### Step 3: Run Python Empirical Disk Song Validation
Validate all 8 versions (11 WAV files each, 48 kHz stereo, true-peak <= 0.966, stem peak <= 1.0, multi-section lyrics, zero UTF-8 BOM):
```powershell
python tests/validate_disk_songs.py
```
*Expected Result*: `RESULT: 100% EMPIRICAL VALIDATION PASSED!` (8 versions checked, 88 WAV files checked).

### Step 4: Run Rust CLI Disk Song Artifacts Test
```powershell
cargo test -p music_cli --test challenger_tier5_2_empirical_stress_tests test_disk_song_artifacts_roots_reggae_and_one_love
```
*Expected Result*: `test test_disk_song_artifacts_roots_reggae_and_one_love ... ok`.

### Step 5: Full Workspace Test Verification
Execute the entire workspace test battery:
```powershell
cargo test -p music_core
cargo test -p music_dsp
cargo test -p music_fusion
cargo test -p music_pipeline
cargo test -p music_cli
cargo test --manifest-path tests/Cargo.toml
```
*Expected Result*: All test suites pass with 0 failures across all crates.
