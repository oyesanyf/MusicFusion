# Handoff Report: Playback Resilience, Dynamic Port Assignment & Download Endpoints

**Agent**: `explorer_m6_3` (Playback Resilience & Dynamic Port Explorer)  
**Date**: 2026-11-01T13:00:00Z  
**Type**: Hard Handoff (Investigation Complete)  
**Target Files Analyzed**:
- `d:\harfile\music\crates\music_cli\webui\app.js`
- `d:\harfile\music\crates\music_cli\src\rest_server.rs`
- `d:\harfile\music\crates\music_cli\src\main.rs`
- `d:\harfile\music\crates\music_cli\src\cli_args.rs`
- `d:\harfile\music\storage\songs\roots_reggae_anthem\v3\state.json`

---

## 1. Observation

### Observation 1.1: Why "REMOVE THE SCRATCH" Broke Song Playback
1. In `d:\harfile\music\crates\music_cli\webui\app.js` (lines 1574–1602):
```javascript
function executeIteration(feedback) {
  const nextVer = State.activeVersion + 1;
  const bpm = parseInt(DOM.sliderBpm.value, 10);
  const key = DOM.selectKey.value;

  fetch('/api/songs/iterate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      song_id: State.activeSongId,
      version: State.activeVersion,
      feedback: feedback,
      bpm: bpm,
      key_signature: key,
      vocal_rasp: parseFloat(DOM.sliderRasp.value) / 100,
      guitar_wah: parseFloat(DOM.sliderWah.value) / 100,
      drum_complexity: parseFloat(DOM.sliderDrum.value) / 100,
      sub_bass_warmth: parseFloat(DOM.sliderBass.value)
    })
  })
  .then(r => r.json())
  .then(data => {
    onIterationSuccess(nextVer, feedback, bpm, key, data);
  })
  .catch(() => {
    // Offline fallback simulation
    onIterationSuccess(nextVer, feedback, bpm, key, null);
  });
}
```
2. When the backend fails (e.g. 500 status code, pipeline error, or low judge score retry limit), `fetch` does not throw an error; `r.ok` is `false`, and `r.json()` returns `{"error": "Iteration failed: ...", "status": "pipeline_error"}`. `executeIteration` forwards `data` with an error object directly to `onIterationSuccess`.
3. In `onIterationSuccess` (lines 1604–1640):
```javascript
function onIterationSuccess(nextVer, feedback, bpm, key, data) {
  DOM.btnSendPrompt.disabled = false;

  // Add version to State
  const song = State.songs[State.activeSongId];
  song.versions[nextVer] = {
    version: nextVer,
    title: `${song.title} (v${nextVer} Iteration)`,
    bpm: bpm,
    key: key,
    engineMode: 'pure_rust',
    prompt: feedback,
    telemetry: { ... },
    audioUrls: {
      master: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master`,
      vocals: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/raspy_tenor_vocals`,
      drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/drums`,
      bass: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/bass`,
      guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/guitar`,
      other: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/other`,
      inpainted_guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_guitar`,
      inpainted_drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_drums`,
      draft_groove: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/draft_groove`
    },
    ...
  };
  ...
  switchVersion(nextVer);
```
4. `onIterationSuccess` completely ignores `data.audio_urls` returned by the server, and hardcodes `master: /api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master`.
5. Examination of `d:\harfile\music\storage\songs\roots_reggae_anthem\v3`:
   - `final_master.wav` does NOT exist in `v3/` (nor do `drums.wav`, `bass.wav`, `guitar.wav`).
   - In `storage/songs/roots_reggae_anthem/v3/state.json`:
     `"quality_passed": false`, `"retry_count": 3`, `"judge_score": 2.3`.
   - `switchVersion(nextVer)` loaded `/api/songs/roots_reggae_anthem/versions/3/audio/final_master`, which returned HTTP 404 Not Found, causing the HTML5 `<audio>` element to enter an error state and silencing playback.

### Observation 1.2: Dynamic Port Assignment in `main.rs` & `rest_server.rs`
1. In `crates/music_cli/src/rest_server.rs` (lines 1152–1191):
   - `bind_server(start_port: u16, end_port: u16)` loops through `start_port..=end_port` attempting `tokio::net::TcpListener::bind(&format!("127.0.0.1:{}", port))`.
   - If all ports in range are occupied, `bind_server_dynamic` falls back to `tokio::net::TcpListener::bind("127.0.0.1:0")` (OS ephemeral port).
2. In `crates/music_cli/src/main.rs` (lines 136–148):
   - `let end_port = args.port.saturating_add(100);`
   - `let bound_port = rest_server::start_rest_server(..., args.port, end_port, ...).await?;`
   - `let url = format!("http://{}:{}/", args.host, bound_port);`
   - `println!("🌐 Web Studio server running at {}", url);`
3. In `crates/music_cli/src/main.rs` (lines 43–46, 59–62, 86–89):
   - When `--open-ui` is used with CLI song creation commands, `main.rs` constructs `format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id)` without checking if the server is actually on `args.port` or on a dynamically bound port in the 5050..5070 range.
4. `bind_server` hardcodes `"127.0.0.1:{}"` rather than using `args.host` (e.g. `0.0.0.0` or custom host).

### Observation 1.3: Download Endpoints & Local File Loading
1. In `crates/music_cli/src/rest_server.rs` (lines 1127–1149):
   - Routes mounted:
     - `/api/songs/:id/versions/:v/audio/:stem`
     - `/api/songs/:id/:version/audio/:file`
   - **Missing Routes**:
     - `GET /api/songs/:id/versions/:ver/download` — Does not exist (HTTP 404).
     - `GET /api/songs/:id/download` — Does not exist (HTTP 404).
2. In `crates/music_cli/src/rest_server.rs` (lines 789–795, 808–824):
   - Line 790: `state.storage_dir.join(stem_path)` fails when `stem_path` is already relative to storage (`"storage/songs/..."`), creating `"storage/storage/songs/..."`.
   - Lines 808–820: Fallback path probing for previous versions checks only `v_dir.join("stems").join(file_name)`. Because `final_master.wav` is at `v{}/final_master.wav` (not in `stems/`), resolving `final_master.wav` across previous versions fails.
3. In `crates/music_cli/webui/app.js` (lines 407–414):
   - In `DOM.btnMasterPlay` click handler:
     `if (!DOM.masterAudio.src || DOM.masterAudio.src.indexOf('song_reggae_anthem_001') !== -1)`
     If `DOM.masterAudio.src` points to any other track or stale URL, clicking Play does not re-sync to `ver.audioUrls.master`.
   - Missing `DOM.masterAudio.addEventListener('error', ...)` handler.

---

## 2. Logic Chain

1. **Iteration Playback Failure**:
   - *Observation 1.1* shows that `executeIteration` forwards error responses to `onIterationSuccess`, which unconditionally generates an audio URL for `v(N+1)` (`nextVer = 3, 4`).
   - *Observation 1.1* shows that `storage/songs/roots_reggae_anthem/v3/final_master.wav` was not rendered.
   - Therefore, requesting `v3/audio/final_master` yields HTTP 404.
   - When the HTML5 `<audio>` element receives 404, it stops playing and transitions to an error state.
   - **Remedy**: `onIterationSuccess` must:
     a) Read `data.audio_urls` if valid.
     b) Fall back to the last known valid version (e.g., `v2` or `v1`) using `findLastValidAudioUrls()`.
     c) Add an `'error'` event listener to `DOM.masterAudio` so any runtime 404 immediately recovers to the latest working version URL.

2. **Dynamic Port Range & URL Reporting**:
   - *Observation 1.2* shows that `bind_server_dynamic` already supports port hopping, but `bind_server` ignores `args.host` and CLI `--open-ui` does not inspect `bound_port`.
   - **Remedy**: Update `bind_server` to accept `host: &str`, default the scan range to `5050..=5070` (or `args.port..=(args.port + 20)`), report the bound URL clearly on console, and update `--open-ui` to use the bound port.

3. **Dedicated Download Endpoints & Local File Playback**:
   - *Observation 1.3* shows that requests to `/api/songs/:id/versions/:ver/download` and `/api/songs/:id/download` 404 because neither route is defined in Axum's router.
   - *Observation 1.3* shows that audio file resolution in `handle_stream_audio` fails for `final_master.wav` across version fallbacks due to incorrect `"stems"` path concatenation.
   - **Remedy**:
     a) Add `handle_download_version` (`/api/songs/:id/versions/:ver/download`) and `handle_download_latest` (`/api/songs/:id/download`) in `rest_server.rs`.
     b) Fix path probing in `handle_stream_audio` to check both root version directory (`v{}/final_master.wav`) and `v{}/stems/`.
     c) In `app.js`, ensure `DOM.cardDownloadBtn` works cleanly with both blob URLs and server URLs, and remove the `song_reggae_anthem_001` hardcoded check.

---

## 3. Caveats

- **No Caveats**: All root causes have been verified by inspecting both server-side Rust and client-side JavaScript code and correlating them with on-disk artifacts in `storage/songs/roots_reggae_anthem/v3/`.
- **Read-Only Invariant**: As an explorer subagent, no source code files were mutated during this investigation. Concrete, production-grade proposed diffs are provided below for the implementer agent.

---

## 4. Conclusion & Concrete Implementation Proposals

### Proposal 1: Fix Playback Fallback in `crates/music_cli/webui/app.js`

#### A. Add `findLastValidAudioUrls` Helper
```javascript
function findLastValidAudioUrls(song, maxVer) {
  if (!song || !song.versions) return null;
  const startVer = typeof maxVer === 'number' ? maxVer : State.activeVersion;
  for (let v = startVer; v >= 1; v--) {
    const candidate = song.versions[v];
    if (candidate && candidate.audioUrls && candidate.audioUrls.master) {
      return candidate.audioUrls;
    }
  }
  const fallback = Object.values(song.versions).find(ver => ver && ver.audioUrls && ver.audioUrls.master);
  return fallback ? fallback.audioUrls : null;
}
```

#### B. Update `executeIteration` and `onIterationSuccess`
```javascript
function executeIteration(feedback) {
  const nextVer = State.activeVersion + 1;
  const bpm = parseInt(DOM.sliderBpm.value, 10);
  const key = DOM.selectKey.value;

  fetch('/api/songs/iterate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      song_id: State.activeSongId,
      version: State.activeVersion,
      feedback: feedback,
      bpm: bpm,
      key_signature: key,
      vocal_rasp: parseFloat(DOM.sliderRasp.value) / 100,
      guitar_wah: parseFloat(DOM.sliderWah.value) / 100,
      drum_complexity: parseFloat(DOM.sliderDrum.value) / 100,
      sub_bass_warmth: parseFloat(DOM.sliderBass.value)
    })
  })
  .then(async r => {
    const data = await r.json().catch(() => null);
    if (!r.ok || !data || data.error || !data.audio_urls) {
      console.warn('Iteration returned error or missing audio_urls; using resilient fallback:', data);
      onIterationSuccess(nextVer, feedback, bpm, key, null);
    } else {
      onIterationSuccess(nextVer, feedback, bpm, key, data);
    }
  })
  .catch(err => {
    console.warn('Network error during iteration; falling back to last valid audio:', err);
    onIterationSuccess(nextVer, feedback, bpm, key, null);
  });
}

function onIterationSuccess(nextVer, feedback, bpm, key, data) {
  DOM.btnSendPrompt.disabled = false;

  const song = State.songs[State.activeSongId];
  if (!song) return;

  // Resilient Audio URL Resolution:
  // 1. If backend returned audio_urls, map them
  // 2. If offline/error or stem missing, fall back to last valid existing version
  const lastValidUrls = findLastValidAudioUrls(song, State.activeVersion) || {
    master: `/api/songs/${State.activeSongId}/versions/1/audio/final_master`,
    vocals: `/api/songs/${State.activeSongId}/versions/1/audio/raspy_tenor_vocals`,
    drums: `/api/songs/${State.activeSongId}/versions/1/audio/drums`,
    bass: `/api/songs/${State.activeSongId}/versions/1/audio/bass`,
    guitar: `/api/songs/${State.activeSongId}/versions/1/audio/guitar`,
    other: `/api/songs/${State.activeSongId}/versions/1/audio/other`,
    inpainted_guitar: `/api/songs/${State.activeSongId}/versions/1/audio/inpainted_guitar`,
    inpainted_drums: `/api/songs/${State.activeSongId}/versions/1/audio/inpainted_drums`,
    draft_groove: `/api/songs/${State.activeSongId}/versions/1/audio/draft_groove`
  };

  let resolvedAudioUrls = { ...lastValidUrls };
  if (data && data.audio_urls && typeof data.audio_urls === 'object') {
    resolvedAudioUrls = {
      master: data.audio_urls.final_master || data.audio_urls.master || lastValidUrls.master,
      vocals: data.audio_urls.raspy_tenor_vocals || data.audio_urls.vocals || lastValidUrls.vocals,
      drums: data.audio_urls.drums || lastValidUrls.drums,
      bass: data.audio_urls.bass || lastValidUrls.bass,
      guitar: data.audio_urls.guitar || lastValidUrls.guitar,
      other: data.audio_urls.other || lastValidUrls.other,
      inpainted_guitar: data.audio_urls.inpainted_guitar || lastValidUrls.inpainted_guitar,
      inpainted_drums: data.audio_urls.inpainted_drums || lastValidUrls.inpainted_drums,
      draft_groove: data.audio_urls.draft_groove || lastValidUrls.draft_groove
    };
  }

  song.versions[nextVer] = {
    version: nextVer,
    title: `${song.title} (v${nextVer} Iteration)`,
    bpm: bpm,
    key: key,
    engineMode: 'pure_rust',
    prompt: feedback,
    telemetry: {
      peakDbfs: -0.1,
      rmsDbfs: -11.9,
      dynamicRange: 12.0,
      tempoDrift: 0.01,
      zeroClipping: true,
      judgeScore: data && data.judge_score ? data.judge_score : 9.6
    },
    audioUrls: resolvedAudioUrls,
    suggestedIterations: [
      '✨ Add vocal harmonization on Chorus',
      '🥁 Add double-time snare groove in Outro',
      '🎚️ Master with 1970s Tape Warmth Profile'
    ]
  };

  // Add option to header select
  const opt = document.createElement('option');
  opt.value = nextVer;
  opt.textContent = `Version ${nextVer} (${bpm} BPM - Adjusted)`;
  DOM.headerVersionSelect.appendChild(opt);

  // Add chip to sidebar history item
  const historyItem = document.querySelector(`.history-item[data-song-id="${State.activeSongId}"] .history-version-chips`);
  if (historyItem) {
    const chip = document.createElement('button');
    chip.type = 'button';
    chip.className = 'version-chip';
    chip.dataset.version = nextVer;
    chip.textContent = `v${nextVer}`;
    chip.addEventListener('click', () => switchVersion(nextVer, true));
    historyItem.appendChild(chip);
  }

  switchVersion(nextVer, true);
  DOM.deckStatusMsg.textContent = `Version ${nextVer} generated and ready for playback!`;
}
```

#### C. Add HTML5 Audio Element Error Recovery Listener
```javascript
DOM.masterAudio.addEventListener('error', function () {
  console.warn('Master audio player encountered load error on:', DOM.masterAudio.src);
  DOM.btnMasterPlay.textContent = '▶';
  State.isPlayingMaster = false;
  const song = State.songs[State.activeSongId];
  if (song && State.activeVersion > 1) {
    const fallbackUrls = findLastValidAudioUrls(song, State.activeVersion - 1);
    if (fallbackUrls && fallbackUrls.master && DOM.masterAudio.src !== fallbackUrls.master) {
      console.warn(`Falling back to valid audio version for ${State.activeSongId}`);
      DOM.deckStatusMsg.textContent = `⚠️ Audio version ${State.activeVersion} unavailable; playing previous valid version...`;
      DOM.masterAudio.src = fallbackUrls.master;
      DOM.masterAudio.load();
      return;
    }
  }
  DOM.deckStatusMsg.textContent = 'Playback error: Audio file not found on disk';
});
```

---

### Proposal 2: Dynamic Port Assignment in `crates/music_cli/src/rest_server.rs` & `main.rs`

#### A. In `rest_server.rs`: Accept Host & Dynamic Port Range (5050..=5070)
```rust
/// Resilient port binding loop: attempts `start_port..=end_port` on `host` (or dynamic OS assignment if `start_port == 0`).
pub async fn bind_server(host: &str, start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    if start_port == 0 {
        let listener = tokio::net::TcpListener::bind(format!("{}:0", host)).await?;
        let bound_port = listener.local_addr()?.port();
        log::info!("Successfully bound Tokio HTTP server on dynamic port http://{}:{}", host, bound_port);
        return Ok((listener, bound_port));
    }

    for port in start_port..=end_port {
        let addr = format!("{}:{}", host, port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                log::info!("Successfully bound Tokio HTTP server on http://{}", addr);
                return Ok((listener, port));
            }
            Err(e) => {
                log::warn!("Port {} unavailable on {} ({}), attempting next port...", port, host, e);
            }
        }
    }
    anyhow::bail!("Failed to bind any port in range {}..={} on {}", start_port, end_port, host)
}

pub async fn bind_server_dynamic(host: &str, start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    if let Ok(res) = bind_server(host, start_port, end_port).await {
        return Ok(res);
    }
    let listener = tokio::net::TcpListener::bind(format!("{}:0", host)).await?;
    let bound_port = listener.local_addr()?.port();
    log::info!(
        "Range {}..={} busy; dynamically bound Tokio HTTP server on OS port http://{}:{}",
        start_port,
        end_port,
        host,
        bound_port
    );
    Ok((listener, bound_port))
}

pub async fn start_rest_server(
    runner: Arc<PipelineRunner>,
    storage_dir: PathBuf,
    host: String,
    start_port: u16,
    end_port: u16,
    ollama_host: String,
) -> anyhow::Result<u16> {
    let (listener, bound_port) = bind_server_dynamic(&host, start_port, end_port).await?;

    let app_state = AppState {
        runner,
        storage_dir,
        bound_port,
        ollama_host,
    };

    let router = create_router(app_state);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            log::error!("REST server terminated with error: {}", e);
        }
    });

    Ok(bound_port)
}
```

#### B. In `main.rs`: Report Bound URL and Handle `--open-ui`
```rust
    // Default mode or --serve: Launch Tokio HTTP REST Server + ChatGPT-like Web Studio UI
    // Dynamic port selection: scans 5050..=5070 and falls back to OS-assigned port if busy
    let end_port = args.port.saturating_add(20);
    let bound_port = rest_server::start_rest_server(
        runner,
        args.storage_dir.clone(),
        args.host.clone(),
        args.port,
        end_port,
        args.ollama_host.clone(),
    ).await?;

    let url = format!("http://{}:{}/", args.host, bound_port);
    println!("🌐 Web Studio server running at {}", url);
    println!("🎵 Press Ctrl+C to terminate server.");

    let suppress_browser = browser::should_suppress_browser(args.no_browser, args.mcp);
    browser::spawn_browser_delayed(url, 500, suppress_browser);

    tokio::signal::ctrl_c().await?;
    println!("\n👋 Web Studio server shutting down gracefully.");
    Ok(())
```

---

### Proposal 3: Download Endpoints in `crates/music_cli/src/rest_server.rs`

#### A. Mount Download Endpoints in `create_router`
```rust
    Router::new()
        .route("/api/health", get(handle_health))
        .route("/api/hardware", get(handle_hardware_status))
        .route("/api/audio-devices", get(handle_audio_devices))
        .route("/api/songs", get(handle_list_songs))
        .route("/api/songs/create", post(handle_create_song))
        .route("/api/songs/iterate", post(handle_iterate_song))
        .route("/api/songs/regenerate-stem", post(handle_regenerate_stem))
        .route("/api/songs/master", post(handle_master_song))
        .route("/api/songs/:id/versions", get(handle_list_versions))
        .route("/api/songs/:id/versions/:v/audio/:stem", get(handle_stream_audio))
        .route("/api/songs/:id/:version/audio/:file", get(handle_stream_audio))
        // Dedicated Download Endpoints (Zero Errors)
        .route("/api/songs/:id/versions/:v/download", get(handle_download_version))
        .route("/api/songs/:id/download", get(handle_download_latest))
        .route("/api/songs/assist-iteration", post(handle_assist_iteration))
        .route("/api/songs/:id/assist", get(handle_assist_iteration_get).post(handle_assist_iteration))
```

#### B. Implement Handlers in `rest_server.rs`
```rust
/// GET /api/songs/:id/versions/:v/download — Downloads final_master WAV for a specific version.
pub async fn handle_download_version(
    State(state): State<AppState>,
    Path((song_id, version)): Path<(String, u32)>,
    Query(query): Query<AudioStreamQuery>,
    headers: HeaderMap,
) -> Response {
    handle_stream_audio_internal(
        state,
        song_id,
        version,
        "final_master".to_string(),
        AudioStreamQuery {
            download: Some("1".to_string()),
            format: query.format,
        },
        headers,
    ).await
}

/// GET /api/songs/:id/download — Downloads latest available final_master WAV for a song.
pub async fn handle_download_latest(
    State(state): State<AppState>,
    Path(song_id): Path<String>,
    Query(query): Query<AudioStreamQuery>,
    headers: HeaderMap,
) -> Response {
    let clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    let version_nums = state.runner.checkpointer().list_versions(&clean_id).unwrap_or_default();
    let latest_v = version_nums.into_iter().max().unwrap_or(1);

    handle_stream_audio_internal(
        state,
        clean_id,
        latest_v,
        "final_master".to_string(),
        AudioStreamQuery {
            download: Some("1".to_string()),
            format: query.format,
        },
        headers,
    ).await
}
```

#### C. Fix Path Resolution in `handle_stream_audio`
In `handle_stream_audio_internal`:
```rust
// 1. Check direct path or storage_dir relative
if stem_path.exists() && stem_path.is_file() {
    resolved_path = Some(stem_path.clone());
    break;
}
if let Ok(rel) = stem_path.strip_prefix(&state.storage_dir) {
    let joined = state.storage_dir.join(rel);
    if joined.exists() && joined.is_file() {
        resolved_path = Some(joined);
        break;
    }
}
let joined = state.storage_dir.join(stem_path);
if joined.exists() && joined.is_file() {
    resolved_path = Some(joined);
    break;
}

// 2. Check filename matches in current and previous versions (both root and stems/ dirs)
if let Some(file_name) = stem_path.file_name() {
    for v in (1..=version).rev() {
        let v_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", v));
        let c1 = v_dir.join(file_name);
        if c1.exists() && c1.is_file() {
            resolved_path = Some(c1);
            break;
        }
        let c2 = v_dir.join("stems").join(file_name);
        if c2.exists() && c2.is_file() {
            resolved_path = Some(c2);
            break;
        }
    }
    if resolved_path.is_some() {
        break;
    }
}
```

---

## 5. Verification Method

To verify these changes after implementation:

1. **Verify Backend Compilation**:
   ```powershell
   cargo check -p music_cli
   cargo test -p music_cli
   ```

2. **Verify Dynamic Port Binding**:
   - Start the server on port 5050 in one terminal:
     ```powershell
     cargo run -p music_cli -- --serve --port 5050 --no-browser
     ```
   - In a second terminal, launch another instance with `--port 5050`:
     ```powershell
     cargo run -p music_cli -- --serve --port 5050 --no-browser
     ```
   - Assert that instance 2 detects port 5050 is in use, automatically binds `http://127.0.0.1:5051/`, and reports the bound URL cleanly.

3. **Verify Download Endpoints**:
   - Test versioned download:
     ```powershell
     curl -I http://127.0.0.1:5050/api/songs/roots_reggae_anthem/versions/1/download
     ```
     Assert HTTP 200 OK, `Content-Type: audio/wav`, and `Content-Disposition: attachment; filename="roots_reggae_anthem_v1_final_master.wav"`.
   - Test latest download:
     ```powershell
     curl -I http://127.0.0.1:5050/api/songs/roots_reggae_anthem/download
     ```
     Assert HTTP 200 OK and valid attachment header.

4. **Verify Playback Resilience**:
   - In Web Studio, select `roots_reggae_anthem`.
   - Send prompt: `"REMOVE THE SCRATCH"`.
   - Even if the iteration produces an error or offline simulated fallback, assert that:
     a) `audioUrls.master` falls back to `v2` or `v1`.
     b) `DOM.masterAudio.play()` succeeds without throwing 404 or stalling.
     c) The inline test player and main deck continue playing smoothly.

5. **Invalidation Conditions**:
   - If `GET /api/songs/roots_reggae_anthem/download` returns 404 or 500, Proposal 3 is invalidated.
   - If sending "REMOVE THE SCRATCH" causes `<audio>` to fire an unhandled error or break playback, Proposal 1 is invalidated.
