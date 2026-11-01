# Codebase Survey & Gap Analysis Report: d:\harfile\music

**Agent**: `explorer_survey_existing`  
**Mission**: Investigate existing codebase in `d:\harfile\music`, inspect DSP, REST, MCP, CLI, and test assets, map gaps against `ORIGINAL_REQUEST.md` (including Universal Multi-Genre updates), and recommend concrete architectural layout.  
**Date**: 2026-11-01  
**Status**: Complete  

---

## 1. Observation

### 1.1 Complete Filesystem Inventory of `d:\harfile\music`
Direct command execution:
```powershell
Get-ChildItem -Path "d:\harfile\music" -Recurse -Force | Select-Object FullName, Length, LastWriteTime
```
Observed output:
- `d:\harfile\music\Cargo.toml` (942 bytes)
- `d:\harfile\music\.gitignore` (26 bytes)
- `d:\harfile\music\crates\music_core\Cargo.toml` (443 bytes)
- `d:\harfile\music\crates\music_core\src\schema.rs` (23,558 bytes)
- `.agents\teamwork\` directory containing agent metadata (`BRIEFING.md`, `ORIGINAL_REQUEST.md`, agent subdirectories).
- **No other files or folders exist on disk.** Specifically:
  - `crates/music_fusion/` DOES NOT EXIST.
  - `crates/music_dsp/` DOES NOT EXIST.
  - `crates/music_pipeline/` DOES NOT EXIST.
  - `crates/music_cli/` DOES NOT EXIST.
  - `crates/music_core/src/lib.rs` DOES NOT EXIST.
  - No Web UI assets, HTML, CSS, or JS files exist.
  - No tests directory or unit tests exist.
  - No documentation (`README.md`, `PROJECT.md`) exists.

### 1.2 Root Workspace Manifest (`Cargo.toml`)
Inspecting `d:\harfile\music\Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "crates/music_core",
    "crates/music_fusion",
    "crates/music_dsp",
    "crates/music_pipeline",
    "crates/music_cli",
]

[workspace.dependencies]
tokio         = { version = "1", features = ["full"] }
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
anyhow        = "1"
thiserror     = "1"
reqwest       = { version = "0.12", features = ["json", "blocking"] }
rusqlite      = { version = "0.31", features = ["bundled"] }
chrono        = { version = "0.4", features = ["serde"] }
regex         = "1"
log           = "0.4"
env_logger    = "0.11"
dotenv        = "0.15"
clap          = { version = "4", features = ["derive"] }
colored       = "2"
walkdir       = "2"
sha2          = "0.10"
hex           = "0.4"
indicatif     = "0.17"
async-trait   = "0.1"
futures       = "0.3"
sysinfo       = "0.30"
uuid          = { version = "1.10", features = ["v4", "serde"] }
```

### 1.3 Compilation Status (`cargo check`)
Running `cargo check` in `d:\harfile\music` produced:
```
error: failed to load manifest for workspace member `D:\harfile\music\crates\music_core`
referenced by workspace at `D:\harfile\music\Cargo.toml`

Caused by:
  failed to parse manifest at `D:\harfile\music\crates\music_core\Cargo.toml`

Caused by:
  no targets specified in the manifest
  either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present
```
Observation: The workspace cannot build or check because `crates/music_core` lacks `src/lib.rs` and the other 4 member crates are absent from the filesystem.

### 1.4 Detailed Inspection of Existing Code in `crates/music_core/src/schema.rs`
The single existing source code file is `crates/music_core/src/schema.rs` (467 lines). It contains:
- Lines 4–48: `PipelineModelStack` struct and its `Default` implementation:
  - Stage 0: `stage0_ollama_panel` (`flint:latest`, `glm4:9b`, `qwen2.5:7b`, `mistral:latest`), `stage0_judge_model`, `stage0_writer_model`
  - Stage 1: `stage1_groove_foundation` (`ACE-Step/ACE-Step-v1.5`)
  - Stage 2: `stage2_yue_semantic` (`m-a-p/YuE-s1-7B-anneal-en-icl`), `stage2_yue_acoustic` (`m-a-p/YuE-s2-1B-general`)
  - Stage 3: `stage3_voice_conversion` (`Plachtaa/Seed-VC`)
  - Stage 4: `stage4_stem_separator` (`facebook/htdemucs_ft`), `stage4_solo_regenerator` (`stabilityai/stable-audio-open-1.0`), `stage4_style_transfer` (`facebook/musicgen-style`)
  - Stage 5: `stage5_super_resolution` (`haoheliu/versatile_audio_super_resolution`), `stage5_reference_mastering` (`Matchering-2.0`)
- Lines 52–90: `StemMixConfig` struct and `Default` (mix levels: lead vocal, gospel choir, drums, dub bass, guitar, organ brass; DSP parameters: analog tape warmth, tape echo wet, sub-bass boost dB, vocal rasp intensity, stereo width, target LUFS).
- Lines 94–122: `SongSection` struct (section ID, yue clean tag, display label, production cue, lyrics, bars, chords, guitar solo intensity, wah pedal depth, drum fill complexity, nyabinghi polyrhythm, gospel choir level, brass swell level, dub sub bass weight).
- Lines 126–215: `SongBlueprint` struct and methods:
  - `total_bars(&self) -> u32`
  - `estimated_duration_secs(&self) -> f64`
  - `to_yue_genre_txt(&self) -> String`
  - `to_yue_lyrics_txt(&self) -> String`
  - `to_suno_udio_custom_txt(&self) -> String`
- Lines 221–417: `SongBlueprint::default_one_love_shining_anthem()` — preset 1: "One Love Shining (One Heart, One Destiny)" at 76 BPM in A Major.
- Lines 421–465: `SongBlueprint::default_rise_up_together_anthem()` — preset 2: "Rise Up Together (Break Every Chain)".
- **Missing from schema.rs**:
  - `MusicProductionState` (specified in `ORIGINAL_REQUEST.md` Section 2) is completely absent.
  - `StemTelemetry` (specified in `ORIGINAL_REQUEST.md` Section 2) is completely absent.

### 1.5 Inspection of DSP, Audio Processing, and Mastering Code
- Zero DSP processing routines exist.
- Zero audio buffer structures (`Vec<f32>`, sample rates, channels).
- Zero WAV encoding/decoding (no RIFF header reader/writer).
- Zero synthesis algorithms (no oscillator, drum synth, Karplus-Strong or physical string model, wah filter, formant filter, saturation, EQ, or limiter).
- Zero audio stem splitters or crossfade splicers.
- Zero audio file telemetry analyzers (no peak dBFS, RMS, dynamic range, or clipping detectors).

### 1.6 Inspection of REST API, MCP Server, and CLI Code
- **REST API (Port 5050)**: Zero code. No listener, no router, no handlers for `POST /api/songs/create`, `POST /api/songs/iterate`, `POST /api/songs/regenerate-stem`, `POST /api/songs/master`, `GET /api/songs/:id/versions`.
- **MCP Server (`--mcp`)**: Zero code. No stdio JSON-RPC 2.0 loop, no MCP tool definitions (`music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`).
- **CLI (`music_cli`)**: Zero code. No binary entry point, no `clap` command parser, no `--create-reggae-anthem` or `--create-hit` CLI flags.

### 1.7 Live Hardware & Local Ollama Environment Probe
1. **GPU & VRAM (`nvidia-smi`)**:
   - Device: `Quadro RTX 4000` (Driver: 516.40, CUDA 11.7)
   - Total VRAM: `8192 MiB` (8.0 GB)
   - Used VRAM: `1151 MiB`
   - Free VRAM: `~7041 MiB` (~6.87 GB)
   - *Impact*: Heavy Hugging Face checkpoints (e.g. YuE 7B FP16 requires ~14+ GB VRAM) CANNOT run in 6.87 GB VRAM. The pure-Rust 48 kHz DSP fallback is an absolute hard necessity for this environment.
2. **Local Ollama Server (`http://127.0.0.1:11434/api/tags`)**:
   - The Ollama server is live and responding.
   - Installed models include:
     - `qwen2.5:32b` (32.8B, Q4_K_M)
     - `glm-4.7-flash:latest` (29.9B, Q4_K_M)
     - `glm4:9b` (9.4B, Q4_0)
     - `flint:latest` (7.6B, Q4_K_M)
     - `qwen2.5:7b` (7.6B, Q4_K_M)
     - `mistral:latest` (7.2B, Q4_K_M)
     - `deepseek-r1:32b` (32.8B, Q4_K_M)
     - `deepseek-r1:1.5b` (1.8B, Q4_K_M)
     - `gemma2:9b` (9.2B, Q4_0)
     - `gemma2:2b` (2.6B, Q4_0)
     - `qwen2.5:1.5b` (1.5B, Q4_K_M)
   - All models referenced in `ORIGINAL_REQUEST.md` (`qwen2.5:32b`, `glm4:9b`, `flint:latest`, `qwen2.5:7b`) are present locally.

---

## 2. Logic Chain

1. **Premise 1**: `Cargo.toml` at `d:\harfile\music` lists 5 workspace member crates (`music_core`, `music_fusion`, `music_dsp`, `music_pipeline`, `music_cli`).
2. **Premise 2**: On disk, only `crates/music_core` exists, but it lacks `src/lib.rs` (it only has `src/schema.rs`). Therefore `cargo check` fails immediately with an unparseable manifest error.
3. **Premise 3**: `ORIGINAL_REQUEST.md` mandates that the entire AI Music Producer Agent, native Rust `StateGraph<MusicProductionState>`, ModelFusion-adapted modules, 5-stage sequential fusion pipeline, REST API (port 5050), MCP server, and CLI live 100% inside `d:\harfile\music` as a standalone Rust workspace without linking to `d:\harfile\ModelFusion` at runtime.
4. **Premise 4**: Live system observation reveals a Quadro RTX 4000 GPU with 8.0 GB total VRAM and ~6.87 GB free VRAM. Since large Hugging Face checkpoints (YuE 7B, MusicGen, Seed-VC) exceed this VRAM budget unless offloaded or heavily quantized, the pure-Rust 48 kHz DSP synthesis engine must be fully operational, real, and capable of generating all stems (groove, harmonic stems, vocal formant rasp, virtuoso solo, mastering) end-to-end.
5. **Premise 5**: The user's follow-up update specifies a Universal Multi-Genre AI Music Production Engine supporting ALL genres of music known to humanity (Afrobeats, Highlife, Amapiano, Roots Reggae, Soul/R&B, Gospel, Hip-Hop/Trap, Blues, Rock, Metal, Jazz, Classical/Cinematic, Pop, EDM, Latin, etc.) across 40–220 BPM, diverse modes/scales, time signatures, and instrumentations, while preserving `--create-reggae-anthem` as a flagship preset.
6. **Inference**: The current workspace is an initial skeleton (`music_core/src/schema.rs`). To fulfill all requirements, we must create and implement the missing crate files and modules across all 5 workspace crates according to a strict, modular layout.

---

## 3. Caveats

1. **Read-Only Explorer Scope**: This investigation did not create or edit any source files in `d:\harfile\music`. All observations were made using read-only inspections, `Get-ChildItem`, and `cargo check`.
2. **External Model Weights**: External Hugging Face CLI checkpoints (e.g. `ACE-Step`, `YuE`, `Seed-VC`, `Matchering`) may or may not be installed in the local Python environment. The system must implement deterministic fallback to pure-Rust 48 kHz DSP synthesis when external tools are unavailable or VRAM is insufficient.
3. **Port 5050 Availability**: During testing, ensure port 5050 is not bound by another daemon or process.

---

## 4. Conclusion & Gap Analysis

### 4.1 Detailed Gap Matrix

| Requirement from ORIGINAL_REQUEST.md | Current State in `d:\harfile\music` | Exact Gap |
|---|---|---|
| **1. Workspace Compilability** | `Cargo.toml` fails in `cargo check` | Missing `crates/music_core/src/lib.rs` and missing crates `music_fusion`, `music_dsp`, `music_pipeline`, `music_cli`. |
| **2. State & Telemetry Models** | Only `SongBlueprint` and `SongSection` exist in `schema.rs` | Missing `MusicProductionState` and `StemTelemetry` structs with all required fields. |
| **3. Universal Genre Taxonomy** | Hardcoded reggae presets in `schema.rs` | Missing universal genre taxonomy module supporting all world genres, tempos 40–220 BPM, scales, and time signatures. |
| **4. Hardware & VRAM Sizing (`memory.rs`)** | Non-existent | Missing port of `SystemMemory::detect_live()` from `ModelFusion` (`crates/model_selection/src/memory.rs`). |
| **5. Multi-Model Ollama Fusion (`fusion_panel.rs`)** | Non-existent | Missing parallel Ollama panel (Lyricist, Arranger, Sound Engineer, Judge) via `reqwest` calling `/api/tags` and `/api/chat`. |
| **6. Native Rust LangGraph (`graph/mod.rs`)** | Non-existent | Missing async `tokio` Pregel/DAG `StateGraph<MusicProductionState>` engine with `add_node`, `add_edge`, `add_conditional_edges`, checkpointing. |
| **7. Sequential 5-Stage Fusion Nodes** | Non-existent | Missing Node 1 (`ollama_producer_panel_node`), Node 2 (`groove_and_yue_synthesis_node`), Node 3 (`vocal_timbre_fusion_node`), Node 4 (`virtuoso_stem_inpainting_node`), Node 5 (`dsp_and_judge_gate_node`), Node 6 (`analog_mastering_node`). |
| **8. Pure-Rust 48 kHz DSP Engine** | Non-existent | Missing WAV encoder/decoder, universal groove/drum synth, multi-stem harmonic synth, guitar solo/wah modeler, vocal formant filter, analog tape saturation, and reference EQ matchering. |
| **9. Tokio REST API (Port 5050)** | Non-existent | Missing HTTP server, routing, endpoints (`/api/songs/create`, `/api/songs/iterate`, `/api/songs/regenerate-stem`, `/api/songs/master`, `/api/songs/:id/versions`), and Web Studio UI streaming. |
| **10. MCP Server (`--mcp`)** | Non-existent | Missing stdio JSON-RPC 2.0 loop with 4 tools: `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`. |
| **11. CLI & Preset Runner** | Non-existent | Missing `clap` CLI with `--create-reggae-anthem`, `--create-hit`, `--genre`, `--bpm`, `--prompt`, and end-to-end execution. |
| **12. Test Suites** | Non-existent | Missing unit and integration tests across DSP, StateGraph, memory, Ollama panel, REST, MCP, and CLI. |

---

## 5. Architectural & File Layout Recommendations

To realize the complete requirements cleanly and maintainable within the 5 declared crates, the following architecture and layout is recommended:

### 5.1 Workspace File Layout Recommendation

```
d:\harfile\music\
├── Cargo.toml                              # Root workspace manifest (5 members)
├── .gitignore                              # /target/, *.wav, *.pdb, .env
├── crates/
│   ├── music_core/                         # Shared schemas, state, and taxonomy
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Exports schema, state, telemetry, taxonomy, store
│   │       ├── schema.rs                   # Existing SongBlueprint, SongSection, StemMixConfig
│   │       ├── state.rs                    # MusicProductionState (serde, clone, versioning)
│   │       ├── telemetry.rs                # StemTelemetry (peak dBFS, RMS, dynamic range, clipping)
│   │       ├── taxonomy.rs                 # Universal Genre & Style Taxonomy (40+ genres, scales, tempos)
│   │       └── store.rs                    # SQLite version history storage via rusqlite
│   │
│   ├── music_fusion/                       # ModelFusion patterns: memory probing & multi-LLM panel
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Exports memory, ollama, fusion_panel, judge
│   │       ├── memory.rs                   # SystemMemory::detect_live(), RAM/VRAM probe, HF vs DSP gating
│   │       ├── ollama.rs                   # Ollama HTTP client (/api/tags, /api/chat) with timeout
│   │       ├── fusion_panel.rs             # Multi-LLM Ollama Songwriter & Arranger panel
│   │       └── judge.rs                    # Multi-LLM Judge model for lyric & arrangement evaluation
│   │
│   ├── music_dsp/                          # Pure-Rust 48 kHz synthesis, stem processing & mastering
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Exports wav, groove_synth, harmonic_synth, vocal_timbre, stem_sep, mastering, telemetry_calc
│   │       ├── wav.rs                      # Pure-Rust 48 kHz 24-bit / 32-bit float stereo WAV reader/writer
│   │       ├── groove_synth.rs             # Universal drum & groove generator (One-Drop, Trap, Afrobeat, Rock, etc.)
│   │       ├── harmonic_synth.rs           # Multi-stem harmonic synth (bass, rhythm keys/guitar, horn swells)
│   │       ├── lead_synth.rs               # Virtuoso lead guitar solo physical model with wah envelope & bends
│   │       ├── vocal_timbre.rs             # Vocal formant filter, pitch-lock, and raspy tenor saturation
│   │       ├── stem_sep.rs                 # 4-stem separator (htdemucs fallback) & crossfade splicer
│   │       ├── mastering.rs                # 48 kHz analog tape saturation, sub-bass warmth, reference EQ matchering, peak limiter
│   │       └── telemetry_calc.rs           # Spectral/waveform telemetry (peak dBFS, RMS, dynamic range, phase drift)
│   │
│   ├── music_pipeline/                     # Native Rust StateGraph & 5-stage sequential pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Exports graph, nodes, runner
│   │       ├── graph/
│   │       │   ├── mod.rs                  # StateGraph<MusicProductionState> async tokio Pregel/DAG engine
│   │       │   ├── node.rs                 # Node trait and execution wrapper
│   │       │   └── edge.rs                 # Direct and conditional edges
│   │       ├── hf_runner.rs                # Subprocess runner for HF checkpoints with deterministic VRAM cleanup
│   │       └── nodes/
│   │           ├── mod.rs                  # Re-exports all pipeline nodes
│   │           ├── node1_ollama_panel.rs   # ollama_producer_panel_node
│   │           ├── node2_groove_yue.rs     # groove_and_yue_synthesis_node
│   │           ├── node3_vocal_timbre.rs   # vocal_timbre_fusion_node (universal / jamaican tenor)
│   │           ├── node4_stem_inpainting.rs# virtuoso_stem_inpainting_node
│   │           ├── node5_dsp_judge.rs      # dsp_and_judge_gate_node (with conditional retry edge)
│   │           └── node6_analog_master.rs  # analog_mastering_node
│   │
│   └── music_cli/                          # CLI binary, Tokio REST API (port 5050), and MCP server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                     # Binary entry point
│           ├── cli.rs                      # Clap argument definition (--create-reggae-anthem, --create-hit, etc.)
│           ├── server.rs                   # Tokio HTTP JSON REST API on port 5050
│           ├── mcp.rs                      # MCP JSON-RPC 2.0 stdio server (4 tools)
│           └── webui.rs                    # Embedded static HTML5/CSS3/JS Web Studio UI assets
```

### 5.2 Key Interface & Data Flow Details

1. **State Definition (`crates/music_core/src/state.rs`)**:
   ```rust
   use std::collections::HashMap;
   use std::path::PathBuf;
   use serde::{Deserialize, Serialize};
   use crate::telemetry::StemTelemetry;

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct MusicProductionState {
       pub song_id: String,
       pub version: u32,
       pub genre: String,
       pub engine_mode: String, // "auto", "pure_rust", "hf_hybrid"
       pub style_prompt: String,
       pub lyrics: String,
       pub bpm: u32,
       pub key_signature: String,
       pub vocal_ref_wav: Option<PathBuf>,
       pub master_ref_wav: Option<PathBuf>,
       pub stem_paths: HashMap<String, PathBuf>,
       pub telemetry: StemTelemetry,
       pub judge_score: f32,
       pub quality_passed: bool,
       pub retry_count: u8,
   }
   ```

2. **Hardware Sizing & VRAM Guard (`crates/music_fusion/src/memory.rs`)**:
   - Live query using `nvidia-smi` on Windows via `create_hidden_command` (`CREATE_NO_WINDOW 0x08000000`).
   - `SystemMemory::detect_live()` computes `free_ram_gb`, `gpu_vram_free_gb`.
   - If `free_vram_gb < 12.0` (or `engine_mode == "pure_rust"`), pipeline nodes automatically route to the built-in 48 kHz pure-Rust DSP engine instead of attempting to load massive FP16 Hugging Face weights into the 8GB Quadro RTX 4000.

3. **Sequential Pipeline Execution & VRAM Deallocation (`crates/music_pipeline`)**:
   - Each HF stage runs via isolated `tokio::process::Command` child process.
   - When a child process terminates, Windows and the NVIDIA driver automatically reclaim 100% of allocated GPU VRAM before the next node executes.
   - In pure-Rust mode, all audio generation runs in process memory using zero-copy DSP slices, producing clean 48 kHz 24-bit stereo WAVs.

4. **REST API & Web Studio UI (`crates/music_cli/src/server.rs`)**:
   - Listens on `http://127.0.0.1:5050`.
   - Endpoints:
     - `POST /api/songs/create`: accepts JSON prompt/genre/BPM, initializes state, and triggers StateGraph.
     - `POST /api/songs/iterate`: runs refinement iteration with modified parameters.
     - `POST /api/songs/regenerate-stem`: targeted execution of Node 4 for a single stem.
     - `POST /api/songs/master`: runs Node 6 analog mastering on selected stems.
     - `GET /api/songs/:id/versions`: returns version history and audio telemetry from SQLite store.
     - `GET /api/hardware`: returns live RAM/VRAM detection from `SystemMemory::detect_live()`.
     - `GET /api/models`: returns available Ollama models from `http://127.0.0.1:11434/api/tags`.
     - `GET /`: serves the embedded Web Studio UI dashboard.

5. **MCP Server (`crates/music_cli/src/mcp.rs`)**:
   - Implements JSON-RPC 2.0 stdio server.
   - Tools:
     - `music_create_hit`: params: `genre`, `prompt`, `lyrics`, `bpm`, `key` -> returns state & audio paths.
     - `music_iterate_song`: params: `song_id`, `version`, `instructions` -> returns new version.
     - `music_regenerate_stem`: params: `song_id`, `stem_name`, `style_prompt` -> returns regenerated stem path.
     - `music_master_track`: params: `song_id`, `master_profile` -> returns mastered 48 kHz WAV.

---

## 6. Verification Method

To independently verify the observations and findings in this report:

1. **Verify Filesystem Contents**:
   ```powershell
   Get-ChildItem -Path "d:\harfile\music" -Recurse -Force | Select-Object FullName
   ```
   *Expected result*: Only `Cargo.toml`, `.gitignore`, `crates/music_core/Cargo.toml`, `crates/music_core/src/schema.rs`, and `.agents/` exist.

2. **Verify Cargo Check Error**:
   ```powershell
   cd d:\harfile\music
   cargo check
   ```
   *Expected result*: Fails with `failed to parse manifest at ... crates\music_core\Cargo.toml: no targets specified in the manifest`.

3. **Verify Existing Schema Content**:
   Inspect `d:\harfile\music\crates\music_core\src\schema.rs`:
   - Contains `SongBlueprint`, `PipelineModelStack`, `StemMixConfig`, `SongSection`.
   - Lacks `MusicProductionState` and `StemTelemetry`.

4. **Verify GPU Hardware Constraints**:
   ```powershell
   nvidia-smi --query-gpu=name,memory.total,memory.free --format=csv
   ```
   *Expected result*: Quadro RTX 4000, 8192 MiB total, ~7000 MiB free.

5. **Verify Local Ollama Models**:
   ```powershell
   curl.exe -s --max-time 2 http://127.0.0.1:11434/api/tags
   ```
   *Expected result*: JSON containing `qwen2.5:32b`, `glm4:9b`, `flint:latest`, `qwen2.5:7b`, etc.

6. **Invalidation Conditions**:
   - If any of the crates `music_fusion`, `music_dsp`, `music_pipeline`, or `music_cli` were secretly present in an unmapped path.
   - If `crates/music_core` actually compiled cleanly with existing files.
   Both conditions were tested and disproven in Section 1.
