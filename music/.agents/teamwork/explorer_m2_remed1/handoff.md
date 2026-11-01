# Remediation Analysis Report: Crossfader Overflow & Groove Synth Mono Buffer Panic

- **Agent**: explorer_m2_remed1
- **Role**: Remediation Explorer (M2)
- **Target**: Milestone 2 Pure-Rust 48 kHz DSP Engine (`crates/music_dsp`)
- **Date**: 2026-11-01
- **Status**: Investigation Complete — Ready for Worker Implementation

---

## 1. Observation

Direct code inspection and test execution were performed on `crates/music_dsp` and the adversarial test suite `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`.

### Observation 1: Integer Multiplication Overflow in `crossfader.rs:108-109` on `bpm == 0`

- **Location**: `crates/music_dsp/src/crossfader.rs:108-111`
  ```rust
  let samples_per_beat = (60.0 / bpm as f32 * SAMPLE_RATE as f32).round() as usize;
  let samples_per_bar = samples_per_beat * beats_per_bar as usize;
  let start_frame = start_bar as usize * samples_per_bar;
  let xfade_frames = (crossfade_ms / 1000.0 * SAMPLE_RATE as f32).round() as usize;
  ```
- **Observed Trigger**: Calling `StemCrossfader::inpaint_bars(&base, &splice, 0, 0, 4, 15.0, CrossfadeCurve::EqualPowerSine)` or `splice_stem(&base, &splice, 0, 0, 4)`.
- **Verbatim Error & Failure Mode**:
  ```
  thread 'test_adversarial_crossfader_zero_bpm_multiplication_overflow' panicked at crates\music_dsp\src\crossfader.rs:109:31:
  attempt to multiply with overflow
  ```
- **Root Cause**:
  When `bpm == 0`, `60.0 / 0.0 * 48000.0` produces `f32::INFINITY`.
  In Rust RFC 2018 saturating float-to-int cast semantics, casting `f32::INFINITY as usize` saturates to `usize::MAX` (`18_446_744_073_709_551_615` on 64-bit platforms).
  Then at line 109: `samples_per_beat * beats_per_bar as usize` computes `usize::MAX * 4`.
  Under Rust debug overflow checks (and standard numeric safety), this triggers an unhandled multiplication overflow panic. Furthermore, `start_bar as usize * samples_per_bar` is susceptible to a secondary overflow panic when `start_bar > 0`.

### Observation 2: Slice Index Out of Bounds in `groove_synth.rs:412-413` on Mono Buffer

- **Location**: `crates/music_dsp/src/groove_synth.rs:408-415`
  ```rust
  for f in start_frame..end_frame {
      let t = (f - start_frame) as f32 / SAMPLE_RATE as f32;
      let sample_val = Self::synthesize_sample(hit.voice, t, hit.velocity, &mut rng);

      buffer.samples[f * 2] = (buffer.samples[f * 2] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
      buffer.samples[f * 2 + 1] = (buffer.samples[f * 2 + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
  }
  ```
- **Observed Trigger**: Calling `UniversalDrumSynthesizer::render_hits(&hits, &mut mono_buf)` where `mono_buf` is a mono `AudioBuffer` (`channels == 1`, e.g. `AudioBuffer::new_mono(1.0)`) and drum hit frame `f >= mono_buf.frame_count() / 2`.
- **Verbatim Error & Failure Mode**:
  ```
  thread panicked at crates\music_dsp\src\groove_synth.rs:413:21:
  index out of bounds: the len is 48000 but the index is 76801
  ```
- **Root Cause**:
  `UniversalDrumSynthesizer::render_hits` unconditionally hardcodes stereo channel indexing (`f * 2` and `f * 2 + 1`).
  For a mono buffer with 48,000 frames, `buffer.samples.len() == 48000 * 1 = 48000`.
  A drum hit at `time_secs = 0.8` starts at frame `38400`. For `f = 38400`, `f * 2 + 1 = 76801`.
  Because `76801 > 48000`, indexing into `buffer.samples` panics with slice index out of bounds.

---

## 2. Logic Chain

1. **Musical Tempo Clamping Invariant**:
   - In `PROJECT.md` §2 line 62, the supported tempo range is explicitly declared as `40–220 BPM`.
   - In `crates/music_dsp/src/groove_synth.rs` lines 86, 93, 99 and `crates/music_dsp/src/guitar_wah.rs` lines 218, 232, the standard defensive pattern is `let clamped_bpm = bpm.clamp(40, 220);`.
   - Applying `let active_bpm = bpm.clamp(40, 220);` inside `StemCrossfader::inpaint_bars` guarantees that `active_bpm >= 40`.
   - Consequently, `60.0 / active_bpm as f32 * 48000.0` is strictly bounded between `13091` samples (at 220 BPM) and `72000` samples (at 40 BPM). `f32::INFINITY` and `usize::MAX` cannot occur.

2. **Defensive Bar & Frame Math**:
   - `beats_per_bar` should be guarded via `let active_beats = beats_per_bar.max(1);` to avoid zero-beat degenerate bar calculations.
   - Using saturating arithmetic `samples_per_beat.saturating_mul(active_beats as usize)` and `(start_bar as usize).saturating_mul(samples_per_bar)` guarantees that even absurdly high adversarial inputs (`u32::MAX`) will saturate safely rather than panicking.
   - In `StemCrossfader::inpaint_splice`, the boundary check `if start_frame >= total_frames || splice_frames == 0 { return out; }` safely handles saturated frame offsets by returning the base buffer unmodified.
   - `crossfade_ms` must be validated against `NaN` or negative values: `let valid_ms = if crossfade_ms.is_nan() || crossfade_ms < 0.0 { 15.0 } else { crossfade_ms };`.

3. **Buffer Multi-Channel Invariant & Contract Preservation**:
   - `AudioBuffer` supports mono (`channels: 1`) and stereo (`channels: 2`).
   - `UniversalDrumSynthesizer::render_hits` accepts `&mut AudioBuffer`.
   - While `*buffer = buffer.to_stereo()` would eliminate the panic, mutating the buffer's channel layout and reallocating its sample buffer violates the caller's contract and POLA (Principle of Least Astonishment), especially when callers intentionally allocate mono buffers for mono stems or memory efficiency.
   - Instead, inspecting `let channels = buffer.channels as usize;` and branching allows native mono rendering:
     - When `channels == 1`:
       The stereo pan weights `pan_l = ((1.0 - hit.pan) * 0.5).sqrt()` and `pan_r = ((1.0 + hit.pan) * 0.5).sqrt()` downmix to mono via `pan_mono = (pan_l + pan_r) * 0.5`. This exactly mirrors `AudioBuffer::to_mono()` acoustic downmixing.
       The sample is written directly to `buffer.samples[f]`. Since `f < end_frame <= frames`, `f < buffer.samples.len()` is guaranteed.
     - When `channels >= 2`:
       The left channel is written to `buffer.samples[f * channels]` and right to `buffer.samples[f * channels + 1]`.
       Since `f < frames`, `f * channels + 1 < (f + 1) * channels <= frames * channels = buffer.samples.len()`.
     - Early return `if channels == 0 || frames == 0 || buffer.samples.is_empty() { return; }` provides complete fail-closed safety for empty or zero-channel buffers.

---

## 3. Caveats

1. **Read-Only Scope**:
   In strict compliance with the Teamwork explorer archetype, this agent did not modify any source code in `crates/music_dsp/src/*`. All findings and proposed code changes are documented below for `worker_m2`.
2. **Parallel Challenger Issues**:
   The two remaining issues flagged in `challenger_m2_1/handoff.md` (`phase_alignment` NaN propagation and `sample_rate == 0` division by zero in `telemetry_calc.rs`) are scoped to parallel remediation tasks.
3. **Multichannel Routing**:
   For channel counts > 2, channel 0 receives left and channel 1 receives right. Additional surround channels are left untouched, which is standard for 48 kHz stereo music mastering.

---

## 4. Conclusion & Actionable Fix Strategy

### Fix 1: `crates/music_dsp/src/crossfader.rs`

#### Target: `inpaint_bars` (lines 99–115)
Replace lines 107–114:

```rust
<<<<
    pub fn inpaint_bars(
        base: &AudioBuffer,
        splice: &AudioBuffer,
        start_bar: u32,
        bpm: u32,
        beats_per_bar: u32,
        crossfade_ms: f32,
        curve: CrossfadeCurve,
    ) -> AudioBuffer {
        let samples_per_beat = (60.0 / bpm as f32 * SAMPLE_RATE as f32).round() as usize;
        let samples_per_bar = samples_per_beat * beats_per_bar as usize;
        let start_frame = start_bar as usize * samples_per_bar;
        let xfade_frames = (crossfade_ms / 1000.0 * SAMPLE_RATE as f32).round() as usize;

        Self::inpaint_splice(base, splice, start_frame, xfade_frames, curve)
    }
====
    pub fn inpaint_bars(
        base: &AudioBuffer,
        splice: &AudioBuffer,
        start_bar: u32,
        bpm: u32,
        beats_per_bar: u32,
        crossfade_ms: f32,
        curve: CrossfadeCurve,
    ) -> AudioBuffer {
        let active_bpm = bpm.clamp(40, 220);
        let active_beats = beats_per_bar.max(1);
        let samples_per_beat = (60.0 / active_bpm as f32 * SAMPLE_RATE as f32).round() as usize;
        let samples_per_bar = samples_per_beat.saturating_mul(active_beats as usize);
        let start_frame = (start_bar as usize).saturating_mul(samples_per_bar);
        let valid_ms = if crossfade_ms.is_nan() || crossfade_ms < 0.0 { 15.0 } else { crossfade_ms };
        let xfade_frames = (valid_ms / 1000.0 * SAMPLE_RATE as f32).round() as usize;

        Self::inpaint_splice(base, splice, start_frame, xfade_frames, curve)
    }
>>>>
```

### Fix 2: `crates/music_dsp/src/groove_synth.rs`

#### Target: `render_hits` (lines 391–416)
Replace lines 391–416:

```rust
<<<<
    /// Renders drum hits into the stereo PCM AudioBuffer.
    pub fn render_hits(hits: &[DrumHit], buffer: &mut AudioBuffer) {
        let frames = buffer.frame_count();
        let mut rng = DspRng::new(0xABCD_1234);

        for hit in hits {
            let start_frame = (hit.time_secs * SAMPLE_RATE as f32).round() as usize;
            if start_frame >= frames {
                continue;
            }

            let voice_duration = Self::voice_duration_secs(hit.voice);
            let hit_frames = (voice_duration * SAMPLE_RATE as f32).round() as usize;
            let end_frame = (start_frame + hit_frames).min(frames);

            let pan_l = ((1.0 - hit.pan) * 0.5).sqrt();
            let pan_r = ((1.0 + hit.pan) * 0.5).sqrt();

            for f in start_frame..end_frame {
                let t = (f - start_frame) as f32 / SAMPLE_RATE as f32;
                let sample_val = Self::synthesize_sample(hit.voice, t, hit.velocity, &mut rng);

                buffer.samples[f * 2] = (buffer.samples[f * 2] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
                buffer.samples[f * 2 + 1] = (buffer.samples[f * 2 + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
            }
        }
    }
====
    /// Renders drum hits into the PCM AudioBuffer (supports mono and stereo).
    pub fn render_hits(hits: &[DrumHit], buffer: &mut AudioBuffer) {
        let frames = buffer.frame_count();
        let channels = buffer.channels as usize;
        if channels == 0 || frames == 0 || buffer.samples.is_empty() {
            return;
        }
        let mut rng = DspRng::new(0xABCD_1234);

        for hit in hits {
            let start_frame = (hit.time_secs * SAMPLE_RATE as f32).round() as usize;
            if start_frame >= frames {
                continue;
            }

            let voice_duration = Self::voice_duration_secs(hit.voice);
            let hit_frames = (voice_duration * SAMPLE_RATE as f32).round() as usize;
            let end_frame = (start_frame + hit_frames).min(frames);

            let pan_l = ((1.0 - hit.pan) * 0.5).sqrt();
            let pan_r = ((1.0 + hit.pan) * 0.5).sqrt();

            for f in start_frame..end_frame {
                let t = (f - start_frame) as f32 / SAMPLE_RATE as f32;
                let sample_val = Self::synthesize_sample(hit.voice, t, hit.velocity, &mut rng);

                if channels == 1 {
                    let pan_mono = (pan_l + pan_r) * 0.5;
                    buffer.samples[f] = (buffer.samples[f] + sample_val * pan_mono * hit.velocity).clamp(-0.95, 0.95);
                } else {
                    buffer.samples[f * channels] = (buffer.samples[f * channels] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
                    buffer.samples[f * channels + 1] = (buffer.samples[f * channels + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
                }
            }
        }
    }
>>>>
```

### Fix 3: Test Updates in `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`

After `worker_m2` applies the two patches above, update the three test functions in `adversarial_dsp_stress_tests.rs` (lines 86–114 and lines 253–271):

1. **Lines 86–99**:
   ```rust
   #[test]
   fn test_adversarial_crossfader_zero_bpm_multiplication_overflow() {
       let base = AudioBuffer::new_stereo(2.0);
       let splice = AudioBuffer::new_stereo(0.5);

       // Remediated: inpaint_bars clamps 0 BPM to [40, 220] safely without panic
       let result = std::panic::catch_unwind(|| {
           StemCrossfader::inpaint_bars(&base, &splice, 0, 0, 4, 15.0, CrossfadeCurve::EqualPowerSine)
       });
       assert!(result.is_ok(), "inpaint_bars should not panic when bpm == 0");
       let out = result.unwrap();
       assert_eq!(out.frame_count(), base.frame_count());
       assert_eq!(out.channels, 2);
   }
   ```

2. **Lines 101–114**:
   ```rust
   #[test]
   fn test_adversarial_splice_stem_zero_bpm() {
       let base = AudioBuffer::new_stereo(2.0);
       let splice = AudioBuffer::new_stereo(0.5);

       // Remediated: splice_stem delegates to inpaint_bars safely without panic
       let result = std::panic::catch_unwind(|| {
           splice_stem(&base, &splice, 0, 0, 4)
       });
       assert!(result.is_ok(), "splice_stem should not panic when bpm == 0");
       let out = result.unwrap();
       assert_eq!(out.frame_count(), base.frame_count());
       assert_eq!(out.channels, 2);
   }
   ```

3. **Lines 253–271**:
   ```rust
   #[test]
   fn test_adversarial_render_hits_mono_buffer_panic() {
       let mut mono_buf = AudioBuffer::new_mono(1.0);
       let hits = vec![DrumHit {
           time_secs: 0.8,
           voice: DrumVoice::KickAcoustic,
           velocity: 0.8,
           pan: 0.0,
       }];

       // Remediated: render_hits supports mono buffers safely without index panic
       let result = std::panic::catch_unwind(move || {
           UniversalDrumSynthesizer::render_hits(&hits, &mut mono_buf);
           mono_buf
       });
       assert!(result.is_ok(), "render_hits should not panic on mono buffer");
       let buf = result.unwrap();
       assert_eq!(buf.channels, 1);
       assert_eq!(buf.frame_count(), 48000);
       assert!(buf.peak_level() > 0.0);
   }
   ```

---

## 5. Verification Method

To independently verify this remediation plan:

1. **Verify Baseline State**:
   ```powershell
   cargo test -p music_dsp --test adversarial_dsp_stress_tests
   ```
   Confirm all 29 tests currently pass (under the pre-fix assertion `result.is_err()` documenting the bugs).

2. **Verify Implementation Steps**:
   Apply the edits in `crossfader.rs`, `groove_synth.rs`, and the updated test assertions in `adversarial_dsp_stress_tests.rs`.

3. **Execute Clean Compilation & Verification**:
   ```powershell
   cargo check --workspace
   cargo test -p music_dsp
   cargo test --workspace
   ```
   - Assert exit code 0.
   - Assert zero compilation warnings.
   - Assert 29/29 adversarial stress tests pass, with `inpaint_bars(0 BPM)`, `splice_stem(0 BPM)`, and `render_hits(mono)` succeeding gracefully without panics.
