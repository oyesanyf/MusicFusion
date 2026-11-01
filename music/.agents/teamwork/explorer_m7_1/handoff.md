# Technical Investigation and Architecture Report: Requirements R1 & R2
**Agent**: `explorer_m7_1` (Physical Modeling DSP Explorer)  
**Date**: 2026-11-01  
**Scope**: Physical Modeling DSP Synthesis (R1), Continuous Phase Accumulator (R1), Headroom & Tape Mastering (R2), and Hound Serialization (R2).

---

## 1. Observation

Direct examination of the workspace revealed the following specific findings across `crates/music_dsp`, `crates/music_pipeline`, and `Cargo.toml`:

### 1.1 Current Tone Generation, Sweep, and Silence Gating
- **Location**: `crates/music_pipeline/src/nodes/node2_groove_yue.rs:464-475` and `478-490`:
  ```rust
  let is_singing = (current_beat % 8) != 7;
  let target_env = if is_singing {
      if syl_phase < 0.15 {
          0.25 + 0.75 * (syl_phase / 0.15 * PI * 0.5).sin()
      } else if syl_phase < 0.80 {
          1.0
      } else {
          0.25 + 0.75 * (((1.0 - syl_phase) / 0.20) * PI * 0.5).sin().max(0.0)
      }
  } else {
      0.0 // Hard silence gating dropping to zero every 8th beat!
  };
  ...
  let note_ratio = melody_scale[current_beat % melody_scale.len()];
  let vibrato = (2.0 * PI * 5.0 * t).sin() * 1.5;
  let target_freq = vocal_root * note_ratio + vibrato;
  smoothed_freq += glide_alpha * (target_freq - smoothed_freq);
  vocal_phase = (vocal_phase + smoothed_freq / sr).fract();
  ```
  - **Symptom**: Step-frequency glides and hard silence gating (`target_env = 0.0`) produce abrupt drops and stepped frequency chirps/siren sweeps rather than warm continuous musical tones.
- **Location**: `crates/music_dsp/src/guitar_wah.rs:347-370`:
  ```rust
  // D4 full bend to E4 (2 semitones)
  notes.push(SoloNote { bar_offset_beats: 21.0, duration_beats: 1.5, pitch_hz: d4, bend_semitones: 2.0, vibrato: true, velocity: 0.95 });
  // Blue note Eb4
  notes.push(SoloNote { bar_offset_beats: 23.0, duration_beats: 0.5, pitch_hz: d_sharp4, bend_semitones: 0.0, vibrato: false, velocity: 0.90 });
  ...
  // Screaming D5 bend up to E5 (+2 semitones) with high vibrato
  ...
  // Soaring peak A5 (880 Hz)
  ```
  - **Symptom**: The solo generator uses chromatic out-of-scale notes (e.g. $D^\sharp_4 = 155.56$ Hz, $A_5 = 880.0$ Hz, multi-semitone continuous pitch bending) which sweep across chromatic semitones rather than remaining strictly locked to the A Major pentatonic / harmonic scale frequencies (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).

### 1.2 Karplus-Strong String Oscillator
- **Location**: `crates/music_dsp/src/guitar_wah.rs:59-71`:
  ```rust
  self.decay_factor = (0.996 - (freq_hz / 10000.0)).clamp(0.985, 0.998);

  // Excite delay line with a warm, phase-continuous harmonic guitar string wave
  let len_f = self.buffer_len.max(1) as f32;
  for i in 0..self.buffer_len {
      let phase = i as f32 / len_f;
      let f1 = (2.0 * PI * phase).sin() * 0.64;
      let f2 = (4.0 * PI * phase).sin() * 0.24;
      let f3 = (6.0 * PI * phase).sin() * 0.12;
      self.delay_buffer[i] = (f1 + f2 + f3) * velocity;
  }
  ```
  - **Symptom**: The delay line ring buffer is excited by additive sine harmonics rather than a low-pass filtered pink noise burst.
  - **Damping**: The feedback damping factor varies dynamically between 0.985 and 0.998, failing the strict 0.994 invariant required for reggae offbeat skank chords and guitar leads.

### 1.3 Drum and Bass Synthesis in `crates/music_dsp/src/groove_synth.rs`
- **Location**: `crates/music_dsp/src/groove_synth.rs:466-481`:
  ```rust
  DrumVoice::Kick808Sub => {
      let pitch = 42.0 + 88.0 * (-t / 0.038).exp(); // 130 Hz down to 42 Hz over 38 ms
  }
  DrumVoice::KickAcoustic | DrumVoice::KickEdmPunch => {
      let pitch = 52.0 + 108.0 * (-t / 0.022).exp(); // 160 Hz down to 52 Hz over 22 ms
  }
  ```
  - **Symptom**: Kick drops from 130 Hz or 160 Hz rather than the authentic Reggae One-Drop specification: 110 Hz down to 42 Hz over 60 ms on Beat 3.
  - **Hi-Hats**: Currently uses filtered white noise brush taps (`smooth_brush`) rather than high-pass filtered metallic square-wave ratios (>6000 Hz, 25 ms decay).
  - **Dub Sub-Bass**: Synthesized in `node2_groove_yue.rs:279-320` as pure sine waves with portamento glide, lacking the low-pass filtered (180 Hz) sine + triangle blend and explicit 15 ms linear attack / 120 ms exponential release.
  - **Hammond Organ Bubble**: Present only as basic chord pad sines, lacking 1x, 2x, 3x, 4x drawbar harmonics, 6 Hz rotary tremolo LFO, and ADSR envelope.

### 1.4 Stem Headroom, Mixing, and Mastering
- **Location**: `crates/music_pipeline/src/nodes/node6_analog_mastering.rs:73-87`:
  ```rust
  let (vocal_gain, guitar_gain, drums_gain, bass_gain, other_gain) = (0.95, 0.75, 0.82, 0.72, 0.65);
  ```
  - Stems were mixed with hot individual gains ($0.72 - 0.95$). Summing hot stems without 0.20 scaling (-14 dBFS RMS headroom) creates bus clipping before mastering.
- **Location**: `crates/music_pipeline/src/nodes/node6_analog_mastering.rs:213-217` and `crates/music_dsp/src/tape_mastering.rs:260-340`:
  - No 12,000 Hz 2nd-order Butterworth low-pass filter is applied to synthetic instruments before mastering.
  - Mastering uses -0.3 dBFS (0.966) brickwall ceiling rather than -1.0 dBFS (0.891) peak normalization and `f32::tanh(sample * 1.2)` analog tape saturation.

### 1.5 WAV Serialization and Test Failures
- **Location**: `crates/music_dsp/src/wav.rs:367-396`:
  ```rust
  let peak = self.peak_level();
  let norm_scale = if peak > 0.966 { 0.966 / peak } else { 1.0 };
  let edge_fade_frames = ((0.004 * sample_rate as f32) as usize).min(total_frames / 8);
  ...
  let abs_s = s.abs();
  if abs_s <= 0.92 { s } else { ... (0.92 + 0.046 * excess.tanh()).copysign(s) }
  ```
  - **Command**: `cargo test -p music_dsp --test dsp_tests`
  - **Result**: FAILED with 4 test failures:
    - `test_wav_16bit_pcm_roundtrip`
    - `test_wav_24bit_pcm_roundtrip`
    - `test_wav_32bit_float_exact_roundtrip`
    - `test_wav_file_roundtrip_disk`
  - **Command**: `cargo test -p music_dsp --test adversarial_dsp_stress_tests`
  - **Result**: FAILED with `test_adversarial_wav_roundtrip_all_formats_extreme_values` (`Diff exceeds tolerance`).
  - **Root Cause**: `wav.rs` attempted to apply edge-fading (`edge_env`) and soft-clipping (`excess.tanh()`) inside low-level byte serialization (`to_wav_bytes_with_format`). This corrupts roundtrip WAV encoding and violates bit-exact serialization.
  - **Dependency**: Neither workspace `Cargo.toml` nor `music_dsp/Cargo.toml` currently references `hound`. Running `cargo add hound@3.5 --package music_dsp --dry-run` succeeded without conflicts.

---

## 2. Logic Chain

1. **Phase Coherence & Zero Discontinuities**:
   - *Premise*: Abrupt phase resets ($0.0$) at buffer boundaries generate high-frequency step discontinuities (clicks and buzzing).
   - *Deduction*: By encapsulating oscillator state in a `ContinuousPhaseAccumulator` that advances incrementally via `phase = (phase + 2.0 * PI * freq / sample_rate) % (2.0 * PI)`, phase evolves smoothly across buffer frames and note transitions.

2. **A Major Pentatonic / Harmonic Scale Locking**:
   - *Premise*: R1 strictly mandates 76 BPM ($0.7894$ s/beat) and the exact frequencies 220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz, forbidding chromatic siren sweeps.
   - *Deduction*: Restricting note selection matrices in `guitar_wah.rs` and `node2_groove_yue.rs` strictly to these 6 fundamental frequencies eliminates chromatic dissonance and siren artifacts.

3. **Karplus-Strong Plucked String Synthesis**:
   - *Premise*: Authentic plucked strings require broad spectral initial excitation decaying via frequency-dependent loss.
   - *Deduction*: Generating a short burst of Paul Kellet pink noise through a 2-pole low-pass filter to populate the ring buffer, combined with a loop averaging filter damped by factor $0.994$, replicates both percussive reggae skank chops (beats 2 & 4) and sustain-damped pentatonic leads.

4. **Reggae One-Drop Rhythm Invariants**:
   - *Premise*: The defining feature of Roots Reggae is the One-Drop (heavy kick and rimshot on Beat 3, empty beat 1).
   - *Deduction*:
     - Kick on beat 3 with rapid exponential pitch-drop from 110 Hz down to 42 Hz over 60 ms reproduces authentic punch.
     - Rimshot snare with band-pass filtered white noise (250–1800 Hz) provides the organic cross-stick crack.
     - 16th-note closed hi-hats with high-pass filtered metallic square waves (>6000 Hz, 25 ms decay) recreate classic analog drum machine sizzle without harsh static.
     - Dub sub-bass combining 55.0 Hz ($A_1$), 73.42 Hz ($D_2$), and 82.41 Hz ($E_2$) through a 180 Hz lowpass filter with 15 ms attack and 120 ms release eliminates dead-air gaps and click transients.
     - Hammond organ bubble with drawbars (1x, 2x, 3x, 4x), 6 Hz rotary tremolo, and ADSR fills the offbeat groove.

5. **Stem Headroom & Mastering Normalization (R2)**:
   - *Premise*: Summing 5 stems at unity gain can peak at $+14$ dBFS, causing severe digital clipping.
   - *Deduction*:
     - Scaling each stem by $0.20$ ($-14$ dBFS RMS headroom) ensures the linear sum $\sum_{i=1}^5 0.20 \times s_i \le 1.0$, guaranteeing zero premaster clipping.
     - Filtering through a 2nd-order Butterworth low-pass filter at 12,000 Hz ($Q = 0.7071$) cuts harsh digital aliasing overtones.
     - Soft-clipping via $f32::\tanh(s \times 1.2)$ introduces warm analog 2nd/3rd harmonics.
     - Peak normalization to $-1.0$ dBFS ($0.891$) leaves a safe $1.0$ dBFS true-peak inter-sample headroom margin.
     - Writing 16-bit PCM via `hound::WavWriter` ensures bit-clean audio files without adulterating raw serializer bytes.

---

## 3. Caveats

1. **Non-Reggae Genre Taxonomy Preservation**:
   - `music_core::genre::GenreTaxonomy` supports universal genres (Afrobeats, Amapiano, EDM, Hip-Hop, Rock, Jazz). When `state.genre` is not reggae, `UniversalDrumSynthesizer` still schedules appropriate rhythm grooves (e.g. `FourOnTheFloor`, `Trap808`), but the newly calibrated physical models (continuous phase accumulator, 12 kHz Butterworth filter, 0.20 stem scaling, tanh tape saturation, and -1.0 dBFS normalization) apply globally to ensure clean mastering across all genres.
2. **Offline Dependency Environment**:
   - `hound = "3.5"` is resolved from standard crates.io or cargo cache. Adding `hound = "3.5"` to `Cargo.toml` must be executed cleanly via cargo.
3. **Existing Test Assertions**:
   - Existing unit tests asserting `BRICKWALL_CEILING = 0.966` will pass cleanly because $-1.0$ dBFS ($0.891$) is strictly less than $0.966$. Removing the intrusive `edge_env` and `excess.tanh()` from `to_wav_bytes_with_format` will restore 100% pass status to the 5 failing WAV roundtrip tests.

---

## 4. Conclusion

The current audio distortion, siren sweep artifacts, and WAV roundtrip test failures stem from two architectural flaws:
1. Stepped frequency glides, chromatic bends up to 880 Hz, sine-excited Karplus-Strong buffers, and silence gating in `node2_groove_yue.rs` and `guitar_wah.rs`.
2. Ad-hoc stem summation gains ($0.65 - 0.95$) coupled with destructive sample modification inside `wav.rs` rather than disciplined mastering.

Replacing these with the specified R1 physical modeling engine (Karplus-Strong with pink noise and 0.994 damping, 76 BPM A Major pentatonic scale, 110->42 Hz kick on beat 3, >6000 Hz metallic hi-hats, 180 Hz dub sub-bass, and Hammond organ bubble) and R2 mastering chain (0.20 stem scaling, 12 kHz Butterworth biquad, $\tanh(s \cdot 1.2)$ tape saturation, -1.0 dBFS normalization, and `hound` WAV serialization) will resolve all audio artifacts, eliminate clipping, and ensure all workspace tests pass.

---

## 5. Verification Method

To independently verify the implementation:

1. **Build & Test Workspace**:
   ```powershell
   cargo check --workspace
   cargo test -p music_dsp
   cargo test -p music_pipeline
   cargo test --workspace
   ```
   *Pass criteria*: Zero test failures across all crates (including `dsp_tests`, `adversarial_dsp_stress_tests`, and `adversarial_pipeline_and_audio_ceiling_tests`).

2. **Flagship Anthem Synthesis & Clipping Inspector**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --pure-rust
   ```
   *Pass criteria*:
   - Generated `final_master.wav` peak is strictly $\le 0.891$ (-1.0 dBFS).
   - Telemetry reports `clipping: false`, `peak_dbfs <= -1.0`, and `headroom_db >= 1.0`.
   - Audio exhibits no stepped pitch sweeps or clicks at buffer boundaries.

3. **Stored Sample Songs Regeneration Verification**:
   Verify on-disk `final_master.wav` and stems in:
   - `storage/songs/roots_reggae_anthem/`
   - `storage/songs/song_afrobeats_002/`
   - `storage/songs/song-one-love-shining-1977/`
   - `storage/songs/song_reggae_anthem_001/`
   Confirm via hound or audio inspection that all files are valid 48 kHz 16-bit PCM with peak $\le 0.891$.

---

## 6. Worker Implementation Plan

### Step 1: Update `Cargo.toml` Dependencies
1. In `d:\harfile\music\Cargo.toml` under `[workspace.dependencies]`:
   ```toml
   hound = "3.5"
   ```
2. In `crates/music_dsp/Cargo.toml` under `[dependencies]`:
   ```toml
   hound = { workspace = true }
   ```
3. In `crates/music_pipeline/Cargo.toml` under `[dependencies]`:
   ```toml
   hound = { workspace = true }
   ```

### Step 2: Implement `ContinuousPhaseAccumulator` and `PinkNoiseGenerator` in `crates/music_dsp`
Create or add in `crates/music_dsp/src/guitar_wah.rs` (or a dedicated `phase_accum.rs`):
```rust
/// Continuous phase accumulator across buffer and note boundaries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousPhaseAccumulator {
    pub phase: f32, // Normalized in [0.0, 2.0 * PI)
}

impl ContinuousPhaseAccumulator {
    pub const fn new() -> Self {
        Self { phase: 0.0 }
    }

    #[inline]
    pub fn advance(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        let current = self.phase;
        self.phase += 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
        if self.phase >= 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
            if self.phase >= 2.0 * std::f32::consts::PI {
                self.phase = self.phase.rem_euclid(2.0 * std::f32::consts::PI);
            }
        }
        current
    }

    #[inline]
    pub fn sin(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        self.advance(freq_hz, sample_rate).sin()
    }
}

/// Paul Kellet's filtered pink noise generator with velvet 2-pole lowpass.
#[derive(Debug, Clone)]
pub struct PinkNoiseGenerator {
    b0: f32, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32, b6: f32,
    lp: f32,
}

impl PinkNoiseGenerator {
    pub fn new() -> Self {
        Self { b0: 0.0, b1: 0.0, b2: 0.0, b3: 0.0, b4: 0.0, b5: 0.0, b6: 0.0, lp: 0.0 }
    }

    pub fn next_sample(&mut self, rng: &mut crate::groove_synth::DspRng) -> f32 {
        let white = rng.next_f32();
        self.b0 = 0.99886 * self.b0 + white * 0.0555179;
        self.b1 = 0.99332 * self.b1 + white * 0.0750759;
        self.b2 = 0.96900 * self.b2 + white * 0.1538520;
        self.b3 = 0.86650 * self.b3 + white * 0.3104856;
        self.b4 = 0.55000 * self.b4 + white * 0.5329522;
        self.b5 = -0.7616 * self.b5 - white * 0.0168980;
        let pink = (self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362) * 0.11;
        self.b6 = white * 0.115926;
        self.lp = 0.77 * self.lp + 0.23 * pink; // ~2000 Hz lowpass
        self.lp
    }
}
```

### Step 3: Upgrade Karplus-Strong String Modeler in `crates/music_dsp/src/guitar_wah.rs`
1. Pluck method:
   ```rust
   pub const KARPLUS_FEEDBACK_DAMPING: f32 = 0.994;

   pub fn pluck_pink_noise(&mut self, freq_hz: f32, velocity: f32, rng: &mut DspRng, pink: &mut PinkNoiseGenerator) {
       let period = (self.sample_rate / freq_hz.clamp(50.0, 4000.0)).max(4.0);
       let int_period = period.round() as usize;
       self.buffer_len = int_period.min(self.delay_buffer.len() - 2);
       self.write_ptr = 0;
       self.prev_filter_sample = 0.0;
       self.decay_factor = KARPLUS_FEEDBACK_DAMPING; // Fixed 0.994

       for i in 0..self.buffer_len {
           self.delay_buffer[i] = pink.next_sample(rng) * velocity;
       }
       self.is_active = true;
   }
   ```
2. Loop filter:
   ```rust
   let filtered = (current_sample + self.prev_filter_sample) * 0.5 * 0.994;
   ```
3. Melodic Scale Locking:
   Restrict solo composition to A Major pentatonic frequencies:
   ```rust
   pub const A_MAJOR_PENTATONIC: [f32; 6] = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0];
   ```
   Replace `compose_16_bar_blues_rock_solo` note pitches with notes exclusively from `A_MAJOR_PENTATONIC`.
4. Reggae Skank Chord Plucking on Beats 2 & 4:
   Implement `synthesize_reggae_skank_chord(bar_beat: usize, string_voices: &mut [KarplusStrongString])` triggered at beats 1 and 3 (0-indexed beat 1 = beat 2, beat 3 = beat 4) across A3 (220.0), C#4 (277.18), E4 (329.63), A4 (440.0).

### Step 4: Reggae One-Drop Drum & Dub Bass Synthesizer in `crates/music_dsp/src/groove_synth.rs`
1. **Kick & Rimshot on Beat 3**:
   - Kick pitch drop: $f(t) = 42.0 + 68.0 \cdot \max(0.0, 1.0 - t / 0.060)^2$ (110 Hz down to 42 Hz over 60 ms).
   - Paired with 250–1800 Hz band-pass white noise snare burst with exponential decay.
2. **Hi-Hats**:
   - 16th-note schedule at 76 BPM ($0.1974$ s interval).
   - Sum of 6 metallic square-wave oscillators ($243, 312, 387, 465, 590, 810$ Hz).
   - 2nd-order High-Pass filter at 7000 Hz (>6000 Hz).
   - $25$ ms exponential decay envelope ($e^{-t / 0.025}$).
3. **Dub Sub-Bass**:
   - Walking on $A_1 = 55.0$ Hz, $D_2 = 73.42$ Hz, $E_2 = 82.41$ Hz.
   - Low-pass filtered (180 Hz cutoff) sine + triangle wave.
   - Envelope: $15$ ms linear attack, continuous sustain, $120$ ms exponential release.
4. **Hammond Organ Bubble**:
   - Additive drawbar harmonics: $1.0\times, 2.0\times, 3.0\times, 4.0\times$ fundamental.
   - $6$ Hz rotary speaker tremolo LFO ($1.0 + 0.18 \sin(2\pi \cdot 6.0 \cdot t)$).
   - ADSR envelope (Attack: 10 ms, Decay: 45 ms, Sustain: 0.70, Release: 35 ms).

### Step 5: Clean WAV Roundtrip & Hound Serialization in `crates/music_dsp/src/wav.rs`
1. In `wav.rs`, sanitize `to_wav_bytes_with_format`:
   - Remove `edge_env` and `excess.tanh()` soft-knee compression.
   - Encode audio samples cleanly into IEEE 32-bit float, 24-bit PCM, or 16-bit PCM (clamping $[-1.0, 1.0]$ for integer PCM).
   - This fixes all 5 failing unit tests in `dsp_tests.rs` and `adversarial_dsp_stress_tests.rs`.
2. Add hound serialization helper in `AudioBuffer`:
   ```rust
   pub fn to_wav_file_hound_pcm16<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), crate::error::DspError> {
       let spec = hound::WavSpec {
           channels: self.channels,
           sample_rate: self.sample_rate,
           bits_per_sample: 16,
           sample_format: hound::SampleFormat::Int,
       };
       let mut writer = hound::WavWriter::create(path, spec)
           .map_err(|e| crate::error::DspError::IoError(e.to_string()))?;
       for &sample in &self.samples {
           let clamped = sample.clamp(-1.0, 1.0);
           let val = (clamped * 32767.0).round() as i16;
           writer.write_sample(val)
               .map_err(|e| crate::error::DspError::IoError(e.to_string()))?;
       }
       writer.finalize()
           .map_err(|e| crate::error::DspError::IoError(e.to_string()))?;
       Ok(())
   }
   ```

### Step 6: 12 kHz Biquad Filter & Mastering Stage in `crates/music_dsp/src/tape_mastering.rs`
1. Implement 2nd-order Butterworth Low-Pass Filter at 12,000 Hz:
   ```rust
   pub struct ButterworthLowpass12kHz {
       b0: f32, b1: f32, b2: f32,
       a1: f32, a2: f32,
       states: Vec<[f32; 2]>,
   }

   impl ButterworthLowpass12kHz {
       pub fn new(channels: usize, sample_rate: f32) -> Self {
           let w0 = 2.0 * std::f32::consts::PI * 12000.0 / sample_rate;
           let alpha = w0.sin() / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
           let cos_w = w0.cos();
           let b0 = (1.0 - cos_w) / 2.0;
           let b1 = 1.0 - cos_w;
           let b2 = (1.0 - cos_w) / 2.0;
           let a0 = 1.0 + alpha;
           let a1 = -2.0 * cos_w;
           let a2 = 1.0 - alpha;
           Self {
               b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
               a1: a1 / a0, a2: a2 / a0,
               states: vec![[0.0, 0.0]; channels],
           }
       }

       #[inline]
       pub fn process_sample(&mut self, ch: usize, x: f32) -> f32 {
           if ch >= self.states.len() { return x; }
           let s = &mut self.states[ch];
           let y = self.b0 * x + s[0];
           s[0] = self.b1 * x - self.a1 * y + s[1];
           s[1] = self.b2 * x - self.a2 * y;
           y
       }
   }
   ```
2. Tape Saturator and Normalization:
   - Apply `f32::tanh(sample * 1.2)`.
   - Peak normalize to -1.0 dBFS ($0.891$):
     ```rust
     let mut max_peak = 0.0f32;
     for &s in &out.samples {
         max_peak = max_peak.max(s.abs());
     }
     if max_peak > 0.0001 {
         let target = 0.891f32; // -1.0 dBFS
         let norm = target / max_peak;
         for s in &mut out.samples {
             *s = (*s * norm).clamp(-0.891, 0.891);
         }
     }
     ```

### Step 7: Update `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`
1. Scale each individual stem by **0.20** (-14 dBFS RMS headroom) before summing:
   ```rust
   mix_buffers(&mut inst_bus, &drums_buf, 0.20);
   mix_buffers(&mut inst_bus, &bass_buf, 0.20);
   mix_buffers(&mut inst_bus, &guitar_buf, 0.20);
   mix_buffers(&mut inst_bus, &other_buf, 0.20);
   mix_buffers(&mut inst_bus, &vocal_buf, 0.20);
   ```
2. Pass premaster through the 12,000 Hz Butterworth lowpass filter.
3. Apply `f32::tanh(s * 1.2)`.
4. Peak normalize to -1.0 dBFS ($0.891$).
5. Serialize `final_master.wav` using `hound` 16-bit PCM.

### Step 8: Update Pure-Rust Dual-Stem Synthesis in `node2_groove_yue.rs`
1. Delete glottal pitch sweep and silence gating in `render_pure_rust_dual_stems`.
2. Connect the physical modeling One-Drop drum, Dub Sub-Bass, Karplus-Strong skank chords, and Hammond organ bubble.
3. Maintain continuous phase across frames with `ContinuousPhaseAccumulator`.
4. Save stems using hound 16-bit PCM.

---
Report finalized by `explorer_m7_1`. Handoff ready for Worker.
