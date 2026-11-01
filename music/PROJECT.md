# Project: Standalone Universal Multi-Genre AI Music Producer Agent

## Architecture
Standalone 100% Rust workspace with 5 crates, native async `tokio` Pregel/DAG `StateGraph<MusicProductionState>`, adapted ModelFusion hardware/VRAM sizing and multi-LLM Ollama fusion panel, pure-Rust 48 kHz DSP synthesis engine, 6 sequential fusion nodes, Tokio HTTP REST (port 5050), MCP JSON-RPC 2.0 stdio, and CLI interfaces.

```
d:\harfile\music\
├── Cargo.toml (Workspace root: 5 member crates)
├── crates\
│   ├── music_core\       # State, StemTelemetry, Universal Genre Taxonomy, Schema, Blueprint
│   ├── music_fusion\     # Hardware/VRAM Sizing (SystemMemory), Multi-LLM Ollama Panel & Judge
│   ├── music_dsp\        # 48 kHz WAV I/O, Drum Synth, Formant/Rasp Filter, Guitar/Wah, Tape Saturation, Telemetry
│   ├── music_pipeline\   # Native StateGraph<MusicProductionState>, Checkpointer, 6 Sequential Nodes
│   └── music_cli\        # CLI, Tokio REST API (port 5050), MCP JSON-RPC 2.0 Stdio Server
```

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Standalone 100% Rust Architecture | All pipeline code compiled in Rust workspace; zero Python LangGraph; zero runtime link to ModelFusion | M1 | ORIGINAL_REQUEST §1 |
| 2 | Live Hardware & VRAM Sizing | `SystemMemory::detect_live()` probes RAM & GPU VRAM; Dynamic Available Memory Law selects Ollama models & decides HF vs. DSP | M1 | ORIGINAL_REQUEST §1.1 |
| 3 | Multi-Model Ollama Fusion Panel | Parallel Ollama panel (Lyricist, Arranger, Sound Engineer) + Judge at `/api/tags` and `/api/chat` synthesizing tags, lyrics, arrangement | M1 | ORIGINAL_REQUEST §1.2 |
| 4 | Universal Genre Taxonomy | Supports all genres known to humanity (Afrobeats, Highlife, Amapiano, Reggae, R&B, Gospel, Hip-Hop, Blues, Rock, Metal, Jazz, Classical, Pop, EDM, Latin, etc.) | M1 | ORIGINAL_REQUEST §Follow-up |
| 5 | Native Rust `StateGraph` Engine | Async Pregel/DAG engine with `add_node`, `add_edge`, `add_conditional_edges`, version snapshotting | M3 | ORIGINAL_REQUEST §2 |
| 6 | `MusicProductionState` & Telemetry | 13+ field state container (`song_id`, `version`, `engine_mode`, `bpm`, `key_signature`, `stem_paths`, `telemetry`, `judge_score`, `quality_passed`, `retry_count`) | M1 | ORIGINAL_REQUEST §2 |
| 7 | Pure-Rust 48 kHz WAV Engine | 24-bit/32-bit float WAV encoder/decoder and audio buffer structures | M2 | ORIGINAL_REQUEST §1.1 |
| 8 | Universal Drum & Rhythm Synthesizer | 48 kHz synthesis for One-Drop, Four-on-the-Floor, Boom-Bap/Trap, Afrobeats/Amapiano, Rock/Metal, Jazz swing, Orchestral | M2 | ORIGINAL_REQUEST §Follow-up |
| 9 | Pure-Rust Formant/Rasp Vocal Modeler | Vocal formant filter, rasp harmonic generator, and pitch-correction vocoder for Jamaican tenor and universal timbres | M2 | ORIGINAL_REQUEST §3 (Node 3) |
| 10 | Karplus-Strong Guitar & Wah Modeler | Physical string modeler with resonant wah-filter envelope and polyrhythmic drum synthesis for stem inpainting | M2 | ORIGINAL_REQUEST §3 (Node 4) |
| 11 | Sample-Accurate Stem Crossfader | Stem inpainting slicer and sample-accurate crossfade combiner locked to BPM grid | M2 | ORIGINAL_REQUEST §3 (Node 4) |
| 12 | Audio Spectral Telemetry Analyzer | Calculates peak dBFS, RMS dBFS, dynamic range, tempo drift, clipping boolean, SNR, spectral flatness | M2 | ORIGINAL_REQUEST §3 (Node 5) |
| 13 | 48 kHz Analog Tape & Sub-Bass Mastering | Tanh saturation, 55–85 Hz warmth boost, and reference spectral EQ matchering | M2 | ORIGINAL_REQUEST §3 (Node 6) |
| 14 | Node 1: `ollama_producer_panel_node` | Refines prompt, bracketed lyric tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`) | M3 | ORIGINAL_REQUEST §3 |
| 15 | Node 2: `groove_and_yue_synthesis_node` | Stage 1 groove generator (`draft_groove.wav`) + Stage 2 YuE generator (`yue_vocals.wav`, `yue_instrumentals.wav`) | M3 | ORIGINAL_REQUEST §3 |
| 16 | Node 3: `vocal_timbre_fusion_node` | Stage 3 Seed-VC / pure-Rust vocal formant/rasp processor (`raspy_tenor_vocals.wav`) | M3 | ORIGINAL_REQUEST §3 |
| 17 | Node 4: `virtuoso_stem_inpainting_node` | Stage 4 Demucs 4-stem split + Stable Audio / DSP inpainting for 16-bar interlude, solo, and nyabinghi fills | M3 | ORIGINAL_REQUEST §3 |
| 18 | Node 5: `dsp_and_judge_gate_node` | Telemetry inspection + Judge score evaluation; conditional retry edge (< 2 retries routes back to Node 4 or Node 2, else Node 6) | M3 | ORIGINAL_REQUEST §3 |
| 19 | Node 6: `analog_mastering_node` | Stage 5 AudioSR + Matchering or pure-Rust 48 kHz tape saturation / EQ (`final_master.wav`) | M3 | ORIGINAL_REQUEST §3 |
| 20 | Deterministic VRAM Process Isolation | Subprocess spawning via `tokio::process::Command` with process termination to guarantee 100% VRAM reclamation | M3 | ORIGINAL_REQUEST §3 |
| 21 | Tokio HTTP JSON REST API (Port 5050) | Endpoints `/api/songs/create`, `/api/songs/iterate`, `/api/songs/regenerate-stem`, `/api/songs/master`, `/api/songs/:id/versions`, SSE streaming | M4 | ORIGINAL_REQUEST §4 |
| 22 | MCP JSON-RPC 2.0 Stdio Server | MCP tools `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track` | M4 | ORIGINAL_REQUEST §4 |
| 23 | Flagship Preset `--create-reggae-anthem` | Pre-loaded 76 BPM "One Love" human compassion lyrics and 1977 roots reggae style prompt producing 48 kHz master | M4 | ORIGINAL_REQUEST §4 |
| 24 | Universal CLI Commands | `--create-hit --genre <genre> --prompt <prompt>`, `--serve`, `--mcp`, `--bpm`, `--key`, `--iterate`, `--assist-iteration` | M4 | ORIGINAL_REQUEST §Follow-up |
| 25 | E2E Testing Suite (Tiers 1-4) & Adversarial Verification (Tier 5) | Opaque-box testing across all features + white-box coverage hardening | M5 | Project Pattern & Dual Track |
| 26 | Multi-LLM Iteration Fusion Panel & Iteration Co-Pilot LLM | Dedicated Iteration Specialist / Co-Pilot LLM in ModelFusion panel deliberating alongside Lyricist, Arranger & Sound Engineer; version diff architecting; clickable suggestion chips in Web UI | M4 | ORIGINAL_REQUEST §Follow-up |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Core Domain, Taxonomy & Fusion Panel | `crates/music_core` (`schema.rs`, `state.rs`, `telemetry.rs`, `genre.rs`, `lib.rs`) + `crates/music_fusion` (`memory.rs`, `panel.rs`, `judge.rs`, `ollama_client.rs`, `lib.rs`) | None | **DONE** (commit `ea8fd21`, 54 workspace tests pass) |
| M2 | Pure-Rust 48 kHz DSP Engine & Telemetry | `crates/music_dsp` (WAV I/O, universal drum synth, vocal formant/rasp, Karplus-Strong guitar/wah, crossfader, telemetry calculator, tape mastering, `lib.rs`) | M1 | **DONE** (commits `23b44f1`, `03daa70`, 94 DSP tests, 150 workspace tests pass) |
| M3 | Native Rust StateGraph & 6 Fusion Nodes | `crates/music_pipeline` (StateGraph DAG engine, checkpointer, 6 sequential nodes, process isolation, `lib.rs`) | M1, M2 | **DONE** (commit `c73f797`, 60 pipeline tests, 193 workspace tests pass) |
| M4 | Tokio REST (5050), Web Studio UI, Multi-LLM Iteration Co-Pilot & CLI | `crates/music_cli` + `music_fusion` iteration co-pilot (REST server on 5050, ChatGPT-like 2-column Web Studio UI, auto-launch browser on startup, dedicated Iteration Co-Pilot LLM + suggestion chips, prompt & parameter song iteration, MCP server, `--create-reggae-anthem`, `--create-hit`, `main.rs`) | M1, M2, M3 | **DONE** (commits `76b52c8`, `2f6dc9d`, 23 music_cli tests, 237 workspace tests, dual routes & lineage streaming verified) |
| M5 | E2E Test Suite (Tiers 1-4) & Adversarial Verification (Tier 5) | Comprehensive test suite + adversarial hardening + test song (`v1` and iterated `v2`) on disk + final release build verification | M1, M2, M3, M4 | **DONE** (191 E2E tests, 252+ workspace tests pass, 44 WAV files on disk verified <= 0.966 peak, CLEAN audit) |
| M6 | TTS Engine, Lyrics Synchronization, Dynamic Ports & High-Pitch Noise Elimination | Dynamic SAPI parsing, ModelFusion TTS integration, sample song multi-section lyrics, high-pitch mosquito sine removal | M1..M5 | **DONE** (243 workspace tests pass, clean audit, lyrics synced across storage and WebUI) |
| M7 | Physical Modeling Synthesis, Stem Headroom & Tape Mastering, Uv Neural Bridge & Melodic Singing Upgrade | Karplus-Strong 0.994 damping string model, One-Drop drums & dub bass, 0.20 stem scaling, 12 kHz Butterworth filter, tanh saturation, hound PCM16 serialization, UvBridge with Python `workers/generate_song.py`, pure-Rust `MelodicSingingSynthesizer` (scale-locking, vibrato, 3-part harmony choir, Schroeder reverb), top 7 open HF model catalog, all 8 stored song versions regenerated | M1..M6 | **DONE** (56 stems > -50 dBFS RMS, 88 WAV files pass validation, 100% workspace & 191 E2E tests pass, CLEAN audit) |

## Interface Contracts

### `music_core` ↔ `music_fusion`
- `music_core::state::MusicProductionState`: Main state container passed into fusion panel.
- `music_core::genre::GenreTaxonomy`: Canonical genre definitions, BPM ranges, time signatures, scales.
- `music_fusion::memory::SystemMemory::detect_live() -> SystemMemory`: Hardware probing struct with `should_use_dsp(&self, stage: StageType) -> bool` and `pick_ollama_model(&self, installed: &[String]) -> String`.
- `music_fusion::panel::OllamaProducerPanel::run_panel(&self, state: &mut MusicProductionState) -> Result<OllamaPanelOutput, FusionError>`.

### `music_core` ↔ `music_dsp`
- `music_dsp::wav::AudioBuffer`: 48 kHz stereo/mono audio buffer with conversion to/from IEEE 32-bit float and 24-bit PCM WAV.
- `music_dsp::telemetry::calculate_telemetry(buffer: &AudioBuffer, bpm: u32) -> StemTelemetry`.
- `music_dsp::groove_synth::generate_groove(genre: &str, bpm: u32, duration_secs: f32) -> AudioBuffer`.
- `music_dsp::vocal_formant::apply_vocal_timbre(buffer: &AudioBuffer, rasp: f32, shift_semitones: f32) -> AudioBuffer`.
- `music_dsp::guitar_wah::synthesize_lead_solo(bars: u32, bpm: u32, scale: &str) -> AudioBuffer`.
- `music_dsp::tape_mastering::apply_analog_mastering(buffer: &AudioBuffer, profile: MasteringProfile) -> AudioBuffer`.

### `music_pipeline` ↔ `music_core` / `music_fusion` / `music_dsp`
- `music_pipeline::graph::StateGraph<MusicProductionState>`:
  - `add_node(name: &str, node: impl GraphNode<MusicProductionState>)`
  - `add_edge(from: &str, to: &str)`
  - `add_conditional_edges(from: &str, condition_fn, branches: HashMap<String, String>)`
  - `compile() -> CompiledGraph<MusicProductionState>`
  - `execute(initial_state: MusicProductionState) -> Result<MusicProductionState, PipelineError>`
- Checkpointing: saves each state transition to `./storage/songs/<song_id>/v<version>/state.json`.

### `music_cli` ↔ `music_pipeline`
- REST Server: `POST /api/songs/create`, `POST /api/songs/iterate`, `POST /api/songs/regenerate-stem`, `POST /api/songs/master`, `GET /api/songs/:id/versions`.
- MCP Server: JSON-RPC 2.0 stdio handling `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`.
- CLI Presets: `--create-reggae-anthem` (76 BPM "One Love" preset), `--create-hit` (universal genre hit generator).

## Code Layout
```
d:\harfile\music\
├── Cargo.toml
├── PROJECT.md
├── crates\
│   ├── music_core\
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── schema.rs
│   │       ├── state.rs
│   │       ├── telemetry.rs
│   │       ├── genre.rs
│   │       └── blueprint.rs
│   ├── music_fusion\
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── memory.rs
│   │       ├── ollama_client.rs
│   │       ├── panel.rs
│   │       ├── judge.rs
│   │       └── prompt_templates.rs
│   ├── music_dsp\
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── wav.rs
│   │       ├── groove_synth.rs
│   │       ├── vocal_formant.rs
│   │       ├── guitar_wah.rs
│   │       ├── crossfader.rs
│   │       ├── telemetry_calc.rs
│   │       └── tape_mastering.rs
│   ├── music_pipeline\
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── graph.rs
│   │       ├── checkpointer.rs
│   │       ├── runner.rs
│   │       └── nodes\
│   │           ├── mod.rs
│   │           ├── node1_producer_panel.rs
│   │           ├── node2_groove_yue.rs
│   │           ├── node3_vocal_timbre.rs
│   │           ├── node4_stem_inpainting.rs
│   │           ├── node5_dsp_judge_gate.rs
│   │           └── node6_analog_mastering.rs
│   └── music_cli\
│       ├── Cargo.toml
│       └── src\
│           ├── main.rs
│           ├── cli_args.rs
│           ├── rest_server.rs
│           ├── mcp_server.rs
│           └── presets.rs
```
