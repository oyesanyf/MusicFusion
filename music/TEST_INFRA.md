# E2E Test Infrastructure: Standalone Universal Multi-Genre AI Music Producer Agent

## Document Metadata
- **Project**: Standalone Universal Multi-Genre AI Music Producer Agent Workspace
- **Workspace Root**: `d:\harfile\music`
- **Role**: E2E Test Suite Architect
- **Authoritative Sources**: `ORIGINAL_REQUEST.md` (Requirements 1–4 & Multi-Genre Follow-up), `PROJECT.md` (Architecture, Feature Inventory 1–25, Interface Contracts)
- **Specification Status**: Active / Definitive Design

---

## 1. Test Philosophy & Architectural Invariants

### 1.1 Opaque-Box & Requirement-Driven Testing
All tests in the E2E test suite are strictly opaque-box (black-box) and requirement-driven. Tests verify external observable behavior, public API contracts, system state transitions, 48 kHz PCM audio buffers, version snapshots, REST API JSON responses, MCP JSON-RPC 2.0 payloads, and CLI exit codes. Test assertions derive exclusively from user requirements in `ORIGINAL_REQUEST.md` and system invariants in `PROJECT.md`, without coupling to internal crate private functions, helper structs, or transient code details.

### 1.2 Universal Multi-Genre AI Music Engine Invariant
The engine is architected and tested as a **Universal Multi-Genre AI Music Production Engine** supporting ALL musical genres known to humanity:
- Reggae & Dub (76 BPM One-Drop, Nyabinghi polyrhythms, melodic bassline, offbeat guitar skank, tape space echo)
- Afrobeats & Amapiano (108–114 BPM syncopated log-drum, clave, shaker polyrhythms, melodic bass)
- Pop & Dance (120–128 BPM Four-on-the-Floor, smooth vocal formants, sidechain pump, modern punchy master)
- Jazz & Bebop / Fusion (110–160 BPM swung ride cymbal, acoustic upright bass, piano/brass improvisation)
- Rock & Heavy Metal (120–180 BPM driving double-kick backbeat, Karplus-Strong screaming guitar solos, wah filter)
- Hip-Hop, Boom-Bap & Trap (70–90 / 140–160 BPM rolling hi-hats, hard snare on 3, deep 808 sub-bass)
- Classical & Cinematic Orchestral (50–100 BPM compound meters `6/8`, `12/8`, timpani rolls, brass/choir swells)
- Latin, Salsa & Bossa Nova (syncopated montuno piano, tumbao congas, brass stabs)
- Ambient, Electronic & Synthwave (analog tape saturation, lush reverb spaces, warm sub-bass)
The flagship preset `--create-reggae-anthem` (the 1977 Roots Reggae "One Love" spiritual successor at 76 BPM) is preserved and tested as a zero-configuration reference track.

### 1.3 Production Code & Absolute Zero-Mock Invariant
Tests must never execute against hardcoded dummy personas, fake mockup audio, or simulated placeholder data. All tests exercise:
1. Real 48 kHz 24-bit / 32-bit float PCM audio buffers synthesized through genuine digital signal processing (DSP) oscillators, filters, and envelope generators.
2. Genuine Ollama HTTP requests against `http://127.0.0.1:11434` with graceful, deterministic fallback to canonical genre blueprints when the daemon is offline.
3. Authentic out-of-process execution via `tokio::process::Command` ensuring 100% GPU VRAM reclamation.
4. Genuine Axum/Tokio HTTP REST routes and MCP JSON-RPC 2.0 stdio protocols.

### 1.4 Deterministic VRAM Process Isolation & Pure-Rust DSP Fallback
Hugging Face model checkpoints (`YuE-s1-7B`, `ACE-Step`, `Seed-VC`, `HTDemucs`, `AudioSR`) demand substantial GPU VRAM (4–16 GB).
- When running in external hybrid mode, each Hugging Face stage executes in a child process; process termination guarantees that Windows WDDM / CUDA drivers immediately reclaim 100% of GPU VRAM between stages.
- When live GPU VRAM is below 4.0 GB or when `engine_mode == "pure_rust"` or when external weights are absent, all stages transparently route to the built-in 48 kHz pure-Rust DSP engine.
- Headroom and clipping invariants: all audio stems and masters must assert peak levels $\le 0.0 \text{ dBFS}$ with zero digital clipping ($|sample| < 1.0$), with final masters targeted at $-0.5 \text{ to } -0.3 \text{ dBFS}$ true peak and $-14.0 \text{ LUFS}$ integrated loudness.

### 1.5 Zero-Touch Environment & System Safety
- Under no circumstances may any test or runner mutate, overwrite, truncate, or wipe system or user environment variables (`HKCU\Environment`, `HKLM\SYSTEM\...\Environment`, `setx`).
- All environment parameters (e.g. `OLLAMA_HOST`, `MUSIC_PORT`) must be strictly process-scoped.

---

## 2. Feature Inventory & 4-Tier Master Mapping Matrix

Every feature from `PROJECT.md § Feature Inventory` (Features 1 through 25) is systematically mapped across all 4 testing tiers plus specialized acceptance suites.

| Feature # | Feature Name | Milestone | Requirement Source | Tier 1 (>=5) | Tier 2 (>=5) | Tier 3 (Pairwise) | Tier 4 (Scenario) | Test Suite Location |
|:---------:|:-------------|:---------:|:-------------------|:------------:|:------------:|:-----------------:|:-----------------:|:--------------------|
| **F01** | Standalone 100% Rust Architecture | M1 | ORIGINAL_REQUEST §1 | 5 | 5 | T3_PAIR_01 | S1, S2, S3, S4, S5, S6, S7, S8 | `tests/e2e_tests.rs` |
| **F02** | Live Hardware & VRAM Sizing | M1 | ORIGINAL_REQUEST §1.1 | 5 | 5 | T3_PAIR_01, T3_PAIR_02 | S1, S2, S3 | `tests/e2e_tests.rs` |
| **F03** | Multi-Model Ollama Fusion Panel | M1 | ORIGINAL_REQUEST §1.2 | 5 | 5 | T3_PAIR_02, T3_PAIR_03 | S1, S2, S4 | `tests/e2e_tests.rs` |
| **F04** | Universal Genre Taxonomy | M1 | ORIGINAL_REQUEST §Follow-up | 5 | 5 | T3_PAIR_03, T3_PAIR_04 | S1–S8 | `tests/e2e_tests.rs` |
| **F05** | Native Rust `StateGraph` Engine | M3 | ORIGINAL_REQUEST §2 | 5 | 5 | T3_PAIR_04, T3_PAIR_05 | S1, S2, S5 | `tests/e2e_tests.rs` |
| **F06** | `MusicProductionState` & Telemetry | M1 | ORIGINAL_REQUEST §2 | 5 | 5 | T3_PAIR_05, T3_PAIR_06 | S1–S8 | `tests/e2e_tests.rs` |
| **F07** | Pure-Rust 48 kHz WAV Engine | M2 | ORIGINAL_REQUEST §1.1 | 5 | 5 | T3_PAIR_06, T3_PAIR_07 | S1–S8 | `tests/e2e_tests.rs` |
| **F08** | Universal Drum & Rhythm Synthesizer | M2 | ORIGINAL_REQUEST §Follow-up | 5 | 5 | T3_PAIR_07, T3_PAIR_08 | S1–S8 | `tests/e2e_tests.rs` |
| **F09** | Pure-Rust Formant/Rasp Vocal Modeler | M2 | ORIGINAL_REQUEST §3 (Node 3) | 5 | 5 | T3_PAIR_08, T3_PAIR_09 | S1, S2, S3, S5 | `tests/e2e_tests.rs` |
| **F10** | Karplus-Strong Guitar & Wah Modeler | M2 | ORIGINAL_REQUEST §3 (Node 4) | 5 | 5 | T3_PAIR_09, T3_PAIR_10 | S1, S4, S5 | `tests/e2e_tests.rs` |
| **F11** | Sample-Accurate Stem Crossfader | M2 | ORIGINAL_REQUEST §3 (Node 4) | 5 | 5 | T3_PAIR_10, T3_PAIR_11 | S1, S4, S5 | `tests/e2e_tests.rs` |
| **F12** | Audio Spectral Telemetry Analyzer | M2 | ORIGINAL_REQUEST §3 (Node 5) | 5 | 5 | T3_PAIR_11, T3_PAIR_12 | S1–S8 | `tests/e2e_tests.rs` |
| **F13** | 48 kHz Analog Tape & Sub-Bass Master | M2 | ORIGINAL_REQUEST §3 (Node 6) | 5 | 5 | T3_PAIR_12, T3_PAIR_13 | S1, S2, S3, S7, S8 | `tests/e2e_tests.rs` |
| **F14** | Node 1: `ollama_producer_panel_node` | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_13, T3_PAIR_14 | S1, S2 | `tests/e2e_tests.rs` |
| **F15** | Node 2: `groove_and_yue_synthesis_node`| M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_14, T3_PAIR_15 | S1, S2, S7 | `tests/e2e_tests.rs` |
| **F16** | Node 3: `vocal_timbre_fusion_node` | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_15, T3_PAIR_16 | S1, S2, S3 | `tests/e2e_tests.rs` |
| **F17** | Node 4: `virtuoso_stem_inpainting_node` | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_16, T3_PAIR_17 | S1, S4, S5 | `tests/e2e_tests.rs` |
| **F18** | Node 5: `dsp_and_judge_gate_node` | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_17, T3_PAIR_18 | S1, S5 | `tests/e2e_tests.rs` |
| **F19** | Node 6: `analog_mastering_node` | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_18, T3_PAIR_19 | S1–S8 | `tests/e2e_tests.rs` |
| **F20** | Deterministic VRAM Process Isolation | M3 | ORIGINAL_REQUEST §3 | 5 | 5 | T3_PAIR_19, T3_PAIR_20 | S1, S2 | `tests/e2e_tests.rs` |
| **F21** | Tokio HTTP JSON REST API (Port 5050) | M4 | ORIGINAL_REQUEST §4 | 5 | 5 | T3_PAIR_20, T3_PAIR_21 | S1, S2, S3 | `tests/e2e_tests.rs` |
| **F22** | MCP JSON-RPC 2.0 Stdio Server | M4 | ORIGINAL_REQUEST §4 | 5 | 5 | T3_PAIR_21, T3_PAIR_22 | S1, S7 | `tests/e2e_tests.rs` |
| **F23** | Flagship Preset `--create-reggae-anthem` | M4 | ORIGINAL_REQUEST §4 | 5 | 5 | T3_PAIR_22, T3_PAIR_23 | S1 | `tests/e2e_tests.rs` |
| **F24** | Universal CLI Commands | M4 | ORIGINAL_REQUEST §Follow-up | 5 | 5 | T3_PAIR_23, T3_PAIR_24 | S2, S3, S4, S5, S7, S8 | `tests/e2e_tests.rs` |
| **F25** | E2E Testing Suite & Adversarial Audit | M5 | Project Pattern & Dual Track | 5 | 5 | T3_PAIR_24, T3_PAIR_25 | S1–S8 | `tests/e2e_tests.rs` |

### Quantitative Summary of Test Inventory:
- **Tier 1 (Feature Coverage)**: 25 features × 5 tests = **125 test cases**
- **Tier 2 (Boundary & Corner Cases)**: 25 features × 5 tests = **125 test cases**
- **Tier 3 (Cross-Feature Pairwise Combinations)**: **25 pairwise integration test cases**
- **Tier 4 (Real-World Multi-Genre Scenarios)**: **8 end-to-end full song production scenarios**
- **Specialized Acceptance Verification Gates**:
  - `dsp_clipping_and_headroom_gate`: **5 verification checks**
  - `universal_genre_taxonomy_gate`: **5 verification checks**
  - `rest_and_mcp_protocol_gate`: **5 verification checks**
- **Grand Total Volume**: **298 distinct test cases** specified and tracked.

---

## 3. Tier 1: Isolated Feature Coverage Catalog (125 Test Cases)

### Feature 1: Standalone 100% Rust Architecture
- `T1-F01-01`: Verify zero runtime link to `d:\harfile\ModelFusion` (no dynamic DLL/so or symbol linkage).
- `T1-F01-02`: Verify zero Python dependency required for baseline pipeline execution (`pure_rust` mode).
- `T1-F01-03`: Cargo workspace dependency graph audit asserting only pure Rust crates in `Cargo.lock`.
- `T1-F01-04`: Clean serialization roundtrip of all core domain structs across crate boundaries.
- `T1-F01-05`: Deterministic standalone build producing standalone binary targets in `target/release`.

### Feature 2: Live Hardware & VRAM Sizing
- `T1-F02-01`: `SystemMemory::detect_live()` returns positive available RAM and CPU physical cores $\ge 1$.
- `T1-F02-02`: Dynamic Available Memory Law selects model tier based strictly on available (not total) RAM.
- `T1-F02-03`: VRAM safety budget calculation enforces `gpu_budget_gb = free_vram_gb * 0.85`.
- `T1-F02-04`: RAM safety budget calculation enforces `ram_budget_gb = ((free_ram_gb - 3.0) * 0.70).max(0.0)`.
- `T1-F02-05`: Sizing engine automatically resolves `engine_mode = "pure_rust"` when free VRAM $< 4.0 \text{ GB}$.

### Feature 3: Multi-Model Ollama Fusion Panel
- `T1-F03-01`: HTTP client discovers installed models from `GET /api/tags` with 2-second timeout.
- `T1-F03-02`: Dynamic model fallback selection maps requested tags to best locally installed alternatives.
- `T1-F03-03`: Parallel panel query dispatch to Lyricist, Arranger, and Sound Engineer roles.
- `T1-F03-04`: Judge model strips reasoning thought blocks (`<think>...</think>`) and markdown fences.
- `T1-F03-05`: Robust fallback to canonical genre template when Ollama daemon connection is refused.

### Feature 4: Universal Genre Taxonomy
- `T1-F04-01`: Canonical taxonomy lookup validates 40+ world genres (Reggae, Afrobeats, Pop, Jazz, Rock, etc.).
- `T1-F04-02`: BPM range verification clamps target tempo within genre bounds (40 to 220 BPM).
- `T1-F04-03`: Musical scale & mode resolution (Major, Minor, Dorian, Mixolydian, Blues, Arabic, Hirajoshi).
- `T1-F04-04`: Time signature mapping (`4/4`, `3/4`, `6/8`, `12/8`, `5/4`, `7/8`) with correct beats per bar.
- `T1-F04-05`: Production cue generation matches target genre instrumentation and acoustic space.

### Feature 5: Native Rust `StateGraph` Engine
- `T1-F05-01`: `StateGraph::new()` and `add_node()` registers nodes with unique stage identifiers.
- `T1-F05-02`: `add_edge()` links sequential stages in deterministic execution order.
- `T1-F05-03`: `add_conditional_edges()` routes dynamic transitions based on evaluation predicates.
- `T1-F05-04`: Graph execution loop executes compiled pipeline steps and tracks stage completion.
- `T1-F05-05`: Version snapshot checkpointer persists state transitions to disk and SQLite store.

### Feature 6: `MusicProductionState` & Telemetry
- `T1-F06-01`: Serialization and deserialization of `MusicProductionState` preserves all 16 fields.
- `T1-F06-02`: Version monotonic counter incrementation on iterative state progression ($V \to V + 1$).
- `T1-F06-03`: `StemTelemetry` default construction provides broadcast-compliant baseline parameters.
- `T1-F06-04`: Stem path mapping (`stem_paths`) correctly tracks all 11 isolated audio channels.
- `T1-F06-05`: Quality pass flag and retry count threshold evaluation ($retry\_count \le 2$).

### Feature 7: Pure-Rust 48 kHz WAV Engine
- `T1-F07-01`: Encode 48 kHz 32-bit IEEE float stereo WAV file with standard RIFF headers.
- `T1-F07-02`: Decode 48 kHz 32-bit float and 24-bit PCM WAV into internal `AudioBuffer` structure.
- `T1-F07-03`: Accurate duration and sample count calculation ($N = \text{sample\_rate} \times \text{duration}$).
- `T1-F07-04`: Stereo channel interleaving and de-interleaving preservation of phase relationships.
- `T1-F07-05`: Clean error reporting on corrupted or truncated WAV headers without panicking.

### Feature 8: Universal Drum & Rhythm Synthesizer
- `T1-F08-01`: Reggae 76 BPM One-Drop synthesis: kick and snare locked to beat 3 with rimshot accent.
- `T1-F08-02`: Afrobeats & Amapiano log-drum synthesis: syncopated sub-bass transient with wood clave.
- `T1-F08-03`: Four-on-the-Floor EDM & Pop kick synthesis: punchy 60 Hz fundamental with offbeat open hi-hat.
- `T1-F08-04`: Rock & Metal backbeat synthesis: acoustic kick on 1 & 3, snare on 2 & 4, continuous 8th ride.
- `T1-F08-05`: Jazz swing pattern synthesis: swung ride cymbal triplet feel with ghost snare taps.

### Feature 9: Pure-Rust Formant/Rasp Vocal Modeler
- `T1-F09-01`: Formant filter shaping vocal tract resonances for male tenor frequencies ($F_1 \approx 500\text{Hz}, F_2 \approx 1500\text{Hz}$).
- `T1-F09-02`: Quadratic harmonic rasp generator introducing authentic 1970s vocal cord grit.
- `T1-F09-03`: Pitch-lock vocoder centering guide pitch onto song key root without digital distortion.
- `T1-F09-04`: Vocal archetype switching (Raspy Tenor, Smooth R&B, Gritty Rock, Operatic Choral).
- `T1-F09-05`: Peak normalization ensuring processed vocal stem maintains $+1.5\text{ dB}$ mix headroom.

### Feature 10: Karplus-Strong Guitar & Wah Modeler
- `T1-F10-01`: Karplus-Strong physical string delay loop generating authentic plucked string decay.
- `T1-F10-02`: Resonant bandpass wah-filter envelope sweeping center frequency ($400\text{Hz}$ to $2200\text{Hz}$).
- `T1-F10-03`: Non-linear soft-clipping overdrive simulating vintage vacuum tube guitar amplifier.
- `T1-F10-04`: Scale-aware solo note generator selecting intervals strictly from song key scale.
- `T1-F10-05`: Expressive string bend and vibrato modulation ($5\text{Hz}$ LFO on delay line length).

### Feature 11: Sample-Accurate Stem Crossfader
- `T1-F11-01`: Equal-power crossfade calculation: $y[n] = \sqrt{1-t} \cdot x_1[n] + \sqrt{t} \cdot x_2[n]$.
- `T1-F11-02`: Constant total power maintenance ($\le 0.02\text{ dB}$ deviation across transition window).
- `T1-F11-03`: Grid-locked bar alignment: computes transition boundaries from exact BPM period.
- `T1-F11-04`: Seamless stem inpainting splice replacing designated bar ranges without clicks or pops.
- `T1-F11-05`: Multi-track synchronization asserting identical buffer lengths across all spliced stems.

### Feature 12: Audio Spectral Telemetry Analyzer
- `T1-F12-01`: Peak amplitude calculation in dBFS ($20 \log_{10}(\max |s|)$).
- `T1-F12-02`: Root Mean Square (RMS) level calculation across audio buffers ($\sqrt{\frac{1}{N} \sum s[n]^2}$).
- `T1-F12-03`: Digital clipping detector flagging true if any sample $|s| \ge 1.0$.
- `T1-F12-04`: Tempo drift score calculation via cross-correlation against master BPM grid.
- `T1-F12-05`: Spectral flatness measurement distinguishing tonal harmonic content from white noise.

### Feature 13: 48 kHz Analog Tape & Sub-Bass Mastering
- `T1-F13-01`: Studer A800 tape saturation modeling via hyperbolic tangent transfer curve ($f(x) = \tanh(1.25x)$).
- `T1-F13-02`: Sub-bass resonant warmth boost (+3.2 dB peak centered at 65 Hz).
- `T1-F13-03`: High-pass sub-rumble cutoff filter (-24 dB/octave attenuation below 28 Hz).
- `T1-F13-04`: Stereo Blumlein spatial width expansion (1.22x stereo field enlargement).
- `T1-F13-05`: Lookahead brickwall limiter preventing inter-sample peaks above $-0.3\text{ dBFS}$.

### Feature 14: Node 1: `ollama_producer_panel_node`
- `T1-F14-01`: Formats lyric sections with structural tags: `[Intro]`, `[Verse 1]`, `[Chorus]`, `[Outro]`.
- `T1-F14-02`: Generates clean YuE `genre.txt` stripping descriptive cues from bracketed lyrics.
- `T1-F14-03`: Generates Suno/Udio rich production cues for custom-mode generation.
- `T1-F14-04`: Dynamically configures panel prompts based on input genre, tempo, and time signature.
- `T1-F14-05`: Attaches section arrangement map and chord progressions to `MusicProductionState`.

### Feature 15: Node 2: `groove_and_yue_synthesis_node`
- `T1-F15-01`: Generates rhythmic foundation track (`draft_groove.wav`) matching requested BPM and genre.
- `T1-F15-02`: Generates dual stems (`yue_vocals.wav` and `yue_instrumentals.wav`) locked to groove grid.
- `T1-F15-03`: External CLI subprocess spawning awaiting child exit for 100% VRAM release.
- `T1-F15-04`: Pure-Rust DSP fallback execution when GPU VRAM is below threshold.
- `T1-F15-05`: Commits generated groove and dual-stem paths to state snapshot.

### Feature 16: Node 3: `vocal_timbre_fusion_node`
- `T1-F16-01`: Transforms guide vocal stem into 1977 soulful raspy Jamaican tenor vocal timbre.
- `T1-F16-02`: Applies optional `vocal_ref_wav` audio conditioning when provided.
- `T1-F16-03`: Adapts vocal processing to universal archetypes (Smooth Pop, Rock Baritone, Gospel Choir).
- `T1-F16-04`: Preserves original vocal phrasing timing within $\le 5\text{ ms}$ alignment error.
- `T1-F16-05`: Outputs `raspy_tenor_vocals.wav` with zero clipping and updates `stem_paths`.

### Feature 17: Node 4: `virtuoso_stem_inpainting_node`
- `T1-F17-01`: Separates instrumental track into 4 distinct stems (`drums`, `bass`, `guitar`, `other`).
- `T1-F17-02`: Inpaints virtuosic electric guitar solo over designated `[Guitar Solo]` bar range.
- `T1-F17-03`: Inpaints polyrhythmic drum rolls (nyabinghi hand drums, snare fills) over designated sections.
- `T1-F17-04`: Slices and crossfades inpainted stems with sample-accurate BPM grid alignment.
- `T1-F17-05`: Updates `stem_paths` with `inpainted_guitar.wav` and `inpainted_drums.wav`.

### Feature 18: Node 5: `dsp_and_judge_gate_node`
- `T1-F18-01`: Evaluates spectral telemetry across all stems and checks for clipping.
- `T1-F18-02`: Obtains Judge model score ($0.0$ to $10.0$) evaluating musicality and balance.
- `T1-F18-03`: Quality gate passes when clipping is false, peak $\le 0.0\text{ dBFS}$, and score $\ge 7.5$.
- `T1-F18-04`: Conditional routing to Node 4 (inpainting) on stem imbalance when $retry < 2$.
- `T1-F18-05`: Conditional routing to Node 2 on vocal tempo drift ($tempo\_drift > 0.5$) when $retry < 2$.

### Feature 19: Node 6: `analog_mastering_node`
- `T1-F19-01`: Combines all isolated stems into cohesive stereo master mix using `StemMixConfig`.
- `T1-F19-02`: Applies 48 kHz analog tape saturation and sub-bass resonance warmth.
- `T1-F19-03`: Evaluates reference mastering profile against `master_ref_wav` frequency envelope.
- `T1-F19-04`: Enforces true peak limiter ceiling of $-0.3\text{ dBFS}$ with zero digital overs.
- `T1-F19-05`: Produces final mastered audio artifact (`final_master.wav`) at 48 kHz 24-bit PCM.

### Feature 20: Deterministic VRAM Process Isolation
- `T1-F20-01`: Spawns external CLI model stages using `tokio::process::Command`.
- `T1-F20-02`: Enforces process termination and awaits child process handle drop before advancing.
- `T1-F20-03`: Verifies GPU memory allocation returns to pre-execution baseline.
- `T1-F20-04`: Implements execution timeout terminating stalled external workers cleanly.
- `T1-F20-05`: Prevents PyTorch CUDA memory leak across sequential pipeline stages.

### Feature 21: Tokio HTTP JSON REST API (Port 5050)
- `T1-F21-01`: Starts Tokio HTTP server listening on configured port (`127.0.0.1:5050`).
- `T1-F21-02`: `POST /api/songs/create` validates request body and initiates StateGraph execution.
- `T1-F21-03`: `POST /api/songs/iterate` creates new version branch with modified parameters.
- `T1-F21-04`: `POST /api/songs/regenerate-stem` triggers targeted Node 4 stem inpainting.
- `T1-F21-05`: `GET /api/songs/:id/versions` returns complete version lineage and telemetry history.

### Feature 22: MCP JSON-RPC 2.0 Stdio Server
- `T1-F22-01`: Processes JSON-RPC 2.0 requests over standard input and responds on standard output.
- `T1-F22-02`: Tool invocation `music_create_hit` parses parameters and returns song state.
- `T1-F22-03`: Tool invocation `music_iterate_song` updates song state and produces new version.
- `T1-F22-04`: Tool invocation `music_regenerate_stem` re-synthesizes specific instrumental stem.
- `T1-F22-05`: Tool invocation `music_master_track` applies analog tape mastering to existing stems.

### Feature 23: Flagship Preset `--create-reggae-anthem`
- `T1-F23-01`: CLI flag `--create-reggae-anthem` loads the 76 BPM "One Love" spiritual successor preset.
- `T1-F23-02`: Pre-loads exact Bob Marley compassion lyrics and roots reggae style prompt.
- `T1-F23-03`: Executes full 6-node pipeline end-to-end in standalone pure-Rust DSP mode.
- `T1-F23-04`: Generates 48 kHz `final_master.wav` with one-drop drums and crying blues guitar solo.
- `T1-F23-05`: Verifies final master exhibits authentic 1977 warm tape saturation and sub-bass weight.

### Feature 24: Universal CLI Commands
- `T1-F24-01`: CLI flag `--create-hit --genre "<genre>" --prompt "<prompt>"` parses universal requests.
- `T1-F24-02`: CLI options `--bpm <bpm>` and `--key <key>` override default genre settings.
- `T1-F24-03`: CLI flag `--pure-rust` forces pure-Rust DSP mode regardless of GPU presence.
- `T1-F24-04`: CLI command `--serve --port <port>` starts HTTP REST server.
- `T1-F24-05`: CLI command `--mcp` enters interactive MCP JSON-RPC 2.0 stdio server mode.

### Feature 25: E2E Testing Suite & Adversarial Audit
- `T1-F25-01`: End-to-end test runner verifies workspace compile and executes without panics.
- `T1-F25-02`: Opaque-box test harness isolates temporary song workspaces and tears down cleanly.
- `T1-F25-03`: Spectral telemetry asserts zero clipping and valid dynamic range across all test runs.
- `T1-F25-04`: Real-world genre test suite covers diverse musical styles with zero mocks.
- `T1-F25-05`: Handoff and test readiness metadata verification asserts complete feature coverage.

---

## 4. Tier 2: Boundary, Extreme Limits & Stress Catalog (125 Test Cases)

### Boundary Category: Extreme Tempos, Meters & Tonal Scalability (B01–B05)
- `T2-B01`: Extreme minimum tempo boundary ($40\text{ BPM}$): verify sample calculation, oscillator stability, and zero divide-by-zero.
- `T2-B02`: Extreme maximum tempo boundary ($220\text{ BPM}$): verify sub-division timing and anti-aliasing filter stability at high speeds.
- `T2-B03`: Odd and compound meters (`7/8`, `5/4`, `12/8`): verify exact bar boundary alignment and crossfade grid calculation.
- `T2-B04`: Microtonal / exotic scales (Arabic Hijaz, Japanese Hirajoshi, Blues Hexatonic): verify guitar solo and vocoder frequency mapping.
- `T2-B05`: Ultra-short 2-bar sections and ultra-long 64-bar sections: verify buffer memory bounds and slice indices.

### Boundary Category: Audio Buffer, Volume & Clipping Stress (B06–B10)
- `T2-B06`: High-amplitude input burst ($+12\text{ dBFS}$ over 0 dB): verify analog tape saturation softly limits output to $[-1.0, 1.0]$.
- `T2-B07`: Pure DC offset / sub-sonic rumble ($5\text{ Hz}$ sine wave): verify 28 Hz high-pass filter eliminates DC drift without phase distortion.
- `T2-B08`: Nyquist-boundary high-frequency burst ($23.9\text{ kHz}$ at 48 kHz sample rate): verify anti-aliasing filter prevents foldback artifacts.
- `T2-B09`: Near-silent input ($-90\text{ dBFS}$ ambient): verify telemetry calculations avoid NaN/inf in logarithmic dBFS conversions.
- `T2-B10`: Zero-length and single-sample audio buffers: verify WAV reader and DSP stages return clean error instead of panicking.

### Boundary Category: Memory, Hardware & Subprocess Stress (B11–B15)
- `T2-B11`: Zero GPU availability (`nvidia-smi` missing or fails): verify `SystemMemory::detect_live()` seamlessly defaults to CPU pure-Rust DSP.
- `T2-B12`: Boundary VRAM edge ($3.99\text{ GB}$ vs $4.01\text{ GB}$ free): verify strict threshold enforcement for HF model offloading.
- `T2-B13`: Worker subprocess abrupt termination (exit code 1 or killed): verify pipeline catches child error and falls back to pure-Rust DSP.
- `T2-B14`: Rapid sequential stage execution (10 runs in a tight loop): verify Windows process handles and temp audio files do not leak.
- `T2-B15`: Concurrent StateGraph executions: verify isolated temporary directories prevent file collisions.

### Boundary Category: Ollama Panel & Prompt Ingestion Stress (B16–B20)
- `T2-B16`: Ollama endpoint offline (`connection refused` on `11434`): verify instant fallback to built-in genre blueprints with zero delay.
- `T2-B17`: Malformed Ollama JSON response (unclosed brackets, trailing garbage): verify JSON extractor recovers valid structure.
- `T2-B18`: DeepSeek reasoning tokens (`<think>...unclosed block...`): verify tokenizer sanitizes prompt before parsing tags.
- `T2-B19`: Extremely long prompt (16,000 characters with special characters and emojis): verify prompt truncation and safe handling.
- `T2-B20`: Raw lyrics missing all structural brackets: verify Node 1 auto-synthesizes mandatory `[Intro]`, `[Verse]`, `[Chorus]`, and `[Outro]` tags.

### Boundary Category: State, Versioning & Quality Gate Retry Stress (B21–B25)
- `T2-B21`: Quality gate retry cap exhaustion: force `quality_passed = false` for 2 retries; verify graph advances to Node 6 on 3rd attempt.
- `T2-B22`: Vocal tempo drift trigger: inject artificial tempo drift ($score = 0.85$); verify conditional edge routes back to Node 2 (groove/YuE).
- `T2-B23`: Stem balance trigger: inject guitar stem imbalance; verify conditional edge routes back to Node 4 (inpainting).
- `T2-B24`: Concurrent version branching: verify `POST /api/songs/iterate` creates distinct monotonic versions ($v1, v2, v3$) without overwrite.
- `T2-B25`: Read-only / full disk simulation: verify checkpointer catches IO error and preserves state in memory.

---

## 5. Tier 3: Pairwise & Cross-Feature Integration Matrix

The following 25 pairwise integration tests validate interface contracts across module boundaries:

| Pair ID | Crates / Modules Involved | Contract Under Test | Verification Objective |
|:---:|:---|:---|:---|
| `T3_PAIR_01` | `music_fusion::memory` ↔ `music_core::state` | VRAM probe determines `engine_mode` | Assert `SystemMemory` auto-sets `engine_mode = "pure_rust"` when VRAM < 4GB. |
| `T3_PAIR_02` | `music_fusion::memory` ↔ `music_fusion::panel` | Available RAM gates Ollama model tier | Assert model selection respects Dynamic Available Memory Law before querying `/api/chat`. |
| `T3_PAIR_03` | `music_fusion::panel` ↔ `music_core::genre` | Dynamic prompt matches canonical genre | Assert Lyricist/Arranger prompts adopt genre-specific terms (e.g. "one-drop", "log drum"). |
| `T3_PAIR_04` | `music_core::genre` ↔ `music_dsp::groove_synth` | Genre taxonomy dictates drum groove | Assert requesting "Afrobeats" generates 108 BPM clave; "Reggae" generates 76 BPM one-drop. |
| `T3_PAIR_05` | `music_core::state` ↔ `music_pipeline::graph` | State passed through StateGraph nodes | Assert mutations in Node 1 are visible to Node 2 and persisted to snapshot. |
| `T3_PAIR_06` | `music_core::state` ↔ `music_dsp::wav` | Stem file paths map to valid WAV files | Assert all keys in `stem_paths` point to valid 48 kHz stereo WAV headers on disk. |
| `T3_PAIR_07` | `music_dsp::groove_synth` ↔ `music_dsp::wav` | Drum synthesis produces valid PCM buffer | Assert synthesized groove encodes to WAV and decodes back with $\le 10^{-6}$ error. |
| `T3_PAIR_08` | `music_dsp::groove_synth` ↔ `music_dsp::vocal_timbre` | Vocal timing phase-locked to groove | Assert vocal transients align with groove beat markers within $\le 5\text{ ms}$. |
| `T3_PAIR_09` | `music_dsp::vocal_timbre` ↔ `music_dsp::guitar_wah` | Vocal and lead guitar frequency separation | Assert vocal formant peaks do not clash with wah bandpass center frequency. |
| `T3_PAIR_10` | `music_dsp::guitar_wah` ↔ `music_dsp::crossfader` | Spliced guitar solo into instrumental bed | Assert crossfader seamlessly splices 8-bar solo with equal power across transition. |
| `T3_PAIR_11` | `music_dsp::crossfader` ↔ `music_dsp::telemetry_calc` | Crossfaded stem analyzed for clipping | Assert crossfade region exhibits zero digital clipping and smooth RMS envelope. |
| `T3_PAIR_12` | `music_dsp::telemetry_calc` ↔ `music_dsp::tape_mastering` | Pre-master telemetry guides tape saturation | Assert master compressor adjusts gain based on input stem peak and dynamic range. |
| `T3_PAIR_13` | `music_dsp::tape_mastering` ↔ `music_pipeline::node1` | Node 1 section map drives master dynamics | Assert master limiter respects chorus crescendo levels defined in section blueprint. |
| `T3_PAIR_14` | `music_pipeline::node1` ↔ `music_pipeline::node2` | YuE tags and lyrics passed to Stage 2 | Assert Node 2 conditions audio generation on exact bracketed tags from Node 1. |
| `T3_PAIR_15` | `music_pipeline::node2` ↔ `music_pipeline::node3` | Dual stems fed into vocal conversion | Assert Node 3 processes `yue_vocals.wav` while leaving `yue_instrumentals.wav` untouched. |
| `T3_PAIR_16` | `music_pipeline::node3` ↔ `music_pipeline::node4` | Vocals and separated stems fed to inpainter | Assert Node 4 inpainting preserves vocal track while regenerating lead stems. |
| `T3_PAIR_17` | `music_pipeline::node4` ↔ `music_pipeline::node5` | Inpainted stems evaluated by gate node | Assert Node 5 analyzes all 4 isolated stems plus vocal stem for phase lock. |
| `T3_PAIR_18` | `music_pipeline::node5` ↔ `music_pipeline::node6` | Gate quality pass triggers mastering | Assert Node 6 executes only when Node 5 flags `quality_passed = true`. |
| `T3_PAIR_19` | `music_pipeline::node6` ↔ `music_pipeline::hf_runner` | Subprocess termination before mastering | Assert GPU VRAM is 100% liberated prior to analog tape mastering execution. |
| `T3_PAIR_20` | `music_pipeline::runner` ↔ `music_cli::rest_server` | REST API streams pipeline state | Assert HTTP SSE stream receives real-time stage progress updates from StateGraph. |
| `T3_PAIR_21` | `music_cli::rest_server` ↔ `music_cli::mcp_server` | REST and MCP share unified StateGraph | Assert a song created via REST can be inspected or mastered via MCP tool call. |
| `T3_PAIR_22` | `music_cli::mcp_server` ↔ `music_cli::presets` | MCP invokes `--create-reggae-anthem` preset | Assert calling MCP `music_create_hit` with preset flag triggers One Love anthem. |
| `T3_PAIR_23` | `music_cli::presets` ↔ `music_cli::cli_args` | CLI flags override preset defaults | Assert passing `--bpm 80` to `--create-reggae-anthem` correctly overrides tempo. |
| `T3_PAIR_24` | `music_cli::cli_args` ↔ `music_core::genre` | CLI `--genre` routes to universal taxonomy | Assert any arbitrary genre string maps to valid taxonomy entry or fallback. |
| `T3_PAIR_25` | `music_pipeline::graph` ↔ `music_core::store` | StateGraph transitions committed to SQLite | Assert version history stored in SQLite matches final `MusicProductionState`. |

---

## 6. Tier 4: Real-World Multi-Genre Application Scenarios

The test suite exercises 8 end-to-end real-world music production scenarios spanning all major global genres:

### Scenario 1: Flagship 1977 Roots Reggae "One Love" Anthem (76 BPM, A Major)
- **Input**: Preset `--create-reggae-anthem` pre-loaded with Bob Marley compassion lyrics:
  `"One love shining through the pouring rain, Soft compassion washing out the pain..."`
- **Orchestration**: Full 6-node StateGraph execution at 76 BPM in A Major.
- **DSP Synthesis**: 76 BPM one-drop drum pattern with rimshot snare on 3, nyabinghi hand drums, deep dub sub-bass, offbeat guitar skank, crying blues-rock electric guitar solo with wah pedal, soulful raspy Jamaican tenor vocal formant, gospel choir backing, and Studer A800 analog tape mastering.
- **Verification Criteria**:
  - `bpm == 76`, `key_signature == "A Major"`.
  - Stems generated: `draft_groove`, `yue_vocals`, `yue_instrumentals`, `raspy_tenor_vocals`, `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, `final_master`.
  - Zero digital clipping ($|sample| < 1.0$), peak level between $-0.5$ and $-0.3\text{ dBFS}$, true RMS around $-14.0\text{ dBFS}$.
  - Version record saved to `./storage/songs/song-one-love-shining-1977/v1/state.json`.

### Scenario 2: High-Energy Afrobeats / Amapiano Hit (108 BPM, F# Minor)
- **Input**: `--create-hit --genre "Afrobeats" --prompt "Lagos midnight energy, infectious dance groove, syncopated log drums, warm acoustic guitar, brass swells" --bpm 108 --key "F# Minor"`.
- **DSP Synthesis**: 108 BPM syncopated log-drum bass, clave shaker polyrhythms, melodic nylon guitar chords, smooth tenor vocals, brass horn stabs, and punchy modern master.
- **Verification Criteria**:
  - Peak level $\le -0.3\text{ dBFS}$, tempo drift score $\le 0.15$.
  - Audio duration matches expected 4-bar intro + 8-bar verse + 8-bar chorus structure.

### Scenario 3: Mainstream Radio Pop Hit (120 BPM, C Major)
- **Input**: `--create-hit --genre "Pop" --prompt "Upbeat summer radio anthem, bright acoustic guitar, soaring female soprano hook, driving four-on-the-floor kick, crystal clear modern production" --bpm 120 --key "C Major"`.
- **DSP Synthesis**: Four-on-the-floor kick with open offbeat hi-hat, bright strummed guitar, smooth vocal formant, radio-ready loud master ($-14\text{ LUFS}$).
- **Verification Criteria**:
  - High dynamic balance, zero digital overs, stereo width $\ge 1.15$.

### Scenario 4: Fast Bebop Jazz / Fusion (140 BPM, Bb Major)
- **Input**: `--create-hit --genre "Jazz" --prompt "Late night Harlem jazz club, fast bebop swing, walking acoustic upright bass, swung ride cymbal, expressive saxophone lead improvisation" --bpm 140 --key "Bb Major"`.
- **DSP Synthesis**: Swung ride cymbal triplet rhythm, walking acoustic bassline, dynamic piano chords, expressive lead phrasing, intimate audiophile dynamic master.
- **Verification Criteria**:
  - High dynamic range ($\ge 14\text{ dB}$), zero compression artifacts, natural acoustic room decay.

### Scenario 5: Stadium Psychedelic Rock & Screaming Guitar Solo (128 BPM, E Minor)
- **Input**: `--create-hit --genre "Rock" --prompt "Epic 1970s stadium rock anthem, driving heavy drums, distorted power chords, screaming psychedelic blues rock guitar solo with heavy wah-wah pedal" --bpm 128 --key "E Minor"`.
- **DSP Synthesis**: Driving acoustic drum backbeat, overdriven rhythm guitar, Karplus-Strong physical string solo with dynamic wah-filter sweep and pitch bends, vintage analog tape warmth.
- **Verification Criteria**:
  - Inpainted solo seamlessly crossfaded over 8-bar solo section, guitar solo intensity $\ge 0.90$.

### Scenario 6: Cinematic Orchestral & Epic Choral Score (84 BPM, D Minor, 6/8 Compound)
- **Input**: `--create-hit --genre "Cinematic" --prompt "Majestic fantasy film score, thunderous orchestral timpani rolls, soaring string sections, powerful brass fanfare, lush cathedral choir harmonies" --bpm 84 --key "D Minor" --time-signature "6/8"`.
- **DSP Synthesis**: Compound 6/8 pulse (2 dotted-quarter beats per bar), orchestral percussion rolls, string ensemble pads, French horn swells, cathedral reverb envelope.
- **Verification Criteria**:
  - Time signature verification ($6/8$), wide stereo imaging ($1.30$), wide dynamic range.

### Scenario 7: Heavy Boom-Bap & 808 Trap Hip-Hop (144 BPM, G Minor)
- **Input**: `--create-hit --genre "Hip-Hop" --prompt "Dark late-night trap banger, booming 808 sub-bass, crisp rolling hi-hat triplets, hard snare, rhythmic cadence vocal delivery" --bpm 144 --key "G Minor"`.
- **DSP Synthesis**: Hard snare on beat 3, 32nd-note rolling hi-hats, pitch-swept 808 sub-bass ($40\text{ Hz}$ to $32\text{ Hz}$), tight rhythmic vocal envelope, punchy brickwall master.
- **Verification Criteria**:
  - Sub-bass energy concentration below 80 Hz, zero inter-sample clipping on hard transients.

### Scenario 8: Club Melodic House & EDM (126 BPM, A Minor)
- **Input**: `--create-hit --genre "EDM" --prompt "Sunrise festival melodic house, driving 4-on-the-floor kick, euphoric synthesizer lead chords, energetic sidechain pump, heavy club sub-bass" --bpm 126 --key "A Minor"`.
- **DSP Synthesis**: Driving four-on-the-floor kick, sidechain volume ducking envelope on chord synths, resonant filter sweeps, club-calibrated sub-heavy master.
- **Verification Criteria**:
  - Sidechain envelope modulation visible in RMS dips on kick hits, integrated loudness $-12.5\text{ to } -14.0\text{ LUFS}$.

---

## 7. Test Architecture, Runner Commands & CI/CD Workflow

### 7.1 Test Harness & File Architecture
Tests are implemented as an opaque-box integration suite residing in `tests/e2e_tests.rs` (and modular test files):

```
d:\harfile\music\
├── Cargo.toml
├── TEST_INFRA.md
├── TEST_READY.md
├── crates\
│   ├── music_core\
│   ├── music_fusion\
│   ├── music_dsp\
│   ├── music_pipeline\
│   └── music_cli\
└── tests\
    ├── e2e_tests.rs           # Master opaque-box integration and scenario runner
    ├── tier1_feature_tests.rs # 125 Tier-1 isolated feature tests
    ├── tier2_boundary_tests.rs# 125 Tier-2 boundary & edge tests
    ├── tier3_pairwise_tests.rs# 25 Tier-3 pairwise contract tests
    └── tier4_scenario_tests.rs# 8 Tier-4 multi-genre real-world scenarios
```

### 7.2 Standard Test Execution Commands

1. **Run Full Test Suite Across Workspace**:
   ```powershell
   cargo test --workspace -- --nocapture
   ```

2. **Run Opaque-Box E2E Suite Specifically**:
   ```powershell
   cargo test --test e2e_tests -- --nocapture
   ```

3. **Run Specific Tiers**:
   - Tier 1: `cargo test --test e2e_tests tier1_ -- --nocapture`
   - Tier 2: `cargo test --test e2e_tests tier2_ -- --nocapture`
   - Tier 3: `cargo test --test e2e_tests tier3_ -- --nocapture`
   - Tier 4: `cargo test --test e2e_tests tier4_ -- --nocapture`

4. **Run Smoke Test (Flagship Reggae Anthem Pure-Rust)**:
   ```powershell
   cargo test --test e2e_tests test_scenario1_flagship_reggae_anthem -- --nocapture
   ```

5. **Run CLI Preset Smoke Test Directly**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --pure-rust
   ```

### 7.3 Quality Thresholds & Automated Invalidation Rules
A test run is classified as **FAILED** if any of the following occur:
1. **Digital Audio Clipping**: Any audio sample reaches $\ge 1.0$ or $\le -1.0$.
2. **Phase / Tempo Drift**: Any synthesized stem deviates by $> 0.30$ relative to the master BPM grid.
3. **Master Level Non-Compliance**: Master true peak exceeds $0.0\text{ dBFS}$ (target: $-0.5\text{ to } -0.3\text{ dBFS}$).
4. **VRAM Leakage**: External child process fails to terminate, leaving zombie GPU handles.
5. **Environment Pollution**: Test alters system/user `PATH` or Windows registry environment keys.

---

## 8. Conclusion
This `TEST_INFRA.md` specification defines an exhaustive, rigorous, zero-mock, requirement-driven test infrastructure covering all 25 features and all musical genres known to humanity, anchored by the flagship 1977 Roots Reggae anthem preset. All tests operate strictly via public interfaces, honoring the dual-track project architecture and safeguarding complete system integrity.
