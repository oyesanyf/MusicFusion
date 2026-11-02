# Empirical Challenge Report & Handoff — Milestone 8 (Legal Originality Guard, RVQ Tokenizer, & Style Blending)

**Agent**: `challenger_m8_1`  
**Verdict**: **APPROVE**  
**Working Directory**: `d:\harfile\music\.agents\teamwork\challenger_m8_1\`  
**Timestamp**: 2026-11-02T16:21:00Z  

---

## 1. Observation

### 1.1 Empirical Verification of Focus Requirements

1. **Legal Originality Guard — Melodic Hook Copying & Transposition Invariance**:
   - Inspected `crates/music_fusion/src/originality_guard.rs` (lines 188–206, 208–231): `calculate_transposition_invariant_lcs` runs dynamic programming for Longest Common Contiguous Substring across all semitone shifts $\Delta \in \{0..11\}$.
   - Implemented `test_empirical_12_semitone_transposition_on_diverse_hooks` in `crates/music_fusion/tests/challenger_m8_empirical_tests.rs`: tested 5 distinct melodic hook shapes (Major Diatonic `[0, 2, 4, 5, 7, 9, 11, 0]`, Minor Pentatonic `[9, 0, 2, 4, 7, 9, 0, 2]`, Blues Riff `[0, 3, 5, 6, 7, 10, 0, 10]`, Chromatic Run `[1, 2, 3, 4, 5, 6, 7, 8]`, Disjunct Arpeggio `[0, 7, 4, 0, 7, 11, 7, 4, 0, 7]`) across all 12 semitones ($0..=11$). 100% of cases yielded `overlap_notes >= 8`, `is_compliant: false`, and `compliance_status: RejectedPotentialCopy`.
   - Implemented `test_empirical_embedded_contiguous_hooks_at_different_offsets`: embedded 8-note hooks inside 26-note sequences with asymmetric non-matching padding notes. Across all 12 semitone transpositions, the contiguous hook was caught at arbitrary offsets ($LCS \ge 8$), triggering `RejectedPotentialCopy`.
   - Implemented `test_empirical_hook_boundary_7_vs_8_notes`: verified boundary behavior where a 7-note phrase (isolated with distinct harmonic chroma) produces `note_ratio = 0.416 <= 0.70` (compliant), whereas an 8-note phrase triggers immediate statutory breach rejection (`is_breached = true`, `RejectedPotentialCopy`).

2. **Legal Originality Guard — Inverted Waveform Polarity & Cross-Correlation**:
   - Inspected `crates/music_fusion/src/originality_guard.rs` (lines 235–318): `calculate_max_waveform_cross_correlation` normalizes dot products by Euclidean norms and evaluates absolute correlation: `let rho = (sum_dot / denom).abs();`.
   - Implemented `test_empirical_inverted_polarity_cross_correlation_detection`: tested 5 diverse synthesized waveforms (440 Hz Sine, Triad Chord, FM modulated, Chirp sweep, and Pulse train) inverted via `sample * -1.0`. All 5 waveforms yielded peak cross-correlation $\rho > 0.95$, breaching the `WAVEFORM_CORRELATION_CEILING = 0.65`, resulting in `is_compliant: false` and `RejectedPotentialCopy`.
   - Implemented `test_empirical_gain_scaling_and_lag_shift_invariance`: tested scaling gains across $\alpha \in [\pm 0.1, \pm 3.0]$ and time lags up to 200 samples. In every case, normalized cross-correlation $\rho > 0.90 \gg 0.65$, confirming invariant detection.
   - Implemented `test_empirical_chroma_cosine_ceiling_check`: identical chroma yielded cosine $\approx 1.0 > 0.75$, while orthogonal chroma yielded $< 10^{-4} \le 0.75$.

3. **RVQ Tokenizer — Monotonic Residual Norm Reduction**:
   - Inspected `crates/music_dsp/src/rvq.rs` (lines 131–160, 205–237, 289–335): The first codebook centroid $k=0$ is deterministically initialized to $\mathbf{0}$ (`vec![0.0f32; dimension]`), which guarantees that distance to centroid $\le$ distance to origin ($\|residual - c_{best}\|^2 \le \|residual - \mathbf{0}\|^2 = \|residual\|^2$).
   - Implemented `test_empirical_rvq_monotonic_residual_reduction_multi_config`: tested configurations $(M, K, D) \in \{(2, 64, 16), (4, 256, 32), (6, 256, 32), (8, 512, 32), (4, 1024, 64)\}$ under all-zeros, high-frequency square wave, tiny values ($10^{-5}$), and large values ($10^3$). Monotonicity verified without exception.
   - Implemented `test_empirical_rvq_500_random_vectors_strict_monotonicity`: executed 500 trials with pseudo-random vectors across 4 stages. Across all $500 \times 4 = 2,000$ stage transitions, $\|r_m\| \le \|r_{m-1}\| + 10^{-5}$ held 100% of the time (0 violations observed). Average norm reduction on random noise was $23.78\%$.

4. **Anti-Monopoly Style Blending**:
   - Inspected `crates/music_dsp/src/acoustic_analyzer.rs` (lines 183–189, 320–373): `clamp_and_normalize_weights` applies simplex projection with an anti-monopoly ceiling of `0.60` (`ANTI_MONOPOLY_CEILING`) whenever $N \ge 3$.
   - Implemented `test_empirical_anti_monopoly_weight_capping_n_ge_3`: tested $N \in \{3, 4, 5, 8, 12, 16, 20\}$ with adversarial inputs (monopoly file requesting $1,000,000\times$ weight, geometric series $10^i$, and zero/negative weights). In every test, the dominant file weight was clamped to $\le 0.60001$, each file received $w_i \ge 0.0$, and the sum equaled $1.0 \pm 10^{-4}$.
   - Implemented `test_empirical_anti_monopoly_boundary_n2_vs_n3`: for $N=2$, anti-monopoly is unconstrained (weight was $0.90$ for a $90:10$ request); for $N=3$, that same file was strictly constrained to $60\%$.

---

## 2. Logic Chain

1. *Claim*: Melodic hook copying and 12-semitone pitch transpositions cannot bypass `LegalOriginalityGuard`.
   *Observation*: In `originality_guard.rs:196`, the loop `for delta in 0..12u8` transforms pitch tokens via `(p + delta) % 12` and calls `longest_common_substring`. In `challenger_m8_empirical_tests.rs:36`, 5 hook genres across all 12 semitones resulted in $LCS = \text{hook.len()} \ge 8$ and $100\%$ `RejectedPotentialCopy`.
   *Inference*: Transposition invariance is mathematically exhaustive and robust against octave/semitone pitch shifts.

2. *Claim*: Inverted waveform polarity is detected by cross-correlation.
   *Observation*: In `originality_guard.rs:279`, `let rho = (sum_dot / denom).abs();` takes the absolute value of normalized dot product. For `x * -1.0`, `sum_dot = -sum x^2`, so `sum_dot / denom = -1.0`, and `abs() = 1.0 > 0.65`.
   *Inference*: Polarity inversion attacks (phase flips) cannot fool the waveform cross-correlation detector.

3. *Claim*: RVQ residual norm reduction is monotonic.
   *Observation*: In `rvq.rs:133`, centroid index $k=0$ is the zero vector. Centroid search minimizes $\|r - c_k\|^2$. Since $c_0 = 0$ is evaluated, $\min_k \|r - c_k\|^2 \le \|r - 0\|^2 = \|r\|^2$. Across 500 trials with random vectors, zero violations occurred ($\|r_m\| \le \|r_{m-1}\| + 10^{-5}$ in $100\%$ of trials).
   *Inference*: Monotonic residual reduction is mathematically guaranteed by the codebook construction.

4. *Claim*: Style blending limits single-file dominance to $\le 60\%$ when $N \ge 3$.
   *Observation*: In `acoustic_analyzer.rs:184`, `if count >= 3` invokes `clamp_and_normalize_weights(&raw_weights, 0.60)`. In `challenger_m8_empirical_tests.rs:555`, even when a file demanded $1,000,000\times$ weight, its final weight was clamped to $0.60000$, and the remaining $0.40000$ was distributed across other reference files.
   *Inference*: The anti-monopoly requirement is strictly enforced on the probability simplex.

---

## 3. Caveats

- **Lag window bound**: Waveform normalized cross-correlation evaluates lags up to `max_lag = min_len.min(8000)` (up to 2 seconds at 4 kHz) with step 4 samples (1ms resolution) to keep per-candidate latency under 50ms during real-time RL loops. Spliced samples outside the 2-second offset window rely on full-duration N-gram melodic LCS and sliding chroma cosine similarity, combined with the zero-splicing architectural invariant (`zero_waveform_sampling: true`).
- **Resilient WAV decoding**: `AudioBuffer::from_wav_bytes` uses `data_size = chunk_size.min(bytes.len().saturating_sub(offset))`, which gracefully decodes truncated audio streams (returning partial samples rather than erroring). Fully invalid headers, non-RIFF, or missing chunks properly return HTTP 400.

---

## 4. Conclusion

**Verdict: APPROVE**

Milestone 8 fully satisfies all empirical requirements:
1. `LegalOriginalityGuard` provides 100% rejection for direct audio copies, polarity-inverted audio ($\rho > 0.95$), and contiguous melodic hooks $\ge 8$ notes across all 12 chromatic semitone transpositions.
2. `RvqTokenizer` guarantees monotonic residual norm reduction across all quantization stages ($M=2..8$).
3. `AcousticAnalyzer::blend_ensemble` strictly caps any single reference file's style weight to $\le 60\%$ whenever $N \ge 3$, preserving simplex normalization ($\sum w_i = 1.0$).
4. All existing and new empirical test suites pass cleanly with 0 failures and 0 warnings.

---

## 5. Verification Method

To independently verify all findings:

```powershell
# 1. Run the worker's original adversarial originality tests (3 tests)
cargo test --test originality_adversarial_tests -p music_fusion

# 2. Run the challenger's dedicated empirical stress suite (12 tests)
cargo test --test challenger_m8_empirical_tests -p music_fusion

# 3. Run unit tests for music_dsp (8 tests) and music_fusion (10 tests)
cargo test -p music_dsp --lib
cargo test -p music_fusion --lib

# 4. Run the REST API and RL pipeline integration tests (4 tests)
cargo test --test reference_wav_and_rl_tests -p music_cli
```
