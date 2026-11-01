# Handoff Report — Explorer M5.5-3: Daemon CLI, REST Endpoints & Web UI Re-Cache Control

## 1. Observation

Direct examination of `crates/quorum-daemon` and `crates/quorum-ui` reveals the current architecture and integration hooks:

1. **CLI Commands (`crates/quorum-daemon/src/cli.rs:47-78`)**:
   ```rust
   #[derive(Subcommand, Debug, Clone)]
   pub enum Commands {
       /// Run standalone reindex worker process on designated directories
       Reindex { ... },
       /// Run standalone drift healing process repairing all 5 drift types
       FixDrift { ... },
   }
   ```
   Currently, only `Reindex` and `FixDrift` are declared as subcommands. There is no `Recache` subcommand.

2. **CLI Main Dispatcher (`crates/quorum-daemon/src/main.rs:28-44`)**:
   ```rust
   match &cli.command {
       Some(Commands::Reindex { path, full, force_reembed, batch_size }) => {
           run_standalone_reindex(cli.clone(), path.clone(), *full, *force_reembed, *batch_size).await?;
       }
       Some(Commands::FixDrift { dry_run, drift_type }) => {
           run_standalone_fix_drift(cli.clone(), *dry_run, drift_type.clone()).await?;
       }
       None => {
           run_daemon_and_ui(cli).await?;
       }
   }
   ```
   Standalone operations follow a dedicated `run_standalone_*` function pattern, instantiating required engines and reporting formatted execution metrics to stdout.

3. **Axum 0.7 Web Server Router (`crates/quorum-daemon/src/server.rs:12-49`)**:
   ```rust
   pub fn build_router(state: Arc<DaemonState>) -> axum::Router {
       axum::Router::new()
           .route("/", get(serve_index))
           .route("/assets/*path", get(serve_asset))
           .route("/favicon.ico", get(serve_favicon))
           .route("/api/chat", post(chat::chat_sse_handler))
           .route("/api/chat/stream", get(chat_stream_compat_handler))
           .route("/api/query", post(chat::query_handler))
           .route("/api/reindex", post(reindex::start_reindex_handler))
           .route("/api/reindex/status", get(reindex::reindex_status_handler))
           .route("/api/reindex/cancel", post(reindex::cancel_reindex_handler))
           .route("/api/drift/fix", post(drift::fix_drift_handler))
           .route("/api/drift/status", get(drift::drift_status_handler))
           .route("/api/langextract", post(langextract::extract_handler))
           .route("/api/langextract/visualize", post(langextract::visualize_handler))
           .route("/api/health", get(health::health_handler))
           .route("/api/models", get(health::models_handler))
           .route("/api/shutdown", post(shutdown_handler))
           .fallback(quorum_ui::static_handler)
           .layer(CorsLayer::permissive())
           .with_state(state)
   }
   ```
   No cache endpoints are currently registered. The daemon state (`crates/quorum-daemon/src/state.rs`) holds `pub storage: Arc<StorageEngine>` and `pub coordinator: Arc<QueryCoordinator>`.

4. **SSE Streaming & Query Handler (`crates/quorum-daemon/src/handlers/chat.rs`)**:
   - `chat_sse_handler` spawns an asynchronous task executing `coordinator.query_agentic(&prompt, family, top_k, Some(tx.clone())).await`.
   - `AgenticTelemetryEvent` stream currently emits `StateTransition`, `IntentClassified`, `HardwareProbed`, `RetrievalCompleted`, `RelevanceGraded`, `QueryRewritten`, `SynthesisStarted`, `TokenDelta`, `CitationEmitted`, `ExecutionFinished`, and `ExecutionError`.
   - Neither `chat_sse_handler` nor `query_handler` exposes or handles cache telemetry (`cache_hit`, `cache_epoch`, `hits`, `misses`).

5. **Health Endpoint (`crates/quorum-daemon/src/handlers/health.rs:41-65`)**:
   - Returns JSON containing `status`, `version`, `contract_hash`, `uptime_secs`, `storage`, `hardware`, `network`, and `active_model_family`.
   - Lacks a `"cache"` summary block.

6. **ChatGPT Web UI (`crates/quorum-ui/assets/index.html`, `app.js`, `styles.css`)**:
   - `index.html` has a 260px sidebar (`.sidebar-260px`) and centered 768px reading column (`.reading-col-768px`).
   - Sidebar contains navigation buttons for `Reindex Engine` and `Drift Remediation` under `<div class="sidebar-section">`.
   - Top navigation (`.top-nav`) contains a model selector pill bar, 3-way search mode toggle (`BM25 Keyword`, `Semantic Vector`, `Hybrid RRF`), and a 5-theme switcher dropdown (`theme-obsidian`, `theme-dark`, `theme-white`, `theme-midnight`, `theme-warm`).
   - UI currently has no "Re-Cache" button, no live cache stats badge, and no cache hit indicator in chat messages.

---

## 2. Logic Chain

1. **Dual-Mode CLI Operation (`quorumrag recache [--clear] [--warm]`)**:
   - When users execute `quorumrag recache`, the command must work reliably whether the daemon process is actively running or offline.
   - If a daemon is actively listening on `cli.host:cli.port` (default `127.0.0.1:3030`), the CLI client should dispatch an HTTP `POST /api/cache/recache` to ensure the live in-memory cache and atomic epoch counter in the running daemon are invalidated and warmed.
   - If the daemon is offline, the CLI worker must initialize `StorageEngine` and `QueryCoordinator` in standalone mode, increment the persistent `index_epoch`, purge cached entries if `--clear` was specified, pre-warm hot embeddings/queries if `--warm` was specified, and report results cleanly.

2. **REST Endpoints Architecture (`POST /api/cache/recache` & `GET /api/cache/stats`)**:
   - `POST /api/cache/recache` accepts optional JSON `{ "clear": bool, "warm": bool }` (defaulting to false).
   - Calling recache triggers:
     - Full eviction of LRU/hash maps if `clear == true`, or soft invalidation via monotonic increment of `cache_epoch`.
     - Invalidation of cached `QueryCache` entries whose epoch < new `cache_epoch`.
     - Optional pre-warming (`warm == true`) that computes and populates embeddings for top-ranked chunks and frequent queries.
     - Returns `{ status: "ok", cache_epoch, cleared_entries, warmed_entries, duration_ms }`.
   - `GET /api/cache/stats` queries the coordinator's caching engine and returns `{ status: "ok", hits, misses, hit_rate, cached_queries, cached_embeddings, cache_epoch }`.
   - `GET /api/health` embeds the cache summary directly inside the health payload, maintaining backward compatibility while giving monitoring probes instant cache visibility.

3. **SSE Telemetry Protocol & One-Shot Query**:
   - In `/api/chat`, `AgenticTelemetryEvent::CacheTelemetry` emits `{ cache_hit: bool, cache_hits: u64, cache_misses: u64, cached_entries: usize, cache_epoch: u64 }`.
   - On a cache hit during `ClassifyIntent`, the coordinator short-circuits the retrieval and synthesis hops, emits the cache telemetry event immediately, and streams the cached response without redundant neural inference.
   - In `/api/query`, `QueryResult` is extended to include `cache_hit: bool` and `cache_epoch: u64`.

4. **ChatGPT Web UI Ergonomics & 5-Theme Invariant**:
   - The user interface must provide immediate visibility and manual control:
     - **Header Quick Action**: A dedicated "⚡ Re-Cache" button next to a live hit-rate badge (`Cache: 85% (120 hits)`).
     - **Sidebar Subsystem Item**: A `⚡ Re-Cache Engine` button under Subsystems, opening a detailed modal with `Clear All` and `Pre-warm` checkboxes, live statistics counters, and execution feedback.
     - **Message Stream Badge**: When an assistant response was resolved from cache, a green/accent pill (`⚡ Cache Hit (Epoch N)`) renders in the message header.
   - The design must be styled across all 5 themes (`theme-obsidian`, `theme-dark`, `theme-white`, `theme-midnight`, `theme-warm`), adhering strictly to ChatGPT layout invariants (260px sidebar, 768px reading column).

---

## 3. Caveats

- **Network Dependency for CLI**: In active daemon mode, `quorumrag recache` makes an HTTP request to `http://127.0.0.1:3030`. The client uses a fast timeout (500ms) to detect daemon absence and cleanly fall back to standalone offline mode without blocking the user.
- **LanceDB / Tantivy Concurrency**: In standalone mode, if another process holds an exclusive lock on LanceDB or Tantivy directories, `StorageEngine::open` might wait or return an error. The CLI gracefully surfaces this condition with a clear actionable message.
- **ZeroizeOnDrop Memory Compliance**: When pre-warming or retrieving unpinned remote peer chunks, volatile memory containers (`VolatileRemoteChunk`) must be preserved and not cached permanently in plaintext. Only local chunk hashes and synthesized responses under valid access tiers are stored.

---

## 4. Conclusion & Proposed Implementation Specifications

### 4.1. CLI Subcommand (`crates/quorum-daemon/src/cli.rs` & `main.rs`)

#### In `crates/quorum-daemon/src/cli.rs`:
```rust
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    // Existing Reindex & FixDrift...

    /// Invalidate, synchronize, and pre-warm BLAKE3 query and embedding caches
    Recache {
        /// Force full eviction of all cached query and embedding entries instead of soft epoch increment
        #[arg(short, long, default_value_t = false)]
        clear: bool,

        /// Pre-warm hot query embeddings and high-frequency document chunks from persistent index
        #[arg(short, long, default_value_t = false)]
        warm: bool,
    },
}
```

#### In `crates/quorum-daemon/src/main.rs`:
```rust
// In main() match:
Some(Commands::Recache { clear, warm }) => {
    run_standalone_recache(cli.clone(), *clear, *warm).await?;
}

/// Executes standalone or client recache command.
pub async fn run_standalone_recache(
    cli: Cli,
    clear: bool,
    warm: bool,
) -> anyhow::Result<()> {
    info!(clear, warm, "Starting QuorumRAG recache process...");
    let daemon_url = format!("http://{}:{}/api/cache/recache", cli.host, cli.port);

    // 1. Attempt client call to active daemon
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()?;

    let payload = serde_json::json!({
        "clear": clear,
        "warm": warm,
    });

    if let Ok(resp) = client.post(&daemon_url).json(&payload).send().await {
        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await?;
            println!("\n=== QuorumRAG Cache Invalidation & Re-Cache Report ===");
            println!("Status:           ok (via active daemon at {}:{})", cli.host, cli.port);
            println!("Execution Time:   {}ms", json["duration_ms"]);
            println!("New Cache Epoch:  {}", json["cache_epoch"]);
            println!("Cleared Entries:  {}", json["cleared_entries"]);
            println!("Warmed Entries:   {}", json["warmed_entries"]);
            println!("======================================================\n");
            return Ok(());
        }
    }

    // 2. Standalone offline fallback
    let start = Instant::now();
    let storage_config = quorum_storage::StorageConfig::new(cli.data_dir.clone());
    let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
    
    // Increment epoch and perform offline cache warming
    let new_epoch = storage.bump_cache_epoch().await?;
    let cleared_count = if clear {
        storage.clear_cache_entries().await?
    } else {
        0
    };

    let warmed_count = if warm {
        // Pre-warm top indexed chunks from LanceDB / Tantivy
        storage.prewarm_embeddings().await?
    } else {
        0
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    println!("\n=== QuorumRAG Cache Invalidation & Re-Cache Report ===");
    println!("Status:           ok (standalone offline mode)");
    println!("Execution Time:   {}ms", duration_ms);
    println!("New Cache Epoch:  {}", new_epoch);
    println!("Cleared Entries:  {}", cleared_count);
    println!("Warmed Entries:   {}", warmed_count);
    println!("======================================================\n");

    Ok(())
}
```

---

### 4.2. REST API Endpoints (`crates/quorum-daemon/src/handlers/cache.rs` & `server.rs`)

#### New File: `crates/quorum-daemon/src/handlers/cache.rs`
```rust
use axum::extract::{Json, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::state::DaemonState;

/// Request payload for `POST /api/cache/recache`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecacheRequest {
    /// If true, completely evicts all cached queries and embeddings.
    #[serde(default)]
    pub clear: bool,

    /// If true, immediately runs pre-warming routine for hot queries and document chunks.
    #[serde(default)]
    pub warm: bool,
}

/// Response payload for `POST /api/cache/recache`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecacheResponse {
    pub status: String,
    pub cache_epoch: u64,
    pub cleared_entries: usize,
    pub warmed_entries: usize,
    pub duration_ms: u64,
}

/// Response payload for `GET /api/cache/stats`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    pub status: String,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub cached_queries: usize,
    pub cached_embeddings: usize,
    pub cache_epoch: u64,
}

/// POST /api/cache/recache - Triggers cache invalidation, epoch bump, and optional warming.
pub async fn recache_handler(
    State(state): State<Arc<DaemonState>>,
    Json(payload): Json<RecacheRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    info!(clear = payload.clear, warm = payload.warm, "Executing POST /api/cache/recache");

    let (cache_epoch, cleared_entries) = if payload.clear {
        state.coordinator.cache_clear_all().await
    } else {
        (state.coordinator.cache_bump_epoch().await, 0)
    };

    let warmed_entries = if payload.warm {
        state.coordinator.cache_prewarm(&state.storage).await.unwrap_or(0)
    } else {
        0
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Json(RecacheResponse {
        status: "ok".to_string(),
        cache_epoch,
        cleared_entries,
        warmed_entries,
        duration_ms,
    })
}

/// GET /api/cache/stats - Returns live telemetry for QueryCache and EmbeddingCache.
pub async fn cache_stats_handler(
    State(state): State<Arc<DaemonState>>,
) -> impl IntoResponse {
    let stats = state.coordinator.cache_stats();
    Json(CacheStatsResponse {
        status: "ok".to_string(),
        hits: stats.hits,
        misses: stats.misses,
        hit_rate: stats.hit_rate,
        cached_queries: stats.cached_queries,
        cached_embeddings: stats.cached_embeddings,
        cache_epoch: stats.cache_epoch,
    })
}
```

#### In `crates/quorum-daemon/src/handlers/mod.rs`:
```rust
pub mod cache;
pub mod chat;
pub mod drift;
pub mod health;
pub mod langextract;
pub mod reindex;
```

#### In `crates/quorum-daemon/src/server.rs`:
```rust
use crate::handlers::{cache, chat, drift, health, langextract, reindex};

pub fn build_router(state: Arc<DaemonState>) -> axum::Router {
    axum::Router::new()
        // ... existing routes ...

        // Cache Management Endpoints
        .route("/api/cache/recache", post(cache::recache_handler))
        .route("/api/cache/stats", get(cache::cache_stats_handler))

        // ... remaining routes ...
}
```

#### In `crates/quorum-daemon/src/handlers/health.rs`:
```rust
let cache_stats = state.coordinator.cache_stats();

Json(serde_json::json!({
    "status": "healthy",
    "version": "0.1.0",
    "contract_hash": contract_hash,
    "uptime_secs": uptime_secs,
    "cache": {
        "cache_epoch": cache_stats.cache_epoch,
        "cached_queries": cache_stats.cached_queries,
        "cached_embeddings": cache_stats.cached_embeddings,
        "hits": cache_stats.hits,
        "misses": cache_stats.misses,
        "hit_rate": cache_stats.hit_rate,
    },
    // ... existing storage, hardware, network blocks ...
}))
```

---

### 4.3. SSE Cache Telemetry & `/api/query` Cache Hit Integration

#### In `crates/quorum-coordinator/src/telemetry.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", content = "data")]
pub enum AgenticTelemetryEvent {
    // Existing variants...

    /// Cache telemetry emitted when query check completes
    #[serde(rename = "cache_telemetry")]
    CacheTelemetry {
        cache_hit: bool,
        cache_hits: u64,
        cache_misses: u64,
        cached_entries: usize,
        cache_epoch: u64,
    },
}
```

#### In `crates/quorum-daemon/src/handlers/chat.rs`:
```rust
// Inside tokio::spawn worker in chat_sse_handler:
// 1. Check cache early:
if let Some(cached_resp) = coordinator.get_cached_query(&prompt, family, _access_tier).await {
    let stats = coordinator.cache_stats();
    let _ = tx.send(AgenticTelemetryEvent::CacheTelemetry {
        cache_hit: true,
        cache_hits: stats.hits,
        cache_misses: stats.misses,
        cached_entries: stats.cached_queries,
        cache_epoch: stats.cache_epoch,
    }).await;

    // Stream cached tokens immediately
    for word in cached_resp.synthesis_prompt.split_inclusive(' ') {
        let _ = tx.send(AgenticTelemetryEvent::TokenDelta { delta: word.to_string() }).await;
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    for cit in cached_resp.sources_cited {
        let _ = tx.send(AgenticTelemetryEvent::CitationEmitted { citation: cit }).await;
    }
    let _ = tx.send(AgenticTelemetryEvent::ExecutionFinished {
        total_hops: 0,
        total_duration_ms: 2,
        total_tokens: cached_resp.synthesis_prompt.len(),
        final_state: quorum_coordinator::types::AgenticStateName::SynthesizeWithCitations,
        model_name: "Cache".to_string(),
    }).await;
    return;
} else {
    let stats = coordinator.cache_stats();
    let _ = tx.send(AgenticTelemetryEvent::CacheTelemetry {
        cache_hit: false,
        cache_hits: stats.hits,
        cache_misses: stats.misses,
        cached_entries: stats.cached_queries,
        cache_epoch: stats.cache_epoch,
    }).await;
}
```

---

### 4.4. ChatGPT Web UI Integration (`crates/quorum-ui`)

#### 1. In `crates/quorum-ui/assets/index.html`:

**A. Subsystem Sidebar Nav Button (Lines 52-66)**:
```html
<button id="btn-open-recache" class="sidebar-nav-btn" title="Re-Cache queries and embeddings">
  <span>⚡</span>
  <span>Re-Cache Engine</span>
  <span id="cache-status-pill" class="status-pill idle">0 hits</span>
</button>
```

**B. Top-Nav Header Group (Lines 113-130)**:
```html
<div class="nav-right">
  <!-- Re-Cache Button & Live Cache Stats Badge -->
  <div class="cache-control-group">
    <button id="btn-quick-recache" class="btn-recache-header" title="Invalidate and pre-warm BLAKE3 cache (POST /api/cache/recache)">
      <span class="icon">⚡</span>
      <span class="btn-text">Re-Cache</span>
    </button>
    <div id="cache-stats-badge" class="cache-badge" title="Live BLAKE3 Cache Hit Rate">
      <span class="pulse-dot-cache"></span>
      <span id="cache-badge-text">Cache: --%</span>
    </div>
  </div>

  <!-- 3-Way Search Mode Toggle... -->
```

**C. Re-Cache Modal Backdrop & Card (Lines 246+)**:
```html
<!-- RE-CACHE MODAL -->
<div id="modal-recache" class="modal-backdrop hidden">
  <div class="modal-card">
    <div class="modal-header">
      <h3>⚡ Multi-Layer BLAKE3 Re-Cache Engine</h3>
      <button class="btn-close-modal" id="btn-close-recache">&times;</button>
    </div>
    <div class="modal-body">
      <p class="modal-desc">
        Invalidates stale query synthesis and dense embeddings, increments the global cache epoch, and synchronizes with LanceDB + Tantivy indices.
      </p>
      <div class="form-group checkbox-group">
        <label><input type="checkbox" id="recache-clear"> Clear All (force full eviction of query &amp; embedding cache)</label>
        <label><input type="checkbox" id="recache-warm" checked> Pre-warm Cache (pre-compute embeddings for top documents)</label>
      </div>
      <div id="recache-stats-box" class="cache-stats-card">
        <div class="stat-row"><span>Cache Epoch:</span><strong id="recache-epoch-val">--</strong></div>
        <div class="stat-row"><span>Total Hits:</span><strong id="recache-hits-val">0</strong></div>
        <div class="stat-row"><span>Total Misses:</span><strong id="recache-misses-val">0</strong></div>
        <div class="stat-row"><span>Hit Rate:</span><strong id="recache-hitrate-val">--%</strong></div>
        <div class="stat-row"><span>Cached Queries:</span><strong id="recache-queries-val">0</strong></div>
        <div class="stat-row"><span>Cached Embeddings:</span><strong id="recache-embeddings-val">0</strong></div>
      </div>
      <div id="recache-result-box" class="progress-box hidden">
        <div id="recache-result-text" class="progress-status"></div>
      </div>
    </div>
    <div class="modal-footer">
      <button id="btn-execute-recache" class="btn-primary">Execute Re-Cache</button>
    </div>
  </div>
</div>
```

---

#### 2. In `crates/quorum-ui/assets/styles.css`:
```css
/* Cache Control Group in Top Navigation */
.cache-control-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-recache-header {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background-color: var(--bg-surface);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-recache-header:hover {
  background-color: var(--hover-bg);
  border-color: var(--accent);
}

.btn-recache-header.loading {
  opacity: 0.6;
  pointer-events: none;
}

.cache-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background-color: var(--bg-surface);
  border: 1px solid var(--border-color);
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
}

.pulse-dot-cache {
  width: 7px;
  height: 7px;
  background-color: var(--accent);
  border-radius: 50%;
  display: inline-block;
  box-shadow: 0 0 5px var(--accent);
}

/* Cache Statistics Card inside Modal */
.cache-stats-card {
  background-color: var(--bg-surface);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-muted);
}

.stat-row strong {
  color: var(--text-primary);
}

/* Cache Hit Pill in Chat Stream Assistant Message */
.cache-hit-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  background: rgba(52, 168, 83, 0.15);
  color: var(--success);
  border: 1px solid rgba(52, 168, 83, 0.3);
  margin-left: 8px;
}
```

---

#### 3. In `crates/quorum-ui/assets/app.js`:
```javascript
  // DOM Elements for Cache Control
  const btnQuickRecache = document.getElementById('btn-quick-recache');
  const cacheStatsBadge = document.getElementById('cache-stats-badge');
  const cacheBadgeText = document.getElementById('cache-badge-text');
  const btnOpenRecache = document.getElementById('btn-open-recache');
  const btnCloseRecache = document.getElementById('btn-close-recache');
  const modalRecache = document.getElementById('modal-recache');
  const btnExecuteRecache = document.getElementById('btn-execute-recache');
  const recacheClear = document.getElementById('recache-clear');
  const recacheWarm = document.getElementById('recache-warm');
  const cacheStatusPill = document.getElementById('cache-status-pill');
  const recacheResultBox = document.getElementById('recache-result-box');
  const recacheResultText = document.getElementById('recache-result-text');

  // Fetch live cache statistics
  async function refreshCacheStats() {
    try {
      const resp = await fetch('/api/cache/stats');
      if (!resp.ok) return;
      const data = await resp.json();
      
      const pct = (data.hit_rate * 100).toFixed(1);
      if (cacheBadgeText) {
        cacheBadgeText.textContent = `Cache: ${pct}% (${data.hits} hits)`;
      }
      if (cacheStatusPill) {
        cacheStatusPill.textContent = `${data.hits} hits`;
        cacheStatusPill.className = data.hits > 0 ? 'status-pill healthy' : 'status-pill idle';
      }

      // Update modal stats
      const epochEl = document.getElementById('recache-epoch-val');
      const hitsEl = document.getElementById('recache-hits-val');
      const missesEl = document.getElementById('recache-misses-val');
      const rateEl = document.getElementById('recache-hitrate-val');
      const qEl = document.getElementById('recache-queries-val');
      const eEl = document.getElementById('recache-embeddings-val');

      if (epochEl) epochEl.textContent = data.cache_epoch;
      if (hitsEl) hitsEl.textContent = data.hits;
      if (missesEl) missesEl.textContent = data.misses;
      if (rateEl) rateEl.textContent = `${pct}%`;
      if (qEl) qEl.textContent = data.cached_queries;
      if (eEl) eEl.textContent = data.cached_embeddings;
    } catch (_) {}
  }

  // Poll cache stats on load and every 10 seconds
  refreshCacheStats();
  setInterval(refreshCacheStats, 10000);

  // Quick Re-Cache from Top Header
  if (btnQuickRecache) {
    btnQuickRecache.addEventListener('click', async () => {
      btnQuickRecache.classList.add('loading');
      btnQuickRecache.innerHTML = `<span class="icon">⏳</span> <span>Re-Caching...</span>`;
      try {
        const resp = await fetch('/api/cache/recache', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ clear: false, warm: true })
        });
        const data = await resp.json();
        await refreshCacheStats();
        btnQuickRecache.innerHTML = `<span class="icon">✓</span> <span>Re-Cached!</span>`;
        setTimeout(() => {
          btnQuickRecache.innerHTML = `<span class="icon">⚡</span> <span>Re-Cache</span>`;
          btnQuickRecache.classList.remove('loading');
        }, 1500);
      } catch (err) {
        btnQuickRecache.innerHTML = `<span class="icon">⚠️</span> <span>Error</span>`;
        setTimeout(() => {
          btnQuickRecache.innerHTML = `<span class="icon">⚡</span> <span>Re-Cache</span>`;
          btnQuickRecache.classList.remove('loading');
        }, 2000);
      }
    });
  }

  // Modal Open/Close
  if (btnOpenRecache) {
    btnOpenRecache.addEventListener('click', () => {
      modalRecache.classList.remove('hidden');
      refreshCacheStats();
    });
  }
  if (btnCloseRecache) {
    btnCloseRecache.addEventListener('click', () => modalRecache.classList.add('hidden'));
  }

  // Modal Execute Re-Cache
  if (btnExecuteRecache) {
    btnExecuteRecache.addEventListener('click', async () => {
      btnExecuteRecache.disabled = true;
      btnExecuteRecache.textContent = 'Executing...';
      recacheResultBox.classList.remove('hidden');
      recacheResultText.textContent = 'Purging stale entries and pre-warming...';

      try {
        const resp = await fetch('/api/cache/recache', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            clear: recacheClear.checked,
            warm: recacheWarm.checked
          })
        });
        const result = await resp.json();
        recacheResultText.innerHTML = `<strong>Success:</strong> Cache Epoch bumped to <strong>${result.cache_epoch}</strong>. Cleared ${result.cleared_entries} entries, pre-warmed ${result.warmed_entries} entries in ${result.duration_ms}ms.`;
        await refreshCacheStats();
      } catch (e) {
        recacheResultText.innerHTML = `<span style="color:var(--danger)">Error: ${e.message}</span>`;
      } finally {
        btnExecuteRecache.disabled = false;
        btnExecuteRecache.textContent = 'Execute Re-Cache';
      }
    });
  }

  // Handle CacheTelemetry SSE Event in handleTelemetryEvent:
  // Inside function handleTelemetryEvent(event, bodyEl, citationsEl, msgHeaderEl):
  if (type === 'cache_telemetry' || type === 'CacheTelemetry') {
    const data = event.data || event;
    if (data.cache_hit) {
      if (msgHeaderEl) {
        const cachePill = document.createElement('span');
        cachePill.className = 'cache-hit-pill';
        cachePill.textContent = `⚡ Cache Hit (Epoch ${data.cache_epoch})`;
        msgHeaderEl.appendChild(cachePill);
      }
    }
    refreshCacheStats();
  }
```

---

## 5. Verification Method

### 5.1. Independent Verification Commands

1. **CLI Parsing & Command Test**:
   ```powershell
   cargo test -p quorum-daemon --test daemon_tests test_cli_parsing_default_and_flags -- --nocapture
   ```
   Assert that `quorumrag recache`, `quorumrag recache --clear`, and `quorumrag recache --warm` parse without errors and populate flags.

2. **Endpoints Verification**:
   ```powershell
   cargo test -p quorum-daemon --test adversarial_endpoint_tests -- --nocapture
   ```
   Assert that:
   - `POST /api/cache/recache` returns `StatusCode::OK` and valid JSON `{ status: "ok", cache_epoch, cleared_entries, warmed_entries, duration_ms }`.
   - `GET /api/cache/stats` returns `StatusCode::OK` and valid JSON `{ status: "ok", hits, misses, hit_rate, cached_queries, cached_embeddings, cache_epoch }`.
   - `GET /api/health` contains the `"cache"` telemetry block.
   - `POST /api/chat` SSE stream emits `event: cache_telemetry`.

3. **Web UI Assets & Visual Verification**:
   ```powershell
   cargo test -p quorum-ui --test ui_tests -- --nocapture
   cargo test -p quorum-ui --test adversarial_ui_tests -- --nocapture
   ```
   Assert that:
   - `index.html` contains `btn-open-recache`, `btn-quick-recache`, `cache-stats-badge`, and `modal-recache`.
   - `styles.css` contains `.btn-recache-header`, `.cache-badge`, and `.cache-hit-pill`.
   - All 5 themes (`theme-obsidian`, `theme-dark`, `theme-white`, `theme-midnight`, `theme-warm`) compile cleanly.

### 5.2. Invalidation Conditions
- If `quorumrag recache` fails to increment `cache_epoch`, tests will fail.
- If `/api/health` drops the `"cache"` key or alters existing `"storage"` / `"hardware"` keys, regression assertions in `adversarial_web_routes_challenge` will fail.
- If UI layout breaks the 260px sidebar or 768px reading column assertions, `test_sidebar_and_reading_column_dimensions_in_css_and_html` will fail.
