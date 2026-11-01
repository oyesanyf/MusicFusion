# Dispatch — Worker M5.5-1: Multi-Layer Caching & Re-Cache Subsystem Implementation

## Mission
Implement Feature 43 (Multi-Layer Caching & Re-Cache Subsystem `recache`) across the QuorumRAG workspace to fulfill the user directive: *"the code needs to have caching and re cache"*.

DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Design Reference Hand-Offs
You MUST read and implement the exact designs specified by the 3 Explorers:
1. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\handoff.md` (Coordinator Caching Engine, BLAKE3 Key Derivation, ZeroizeOnDrop unpinned memory rules, Agentic State Graph integration).
2. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_2\handoff.md` (Storage `index_epoch` persistence in `.metadata.json`, Ingestion cache coherence hooks in `reindex`, `drift_healer`, and `watcher`, and `CacheWarmer`).
3. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_3\handoff.md` (Daemon CLI `quorumrag recache [--clear] [--warm]`, REST API `POST /api/cache/recache` & `GET /api/cache/stats`, health endpoint update, and ChatGPT Web UI Re-Cache controls and live hit-rate badge).

## Step-by-Step Implementation Instructions

### Step 1: `crates/quorum-types`
- In `crates/quorum-types/src/lib.rs`:
  - Add `CacheTelemetry` struct with `cache_hit`, `hits`, `misses`, `hit_rate`, `cached_queries`, `cached_embeddings`, `cache_epoch`.
  - Add `RecacheResponse` struct with `status`, `cache_epoch`, `cleared_entries`, `warmed_entries`, `duration_ms`.
  - Add `cache_hit: bool` (default `false`) and `cache_epoch: u64` (default `0`) to `QueryResult`.

### Step 2: `crates/quorum-storage`
- In `crates/quorum-storage/src/config.rs`:
  - Add `pub fn metadata_path(&self) -> PathBuf { self.storage_path.join(".metadata.json") }`.
- In `crates/quorum-storage/src/engine.rs`:
  - Add `StorageMetadata` struct (`index_epoch`, `last_epoch_bump_utc`, `last_bump_reason`, `mutation_count`).
  - Add `index_epoch: Arc<AtomicU64>` and `metadata_lock: Arc<parking_lot::Mutex<StorageMetadata>>` to `StorageEngine`.
  - In `StorageEngine::open`, load `.metadata.json` if it exists, or initialize default.
  - Add `pub fn index_epoch(&self) -> u64`.
  - Add `pub fn bump_index_epoch(&self, reason: &str) -> Result<u64, StorageError>` persisting `.metadata.json` atomically via tmp file.
  - Add `pub fn set_index_epoch(&self, epoch: u64, reason: &str) -> Result<u64, StorageError>`.

### Step 3: `crates/quorum-coordinator`
- Create `crates/quorum-coordinator/src/cache/`:
  - `key.rs`: `derive_embedding_cache_key(contract_hash, raw_text)` and `derive_query_cache_key(query, search_mode, model_family, access_tier, index_epoch)` using BLAKE3 and `normalize_text_for_caching`.
  - `embedding_cache.rs`: `EmbeddingCache` with `capacity`, `ttl`, `RwLock<HashMap<[u8; 32], EmbeddingEntry>>`, `RwLock<VecDeque<[u8; 32]>>`, `hits`, `misses`, `get`, `insert`, `clear`, `stats`.
  - `query_cache.rs`: `QueryCache` with `CachedQueryEntry` implementing `Drop` (which zeroizes text of remote unpinned chunks), `get`, `insert`, `clear`, `stats`.
  - `manager.rs`: `CacheManager` with `query_cache`, `embedding_cache`, `cache_epoch`, `epoch`, `advance_epoch`, `invalidate`, `telemetry(latest_hit)`, `recache(storage, embedder, clear, warm)`.
  - `mod.rs`: export all modules.
- In `crates/quorum-coordinator/src/telemetry.rs`:
  - Add `AgenticTelemetryEvent::CacheTelemetry { cache_hit, cache_epoch, cached_queries, cached_embeddings, hits, misses, hit_rate }`.
- In `crates/quorum-coordinator/src/coordinator.rs`:
  - Add `pub cache_manager: Arc<CacheManager>` to `QueryCoordinator`.
  - Add methods `pub fn cache_stats(&self) -> CacheTelemetry`, `pub fn cache_bump_epoch(&self) -> u64`, `pub fn cache_clear_all(&self) -> (u64, usize)`, `pub async fn cache_prewarm(...)`.
  - In `QueryCoordinator::query`: check `query_cache` using `storage.index_epoch()`. Return cached `QueryResult` on hit with `cache_hit = true`.
- In `crates/quorum-coordinator/src/agentic/mod.rs`:
  - Pass `cache_manager` to `AgenticStateGraphRunner`.
  - In `ClassifyIntent`, check `QueryCache`. On hit, emit `CacheTelemetry { cache_hit: true }`, emit citations, state transition to `Complete`, and return early! On miss, emit `CacheTelemetry { cache_hit: false }` and continue to `RetrieveMesh`.
  - In `SynthesizeWithCitations`, store final result in `QueryCache`.

### Step 4: `crates/quorum-ingest`
- In `crates/quorum-ingest/src/reindex.rs`:
  - In `execute_pipeline`, after micro-batches/compaction, if mutations occurred and not dry run, invoke `storage.bump_index_epoch("reindex_completed")`.
- In `crates/quorum-ingest/src/drift_healer.rs`:
  - In `fix_all_drifts`, if not dry run and `report.total_anomalies_healed > 0`, invoke `storage.bump_index_epoch("drift_healer_remediated")`.
- In `crates/quorum-ingest/src/engine.rs`:
  - In `handle_watcher_event`, if `storage_mutated`, invoke `storage.bump_index_epoch("watcher_filesystem_mutation")`.
- Create `crates/quorum-ingest/src/warmer.rs` (`CacheWarmer` with `warm_top_candidates`). Export in `lib.rs`.

### Step 5: `crates/quorum-daemon`
- In `crates/quorum-daemon/src/cli.rs`:
  - Add `Commands::Recache { clear: bool, warm: bool }`.
- In `crates/quorum-daemon/src/main.rs`:
  - Add `run_standalone_recache(cli, clear, warm)` (calling `POST /api/cache/recache` if daemon is running, fallback to storage engine if offline).
- Create `crates/quorum-daemon/src/handlers/cache.rs`:
  - `recache_handler` (`POST /api/cache/recache`).
  - `cache_stats_handler` (`GET /api/cache/stats`).
- In `crates/quorum-daemon/src/handlers/mod.rs`: export `pub mod cache;`.
- In `crates/quorum-daemon/src/server.rs`:
  - Register `.route("/api/cache/recache", post(cache::recache_handler))` and `.route("/api/cache/stats", get(cache::cache_stats_handler))`.
- In `crates/quorum-daemon/src/handlers/health.rs`:
  - Add `"cache"` telemetry block to the JSON output.
- In `crates/quorum-daemon/src/handlers/chat.rs`:
  - In `chat_sse_handler`, wire `AgenticTelemetryEvent::CacheTelemetry` SSE emission and cached streaming.
  - In `query_handler`, return `QueryResult` with `cache_hit` and `cache_epoch`.

### Step 6: `crates/quorum-ui`
- In `crates/quorum-ui/assets/index.html`:
  - Add top-nav header group with `#btn-quick-recache` and `#cache-stats-badge`.
  - Add sidebar nav button `#btn-open-recache` with `#cache-status-pill`.
  - Add `#modal-recache` modal dialog with checkboxes `#recache-clear` and `#recache-warm`, stats card `#recache-stats-box`, and execution button `#btn-execute-recache`.
- In `crates/quorum-ui/assets/styles.css`:
  - Add styling for `.cache-control-group`, `.btn-recache-header`, `.cache-badge`, `.pulse-dot-cache`, `.cache-stats-card`, `.stat-row`, `.cache-hit-pill`. Ensure 5-theme compatibility.
- In `crates/quorum-ui/assets/app.js`:
  - Add `refreshCacheStats()` polling `/api/cache/stats` and updating DOM elements.
  - Wire click events for `#btn-quick-recache`, `#btn-open-recache`, `#btn-close-recache`, `#btn-execute-recache`.
  - In `handleTelemetryEvent`, handle `cache_telemetry` / `CacheTelemetry`: if `cache_hit: true`, append `.cache-hit-pill` (`⚡ Cache Hit (Epoch N)`) to the assistant message header.

### Step 7: Tests & Verification
- Add unit tests for `EmbeddingCache`, `QueryCache`, and `key.rs` in `crates/quorum-coordinator`.
- Add test for `index_epoch` and `.metadata.json` in `crates/quorum-storage`.
- Add test for `CacheWarmer` and watcher invalidation in `crates/quorum-ingest`.
- Add test for CLI parsing and `/api/cache/recache` & `/api/cache/stats` in `crates/quorum-daemon`.
- Add test for UI assets and HTML/CSS presence in `crates/quorum-ui`.
- Run `cargo clippy --workspace --all-targets -- -D warnings` (must be 0 warnings).
- Run all unit and integration tests with `-- --test-threads=2`.
- Stage and commit all changes via git with descriptive commit message.

## Output
Write your comprehensive completion report to `.agents/teamwork/worker_m5_5_1/handoff.md` and report back via `send_message`.


## 2026-11-01T04:06:58Z
You are Worker M5.5-1. Read d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_5_1\DISPATCH.md.
Also read the design reports from the 3 Explorers:
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\handoff.md
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_2\handoff.md
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_3\handoff.md

Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_5_1\.
Implement the Multi-Layer BLAKE3 Caching Engine (QueryCache & EmbeddingCache), Storage index_epoch persistence in .metadata.json, Ingestion cache coherence hooks & CacheWarmer, Daemon CLI recache & REST endpoints (/api/cache/recache, /api/cache/stats), and ChatGPT Web UI Re-Cache button, live badge, and modal.
Ensure all tests compile and pass with -- --test-threads=2, clippy passes with 0 warnings, and git commit all changes.
Write your completion report to d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_5_1\handoff.md and report back via send_message when complete.

## 2026-11-01T04:38:02Z
**Context**: Milestone 5.5 Implementation Check
**Content**: Checking on your progress across the remaining steps (wiring cache routes into server.rs, UI app.js/styles.css, and running test/clippy verification). How is it progressing and do you need any clarification or assistance?
**Action**: Please provide a brief status update on your current step.
