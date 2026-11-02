# Original User Request

## Initial Request — 2026-11-02T14:32:31Z

Build, test, and ship the 20-file Reference WAV Importer and RL Pipeline with built-in Legal Originality / Anti-Plagiarism Guard in `d:\harfile\music`:

Key Deliverables:
1. **Multi-File Reference WAV Importer (Up to 20 `.wav` Files)**:
   - Native file picker (`<input type="file" multiple accept=".wav,audio/wav">`) and drag-and-drop in `crates/music_cli/webui/index.html` and `crates/music_cli/webui/app.js` allowing the user to select/upload 1 to 20 `.wav` files at once (enforcing the 20-file cap clearly in UI and backend).
   - Backend endpoint(s) in `crates/music_cli/src/rest_server.rs` (e.g., `POST /api/reference-wavs/upload`, `GET /api/reference-wavs`, `DELETE /api/reference-wavs`, `POST /api/reference-wavs/generate-rl`) that parse and analyze up to 20 real `.wav` files.
2. **Stage 1: Reference WAV Tokenization (RVQ) & Multi-File Feature Extraction**:
   - Residual Vector Quantization (RVQ) discrete codebook tokenizer + acoustic analyzer extracting per-file and ensemble-blended features across up to 20 reference WAVs:
     - BPM / beat grid & rhythmic onset envelope
     - Krumhansl-Schmuckler 12-bin chroma vector, musical key & scale profile
     - Spectral timbre envelope, centroid, crest factor, dynamic range
     - Vocal presence profile (1–4 kHz formant band ratio)
     - Joint audio style embedding vector
3. **Stage 2 & 3: RL Policy (PPO / Contextual Bandit + DPO) & Composite Reward with Mandatory Originality / Anti-Plagiarism Guard**:
   - Multi-file style blending across the uploaded reference WAVs (up to 20 files) so no single input file is copied.
   - Composite Reward Function:
     `R_total = w_1 * R_style(y, x_ref_ensemble) + w_2 * R_harmony(y) + w_3 * R_rhythm(y) + w_4 * R_vocal_clarity(y) + w_5 * R_human + w_6 * R_originality_guard(y, {x_ref_1..x_ref_N})`
   - **Mandatory Built-in Legal Originality / Anti-Plagiarism Guard (`R_originality_guard`)**:
     - Computes mathematical similarity checks between the generated song/melody and every individual uploaded reference WAV:
       1. Max N-gram pitch/chroma sequence overlap penalty (ensuring no copied melodic phrases/hooks).
       2. Waveform cross-correlation & frame-level cosine similarity ceiling (penalizing/rejecting any candidate whose frame-level similarity or chroma sequence correlation to any single input file exceeds a strict originality threshold, while rewarding matching the blended ensemble style distribution).
       3. Reports a verifiable **Originality & Legal Compliance Score (e.g. 100% Original / Max Single-File Overlap < threshold)** in the API response and WebUI card.
   - Interactive DPO (`R_human`) preference learning wired to Co-Pilot Keep / Revert / Thumbs-Up feedback pairs `(y_chosen, y_rejected)`.
4. **End-to-End Verification**:
   - Unit and integration tests verifying 1..=20 WAV import, rejection/capping >20 files, RVQ tokenization, RL policy improvement over baseline (`Disabled`, `Random`, `Heuristic`, `NoLearning`), and mathematical proof that the Originality / Anti-Plagiarism Guard guarantees distinct melodies and waveforms from every input reference WAV.
   - Rebuild `music_cli` and restart the live server so the WebUI is immediately usable.

## Follow-up — 2026-11-02T14:46:20Z

ADDITIONAL MANDATORY LEGAL SAFEGUARDS FROM USER (Enforce in DSP/RL code, REST API report, and WebUI Legal Compliance Badge):

1. **Zero Direct Waveform Splicing / Sampling (`no_master_sampling_verified: true`)**:
   - Never splice, concatenate, or decode raw prefix tokens/samples from any uploaded reference `.wav` into the generated output audio (Bridgeport Music safeguard).
   - Reference WAVs are used strictly for extracting uncopyrightable statistical/style building blocks (BPM/tempo, musical key/scale, common chord progression transitions, spectral EQ balance, crest factor, and 1–4 kHz vocal mix ratio). All output audio and stems are synthesized 100% from scratch.
2. **Anti-Plagiarism Melodic & Waveform Ceiling (`max_ngram_melody_overlap` & `max_waveform_correlation` below strict ceiling)**:
   - Enforce a hard similarity ceiling in `R_style` / `R_originality_guard` that penalizes any candidate whose pitch-contour N-gram overlap or frame-level chroma/waveform correlation to any single uploaded reference WAV exceeds the originality threshold.
3. **Zero Biometric Voice Cloning (`synthetic_voice_only_elvis_compliant: true`)**:
   - Do not clone or copy speaker-identifying vocal timbres from uploaded WAVs (ELVIS Act / Right of Publicity safeguard). Only extract mix/presence targets (1–4 kHz vocal-to-accompaniment ratio and `-2.6 dB` sidechain ducking target) and synthesize all vocals using `MelodicSingingSynthesizer` (`FusionTtsEngine`).
4. **Surface Legal Compliance Report in API & WebUI**:
   - Include a clear `legal_compliance` object in the `/api/reference-wavs/generate-rl` response and display a visible **Legal & Originality Safeguard Verified** badge in the Web Studio UI showing:
     - `zero_waveform_sampling: true` (100% Newly Synthesized Master)
     - `max_single_file_melody_overlap_pct` (Strictly below ceiling, e.g. < 15%)
     - `synthetic_vocal_timbre_only: true` (No Unauthorized Voice Cloning)
     - `originality_score_pct` (e.g. 94%–100% Original)

## Follow-up — 2026-11-02T15:09:35Z

USER REQUIREMENT ADDITION FOR MILESTONE 8 (Include in active `worker_m8` implementation before finishing REST API & WebUI):

Add the **"Convert URL to WAV & Add to Current Mix"** workflow right alongside the 20-file Reference WAV importer:

1. **Web Studio UI (`crates/music_cli/webui/index.html` & `app.js`)**:
   - Right next to the "Import Reference WAVs (Up to 20 Files)" file picker, include a **YouTube / Audio URL input box** + **"Convert to WAV & Add to Mix"** button (supporting one or multiple URLs up to the 20-file pool limit).
   - Counts toward the up-to-20 reference file slots for the active mix and displays the converted `.wav` filename, source URL badge, duration, detected BPM, musical key, and inline audio preview player (`<audio controls>`) + download link in the Reference Pool list.
2. **Backend Endpoint (`POST /api/reference-wavs/from-url` in `crates/music_cli/src/rest_server.rs`)**:
   - **Strict SSRF & URL Validation**: Validate the URL scheme (`http://` or `https://` only) and mathematically block private/loopback/link-local IPv4/IPv6 ranges (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `::1`, `fc00::/7`, `fe80::/10`, `0.0.0.0`) before fetching or invoking external converters.
   - **URL-to-WAV Conversion Pipeline**:
     - For YouTube / streaming URLs: invoke `yt-dlp` (`--no-playlist -x --audio-format wav --postprocessor-args "ffmpeg:-ar 44100 -ac 2"`) or `ffmpeg` to extract clean stereo 44.1 kHz PCM `.wav` audio.
     - For direct `.wav` / audio HTTP(S) URLs (or if `yt-dlp` is not needed for a direct audio link): support direct download/conversion via `ffmpeg` or native WAV decoding so it works both with YouTube URLs and direct audio URLs.
   - **Saved to Current Mix Storage**: Persist the converted `.wav` file inside the active mix/reference storage directory (e.g. `storage/reference_wavs/` or `storage/reference_wavs/<song_id>/`) and expose it via GET so the user can preview, replay, or download the converted `.wav` for their mix.
   - **Automatic Stage 1 Feature & RVQ Analysis**: Immediately analyze the converted `.wav` (BPM, Krumhansl-Schmuckler key/chroma vector, spectral timbre envelope, 1–4 kHz vocal presence target, and RVQ discrete tokens) and add it to the ensemble style blend (enforcing the 20-file cap).
3. **Full Legal & Anti-Plagiarism Safeguards**:
   - Apply the exact same `LegalOriginalityGuard` (`zero_waveform_sampling: true`, maximum melodic N-gram similarity ceiling, and `synthetic_vocal_timbre_only: true` ELVIS Act compliance) to URL-converted reference WAVs so only uncopyrightable musical building blocks (tempo, key/scale, groove, mix balance) guide the RL synthesis.
