# DISPATCH: Audio DSP & Vocal TTS Explorer (explorer_m6_1)

## Objective
Investigate Audio DSP scratch removal, ModelFusion Text-to-Voice (TTS) vocal engine integration, and disk sample song regeneration.

## Scope & Files
- `d:\harfile\music\crates\music_dsp\src\guitar_wah.rs`
- `d:\harfile\music\crates\music_dsp\src\groove_synth.rs`
- `d:\harfile\music\crates\music_dsp\src\crossfader.rs`
- `d:\harfile\music\crates\music_pipeline\src\nodes\node2_groove_yue.rs`
- `d:\harfile\music\crates\music_pipeline\src\nodes\node3_vocal_timbre.rs`
- `d:\harfile\music\crates\music_pipeline\src\nodes\node4_stem_inpainting.rs`
- `d:\harfile\music\crates\music_pipeline\src\nodes\node6_analog_mastering.rs`
- `d:\harfile\music\crates\music_fusion\`
- Sample songs on disk: `d:\harfile\music\storage\songs\` (`roots_reggae_anthem`, `song_afrobeats_002`, `song-one-love-shining-1977`, `song_reggae_anthem_001`)

## Requirements
1. Identify all sources of harsh static or scratch noise (snare/hi-hat/shaker white noise bursts, Karplus-Strong string pluck zipper noise, crossfader clicks).
2. Check how articulated English vocal synthesis is wired (`Microsoft David Desktop` lead + `Microsoft Zira Desktop` choir + melodic harmonic carrier + vocal-priority sidechain ducking) and how ModelFusion TTS models (`f5-tts`, `kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`) are integrated.
3. Detail how to regenerate all sample songs on disk so all WAV files have loud, clear articulated English vocals and zero scratch.

## Output
Write comprehensive handoff report to `d:\harfile\music\.agents\teamwork\explorer_m6_1\handoff.md`.

## 2026-11-01T12:51:13Z
[Message] timestamp=2026-11-01T12:51:13Z sender=22bd2f00-efbc-429d-9417-8639ee3e2bc7 priority=MESSAGE_PRIORITY_HIGH content=You are explorer_m6_1 (Audio DSP & Vocal TTS Explorer).
Your working directory is `d:\harfile\music\.agents\teamwork\explorer_m6_1\`.
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` before starting work.
Read `d:\harfile\music\.agents\teamwork\explorer_m6_1\DISPATCH.md` for your full task description.

Task:
1. Investigate scratch noise in `crates/music_dsp/src/guitar_wah.rs` and `crates/music_dsp/src/groove_synth.rs` (snare/hi-hat/shaker white noise bursts or crossfader clicks in `node4_stem_inpainting.rs`). Verify smooth band-passing / low-passing / attenuation of white-noise bursts and Karplus-Strong string pluck excitation so zero static or scratch exists in any generated audio.
2. Investigate ModelFusion Text-to-Voice (TTS) vocal engine integration across `crates/music_fusion` and pipeline nodes (`node2_groove_yue.rs`, `node3_vocal_timbre.rs`, `node6_analog_mastering.rs`) and REST endpoint `POST /api/songs/tts-vocals`. Check support for ModelFusion TTS models (`f5-tts`, `kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`) plus 48kHz stereo fallback via Windows SpeechSynthesizer (`Microsoft David Desktop` lead + `Microsoft Zira Desktop` choir + harmonic carrier).
3. Investigate the steps required to regenerate all sample songs on disk (`roots_reggae_anthem` v1/v2, `song_afrobeats_002` v1/v2, `song-one-love-shining-1977` v1/v2, `song_reggae_anthem_001` v1/v2) with loud, clear articulated English vocals and zero scratch.

Write a complete, structured handoff report to `d:\harfile\music\.agents\teamwork\explorer_m6_1\handoff.md`.
When finished, notify the orchestrator with send_message.
