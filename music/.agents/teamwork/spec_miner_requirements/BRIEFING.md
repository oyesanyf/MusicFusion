# BRIEFING — 2026-11-01T01:00:00Z

## Mission
Mine, structure, and formalize the complete specification matrix for refactoring d:\harfile\music into a 100% standalone Universal Multi-Genre AI Music Production Engine Rust workspace.

## 🔒 My Identity
- Archetype: Specification Miner
- Roles: Specification Mining, Requirements Formalization, Invariant Verification
- Working directory: d:\harfile\music\.agents\teamwork\spec_miner_requirements
- Original parent: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Milestone: Specification Mining & Architecture Matrix

## 🔒 Key Constraints
- Read-only on source files (do NOT edit code files; write only to own directory .agents/teamwork/spec_miner_requirements/).
- 100% Rust workspace, zero Python LangGraph project, zero runtime link to ModelFusion.
- Universal Multi-Genre support: ALL musical genres known to humanity (Afrobeats, Highlife, Amapiano, Reggae, Soul/R&B, Gospel, Hip-Hop/Trap, Blues, Rock, Metal, Jazz, Classical, Pop, EDM, Latin, etc.) with dynamic genre taxonomy, BPM, scales/modes, and instruments, while keeping --create-reggae-anthem as a flagship preset.
- Deterministic VRAM freeing: wait for process termination between stages.
- Pure-Rust 48 kHz DSP fallback engine.
- Exact MusicProductionState struct specification (all 13 fields with types).
- 6 sequential async nodes in StateGraph with retry edge.
- REST (port 5050), MCP JSON-RPC 2.0, and CLI external interfaces.
- 4 tiers of verification testing.

## Current Parent
- Conversation ID: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Updated: 2026-11-01T01:00:00Z

## Loaded Skills
- None loaded (no Antigravity skill paths requested in dispatch)

## Task Summary
- **What to build**: Comprehensive, authoritative, and formal specification matrix for the Universal Multi-Genre Music Production Engine in standalone Rust.
- **Success criteria**: Complete handoff report with Observation, Logic Chain, Caveats, Conclusion, Verification Method, plus Features Discovered and Edge Cases tables covering all invariants, state schema, node specifications, DSP fallback, external interfaces, and test criteria.
- **Interface contracts**: ORIGINAL_REQUEST.md + crates/music_core/src/schema.rs
- **Code layout**: Standalone multi-crate workspace (music_core, music_fusion, music_dsp, music_pipeline, music_cli)

## Key Decisions Made
- Architecture must support universal multi-genre taxonomy dynamically through MusicProductionState, Ollama prompt personas, groove synthesis patterns, timbre conversions, stem inpainting, and mastering profiles, while retaining the 1977 Roots Reggae "One Love" as the flagship preset.
- All 13 fields of MusicProductionState must be fully specified with exact Rust types and serialization attributes.
- Sequential StateGraph must have explicit async execution semantics, version checkpointing, and conditional routing (quality_passed == false && retry_count < 2).

## Artifact Index
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` — Authoritative requirements and follow-up
- `d:\harfile\music\.agents\teamwork\spec_miner_requirements\DISPATCH.md` — Inbound messages
- `d:\harfile\music\.agents\teamwork\spec_miner_requirements\progress.md` — Liveness & task tracker
- `d:\harfile\music\.agents\teamwork\spec_miner_requirements\handoff.md` — Final formal specification matrix
