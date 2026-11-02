# Handoff Report: Deliverable 1 — Multi-File Reference WAV Importer (Up to 20 .wav Files)

**Agent**: `explorer_m8_1`  
**Parent**: `f3dc28a2-6906-4296-bad9-049bc88a16ae`  
**Date**: 2026-11-02  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m8_1\`  
**Detailed Analysis**: `d:\harfile\music\.agents\teamwork\explorer_m8_1\analysis.md`

---

## 1. Observation

1. **Current WebUI File Selection**:
   - `crates/music_cli/webui/index.html:49`:
     ```html
     <input type="file" id="local-audio-file-input" accept=".wav,.mp3,.ogg,.flac,.m4a,audio/*" multiple style="display:none;">
     ```
   - `crates/music_cli/webui/app.js:1613-1634`: `handleLocalFilesSelected` reads files solely into in-memory ArrayBuffers via `file.arrayBuffer()`, transcodes client-side via `transcodeWavArrayBufferToPcm16IfFloat`, and generates browser `URL.createObjectURL(blob)`. It does not send audio to the backend, does not manage a multi-file reference session, and does not enforce a 20-file cap.
2. **Axum REST Server Routes**:
   - `crates/music_cli/src/rest_server.rs:1628-1660`: Defines `create_router(state: AppState) -> Router`. Current routes include `/api/health`, `/api/hardware`, `/api/audio-devices`, `/api/songs`, `/api/songs/create`, `/api/songs/iterate`, `/api/songs/regenerate-stem`, `/api/songs/master`, `/api/songs/tts-vocals`, and static files (`/`, `/index.html`, `/styles.css`, `/app.js`).
   - There are currently no reference WAV endpoints (`/api/reference-wavs*`).
3. **Pure-Rust WAV Parsing in DSP Engine**:
   - `crates/music_dsp/src/wav.rs:395-488`: Implements `AudioBuffer::from_wav_bytes(bytes: &[u8]) -> Result<Self, DspError>`. Directly validates RIFF/WAVE header chunk headers, supports 16-bit PCM, 24-bit PCM, and 32-bit Float, computes `duration_secs()`, `sample_rate`, and `channels`, and rejects truncated or invalid headers.
   - `crates/music_cli/Cargo.toml:9`: Already depends on `music_dsp = { path = "../music_dsp" }`.
4. **Axum Dependencies**:
   - `crates/music_cli/Cargo.toml:24`: `axum = { version = "0.7", features = ["macros"] }`.
   - `Cargo.lock:115-130`: Workspace already resolves `axum` 0.7.9. Enabling the `"multipart"` feature will allow `axum::extract::Multipart` for browser `FormData` uploads.
5. **Compilation Baseline**:
   - Executing `cargo check -p music_cli` passed with code 0 in 1.05s.

---

## 2. Logic Chain

1. **Observation 1 & 2** show that the WebUI currently lacks multi-file reference WAV management and the backend has no reference WAV endpoints.
2. Therefore, to fulfill Deliverable 1, we must:
   - Introduce `<input type="file" id="ref-wav-file-input" multiple accept=".wav,audio/wav">` and a drag-and-drop zone (`#ref-wav-dropzone`) in `index.html`.
   - Wire event handlers in `app.js` to enforce the 1..=20 file cap both client-side (via counter badge `#ref-wav-counter` and warning alert `#ref-wav-cap-warning`) and server-side.
3. **Observation 3** establishes that `crates/music_dsp/src/wav.rs` already contains genuine pure-Rust WAV decoding (`AudioBuffer::from_wav_bytes`).
   - Consequently, according to `RULE[no_mocks_write_actual_code.md]`, the backend can validate uploaded WAV headers and calculate duration, sample rate, channels, and peak amplitude using real DSP code without any synthetic or mock placeholders.
4. **Observation 4** confirms that enabling `features = ["macros", "multipart"]` in `crates/music_cli/Cargo.toml` provides native multipart parsing for `POST /api/reference-wavs/upload`.
5. Under `containment_ring_breakout_invariants.md`, file identifiers and path joins must be sanitized to reject directory traversal (`..`, `/`, `\`, `:`).
6. Under `ORIGINAL_REQUEST.md`, `POST /api/reference-wavs/generate-rl` must execute multi-WAV RVQ analysis, RL policy blending, and calculate the **Originality & Legal Compliance Score** to confirm zero melodic plagiarism against any individual reference file before returning the audio URLs.

---

## 3. Caveats

1. **Stage 1 (RVQ Tokenization) & Stage 2/3 (RL Policy & Anti-Plagiarism Guard Modules)**:
   - This exploration focuses on Deliverable 1 (Importer UI, file capping, and REST server API contracts). The specific mathematical implementation of the RVQ codebook and the PPO/Bandit RL reward functions belong to Deliverables 2 & 3.
2. **File Size Considerations**:
   - 20 uncompressed WAV files can sum to ~50–100 MB. The server's multipart body limit should accommodate up to 250 MB (`DefaultBodyLimit::max(250 * 1024 * 1024)` or per-request limits).

---

## 4. Conclusion

Deliverable 1 can be cleanly implemented without disrupting existing production routes:
1. **WebUI**: Add the `<input type="file" multiple accept=".wav,audio/wav">` picker, interactive drag-and-drop zone, 20-file cap counter (`X / 20`), warning banner, reference track card grid with audition playback/delete buttons, and the RL generation card displaying the **Originality & Legal Compliance Score**.
2. **REST Endpoints**:
   - `POST /api/reference-wavs/upload`: Accepts up to 20 `.wav` files via multipart, validates headers via `AudioBuffer::from_wav_bytes`, rejects empty/corrupted files, strictly enforces the 20-file cap with 400 Bad Request, and stores metadata + files.
   - `GET /api/reference-wavs`: Returns active reference WAVs, durations, sample rates, and file count.
   - `DELETE /api/reference-wavs`: Clears active reference WAV session and storage.
   - `DELETE /api/reference-wavs/:id`: Removes a specific reference WAV by ID with path sanitization.
   - `POST /api/reference-wavs/generate-rl`: Verifies 1..=20 files are present, executes multi-WAV ensemble pipeline, and returns the verified Originality Score and stem URLs.

---

## 5. Verification Method

1. **Compile & Check**:
   ```powershell
   cargo check -p music_cli
   ```
2. **Integration Tests**:
   Run the CLI & REST test suite:
   ```powershell
   cargo test -p music_cli --test cli_and_rest_tests
   ```
3. **Endpoint Validation via curl/reqwest**:
   - `POST /api/reference-wavs/upload` with 1 to 20 valid `.wav` files -> expect HTTP 200/201 and JSON metadata array.
   - `POST /api/reference-wavs/upload` with 21 files -> expect HTTP 400 with cap error message.
   - `POST /api/reference-wavs/upload` with corrupt/non-WAV file -> expect HTTP 400 with invalid header error.
   - `GET /api/reference-wavs` -> expect list of active files.
   - `DELETE /api/reference-wavs/:id` -> expect removal and updated count.
   - `POST /api/reference-wavs/generate-rl` -> expect `originality_report` with `originality_score_pct >= 90.0%` and `is_compliant: true`.
4. **Invalidation Conditions**:
   - Any test that generates fake or mock metadata rather than parsing real audio samples violates `no_mocks_write_actual_code.md`.
   - Accepting >20 reference files without returning HTTP 400 invalidates compliance with the 20-file cap requirement.
