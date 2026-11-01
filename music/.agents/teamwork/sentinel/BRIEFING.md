# BRIEFING — 2026-11-01T19:05:00Z

## Mission
Manage orchestrator execution for physical modeling DSP synthesis, anti-clipping mastering, Python neural bridge, melodic singing synthesis, and song regeneration in d:\harfile\music.

## 🔒 My Identity
- Archetype: sentinel
- Working directory: d:\harfile\music\.agents\teamwork\sentinel
- Orchestrator: 31352664-c681-465a-a22e-aed6e548c6f0
- Victory Auditor: to be spawned on victory claim

## 🔒 Key Constraints
- No technical decisions — relay only
- Victory Audit is MANDATORY before reporting completion
- R1: Pure-Rust 48 kHz engine locked to 76 BPM and A Major pentatonic/harmonic scale with Karplus-Strong strings, continuous phase accumulator, one-drop drums, dub bass, and Hammond organ bubble
- R2: Internal stems in f32 scaled by 0.20 headroom, 12 kHz Biquad LPF, tanh tape saturation, peak normalization to -1.0 dBFS
- R3: Python worker via uv run for Hugging Face neural models with automatic fallback to pure-Rust physical modeling
- R4: Regenerate stored songs in storage/songs/ and verify zero clipping and full cargo test pass
- Automatic Git Commit & Push upon verification of changes
- ALL instruments must be actively synthesized, separated across stems, and mixed into final_master.wav: One-Drop Kick & Rimshot/Snare, 16th-Note Hi-Hats & Nyabinghi Percussion, Dub Sub-Bass, Karplus-Strong Offbeat Skank Guitar, Karplus-Strong Lead Wah Guitar, Hammond B3 Organ Bubble, Lead & Backing Vocals — zero dropped, muted, or silent stems
- Vocal synthesis must be SUNG MUSICALLY: melodic pitch-locked sustained vowels on A Major Pentatonic notes (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz), 5.5 Hz vibrato, 3-part harmony choir backing (Root + Major 3rd + Perfect 5th), 76 BPM rhythmic phrasing, and studio formant reverb — NEVER flat spoken desktop screen-reader TTS (Microsoft David)
- Hugging Face model catalog in hf_model_discovery.rs, workers/generate_song.py, and WebUI selectors must include top free Apache-2.0 / MIT open-weight models: DiffRhythm (ASLP-lab/DiffRhythm), ACE-Step (ACE-Step/ACE-Step-v1-3.5B), YuE (m-a-p/YuE-s1-7B-anneal-en-cot), Bark (suno/bark), Fish Speech (fishaudio/fish-speech-1.5), Seed-VC (Plachtaa/Seed-VC), and MusicGen (facebook/musicgen-medium)

## User Context
- **Last user request**: User shared HF music models comparison and requested inclusion of top free Apache-2.0 / MIT models (DiffRhythm, ACE-Step, YuE, Bark, Fish Speech, Seed-VC, MusicGen) in discovery and WebUI, along with completion of melodic singing synthesis.
- **Pending clarifications**: None.
- **Delivered results**: worker_m7_vocal completed implementation, regenerated all sample songs, and committed; Iteration 2 recheck gate dispatched.

## Project Status
- **Phase**: in progress (Milestone 7 Iteration 2 Recheck Gate: reviewer_m7_recheck, challenger_m7_recheck, auditor_m7_recheck active)
- **Route**: General (teamwork_orchestrator: 31352664-c681-465a-a22e-aed6e548c6f0)
- **Routing Rationale**: Multi-component SWE project across DSP algorithms, audio mastering, IPC subprocess bridge, and song regeneration across multiple crates; not a document review, math proof, or single self-contained light change.

## Victory Audit Status
- **Triggered**: no
- **Verdict**: pending
- **Retry count**: 0

## Artifact Index
- d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md — Authoritative record of approved task prompt and user follow-ups
- d:\harfile\music\.agents\teamwork\orchestrator\progress.md — Orchestrator progress log
- d:\harfile\music\.agents\teamwork\orchestrator\GATE_STATUS.md — Gate Status Matrix
- d:\harfile\music\.agents\teamwork\worker_m7_vocal\handoff.md — Vocal Remediation Worker handoff
