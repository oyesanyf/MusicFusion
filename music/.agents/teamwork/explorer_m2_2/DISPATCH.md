## 2026-11-01T01:59:50Z
You are an Explorer subagent in a Teamwork hierarchy for Milestone 2.
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m2_2\

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
Plan the detailed implementation of the Universal Drum & Rhythm Synthesizer and Physical Guitar String / Wah Modeler in `crates/music_dsp`.
You are read-only: do NOT modify source code files directly.
1. Design `crates/music_dsp/src/groove_synth.rs`:
   - Universal drum pattern synthesizer generating 48 kHz stereo audio buffers conditioned on `GenreTaxonomy`, `RhythmGroove`, and BPM (40 to 220).
   - Genre-authentic synthesis:
     * One-Drop Reggae (76 BPM): kick & rimshot on beat 3, polyrhythmic nyabinghi percussion, shuffle hi-hats.
     * Four-on-the-Floor (120–128 BPM): EDM/House/Pop punchy kick on every beat, clap on 2 & 4.
     * Boom-Bap & Trap (70–144 BPM): deep pitch-bent 808 sub-bass kick, hard snare on 3, trap triplets.
     * Afrobeats & Amapiano (108–114 BPM): log-drum pitch drops, clave syncopation, shaker loops.
     * Rock & Metal (128–180 BPM): driving acoustic double-kick, heavy backbeat.
     * Jazz Swing (110–160 BPM): swung ride cymbal (triplet feel), feathered kick.
     * Orchestral (60–100 BPM): timpani rolls, concert snare, compound meters (6/8, 12/8).
2. Design `crates/music_dsp/src/guitar_wah.rs`:
   - Karplus-Strong physical string modeling algorithm with pick noise excitation and feedback loop.
   - Dynamic resonant Wah-Wah filter (state-variable or biquad bandpass filter with modulated center frequency).
   - Solo generator creating 16-bar soaring psychedelic blues/rock guitar solos in A Major / modal scales.
3. Detail comprehensive unit tests for groove pattern synthesis and string physical modeling.

Write your report to:
d:\harfile\music\.agents\teamwork\explorer_m2_2\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\explorer_m2_2\progress.md.
When finished, send a brief message with your handoff path.
