# DISPATCH: Milestone 6 Remediation & Feature Worker (worker_m6_remed)

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Objective
Execute Milestone 6 Iteration 2: Remediate the `challenger_m6_1` finding in `tts_engine.rs`, implement Dynamic Hugging Face Music Model Discovery (`hf_model_discovery.rs`), implement Ollama Lifecycle Detection & Memory Sizing (`ollama_lifecycle.rs`), verify all tests, and configure git remote `https://github.com/oyesanyf/MusicFusion.git`.

## File Ownership
You exclusively own and may edit:
- `crates/music_fusion/src/tts_engine.rs`
- `crates/music_fusion/src/hf_model_discovery.rs` (new module)
- `crates/music_fusion/src/ollama_lifecycle.rs` (new module)
- `crates/music_fusion/src/lib.rs`
- `crates/music_cli/src/rest_server.rs`
- `crates/music_cli/src/main.rs`
- `crates/music_cli/src/cli_args.rs`
- `crates/music_cli/webui/index.html`
- `crates/music_cli/webui/app.js`
- `crates/music_cli/webui/styles.css`
- `README.md`

## Detailed Implementation Tasks

### 1. Fix SAPI WAV Header Chunk Parsing in `crates/music_fusion/src/tts_engine.rs`
In `read_wav_pcm16_stereo()`:
- DO NOT hardcode `let mut data_offset = 36;`.
- Dynamically walk RIFF subchunks from byte 12:
  ```rust
  let mut offset = 12;
  let mut data_offset = None;
  let mut data_size = 0;
  while offset + 8 <= bytes.len() {
      let chunk_id = &bytes[offset..offset + 4];
      let chunk_size = u32::from_le_bytes([
          bytes[offset + 4],
          bytes[offset + 5],
          bytes[offset + 6],
          bytes[offset + 7],
      ]) as usize;
      if chunk_id == b"data" {
          data_offset = Some(offset + 8);
          data_size = chunk_size;
          break;
      }
      offset += 8 + chunk_size;
  }
  ```
- Ensure temporary SAPI script/wav files (`_sapi_script.ps1`, `_sapi_lead.wav`, `_sapi_choir.wav`) are cleaned up immediately after reading.
- Verify `cargo test -p music_fusion --test challenger_m6_tts_stress_tests` passes 100%.

### 2. Dynamic Hugging Face Model Discovery & Upgrade (`crates/music_fusion/src/hf_model_discovery.rs`)
Implement `HfModelDiscoveryEngine`:
- Queries `https://huggingface.co/api/models` (with timeout and local cached fallback) across:
  - Music generation (`text-to-audio`, tag `music`)
  - Vocal/TTS synthesis (`text-to-speech`, tag `singing`)
  - Voice conversion (`audio-to-audio`, tag `voice-conversion`)
  - Stem separation (`audio-source-separation`)
- Evaluates discovered models against live system resources via `SystemMemory::detect_live()` to ensure active models fit free RAM & GPU VRAM without OOM.
- Saves active model registry to `storage/hf_music_models_registry.json`.
- Expose REST endpoints:
  - `GET /api/models/registry`
  - `POST /api/models/discover-upgrade`
- Add CLI flag `--update-models` in `cli_args.rs` and `main.rs`.
- In WebUI: add "✨ Discover & Upgrade AI Music Models" button and active model badge.

### 3. Automatic Ollama Detection, Installation & Memory Sizing (`crates/music_fusion/src/ollama_lifecycle.rs`)
Implement `OllamaLifecycleManager`:
- Probe `http://127.0.0.1:11434/api/tags` with a 400ms timeout.
- If not running:
  - Check if `ollama.exe` is in process PATH or standard installation directories (`%LOCALAPPDATA%\Programs\Ollama\ollama.exe`).
  - If missing on system, trigger silent winget install (`winget install Ollama.Ollama --silent --accept-package-agreements --accept-source-agreements`) or `OllamaSetup.exe /silent`.
  - Strictly observe Universal Zero-Touch PATH Law: process-scoped PATH only (`std::env::set_var("PATH", ...)`), NEVER mutate registry.
  - Spawn background daemon with `CREATE_NO_WINDOW` (`0x08000000`).
- Model sizing: strictly evaluate runtime free/available memory (`res.free_ram_gb`, `res.free_vram_mb`), NEVER total physical RAM:
  - Free RAM >= 48 GB OR Free VRAM >= 24 GB -> `qwen2.5:32b`
  - Free RAM >= 24 GB OR Free VRAM >= 14 GB -> `qwen2.5:14b`
  - Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> `qwen2.5:7b`
  - Free RAM >= 6 GB OR Free VRAM >= 2 GB -> `qwen2.5:3b`
  - Free RAM < 6 GB -> `qwen2.5:1.5b`
  - NEVER append `-coder` suffixes.
- Background pull: `ollama pull <model>` if recommended model is not present in installed tags.
- Expose REST endpoint:
  - `POST /api/ollama/bootstrap` (returns live status, recommended model, installed models).
- In WebUI: add Ollama status header pill and model selector.

### 4. Git Remote Setup, Build & Verification
- Verify all workspace tests pass: `cargo test --workspace`.
- Check git remote: configure origin to `https://github.com/oyesanyf/MusicFusion.git` on branch `main`.
- Verify `README.md` is present, comprehensive, and clean.

## Output
Write complete 5-component report to `d:\harfile\music\.agents\teamwork\worker_m6_remed\handoff.md`.


## 2026-11-01T15:01:46Z
You are worker_m6_remed (Milestone 6 Remediation & Feature Worker).
Your working directory is `d:\harfile\music\.agents\teamwork\worker_m6_remed\`.
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` before starting work.
Read `d:\harfile\music\.agents\teamwork\worker_m6_remed\DISPATCH.md` for your full task description.
Read `d:\harfile\music\.agents\teamwork\challenger_m6_1\handoff.md` for the exact SAPI WAV parsing bug details.

Scope & Tasks:
1. Fix `read_wav_pcm16_stereo()` in `crates/music_fusion/src/tts_engine.rs`: dynamically walk RIFF subchunks from offset 12 so 18-byte `fmt ` chunks (standard Windows SAPI headers starting at offset 38) parse cleanly. Ensure temp SAPI files are deleted.
2. Implement Dynamic Hugging Face Music Model Discovery (`crates/music_fusion/src/hf_model_discovery.rs`, export in `lib.rs`, add REST `/api/models/discover-upgrade` and `GET /api/models/registry`, `--update-models` CLI flag, and WebUI button).
3. Implement Automatic Ollama Detection, Silent Install & Dynamic Memory Sizing (`crates/music_fusion/src/ollama_lifecycle.rs`, export in `lib.rs`, add REST `/api/ollama/bootstrap`, process-scoped PATH only, dynamic RAM/VRAM sizing rules, WebUI card).
4. Run tests: `cargo test -p music_fusion` and `cargo test --workspace`.
5. Check git remote: configure origin to `https://github.com/oyesanyf/MusicFusion.git` on branch `main`, ensure `README.md` is complete.


## 2026-11-01T15:12:19Z
**Context**: Milestone 6 Remediation Progress Check
**Content**: Heartbeat check: Please provide a brief update on your current step across Tasks 1–11 (SAPI WAV chunk fix, hf_model_discovery, ollama_lifecycle, REST routes, WebUI, git remote setup).
**Action**: Please reply with your current status.


## 2026-11-01T15:14:09Z
**Context**: URGENT USER FEEDBACK & HIGH-PITCH NOISE FIX DISPATCH
**Content**: The user reported: "I am hearing some high pitch noise".
You must fix ALL FOUR root causes immediately before completing:

1. `crates/music_dsp/src/groove_synth.rs`:
   - Remove ALL high-frequency sine waves (> 650 Hz) from drum voices (`SnareAcoustic`, `SnareRimshot`, `Clap`, `HiHatClosed`, `HiHatTriplet`, `HiHatOpen`, `Clave`, `Shaker`, `JazzRideCymbal`, `CrashCymbal`). Pure sines at 2.4k, 3.2k, 4.5k, 5.8k, 6.8k, 7.2k, 8.3k, 8.9k, 9.4k sound like piercing beeps/mosquito whines!
   - Replace with a warm, 2-stage low-passed soft percussion brush (running `rng.next_f32()` through a 3-sample moving-average filter attenuated to 0.05..0.10 with fast decay) and warm low-mid body tones (110–420 Hz).
2. `crates/music_pipeline/src/nodes/node2_groove_yue.rs` & `crates/music_fusion/src/tts_engine.rs`:
   - Change Windows SAPI SSML pitch shifts from extreme +6st/+8st/+9st/+11st down to natural human range: @('+0st', '+1st', '+2st', '-1st') for Lead and @('+2st', '+3st') for Choir.
   - Remove 1800–3010 Hz formant sine waves (vf1, vf2, vf3 / res1, res2, res3); keep harmonic singing carrier strictly in the warm 110–330 Hz range.
3. `crates/music_pipeline/src/nodes/node6_analog_mastering.rs` & `node3_vocal_timbre.rs`:
   - Reduce f3 (2600 Hz) and presence filter (3200 Hz) gain to 0.0 dB.
   - In `node6_analog_mastering.rs`, apply a gentle 1-pole warm analog tape high-cut filter (`y[n] = 0.65 * x[n] + 0.35 * y[n-1]`) to `inst_bus` rolling off above 3.5 kHz.
4. Regenerate all sample songs on disk (`roots_reggae_anthem` v1/v2, `song_afrobeats_002` v1/v2, `song-one-love-shining-1977` v1/v2, `song_reggae_anthem_001` v1/v2) so all WAV files on disk have warm, natural vocals and zero high-pitched whistle.
**Action**: Apply these 4 audio fixes immediately alongside your current deliverables, recompile, regenerate the sample songs, and report in handoff.md.
