# Original User Request

## Initial Request — 2026-11-01T00:55:00Z

Refactor and expand `d:\harfile\music` so that the entire AI Music Producer Agent, the LangGraph `StateGraph` orchestration engine, the adapted ModelFusion hardware/LLM fusion modules, and the 5-stage sequential music pipeline live 100% inside the standalone `d:\harfile\music` Rust workspace. Do not create a separate Python LangGraph project and do not link against `d:\harfile\ModelFusion` at runtime.

**1. Adapt ModelFusion Patterns Directly Into `d:\harfile\music` (Standalone Rust)**
Port the core architectural patterns from `d:\harfile\ModelFusion` directly into internal modules inside `d:\harfile\music`:

* **Hardware & VRAM Sizing (`src/memory.rs`)**: Port the `SystemMemory::detect_live()` pattern from `ModelFusion` (`crates/model_selection/src/memory.rs`) to probe live system RAM and GPU VRAM before each music stage runs, automatically selecting installed Ollama models (`qwen2.5:32b`, `glm4:9b`, `flint:latest`, `qwen2.5:7b`) for songwriting and deciding whether enough VRAM is free for heavy Hugging Face audio checkpoints or if a stage should route to the built-in 48 kHz pure-Rust DSP synthesizer.
* **Multi-Model Ollama Songwriter/Producer Fusion (`src/fusion_panel.rs`)**: Adapt `ModelFusion`'s `fusion.rs` and `judge.rs` panel using `reqwest` + `serde_json` against `http://127.0.0.1:11434/api/tags` and `http://127.0.0.1:11434/api/chat`. Run a parallel panel of local Ollama models (Lyricist, Roots Reggae Arranger, and Sound Engineer), evaluate them with a Judge model, and synthesize the final YuE `genre.txt` style tags, bracketed `lyrics.txt`, and bar-by-bar arrangement map.

**2. Native Rust LangGraph Engine (`src/graph/mod.rs`)**
Implement a native Rust `StateGraph<MusicProductionState>` engine inside `d:\harfile\music` (either using the Rust `langgraph` crate or a self-contained async `tokio` Pregel/DAG `StateGraph` struct with `add_node`, `add_edge`, `add_conditional_edges`, and state checkpointing per song version).

Define `MusicProductionState` (`#[derive(Clone, Serialize, Deserialize)]`) with:

* `song_id`: String
* `version`: u32
* `engine_mode`: String (`"auto"`, `"pure_rust"`, or `"hf_hybrid"`)
* `style_prompt`: String
* `lyrics`: String
* `bpm`: u32 (`76`)
* `key_signature`: String (`"A Major"`)
* `vocal_ref_wav`: Option<PathBuf>
* `master_ref_wav`: Option<PathBuf>
* `stem_paths`: HashMap<String, PathBuf> (`draft_groove`, `yue_vocals`, `yue_instrumentals`, `raspy_tenor_vocals`, `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, `inpainted_drums`, and `final_master`)
* `telemetry`: StemTelemetry (peak dBFS, RMS, dynamic range, tempo drift score, clipping bool)
* `judge_score`: f32
* `quality_passed`: bool
* `retry_count`: u8

**3. Native Rust LangGraph Nodes (Sequential 5-Stage Fusion Pipeline)**
Wire the following async Rust nodes into the `StateGraph`, where each node checks `SystemMemory::detect_live()`, executes either the external Hugging Face stage via `tokio::process::Command` (waiting for process termination so GPU VRAM is 100% freed between nodes) or the built-in 48 kHz pure-Rust synthesis/DSP engine, and records a new snapshot in the version tracker:

* **Node 1 (`ollama_producer_panel_node`)**: Runs the adapted ModelFusion multi-LLM Ollama panel (`http://127.0.0.1:11434/api/chat`) to refine the 1977 roots reggae compassion prompt and verify that all lyric sections use clean structural tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`).
* **Node 2 (`groove_and_yue_synthesis_node`)**: Executes Stage 1 (`ACE-Step/ACE-Step-v1.5` or pure-Rust 76 BPM one-drop groove generator) to create `draft_groove.wav`, followed immediately by Stage 2 (`m-a-p/YuE-s1-7B-anneal-en-icl` + `m-a-p/YuE-s2-1B-general` or pure-Rust multi-stem harmonic renderer) conditioned on `draft_groove.wav` to output isolated `yue_vocals.wav` and `yue_instrumentals.wav`.
* **Node 3 (`jamaican_tenor_timbre_node`)**: Executes Stage 3 (`Plachtaa/Seed-VC` with `f0_condition=True` or the built-in pure-Rust vocal formant/rasp/pitch-lock processor) to imprint the soulful 1977 raspy Jamaican male tenor timbre onto `raspy_tenor_vocals.wav`.
* **Node 4 (`virtuoso_stem_inpainting_node`)**: Executes Stage 4. Splits the instrumental via `facebook/htdemucs_ft` into 4 stems (`drums`, `bass`, `guitar`, `other`) and regenerates the 16-bar `[Instrumental Interlude]` and `[Guitar Solo]` sections plus the polyrhythmic nyabinghi and rimshot snare rolls using `stabilityai/stable-audio-open-1.0` (or the pure-Rust physical guitar string/wah modeler and polyrhythmic drum synthesizer), splicing them with sample-accurate crossfades at 76 BPM.
* **Node 5 (`dsp_and_judge_gate_node`)**: Runs native Rust spectral/waveform inspection on all stems (verifying zero digital clipping, proper vocal-to-instrumental headroom, and 76 BPM phase alignment) plus an Ollama Judge evaluation. Implement a conditional edge: if `quality_passed == false` and `retry_count < 2`, route back to `virtuoso_stem_inpainting_node` (or `groove_and_yue_synthesis_node` if vocal drift occurred); otherwise route to `analog_mastering_node`.
* **Node 6 (`analog_mastering_node`)**: Executes Stage 5 (`AudioSR` 48 kHz upsampling + `Matchering 2.0` or the built-in pure-Rust 48 kHz analog tape saturation, sub-bass warmth, and reference EQ Matchering engine) to produce `final_master.wav`.

**4. Expose the Embedded LangGraph via Tokio REST, MCP Stdio, and CLI**
Wire the compiled Rust `StateGraph` directly into the existing standalone interfaces in `d:\harfile\music`:

* **Tokio HTTP JSON REST API (`http://127.0.0.1:5050`)**: `POST /api/songs/create`, `POST /api/songs/iterate`, `POST /api/songs/regenerate-stem`, `POST /api/songs/master`, and `GET /api/songs/:id/versions` invoke the compiled Rust `StateGraph` (either full graph execution or targeted single-node re-execution for stem regeneration) and stream real-time node transitions to the Web Studio UI.
* **MCP JSON-RPC 2.0 (`--mcp`)**: Calling `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, or `music_master_track` triggers the internal Rust `StateGraph`.
* **Default "One Love" Anthem Preset (`--create-reggae-anthem`)**: Add a CLI command that boots the standalone Rust `StateGraph` pre-loaded with our 76 BPM "One Love" human compassion lyrics and the exact style prompt (`"1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, 76 BPM, hypnotic one drop master drumming with complex polyrhythmic snare rolls and deep sub bass, virtuosic soaring psychedelic blues rock electric guitar solos, warm analog tape production, lush gospel backing choir"`), running end-to-end and outputting the mastered 48 kHz `.wav` and version tree in `d:\harfile\music`.


## Follow-up — 2026-11-01T00:58:27Z

CRITICAL USER REQUIREMENT UPDATE:
The user just emphasized:
> "this can be any type of Music not just reggae it can be all types of music known to man"

Please ensure that the architecture in `d:\harfile\music` is a **Universal Multi-Genre AI Music Production Engine** supporting ALL musical genres known to humanity, not hardcoded only to reggae! Specifically:
1. **Universal Genre & Style Taxonomy**:
   - Support any genre/style passed via CLI (`--genre`, `--style`, `--bpm`, `--key`, `--create-hit`), REST API (`POST /api/songs/create`), MCP (`music_create_hit`), or Web Studio UI—including Roots Reggae, Afrobeats, Highlife, Amapiano, Soul/R&B, Gospel, Hip-Hop/Trap, Blues, Psychedelic/Stadium Rock, Metal, Jazz/Fusion, Classical/Orchestral/Cinematic, Pop, EDM/House/Techno/Synthwave, Country/Folk/Acoustic, Latin/Salsa/Bossa Nova, Ambient, and any custom hybrid fusion prompt!
   - Keep `--create-reggae-anthem` (the 1977 Roots Reggae "One Love" spiritual successor at 76 BPM) as a flagship preset, AND also support `--create-hit --genre "<any genre>" --prompt "<any topic>"` (plus built-in genre presets across world music styles).
2. **Genre-Adaptive LangGraph Nodes & DSP Synthesis**:
   - **Node 1 (`ollama_producer_panel_node`)**: Dynamically adapts the Ollama specialist panel personas (Lyricist, Genre Music Theorist/Arranger, Lead Instrumentalist/Sound Engineer, and Judge) to the target genre, era, tempo (40–220 BPM), time signature (`4/4`, `3/4`, `6/8`, `12/8`, `5/4`, `7/8`), scale/mode (Major, Natural/Harmonic/Melodic Minor, Dorian, Mixolydian, Phrygian, Lydian, Pentatonic, Blues, Arabic/ Hijaz, Japanese/Hirajoshi, Afrobeat polyrhythms), and instrumentation.
   - **Node 2 (`groove_and_yue_synthesis_node`)**: Generates genre-authentic drum & rhythm grooves (`ACE-Step/ACE-Step-v1.5` or universal pure-Rust drum pattern synthesizer supporting One-Drop Reggae, Four-on-the-Floor House/EDM, Boom-Bap/Trap 808s, Afrobeats/Amapiano log-drum Tresillo/Clave syncopation, Rock/Metal double-kick backbeat, Jazz swing ride, Orchestral timpani/percussion, Waltz/Compound meters) and conditions `YuE` (`m-a-p/YuE-s1-7B-anneal-en-icl` + `m-a-p/YuE-s2-1B-general`).
   - **Node 3 (`vocal_timbre_fusion_node` / `jamaican_tenor_timbre_node`)**: Supports `Plachtaa/Seed-VC` and pure-Rust formant/timbre modeling for ANY vocal archetype (Soulful Raspy Tenor, Smooth R&B/Pop Soprano/Alto, Operatic/Choral, Gritty Rock Baritone, Rap/Hip-Hop rhythmic cadence, Warm Acoustic Crooner, Gospel Choir harmonies).
   - **Node 4 (`virtuoso_stem_inpainting_node`)**: Supports `facebook/htdemucs_ft` + `stabilityai/stable-audio-open-1.0` / `facebook/musicgen-style` and pure-Rust physical modeling across diverse lead & harmonic instruments (Electric Guitar with Wah/Overdrive, Acoustic Guitar/Nylon, Grand Piano/Rhodes/Hammond B3 Organ, Analog/FM Synths, Orchestral Strings & Brass Section, Saxophone/Trumpet leads, 808 Sub-Bass / Acoustic Upright Bass / Electric Slap & Dub Bass).
   - **Node 6 (`analog_mastering_node`)**: Supports genre-matched reference mastering profiles in `AudioSR` 48 kHz + `Matchering 2.0` (1970s Warm Analog Tape, Modern Punchy Pop/Hip-Hop Master, Club/EDM Sub-Heavy Master, Wide Cinematic Orchestral Master, Intimate Acoustic/Jazz Audiophile Master).
