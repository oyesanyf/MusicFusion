# DISPATCH: explorer_m7_2

## Objective
Technical exploration and architecture mapping for Requirement R3:
- Subprocess bridge via `uv` for neural vocals & song generation when `engine_mode` is "auto" or "hf_hybrid".
- Automatic bootstrapping of local Python virtual environment inside `d:\harfile\music\.venv` using `uv venv`.
- Installing Python packages via uv: `torch`, `torchaudio`, `transformers`, `diffusers`, `accelerate`, and `soundfile`.
- Python worker script `workers/generate_song.py` invoked by Rust via `tokio::process::Command`.
- Hugging Face models selection based on live system VRAM from `SystemMemory::detect_live()`: `ACE-Step-v1.5`, `YuE-s1-7B`, or `musicgen-medium`.
- Fallback logic: when GPU VRAM is insufficient or neural weights/dependencies cannot be fetched, automatically fall back to pure-Rust physical modeling synthesizer with a clear console notification.
- Tokio console logging: clearly report whether a track was rendered by Hugging Face GPU weights or pure-Rust physical modeler.

## Authoritative Files to Read
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- `d:\harfile\music\PROJECT.md`
- `crates/music_fusion/src/memory.rs` (`SystemMemory::detect_live()`)
- `crates/music_pipeline/src/nodes/node2_groove_yue.rs`
- `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`
- `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`
- `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`
- `crates/music_pipeline/src/runner.rs`
- Any existing `workers/` or python scripts in the repo.

## Expected Output
Deliver a comprehensive analysis report in `d:\harfile\music\.agents\teamwork\explorer_m7_2\handoff.md` detailing:
1. Current subprocess execution and Python environment handling in `music_pipeline` and `music_fusion`.
2. How to implement the `uv` bootstrap routine in Rust (locating `uv`, running `uv venv d:\harfile\music\.venv`, running `uv pip install ...` or `uv run`).
3. Complete architecture and specification for `workers/generate_song.py` (CLI arguments, model loading, audio synthesis, output file structure).
4. Concrete Rust orchestration code in `node2_groove_yue.rs` / `music_pipeline` to invoke `workers/generate_song.py` via `tokio::process::Command`, inspect exit codes/timeouts, and fall back cleanly to pure-Rust physical modeling.
5. Exact Tokio console logging messages for Hugging Face GPU vs. pure-Rust physical modeler rendering.
6. Concrete file-by-file modification plan for the Worker.


## 2026-11-01T15:59:44Z
You are explorer_m7_2 (Python Neural Bridge Explorer).
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m7_2\
Read your instructions in d:\harfile\music\.agents\teamwork\explorer_m7_2\DISPATCH.md and the authoritative request in d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md.

Investigate Requirement R3:
- Subprocess bridge via `uv` for neural vocals & song generation when `engine_mode` is "auto" or "hf_hybrid".
- Automatic bootstrap of local Python virtual environment inside `d:\harfile\music\.venv` using `uv venv`.
- Installing Python packages via uv: `torch`, `torchaudio`, `transformers`, `diffusers`, `accelerate`, and `soundfile`.
- Python worker script `workers/generate_song.py` invoked by Rust via `tokio::process::Command`.
- Hugging Face models selection based on live system VRAM from `SystemMemory::detect_live()`: `ACE-Step-v1.5`, `YuE-s1-7B`, or `musicgen-medium`.
- Fallback logic: when GPU VRAM is insufficient or neural weights/dependencies cannot be fetched, automatically fall back to pure-Rust physical modeling synthesizer with a clear console notification.
- Tokio console logging: clearly report whether a track was rendered by Hugging Face GPU weights or pure-Rust physical modeler.

Write your complete findings and implementation plan into d:\harfile\music\.agents\teamwork\explorer_m7_2\handoff.md. Send a message to orchestrator when done.
