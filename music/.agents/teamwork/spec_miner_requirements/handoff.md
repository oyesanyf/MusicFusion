# Handoff Report: Specification Mining & Formal Requirements Matrix

**Working Directory**: `d:\harfile\music\.agents\teamwork\spec_miner_requirements`  
**Author**: Spec Miner Subagent (`spec_miner_requirements`)  
**Parent Orchestrator ID**: `4caf3f77-b3ac-4b67-8bda-934a91faafab`  
**Timestamp**: 2026-11-01T01:05:00Z  
**Type**: Hard Handoff (Specification Phase Complete)

---

## 1. Observation

Direct observations extracted from authoritative repository artifacts:

1. **User Requirements (`d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md`)**:
   - Lines 5: *"Refactor and expand `d:\harfile\music` so that the entire AI Music Producer Agent, the LangGraph `StateGraph` orchestration engine, the adapted ModelFusion hardware/LLM fusion modules, and the 5-stage sequential music pipeline live 100% inside the standalone `d:\harfile\music` Rust workspace. Do not create a separate Python LangGraph project and do not link against `d:\harfile\ModelFusion` at runtime."*
   - Lines 10–11: Port `SystemMemory::detect_live()` from `ModelFusion` (`crates/model_selection/src/memory.rs`) for live RAM and VRAM probing. Adapt `fusion.rs` and `judge.rs` using `reqwest` + `serde_json` against `http://127.0.0.1:11434/api/tags` and `/api/chat`.
   - Lines 16–32: Exact state fields specified for `MusicProductionState`: `song_id`, `version`, `engine_mode`, `style_prompt`, `lyrics`, `bpm`, `key_signature`, `vocal_ref_wav`, `master_ref_wav`, `stem_paths`, `telemetry`, `judge_score`, `quality_passed`, `retry_count`.
   - Lines 34–42: 6 Sequential Nodes:
     * Node 1: `ollama_producer_panel_node` (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`)
     * Node 2: `groove_and_yue_synthesis_node` (`ACE-Step` -> `draft_groove.wav`; `YuE` -> `yue_vocals.wav` + `yue_instrumentals.wav`)
     * Node 3: `jamaican_tenor_timbre_node` (`Seed-VC` -> `raspy_tenor_vocals.wav`)
     * Node 4: `virtuoso_stem_inpainting_node` (`htdemucs_ft` 4 stems -> `stable-audio-open-1.0` inpainting 16-bar interlude, solo, nyabinghi, rimshot -> sample-accurate crossfade at 76 BPM)
     * Node 5: `dsp_and_judge_gate_node` (telemetry inspection, zero clipping, 76 BPM alignment, Judge evaluation, conditional retry logic: `quality_passed == false && retry_count < 2` routes back to Node 4 or Node 2, else Node 6)
     * Node 6: `analog_mastering_node` (`AudioSR` 48 kHz + `Matchering 2.0` or pure-Rust 48 kHz tape saturation/sub-bass/EQ -> `final_master.wav`)
   - Lines 46–48: External interfaces: Tokio HTTP JSON REST API on port 5050, MCP JSON-RPC 2.0 stdio, and CLI preset `--create-reggae-anthem`.
   - Lines 53–67 (Critical Follow-up): *"this can be any type of Music not just reggae it can be all types of music known to man"* -> Universal Multi-Genre AI Music Production Engine supporting ALL genres known to humanity (Afrobeats, Highlife, Amapiano, Reggae, Soul/R&B, Gospel, Hip-Hop/Trap, Blues, Rock, Metal, Jazz, Classical, Pop, EDM, Latin, etc.) with dynamic genre taxonomy, BPM (40–220), scales/modes, and instruments, while keeping `--create-reggae-anthem` as a flagship preset.

2. **Existing Workspace State (`d:\harfile\music\Cargo.toml` & `crates/music_core/`)**:
   - `Cargo.toml`: Root workspace defines 5 member crates:
     `members = ["crates/music_core", "crates/music_fusion", "crates/music_dsp", "crates/music_pipeline", "crates/music_cli"]`.
   - `crates/music_core/src/schema.rs`: Lines 4–48 define `PipelineModelStack`; lines 51–90 define `StemMixConfig`; lines 93–122 define `SongSection`; lines 125–466 define `SongBlueprint` with flagship presets `default_one_love_shining_anthem()` and `default_rise_up_together_anthem()`.
   - Currently, `MusicProductionState`, `StemTelemetry`, and the StateGraph engine are not yet implemented in `crates/music_core`.

3. **Reference Implementations Probed**:
   - `d:\harfile\ModelFusion\crates\model_selection\src\memory.rs`: `SystemMemory::detect_live()` queries `sysinfo::System` for RAM/CPU and executes hidden `nvidia-smi` to parse GPU name, total VRAM, and free VRAM in MB/GB. Dynamic memory rules: Ollama model sizing, VRAM budget calculation (`gpu_vram_free_gb * 0.85`), RAM budget calculation (`(free_ram_gb - 3.0) * 0.70`).
   - `d:\harfile\ModelFusion\crates\core\src\fusion_engine\fusion.rs` & `judge.rs`: Batched execution, JSON extraction stripping `<think>...</think>` and markdown fences, fallback mechanisms when models error out.
   - `d:\harfile\killswitch\crates\killswitch-orchestrator\src\graph.rs`: Proven pure-Rust cyclic StateGraph builder pattern (`StateGraphBuilder`, `CompiledStateGraph`, `LifecycleNode` trait, `add_node`, `add_edge`, `add_conditional_edge`, `step`, BLAKE3 input/output hashing, version checkpointer).

---

## 2. Logic Chain

1. **Independence Invariant**: Because `ORIGINAL_REQUEST.md` mandates zero Python LangGraph and zero runtime link to `d:\harfile\ModelFusion`, all memory probing, Ollama JSON-RPC communication, state graph orchestration, audio DSP, and CLI/REST/MCP layers must be natively compiled inside `d:\harfile\music`'s 5 member crates.
2. **Deterministic VRAM Freeing Invariant**: Hugging Face pipelines (`ACE-Step`, `YuE`, `Seed-VC`, `HTDemucs`, `Stable Audio Open`, `AudioSR`) require significant GPU VRAM (4–16 GB). If loaded in-process into a single runtime, PyTorch CUDA memory caching prevents VRAM from returning to the OS. Spawning external CLI processes (`tokio::process::Command`) and awaiting process termination guarantees that Windows WDDM / CUDA drivers immediately reclaim 100% of GPU VRAM between stages.
3. **Pure-Rust DSP Fallback Invariant**: When GPU VRAM is below required thresholds (< 4 GB) or when external models are absent or when `engine_mode == "pure_rust"`, each node must route execution to a native 48 kHz pure-Rust DSP engine:
   - Node 2: 76 BPM reggae / genre rhythm drum generator and FM/additive synth.
   - Node 3: Formant filter, rasp harmonic generator, and pitch-correction vocoder.
   - Node 4: Physical Karplus-Strong string guitar modeler, resonant wah-filter envelope, and polyrhythmic drum synth.
   - Node 6: 48 kHz analog tape saturation (tanh curve), 55–85 Hz sub-bass warmth, and reference spectral EQ matching.
4. **Universal Multi-Genre Taxonomy Invariant**: Supporting all genres requires:
   - Dynamic prompt generation in Node 1 matching the requested genre, tempo (40–220 BPM), time signature, and modal scale.
   - Dynamic drum pattern synthesis in Node 2 (One-Drop, Four-on-the-Floor, Boom-Bap, Trap 808, Amapiano log-drum, Rock/Metal double-kick, Jazz swing, Orchestral).
   - Dynamic vocal timbre conversion in Node 3 (Raspy Jamaican Tenor, Smooth R&B, Operatic Choral, Gritty Rock, Rap cadence, Acoustic Crooner).
   - Dynamic lead/harmony inpainting in Node 4 (Electric Guitar/Wah, Acoustic Nylon, Grand Piano/Hammond Organ, Synths, Strings, Brass, 808 Bass).
   - Dynamic mastering profiles in Node 6 (1970s Warm Tape, Modern Punchy Pop, Club Sub-Heavy, Cinematic Orchestral, Acoustic Audiophile).
   - Preservation of `--create-reggae-anthem` as the zero-configuration 1977 Roots Reggae flagship preset.

---

## 3. Caveats

1. **External Model Availability**: When local Python or Hugging Face model weights (`YuE`, `ACE-Step`, `Seed-VC`, `HTDemucs`, `AudioSR`) are not installed on the host machine, the system must transparently fall back to the built-in pure-Rust DSP synthesis engine rather than aborting or erroring.
2. **Ollama Daemon State**: Node 1 and Node 5 rely on `http://127.0.0.1:11434`. If Ollama is offline or uninstalled, the system must synthesize valid fallback genre tags, bracketed lyrics, and arrangement maps from built-in template matrices (such as `SongBlueprint::default_one_love_shining_anthem()`).
3. **Audio Playback**: The engine strictly generates 48 kHz 24-bit / 32-bit float `.wav` audio files on disk and streams buffer slices over REST/MCP; it does not require direct access to Windows WASAPI hardware audio endpoints during headless test execution.

---

## 4. Conclusion & Formal Specifications

### 4.1 Crate Architecture Layout

The 5 workspace member crates in `d:\harfile\music` map to the specification as follows:

| Crate | Responsibility | Key Modules |
|---|---|---|
| `crates/music_core` | Core domain types, state schemas, version store, and configuration | `schema.rs`, `state.rs`, `telemetry.rs`, `genre.rs`, `memory.rs` |
| `crates/music_fusion` | Adapted ModelFusion multi-model Ollama panel, Judge, and memory sizing | `panel.rs`, `judge.rs`, `ollama_client.rs`, `prompt_templates.rs` |
| `crates/music_dsp` | 48 kHz pure-Rust DSP synthesis, physical modeling, inpainting, and mastering | `groove_synth.rs`, `vocal_formant.rs`, `guitar_wah.rs`, `tape_mastering.rs`, `telemetry_calc.rs` |
| `crates/music_pipeline` | Native Rust LangGraph `StateGraph` engine, 6 sequential nodes, and version checkpointer | `graph.rs`, `checkpointer.rs`, `nodes/` (nodes 1–6), `runner.rs` |
| `crates/music_cli` | Tokio HTTP REST API (port 5050), MCP JSON-RPC 2.0 stdio server, and CLI commands | `main.rs`, `rest_server.rs`, `mcp_server.rs`, `cli_args.rs`, `presets.rs` |

---

### 4.2 Exact Struct Specification: `MusicProductionState`

Location: `crates/music_core/src/state.rs`

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Audio analysis telemetry for stems and master mix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StemTelemetry {
    /// Peak amplitude in dBFS (e.g. -0.5 dBFS; must not exceed 0.0)
    pub peak_dbfs: f32,
    /// Root Mean Square level in dBFS (e.g. -14.2 dBFS)
    pub rms_dbfs: f32,
    /// Dynamic range in dB (peak minus minimum perceived RMS)
    pub dynamic_range_db: f32,
    /// Phase/tempo alignment drift relative to master BPM (0.0 = perfect lock, > 1.0 = drift)
    pub tempo_drift_score: f32,
    /// Boolean flag: true if any sample >= 1.0 or <= -1.0 (clipping detected)
    pub clipping: bool,
    /// Signal-to-Noise ratio in dB
    pub snr_db: f32,
    /// Spectral flatness measure (0.0 = pure tones, 1.0 = white noise)
    pub spectral_flatness: f32,
}

impl Default for StemTelemetry {
    fn default() -> Self {
        Self {
            peak_dbfs: -1.0,
            rms_dbfs: -14.0,
            dynamic_range_db: 12.0,
            tempo_drift_score: 0.0,
            clipping: false,
            snr_db: 65.0,
            spectral_flatness: 0.15,
        }
    }
}

/// The 13+ field core state container passed through the StateGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicProductionState {
    /// 1. Unique song identifier (UUID v4 or slug)
    pub song_id: String,
    /// 2. Monotonically increasing version counter (1, 2, 3...)
    pub version: u32,
    /// 3. Execution mode: "auto", "pure_rust", or "hf_hybrid"
    pub engine_mode: String,
    /// 4. Primary genre identifier (e.g. "reggae", "afrobeats", "rock", "jazz", etc.)
    pub genre: String,
    /// 5. Full stylistic prompt describing instrumentation, mood, and production
    pub style_prompt: String,
    /// 6. Complete song lyrics with bracketed structural tags ([Intro], [Verse 1], etc.)
    pub lyrics: String,
    /// 7. Tempo in Beats Per Minute (40 to 220, default 76 for reggae anthem)
    pub bpm: u32,
    /// 8. Musical key signature (e.g. "A Major", "G Minor", "D Dorian")
    pub key_signature: String,
    /// 9. Time signature (e.g. "4/4", "3/4", "6/8", "12/8", "5/4", "7/8")
    pub time_signature: String,
    /// 10. Optional reference audio file for zero-shot vocal timbre conditioning
    pub vocal_ref_wav: Option<PathBuf>,
    /// 11. Optional reference audio file for Stage 5 Matchering EQ/master profile
    pub master_ref_wav: Option<PathBuf>,
    /// 12. Isolated audio stem filepaths:
    /// Keys: "draft_groove", "yue_vocals", "yue_instrumentals", "raspy_tenor_vocals",
    ///       "drums", "bass", "guitar", "other", "inpainted_guitar", "inpainted_drums",
    ///       and "final_master"
    pub stem_paths: HashMap<String, PathBuf>,
    /// 13. Telemetry inspection results (peak dBFS, RMS, dynamic range, tempo drift, clipping)
    pub telemetry: StemTelemetry,
    /// 14. Judge model score (0.0 to 10.0; >= 7.5 indicates broadcast quality)
    pub judge_score: f32,
    /// 15. Quality gate boolean flag (true = passes gate, false = trigger retry)
    pub quality_passed: bool,
    /// 16. Number of retries executed in current generation cycle (capped at 2)
    pub retry_count: u8,
}
```

---

### 4.3 Native Rust StateGraph Engine & Semantics

Location: `crates/music_pipeline/src/graph.rs`

- **Execution Model**: Async tokio DAG / Pregel execution loop over `MusicProductionState`.
- **Node Trait**:
  ```rust
  #[async_trait::async_trait]
  pub trait MusicPipelineNode: Send + Sync {
      fn stage_name(&self) -> &'static str;
      async fn execute(&self, state: &mut MusicProductionState) -> anyhow::Result<()>;
  }
  ```
- **StateGraph Interface**:
  - `add_node(stage: LifecycleStage, node: Arc<dyn MusicPipelineNode>) -> Self`
  - `add_edge(from: LifecycleStage, to: LifecycleStage) -> Self`
  - `add_conditional_edges(from: LifecycleStage, router: Arc<dyn Fn(&MusicProductionState) -> LifecycleStage + Send + Sync>) -> Self`
  - `compile(self) -> CompiledStateGraph`
- **Execution Step Semantics**:
  1. Record input state hash (BLAKE3 / SHA-256).
  2. Inspect `SystemMemory::detect_live()`.
  3. Execute `node.execute(&mut state).await`.
  4. Save checkpoint to version store (`.music/versions/<song_id>/v<version>.json` and SQLite database).
  5. Evaluate conditional edge or advance along direct edge.
- **Conditional Routing Rule (Node 5 Gate)**:
  ```rust
  let router = |state: &MusicProductionState| -> LifecycleStage {
      if state.quality_passed {
          LifecycleStage::AnalogMastering // Node 6
      } else if state.retry_count < 2 {
          // If vocal timing drift occurred, re-synthesize groove & YuE (Node 2)
          if state.telemetry.tempo_drift_score > 0.5 {
              LifecycleStage::GrooveAndYueSynthesis // Node 2
          } else {
              LifecycleStage::VirtuosoStemInpainting // Node 4
          }
      } else {
          // Retry exhausted: proceed to mastering with best-effort warning
          LifecycleStage::AnalogMastering // Node 6
      }
  };
  ```

---

### 4.4 The 6 Sequential Nodes Specification

#### Node 1: `ollama_producer_panel_node` (Stage 0)
- **Role**: Multi-agent songwriting, theory, and arrangement panel.
- **Inputs**: `style_prompt`, `genre`, `bpm`, `key_signature`, `time_signature`, `lyrics`.
- **Panel Personas**:
  1. *Lyricist*: Verifies thematic depth (e.g. human compassion, spiritual unity) and formats strictly into clean bracketed structural tags: `[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`.
  2. *Genre Theorist / Arranger*: Assigns bar counts per section (e.g. 4-bar intro, 8-bar verse), chord progressions, and dynamic markings suited to the genre and scale/mode.
  3. *Lead Instrumentalist / Sound Engineer*: Formulates acoustic production cues and YuE `genre.txt` tags.
  4. *Judge Model*: Synthesizes final cohesive arrangement map.
- **Outputs**: Generates clean YuE `genre.txt`, bracketed `lyrics.txt`, and section arrangement metadata stored in `MusicProductionState`.
- **Fallback**: If Ollama endpoint is unavailable, loads genre templates from `SongBlueprint` presets.

#### Node 2: `groove_and_yue_synthesis_node` (Stages 1 & 2)
- **Role**: Rhythmic groove generation and dual-stem vocal/instrumental audio generation.
- **Inputs**: Arrangement map, BPM, key, genre rhythm style.
- **Stage 1 (Groove Foundation)**:
  - External mode: `ACE-Step/ACE-Step-v1.5` via `tokio::process::Command` waiting for process termination -> `draft_groove.wav`.
  - Pure-Rust fallback: 48 kHz multi-pattern rhythm generator producing:
    * Reggae: 76 BPM one-drop kick/snare on beat 3 with rimshot and syncopated hi-hat.
    * Afrobeats / Amapiano: 108–114 BPM syncopated log-drum, clave, and shaker groove.
    * House / EDM: 124–128 BPM four-on-the-floor kick with open offbeat hi-hat.
    * Hip-Hop / Trap: 70–90 / 140–160 BPM rolling hi-hats, hard snare on 3, and deep 808 subs.
    * Rock / Metal: 120–170 BPM acoustic/double-kick driving backbeat.
    * Jazz / Fusion: 110–140 BPM swung ride cymbal with feather kick and ghost notes.
- **Stage 2 (YuE In-Context Learning Synthesis)**:
  - External mode: `m-a-p/YuE-s1-7B-anneal-en-icl` + `m-a-p/YuE-s2-1B-general` conditioned on `draft_groove.wav` via CLI process worker -> outputs `yue_vocals.wav` and `yue_instrumentals.wav`.
  - Pure-Rust fallback: Multi-track harmonic synthesizer generating guide vocal sine/formant track and rich rhythm skank / bass / organ chordal accompaniment.
- **VRAM Guarantee**: Process terminates before returning -> 100% VRAM freed.

#### Node 3: `vocal_timbre_fusion_node` (formerly `jamaican_tenor_timbre_node`, Stage 3)
- **Role**: Zero-shot vocal timbre conversion and formant molding.
- **Inputs**: `yue_vocals.wav`, optional `vocal_ref_wav`, vocal archetype selector.
- **External Mode**: `Plachtaa/Seed-VC` with `f0_condition=True` executed as isolated CLI subprocess -> outputs `raspy_tenor_vocals.wav` (or specified archetype).
- **Pure-Rust Fallback**:
  - Linear Predictive Coding (LPC) formant filter molding vocal tract resonance.
  - Quadratic harmonic distortion generator for authentic vocal cord rasp.
  - Pitch-lock vocoder with selectable formant shifting.
- **Vocal Archetypes Supported**:
  * 1977 Soulful Raspy Jamaican Male Tenor (default reggae anthem)
  * Smooth Contemporary R&B / Pop Soprano & Alto
  * Classical / Operatic Choral Vocalist
  * Gritty Classic Rock Baritone
  * Rhythmic Hip-Hop Cadence Delivery
  * Warm Intimate Acoustic Folk Crooner

#### Node 4: `virtuoso_stem_inpainting_node` (Stage 4)
- **Role**: 4-stem instrumental separation and virtuosic solo/fill audio inpainting.
- **Inputs**: `yue_instrumentals.wav`, section layout (`[Instrumental Interlude]`, `[Guitar Solo]`).
- **Stem Separation**:
  - External mode: `facebook/htdemucs_ft` splits instrumental into `drums.wav`, `bass.wav`, `guitar.wav`, `other.wav`.
  - Pure-Rust fallback: Multiband frequency crossover & stereo mid-side matrix separator.
- **Solo & Fill Inpainting**:
  - External mode: `stabilityai/stable-audio-open-1.0` or `facebook/musicgen-style` inpainting 16-bar interlude, stadium blues-rock guitar solos, and polyrhythmic drum fills.
  - Pure-Rust fallback:
    * Physical Karplus-Strong string modeling with non-linear overdrive and CryBaby wah-pedal bandpass envelope.
    * Polyrhythmic hand-drum (Nyabinghi / Amapiano log-drum / Latin conga) synthesizer.
- **Crossfading**: Sample-accurate equal-power crossfade ($y[n] = \sqrt{1-t}\cdot x_1[n] + \sqrt{t}\cdot x_2[n]$) aligned strictly to bar boundaries calculated from BPM ($T_{\text{bar}} = \frac{4 \times 60}{\text{BPM}}$ seconds).

#### Node 5: `dsp_and_judge_gate_node`
- **Role**: Objective audio telemetry inspection and AI musicality evaluation.
- **Inputs**: All stem files (`drums`, `bass`, `guitar`, `other`, `vocals`, `inpainted_guitar`, `inpainted_drums`).
- **Telemetry Inspection (`StemTelemetry`)**:
  - Peak dBFS: $20 \log_{10}(\max |s|) \le -0.5 \text{ dBFS}$.
  - RMS level: $\sqrt{\frac{1}{N}\sum s[n]^2} \approx -14.0 \text{ dBFS}$.
  - Clipping check: assert zero samples reach $\pm 1.0$.
  - Phase and tempo alignment: Cross-correlation against master BPM grid. Tempo drift score must be $\le 0.3$.
  - Headroom: Vocals must sit $+1.5 \text{ dB}$ above instrumental bed without masking.
- **Judge Evaluation**:
  - Calls Ollama Judge (`qwen2.5:7b` / `mistral`) to score arrangement cohesion, lyrical tags match, and audio balance (score 0.0 to 10.0).
- **Gate Evaluation**:
  - `quality_passed = (clipping == false) && (peak_dbfs <= 0.0) && (tempo_drift_score <= 0.3) && (judge_score >= 7.5)`.
  - If failed and `retry_count < 2`: increments `retry_count`, logs reason, routes to Node 2 (vocal drift) or Node 4 (stem solo).
  - Else: proceeds to Node 6.

#### Node 6: `analog_mastering_node` (Stage 5)
- **Role**: 48 kHz upsampling, analog tape warmth, sub-bass enhancement, and reference mastering.
- **Inputs**: Stem mix, optional `master_ref_wav`, target loudness LUFS (-14.0 LUFS).
- **External Mode**: `haoheliu/versatile_audio_super_resolution` (AudioSR) 48 kHz + `Matchering 2.0` (matching frequency curve of 1977 Bob Marley / Island Records tape masters).
- **Pure-Rust Fallback**:
  - Studer A800 2-inch tape simulation: hyperbolic tangent transfer function $f(x) = \tanh(1.25 x) + 0.08 x^2 - 0.03 x^3$.
  - Sub-bass resonant shelf filter (+3.2 dB boost at 55–85 Hz with 24 dB/octave high-pass rumble filter at 28 Hz).
  - Stereo Blumlein spatial widener (1.22x width).
  - Brickwall Lookahead Limiter with 2 ms attack and 50 ms release to guarantee -0.3 dBFS true peak.
- **Output**: `final_master.wav` (48 kHz 24-bit PCM WAV) committed to version tree.

---

### 4.5 External Interfaces Specification

#### 1. Tokio HTTP REST API (Default Port: 5050)
Server: `crates/music_cli/src/rest_server.rs`

| Method | Endpoint | Request Body | Response Body | Description |
|---|---|---|---|---|
| `POST` | `/api/songs/create` | `{ "genre": string, "prompt": string, "lyrics": string?, "bpm": u32?, "key": string?, "engine_mode": string? }` | `{ "song_id": string, "version": u32, "status": "started" }` | Bootstraps a new song and launches the full StateGraph |
| `POST` | `/api/songs/iterate` | `{ "song_id": string, "instructions": string, "regenerate_sections": string[]? }` | `{ "song_id": string, "version": u32, "status": "started" }` | Advances version counter and re-runs relevant nodes |
| `POST` | `/api/songs/regenerate-stem` | `{ "song_id": string, "stem_name": string, "prompt": string? }` | `{ "song_id": string, "stem_path": string, "status": "completed" }` | Runs isolated Node 4 execution for specific stem |
| `POST` | `/api/songs/master` | `{ "song_id": string, "target_lufs": f32?, "tape_warmth": f32?, "ref_wav": string? }` | `{ "song_id": string, "master_wav": string, "telemetry": StemTelemetry }` | Re-masters existing stems through Node 6 |
| `GET` | `/api/songs/:id/versions` | *None* | `[ { "version": u32, "parent_version": u32?, "created_at": string, "master_wav": string, "telemetry": StemTelemetry } ]` | Lists version history and snapshots for a song |
| `GET` | `/api/songs/:id/stream` | *None* (SSE headers) | `text/event-stream` chunks: `data: { "node": string, "status": string, "progress": f32 }` | Server-Sent Events stream of real-time node transitions |

#### 2. MCP JSON-RPC 2.0 Interface (`--mcp`)
Server: `crates/music_cli/src/mcp_server.rs`

| MCP Tool Name | Description | Required Parameters | Optional Parameters |
|---|---|---|---|
| `music_create_hit` | Generates a complete song from scratch using the 6-stage StateGraph | `genre: string`, `prompt: string` | `lyrics: string`, `bpm: number`, `key: string`, `engine_mode: string` |
| `music_iterate_song` | Creates a new song version by modifying lyrics, arrangement, or mix | `song_id: string`, `instructions: string` | `bpm: number`, `key: string` |
| `music_regenerate_stem` | Re-synthesizes a specific stem (e.g. `guitar`, `drums`, `bass`) | `song_id: string`, `stem_name: string` | `solo_intensity: number`, `wah_depth: number` |
| `music_master_track` | Applies 48 kHz analog tape mastering and EQ Matchering to stems | `song_id: string` | `target_lufs: number`, `tape_warmth: number`, `ref_wav: string` |

#### 3. Command Line Interface (CLI)
Binary: `crates/music_cli/src/main.rs`

- `--create-reggae-anthem`:
  Boots the standalone Rust StateGraph pre-loaded with the 76 BPM "One Love" compassion lyrics and style prompt (`"1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, 76 BPM, hypnotic one drop master drumming with complex polyrhythmic snare rolls and deep sub bass, virtuosic soaring psychedelic blues rock electric guitar solos, warm analog tape production, lush gospel backing choir"`). Runs end-to-end to produce `final_master.wav`.
- `--create-hit --genre "<genre>" --prompt "<prompt>" [--bpm <bpm>] [--key <key>]`:
  Universal multi-genre song production command.
- `--serve --port <port>`:
  Starts the Tokio HTTP REST API (default port 5050).
- `--mcp`:
  Starts the Model Context Protocol stdio JSON-RPC server.
- `--pure-rust`:
  Forces the pure-Rust 48 kHz DSP fallback engine across all stages.

---

### 4.6 Verification & Test Matrix (All 4 Tiers)

#### Tier 1: Unit Tests
- **Scope**: DSP algorithms, math, data serialization, and audio buffer transformations.
- **Tests**:
  1. `test_system_memory_sizing`: Validates RAM/VRAM budget math and Ollama model selection logic.
  2. `test_bracketed_tag_parser`: Validates parsing and enforcement of clean structural tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`).
  3. `test_yue_genre_and_lyrics_export`: Verifies `SongBlueprint::to_yue_genre_txt()` and `to_yue_lyrics_txt()` output exact format required by YuE.
  4. `test_pure_rust_groove_76bpm`: Asserts generated 76 BPM reggae groove WAV has correct duration, sample rate (48,000 Hz), zero clipping, and peak at beat 3.
  5. `test_equal_power_crossfader`: Tests crossfading two buffers at bar boundary with $\le 0.01 \text{ dB}$ power deviation.
  6. `test_stem_telemetry_clipping_detection`: Verifies `StemTelemetry` flags clipping when samples exceed $\pm 1.0$.
  7. `test_analog_tape_saturation`: Verifies tanh transfer function adds warm odd harmonics and bounds output to $[-1.0, 1.0]$.

#### Tier 2: Integration Tests
- **Scope**: Native StateGraph engine transitions, conditional routing, and version checkpointing.
- **Tests**:
  1. `test_stategraph_direct_edges`: Executes sequential nodes and asserts state passes through nodes 1 -> 2 -> 3 -> 4 -> 5 -> 6.
  2. `test_stategraph_conditional_retry_logic`: Mocks `quality_passed = false` with `retry_count = 0` -> asserts graph re-executes Node 4; when `retry_count` reaches 2 -> asserts graph advances to Node 6.
  3. `test_durable_version_checkpointing`: Verifies saving and restoring `MusicProductionState` snapshots from disk preserves all 13+ fields.
  4. `test_ollama_panel_mock_response`: Mocks Ollama HTTP `/api/chat` and validates extraction of genre tags, lyrics, and judge score.

#### Tier 3: Hardware & VRAM Allocation Tests
- **Scope**: Live hardware detection, out-of-process isolation, and VRAM reclamation.
- **Tests**:
  1. `test_live_memory_detection`: Calls `SystemMemory::detect_live()` and asserts positive total RAM, physical CPU cores $\ge 1$, and valid VRAM numbers.
  2. `test_process_termination_vram_cleanup`: Spawns mock/real worker subprocess with memory allocation, waits for `.wait()`, and asserts child process exits with code 0 and releases handles.
  3. `test_dynamic_engine_mode_selection`: Verifies that when detected VRAM $< 4.0 \text{ GB}$, `engine_mode` automatically resolves to `"pure_rust"`.

#### Tier 4: End-to-End Pipeline Tests
- **Scope**: Complete CLI, REST API, and MCP tool invocations.
- **Tests**:
  1. `test_cli_create_reggae_anthem_pure_rust`: Runs `music_cli --create-reggae-anthem --pure-rust`, asserts generation of 48 kHz `final_master.wav` and version entry.
  2. `test_cli_create_hit_universal_genre`: Runs `music_cli --create-hit --genre "Afrobeats" --prompt "Lagos midnight energy" --pure-rust`, asserts completed master WAV with Afrobeats 108 BPM rhythm.
  3. `test_rest_api_song_lifecycle`: Spawns REST server on test port, executes `POST /api/songs/create`, polls `/api/songs/:id/versions`, and requests `/api/songs/regenerate-stem`.
  4. `test_mcp_json_rpc_create_and_master`: Sends JSON-RPC `music_create_hit` and `music_master_track` over stdio, asserts valid JSON-RPC 2.0 response format.

---

## 5. Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|---|---|---|---|---|---|---|
| 1 | Architectural Invariants | 100% Standalone Rust Workspace | Zero Python LangGraph, zero runtime ModelFusion link | CLI / REST / MCP calls | Multi-crate Rust binaries | Build fails if foreign dependencies present | `ORIGINAL_REQUEST.md:5` |
| 2 | Architectural Invariants | Deterministic VRAM Freeing | External Hugging Face stages run in child process and terminate | Node execution trigger | Subprocess PID & exit status | Kill process on timeout / error | `ORIGINAL_REQUEST.md:34` |
| 3 | Architectural Invariants | Pure-Rust 48 kHz DSP Fallback | Synthesizes groove, vocal formants, guitar solos, and tape mastering natively | `engine_mode = "pure_rust"` or VRAM < 4 GB | 48 kHz WAV files | Returns `Result<()>` with DSP error | `ORIGINAL_REQUEST.md:10,34` |
| 4 | Universal Multi-Genre | Dynamic Genre & Style Taxonomy | Supports all genres known to humanity (Afrobeats, Reggae, Rock, Jazz, etc.) | `--genre`, `--bpm`, `--key` | Tailored prompts, rhythms, and mix profiles | Falls back to generic style mapping | `ORIGINAL_REQUEST.md:58-67` |
| 5 | State Specification | `MusicProductionState` Struct | 13+ field state container tracking version, prompt, lyrics, stems, telemetry | Serialized state JSON | Updated state snapshot | Deserialization error logged | `ORIGINAL_REQUEST.md:16-32` |
| 6 | State Specification | `StemTelemetry` Struct | Audio metrics: peak dBFS, RMS, dynamic range, tempo drift, clipping | PCM audio buffer | Telemetry struct | Flags clipping if sample >= 1.0 | `ORIGINAL_REQUEST.md:28` |
| 7 | StateGraph Engine | Native Tokio StateGraph Engine | Async Pregel DAG engine with `add_node`, `add_edge`, `add_conditional_edges` | Node instances & state | Compiled runnable StateGraph | GraphHalted on missing node | `killswitch/engine.rs` & `ORIGINAL_REQUEST.md:13` |
| 8 | StateGraph Engine | Version Checkpointer | Durable checkpointing of state per version iteration | State snapshot | SQLite / JSON version file | Rollback on write failure | `ORIGINAL_REQUEST.md:14` |
| 9 | Pipeline Nodes | Node 1: Producer Panel | Multi-agent Ollama panel (Lyricist, Theorist, Engineer, Judge) | Style prompt & lyrics | Bracketed tags, `genre.txt`, `lyrics.txt` | Fallback to built-in genre blueprint | `ORIGINAL_REQUEST.md:36,62` |
| 10 | Pipeline Nodes | Node 2: Groove & YuE Synthesis | Stage 1 ACE-Step / pure-Rust groove + Stage 2 YuE dual-stem synthesis | Draft groove conditioning | `draft_groove.wav`, `yue_vocals`, `yue_inst` | Fallback to pure-Rust synth | `ORIGINAL_REQUEST.md:37,63` |
| 11 | Pipeline Nodes | Node 3: Vocal Timbre Fusion | Stage 3 Seed-VC / pure-Rust formant/rasp vocoder for genre vocal archetypes | Vocals WAV + reference | `raspy_tenor_vocals.wav` | Fallback to pure-Rust formant/rasp | `ORIGINAL_REQUEST.md:38,64` |
| 12 | Pipeline Nodes | Node 4: Virtuoso Stem Inpainting | Stage 4 HTDemucs 4-stem split + Stable Audio Open solo inpainting & crossfade | Stems + section layout | Inpainted solos/fills with crossfades | Fallback to physical string/drum DSP | `ORIGINAL_REQUEST.md:39,65` |
| 13 | Pipeline Nodes | Node 5: DSP & Judge Gate | Telemetry inspection + Ollama Judge evaluation with conditional retry edge | All stem audio files | `quality_passed`, `judge_score` | Routes to Node 4 or 2 if retry < 2 | `ORIGINAL_REQUEST.md:40` |
| 14 | Pipeline Nodes | Node 6: Analog Mastering | Stage 5 AudioSR 48 kHz upsampling + Matchering 2.0 / pure-Rust tape saturation | Mix stems + reference | 48 kHz `final_master.wav` | Fallback to pure-Rust tape mastering | `ORIGINAL_REQUEST.md:41,66` |
| 15 | External Interfaces | Tokio HTTP REST API | REST endpoints on port 5050 for song creation, iteration, stem regen, mastering | HTTP JSON requests | JSON responses + SSE stream | Standard HTTP 4xx/5xx errors | `ORIGINAL_REQUEST.md:46` |
| 16 | External Interfaces | MCP JSON-RPC 2.0 Interface | Stdio tools: `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, etc. | JSON-RPC 2.0 stdin | JSON-RPC stdout | JSON-RPC error objects | `ORIGINAL_REQUEST.md:47` |
| 17 | External Interfaces | CLI Preset: Reggae Anthem | `--create-reggae-anthem` pre-loaded with 76 BPM "One Love" lyrics | CLI invocation | Mastered WAV & version tree | Exit code 1 with error log | `ORIGINAL_REQUEST.md:48` |
| 18 | External Interfaces | CLI Command: Create Hit | `--create-hit --genre "<genre>" --prompt "<prompt>"` | CLI arguments | Mastered WAV & version tree | Exit code 1 on invalid argument | `ORIGINAL_REQUEST.md:60` |

---

## 6. Edge Cases

| # | Feature | Input / Condition | Observed / Documented Behavior |
|---|---|---|---|
| 1 | Hardware Sizing | System has 0 GPU / `nvidia-smi` missing or fails | `SystemMemory::detect_live()` detects CPU only; automatically forces `engine_mode = "pure_rust"`. |
| 2 | Hardware Sizing | GPU detected but free VRAM < 4.0 GB | Flags heavy Hugging Face models as inadequate; routes stages 1–5 to pure-Rust DSP synthesis. |
| 3 | Ollama Connectivity | `http://127.0.0.1:11434` offline or connection refused | Ollama client catches timeout/connect error; immediately falls back to built-in `SongBlueprint` genre templates. |
| 4 | Lyric Tag Parsing | Lyrics provided without structural brackets or malformed brackets | Node 1 cleans and inserts missing tags; asserts `[Intro]`, `[Verse 1]`, `[Chorus]`, `[Outro]` boundaries. |
| 5 | Crossfade Alignment | BPM with fractional period (e.g. 76 BPM = 0.78947s per beat) | Computes exact sample count ($N = \text{round}(\text{duration} \times 48000)$) avoiding phase clicks or drift. |
| 6 | Quality Gate Retry | `clipping = true` or `judge_score < 7.5` and `retry_count < 2` | Increments `retry_count` and cycles back to Node 4 (or Node 2 if tempo drift score > 0.5); does not halt pipeline. |
| 7 | Quality Gate Exhaustion | `retry_count == 2` and `quality_passed == false` | Logs warning, applies aggressive brickwall limiter in Node 6, and produces best-effort master without crashing. |
| 8 | Stem Regeneration | `POST /api/songs/regenerate-stem` for single stem (e.g. `"guitar"`) | Re-runs Node 4 solely for guitar stem in isolation; updates `stem_paths["inpainted_guitar"]` and re-masters. |
| 9 | Extreme Tempos | User specifies 40 BPM or 220 BPM | Clamps BPM within valid bounds (40 to 220 BPM); generates appropriate note subdivisions. |
| 10 | Version Checkpointing | Disk full or SQLite database locked | Checkpointer reports IO error; returns in-memory state transition without losing pipeline progress. |

---

## 7. Verification Method

To independently verify the completeness, correctness, and implementation readiness of this specification matrix:

1. **Verify State Definition & Schemas**:
   - Inspect `d:\harfile\music\crates\music_core\src\schema.rs` and verify how `SongBlueprint`, `SongSection`, `StemMixConfig`, and `PipelineModelStack` align with the `MusicProductionState` fields specified in Section 4.2.
2. **Verify Porting References**:
   - Inspect `d:\harfile\ModelFusion\crates\model_selection\src\memory.rs` (lines 71–236) to confirm `SystemMemory::detect_live()` and VRAM budgeting logic.
   - Inspect `d:\harfile\killswitch\crates\killswitch-orchestrator\src\graph.rs` (lines 24–150) to confirm `StateGraphBuilder` and `CompiledStateGraph` execution semantics.
3. **Run Unit & Integration Tests When Implemented**:
   - Execute workspace compile check: `cargo check --workspace`
   - Execute unit and integration tests: `cargo test --workspace`
   - Execute reggae anthem pure-Rust CLI smoke test: `cargo run -p music_cli -- --create-reggae-anthem --pure-rust`
   - Invalidation conditions: Any compilation failure, missing field in `MusicProductionState`, or failure to liberate VRAM between external model stages invalidates the implementation against this specification.
