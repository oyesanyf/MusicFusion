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


## Follow-up — 2026-11-01T03:45:20Z

CRITICAL USER REQUIREMENT UPDATE FOR MILESTONE 4 (`crates/music_cli` & Web Studio UI):
The user just requested:
> "this should have a web uI that loads ths browser on startup"

Please ensure that Milestone 4 (`crates/music_cli` and `webui/`) implements a complete, interactive **Web Studio UI** served at `http://127.0.0.1:5050/` AND **automatically opens the browser on startup**:

1. **Auto-Launch Browser on Startup**:
   - When the binary starts in default mode (running with no flags, or with `--serve` / `--studio`), it must start the Tokio HTTP server on `http://127.0.0.1:5050` (with resilient port fallback `5050..5060` if port 5050 is busy) AND automatically launch the default web browser to `http://127.0.0.1:<port>/` once the listener is bound (on Windows: `cmd /C start "" "http://127.0.0.1:5050"` using `CREATE_NO_WINDOW` `0x08000000`).
   - Also support a `--no-browser` flag so automated tests and headless callers can run the server without spawning a browser window (and never launch a browser when `--mcp` stdio mode is used).
   - Even when CLI generation flags like `--create-reggae-anthem` or `--create-hit` are run, support launching into the Web UI (or provide `--open-ui` / default behavior) so the user can immediately play, inspect, and iterate on the generated song in the browser!

2. **Full Interactive DAW & Song Iteration Web Studio UI (`GET /` and `GET /index.html`)**:
   - Serve a rich, modern, dark-themed studio Web UI from `http://127.0.0.1:5050/` (and audio files via `GET /api/songs/:id/versions/:version/audio/:stem` with `Content-Type: audio/wav` so HTML5 `<audio>` players stream and play every stem and master track directly in the browser!).
   - **Interactive Controls & Zero Empty States**:
     - **Genre & Style Studio**: Dropdown/pills for all 20+ world music genres (Roots Reggae, Afrobeats, Amapiano, Hip-Hop/Trap, R&B/Soul, Gospel, Blues, Stadium Rock, Metal, Jazz, Classical/Orchestral, EDM/House, Pop, Latin, etc.), editable Style Prompt, BPM slider/input (40–220 BPM), Key Signature, Scale/Mode, Time Signature, Vocal Archetype, and Engine Mode (`auto`, `pure_rust`, `hf_hybrid`).
     - **Pre-Loaded Hit Templates**: One-click preset loader for the 76 BPM 1977 Roots Reggae "One Love" Compassion Anthem, "Rise Up Together", and multi-genre hit templates.
     - **Editable Structured Lyrics & Arrangement Sections**: Interactive editor for `[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`, plus live preview of sanitized YuE `genre.txt` and `lyrics.txt`.
     - **Live 6-Node LangGraph Pipeline Visualizer**: Visual status cards for all 6 nodes (`Node 1: Ollama Producer Panel` -> `Node 2: Groove & YuE` -> `Node 3: Vocal Timbre Fusion` -> `Node 4: Virtuoso Stem Inpainting` -> `Node 5: DSP & Judge Gate` -> `Node 6: 48 kHz Analog Mastering`) with live telemetry (Peak dBFS, RMS, Dynamic Range, Tempo Drift, Judge Score, Quality Gate status).
     - **Multi-Track Stem Mixer & Audio Player**: Built-in audio players for `final_master.wav` and all isolated stems (`draft_groove`, `yue_vocals`, `raspy_tenor_vocals`, `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, `inpainted_drums`) with play/pause, mute/solo, and one-click stem regeneration (`POST /api/songs/regenerate-stem`) and re-mastering (`POST /api/songs/master`).
     - **Iterative Song Refinement & Version Tree**: Interactive iteration bar (`POST /api/songs/iterate`) where the user can type changes (e.g. "Make the guitar solo heavier with more wah, boost sub-bass, and update Chorus lyrics") to produce `v2`, `v3`, etc., and switch between versions (`GET /api/songs/:id/versions`).
     - **Live Hardware & Ollama Status Header**: Displays live detected RAM/VRAM (`SystemMemory::detect_live()`) and active Ollama models.


## Follow-up — 2026-11-01T03:49:15Z

CRITICAL USER REQUIREMENT UPDATE:
The user just added:
> "if a user does not like a sone he must be able to itegrate and adjust it. create a test sone when you are complete"
("if a user does not like a song, he must be able to iterate and adjust it. Create a test song when you are complete.")

Please ensure the following are fully implemented and executed before completion:
1. **Rich Song Iteration & Granular Adjustment (Web UI + REST + CLI + MCP)**:
   - When a user does not like a song (or any part of it), they must be able to easily iterate and adjust it:
     - **Natural-Language / Prompt Iteration (`POST /api/songs/iterate` & CLI `--iterate`)**: The user describes what they want changed (e.g., *"Make the guitar solo more fiery with deeper wah, boost the sub-bass, speed up from 76 to 80 BPM, or rewrite the Chorus lyrics"*), and the engine loads the current version (`vN`), applies the Ollama Producer Panel + targeted node re-synthesis, and saves a new version (`v(N+1)`) in the version tree without overwriting `vN`.
     - **Direct Parameter & Section Controls in Web UI**: Sliders/inputs to tweak BPM, Key Signature, Scale/Mode, Vocal Archetype/Rasp, Guitar Solo/Wah intensity, Drum fill complexity, Sub-Bass weight, Tape Warmth, and section-by-section lyrics, with one-click **"Iterate & Adjust Song"** (`vN -> v(N+1)`), **"Regenerate Single Stem"** (`drums`, `bass`, `guitar`, `vocals`, `other`), and **"Re-Master Track"**.
     - **A/B Version Switcher**: Users can switch between `v1`, `v2`, `v3` in the Web UI to compare versions side-by-side and continue iterating from any version.
2. **Generate a Complete Test Song (Plus an Iteration `v1 -> v2`) Upon Completion**:
   - Before finishing Milestone 4 / Milestone 5, run the compiled release/debug binary to generate a complete, playable 48 kHz test song on disk in `d:\harfile\music` (including the 76 BPM 1977 Roots Reggae *"One Love"* Anthem `v1` AND an iterated/adjusted `v2` demonstrating the iteration workflow) so that all `.wav` stems, `final_master.wav`, and the version history are already populated and immediately playable when the Web Studio UI opens in the browser!


## Follow-up — 2026-11-01T03:50:42Z

CRITICAL USER UI DESIGN REQUIREMENT:
The user just specified:
> "the uI must be chatgpt like the onde used in the broser under code fusion simple and functional"
("The UI must be ChatGPT-like, like the one used in the browser under ModelFusion (`d:\harfile\ModelFusion\browser\ui\index.html` and `styles.css`) — simple and functional.")

Please inspect `d:\harfile\ModelFusion\browser\ui\index.html` and `d:\harfile\ModelFusion\browser\ui\styles.css` and model the Web UI in `d:\harfile\music` directly on that clean, simple, functional ChatGPT layout:
1. **ChatGPT 2-Column Layout (`.chatgpt-app`)**:
   - **Left Sidebar (`.chatgpt-sidebar`)**:
     - Brand header (`MusicFusion AI` + sidebar collapse toggle `◫`)
     - `+ New Song / Chat` button (`#sidebar-new-chat`)
     - **Songs & Version History List**: Clickable list of created songs (pre-populated with the generated test song `v1` and iterated `v2`!) so clicking a song loads its conversation and audio players immediately.
     - **Genre & Hit Presets Accordion**: Quick-click presets (e.g., `🇯🇲 1977 Roots Reggae ("One Love" Anthem)`, `🌍 Afrobeats Hit`, `🎹 Amapiano Log-Drum`, `🎸 Stadium Blues-Rock`, `🎤 Soul / Gospel Choir`, `🥁 Hip-Hop / 808 Trap`, `🎷 Jazz Fusion`, `🎻 Cinematic Orchestral`, `⚡ EDM / House`).
     - **Sidebar Footer**: Live Hardware & Ollama status badge (`RAM / VRAM` + active Ollama model).
   - **Main Chat Canvas (`.chatgpt-main`)**:
     - **Minimal Top Header**: Model selector pill (`Auto (Ollama Fusion)`), Genre selector pill (`All Genres / Auto` or 20+ genres), BPM & Key quick pills, and Theme toggle (Clean Light / ChatGPT Slate Dark).
     - **Hero View / Conversation Stream (`.chat-messages`)**:
       - Starts with a clean centered greeting (`"What hit song should we create today?"`) or immediately displays the active song conversation thread.
       - Every generated or iterated song renders as a clean **Interactive Song Card** inside the assistant message bubble containing:
         1. **Master Audio Player**: Prominent `<audio controls>` player for `final_master.wav` + Download WAV button + Version badge (`v1`, `v2`, ...) and Judge Score / Telemetry pills (`Peak dBFS`, `RMS`, `BPM`, `Key`).
         2. **Collapsible Stems & Studio Drawer**: Clean expandable section with `<audio controls>` players for each isolated stem (`raspy_tenor_vocals`, `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, `inpainted_drums`, `draft_groove`) and one-click `Regenerate Stem` & `Re-Master` buttons.
         3. **Collapsible Lyrics & Prompt Sheet**: Clean tabs or accordion for `Lyrics` (editable so the user can tweak lines directly!), `YuE genre.txt / lyrics.txt`, and `6-Node LangGraph Trace`.
     - **Pinned Bottom Floating Input Capsule (`.floating-input-capsule`)**:
       - Just like ChatGPT and `ModelFusion/browser/ui`: a clean rounded capsule at the bottom with a `<textarea>` (`"Describe a new song, or tell me how to adjust/iterate the current song (e.g. 'make the guitar solo heavier, boost sub-bass, change chorus lyrics')..."`), a Genre/Preset pill, an Iterate vs. New Song mode toggle (auto-detects iteration when a song is active!), and the circular `↑` Send button!


## Follow-up — 2026-11-01T04:07:28Z

CRITICAL USER REQUIREMENT UPDATE:
The user just requested:
> "ADD ANOTHER llm THAT CAN ASSIST WITH ITERATION USING FUSION"

Please integrate an additional dedicated **Iteration Assistant / Co-Pilot LLM** into the ModelFusion panel and workflow (`crates/music_fusion`, `crates/music_pipeline`, and `crates/music_cli` + ChatGPT Web UI):

1. **Dedicated Iteration Specialist LLM in the ModelFusion Panel (`crates/music_fusion`)**:
   - Notice the rich set of local Ollama models installed on this workstation: `qwen2.5:32b`, `deepseek-r1:32b`, `glm-4.7-flash:latest`, `glm4:9b`, `flint:latest`, `gemma2:9b`, `mistral:latest`, `qwen2.5:7b`, `strix-qwen:latest`, `deepseek-r1:1.5b`, `qwen2.5:1.5b`.
   - Expand the ModelFusion panel with a dedicated **`IterationAssistant` / `IterationCoPilot` LLM persona** (selecting an distinct installed reasoning/creative model from Ollama, e.g., pairing `deepseek-r1` / `glm-4.7-flash:latest` / `gemma2:9b` / `flint:latest` with `qwen2.5` and `glm4:9b` so multiple distinct LLMs deliberate together during iteration!).
   - Implement a **Multi-LLM Iteration Fusion Pipeline (`run_iteration_fusion` / `assist_song_iteration`)** where:
     - **Model 1 (Lyric & Hook Refiner LLM)**: Refines lyrical imagery, cadence, and emotional hook based on user feedback.
     - **Model 2 (Arrangement & Groove Producer LLM)**: Adjusts BPM, key/mode, chord progression, drum complexity, and instrument solo intensity.
     - **Model 3 (Dedicated Iteration Co-Pilot & Diff Architect LLM — the newly added LLM)**: Inspects the previous version (`vN`) state + `StemTelemetry` + user critique, identifies exact delta improvements across stems (`vocals`, `drums`, `bass`, `guitar`, `other`), and proposes targeted parameter/stem adjustments and next-step creative suggestions.
     - **Judge / Synthesizer LLM**: Fuses the outputs of all panel LLMs into the upgraded `v(N+1)` `MusicProductionState` plus a structured `iteration_summary` and `suggested_next_iterations: Vec<String>`.

2. **Expose the Iteration Fusion Assistant in REST API, CLI, MCP, and ChatGPT Web UI**:
   - **REST API**:
     - In `POST /api/songs/iterate`: Run the Multi-LLM Iteration Fusion Panel (including the new Iteration Assistant LLM) to produce `v(N+1)`, returning both the new version state/audio AND the Iteration Assistant LLM's critique, diff summary, and clickable follow-up iteration suggestions.
     - Add `POST /api/songs/assist-iteration` (or `GET /api/songs/:id/assist`): Invokes the Iteration Fusion LLM panel on the current song version to critique the track and return 3–5 concrete, clickable iteration suggestions (e.g., *"🔥 Boost guitar solo wah & sustain in [Guitar Solo]"*, *"🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]"*, *"🎤 Elevate gospel choir call-and-response harmonies in [Chorus]"*).
   - **ChatGPT Web UI (`webui/`)**:
     - Show the active **Multi-LLM Iteration Fusion Panel** models in the UI (e.g. `Lyricist + Arranger + Sound Engineer + Iteration Co-Pilot LLM + Judge`).
     - Include a **"✨ Ask Fusion LLM to Assist / Critique & Suggest Iterations"** button on every Song Card and in the bottom ChatGPT input bar that calls the Iteration Fusion Assistant and renders clickable suggestion chips that the user can click to immediately apply and generate `v(N+1)`!
