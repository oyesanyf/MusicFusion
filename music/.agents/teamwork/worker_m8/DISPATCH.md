## 2026-11-02T14:48:23Z
You are worker_m8 in the orchestrator team for d:\harfile\music.
Working directory: d:\harfile\music\.agents\teamwork\worker_m8\
Scope document: d:\harfile\music\.agents\teamwork\orchestrator\SCOPE.md
Authoritative request: d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md

You MUST read d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md immediately.
You MUST read d:\harfile\music\.agents\teamwork\orchestrator\SCOPE.md immediately.
You MUST read the explorer and spec miner reports:
- d:\harfile\music\.agents\teamwork\explorer_m8_1\handoff.md and analysis.md
- d:\harfile\music\.agents\teamwork\explorer_m8_2\handoff.md and analysis.md
- d:\harfile\music\.agents\teamwork\spec_miner_m8\handoff.md and analysis.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

MANDATORY USER RULES:
1. RULE[auto_commit_and_push.md]: Commit and push immediately after verified increments via `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "..."` or git commit.
2. RULE[rl_benchmarking_and_evaluation.md]: Benchmarks MUST explicitly label backend as "SyntheticAudioEnvironment", compare learned policy against 4 baselines (Disabled, Random, Heuristic, NoLearning) with matched episode seeds, and report 95% Confidence Intervals.
3. RULE[no_mocks_write_actual_code.md]: Production code only: no mocks, no fake data, real WAV decoding and genuine math.
4. RULE[environment_safety.md]: Process-scoped environment only; never touch system/user registry or mutate system PATH.

MANDATORY LEGAL SAFEGUARDS:
1. Zero Direct Waveform Splicing / Sampling (Bridgeport Music safeguard: `no_master_sampling_verified: true`):
   - Never splice, concatenate, or decode raw prefix tokens/samples from any uploaded reference .wav into output audio.
   - All output audio and stems are synthesized 100% from scratch. Reference WAVs are strictly for statistical feature targets (BPM, key, spectral balance, 1-4 kHz ratio).
2. Anti-Plagiarism Melodic & Waveform Ceiling:
   - Hard similarity ceiling penalizing any candidate whose pitch-contour N-gram overlap or frame-level chroma/waveform correlation to ANY single reference WAV exceeds threshold.
3. Zero Biometric Voice Cloning (ELVIS Act safeguard: `synthetic_voice_only_elvis_compliant: true`):
   - Do not clone speaker-identifying vocal timbres from uploaded WAVs. Only extract mix targets (1-4 kHz ratio and -2.6 dB ducking); all vocals synthesized via MelodicSingingSynthesizer.
4. Surface Legal Compliance Report in API & WebUI:
   - Return `legal_compliance` object in `/api/reference-wavs/generate-rl` response and render visible "Legal & Originality Safeguard Verified" badge in WebUI showing `zero_waveform_sampling: true`, `max_single_file_melody_overlap_pct`, `synthetic_vocal_timbre_only: true`, and `originality_score_pct`.

IMPLEMENTATION DELIVERABLES & EXCLUSIVE WRITE OWNERSHIP:
1. `crates/music_dsp`:
   - `crates/music_dsp/src/acoustic_analyzer.rs`: AcousticAnalyzer extracting per-file & ensemble features (BPM, Krumhansl-Schmuckler 12-bin chroma, timbre envelope, centroid, crest factor, dynamic range, 1-4 kHz formant ratio, 64-dim style embedding, anti-monopoly <=60% blending).
   - `crates/music_dsp/src/rvq.rs`: Multi-stage RvqTokenizer (M=4..8, K=256..1024, D=32) with deterministic orthogonal DCT-II/Haar codebooks, monotonic residual norm reduction, rolling N-gram hashing.
   - `crates/music_dsp/src/lib.rs`: export modules.
2. `crates/music_core`:
   - `crates/music_core/src/state.rs`: update `MusicProductionState` with `reference_wavs` and `originality_report`.
3. `crates/music_fusion`:
   - `crates/music_fusion/src/originality_guard.rs`: Triple-metric LegalOriginalityGuard (transposition-invariant N-gram overlap, frame-level chroma cosine ceiling <= 0.75, waveform cross-correlation ceiling <= 0.65, LegalOriginalityReport & LegalComplianceInfo).
   - `crates/music_fusion/src/rl_policy.rs`: PPO/Contextual Bandit policy with composite reward R_total, online DPO preference learning wired to Keep / Revert pairs.
   - `crates/music_fusion/src/lib.rs`: export modules.
4. `crates/music_cli`:
   - `crates/music_cli/Cargo.toml`: add multipart feature to axum (`features = ["macros", "multipart"]`).
   - `crates/music_cli/src/rest_server.rs`: routes `/api/reference-wavs/upload` (capping 1..=20, rejecting >20 with 400), `GET /api/reference-wavs`, `DELETE /api/reference-wavs`, `DELETE /api/reference-wavs/:id`, `POST /api/reference-wavs/generate-rl`, `/api/feedback/preference`.
   - `crates/music_cli/webui/index.html`, `crates/music_cli/webui/app.js`, `crates/music_cli/webui/styles.css`: native file picker `<input type="file" multiple accept=".wav,audio/wav">`, drag-drop zone, 20-file cap counter badge, track cards with delete/audition, RL generation card, Legal & Originality Safeguard Verified badge.
5. Verification & Tests:
   - DSP unit tests for acoustic analyzer & RVQ.
   - Fusion tests for Legal Originality Guard & DPO.
   - CLI/REST tests for 1..=20 WAV upload, >20 rejection, invalid WAV rejection.
   - RL 4-baseline empirical superiority benchmark test with 95% CIs.
   - Adversarial originality test proving zero melodic hooks copied.
   - Build workspace (`cargo build --workspace`, `cargo test --workspace`) and verify 100% pass.
   - Rebuild `music_cli` and verify the live server can start.
   - Commit & solo PR ship via `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "feat: 20-file reference WAV importer & RL pipeline with legal originality guard"`.

## 2026-11-02T15:07:07Z
**Context**: Status check on Milestone 8 implementation
**Content**: Heartbeat check-in: please update progress.md with current completed components (DSP acoustic analyzer, RVQ, RL policy, REST server, WebUI).
**Action**: Continue implementation and provide quick status update when convenient.

## 2026-11-02T15:10:29Z
**Context**: CRITICAL SCOPE EXPANSION FOR MILESTONE 8
**Content**: The user has added the "Convert URL to WAV & Add to Current Mix" workflow right alongside the 20-file Reference WAV importer:

1. **Web Studio UI (`crates/music_cli/webui/index.html` & `app.js`)**:
   - Next to the "Import Reference WAVs (Up to 20 Files)" picker, add a **YouTube / Audio URL input box** + **"Convert to WAV & Add to Mix"** button (`#ref-url-input`, `#btn-convert-url-wav`).
   - Counts toward the up-to-20 reference file slots for the active mix.
   - Displays converted `.wav` filename, source URL badge, duration, detected BPM, musical key, and inline audio preview player (`<audio controls>`) + download link in the Reference Pool list.
2. **Backend Endpoint (`POST /api/reference-wavs/from-url` in `crates/music_cli/src/rest_server.rs`)**:
   - **Strict SSRF & URL Validation**: Validate URL scheme (`http://` or `https://` only) and mathematically block private/loopback/link-local IPv4/IPv6 ranges (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `::1`, `fc00::/7`, `fe80::/10`, `0.0.0.0`) per `containment_ring_breakout_invariants.md`.
   - **URL-to-WAV Conversion Pipeline**:
     - For YouTube / streaming URLs: invoke `yt-dlp` (`--no-playlist -x --audio-format wav --postprocessor-args "ffmpeg:-ar 44100 -ac 2"`) or `ffmpeg` via `tokio::process::Command` to extract clean stereo 44.1/48 kHz PCM `.wav`.
     - For direct `.wav` / audio HTTP(S) URLs: download bytes via `reqwest` and validate/convert using `AudioBuffer::from_wav_bytes` or ffmpeg.
   - **Saved to Current Mix Storage**: Persist converted `.wav` inside reference storage directory and expose via GET so the user can preview, replay, or download.
   - **Automatic Stage 1 Feature & RVQ Analysis**: Immediately run `AcousticAnalyzer` + `RvqTokenizer` on converted WAV and add to ensemble style blend (enforcing 20-file cap).
3. **Legal Safeguards**:
   - Zero direct waveform splicing (`no_master_sampling_verified: true`), melodic N-gram similarity ceiling, and ELVIS Act compliance (`synthetic_voice_only_elvis_compliant: true`).

**Action**: Incorporate `POST /api/reference-wavs/from-url`, SSRF checks, URL-to-WAV conversion, and WebUI input/button into your implementation before shipping.
