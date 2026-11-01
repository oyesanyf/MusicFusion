## 2026-11-01T02:00:01Z
Sender: 4caf3f77-b3ac-4b67-8bda-934a91faafab
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are an Explorer subagent in a Teamwork hierarchy for Milestone 2.
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m2_3\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md
and test infra contracts at:
d:\harfile\music\TEST_INFRA.md
and existing reference DSP implementations in tests:
d:\harfile\music\tests\common\mod.rs

YOUR MISSION:
Plan the detailed implementation of the Vocal Formant/Rasp Vocoder, Sample-Accurate Crossfader, and 48 kHz Analog Tape/Sub-Bass Mastering in `crates/music_dsp`.
You are read-only: do NOT modify source code files directly.
1. Design `crates/music_dsp/src/vocal_formant.rs`:
   - Formant filter bank (F1, F2, F3 resonant bandpass filters) modeling human vocal tract.
   - Asymmetric non-linear harmonic saturation for the 1977 raspy Jamaican male tenor timbre and universal vocal archetypes.
   - Pitch-lock vocoder / phase alignment helper.
2. Design `crates/music_dsp/src/crossfader.rs`:
   - Sample-accurate stem inpainting crossfader locked to BPM beat grid.
   - Equal-power (sine/cosine) and linear crossfade curves preventing phase cancellation and energy dips.
   - Splice lead guitar solos and drum fills into existing stem tracks seamlessly.
3. Design `crates/music_dsp/src/tape_mastering.rs`:
   - Analog tape saturation using $\tanh(x)$ curve with drive and bias controls.
   - 55–85 Hz sub-bass warmth resonant filter.
   - Reference spectral EQ matchering across 5 profiles (1970s Studer A800 Tape, Modern Punchy Pop, Club EDM, Cinematic Orchestral, Audiophile Jazz).
   - Peak brickwall limiter guaranteeing zero digital clipping ($|sample| \le 0.98$).
4. Design `crates/music_dsp/src/lib.rs` public exports and comprehensive unit test suite in `crates/music_dsp/tests/dsp_tests.rs`.

Write your report to:
d:\harfile\music\.agents\teamwork\explorer_m2_3\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\explorer_m2_3\progress.md.
When finished, send a brief message with your handoff path.
