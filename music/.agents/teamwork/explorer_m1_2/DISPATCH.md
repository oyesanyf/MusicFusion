## 2026-11-01T01:05:04Z
From: 4caf3f77-b3ac-4b67-8bda-934a91faafab
Priority: MESSAGE_PRIORITY_HIGH

You are an Explorer subagent in a Teamwork hierarchy for Milestone 1.
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m1_2\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md
and the ModelFusion blueprint at:
d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\handoff.md

YOUR MISSION:
Plan the detailed implementation of `crates/music_fusion` for Milestone 1.
You are read-only: do NOT modify source code files directly.
1. Design exact implementation for:
   - `crates/music_fusion/Cargo.toml`: dependencies (sysinfo, reqwest, serde, tokio, etc., plus path dependency on music_core).
   - `crates/music_fusion/src/memory.rs`: `SystemMemory::detect_live()` querying sysinfo and nvidia-smi with CREATE_NO_WINDOW on Windows. Implement dynamic sizing rules (qwen2.5:32b, glm4:9b, flint:latest, qwen2.5:7b) and stage routing (Hugging Face GPU vs. 48 kHz pure-Rust DSP fallback).
   - `crates/music_fusion/src/ollama_client.rs`: HTTP client for Ollama at 127.0.0.1:11434 with /api/tags, /api/chat, fast timeout, and zero-crash fallbacks.
   - `crates/music_fusion/src/panel.rs`: Multi-model parallel panel (Lyricist, Arranger, Sound Engineer) adapted dynamically to any genre or the 1977 reggae anthem.
   - `crates/music_fusion/src/judge.rs`: Judge model that evaluates outputs, strips <think> tags and markdown fences, and produces final genre.txt, bracketed lyrics.txt, and bar arrangement map.
   - `crates/music_fusion/src/lib.rs`: Exposing memory, ollama_client, panel, judge, and error types.
2. Design unit tests verifying live and simulated memory detection, model fallback selection, and JSON synthesis.

Write your report to:
d:\harfile\music\.agents\teamwork\explorer_m1_2\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\explorer_m1_2\progress.md.
When finished, send a brief message with the handoff path.
