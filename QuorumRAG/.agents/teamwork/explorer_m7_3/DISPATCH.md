## 2026-11-02T01:53:49Z
Sender: facfe77d-94d3-44be-95df-eec2b9dfe368
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are a read-only exploration agent (teamwork_explorer) for Milestone M7: Feature 47 (Watcher Resilience & Zero-Overhead Host Throttling).
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m7_3
Original request log: d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md (lines 250-292)
PROJECT.md path: d:\harfile\QuorumRAG\PROJECT.md
Reference survey: d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen2_1\handoff.md

Your mission:
Formulate the exact implementation plan and code specifications for Feature 47 and Host Overhead Throttling:
1. Watcher Directory & Rename Event Handling:
   - In `crates/quorum-ingest/src/watcher.rs:120-155`:
     - When `WatcherEvent::Create(path)` or `WatcherEvent::Modify(path)` arrives: if `path.is_dir()`, recursively discover all files within and dispatch individual file events.
     - In `crates/quorum-ingest/src/engine.rs:255-275`: handle directory rename events properly (re-indexing all files under the new directory path and deleting chunks with old path prefix).
2. Windows Sharing Violation (Error 32) Resilience:
   - In `crates/quorum-ingest/src/classifier_router.rs` and `crates/quorum-classifier/src/lib.rs`:
     - Wrap `File::open` in a retry loop (3 attempts with exponential backoff: 25ms, 50ms, 100ms) to handle transient Windows Error 32 sharing violations during active writes.
3. Queue Overflow Drift Healing:
   - In `watcher.rs`, if internal channel `try_send` fails, emit `WatcherEvent::QueueOverflow` so the engine triggers a background drift reconciliation pass instead of silently dropping file changes.
4. Zero-Overhead Host Impact:
   - In `crates/quorum-ingest/src/reindex.rs:303-491`, insert cooperative `tokio::task::yield_now().await` checkpoints in the reindexing candidate processing loop.
   - Investigate setting background thread priority on the worker thread.

Scope boundaries:
- Read-only investigation. DO NOT write or edit source code files.
- Produce a detailed handoff.md in your working directory (d:\harfile\QuorumRAG\.agents\teamwork\explorer_m7_3\handoff.md) following the Handoff Protocol.
- Notify orchestrator with send_message when done.
