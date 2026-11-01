Build an autonomous AI music production agent with a lightweight Python LangGraph orchestration client (managed via uv) communicating with an existing Rust fusion engine for model execution, hardware scheduling, VRAM lifecycle management, and native audio DSP. [OPEN: Purpose - showcase, production, exploration, or let team decide?]

Working directory: d:\harfile\music
Integrity mode: [OPEN: Integrity mode based on forbidden activities]

Reference Material:
- Models: ACE-Step/ACE-Step-v1.5, m-a-p/YuE-s1-7B-anneal-en-icl, Plachtaa/Seed-VC, facebook/htdemucs_ft, stabilityai/stable-audio-open-1.0
- Style prompt: "1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, 76 BPM, hypnotic one drop master drumming with complex polyrhythmic snare rolls and deep sub bass, virtuosic soaring psychedelic blues rock electric guitar solos, warm analog tape production, lush gospel backing choir."
- Lyrics: 1977 Roots Reggae Compassion Anthem ("One Love" spiritual successor) and Freedom Variant ("Rise Up Together")

## Requirements

### R1. Python LangGraph Workflow Client
Implement a lightweight Python LangGraph state machine (state.py, nodes.py, main.py) managed with uv. The state machine tracks MusicProductionState across prompt preparation, rhythm/song dispatch, vocal timbre conversion, virtuoso instrumentation stem generation/splicing, DSP verification, and final mastering.

### R2. Rust Fusion Engine IPC & Audio DSP Interface
Connect the LangGraph client to the Rust fusion engine via an IPC/RPC endpoint or bridge. [OPEN: IPC/bridge mechanism: inspect existing crate, HTTP/gRPC, MCP server, PyO3, or let team decide?] Rust manages hardware-aware GPU scheduling, sequential model execution, deterministic VRAM teardown between stages, and native zero-copy audio DSP (symphonia, hound, rubato) for stem crossfading, beat-grid alignment at 76 BPM, clipping checks, and stem mixing (-3 dB instrumental headroom, 0 dB center vocals). Audio buffers are not serialized across Python; only file paths and telemetry metadata cross the boundary.

### R3. Automated DSP Quality Verification & Retry Loop
Implement a verification stage in LangGraph querying the Rust DSP analyzer for zero-crossing anomalies, silence dropouts, and digital clipping, updating telemetry (peak dB, RMS, tempo drift) and routing conditionally back to rhythm/song generation (up to 2 retries) if quality checks fail.

### R4. Pre-Configured Anthem Execution & Mock Verification Mode
Provide main.py pre-loaded with the 76 BPM roots reggae anthem lyrics and style prompt. Include a `--mock-inference` flag to verify the full Python-to-Rust IPC handshake, state machine transitions, and audio DSP mixing using synthetic test waveforms without requiring live GPU weights.

## Acceptance Criteria

### Workflow & State Transitions
- [ ] LangGraph StateGraph executes all nodes in sequence: prepare_prompts -> dispatch_rhythm_and_song -> dispatch_vocal_fusion -> dispatch_virtuoso_instrumentation -> rust_dsp_verification -> dispatch_mastering.
- [ ] Lyrics are sanitized for YuE compatibility (descriptive text stripped from structural tags, instrumental cues moved to style prompt).
- [ ] Telemetry metrics (peak dB, RMS, tempo drift) are populated from Rust DSP analysis into MusicProductionState.
- [ ] Conditional edge triggers retry on failed DSP verification up to 2 times before failing or passing.

### Rust Engine Integration & DSP
- [ ] Rust engine interface receives task dispatch calls and returns local .wav artifact paths and telemetry metadata.
- [ ] Audio waveforms are processed natively in Rust with zero audio buffer serialization over Python IPC.
- [ ] [OPEN: Acceptance criteria for verification resources and test execution: mock inference only, live GPU inference, or both?]
