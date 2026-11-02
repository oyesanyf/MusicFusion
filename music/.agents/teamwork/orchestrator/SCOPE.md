# Scope: Milestone 8 — 20-File Reference WAV Importer, Multi-File RVQ Tokenizer, RL Pipeline & Legal Originality Guard

## Architecture & Data Flow

```
+------------------------------------------------------------------------------------------------+
| WebUI (index.html / app.js / styles.css)                                                       |
|  - Native Multi-WAV Picker (<input type="file" multiple accept=".wav,audio/wav">)              |
|  - Drag-and-Drop Zone (#ref-wav-dropzone) with 1..=20 File Cap Badge & Real-Time Alert        |
|  - Uploaded Reference WAV List (Audition playback, metadata tags, delete button)               |
|  - RL Generation Card with Live Verifiable Legal Originality & Compliance Score Card          |
+------------------------------------------------------------------------------------------------+
                                           │
                    POST /api/reference-wavs/upload (1..=20 WAVs)
                    GET /api/reference-wavs
                    DELETE /api/reference-wavs / DELETE /api/reference-wavs/:id
                    POST /api/reference-wavs/generate-rl
                                           ▼
+------------------------------------------------------------------------------------------------+
| crates/music_cli/src/rest_server.rs (Axum REST API)                                           |
|  - Pure-Rust WAV header verification & duration/sample_rate extraction (AudioBuffer)          |
|  - Strict 20-file cap enforcement (rejects >20 with HTTP 400 Bad Request)                     |
|  - Session reference WAV storage management & path traversal sanitization                      |
+------------------------------------------------------------------------------------------------+
                                           │
                                           ▼
+------------------------------------------------------------------------------------------------+
| crates/music_dsp/src/acoustic_analyzer.rs & crates/music_dsp/src/rvq.rs                        |
|  - AcousticAnalyzer:                                                                           |
|    * BPM / beat grid & rhythmic onset envelope (spectral flux + autocorrelation + log-Gaussian) |
|    * 12-bin Pitch Class Profile & Krumhansl-Schmuckler 24-key Pearson correlation              |
|    * 8-band spectral timbre envelope, centroid (Hz), crest factor, dynamic range               |
|    * Vocal presence profile (1–4 kHz formant band ratio, activity ratio)                       |
|    * 64-dim L2-normalized style embedding vector per WAV                                       |
|    * Ensemble Blending: Anti-monopoly weighted blending (<=60% single file when N>=3)          |
|  - RvqTokenizer:                                                                               |
|    * Multi-stage residual vector quantization (M=4..8 stages, K=256..1024, D=32)               |
|    * Pure-Rust deterministic orthogonal DCT-II / Haar basis codebooks                          |
|    * Guaranteed monotonic residual norm reduction (||r_m|| <= ||r_{m-1}||)                     |
|    * Discrete token sequence & rolling N-gram token hashing                                    |
+------------------------------------------------------------------------------------------------+
                                           │
                                           ▼
+------------------------------------------------------------------------------------------------+
| crates/music_fusion/src/rl_policy.rs & crates/music_fusion/src/originality_guard.rs            |
|  - Composite Reward Engine:                                                                    |
|    R_total = w_1*R_style + w_2*R_harmony + w_3*R_rhythm + w_4*R_vocal + w_5*R_human + w_6*R_orig  |
|  - Mandatory Legal Originality / Anti-Plagiarism Guard (R_originality_guard):                 |
|    1. 12-semitone transposition-invariant N-gram melodic overlap penalty                       |
|    2. Frame-level chroma cosine similarity ceiling (<= 0.75) vs EVERY individual input WAV     |
|    3. Waveform cross-correlation ceiling (rho_max <= 0.65) vs EVERY individual input WAV       |
|    * Generates LegalOriginalityReport with percentage score & compliance boolean               |
|  - Online DPO Preference Learning:                                                             |
|    * Wired to Co-Pilot Keep / Revert / Thumbs-Up feedback pairs (y_chosen, y_rejected)         |
|  - RL Policy (Contextual Bandit / PPO):                                                        |
|    * Optimizes music generation hyperparameters against R_total                                |
+------------------------------------------------------------------------------------------------+
                                           │
                                           ▼
+------------------------------------------------------------------------------------------------+
| Verification & Benchmarking Suites                                                             |
|  - rl_benchmarking_tests: Empirical learning superiority proof over 4 baselines                |
|    (Disabled, Random, Heuristic, NoLearning) with 95% CIs on SyntheticAudioEnvironment         |
|  - originality_guard_adversarial_tests: Mathematical proof of distinct melodies & waveforms   |
|  - reference_wav_import_tests: 1..=20 WAV import, >20 cap rejection, corrupt WAV rejection    |
+------------------------------------------------------------------------------------------------+
```

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| F1 | Multi-WAV WebUI Picker & Dropzone | Native `<input type="file" multiple accept=".wav,audio/wav">` & drag-drop in `index.html`/`app.js` | M8 | ORIGINAL_REQUEST §1 |
| F2 | Real-Time 20-File Cap UI Enforcement | Visual badge (`X / 20`), warning banner upon >20 selection, file counter | M8 | ORIGINAL_REQUEST §1 |
| F3 | Reference Track Card Grid & Audition | Dynamic cards for uploaded WAVs with duration, sample rate, channels, audition player & remove button | M8 | ORIGINAL_REQUEST §1 |
| F4 | REST Upload Endpoint | `POST /api/reference-wavs/upload` with multipart, pure-Rust header check, strict >20 reject (400) | M8 | ORIGINAL_REQUEST §1 |
| F5 | REST Reference Query & Delete Endpoints | `GET /api/reference-wavs`, `DELETE /api/reference-wavs`, `DELETE /api/reference-wavs/:id` | M8 | ORIGINAL_REQUEST §1 |
| F6 | REST RL Generation Trigger | `POST /api/reference-wavs/generate-rl` executing RVQ, RL style blending, & Originality Guard | M8 | ORIGINAL_REQUEST §1 |
| F7 | Pure-Rust Acoustic Feature Analyzer | BPM/beat grid, 12-bin chroma, Krumhansl-Schmuckler key, spectral timbre, centroid, crest factor, dynamic range, 1-4 kHz formant ratio | M8 | ORIGINAL_REQUEST §2 |
| F8 | Multi-WAV Ensemble Blending | Style vector weighted blending, tempo harmonization, consensus key, anti-monopoly ceiling (<=60%) | M8 | ORIGINAL_REQUEST §2 & §3 |
| F9 | Multi-Stage RVQ Tokenizer | Residual vector quantization ($M=4..8, K=256..1024$), deterministic orthogonal codebook, monotonic error reduction | M8 | ORIGINAL_REQUEST §2 |
| F10 | Composite Reward Function | $R_{total} = w_1 R_{style} + w_2 R_{harmony} + w_3 R_{rhythm} + w_4 R_{vocal} + w_5 R_{human} + w_6 R_{orig}$ | M8 | ORIGINAL_REQUEST §3 |
| F11 | Transposition-Invariant Melodic Plagiarism Check | 12-semitone invariant N-gram chroma/pitch sequence overlap penalty | M8 | ORIGINAL_REQUEST §3 |
| F12 | Waveform & Chroma Similarity Ceilings | Waveform $\rho_{max} \le 0.65$ & chroma cosine $\le 0.75$ ceilings vs EVERY individual reference WAV | M8 | ORIGINAL_REQUEST §3 |
| F13 | Verifiable Legal Originality Report | Structured report with `originality_score_pct`, `max_single_file_overlap`, `is_compliant`, and `legal_compliance` displayed in UI | M8 | ORIGINAL_REQUEST §3 & Legal Update |
| F14 | Interactive DPO Preference Learning | Preference updates $\theta \leftarrow \theta + \eta \beta (1-\hat{p})\Delta\phi$ from Keep / Revert pairs | M8 | ORIGINAL_REQUEST §3 |
| F15 | RL 4-Baseline Benchmark Suite | Rigorous proof of improvement over `Disabled`, `Random`, `Heuristic`, `NoLearning` with 95% CIs | M8 | ORIGINAL_REQUEST §4 & Rule |
| F16 | Originality Adversarial Verification | Empirical tests confirming distinct melodies and rejection of direct plagiarisms | M8 | ORIGINAL_REQUEST §4 |
| F17 | Zero Direct Waveform Splicing Guard | Bridgeport Music safeguard: zero raw prefix tokens/samples spliced from reference WAVs; 100% newly synthesized master | M8 | Legal Safeguards Update |
| F18 | Zero Biometric Voice Cloning Guard | ELVIS Act safeguard: zero voice cloning from uploaded audio; only 1-4 kHz presence targets used; all vocals synthesized via MelodicSingingSynthesizer | M8 | Legal Safeguards Update |
| F19 | Convert URL to WAV & Add to Mix | YouTube/audio URL input box, SSRF mathematical IP validation, yt-dlp/ffmpeg conversion to 44.1/48 kHz WAV, automatic RVQ feature extraction, adds to active mix pool | M8 | Scope Expansion Update |

## Implementation Plan
1. **Module 1: DSP Engine Expansion (`crates/music_dsp`)**:
   - `crates/music_dsp/src/acoustic_analyzer.rs`: Acoustic analyzer, Krumhansl-Schmuckler key profiler, timbre, formant ratio, ensemble blender.
   - `crates/music_dsp/src/rvq.rs`: Multi-stage RVQ tokenizer, codebook initialization, rolling N-gram token hashing.
   - Register in `crates/music_dsp/src/lib.rs`.
2. **Module 2: Core State & Fusion RL Engine (`crates/music_core` & `crates/music_fusion`)**:
   - `crates/music_core/src/state.rs`: Add `reference_wavs: Vec<ReferenceWavMeta>`, `originality_report: Option<LegalOriginalityReport>`.
   - `crates/music_fusion/src/originality_guard.rs`: Triple-check originality guard, cross-correlation ceiling, N-gram overlap penalty, `LegalOriginalityReport`.
   - `crates/music_fusion/src/rl_policy.rs`: PPO/Contextual Bandit policy, composite reward $R_{total}$, online DPO update.
   - Register in `crates/music_fusion/src/lib.rs`.
3. **Module 3: REST Server & WebUI (`crates/music_cli`)**:
   - `crates/music_cli/Cargo.toml`: Enable `axum = { version = "0.7", features = ["macros", "multipart"] }`.
   - `crates/music_cli/src/rest_server.rs`: Reference WAV session state, `/api/reference-wavs*` endpoints, 20-file cap check, integration with RL generator.
   - `crates/music_cli/webui/index.html`, `app.js`, `styles.css`: Native file picker, drag-and-drop zone, counter badge, track cards, RL generation card with originality score.
4. **Module 4: Verification & Benchmarks**:
   - Integration tests in `crates/music_dsp/tests/`, `crates/music_fusion/tests/`, `crates/music_cli/tests/`.
   - RL 4-baseline benchmark with 95% CIs (`SyntheticAudioEnvironment`).
   - Adversarial plagiarism tests proving non-infringement.
