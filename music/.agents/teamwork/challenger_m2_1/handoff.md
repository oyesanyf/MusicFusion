# Adversarial Challenge & Stress-Testing Report: Milestone 2 Pure-Rust 48 kHz DSP Engine

- **Agent**: challenger_m2_1
- **Role**: Empirical Challenger (critic, specialist)
- **Target**: `crates/music_dsp` (Milestone 2 implementation by `worker_m2`, commit `23b44f1`)
- **Date**: 2026-11-01
- **Definitive Verdict**: **REJECT** (Blocking until 4 empirical edge-case vulnerabilities are remediated)

---

## 1. Observation

Direct empirical observations were gathered by authoring and executing the adversarial test harness `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs` (29 test cases) alongside the baseline unit tests in `crates/music_dsp/tests/dsp_tests.rs` (45 tests) and workspace integration tests (191 tests).

### Baseline Test Execution
```powershell
cargo test -p music_dsp
```
- 45/45 existing unit tests in `dsp_tests.rs` passed.
- 20/20 fidelity tests in `adversarial_fidelity_and_compliance_tests.rs` passed.
- 29/29 new empirical tests in `adversarial_dsp_stress_tests.rs` executed.
- `cargo check --workspace` exited code 0 with zero compiler warnings.

### Empirical Vulnerabilities Directly Observed

#### Observation 1: Integer Multiplication Overflow Panic in `crossfader.rs` on `bpm == 0`
- **Location**: `crates/music_dsp/src/crossfader.rs:108-109`:
  ```rust
  let samples_per_beat = (60.0 / bpm as f32 * SAMPLE_RATE as f32).round() as usize;
  let samples_per_bar = samples_per_beat * beats_per_bar as usize;
  ```
- **Observed Behavior**:
  Calling `StemCrossfader::inpaint_bars(&base, &splice, 0, 0, 4, 15.0, CrossfadeCurve::EqualPowerSine)` or `splice_stem(&base, &splice, 0, 0, 4)` panics immediately:
  ```
  thread 'test_adversarial_crossfader_zero_bpm_multiplication_overflow' panicked at crates\music_dsp\src\crossfader.rs:109:31:
  attempt to multiply with overflow
  ```
- **Verbatim Cause**:
  `60.0 / 0.0 * 48000.0` yields `f32::INFINITY`. When cast to `usize`, it saturates to `usize::MAX` (`18_446_744_073_709_551_615`). Then `usize::MAX * 4` overflows `usize`, triggering an unhandled panic in debug mode.

#### Observation 2: NaN Score Propagation in `telemetry_calc.rs::calculate_phase_alignment`
- **Location**: `crates/music_dsp/src/telemetry_calc.rs:259-265`:
  ```rust
  let denom = (sum_ll * sum_rr).sqrt();
  if denom <= 1e-12 {
      1.0
  } else {
      let rho = (sum_lr / denom) as f32;
      ((rho + 1.0) / 2.0).clamp(0.0, 1.0)
  }
  ```
- **Observed Behavior**:
  When `buffer.samples` contains an invalid float or `NaN`, `sum_ll` and `sum_rr` become `NaN`, resulting in `denom = NaN`.
  In IEEE 754, `NaN <= 1e-12` evaluates to `false`. The function branches to `else`, calculating `rho = NaN / NaN = NaN`.
  In standard Rust, `(f32::NAN).clamp(0.0, 1.0)` evaluates to `NaN`.
  The function returns `NaN` as `phase_alignment_score` in `StemTelemetry`.

#### Observation 3: Slice Index Out of Bounds Panic in `groove_synth.rs::render_hits` on Mono Buffer
- **Location**: `crates/music_dsp/src/groove_synth.rs:412-413`:
  ```rust
  buffer.samples[f * 2] = (buffer.samples[f * 2] + sample_val * pan_l * hit.velocity).clamp(-0.95, 0.95);
  buffer.samples[f * 2 + 1] = (buffer.samples[f * 2 + 1] + sample_val * pan_r * hit.velocity).clamp(-0.95, 0.95);
  ```
- **Observed Behavior**:
  `UniversalDrumSynthesizer::render_hits` is a public API accepting `&mut AudioBuffer`. It unconditionally assumes a stereo layout (`f * 2` and `f * 2 + 1`).
  When invoked on a 1.0-second mono buffer (`channels: 1`, `samples.len() == 48000`, `frames == 48000`) with a drum hit at `time_secs: 0.8` (`start_frame: 38400`), `f * 2 + 1` reaches `76801 > 48000`, triggering:
  ```
  thread panicked at crates\music_dsp\src\groove_synth.rs:413:21:
  index out of bounds: the len is 48000 but the index is 76801
  ```

#### Observation 4: Division by Zero Panic in `telemetry_calc.rs` on `sample_rate == 0`
- **Location**: `crates/music_dsp/src/telemetry_calc.rs:130, 206`:
  ```rust
  // in calculate_tempo_drift:
  let hop_size = (sample_rate * 0.010).round() as usize;
  let num_hops = mono.samples.len() / hop_size;

  // in calculate_snr:
  let frame_len = (buffer.sample_rate as f32 * 0.05).round() as usize;
  let num_blocks = total_frames / frame_len;
  ```
- **Observed Behavior**:
  If an `AudioBuffer` has `sample_rate: 0`, `hop_size` and `frame_len` become `0`. Integer division by zero triggers:
  ```
  thread panicked at crates\music_dsp\src\telemetry_calc.rs:206:26:
  attempt to divide by zero
  ```

---

## 2. Logic Chain

1. **Adversarial Mandate**:
   The dispatch explicitly assigned:
   *"Adversarially stress-test crates/music_dsp for numeric stability, buffer edge cases (0 BPM, extreme BPMs, 0-length buffers, channel mismatches, NaN/Inf immunity in FFT and filters)."*

2. **Analysis of 0 BPM**:
   In `groove_synth.rs` and `guitar_wah.rs`, BPM is defensively clamped via `bpm.clamp(40, 220)`.
   However, in `crossfader.rs`, `bpm` was not clamped or checked for zero. Because `inpaint_bars` calculates `(60.0 / bpm as f32 * SAMPLE_RATE as f32).round() as usize`, passing `0` produces `usize::MAX`. Multiplying `usize::MAX * beats_per_bar` causes a fatal runtime overflow panic. Because `splice_stem` forwards `bpm` directly to `inpaint_bars`, this exposes a user-facing panic vector.

3. **Analysis of NaN Immunity**:
   In `telemetry_calc.rs`, while `calculate_spectral_flatness` and `calculate_snr` gracefully clamp against NaN/Inf, `calculate_phase_alignment` assumes `denom > 1e-12` implies finite numbers. Under IEEE 754, `NaN <= 1e-12` is false, propagating NaN into `StemTelemetry.phase_alignment_score`.

4. **Analysis of Buffer Layout Invariants**:
   `AudioBuffer` supports mono (`channels: 1`) and stereo (`channels: 2`). Public functions like `render_hits` that accept `&mut AudioBuffer` must respect `buffer.channels` or convert to stereo before writing to `f * 2 + 1`.

5. **Severity Assessment**:
   While normal musical paths with valid inputs (40–220 BPM, stereo buffers, 48 kHz) succeed cleanly, unhandled edge cases cause panic crashes or NaN poisoning. In production pipeline nodes (Nodes 2, 4, 5), an unhandled panic will crash the Tokio worker task.

---

## 3. Caveats

1. **Review-Only Constraint**:
   In strict accordance with the Teamwork protocol (*"Review-only — do NOT modify implementation code"*), this agent did not modify `crates/music_dsp/src/*`. All vulnerabilities were diagnosed through non-invasive test harnesses in `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`.
2. **Standard Path Robustness**:
   For normal inputs within contract boundaries, `music_dsp` demonstrates exceptional algorithmic fidelity, zero external dependencies, robust Radix-2 FFT, and strict $-0.3\text{ dBFS}$ brickwall limiting. The rejection is solely due to the 4 unhandled boundary panic/NaN conditions.

---

## 4. Conclusion

**Verdict: REJECT**

Milestone 2 implementation cannot be approved in its current state because calling public APIs with 0 BPM, mono buffers in drum synthesis, or NaN/zero sample rates triggers unhandled panic crashes and NaN poisoning.

### Actionable Remediation Instructions for `worker_m2`:

1. **Fix `crates/music_dsp/src/crossfader.rs:108`**:
   Clamp `bpm` or guard against zero:
   ```rust
   let active_bpm = bpm.clamp(40, 220);
   let samples_per_beat = (60.0 / active_bpm as f32 * SAMPLE_RATE as f32).round() as usize;
   ```
2. **Fix `crates/music_dsp/src/telemetry_calc.rs:260-265`**:
   Guard against `denom.is_nan()` and ensure finite rho:
   ```rust
   let denom = (sum_ll * sum_rr).sqrt();
   if denom.is_nan() || denom <= 1e-12 {
       1.0
   } else {
       let rho = (sum_lr / denom) as f32;
       if rho.is_nan() {
           1.0
       } else {
           ((rho + 1.0) / 2.0).clamp(0.0, 1.0)
       }
   }
   ```
3. **Fix `crates/music_dsp/src/groove_synth.rs:392-416`**:
   Convert mono to stereo or handle channel count:
   ```rust
   if buffer.channels == 1 {
       *buffer = buffer.to_stereo();
   }
   ```
4. **Fix `crates/music_dsp/src/telemetry_calc.rs:123, 199`**:
   Add guard for `buffer.sample_rate == 0`:
   ```rust
   // in calculate_tempo_drift:
   if buffer.is_empty() || buffer.sample_rate == 0 || expected_bpm == 0 {
       return 0.02;
   }
   // in calculate_snr:
   if buffer.is_empty() || buffer.sample_rate == 0 {
       return 92.0;
   }
   ```

---

## 5. Verification Method

To independently verify all claims and reproduce the empirical failure modes:

1. **Run the Adversarial Stress Test Suite**:
   ```powershell
   cargo test -p music_dsp --test adversarial_dsp_stress_tests
   ```
   *Current Result*: 29/29 tests pass (confirming that `inpaint_bars(0 BPM)`, `splice_stem(0 BPM)`, `render_hits(mono)`, and `sample_rate(0)` panic, and that `calculate_phase_alignment(NaN)` yields `NaN`).

2. **Verify Remediation**:
   Once `worker_m2` applies the 4 recommended mitigations, update the 4 test assertions in `adversarial_dsp_stress_tests.rs` (e.g. changing `assert!(result.is_err())` to asserting graceful execution without panic and finite numbers).
   Run:
   ```powershell
   cargo test --workspace
   ```
   *Expected Result*: All 130+ unit tests across the workspace and 191 E2E tests pass cleanly with zero panics.
