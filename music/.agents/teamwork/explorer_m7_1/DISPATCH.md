# DISPATCH: explorer_m7_1

## Objective
Technical exploration and architecture mapping for Requirements R1 and R2:
- Remove chromatic siren sweeps and stepped frequency-sweep oscillator in pure-Rust 48 kHz engine.
- Implement genuine physical modeling locked to 76 BPM (0.7894s / quarter note) and A Major pentatonic / harmonic scale (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).
- Continuous phase accumulator (`phase += 2.0 * PI * freq / sample_rate`) across buffer boundaries so phase never resets abruptly to 0.0 mid-waveform.
- Karplus-Strong plucked string with delay-line ring buffer excited by low-pass filtered pink noise burst, 0.994 feedback damping factor for reggae offbeat skank chords (beats 2 & 4) and pentatonic guitar leads.
- Reggae One-Drop Drum & Dub Bass synthesizer:
  - Kick & Rimshot (Beat 3 One-Drop): kick on beat 3 with rapid exponential pitch-drop sine (110 Hz down to 42 Hz over 60 ms) + band-pass white-noise snare/rimshot burst (250-1800 Hz with exponential decay).
  - Hi-Hats: 16th-note closed hi-hats with high-pass filtered metallic square-wave ratios (>6000 Hz) with tight 25 ms exponential decay envelope.
  - Dub Sub-Bass: warm low-pass filtered (cutoff 180 Hz) sine + triangle bassline walking on roots and fifths (A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz) with 15 ms linear attack, 120 ms exponential release to eliminate clicks and dead-air gaps.
  - Hammond Organ Bubble: Additive drawbar sine harmonics (1.0x, 2.0x, 3.0x, 4.0x fundamental) with gentle 6 Hz rotary speaker tremolo LFO, enveloped smoothly with ADSR.
- Stem Headroom & Mastering (R2):
  - Process all internal stems in normalized f32 (-1.0 to 1.0).
  - Scale each individual stem by 0.20 (-14 dBFS RMS headroom) before summing stems together.
  - Apply Biquad 2nd-order Butterworth Low-Pass filter at 12,000 Hz on synthetic instruments.
  - Smooth `f32::tanh(sample * 1.2)` soft-clipping analog tape saturator and peak normalization to -1.0 dBFS (0.891) before writing 16-bit PCM samples via hound.

## Authoritative Files to Read
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- `d:\harfile\music\PROJECT.md`
- `crates/music_dsp/src/` (`groove_synth.rs`, `guitar_wah.rs`, `tape_mastering.rs`, `wav.rs`, `lib.rs`)
- `crates/music_pipeline/src/nodes/` (`node2_groove_yue.rs`, `node4_stem_inpainting.rs`, `node6_analog_mastering.rs`)
- `crates/music_core/src/telemetry.rs`

## Expected Output
Deliver a comprehensive analysis report in `d:\harfile\music\.agents\teamwork\explorer_m7_1\handoff.md` detailing:
1. Exact locations of current tone generation, frequency sweep, or silence gating in `crates/music_dsp/` and `crates/music_pipeline/`.
2. Exact design and code structures needed for continuous phase accumulation across buffer boundaries.
3. Karplus-Strong string synthesis implementation plan (delay line, pink noise excitation, 0.994 damping, skank chords on 2 & 4, pentatonic lead).
4. One-drop drum, hi-hat, dub sub-bass, and Hammond organ bubble synthesizer specifications.
5. Stem headroom scaling (0.20), 12 kHz Butterworth Biquad filter, tanh tape saturator, peak normalization to -1.0 dBFS (0.891), and hound 16-bit PCM WAV serialization.
6. Concrete file-by-file modification plan for the Worker.


## 2026-11-01T15:59:37Z
You are explorer_m7_1 (Physical Modeling DSP Explorer).
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m7_1\
Read your instructions in d:\harfile\music\.agents\teamwork\explorer_m7_1\DISPATCH.md and the authoritative request in d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md.

Investigate Requirements R1 and R2:
- Remove chromatic siren sweeps and stepped frequency-sweep oscillator in pure-Rust 48 kHz engine.
- Replace with physical modeling locked to 76 BPM and A Major pentatonic / harmonic scale (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).
- Continuous phase accumulator (phase += 2.0 * PI * freq / sample_rate) across buffer boundaries.
- Karplus-Strong string algorithm: delay-line ring buffer excited by low-pass filtered pink noise burst, 0.994 feedback damping factor for reggae offbeat skank chords (beats 2 & 4) and pentatonic guitar leads.
- Reggae One-Drop Drum & Dub Bass synthesizer:
  - Kick & Rimshot (Beat 3 One-Drop): kick on beat 3 (110 Hz down to 42 Hz pitch-drop sine over 60 ms) + band-passed white-noise snare/rimshot burst (250-1800 Hz).
  - Hi-Hats: 16th-note closed hi-hats with high-pass filtered metallic square-wave ratios (>6000 Hz, 25 ms decay).
  - Dub Sub-Bass: warm low-pass filtered (cutoff 180 Hz) sine + triangle bassline on roots and fifths (A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz) with 15 ms linear attack, 120 ms exponential release.
  - Hammond Organ Bubble: Additive drawbar sine harmonics (1x, 2x, 3x, 4x) with 6 Hz rotary speaker tremolo LFO, ADSR.
- Stem Headroom & Mastering (R2):
  - Normalized f32 stems (-1.0 to 1.0); scale each stem by 0.20 (-14 dBFS RMS headroom) before summing.
  - Biquad 2nd-order Butterworth Low-Pass filter at 12,000 Hz on synthetic instruments.
  - Smooth f32::tanh(sample * 1.2) analog tape saturator and peak normalization to -1.0 dBFS (0.891) before writing 16-bit PCM via hound.

Write your complete findings and implementation plan into d:\harfile\music\.agents\teamwork\explorer_m7_1\handoff.md. Send a message to orchestrator when done.
