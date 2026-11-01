# Dispatch: Explorer M3-2 (Pipeline Nodes 1, 2, 3: Panel, Groove/YuE, Vocal Timbre)

## Objective
Design and specify Nodes 1, 2, and 3 of the 6-stage sequential fusion pipeline in `crates/music_pipeline`:
1. **Node 1**: `ollama_producer_panel_node.rs`
   - Invokes `music_fusion::panel::OllamaProducerPanel::run_panel`.
   - Structures bracketed lyric tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`).
   - Populates `MusicBlueprint` and transitions state to `StageType::CompositionPlanning`.
2. **Node 2**: `groove_and_yue_synthesis_node.rs`
   - Probes `SystemMemory::should_use_dsp(StageType::YuEComposition)`.
   - Stage 1: Generates `draft_groove.wav` using `music_dsp::groove_synth::generate_groove` across all 7 canonical styles (One-Drop Reggae at 76 BPM for anthem preset, Afrobeats, Amapiano, EDM, Rock, Jazz, Classical).
   - Stage 2: YuE multi-track vocal and accompaniment synthesis. If GPU/HF, spawns external subprocess via `tokio::process::Command` with Windows `CREATE_NO_WINDOW` (0x08000000) and awaits clean termination to guarantee 100% VRAM reclamation. If DSP fallback, generates acoustic stems.
3. **Node 3**: `vocal_timbre_fusion_node.rs`
   - Stage 3: Seed-VC or pure-Rust vocal formant & rasp processor.
   - If DSP fallback, executes `music_dsp::vocal_formant::apply_vocal_timbre` with archetype matching (Jamaican Tenor, Soulful Soprano, Gritty Rock Baritone, etc.) and `PitchLockVocoder`.
   - Saves `raspy_tenor_vocals.wav` (or lead vocal stem).

## Inputs
- Read `ORIGINAL_REQUEST.md`, `PROJECT.md`, `TEST_INFRA.md`.
- Inspect `crates/music_fusion/src/panel.rs`, `memory.rs`.
- Inspect `crates/music_dsp/src/groove_synth.rs`, `vocal_formant.rs`.

## Output
Deliver `d:\harfile\music\.agents\teamwork\explorer_m3_2\handoff.md` with complete node implementations, data flows, subprocess spawning commands, and test strategies.

## 2026-11-01T02:55:02Z
From: 4caf3f77-b3ac-4b67-8bda-934a91faafab
Priority: MESSAGE_PRIORITY_HIGH
Content: You are Explorer M3-2 for Milestone 3 (Native Rust StateGraph & 6 Fusion Nodes).
Your working directory is d:\harfile\music\.agents\teamwork\explorer_m3_2\.
Read d:\harfile\music\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, d:\harfile\music\TEST_INFRA.md, and your DISPATCH.md at d:\harfile\music\.agents\teamwork\explorer_m3_2\DISPATCH.md.
Design Nodes 1, 2, and 3:
- Node 1: ollama_producer_panel_node (lyric bracket tags, blueprint).
- Node 2: groove_and_yue_synthesis_node (stage routing: pure-Rust groove synth vs external YuE subprocess with deterministic VRAM reclamation).
- Node 3: vocal_timbre_fusion_node (vocal formant & rasp processor, Seed-VC or pure-Rust vocal formant filter, vocoder pitch lock).
Write your design and handoff report to d:\harfile\music\.agents\teamwork\explorer_m3_2\handoff.md.
Send a message back to parent when done.
