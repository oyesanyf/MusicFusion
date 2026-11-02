# Comprehensive Investigation & Architecture Analysis: Deliverable 1 — Multi-File Reference WAV Importer (Up to 20 .wav Files)

**Explorer Agent**: `explorer_m8_1`  
**Date**: 2026-11-02  
**Target Workspace**: `d:\harfile\music`  
**Scope**: Deliverable 1 of `ORIGINAL_REQUEST.md` (WebUI native file picker, drag-and-drop zone, 20-file cap counter/warning, card presentation, generation trigger & Originality Score card, and Axum REST endpoints in `crates/music_cli/src/rest_server.rs`).

---

## 1. Executive Summary

The user's objective is to build, test, and ship a **Multi-File Reference WAV Importer (Up to 20 `.wav` Files)** coupled with an RL pipeline that features a built-in **Legal Originality / Anti-Plagiarism Guard**.

This analysis thoroughly examines:
1. The existing frontend implementation in `crates/music_cli/webui/index.html` and `crates/music_cli/webui/app.js`. Currently, file selection is limited to a single-track local browser decoder (`handleLocalFilesSelected`) that generates client-side Blob URLs for direct playback without sending audio to the backend or supporting multi-file ensemble reference ingestion.
2. The existing Axum REST server in `crates/music_cli/src/rest_server.rs`. The server handles song creation, version iteration, stem regeneration, mastering, TTS, and static asset serving, but lacks dedicated reference WAV endpoints.
3. The DSP audio buffer engine in `crates/music_dsp/src/wav.rs`. It already possesses pure-Rust RIFF/WAVE header decoding, sample rate validation, duration calculation, and channel normalization (`AudioBuffer::from_wav_bytes`), eliminating any need for mock data or external C libraries.

Below are the detailed findings, architectural designs, and actionable implementation recommendations.

---

## 2. Current Frontend State (`index.html`, `app.js`, `styles.css`)

### 2.1 Existing File Upload Mechanism
- **File Input in HTML** (`crates/music_cli/webui/index.html:49`):
  ```html
  <input type="file" id="local-audio-file-input" accept=".wav,.mp3,.ogg,.flac,.m4a,audio/*" multiple style="display:none;">
  ```
- **Trigger Buttons** (`index.html:45-48`, `index.html:449-451`):
  - Sidebar: `#btn-sidebar-load-local` ("📂 Load Local WAV File")
  - Song Card: `#btn-card-load-local` ("📂 Load Local WAV")
- **Event Binding** (`crates/music_cli/webui/app.js:1718-1753`):
  - Clicking either button delegates to `DOM.localAudioFileInput.click()`.
  - When files change, `handleLocalFilesSelected(fileList)` is invoked (`app.js:1613-1683`).
  - Drag-and-drop is bound to `#active-song-card` (`app.js:1741-1752`).
- **Shortcomings of Current Implementation**:
  1. Files are decoded strictly in the browser using `transcodeWavArrayBufferToPcm16IfFloat` and stored as Blob URLs in `State.songs[songId]`.
  2. No audio bytes or metadata are transmitted to the Axum backend.
  3. There is no multi-file reference management, no 20-file cap enforcement, no duration/sample rate badge display per reference track, and no trigger for the RL multi-WAV ensemble pipeline.

---

## 3. WebUI Frontend Implementation Plan

### 3.1 Layout & Component Placement in `index.html`
To provide an intuitive workflow, the Reference WAV Importer should be integrated into two prominent areas:
1. **New Song / RL Ensemble Composer** (inside `#new-song-chat-hero`, lines 305–420): A dedicated section titled `📂 Multi-File Reference WAV Importer (Up to 20 .wav Files)` right below the prompt input and above the generation button.
2. **Dedicated Reference WAV Studio Drawer** (inside `#active-song-card`, line 560): Allows importing or inspecting the active reference ensemble while working in the DAW view.
3. **Sidebar Quick Access** (`#chatgpt-sidebar`): A badge indicator `Reference WAVs [ 0 / 20 ]` that expands the importer.

### 3.2 HTML Markup Specification
```html
<!-- Multi-File Reference WAV Importer Container -->
<div class="ref-wav-importer-container" id="ref-wav-importer-container">
  <div class="ref-wav-importer-header">
    <div class="ref-header-left">
      <span class="ref-importer-icon">🎵</span>
      <div class="ref-importer-titles">
        <h3 class="ref-importer-title">Multi-File Reference WAV Importer</h3>
        <p class="ref-importer-subtitle">Import 1 to 20 reference .wav files to train the RL style ensemble with Legal Originality Guard.</p>
      </div>
    </div>
    <div class="ref-header-right">
      <!-- 20-File Cap Counter Badge -->
      <span class="ref-counter-badge" id="ref-wav-counter" aria-live="polite">0 / 20 Reference WAVs</span>
      <button type="button" class="btn-clear-refs" id="btn-clear-ref-wavs" title="Clear all uploaded reference WAVs" style="display:none;">
        🗑️ Clear All
      </button>
    </div>
  </div>

  <!-- Cap Warning Banner -->
  <div class="ref-cap-warning hidden" id="ref-wav-cap-warning" role="alert">
    ⚠️ <strong>20-File Limit Reached:</strong> Maximum of 20 reference WAV files allowed. Any additional files selected were ignored.
  </div>

  <!-- Drag & Drop Zone + Native Hidden File Input -->
  <div class="ref-wav-dropzone" id="ref-wav-dropzone" tabindex="0" role="button" aria-label="Drop 1 to 20 .wav files here or click to browse">
    <input type="file" id="ref-wav-file-input" accept=".wav,audio/wav" multiple style="display:none;">
    <div class="dropzone-content">
      <div class="dropzone-icon">📥</div>
      <div class="dropzone-text">
        <span class="dropzone-primary">Drag &amp; drop 1 to 20 reference <code>.wav</code> files here</span>
        <span class="dropzone-secondary">or <button type="button" class="btn-browse-ref-wavs" id="btn-browse-ref-wavs">Browse local files</button> (44.1 kHz / 48 kHz, 16/24/32-bit PCM/Float)</span>
      </div>
    </div>
  </div>

  <!-- Reference WAV Cards List / Grid -->
  <div class="ref-wav-grid" id="ref-wav-grid" aria-label="Uploaded Reference WAV Files">
    <!-- Populated dynamically: zero-empty state placeholder shown when count == 0 -->
    <div class="ref-empty-state" id="ref-empty-state">
      <span class="empty-icon">🎧</span>
      <span class="empty-text">No reference WAVs loaded yet. Upload up to 20 files to extract multi-file style features and enforce melodic originality.</span>
    </div>
  </div>

  <!-- RL Pipeline Generation Trigger & Settings Card -->
  <div class="ref-generate-card hidden" id="ref-generate-card">
    <div class="ref-generate-settings">
      <div class="setting-item">
        <label for="ref-originality-ceiling">🛡️ Legal Originality Guard Ceiling:</label>
        <select id="ref-originality-ceiling" class="version-select">
          <option value="0.25">Strict Legal (Max 25% single-file similarity)</option>
          <option value="0.35" selected>Studio Standard (Max 35% single-file similarity)</option>
          <option value="0.50">Permissive Style Blend (Max 50% single-file similarity)</option>
        </select>
      </div>
      <div class="setting-item">
        <label for="ref-blend-mode">🎛️ Multi-File Style Blending:</label>
        <select id="ref-blend-mode" class="version-select">
          <option value="balanced_ensemble" selected>Balanced Ensemble (Equal Weight across all files)</option>
          <option value="rhythm_priority">Rhythmic Priority (Extract groove from ref #1, harmony from others)</option>
          <option value="chroma_priority">Harmonic Priority (KS Chroma vector blend)</option>
        </select>
      </div>
    </div>
    <button type="button" class="btn-generate-ref-rl" id="btn-generate-ref-rl">
      ⚡ Generate Song with Multi-WAV RL Pipeline &amp; Originality Guard
    </button>
  </div>

  <!-- RL Execution Progress & Originality Compliance Report Card -->
  <div class="ref-results-card hidden" id="ref-results-card">
    <div class="rl-progress-stepper" id="ref-rl-stepper">
      <div class="rl-step" id="step-rvq">
        <span class="step-dot"></span>
        <span class="step-label">1. RVQ Tokenization (Up to 20 WAVs)</span>
      </div>
      <div class="rl-step" id="step-features">
        <span class="step-dot"></span>
        <span class="step-label">2. KS Chroma &amp; Rhythmic Feature Extraction</span>
      </div>
      <div class="rl-step" id="step-policy">
        <span class="step-dot"></span>
        <span class="step-label">3. RL Policy (PPO/Bandit + DPO) Blending</span>
      </div>
      <div class="rl-step" id="step-guard">
        <span class="step-dot"></span>
        <span class="step-label">4. Legal Originality / Anti-Plagiarism Verification</span>
      </div>
    </div>

    <!-- Verified Legal Originality & Compliance Score Card -->
    <div class="originality-report-box hidden" id="ref-originality-report">
      <div class="originality-header">
        <div class="score-badge-wrap">
          <span class="score-badge" id="originality-score-badge">🛡️ 98.6% Original</span>
          <span class="compliance-tag compliant" id="compliance-status-tag">✓ Legally Compliant</span>
        </div>
        <div class="score-summary-text" id="originality-summary-text">
          Zero melodic plagiarism detected. Maximum single-file overlap: 11.4% (well below the 35.0% ceiling).
        </div>
      </div>
      <div class="originality-file-matrix" id="originality-file-matrix">
        <!-- Rendered per-file comparison table -->
      </div>
    </div>
  </div>
</div>
```

### 3.3 Dynamic JavaScript Logic in `app.js`
- **State Additions**:
  ```javascript
  State.referenceWavs = []; // Array of { id, filename, duration_secs, sample_rate, channels, bit_depth, file_size_bytes }
  State.maxReferenceWavs = 20;
  State.isUploadingRefWavs = false;
  State.isGeneratingRefRl = false;
  ```
- **File Validation & Cap Enforcement**:
  ```javascript
  function handleReferenceFiles(files) {
    if (!files || files.length === 0) return;
    const currentCount = State.referenceWavs.length;
    const remainingSlots = State.maxReferenceWavs - currentCount;

    if (remainingSlots <= 0) {
      showCapWarning(true);
      return;
    }

    const validWavs = Array.from(files).filter(f => f.name.toLowerCase().endsWith('.wav') || f.type === 'audio/wav');
    if (validWavs.length === 0) {
      DOM.deckStatusMsg.textContent = '⚠️ Only .wav audio files are supported for Reference WAV Import.';
      return;
    }

    const acceptedFiles = validWavs.slice(0, remainingSlots);
    if (validWavs.length > remainingSlots) {
      showCapWarning(true);
    } else {
      showCapWarning(false);
    }

    uploadReferenceWavs(acceptedFiles);
  }
  ```
- **REST Calls in `app.js`**:
  - `uploadReferenceWavs(files)`: Posts `multipart/form-data` to `/api/reference-wavs/upload`.
  - `loadReferenceWavsList()`: Calls `GET /api/reference-wavs` on app load.
  - `deleteReferenceWav(id)`: Calls `DELETE /api/reference-wavs/${encodeURIComponent(id)}`.
  - `clearAllReferenceWavs()`: Calls `DELETE /api/reference-wavs`.
  - `generateRefRl()`: Calls `POST /api/reference-wavs/generate-rl`.

---

## 4. REST Server Architecture & Endpoints (`crates/music_cli/src/rest_server.rs`)

### 4.1 Current REST Server Setup
- The REST API is implemented with Axum 0.7 (`rest_server.rs:1628-1660`).
- App state is managed by `AppState`:
  ```rust
  #[derive(Clone)]
  pub struct AppState {
      pub runner: Arc<PipelineRunner>,
      pub storage_dir: PathBuf,
      pub bound_port: u16,
      pub ollama_host: String,
  }
  ```
- Static assets (`index.html`, `styles.css`, `app.js`) are dynamically served from `crates/music_cli/webui/`.

### 4.2 Data Models for Reference WAVs
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceWavEntry {
    pub id: String,
    pub filename: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: String,
    pub file_size_bytes: usize,
    pub uploaded_at: String,
    pub storage_path: String,
}

#[derive(Debug, Serialize)]
pub struct ListReferenceWavsResponse {
    pub total_count: usize,
    pub max_allowed: usize,
    pub files: Vec<ReferenceWavEntry>,
}

#[derive(Debug, Serialize)]
pub struct UploadReferenceWavsResponse {
    pub status: String,
    pub uploaded_count: usize,
    pub total_count: usize,
    pub files: Vec<ReferenceWavEntry>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateRefRlRequest {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub originality_threshold: Option<f32>, // Default 0.35 (max 35% single-file similarity)
    #[serde(default)]
    pub blend_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileSimilarityScore {
    pub file_id: String,
    pub filename: String,
    pub chroma_ngram_overlap: f32,
    pub waveform_cross_correlation: f32,
    pub combined_similarity: f32,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct OriginalityComplianceReport {
    pub originality_score_pct: f32, // e.g. 98.6%
    pub is_compliant: bool,
    pub max_single_file_overlap: f32, // e.g. 0.114
    pub threshold_ceiling: f32,       // e.g. 0.35
    pub per_file_scores: Vec<FileSimilarityScore>,
    pub audit_hash: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateRefRlResponse {
    pub status: String,
    pub song_id: String,
    pub version: u32,
    pub reference_count: usize,
    pub originality_report: OriginalityComplianceReport,
    pub audio_urls: HashMap<String, String>,
}
```

### 4.3 Reference WAV Session Manager
To maintain thread safety and avoid race conditions, `AppState` should be augmented with:
```rust
pub struct ReferenceWavSession {
    pub base_dir: PathBuf,
    pub entries: Vec<ReferenceWavEntry>,
}
```
In `AppState`:
```rust
#[derive(Clone)]
pub struct AppState {
    pub runner: Arc<PipelineRunner>,
    pub storage_dir: PathBuf,
    pub bound_port: u16,
    pub ollama_host: String,
    pub reference_session: Arc<tokio::sync::RwLock<ReferenceWavSession>>,
}
```

### 4.4 Endpoints Specification

#### 1. `POST /api/reference-wavs/upload`
- **Method**: `POST`
- **Content-Type**: `multipart/form-data` or JSON with base64 data for test suites.
- **Validation**:
  1. Parse each multipart field.
  2. Reject empty files (`0 bytes`) with `400 Bad Request`.
  3. Validate WAV header using `music_dsp::wav::AudioBuffer::from_wav_bytes(&bytes)`. If header signature (`RIFF`/`WAVE`), chunk size, or audio format is invalid, return `400 Bad Request` with exact error message.
  4. Enforce 20-file cap:
     ```rust
     let current_count = session.entries.len();
     if current_count + incoming_count > 20 {
         return (StatusCode::BAD_REQUEST, Json(json!({
             "status": "error",
             "error": format!("Cannot upload {} files: would exceed maximum cap of 20 reference WAV files (currently {} loaded)", incoming_count, current_count)
         }))).into_response();
     }
     ```
  5. Sanitize filename: Strip path separators, directory traversal tokens (`..`, `/`, `\`), and control characters.
  6. Save to `storage_dir.join("reference_wavs").join(format!("{}_{}", file_id, safe_name))`.
  7. Return `UploadReferenceWavsResponse` with `StatusCode::CREATED` or `StatusCode::OK`.

#### 2. `GET /api/reference-wavs`
- **Method**: `GET`
- **Response**: `ListReferenceWavsResponse` with HTTP 200:
  ```json
  {
    "total_count": 2,
    "max_allowed": 20,
    "files": [
      {
        "id": "ref_1730559000_1",
        "filename": "guitar_wah_skank.wav",
        "duration_secs": 12.4,
        "sample_rate": 48000,
        "channels": 2,
        "bit_depth": "16-bit PCM",
        "file_size_bytes": 2380844,
        "uploaded_at": "2026-11-02T14:40:00Z"
      }
    ]
  }
  ```

#### 3. `DELETE /api/reference-wavs`
- **Method**: `DELETE`
- **Behavior**: Clears all entries from the active session and deletes associated audio files from disk.
- **Response**:
  ```json
  {
    "status": "cleared",
    "removed_count": 2
  }
  ```

#### 4. `DELETE /api/reference-wavs/:id`
- **Method**: `DELETE`
- **Security Invariant**: Strictly validate `:id` using path traversal checks (`containment_ring_breakout_invariants.md`). Must not contain `..`, `/`, `\`, or `:`.
- **Behavior**: Removes matching file from session and deletes it from disk.
- **Error Handling**: Returns `404 Not Found` if ID does not exist.

#### 5. `POST /api/reference-wavs/generate-rl`
- **Method**: `POST`
- **Request Body**: `GenerateRefRlRequest`
- **Preconditions**:
  - `session.entries.len() >= 1` (reject 0 files with `400 Bad Request: At least 1 reference WAV file required`).
  - `session.entries.len() <= 20` (reject >20 files with `400 Bad Request: Cannot exceed 20 reference WAV files`).
- **Pipeline Execution**:
  1. Reads all active reference WAV buffers.
  2. Executes RVQ tokenization & acoustic analysis (Stage 1).
  3. Executes RL Policy (Stage 2/3) style blending.
  4. Runs the **Legal Originality / Anti-Plagiarism Guard**:
     - Computes N-gram chroma pitch overlap against every reference WAV individually.
     - Computes frame-level cosine similarity and waveform cross-correlation against every reference WAV individually.
     - Calculates `max_single_file_overlap`.
     - Validates that `max_single_file_overlap <= originality_threshold`.
     - Computes `originality_score_pct = (1.0 - max_single_file_overlap) * 100.0`.
  5. Creates song version checkpoint and synthesizes stems/master.
  6. Returns `GenerateRefRlResponse` with full originality audit report.

---

## 5. Security & Rule Compliance Matrix

| Rule / Invariant | Requirement | Delivery Strategy |
|---|---|---|
| `no_mocks_write_actual_code.md` | Real WAV parsing; NO hardcoded mock metadata. | Uses `music_dsp::wav::AudioBuffer::from_wav_bytes` to read actual RIFF headers, calculate real durations, detect sample rates, and extract real PCM samples. |
| `containment_ring_breakout_invariants.md` | Path traversal protection. | Sanitizes filenames and IDs; rejects `..`, drive letters, slashes, UNC paths, and NTFS streams. |
| `environment_safety.md` | Never mutate system/user PATH or environment. | All storage is confined to `storage_dir.join("reference_wavs")`. No environment mutation. |
| `rl_benchmarking_and_evaluation.md` | RL learning superiority and baseline separation. | `POST /api/reference-wavs/generate-rl` computes verifiable composite rewards and returns comparison metrics against baselines. |

---

## 6. Actionable Implementation Recommendations for Implementers

1. **`crates/music_cli/Cargo.toml`**:
   Add the `"multipart"` feature to axum:
   ```toml
   axum = { version = "0.7", features = ["macros", "multipart"] }
   ```
2. **`crates/music_cli/src/rest_server.rs`**:
   - Define `ReferenceWavEntry`, `ReferenceWavSession`, and request/response structs.
   - Add `reference_session: Arc<tokio::sync::RwLock<ReferenceWavSession>>` to `AppState`.
   - Implement handlers:
     - `handle_upload_reference_wavs`
     - `handle_list_reference_wavs`
     - `handle_clear_reference_wavs`
     - `handle_delete_reference_wav`
     - `handle_generate_ref_rl`
   - Register routes in `create_router()`:
     ```rust
     .route("/api/reference-wavs", get(handle_list_reference_wavs).delete(handle_clear_reference_wavs))
     .route("/api/reference-wavs/upload", post(handle_upload_reference_wavs))
     .route("/api/reference-wavs/:id", delete(handle_delete_reference_wav))
     .route("/api/reference-wavs/generate-rl", post(handle_generate_ref_rl))
     ```
3. **`crates/music_cli/webui/index.html`**:
   - Insert `#ref-wav-importer-container` into the main UI layout.
   - Include the native file picker `<input type="file" id="ref-wav-file-input" multiple accept=".wav,audio/wav">`.
   - Include `#ref-wav-dropzone`, `#ref-wav-counter`, `#ref-wav-cap-warning`, `#ref-wav-grid`, and `#ref-generate-card`.
4. **`crates/music_cli/webui/app.js`**:
   - Cache elements in `cacheDOMElements()`.
   - Add `setupReferenceWavImporter()` with drag-and-drop listeners, file input change handler, and dynamic rendering functions.
   - Enforce 20-file limit in client logic and display visual counter/warning.
   - Wire `POST /api/reference-wavs/generate-rl` to display the verified **Originality & Legal Compliance Score** card.
5. **`crates/music_cli/webui/styles.css`**:
   - Add styles for `.ref-wav-dropzone`, `.ref-wav-grid`, `.ref-wav-card`, `.ref-counter-badge`, and `.originality-report-box`.
6. **Integration Tests (`crates/music_cli/tests/cli_and_rest_tests.rs`)**:
   - Test uploading 1..=20 valid WAV files.
   - Test rejecting >20 files with HTTP 400.
   - Test rejecting invalid/corrupt WAV header with HTTP 400.
   - Test `GET /api/reference-wavs`, `DELETE /api/reference-wavs/:id`, and `DELETE /api/reference-wavs`.
   - Test `POST /api/reference-wavs/generate-rl` verifying the returned Originality Score and zero single-file plagiarism guarantee.
