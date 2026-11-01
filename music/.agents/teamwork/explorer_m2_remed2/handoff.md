# Remediation Analysis Report: Milestone 2 Pure-Rust DSP Engine Telemetry Calculation

- **Agent**: Remediation Explorer M2-2 (`explorer_m2_remed2`)
- **Role**: Remediation Explorer (investigation, synthesis, exact fix strategy)
- **Target**: `crates/music_dsp/src/telemetry_calc.rs`
- **Associated Test Suite**: `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`
- **Date**: 2026-11-01

---

## 1. Observation

Direct empirical observations were verified against `crates/music_dsp/src/telemetry_calc.rs` and the adversarial test harness in `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`.

### Observation 1.1: NaN Score Propagation in `calculate_phase_alignment`
- **Location**: `crates/music_dsp/src/telemetry_calc.rs:259-265`
- **Code**:
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
  When `buffer.samples` contains a `NaN` (or unnormalized non-finite floats), `sum_ll` and/or `sum_rr` evaluates to `NaN`.
  Consequently, `denom = (NaN).sqrt() = NaN`.
  In IEEE 754 float semantics (and Rust's standard float implementation), any comparison `NaN <= 1e-12` evaluates strictly to `false`.
  The function branches into `else`, computing `rho = (sum_lr / denom) as f32` (`NaN / NaN`), which evaluates to `NaN`.
  In Rust's standard library, `f32::clamp(0.0, 1.0)` on `NaN` evaluates to `NaN` (as neither `< 0.0` nor `> 1.0` holds true).
  The function therefore returns `NaN` as `phase_alignment_score` in `StemTelemetry`.
- **Empirical Test Location**:
  `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs:317-328` (`test_adversarial_phase_alignment_nan_propagation`).

### Observation 1.2: Integer Division by Zero Panic in `calculate_tempo_drift`
- **Location**: `crates/music_dsp/src/telemetry_calc.rs:123-130`
- **Code**:
  ```rust
  pub fn calculate_tempo_drift(buffer: &AudioBuffer, expected_bpm: u32) -> f32 {
      if buffer.is_empty() || expected_bpm == 0 {
          return 0.02;
      }

      let sample_rate = buffer.sample_rate as f32;
      let hop_size = (sample_rate * 0.010).round() as usize; // 10ms hops
      let mono = buffer.to_mono();
      let num_hops = mono.samples.len() / hop_size;
  ```
- **Observed Behavior**:
  Line 123 checks `expected_bpm == 0`, but does not guard against `buffer.sample_rate == 0`.
  When `buffer.sample_rate == 0` (or `sample_rate < 50` where `sample_rate * 0.010` rounds to 0):
  `hop_size` becomes `0`.
  Line 130 computes `let num_hops = mono.samples.len() / hop_size;`.
  Because `hop_size == 0`, integer division by zero triggers an unhandled panic:
  `thread panicked at crates\music_dsp\src\telemetry_calc.rs:130:26: attempt to divide by zero`.
- **Empirical Test Location**:
  `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs:480-488` in `test_adversarial_sample_rate_zero_telemetry_division_by_zero`.

### Observation 1.3: Integer Division by Zero Panic in `calculate_snr`
- **Location**: `crates/music_dsp/src/telemetry_calc.rs:199-206`
- **Code**:
  ```rust
  pub fn calculate_snr(buffer: &AudioBuffer) -> f32 {
      if buffer.is_empty() {
          return 92.0;
      }

      let frame_len = (buffer.sample_rate as f32 * 0.05).round() as usize; // 50ms frames
      let ch = buffer.channels as usize;
      let total_frames = buffer.frame_count();
      let num_blocks = total_frames / frame_len;
  ```
- **Observed Behavior**:
  Line 199 checks only `if buffer.is_empty()`. It does not check `buffer.sample_rate == 0` or `buffer.channels == 0`.
  When `buffer.sample_rate == 0` (or `sample_rate < 10` where `sample_rate * 0.05` rounds to 0):
  `frame_len` becomes `0`.
  Line 206 computes `let num_blocks = total_frames / frame_len;`.
  Because `frame_len == 0`, integer division by zero triggers an unhandled panic:
  `thread panicked at crates\music_dsp\src\telemetry_calc.rs:206:26: attempt to divide by zero`.
  Additionally, if `buffer.channels == 0`, line 220 `sum_sq / (frame_len * ch) as f64` risks floating division by zero.
- **Empirical Test Location**:
  `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs:472-479` in `test_adversarial_sample_rate_zero_telemetry_division_by_zero`.

---

## 2. Logic Chain

1. **Safety Contract for Telemetry Analysis**:
   - `StemTelemetry` is consumed by Node 5 (`dsp_and_judge_gate_node`) in `music_pipeline` to score and gate audio quality.
   - Any panic inside `calculate_telemetry` or its component routines (`calculate_phase_alignment`, `calculate_tempo_drift`, `calculate_snr`) aborts the Tokio async pipeline worker task.
   - Any `NaN` returned in `StemTelemetry` fields poisons downstream quality thresholds (`state.judge_score`, `quality_passed`), causing undefined behavior in threshold comparisons (`score >= 0.85` evaluates to false on `NaN`).

2. **Deduction of Phase Alignment Root Cause**:
   - Normalized stereo phase correlation $\rho = \frac{\sum L R}{\sqrt{\sum L^2 \sum R^2}}$ requires both channels to have non-zero, finite power.
   - When samples contain `NaN`, standard IEEE 754 ordering rules dictate that `NaN <= X` is false for all finite `X`.
   - The current check `if denom <= 1e-12` erroneously assumes that any value not `<= 1e-12` is a strictly positive, finite float.
   - In fact, `NaN` fails this check, entering the division branch and computing `rho = NaN`.
   - Neutral phase alignment for empty, silent, mono, or undefined buffers is canonically defined as `1.0` (compatible with mono, no phase cancellation).
   - Therefore, verifying `!denom.is_finite() || denom <= 1e-12` and `!rho.is_finite()`, and returning `1.0`, mathematically eliminates NaN propagation.

3. **Deduction of Division by Zero Root Cause**:
   - In `calculate_tempo_drift`:
     - 10 ms hop sizing is calculated as `(sample_rate * 0.010).round() as usize`.
     - When `sample_rate == 0`, `hop_size == 0`.
     - Rust integer division `mono.samples.len() / hop_size` panics when `hop_size == 0`.
     - Standard fallback drift score for unanalyzable audio (e.g. empty buffer or 0 BPM) is `0.02` (as already implemented for `buffer.is_empty() || expected_bpm == 0`).
     - Extending the guard to `if buffer.is_empty() || buffer.sample_rate == 0 || expected_bpm == 0` and adding `if hop_size == 0 { return 0.02; }` eliminates the panic while preserving the expected contract.
   - In `calculate_snr`:
     - 50 ms frame sizing is calculated as `(sample_rate * 0.05).round() as usize`.
     - When `sample_rate == 0`, `frame_len == 0`.
     - Rust integer division `total_frames / frame_len` panics when `frame_len == 0`.
     - Standard fallback SNR for unanalyzable audio (e.g. empty or silent buffer) is `92.0` dB.
     - Extending the guard to `if buffer.is_empty() || buffer.sample_rate == 0 || buffer.channels == 0` and adding `if frame_len == 0 { return 92.0; }` guarantees zero-division prevention.
     - Further guarding `snr.is_finite()` guarantees that even if non-finite powers occur in corrupt buffers, `92.0` dB is returned cleanly.

---

## 3. Caveats

1. **Read-Only Explorer Scope**:
   In accordance with the Teamwork protocol, this agent does not mutate `crates/music_dsp/src/*` directly. This analysis provides the verified drop-in code modifications and test suite updates for `worker_m2`.
2. **Scope Boundaries**:
   Vulnerabilities in `crossfader.rs` (0 BPM multiplication overflow) and `groove_synth.rs` (mono buffer index out of bounds) are concurrently addressed by peer explorer `explorer_m2_remed1`.

---

## 4. Conclusion & Actionable Fix Strategy

### 4.1. Precise Modifications in `crates/music_dsp/src/telemetry_calc.rs`

#### Fix A: `calculate_phase_alignment` (Lines 259–265)
- **Target File**: `crates/music_dsp/src/telemetry_calc.rs`
- **Target Range**: Lines 259–265
- **Existing Code**:
  ```rust
      let denom = (sum_ll * sum_rr).sqrt();
      if denom <= 1e-12 {
          1.0
      } else {
          let rho = (sum_lr / denom) as f32;
          ((rho + 1.0) / 2.0).clamp(0.0, 1.0)
      }
  ```
- **Replacement Code**:
  ```rust
      let denom = (sum_ll * sum_rr).sqrt();
      if !denom.is_finite() || denom <= 1e-12 {
          1.0
      } else {
          let rho = (sum_lr / denom) as f32;
          if !rho.is_finite() {
              1.0
          } else {
              ((rho + 1.0) / 2.0).clamp(0.0, 1.0)
          }
      }
  ```

#### Fix B: `calculate_tempo_drift` (Lines 122–131)
- **Target File**: `crates/music_dsp/src/telemetry_calc.rs`
- **Target Range**: Lines 122–131
- **Existing Code**:
  ```rust
  pub fn calculate_tempo_drift(buffer: &AudioBuffer, expected_bpm: u32) -> f32 {
      if buffer.is_empty() || expected_bpm == 0 {
          return 0.02;
      }

      let sample_rate = buffer.sample_rate as f32;
      let hop_size = (sample_rate * 0.010).round() as usize; // 10ms hops
      let mono = buffer.to_mono();
      let num_hops = mono.samples.len() / hop_size;
  ```
- **Replacement Code**:
  ```rust
  pub fn calculate_tempo_drift(buffer: &AudioBuffer, expected_bpm: u32) -> f32 {
      if buffer.is_empty() || buffer.sample_rate == 0 || expected_bpm == 0 {
          return 0.02;
      }

      let sample_rate = buffer.sample_rate as f32;
      let hop_size = (sample_rate * 0.010).round() as usize; // 10ms hops
      if hop_size == 0 {
          return 0.02;
      }
      let mono = buffer.to_mono();
      let num_hops = mono.samples.len() / hop_size;
  ```
- **Additional Hardening** (Line 194):
  ```rust
      let drift = (best_lag as f32 - grid_lag).abs() / step_16th_hops;
      if !drift.is_finite() {
          0.02
      } else {
          drift.clamp(0.002, 1.0)
      }
  ```

#### Fix C: `calculate_snr` (Lines 198–207, 236–238)
- **Target File**: `crates/music_dsp/src/telemetry_calc.rs`
- **Target Range**: Lines 198–207
- **Existing Code**:
  ```rust
  pub fn calculate_snr(buffer: &AudioBuffer) -> f32 {
      if buffer.is_empty() {
          return 92.0;
      }

      let frame_len = (buffer.sample_rate as f32 * 0.05).round() as usize; // 50ms frames
      let ch = buffer.channels as usize;
      let total_frames = buffer.frame_count();
      let num_blocks = total_frames / frame_len;
  ```
- **Replacement Code**:
  ```rust
  pub fn calculate_snr(buffer: &AudioBuffer) -> f32 {
      if buffer.is_empty() || buffer.sample_rate == 0 || buffer.channels == 0 {
          return 92.0;
      }

      let frame_len = (buffer.sample_rate as f32 * 0.05).round() as usize; // 50ms frames
      if frame_len == 0 {
          return 92.0;
      }
      let ch = buffer.channels as usize;
      let total_frames = buffer.frame_count();
      let num_blocks = total_frames / frame_len;
  ```
- **Additional Hardening** (Lines 236–238):
  ```rust
      let snr = 10.0 * (signal_power / noise_power).log10() as f32;
      if !snr.is_finite() {
          92.0
      } else {
          snr.clamp(20.0, 96.0)
      }
  ```

---

### 4.2. Test Suite Updates in `crates/music_dsp/tests/adversarial_dsp_stress_tests.rs`

Once `worker_m2` implements the remediations above, the two adversarial tests that originally verified the vulnerability by asserting `is_nan()` and `is_err()` must be updated to assert safe, non-panicking execution and finite outputs:

#### Update Test 1: `test_adversarial_phase_alignment_nan_propagation` (Lines 317–328)
- **Before**:
  ```rust
  #[test]
  fn test_adversarial_phase_alignment_nan_propagation() {
      let mut buf = AudioBuffer::new_stereo(0.1);
      buf.samples[10] = f32::NAN;

      // Documented Bug: calculate_phase_alignment does not check for NaN in denom,
      // causing rho = NaN and returning NaN from ((rho + 1.0)/2.0).clamp(0.0, 1.0).
      let score = calculate_phase_alignment(&buf);
      assert!(
          score.is_nan(),
          "Empirical confirmation: calculate_phase_alignment propagates NaN when buffer contains NaN"
      );
  }
  ```
- **After**:
  ```rust
  #[test]
  fn test_adversarial_phase_alignment_nan_propagation() {
      let mut buf = AudioBuffer::new_stereo(0.1);
      buf.samples[10] = f32::NAN;

      // Remediated: calculate_phase_alignment guards against NaN/non-finite denom and rho,
      // returning safe neutral score 1.0 instead of propagating NaN.
      let score = calculate_phase_alignment(&buf);
      assert!(
          !score.is_nan(),
          "Remediated: calculate_phase_alignment must NOT propagate NaN"
      );
      assert_eq!(score, 1.0);
  }
  ```

#### Update Test 2: `test_adversarial_sample_rate_zero_telemetry_division_by_zero` (Lines 463–488)
- **Before**:
  ```rust
  #[test]
  fn test_adversarial_sample_rate_zero_telemetry_division_by_zero() {
      let zero_sr_buf = AudioBuffer {
          sample_rate: 0,
          channels: 2,
          samples: vec![0.1; 100],
      };

      // calculate_snr calculates frame_len = 0 * 0.05 = 0, then num_blocks = 50 / 0.
      let result_snr = std::panic::catch_unwind(|| {
          calculate_snr(&zero_sr_buf)
      });
      assert!(
          result_snr.is_err(),
          "Empirical confirmation: calculate_snr panics with attempt to divide by zero when sample_rate == 0"
      );

      // calculate_tempo_drift calculates hop_size = 0 * 0.01 = 0, then num_hops = len / 0.
      let result_drift = std::panic::catch_unwind(|| {
          calculate_tempo_drift(&zero_sr_buf, 120)
      });
      assert!(
          result_drift.is_err(),
          "Empirical confirmation: calculate_tempo_drift panics with attempt to divide by zero when sample_rate == 0"
      );
  }
  ```
- **After**:
  ```rust
  #[test]
  fn test_adversarial_sample_rate_zero_telemetry_division_by_zero() {
      let zero_sr_buf = AudioBuffer {
          sample_rate: 0,
          channels: 2,
          samples: vec![0.1; 100],
      };

      // Remediated: calculate_snr guards against sample_rate == 0 and returns fallback default (92.0)
      let result_snr = std::panic::catch_unwind(|| {
          calculate_snr(&zero_sr_buf)
      });
      assert!(
          result_snr.is_ok(),
          "Remediated: calculate_snr must NOT panic when sample_rate == 0"
      );
      assert_eq!(result_snr.unwrap(), 92.0);

      // Remediated: calculate_tempo_drift guards against sample_rate == 0 and returns fallback default (0.02)
      let result_drift = std::panic::catch_unwind(|| {
          calculate_tempo_drift(&zero_sr_buf, 120)
      });
      assert!(
          result_drift.is_ok(),
          "Remediated: calculate_tempo_drift must NOT panic when sample_rate == 0"
      );
      assert_eq!(result_drift.unwrap(), 0.02);
  }
  ```

---

## 5. Verification Method

To independently verify this remediation strategy:

1. **Verify Line Numbers and Call Chains**:
   Inspect `crates/music_dsp/src/telemetry_calc.rs` at lines 122–131, 198–207, and 259–265.
2. **Execute DSP Test Suite**:
   ```powershell
   cargo test -p music_dsp
   ```
   *Expected Outcome*:
   - Unit tests in `dsp_tests.rs` (45 tests) remain 100% passing.
   - Fidelity tests in `adversarial_fidelity_and_compliance_tests.rs` (20 tests) remain 100% passing.
   - Adversarial stress tests in `adversarial_dsp_stress_tests.rs` (29 tests) all pass with the updated non-panic/finite assertions.
3. **Workspace Invalidation Check**:
   ```powershell
   cargo check --workspace
   cargo test --workspace
   ```
   *Expected Outcome*:
   - Clean compilation across all 5 member crates with zero compiler warnings.
   - Zero runtime panics and zero NaN outputs.
