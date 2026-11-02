# Scope: Milestone M8 — Low-Latency Query Execution, Consensus & P2P Security Hardening

## Module Boundaries
- `crates/quorum-coordinator/` (`retrieve.rs`, `mesh.rs`, `fusion.rs`, `offline_engine.rs`, `scatter_gather.rs`)
- `crates/quorum-network/` (`transport.rs`, `actor.rs`)
- `crates/quorum-contract/` (`lib.rs` -> `DeterministicOfflineEngine`)
- `crates/quorum-storage/` (`tantivy_fts.rs`)

## Features Assigned
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 48 | Low-Latency Multi-Hop Query Execution | Pre-computed static chunk scoring, stack buffer reuse in offline engine, eliminate redundant vector scans on rewrite hops | M8 | follow-up (R2) |
| 49 | P2P Cryptographic Handshake & Anti-Replay | Enforce Ed25519 signature & timestamp (<60s) verification in `OsoosiMeshInboundHandler::handle_query` with sender pubkey validation | M8 | follow-up (R2) |
| 50 | Byzantine Quorum Consensus Hardening | Bound per-peer voting influence in `RrfCombiner::fuse_rankings`, key on `blake3_hash`, clamp remote chunk tiers | M8 | follow-up (R2) |
| 51 | Air-Gapped Offline Enforcement | Assert loopback address on local Ollama client, restrict swarm dialing to loopback/private subnets | M8 | follow-up (R2) |

## Implementation Directives
1. **Low-Latency Multi-Hop Query Execution (Feature 48)**:
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs`:
     - In `RetrieveMeshNode::execute`, compute scoring criteria once per chunk upfront into a static struct instead of re-allocating lowercase strings inside the $O(N \log N)$ sort comparator.
   - In `crates/quorum-contract/src/lib.rs`:
     - In `DeterministicOfflineEngine::generate_vector`, reuse pre-allocated slice buffer instead of re-allocating `vec![0u8; dim * 4]` (3KB) per token.
   - In `crates/quorum-storage/src/tantivy_fts.rs`:
     - In `reload_with_retry`, fix the sequence fast-path check so `target_seq == 0` does not hit the physical drive with `index.load_metas()` when index has not mutated.
   - In `crates/quorum-coordinator/src/scatter_gather.rs` and `agentic/mod.rs`:
     - During query rewrite hops where the embedding vector is unchanged, skip duplicate LanceDB vector nearest-neighbor scans.
2. **P2P Cryptographic Handshake & Anti-Replay Verification (Feature 49)**:
   - In `crates/quorum-coordinator/src/mesh.rs` (`OsoosiMeshInboundHandler::handle_query`):
     - Enforce `crypto::verify_request(&req, access_tier, max_drift_secs=60)` before processing any remote query.
     - Validate freshness: reject requests with timestamp drift > 60 seconds.
     - Validate that `req.sender_pubkey` derives to `req.sender_peer_id` via `libp2p::PeerId::from_public_key(&req.sender_pubkey)`.
     - Reject malformed payloads: validate `req.embedding.len() == 768`, finite floats (no NaN/Inf), and `req.query_text.len() <= 4096`.
3. **Byzantine Quorum Consensus Hardening (Feature 50)**:
   - In `crates/quorum-coordinator/src/fusion.rs` (`RrfCombiner::fuse_rankings`):
     - Eliminate intra-peer duplicate score stuffing: ensure each remote peer contributes at most ONCE to any unique chunk's score, taking its highest/first rank.
     - Switch aggregation map key from arbitrary `chunk_id` to cryptographic content hash `blake3_hash`.
     - In `scatter_gather.rs`: clamp remote chunk tiers (`chunk.access_tier <= requested_tier`) and validate `line_start <= line_end`.
4. **Air-Gapped Offline Enforcement (Feature 51)**:
   - In `crates/quorum-coordinator/src/offline_engine.rs`: assert that Ollama destination socket address is strictly a loopback address (`addr.ip().is_loopback()`), failing closed if a non-loopback IP is targeted.
   - In `crates/quorum-network/src/transport.rs`: filter out public WAN IPs when dialing swarm peers, restricting connectivity to loopback (`127.0.0.0/8`, `::1`) and private RFC1918 subnets (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`).
5. **Invariants**:
   - Public Rust APIs, 10-column LanceDB schema, Tantivy schema, and cryptographic contracts must remain strictly unchanged.

## Survey Reference
- Read `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen2_2\handoff.md` for complete observations, line numbers, and architectural designs.


6. **Critical Query Precision & Streaming Enhancements (User Requirements)**:
   - In `crates/quorum-coordinator/src/offline_engine.rs` & `src/agentic/retrieve.rs`:
     - Expand `GENERIC_ATTRIBUTE_TERMS` to include generic financial/common nouns (`income`, `tax`, `taxes`, `statement`, `plan`, `payment`, `credit`, `account`, `amount`, `rate`, `form`, `section`, `page`, `total`, `year`, `day`, `heart`, `soul`, `life`, `time`, `work`, `world`, `reading`, `book`, `universal`).
     - Strictly enforce multi-term entity matching: when `entity_terms.len() >= 2`, a document chunk must match at least `(entity_terms.len() + 1) / 2` distinct `entity_terms` (for 2 or 3 terms, at least 2 distinct terms must appear, and at least one non-generic primary subject term must match). Single-word accidental matches on multi-word queries MUST be rejected (`is_relevant = false`).
     - Never output `Retrieved Grounded Evidence (N matching chunks)`.
   - In `crates/quorum-coordinator/src/agentic/synthesize.rs` & `src/offline_engine.rs`:
     - "source must be listed after all response": Stream/emit all `TokenDelta` tokens FIRST, and emit `CitationEmitted` events ONLY AFTER the full response text has completed.
     - For general knowledge questions with 0 matching local files, emit clean bottom source attribution citation (`Local Neural Model (Air-Gapped Ollama)`) so `SOURCES` is always present at the bottom of every response.
   - In `src/offline_engine.rs`:
     - Eliminate delay on "Synthesizing comprehensive response...": ensure `query_local_ollama_chat` streams tokens immediately via SSE (`TokenDelta`) and uses a fast, compact prompt (`top 4` deduplicated chunks, max 900 chars each, `num_predict: 600`, `num_ctx: 2048`) so tokens stream within 1-2 seconds.
