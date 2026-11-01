## 2026-11-01T01:11:36Z
You are a Worker subagent in a Teamwork hierarchy for Milestone 1.
Your working directory is: d:\harfile\music\.agents\teamwork\worker_m1\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md

READ THE EXPLORER HANDOFF REPORTS BEFORE CODING:
1. crates/music_core specifications:
   d:\harfile\music\.agents\teamwork\explorer_m1_1\handoff.md
2. crates/music_fusion specifications:
   d:\harfile\music\.agents\teamwork\explorer_m1_2\handoff.md
3. Workspace integration & compilation plan:
   d:\harfile\music\.agents\teamwork\explorer_m1_3\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

EXCLUSIVE WRITE OWNERSHIP:
You exclusively own:
- crates/music_core/ (Cargo.toml, src/lib.rs, src/schema.rs, src/state.rs, src/telemetry.rs, src/genre.rs, src/blueprint.rs, src/error.rs, tests/core_tests.rs)
- crates/music_fusion/ (Cargo.toml, src/lib.rs, src/memory.rs, src/ollama_client.rs, src/prompt_templates.rs, src/panel.rs, src/judge.rs, src/error.rs, tests/fusion_tests.rs)
- Minimal workspace stubs for crates/music_dsp, crates/music_pipeline, and crates/music_cli (Cargo.toml + src/lib.rs / src/main.rs) so that the entire root workspace compiles cleanly without cargo manifest errors.

IMPLEMENTATION REQUIREMENTS:
1. Implement `crates/music_core`:
   - `MusicProductionState` (13+ fields, Serde, clone, state mutation helpers).
   - `StemTelemetry` (peak dBFS, RMS, dynamic range, tempo drift, clipping, SNR, spectral flatness).
   - `GenreTaxonomy` & `GenrePreset` supporting ALL genres known to humanity (Roots Reggae, Afrobeats, Highlife, Amapiano, R&B, Gospel, Hip-Hop, Blues, Rock, Metal, Jazz, Classical, Pop, EDM, Latin, etc.) with dynamic fallback and the 1977 Roots Reggae "One Love" flagship preset.
   - Comprehensive unit tests in `crates/music_core/tests/core_tests.rs`.
2. Implement `crates/music_fusion`:
   - `SystemMemory::detect_live()` querying sysinfo and nvidia-smi with CREATE_NO_WINDOW (0x08000000) on Windows.
   - Dynamic Available Memory Law model sizing (qwen2.5:32b, 14b, 7b, 3b, 1.5b), flint:latest and glm4:9b gating.
   - Stage routing between Hugging Face GPU vs. pure-Rust 48 kHz DSP fallback.
   - Resilient `OllamaClient` with 400ms fast health probe, /api/tags, /api/chat, interactive missing model guidance.
   - Multi-model parallel `OllamaProducerPanel` (Lyricist, Arranger, Sound Engineer) and `JudgeEvaluator` with <think> tag and markdown fence stripping, producing YuE genre tags and bracketed lyrics.
   - Comprehensive unit tests in `crates/music_fusion/tests/fusion_tests.rs`.
3. Build and test verification:
   - Run `cargo check --workspace` and `cargo test -p music_core -p music_fusion`.
   - Ensure 100% tests pass with zero warnings or errors.
4. Git Commit:
   - Once verified, stage and commit the changes cleanly.

Write your completion report into:
d:\harfile\music\.agents\teamwork\worker_m1\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\worker_m1\progress.md.
When finished, send a brief message with the handoff path.
