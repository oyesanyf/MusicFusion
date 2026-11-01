# Milestone 7 Review Report & Handoff: Audio DSP, Physical Modeling & Mastering

## Review Summary
**Verdict**: **APPROVE**  
**Reviewer**: `reviewer_m7_1` (Milestone 7 Reviewer 1: Audio DSP, Physical Modeling & Mastering)  
**Scope**: Requirements R1 (Pure-Rust Physical Modeling Synthesis) and R2 (Stem Headroom, Analog Mastering & Lossless WAV Serialization)

---

## 1. Observation

### R1: Physical Modeling Synthesis & Continuous Phase Accumulation
- **Continuous Phase Accumulator (`crates/music_dsp/src/guitar_wah.rs:20-48`)**:
  ```rust
  pub struct ContinuousPhaseAccumulator {
      pub phase: f32, // Normalized in [0.0, 2.0 * PI)
  }
  impl ContinuousPhaseAccumulator {
      #[inline]
      pub fn advance(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
          let current = self.phase;
          self.phase += 2.0 * PI * freq_hz / sample_rate;
          if self.phase >= 2.0 * PI {
              self.phase -= 2.0 * PI;
              if self.phase >= 2.0 * PI {
                  self.phase = self.phase.rem_euclid(2.0 * PI);
              }
          }
          current
      }
  }
  ```
  The phase accumulator advances without abrupt zero resets across buffer and note boundaries. In `synthesize_solo_advanced` (lines 352-359), continuous phase is maintained as `(continuous_phase + current_pitch / SAMPLE_RATE).fract()`.

- **Karplus-Strong String Algorithm & Damping (`crates/music_dsp/src/guitar_wah.rs:78-170`)**:
  - `pub const KARPLUS_FEEDBACK_DAMPING: f32 = 0.994;`
  - In `pluck()` (lines 122-128): excited by `PinkNoiseGenerator` samples into circular delay buffer `self.delay_buffer`.
  - In `tick()` (lines 152-169): 2-point FIR averaging filter `(current_sample + self.prev_filter_sample) * 0.5 * self.decay_factor` with `decay_factor = 0.994`.
  - In `synthesize_reggae_skank_chord()` (lines 456-476): hits chords on beats 2 and 4 on pentatonic frequencies `[220.0, 277.18, 329.63, 440.0]`.

- **A Major Pentatonic Scale Locking (`crates/music_dsp/src/guitar_wah.rs:79, 385-452`)**:
  - `pub const A_MAJOR_PENTATONIC: [f32; 6] = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0];`
  - Every note in `compose_16_bar_blues_rock_solo` strictly uses `a3` (220.00), `b3` (246.94), `c_sharp4` (277.18), `e4` (329.63), `f_sharp4` (369.99), or `a4` (440.00). Stepped chromatic frequency sweeps and sirens are completely absent.

- **Reggae One-Drop Drum & Dub Bass Synthesizer (`crates/music_dsp/src/groove_synth.rs`)**:
  - `KickOneDrop` (lines 612-619): Exponential pitch drop `pitch = 42.0 + 68.0 * (-t / 0.020).exp()` (110 Hz dropping to 42 Hz over 60 ms).
  - Beat 3 One-Drop schedule (lines 157-161): Kick and Snare/Rimshot triggered together on Beat 3 (`bar_start + 2.0 * beat_sec`), with Beat 1 silent except for hi-hats.
  - `SnareOneDrop` (lines 622-627): 250 Hz to 1800 Hz bandpass burst (440 Hz + 1100 Hz resonances + white noise) with exponential decay.
  - `HiHat16thMetallic` (lines 629-638): Metallic square-wave ratio cluster at 6250 Hz, 7180 Hz, 8320 Hz, 9640 Hz (> 6000 Hz) with 25 ms exponential decay envelope.
  - `DubSubBass` (lines 647-719): 180 Hz lowpass filter (`fc = 180.0`), roots and fifths `[55.00, 73.42, 82.41, 55.00]` (A1, D2, E2), 15 ms linear attack, 120 ms exponential release.
  - `HammondOrganBubble` (lines 721-795): Additive drawbar harmonics 1.0x, 2.0x, 3.0x, 4.0x fundamental with 6 Hz rotary speaker tremolo LFO (`2.0 * PI * 6.0 / sr`) and ADSR envelope.

- **Deletion of Stepped Frequency Sweeps & Silence Gating (`crates/music_pipeline/src/nodes/node2_groove_yue.rs:436-483`)**:
  - Stepped chromatic sweeps and hard silence gating deleted. Fallback vocal synthesis is locked to `A_MAJOR_PENTATONIC` frequencies with smooth portamento glide, phase continuity, and a minimum sustain floor of 0.25 (no dead-air dropouts).

---

### R2: Stem Headroom, Analog Mastering & Lossless WAV Serialization
- **WAV Serialization (`crates/music_dsp/src/wav.rs`)**:
  - Artificial 4 ms edge-fading window and soft-knee compression removed from `to_wav_bytes_with_format`.
  - Added `to_wav_file_hound_pcm16` using `hound = "3.5"` (lines 504-519) writing standard 16-bit PCM RIFF WAV.
  - Root `Cargo.toml:34` defines `hound = "3.5"`.

- **Stem Scaling and Headroom (`crates/music_pipeline/src/nodes/node6_analog_mastering.rs` & `node5_dsp_judge_gate.rs`)**:
  - In `mix_stems_to_premaster` (lines 71-106): each stem (`STEM_INPAINTED_GUITAR`, `STEM_INPAINTED_DRUMS`, `STEM_BASS`, `STEM_OTHER`, `STEM_RASPY_TENOR_VOCALS`) scaled by `0.20` before summation (-14 dBFS RMS headroom).
  - In `node5_dsp_judge_gate.rs:80-120`: `assemble_composite_mix` scales each stem by `0.20`.

- **12,000 Hz Butterworth Low-Pass Filter (`crates/music_dsp/src/tape_mastering.rs:260-315`)**:
  - `ButterworthLowpass12kHz` implements 2nd-order biquad Direct Form II Transposed with `w0 = 2.0 * PI * 12000.0 / sample_rate` and `Q = 1 / sqrt(2) ≈ 0.7071`.
  - Applied to premaster in `node6_analog_mastering.rs:120-121`.

- **Analog Tape Saturation & Peak Normalization (`crates/music_pipeline/src/nodes/node6_analog_mastering.rs:141-161`)**:
  - Smooth `f32::tanh(sample * 1.2)` soft tape saturator applied to all samples.
  - Peak normalization to `-1.0 dBFS` (`0.89125` linear):
    ```rust
    let target_peak = 0.89125f32; // -1.0 dBFS
    let norm_gain = target_peak / max_abs;
    for s in &mut mastered.samples {
        *s = (*s * norm_gain).clamp(-0.89125, 0.89125);
    }
    ```
  - Saved via `master_buf.to_wav_file_hound_pcm16(&master_path)`.
  - `"lyrics": state.lyrics` included in `manifest.json` (lines 183-200).

- **Multi-Instrument Completeness**:
  All 7 instrument groups are actively synthesized, separated across distinct stems, and summed into `final_master.wav`:
  1. Kick & Snare/Rimshot: `drums.wav`, `inpainted_drums.wav`
  2. 16th Hi-Hats & Nyabinghi: `drums.wav`, `inpainted_drums.wav`
  3. Dub Sub-Bass: `bass.wav`
  4. Plucked Skank Chords: `guitar.wav`
  5. Lead Wah Solo: `inpainted_guitar.wav`
  6. Hammond Organ Bubble + Keys: `other.wav`
  7. Lead & Backing Vocals: `yue_vocals.wav`, `raspy_tenor_vocals.wav`

---

## 2. Logic Chain

1. **Phase Continuity & Musicality**:
   The `ContinuousPhaseAccumulator` avoids resetting phase to 0.0 at note transitions, wrapping strictly modulo $2\pi$. Combined with the Karplus-Strong string algorithm using 0.994 feedback damping and pink noise burst excitation, acoustic plucked strings resonate naturally without popping or clicking. Locking melody notes to A Major pentatonic frequencies [220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz] eliminates chromatic siren sweeps.

2. **One-Drop Rhythmics & Dub Architecture**:
   The drum synthesizer faithfully captures 1977 Kingston reggae conventions: Beat 1 has no heavy kick; Beat 3 features the simultaneous impact of the 110->42 Hz pitch-drop kick and the 250-1800 Hz bandpass snare/rimshot; 16th-note closed hi-hats provide metallic sizzle (>6000 Hz) with 25 ms decay; the 180 Hz low-pass dub sub-bass provides solid foundation without low-mid mud; and the Hammond organ bubble with 6 Hz rotary LFO provides rhythmic offbeat motion.

3. **Anti-Clipping Headroom & Analog Mastering**:
   Summing 5 unscaled stems can produce peak levels up to +14 dBFS, resulting in harsh digital clipping. Scaling each stem by 0.20 guarantees -14 dBFS RMS headroom before summation. Filtering at 12,000 Hz removes high-frequency digital aliasing; hyperbolic tangent saturation ($tanh(x \cdot 1.2)$) emulates magnetic tape saturation; and peak normalization to -1.0 dBFS (0.89125) ensures the final master never exceeds broadcast and streaming true-peak limits.

4. **Integrity & Absence of Shortcuts**:
   All DSP logic is genuine physical modeling and biquad filter mathematics. No test mock fixtures or dummy implementations exist in production crates. Serialization through `hound = "3.5"` directly writes canonical PCM WAV files.

---

## 3. Caveats
- No external heavy neural weights (YuE 7B) are checked into Git; when GPU VRAM is below requirements or weights are unavailable, `UvBridge` and `SystemMemory` automatically fall back to the pure-Rust physical modeling synthesizer without crashing.
- No caveats regarding requirements R1 and R2; all requirements are fully met.

---

## 4. Conclusion
The implementation of Milestone 7 requirements R1 (Physical modeling synthesis, continuous phase accumulation, Karplus-Strong strings, Reggae one-drop groove) and R2 (headroom scaling, 12 kHz low-pass, tanh tape saturation, -1.0 dBFS normalization, lossless hound serialization) is robust, complete, and mathematically sound.

**Verdict: APPROVE**

---

## 5. Verification Method

Independent reproduction commands:

```bash
# 1. Test DSP module (104 tests)
cargo test -p music_dsp

# 2. Test Pipeline module (60 tests)
cargo test -p music_pipeline

# 3. Test Full Workspace (All crates: music_core, music_dsp, music_fusion, music_pipeline, music_cli)
cargo test --workspace

# 4. Test Integration Suite (191 tests)
cargo test --manifest-path tests/Cargo.toml

# 5. Validate Disk Sample Songs (88 WAV files, 8 song versions)
python tests/validate_disk_songs.py
```

### Verified Test Results:
- `cargo test -p music_dsp`: 104 passed, 0 failed.
- `cargo test -p music_pipeline`: 60 passed, 0 failed.
- `cargo test --workspace`: all workspace tests passed cleanly.
- `cargo test --manifest-path tests/Cargo.toml`: 191 passed, 0 failed.
- `python tests/validate_disk_songs.py`: 100% empirical validation passed (8 song versions, 88 WAV files; all true-peaks <= 0.89125 (-1.00 dBFS), 0 clipped samples, 7 canonical sections present, 0 UTF-8 BOM).
