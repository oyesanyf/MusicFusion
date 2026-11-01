# MusicFusion: Autonomous Multi-LLM AI Music Studio & Production Pipeline

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux-lightgrey.svg)](https://github.com/oyesanyf/MusicFusion)

**MusicFusion** is an autonomous, production-grade music composition and production workstation built in Rust and modern Web technologies. It bridges state-of-the-art local Large Language Models (Ollama `qwen2.5:32b`, `deepseek-r1:32b`, `glm4:9b`, `flint:latest`), ModelFusion voice synthesis, and high-fidelity 48 kHz 24-bit/16-bit digital signal processing (DSP) to produce radio-ready anthems with soulful lead vocals, rich gospel backing choirs, virtuoso instrumental solos, and analog tape mastering.

---

## 🌟 Key Highlights

- **ChatGPT-Style Web Studio UI**: Responsive, zero-dependency vanilla ES6+ web interface with ChatGPT clean light theme, centered natural-language pill composer, live A/B version comparison, multi-track stem mixer, and interactive waveform visualizer.
- **ModelFusion Text-to-Voice (TTS) Vocal Engine**: Dedicated vocal synthesis engine supporting cutting-edge neural vocalists (F5-TTS, Kokoro-82M, Suno Bark, Coqui XTTS-v2, Fish-Speech, Parler-TTS, Seed-VC) and zero-crash Windows SAPI fallback (`Microsoft David Desktop` lead + `Microsoft Zira Desktop` choir + melodic singing pitch carrier).
- **Conversational Assistant Producer Experience**: Every iteration prompt spawns a dedicated Assistant Producer reply bubble with live stage-by-stage feedback badges (`🎤 VOCALS ADDED`, `🎸 ALL INSTRUMENTS MIXED`, `🛠️ SCRATCH REMOVED`), inline Test / Solo Vocals / Revert players, and direct WAV download links.
- **Scratch-Free Drum & Guitar DSP**: Unfiltered broadband white noise is replaced by shaped band-passed resonators (3.2 kHz snare wire sizzle, 7.2–9.4 kHz hi-hat clusters, 4.5 kHz shaker bandpass, 4.2/6.1/8.3 kHz crash cymbals) and warm phase-continuous Karplus-Strong string synthesis with smooth wah-wah envelopes.
- **Full Multi-Section English Lyrics**: Complete lyrics (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Verse 2]`, `[Guitar Solo]`, `[Bridge]`, `[Outro]`) displayed by default in an expanded song card drawer and synchronized live with playback and TTS inputs.
- **Hardware Sizing & Dynamic Available Memory Law**: Dynamic free memory evaluation (strictly Free RAM and Free GPU VRAM) prevents OOM errors while routing models across 5 memory tiers.
- **Dynamic Port Selection & RFC 7233 Audio Streaming**: Scans 5050..=5070 with ephemeral OS port fallback, serving 48 kHz stereo audio with byte-range scrubbing and multi-version lineage fallback.

---

## 🏗️ Architecture & Sequential Pipeline

MusicFusion operates as a directed execution graph with state verification, deterministic retries, and checkpointing:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        MusicFusion StateGraph                          │
└────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 1: Blueprint & Structural Lyric Architecture       │
       │ Multi-LLM Prompt Deliberation (Lyricist: flint, glm4)    │
       │ Generates [Intro]..[Outro] bracketed tags, BPM, and Key  │
       └──────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 2: Groove & Draft YuE Audio Synthesis              │
       │ 48 kHz stereo PCM drum pattern + guide vocal carrier     │
       │ Band-passed drum resonators eliminate harsh noise        │
       └──────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 3: Vocal Timbre, Resonant Formants & TTS Engine    │
       │ ModelFusion Vocal Synthesizer (F5-TTS, Bark, XTTS-v2)   │
       │ Windows SAPI + singing harmonic vocoder fallback         │
       └──────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 4: Stem Separation & Virtuoso Solo Inpainting      │
       │ 16-bar soaring blues-rock wah-wah electric guitar solo   │
       │ Nyabinghi burru percussion fills and organ bubble        │
       └──────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 5: Multi-LLM Deliberation Panel & Judge Gate       │
       │ Ollama qwen2.5:32b + deepseek-r1 evaluate quality score  │
       │ Automatic retry edge if score < 7.0 (max 3 retries)      │
       └──────────────────────────────────────────────────────────┘
                                    │
                                    ▼
       ┌──────────────────────────────────────────────────────────┐
       │ Stage 6: Kingston Studer Tape Mastering & Telemetry      │
       │ Vocal-priority sidechain ducking (-1.8 dB)               │
       │ Studer A800 tape saturation & brickwall limiter (-0.3dB) │
       └──────────────────────────────────────────────────────────┘
```

---

## 📦 Workspace Crates

| Crate | Purpose | Key Modules |
|-------|---------|-------------|
| `music_core` | Core domain types, state serialization, checkpointing | `state.rs`, `checkpoint.rs`, `telemetry.rs` |
| `music_dsp` | Digital signal processing, filters, drum & guitar synths | `groove_synth.rs`, `guitar_wah.rs`, `wav.rs`, `limiter.rs` |
| `music_fusion` | LLM orchestration, ModelFusion TTS, hardware sizing | `tts_engine.rs`, `iteration_panel.rs`, `memory.rs`, `judge.rs` |
| `music_pipeline`| 6-stage execution runner, StateGraph orchestration | `runner.rs`, `nodes/node1..node6` |
| `music_cli` | Tokio HTTP REST API, Web Studio UI, CLI entrypoint | `rest_server.rs`, `cli_args.rs`, `webui/` |

---

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (stable toolchain)
- (Optional) [Ollama](https://ollama.com/) running locally on `http://127.0.0.1:11434` with `qwen2.5:32b` or `deepseek-r1:32b`. Pure-Rust DSP fallback operates with zero external dependencies.

### Build and Test
```powershell
# Build entire workspace
cargo build --workspace

# Run all 243 unit, empirical, and adversarial stress tests
cargo test --workspace
```

### Launch the Web Studio UI
```powershell
# Launch REST server & Web Studio (auto-binds to available port in 5050..=5070)
cargo run -p music_cli -- --serve

# Specify explicit port and open browser
cargo run -p music_cli -- --serve --port 5050
```

### Command-Line Song Creation & Iteration
```powershell
# Create Flagship Roots Reggae Anthem (76 BPM, A Major, pure-Rust DSP)
cargo run -p music_cli -- --create-reggae-anthem --song-id roots_reggae_anthem --bpm 76 --key "A Major" --pure-rust

# Create Modern Afrobeats Hit (104 BPM, C Major)
cargo run -p music_cli -- --create-hit --genre afrobeats --song-id song_afrobeats_002 --bpm 104 --key "C Major" --prompt "Lagos Sunrise Groove, 104 BPM Afrobeats anthem, syncopated Tresillo log-drum bassline, bright highlife guitar plucks, warm brass stabs" --pure-rust

# Iterate on Existing Song (Creates v2 with targeted changes)
cargo run -p music_cli -- --iterate "Boost guitar solo wah & sustain in [Guitar Solo]" --song-id roots_reggae_anthem --version 1 --pure-rust
```

---

## 🎧 Web Studio REST API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/health` | Health probe & pipeline status |
| `GET` | `/api/hardware` | Free VRAM, RAM, and detected Ollama models |
| `GET` | `/api/audio-devices` | System sound output device enumeration |
| `GET` | `/api/songs` | Catalog of all songs and version lineages |
| `POST` | `/api/songs/create` | Create new song from natural language prompt |
| `POST` | `/api/songs/iterate` | Iterate existing song version with feedback |
| `POST` | `/api/songs/assist-iteration` | Consult Iteration Co-Pilot LLM for smart suggestions |
| `POST` | `/api/songs/tts-vocals` | Synthesize new ModelFusion vocal stem & remix master |
| `POST` | `/api/songs/regenerate-stem` | Regenerate a single isolated stem (e.g. guitar solo) |
| `POST` | `/api/songs/master` | Re-master track with different analog tape profile |
| `GET` | `/api/songs/:id/versions/:v/audio/:stem` | RFC 7233 byte-range streaming for WAV stems |
| `GET` | `/api/songs/:id/versions/:v/download` | Direct download of mastered 48 kHz WAV |
| `GET` | `/api/songs/:id/download` | Download latest version final master |

---

## 🔒 Verification & Compliance

MusicFusion enforces strict verification invariants across audio quality, security, and hardware resilience:
- **True Peak Amplitude Ceiling**: Master WAV files strictly enforce brickwall limiting to $\le -0.30\text{ dBFS}$ ($\le 0.96605$).
- **Zero Digital Clipping**: All stems and master files verify zero digital clipping ($> 1.0$) across full song durations.
- **Dynamic Port Selection**: Gracefully steps through ports 5050..=5070 without crashing if ports are occupied.
- **RFC 7233 Byte-Range Scrubbing**: Validates standard, open-ended, suffix, and boundary byte ranges for real-time waveform scrubbing.
- **Path Traversal & Injection Security**: Rejects null bytes, path traversal sequences (`..`), UNC paths, and drive specifiers in stem resolution.

---

## 📄 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
