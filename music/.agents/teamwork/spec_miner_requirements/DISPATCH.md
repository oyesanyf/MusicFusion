## 2026-11-01T00:57:59Z
You are a Spec Miner subagent in a Teamwork hierarchy.
Your working directory is: d:\harfile\music\.agents\teamwork\spec_miner_requirements\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md

YOUR MISSION:
Mine, structure, and formalize the complete specification matrix for refactoring d:\harfile\music into a 100% standalone Rust workspace.
You are read-only: do NOT edit source files.
Extract and document:
1. Architectural Invariants:
   - 100% Rust workspace, zero Python LangGraph project, zero runtime link to ModelFusion.
   - Deterministic VRAM freeing (wait for process termination between stages).
   - Pure-Rust 48 kHz DSP fallback engine.
2. Native Rust StateGraph Engine & State Specification:
   - MusicProductionState exact struct definition (all 13 fields with types).
   - Async tokio DAG/Pregel StateGraph semantics: add_node, add_edge, add_conditional_edges, version checkpointing.
3. The 6 Sequential Nodes Specification:
   - Node 1: ollama_producer_panel_node (bracketed tags: [Intro], [Verse 1], [Chorus], [Instrumental Interlude], [Verse 2], [Bridge], [Guitar Solo], [Outro]).
   - Node 2: groove_and_yue_synthesis_node (Stage 1 ACE-Step -> draft_groove.wav; Stage 2 YuE -> yue_vocals.wav + yue_instrumentals.wav).
   - Node 3: jamaican_tenor_timbre_node (Stage 3 Seed-VC -> raspy_tenor_vocals.wav).
   - Node 4: virtuoso_stem_inpainting_node (Stage 4 htdemucs_ft 4 stems -> stable-audio-open-1.0 inpainting 16-bar interlude, solo, nyabinghi, rimshot -> sample-accurate crossfade at 76 BPM).
   - Node 5: dsp_and_judge_gate_node (telemetry inspection, zero clipping, 76 BPM alignment, Judge evaluation, conditional retry logic: retry_count < 2 routes back to node 4 or node 2, else node 6).
   - Node 6: analog_mastering_node (Stage 5 AudioSR + Matchering 2.0 or 48 kHz analog tape saturation/sub-bass/EQ -> final_master.wav).
4. External Interfaces:
   - Tokio HTTP REST API (port 5050): endpoints /api/songs/create, /api/songs/iterate, /api/songs/regenerate-stem, /api/songs/master, /api/songs/:id/versions, streaming transitions.
   - MCP JSON-RPC 2.0: music_create_hit, music_iterate_song, music_regenerate_stem, music_master_track.
   - CLI preset: --create-reggae-anthem pre-loaded with 76 BPM "One Love" lyrics and style prompt.
5. Verification & Test Criteria for all 4 Tiers of tests.

Write your structured specification matrix into:
d:\harfile\music\.agents\teamwork\spec_miner_requirements\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\spec_miner_requirements\progress.md.
When finished, send a brief message to your parent with the handoff path.

## 2026-11-01T00:59:23Z
**Context**: Mining specifications
**Content**: CRITICAL REQUIREMENT UPDATE: A follow-up was appended to ORIGINAL_REQUEST.md. The engine MUST be a Universal Multi-Genre AI Music Production Engine supporting ALL genres of music known to humanity (Afrobeats, Highlife, Amapiano, Reggae, Soul/R&B, Gospel, Hip-Hop/Trap, Blues, Rock, Metal, Jazz, Classical, Pop, EDM, Latin, etc.) with dynamic genre taxonomy, BPM, scales/modes, and instruments, while keeping --create-reggae-anthem as a flagship preset.
**Action**: Please read the appended follow-up in ORIGINAL_REQUEST.md and incorporate this into your handoff report (dynamic genre taxonomy in State, Nodes 1-6, CLI, REST, and MCP).
