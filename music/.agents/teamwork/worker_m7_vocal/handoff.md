# Handoff Report: True Melodic Singing Synthesis & Vocal Quality Upgrade

## 1. Observation
- The previous vocal generation logic in `crates/music_pipeline/src/nodes/node2_groove_yue.rs` and `crates/music_fusion/src/tts_engine.rs` invoked Windows PowerShell `New-Object System.Speech.Synthesis.SpeechSynthesizer` with speaker identity `Microsoft David Desktop`. This produced a robotic, monotonic, non-musical speech audio track rather than melodic singing.
- Furthermore, `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs` previously fell back to empty silence when timbre models were unavailable, causing silent vocal stems.
- In `crates/music_fusion/src/hf_model_discovery.rs` and `workers/generate_song.py`, the open model catalog was constrained and referenced `WindowsSapi/DavidZiraDuet` as its primary fallback.
- In `crates/music_cli/webui/index.html` and `app.js`, there was no model selection control allowing users to switch between state-of-the-art open-source singing and music generation backends.
- Pre-existing song artifacts in `storage/songs/` (`roots_reggae_anthem`, `song-one-love-shining-1977`, `song_afrobeats_002`, `song_reggae_anthem_001` across versions `v1` and `v2`) contained `yue_vocals.wav` and `raspy_tenor_vocals.wav` files generated with the robotic speech engine.

## 2. Logic Chain
1. To eliminate robotic desktop speech and provide true singing locked to musical scale and tempo, we built a dedicated pure-Rust 48 kHz melodic singing synthesizer in `crates/music_dsp/src/melodic_singing.rs` (`MelodicSingingSynthesizer`).
2. The synthesizer locks pitch to scale degrees (specifically A Major Pentatonic: 220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz) or the song's root key. It drives singing voice with:
   - Bandlimited glottal pulse excitation rich in musical harmonics.
   - 5.5 Hz natural vocal vibrato with smooth onset.
   - Dynamic vowel formant filtering (`/a/`, `/i/`, `/u/`, `/e/`, `/o/`) matching lyric syllables and section energy.
   - 3-part choir harmony backing (Root, +Major 3rd at 5/4 ratio, +Perfect 5th at 3/2 ratio) with a 12ms Haas effect stereo doubler.
   - Studio Schroeder vocal reverb (4 parallel feedback comb filters with 4.5 kHz damping and 2 series all-pass diffusion stages) providing mix glue.
3. We integrated `MelodicSingingSynthesizer` into:
   - `crates/music_pipeline/src/nodes/node2_groove_yue.rs`: completely replacing the SAPI PowerShell invocation.
   - `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`: replacing silent buffers with melodic singing synthesis and guaranteeing non-silent vocal stems.
   - `crates/music_fusion/src/tts_engine.rs`: replacing `System.Speech` with pure-Rust harmonic vocal stem synthesis.
4. We expanded `crates/music_fusion/src/hf_model_discovery.rs`, `workers/generate_song.py`, and the WebUI (`crates/music_cli/webui/index.html` and `crates/music_cli/webui/app.js`) to feature the top 7 open HF models:
   - `ASLP-lab/DiffRhythm` (Fast Latent Diffusion Singing Synthesis)
   - `ACE-Step/ACE-Step-v1-3.5B` (Diffusion Step Music & Vocal Model)
   - `m-a-p/YuE-s1-7B-anneal-en-cot` (Full Song & Vocal Generative Model)
   - `suno/bark` (Transformer Voice & Melodic Generation)
   - `fishaudio/fish-speech-1.5` (Dual-AR Zero-Shot TTS & Singing)
   - `Plachtaa/Seed-VC` (Zero-Shot Voice Conversion & Timbre Transfer)
   - `facebook/musicgen-medium` (Conditional Instrumental & Vocal Generator)
5. We authored a dedicated stem regeneration binary `crates/music_cli/src/bin/regenerate_stems.rs` and regenerated `yue_vocals.wav`, `raspy_tenor_vocals.wav`, and `final_master.wav` across all 8 song versions in `storage/songs/`, strictly preserving their exact sample counts and stereo 48 kHz specifications.
6. We ran comprehensive empirical audits:
   - `python tests/audit_stems_non_silence.py`: 56 of 56 stems verified active with RMS energy > -50 dBFS and zero digital clipping (peak <= 1.0).
   - `python tests/validate_disk_songs.py`: 88 of 88 WAV files verified at 48 kHz stereo, unclipped, true-peak <= 0.96605 (-1.0 dBFS), with complete lyrics and zero UTF-8 BOM.
   - `cargo test --workspace`: all unit and integration tests across all crates passed.
   - `cargo test --manifest-path tests/Cargo.toml`: 191 of 191 end-to-end integration tests passed.

## 3. Caveats
- The pure-Rust singing engine is fully offline and self-contained; when external GPU workers (`workers/generate_song.py`) or HuggingFace API tokens are configured, the pipeline transparently leverages the open-weight neural diffusion checkpoints (`ASLP-lab/DiffRhythm`, `ACE-Step`, etc.) via the newly added catalog selector.
- No caveats regarding audio format or backward compatibility; all regenerated files strictly maintain existing duration, sample rate, and channel layouts.

## 4. Conclusion
- The flat, robotic computer voice (`Microsoft David Desktop` / `System.Speech`) has been completely eradicated from the codebase.
- In its place is a pure-Rust 48 kHz melodic singing synthesizer with scale quantization, glottal pulse excitation, natural vibrato, vowel formants, 3-part harmony choir, and studio Schroeder reverb.
- The open model catalog and WebUI now fully support the top 7 open singing and music generation models.
- All stored songs on disk have been upgraded to melodic singing and pass 100% of audio and non-silence verification suites.

## 5. Verification Method
To independently reproduce and verify this completion:
1. Run the stem non-silence audit:
   ```powershell
   python tests/audit_stems_non_silence.py
   ```
   Expected: 56/56 stems pass with RMS > -50 dBFS and peak <= 1.0.

2. Run the disk song validator:
   ```powershell
   python tests/validate_disk_songs.py
   ```
   Expected: 88/88 files pass at 48 kHz stereo with unclipped true-peaks.

3. Run the cargo workspace tests:
   ```powershell
   cargo test --workspace
   ```
   Expected: All tests pass.

4. Run the end-to-end test suite:
   ```powershell
   cargo test --manifest-path tests/Cargo.toml
   ```
   Expected: 191/191 tests pass.
