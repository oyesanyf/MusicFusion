# Forensic Integrity Audit Report: Milestone 7 Recheck & Melodic Singing Upgrade

**Auditor**: `auditor_m7_recheck`  
**Milestone**: Milestone 7 (Remediation Recheck & Melodic Singing Upgrade)  
**Integrity Mode**: Development (per `ORIGINAL_REQUEST.md` line 8)  
**Verdict**: **CLEAN**

---

## 1. Observation

Direct empirical observations and code analysis:

### A. Zero Mocks & Authentic DSP Physical Modeling (`no_mocks_write_actual_code.md`)
1. **Karplus-Strong String Oscillator (`crates/music_dsp/src/guitar_wah.rs` lines 78–170)**:
   - Delay-line ring buffer (`KarplusStrongString`) excited by Paul Kellet's filtered pink noise burst (`PinkNoiseGenerator`).
   - Constant `KARPLUS_FEEDBACK_DAMPING = 0.994` exactly matches specification.
   - Lowpass averaging filter `(current_sample + prev_filter_sample) * 0.5 * decay_factor` ensures BIBO asymptotic stability without mock sample buffers.
   - Continuous phase accumulator (`ContinuousPhaseAccumulator`, lines 22–48) advances via `phase += 2.0 * PI * freq / sample_rate` across buffer boundaries with zero abrupt phase resets to 0.0 mid-waveform.
   - Scale locking: Restricted to A Major Pentatonic frequencies (`A_MAJOR_PENTATONIC = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0]`), with zero chromatic siren sweeps (pitch bend semitones strictly 0.0 in reggae presets).
2. **Reggae One-Drop Drums & Dub Sub-Bass (`crates/music_dsp/src/groove_synth.rs` lines 146–180, 610–795)**:
   - Kick Drum on Beat 3 (`DrumVoice::KickOneDrop`): Rapid exponential pitch-drop sine wave `pitch = 42.0 + 68.0 * (-t / 0.020).exp()` dropping 110 Hz down to 42 Hz over 60 ms, blended with a band-passed white noise burst and tanh saturation.
   - Rimshot / Snare on Beat 3 (`DrumVoice::SnareOneDrop`): 250 Hz to 1800 Hz bandpass filtered white noise burst with exponential decay.
   - Metallic Hi-Hats (`DrumVoice::HiHat16thMetallic`): High-pass filtered metallic square-wave ratios (6250 Hz, 7180 Hz, 8320 Hz, 9640 Hz > 6000 Hz) with a tight 25 ms exponential decay envelope.
   - Dub Sub-Bass (`DubSubBass`): 180 Hz single-pole lowpass filter (`dt / (rc + dt)`), sine (0.70) + triangle (0.30) oscillator walking roots and fifths (55.0 Hz, 73.42 Hz, 82.41 Hz) with 15 ms linear attack and 120 ms exponential release.
   - Hammond Organ Bubble (`HammondOrganBubble`): Additive drawbars (1.0x, 2.0x, 3.0x, 4.0x) with a 6 Hz rotary speaker tremolo LFO (`1.0 + 0.18 * sin(2*pi*6.0*t)`) and ADSR envelope.
3. **Mastering Headroom & Analog Tape Chain (`crates/music_dsp/src/tape_mastering.rs` & `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`)**:
   - Stems scaled by `0.20` (-14 dBFS headroom) before bus summation.
   - 2nd-order Butterworth low-pass filter at 12,000 Hz (`ButterworthLowpass12kHz`, Q = 0.7071) eliminates harsh digital aliasing.
   - Smooth `f32::tanh(sample * 1.2)` analog tape saturation.
   - Peak normalization to strictly -1.0 dBFS (`0.89125`) with lookahead brickwall peak limiter (`BRICKWALL_CEILING = 0.966`).
4. **Authentic Melodic Singing Synthesis (`crates/music_dsp/src/melodic_singing.rs` lines 1–385)**:
   - Replaces Windows desktop screen-reader TTS (`Microsoft David Desktop` / `System.Speech`) with dedicated pure-Rust 48 kHz melodic singing.
   - Formant filter banks (`FormantFilterBank`), glottal excitation pulses (`glottal_pulse`), natural 5.5 Hz vibrato with dynamic onset swelling to 1.5%, scale-degree quantization, 3-part harmony choir (Root + Major 3rd + Perfect 5th), 12 ms stereo doubler, and Schroeder studio reverb (4 parallel damped comb filters + 2 series allpass diffusers).
5. **Open Hugging Face Model Discovery & WebUI (`hf_model_discovery.rs`, `generate_song.py`, `index.html`, `app.js`)**:
   - Curated built-in catalog and dynamic query engine features the top 7 open models: `ASLP-lab/DiffRhythm` (Apache 2.0), `ACE-Step/ACE-Step-v1-3.5B` (Apache 2.0), `m-a-p/YuE-s1-7B-anneal-en-cot` (Apache 2.0), `suno/bark` (MIT), `fishaudio/fish-speech-1.5` (Apache 2.0), `Plachtaa/Seed-VC` (MIT), and `facebook/musicgen-medium`.
   - `workers/generate_song.py` catches general `Exception` (including Windows C-extension DLL import errors) and exits cleanly with code 2.
   - WebUI features dynamic model selection and two-way sync between settings modal and drawer.

### B. Zero-Touch PATH & Environment Protection (`environment_safety.md`)
- `git grep -i "SetEnvironmentVariable"`: 0 occurrences in executable code (only in documentation and auditor reports).
- `git grep -i "reg add"`: 0 occurrences.
- `git grep -i "setx"`: 0 occurrences.
- `git grep -i "HKCU"` / `HKLM`: 0 registry write calls. The sole registry invocation in `rest_server.rs:1167` is a read-only query (`reg query HKLM\SYSTEM\CurrentControlSet\Enum\SWD\MMDEVAPI /s /v FriendlyName`) with `CREATE_NO_WINDOW = 0x08000000` to list audio playback/capture devices for the WebUI.
- Zero process, user, or machine `PATH` mutations.

### C. Empirical Audio Waveform Validation (88 on-disk WAV files)
1. `python tests/audit_stems_non_silence.py`:
   - All 56 stems across all 8 song versions verified active with RMS energy > -50.0 dBFS (ranging from -8.86 dBFS to -42.47 dBFS) and zero digital clipping (peak <= 1.0).
   - Remediated Afrobeats `drums.wav`: v1 RMS = -15.03 dBFS (peak -1.11 dBFS); v2 RMS = -14.42 dBFS (peak -1.15 dBFS) — fully active and audible.
   - Remediated melodic singing vocals: `yue_vocals.wav` and `raspy_tenor_vocals.wav` RMS = -8.86 to -10.36 dBFS, peak = -1.11 to -1.41 dBFS (loud, clear, unclipped).
2. `python tests/validate_disk_songs.py`:
   - 88/88 WAV files validated at 48 kHz stereo, unclipped, true-peaks <= 0.89124 (-1.0 dBFS).
   - Multi-section English lyrics verified across all 3 metadata formats (`lyrics.txt`, `manifest.json`, `state.json`) with all 7 structural sections present.
   - Zero UTF-8 BOM across all JSON and text files.

### D. Automated Test Execution
1. `cargo test --workspace`:
   - `Finished test profile in 0.51s`.
   - All test suites across `music_cli`, `music_core`, `music_dsp`, `music_fusion`, and `music_pipeline` passed: **0 failed, 0 ignored**.
2. `cargo test --manifest-path tests/Cargo.toml`:
   - `running 191 tests`
   - `test result: ok. 191 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s`.

---

## 2. Logic Chain

1. **Anti-Mock / Facade Verification**:
   Inspection of `crates/music_dsp/src/guitar_wah.rs`, `groove_synth.rs`, `tape_mastering.rs`, and `melodic_singing.rs` confirmed that all signal processing is performed via genuine numerical math: discrete ring-buffer circular delay lines, pink noise filtering, Chamberlin state-variable filters, hyperbolic tangent soft-knee saturation, and Biquad filtering. There are no static hardcoded sample tables, dummy return constants, or facade test assertions.
2. **Vocal Quality & Musicality Upgrade**:
   The flat screen-reader speech engine (`Microsoft David Desktop` / `System.Speech`) has been completely decoupled from the production pipeline. In its place, `MelodicSingingSynthesizer` generates authentic sung vocal tracks driven by scale-degree frequency quantization, glottal pulse excitation, natural 5.5 Hz vibrato, 3-part harmony choir voicing, and Schroeder reverberation. All stored song vocal stems reflect this musical upgrade.
3. **All-Instrument Headroom & Non-Silence Remediations**:
   The drum separation defect in `node4_stem_inpainting.rs` (which previously filtered Afrobeats percussion with a 3500 Hz highpass) was replaced by a 3-band percussion filter (120 Hz punch, 1100 Hz body, 2500 Hz sizzle) combined with active groove blending. As verified empirically by `audit_stems_non_silence.py`, all 56 stems across all 8 song versions exceed the -50.0 dBFS RMS threshold with zero clipping.
4. **Environment & PATH Safety**:
   Exhaustive static grep scans and code audits confirm zero registry writes, zero PATH manipulations, and zero `setx` calls across the entire repository.
5. **Empirical Test Verification**:
   100% of unit tests, integration tests, E2E tests, and Python audio analysis scripts execute and pass without errors.

---

## 3. Caveats

- No caveats. All requirements from `ORIGINAL_REQUEST.md`, follow-up requirements, and dispatch instructions have been verified empirically with raw tool execution proof.

---

## 4. Conclusion

The work product passes all forensic integrity checks:
- Pure-Rust physical modeling DSP is 100% genuine signal processing without fake mocks or dummy test constants.
- Vocal synthesis is authentic melodic singing locked to musical scale degrees and rhythm, with robotic screen-reader TTS completely eradicated.
- Hugging Face model discovery, Python bridge, and WebUI correctly expose the top 7 open-source models.
- All 88 on-disk WAV files contain authentic, audible, non-silent, and unclipped 48 kHz stereo audio.
- Zero-Touch PATH and environment safety invariants are 100% respected.
- Workspace and E2E tests pass 100%.

**Forensic Verdict**: **CLEAN**.

---

## 5. Verification Method

To independently reproduce and verify this audit:

1. **Run stem non-silence & RMS energy audit**:
   ```powershell
   python tests/audit_stems_non_silence.py
   ```
   *Expected output*: `AUDIT PASSED: All 56 stems have robust RMS energy (> -50.0 dBFS) and zero clipping!`

2. **Run on-disk song & WAV validator**:
   ```powershell
   python tests/validate_disk_songs.py
   ```
   *Expected output*: `RESULT: 100% EMPIRICAL VALIDATION PASSED! Checked 8 song versions, 88 total WAV files.`

3. **Run cargo workspace test suite**:
   ```powershell
   cargo test --workspace
   ```
   *Expected output*: All test binaries pass with 0 failures across all crates.

4. **Run cargo end-to-end integration test suite**:
   ```powershell
   cargo test --manifest-path tests/Cargo.toml
   ```
   *Expected output*: `191 passed; 0 failed; 0 ignored; 0 measured; finished in ~0.3s.`
