Fix audio generation in d:\harfile\music by replacing the pure-Rust test-tone oscillator with physical modeling synthesis, correcting stem headroom and tape-saturation mastering to eliminate clipping, enabling a Python neural model bridge via uv with graceful fallback, and regenerating all stored sample songs.

Working directory: d:\harfile\music
Integrity mode: development

## Requirements

### R1. Remove Chromatic Siren Sweep & Implement Real Physical Modeling in Pure-Rust Engine
In the pure-Rust 48 kHz synthesis module, completely delete the stepped frequency-sweep oscillator and hard silence gating. Replace it with genuine musical synthesis locked to 76 BPM (0.7894 seconds per quarter note) and the A Major pentatonic / harmonic scale (220.0 Hz, 246.94 Hz, 277.18 Hz, 329.63 Hz, 369.99 Hz, 440.0 Hz — never sweep chromatically across all 12 semitones):
- Maintain a continuous phase accumulator (`phase += 2.0 * PI * freq / sample_rate`) across buffer boundaries so phase never resets abruptly to 0.0 mid-waveform.
- Implement the Karplus-Strong plucked string algorithm using a delay-line ring buffer excited by a short burst of low-pass filtered pink noise with a 0.994 feedback damping factor for reggae offbeat skank chords (beats 2 and 4) and pentatonic guitar leads.
- Implement a Reggae One-Drop Drum & Dub Bass synthesizer:
  - Kick & Rimshot (Beat 3 One-Drop): Synthesize the kick drum on Beat 3 using a rapid exponential pitch-drop sine wave (110 Hz down to 42 Hz over 60 ms) paired with a band-passed white-noise snare/rimshot burst (250 Hz to 1800 Hz with exponential decay).
  - Hi-Hats: Synthesize sixteenth-note closed hi-hats using high-pass filtered metallic square-wave ratios (> 6000 Hz) with a tight 25 ms exponential decay envelope.
  - Dub Sub-Bass: Synthesize a warm, low-pass filtered (cutoff = 180 Hz) sine + triangle bassline walking on roots and fifths (A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz) with a smooth 15 ms linear attack and 120 ms exponential release to eliminate clicks and dead-air gaps.
  - Hammond Organ Bubble: Additive drawbar sine harmonics (1.0x, 2.0x, 3.0x, 4.0x fundamental) with a gentle 6 Hz rotary speaker tremolo LFO, enveloped smoothly with ADSR.

### R2. Fix Digital Clipping, Headroom, and WAV Serialization
In the Rust mixer and mastering stage:
- Process all internal stems in normalized f32 (-1.0 to 1.0).
- Scale each individual stem by 0.20 (-14 dBFS RMS headroom) before summing stems together.
- Apply a Biquad 2nd-order Butterworth Low-Pass filter at 12,000 Hz on synthetic instruments to remove harsh digital aliasing harmonics, followed by a smooth `f32::tanh(sample * 1.2)` soft-clipping analog tape saturator and peak normalization to -1.0 dBFS (0.891) before writing 16-bit PCM samples via hound.

### R3. Enable Neural Vocals & Song Generation via uv Subprocess Bridge
When `engine_mode` is "auto" or "hf_hybrid":
- Automatically bootstrap a local Python environment inside `d:\harfile\music\.venv` using `uv venv` and install torch, torchaudio, transformers, diffusers, accelerate, and soundfile.
- Provide a working Python worker script (`workers/generate_song.py`) invoked by Rust via `tokio::process::Command` that uses Hugging Face models (ACE-Step-v1.5, YuE-s1-7B, or musicgen-medium based on detected VRAM from `SystemMemory::detect_live()`) to synthesize neural audio onto disk.
- When GPU VRAM is insufficient or neural weights/dependencies cannot be fetched, automatically fall back to the pure-Rust physical modeling synthesizer with a clear console notification.
- Log clear progress in the Tokio console reporting whether a track was rendered by Hugging Face GPU weights or the pure-Rust physical modeler.

### R4. Regenerate Stored Sample Songs & Run Verification
- Re-run `--create-reggae-anthem` and verify via the Rust spectral/clipping inspector that zero samples clip and no silence dropouts occur.
- Regenerate stored sample songs in `d:\harfile\music\storage\songs\` (`roots_reggae_anthem`, `song_afrobeats_002`, `song-one-love-shining-1977`, `song_reggae_anthem_001` across v1 and v2) so all on-disk `final_master.wav` and stem files reflect the new synthesis and mastering pipeline.

## Verification Resources
- Existing workspace tests run via `cargo test` across all crates (`music_core`, `music_dsp`, `music_fusion`, `music_pipeline`, `music_cli`).
- Rust spectral and clipping inspector: `--create-reggae-anthem`.

## Acceptance Criteria

### Audio Synthesis & DSP Quality
- [ ] Phase accumulator is continuous across buffer boundaries with zero abrupt phase resets to 0.0 mid-waveform.
- [ ] Pure-Rust engine synthesizes reggae offbeat skank chords and guitar leads via Karplus-Strong string algorithm with 0.994 feedback damping.
- [ ] Reggae one-drop drum and dub bass synthesizer produces kick on beat 3 (110 Hz to 42 Hz pitch drop), hi-hats (> 6000 Hz with 25 ms decay), dub sub-bass (180 Hz cutoff, 15 ms attack, 120 ms release), and enveloped Hammond organ bubble.
- [ ] Pure-Rust output adheres strictly to 76 BPM and A Major pentatonic / harmonic scale frequencies without chromatic siren sweeps.

### Mastering & Anti-Clipping
- [ ] Internal audio stems are processed in normalized f32 and scaled by 0.20 before summation.
- [ ] Output audio passes through 12 kHz Biquad low-pass filtering, `tanh` tape saturation, and peak normalization to -1.0 dBFS (0.891).
- [ ] Rust spectral/clipping inspector confirms 0 clipped samples (> 0 dBFS) on generated WAV files.

### Subprocess Bridge & Fallback
- [ ] Python worker script `workers/generate_song.py` exists and is invocable via `uv run` by Rust `tokio::process::Command`.
- [ ] System automatically falls back to pure-Rust physical modeling if GPU VRAM is insufficient or weights are unavailable.
- [ ] Tokio console logs explicitly state whether audio was generated via Hugging Face neural weights or pure-Rust synthesis.

### Regeneration & Test Pass
- [ ] All stored sample songs in `storage/songs/` are regenerated with updated `final_master.wav` and stems.
- [ ] `cargo test` passes across all crates in the workspace.
