# Dispatch — Explorer M5.5-3: Daemon CLI, REST Endpoints & Web UI Re-Cache Control

## Objective
Investigate and design the user-facing and external interfaces for Caching and Re-Cache across `crates/quorum-daemon` and `crates/quorum-ui`:
1. CLI Subcommand:
   - Add `quorumrag recache [--clear] [--warm]` in `crates/quorum-daemon/src/cli.rs`.
   - Handler in `main.rs` that loads the coordinator / storage, clears or warms the cache, and prints formatted metrics (`cache_epoch`, `cleared_entries`, `warmed_entries`, `duration_ms`).
2. REST API Endpoints:
   - `POST /api/cache/recache`:
     - Triggers cache invalidation / epoch bump, clears stale entries, executes pre-warming (if configured), returns `{ status: "ok", cache_epoch, cleared_entries, warmed_entries, duration_ms }`.
   - `GET /api/cache/stats`:
     - Returns live cache telemetry `{ hits, misses, hit_rate, cached_queries, cached_embeddings, cache_epoch }`.
   - Update `GET /api/health` to include cache summary.
   - Update `/api/chat` SSE stream and `/api/query` response to include `cache_hit: bool`.
3. ChatGPT Web UI Integration (`crates/quorum-ui`):
   - Sidebar/Header control: Add an interactive **"Re-Cache"** button.
   - Live Cache Badge / Indicator: Show current Cache Hit status / Cache Hit Rate (e.g. `Cache: 85% (120 hits)`).
   - On click of "Re-Cache": Send `POST /api/cache/recache`, show a toast or spinning state, update the cache badge with new stats.
   - Ensure clean dark/light theme integration matching existing 5 themes in `styles.css`.

## Input Files to Inspect
- `crates/quorum-daemon/src/cli.rs`
- `crates/quorum-daemon/src/server.rs`
- `crates/quorum-daemon/src/main.rs`
- `crates/quorum-daemon/src/handlers/mod.rs`
- `crates/quorum-ui/src/assets/index.html`
- `crates/quorum-ui/src/assets/app.js`
- `crates/quorum-ui/src/assets/styles.css`
- `ORIGINAL_REQUEST.md`

## Output
Write your findings and design report to `.agents/teamwork/explorer_m5_5_3/handoff.md`. Include concrete route definitions, handler implementations, and HTML/CSS/JS snippets for the Web UI.


## 2026-11-01T03:59:52Z
[Message] timestamp=2026-11-01T03:59:52Z sender=e13403c5-b0e6-48a4-a1ec-a1f096f9f53e priority=MESSAGE_PRIORITY_HIGH content=You are Explorer M5.5-3. Read d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_3\DISPATCH.md. Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_3\. Investigate crates/quorum-daemon and crates/quorum-ui to design the CLI command `quorumrag recache [--clear] [--warm]`, REST endpoints POST /api/cache/recache and GET /api/cache/stats, SSE cache telemetry, and ChatGPT Web UI 'Re-Cache' button + live stats badge. Write your detailed design to d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_3\handoff.md and notify me when complete.
