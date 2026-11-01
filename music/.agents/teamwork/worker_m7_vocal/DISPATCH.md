# DISPATCH: worker_m7_vocal

## Objective
CRITICAL REMEDIATION: Replace flat spoken desktop screen-reader TTS ("computer voice that is talking not music") with TRUE MELODIC SINGING SYNTHESIS.

## Problem Statement
The user reported: "the voice outs like a computer voice and it is talking not music".
Root cause: The vocal synthesis (`try_synthesize_windows_sapi_vocals` in `crates/music_pipeline/src/nodes/node2_groove_yue.rs`, `crates/music_fusion/src/tts_engine.rs`, etc.) was calling Windows `System.Speech.Synthesis.SpeechSynthesizer` (`Microsoft David Desktop`), which is a flat screen-reader voice that speaks monotonously like a computer rather than singing a melody.

## Requirements

### 1. Replace Flat Spoken TTS with True Melodic Singing Synthesis
In `crates/music_pipeline/src/nodes/node2_groove_yue.rs` (and `node3_vocal_timbre.rs` / `crates/music_fusion/src/tts_engine.rs`):
- Completely eliminate flat spoken screen-reader talking.
- Implement true melodic singing synthesis:
  1. **Pitch-Lock & Melodic Quantization**: Resynthesize and lock every sung phrase/syllable strictly onto the musical scale notes:
     - For Roots Reggae / Flags: A Major Pentatonic melody notes (A3 = 220.0 Hz, B3 = 246.94 Hz, C#4 = 277.18 Hz, E4 = 329.63 Hz, F#4 = 369.99 Hz, A4 = 440.0 Hz).
     - Modulate with natural musical vibrato (5.5 Hz, ±1.5% pitch depth).
  2. **Sustained Sung Vowels & Rhythmic Phrasing**:
     - Unlike spoken syllables that drop off abruptly, singing sustains vowel formants (F1, F2, F3) across the beat duration locked to 76 BPM (0.7894s / beat).
     - Formant modeling: warm male tenor formants (F1 ≈ 600–700 Hz, F2 ≈ 1100–1300 Hz, F3 ≈ 2400–2600 Hz) with harmonic glottal pulse excitation, smooth exponential attack/decay, and zero silence dropouts.
  3. **3-Part Harmony Choir Backing**:
     - Synthesize rich 3-part vocal harmonies:
       * Lead Vocal: Melody fundamental note
       * High Harmony (Choir): Major 3rd (+4 semitones or pentatonic 3rd, e.g. 1.25x or pentatonic step)
       * Low Harmony (Choir): Perfect 5th (+7 semitones, e.g. 1.5x)
     - Spread in stereo with gentle detuning (±4 cents) and 12ms stereo doubler.
  4. **Studio Reverb & Warmth**:
     - Apply warm studio vocal reverb (early reflections + comb/allpass Schroeder diffuser) so vocals sit lushly in the mix.
  5. **Neural / Python Option**:
     - If `workers/generate_song.py` or Python neural singing models (e.g. bark/edge-tts/diffsinger) are active, use neural singing; otherwise, the pure-Rust harmonic melodic singing synthesizer ensures loud, warm, musical singing with zero dependence on external speech engines.

### 2. Regenerate All Stored Sample Songs
Regenerate all 4 sample songs across v1 and v2:
- `roots_reggae_anthem` (v1, v2)
- `song-one-love-shining-1977` (v1, v2)
- `song_afrobeats_002` (v1, v2)
- `song_reggae_anthem_001` (v1, v2)
Ensure that on disk:
- `yue_vocals.wav`, `raspy_tenor_vocals.wav`, and `final_master.wav` feature true musical singing in pitch and rhythm, and NEVER flat spoken screen-reader speech.

### 3. Verification
- Run `python tests/audit_stems_non_silence.py` -> 100% pass!
- Run `python tests/validate_disk_songs.py` -> 100% pass!
- Run `cargo test --workspace` -> 100% pass!
- Run `cargo test --manifest-path tests/Cargo.toml` -> 100% pass!
- Commit and push verified changes per auto-commit protocol.

## Mandatory Reading
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.


## 2026-11-01T18:12:28Z
**Context**: User Requirement Enhancement (Hugging Face Open Model Catalog & Melodic Singing Completion)
**Content**: Sentinel has relayed an enhancement from the user:
Ensure our Hugging Face model catalog (`crates/music_fusion/src/hf_model_discovery.rs`), `workers/generate_song.py`, and the WebUI model selectors (`index.html` / `app.js`) include the top free Apache-2.0 / MIT open-weight models:
1. DiffRhythm (`ASLP-lab/DiffRhythm` — Apache 2.0, 44.1 kHz stereo full-song lyrics+music diffusion)
2. ACE-Step (`ACE-Step/ACE-Step-v1-3.5B` — Apache 2.0)
3. YuE (`m-a-p/YuE-s1-7B-anneal-en-cot` — Apache 2.0)
4. Bark (`suno/bark` — MIT singing & music)
5. Fish Speech (`fishaudio/fish-speech-1.5` — Apache 2.0)
6. RVC / Seed-VC (`Plachtaa/Seed-VC` — MIT singing voice conversion)
7. MusicGen (`facebook/musicgen-medium`)

In addition, ensure your primary task completes cleanly: vocals across all stored songs (`storage/songs/`) are melodically sung (A Major Pentatonic scale, 5.5 Hz vibrato, sustained vowels, 3-part harmony choir, studio reverb) and NEVER flat spoken screen-reader desktop TTS.
**Action**: Incorporate the model catalog entries into `crates/music_fusion/src/hf_model_discovery.rs`, `workers/generate_song.py`, and WebUI alongside your melodic singing engine implementation. Report status in your handoff.
