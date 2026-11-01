# DISPATCH: reviewer_m7_2

## Objective
Independent Review of Milestone 7 Implementation — Focus on Requirements R3 & R4:
- R3: Neural Model Selection & uv Python Worker Bridge
  - `NeuralModelChoice` in `crates/music_fusion/src/memory.rs` mapping live VRAM from `SystemMemory::detect_live()`:
    * >= 12GB -> YuE-s1-7B
    * >= 6GB -> ACE-Step-v1.5
    * >= 3.5GB -> musicgen-medium
    * < 3.5GB or `pure_rust` -> PureRustFallback
  - `crates/music_pipeline/src/python_bridge.rs` (`UvBridge`):
    * Locates `uv`, bootstraps `d:\harfile\music\.venv` via `uv venv`.
    * Installs dependencies (`torch`, `torchaudio`, `transformers`, `diffusers`, `accelerate`, `soundfile`).
    * Caches install status via `.deps_installed`.
    * Invokes `workers/generate_song.py` with `CREATE_NO_WINDOW` on Windows.
  - `workers/generate_song.py`:
    * Accepts CLI args (`--model`, `--prompt`, `--lyrics_file`, `--bpm`, `--key`, `--duration`, `--output_dir`, `--device`).
    * Exits with code 0 on success, code 2 on missing weights/offline, code 3 on CUDA OOM.
  - Wiring in `crates/music_pipeline/src/nodes/node2_groove_yue.rs`:
    * Graceful fallback to pure-Rust physical modeling if GPU/weights/uv unavailable.
    * Tokio console logs:
      - HF GPU: `🎛️  [SYNTHESIS ENGINE] Track rendered by Hugging Face GPU weights (Model: ..., Device: CUDA, VRAM: ... GB free)`
      - Fallback: `⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: .... Automatically falling back to pure-Rust physical modeling synthesizer.`
      - Pure-Rust: `🎹  [SYNTHESIS ENGINE] Track rendered by pure-Rust physical modeling synthesizer (Karplus-Strong string algorithm, Reggae one-drop drums & dub bass)`
- R4: Stored Sample Songs Regeneration & All Instruments Constraint
  - Verify all 4 sample songs in `storage/songs/` across v1 and v2:
    * `roots_reggae_anthem` (v1, v2)
    * `song-one-love-shining-1977` (v1, v2)
    * `song_afrobeats_002` (v1, v2)
    * `song_reggae_anthem_001` (v1, v2)
  - Verify `python tests/validate_disk_songs.py` passes 100%.
  - Verify "ALL INSTRUMENTS MUST BE USED" invariant:
    1. One-Drop Kick & Rimshot/Snare
    2. 16th-Note Closed Hi-Hats & Nyabinghi
    3. Dub Sub-Bass
    4. Offbeat Skank Guitar Chords on beats 2 & 4
    5. Pentatonic Lead Wah Guitar Solo
    6. Hammond B3 Organ Bubble + Brass/Keys
    7. Lead & Choir Vocals
    Ensure no stem is silent or muted!
  - Verify workspace tests: `cargo test --workspace`.

## Expected Output
Deliver your structured review report and explicit verdict (**APPROVE** or **REQUEST_CHANGES**) in:
`d:\harfile\music\.agents\teamwork\reviewer_m7_2\handoff.md`.


## 2026-11-01T17:17:03Z
[Message] timestamp=2026-11-01T17:17:03Z sender=31352664-c681-465a-a22e-aed6e548c6f0 priority=MESSAGE_PRIORITY_HIGH
You are reviewer_m7_2 (Milestone 7 Reviewer 2: Neural Bridge, Sample Songs & All Instruments).
Your working directory is: d:\harfile\music\.agents\teamwork\reviewer_m7_2\

Read:
1. d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md (MUST READ FIRST)
2. d:\harfile\music\.agents\teamwork\reviewer_m7_2\DISPATCH.md
3. d:\harfile\music\.agents\teamwork\worker_m7\handoff.md

Review R3 and R4:
- NeuralModelChoice in memory.rs mapping live VRAM from detect_live() (>=12GB YuE-s1-7B, >=6GB ACE-Step-v1.5, >=3.5GB musicgen-medium, <3.5GB pure_rust fallback).
- UvBridge in python_bridge.rs (.venv bootstrap via uv venv, dependency install via uv pip install, .deps_installed marker, CREATE_NO_WINDOW).
- workers/generate_song.py (CLI args, exit codes 0, 2, 3).
- node2_groove_yue.rs: model selection, uv bridge invocation, deterministic fallback, Tokio console logging.
- Sample songs regeneration in storage/songs/ across v1 and v2 for all 4 songs.
- validate_disk_songs.py passing 100%.
- "ALL INSTRUMENTS MUST BE USED" invariant: verify all 7 instrument groups (kick/rimshot, hi-hats/nyabinghi, dub sub-bass, skank chords on 2 & 4, lead wah solo, Hammond organ bubble + brass/keys, lead & choir vocals) are actively synthesized and non-silent in final_master.wav and individual stems.
- Run tests: `cargo test --workspace`.

Deliver your structured review report and explicit verdict (APPROVE or REQUEST_CHANGES) in `d:\harfile\music\.agents\teamwork\reviewer_m7_2\handoff.md`. Send a message when finished.
