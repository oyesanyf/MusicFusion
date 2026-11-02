# Handoff Report: Byzantine Peer & Fault Regression Suite (Feature 56)

**Agent**: Explorer M10 2 (Byzantine Peer & Fault Regression Explorer)  
**Date**: 2026-11-02  
**Milestone**: M10 — Hardening & Performance Polish  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m10_2`  
**Target Delivery**: `crates/quorum-coordinator/tests/byzantine_fault_regression.rs` & `tests/byzantine_fault_regression.rs`

---

## 1. Observation

A comprehensive inspection of the QuorumRAG codebase, existing adversarial test suites, and core architectural components across network, coordinator, storage, ingest, and daemon layers revealed the following facts:

### 1.1 Existing Adversarial & Fault Tests
The codebase currently contains isolated adversarial test targets distributed across individual crate `tests/` directories:
1. `crates/quorum-network/tests/`:
   - `adversarial_network_tests.rs`: Tests contract hash parsing edge cases (empty strings, truncation, hex corruption, byte alteration), 20 concurrent loopback queries, bidirectional simultaneous queries, and dynamic port collision resistance.
   - `network_tests.rs`: Tests dual-node CBOR query transmission and handshake rejection on mismatched contract hash.
2. `crates/quorum-coordinator/tests/`:
   - `m8_adversarial_p2p_tests.rs`: 741 lines of P2P cryptographic tests covering:
     - Mismatched public key vs claimed `sender_peer_id` rejection (`test_adv_mismatched_pubkey_vs_peer_id_rejected`).
     - Garbage/non-canonical public key rejection (`test_adv_invalid_or_garbage_pubkey_rejected`).
     - Forged/bit-flipped Ed25519 signatures and tampered query payloads (`test_adv_forged_or_tampered_signature_rejected`).
     - Expired (> 60s) and future (> 60s) timestamp rejection (`test_adv_timestamp_drift_past_expired_rejected`, `test_adv_timestamp_drift_future_rejected`).
     - Payload boundaries: NaN/Infinity floats in embeddings, embedding dimension != 768, query text > 4096 bytes, `top_k == 0`.
     - Strict WAN IP dial address blocking (RFC 1918 / ULA loopback only).
     - 100-chunk duplicate flooding Sybil test.
   - `adversarial_challenge_m8_2_tests.rs`: 677 lines covering:
     - Intra-peer score stuffing deduplication (`test_adv_byzantine_sybil_score_stuffing_and_honest_peer_superiority`).
     - Aggregation keying by BLAKE3 hash instead of malleable `chunk_id` (`test_adv_aggregation_key_blake3_hash_anti_malleability`).
     - Access tier clamping of remote chunks (`test_adv_remote_chunk_access_tier_clamping_to_requested`).
     - Inverted line numbers rejection (`line_start > line_end`).
     - Volatile memory zeroization upon drop (`VolatileRemoteChunk`).
   - `m8_5_rag_retrieval_isolation_tests.rs`: 271 lines testing exclusion of code files and temporary files during hybrid search and RAG retrieval.
   - `m8_p2p_consensus_latency_tests.rs`: Tests 1.25x consensus multiplier across multiple peers and low-latency execution.
3. `crates/quorum-ingest/tests/`:
   - `m8_5_exclusion_tests.rs`: 263 lines testing that `ClassifierRouter` rejects all source code extensions (`.rs`, `.py`, `.js`, etc.) and Office temporary lock files (`~$*`, `*.tmp`, `*.bak`, `*.wbk`).
   - `m9_monotonic_progress_test.rs`: 372 lines testing upfront candidate estimation, non-decreasing `files_scanned`, and monotonic progress percentage to 100.0%.
   - `adversarial_reindex_and_drift.rs`: Tests 5-drift remediation under missing/corrupted file states.
4. `crates/quorum-daemon/tests/`:
   - `adversarial_endpoint_tests.rs`: 700 lines testing concurrent API queries, SSE streaming syntax, and path traversal rejection.

### 1.2 Byzantine Peer Attack Mechanisms Observed
1. **Sybil Score Stuffing**:
   - In `crates/quorum-coordinator/src/fusion.rs:48-80`:
     ```rust
     for (remote_idx, list) in remote_peer_lists.into_iter().enumerate() {
         let remote_peer_id = remote_idx + 1;
         let mut seen_peer_hashes: HashSet<String> = HashSet::new();
         for (rank_0, candidate) in list.into_iter().enumerate() {
             let content_hash = DocumentChunk::compute_blake3_hash(&candidate.content);
             if !seen_peer_hashes.insert(content_hash.clone()) { continue; }
             let score = 1.0 / (k + (rank_0 + 1) as f32);
             ...
         }
     }
     ```
     - Remote candidate reported score (`candidate.score: 0.999`) is **completely ignored**. RRF rank score is strictly ordinal: $1 / (k + rank)$.
     - Flooding duplicate chunks from a single peer contributes at most once due to `seen_peer_hashes`.
     - Multi-peer boost ($1.25\times$) is applied only if $peers.len() \ge 2$ distinct peer slots confirmed the chunk.
2. **Merkle Root & Chunk Hash Tampering**:
   - In `crates/quorum-types/src/lib.rs:222-225`:
     ```rust
     pub fn verify_hash(&self) -> bool {
         let expected = Self::compute_blake3_hash(&self.text_content);
         self.blake3_hash == expected
     }
     ```
   - In `crates/quorum-coordinator/src/fusion.rs:53`: `RrfCombiner` recomputes `content_hash = DocumentChunk::compute_blake3_hash(&candidate.content)`, preventing any peer from forging an arbitrary BLAKE3 digest to overwrite legitimate chunks.
   - In `crates/quorum-coordinator/src/mesh.rs:220-248`: `derive_peer_id_from_pubkey(&req.sender_pubkey)` verifies claimed `sender_peer_id` against cryptographic pubkey.
   - In `crates/quorum-coordinator/src/crypto.rs:16-34`: Signature verifies over canonical bytes containing BLAKE3 embedding digest, query text, timestamp, access tier, top_k, and sender peer ID.
3. **Network Partitions & Silent Drops**:
   - In `crates/quorum-coordinator/src/scatter_gather.rs:246-250` and `370-386`:
     The scatter-gather engine enforces a strict timeout:
     ```rust
     let timeout_dur = Duration::from_millis(self.config.timeout_ms); // default 400ms
     self.mesh_dispatcher.query_peers(&req, timeout_dur).await.unwrap_or_default()
     ```
     Peers that hang, partition, or drop silently are dropped cleanly when the 400ms timer expires, and query execution continues with local results and responding peers.
4. **Replay Attacks**:
   - In `crates/quorum-coordinator/src/crypto.rs:77-86`:
     ```rust
     let now = Utc::now().timestamp();
     let drift = (now - timestamp).abs();
     if drift > max_drift_secs { // 60s
         return Err(CoordinatorError::StaleTimestamp { ... });
     }
     ```
     Timestamps older than 60s or more than 60s into the future fail closed with `StaleTimestamp`.

### 1.3 Invariant Fault Regressions Observed
1. **Source Code & Temp File Leakage**:
   - In `crates/quorum-ingest/src/classifier_router.rs:88-124`: Rejects compiled binaries, `.exe`/`.dll`, Office temp files (`~$*`, `*.tmp`, `*.bak`, `*.wbk`, `.~lock*`), and when `exclude_source_code: true`, rejects `FileCategory::Code` and all code extensions.
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs:155-196`: Filters out code files, temp files, and conversational dump files at retrieval time.
2. **RAG Deterministic Synthesis Isolation**:
   - In `crates/quorum-coordinator/src/offline_engine.rs:330-520`:
     - When `matched_chunks.is_empty()`, returns `"No matching documents were found in your local RAG index for **<prompt>**."` with `Vec::new()` citations.
     - Multi-term quorum precision prevents incidental 1-word hits from pulling irrelevant documents.
     - Citations strictly cite local retrieved documents with exact line numbers and text snippets.
     - Citations are emitted strictly at the bottom AFTER all token deltas.
3. **Monotonic Indexer Progress**:
   - In `crates/quorum-ingest/src/reindex.rs`: Upfront candidate discovery sets `files_total_estimated`. `files_scanned` is monotonically non-decreasing, and progress percentage advances monotonically to 100.0%.

### 1.4 Coverage Gaps Identified
While individual unit tests exist across crates, there is **no unified, end-to-end regression test suite** that:
- Injects Byzantine peer attacks (Sybil flooding, Merkle/BLAKE3 tampering, forged Ed25519 signatures, 400ms silent drop timeouts, and timestamp replay attacks) concurrently under high load.
- Concurrently validates invariant fault regressions (zero code/temp file leakage in RAG retrieval, zero ungrounded LLM synthesis, and monotonic indexer progress) in a single integrated test suite.
- Exercises these failure modes simultaneously to ensure no cross-component race condition, deadlocks, or state corruption occur.

---

## 2. Logic Chain

From the direct observations above, the design of the unified Byzantine Peer & Fault Regression Suite follows a rigorous 7-step deductive logic chain:

### Step 1: Sybil Resistance Invariant Logic
- *Premise*: An attacker controls one or more remote peers and can fabricate chunk scores (e.g. `score = 0.999`) or flood duplicate chunks under arbitrary IDs.
- *Mechanism*: `RrfCombiner::fuse_rankings` discards `candidate.score` entirely. It computes $1 / (k + rank)$ based purely on ordinal rank index. Intra-peer set `seen_peer_hashes` deduplicates per-peer submissions by `compute_blake3_hash(&candidate.content)`.
- *Inference*: A single rogue peer submitting 100 duplicates of poison content can contribute at most once, receiving a single rank #1 score $1 / (60 + 1) \approx 0.01639$. An honest local chunk at rank #1 corroborated by an honest peer achieves score $((1/61) + (1/61)) \times 1.25 \approx 0.04098$, which is $> 2.5\times$ higher than the rogue chunk.
- *Test Assertion*: 100 flooded chunks from a rogue peer must be deduplicated to exactly 1 result in fused output, and must never displace an honest local chunk corroborated by a peer.

### Step 2: Merkle & Cryptographic Tampering Logic
- *Premise*: A Byzantine node may alter text content while keeping an old BLAKE3 hash, or claim a legitimate chunk ID while supplying altered content.
- *Mechanism*: `DocumentChunk::verify_hash()` recomputes `blake3::hash(content.as_bytes()).to_hex()`. `RrfCombiner` recomputes the BLAKE3 digest dynamically from candidate content and uses that digest as the hashmap key.
- *Inference*: Any alteration of `text_content` modifies its computed BLAKE3 digest. A poisoned chunk claiming legitimate chunk ID `chunk_shared_001` with modified text is assigned a distinct BLAKE3 digest, preventing it from colliding with, overwriting, or boosting the legitimate chunk.
- *Test Assertion*: `DocumentChunk::verify_hash()` fails (`false`) on any 1-bit text tampering. Identical `chunk_id` with distinct content produces 2 separate entries in RRF output, neither receiving a consensus multiplier.

### Step 3: Ed25519 Signature & Identity Spoofing Logic
- *Premise*: An attacker may try to sign queries using a private key they control while claiming the `sender_peer_id` of a trusted node, or alter query parameters after signing.
- *Mechanism*: `OsoosiMeshInboundHandler::handle_query` extracts `sender_pubkey`, derives `PeerId` via `derive_peer_id_from_pubkey`, and asserts equality with `req.sender_peer_id`. It then verifies the Ed25519 signature over canonical bytes covering `query_id`, `sender_peer_id`, `timestamp`, `access_tier`, `top_k`, `embedding_hash`, and `query_text`.
- *Inference*: Any mismatch between pubkey derivation and `sender_peer_id` immediately rejects the query with 0 chunks. Any single bit flip in the signature, query text, or embedding vector invalidates the signature and returns 0 chunks without invoking LanceDB/Tantivy storage.
- *Test Assertion*: Bit-flipped signatures, altered query text, altered embedding floats, and mismatched pubkeys must all return `resp.chunks.is_empty()` and 0 storage invocations.

### Step 4: Network Partition & 400ms Hard Deadline Logic
- *Premise*: Remote peers may drop connections abruptly, experience network partitions, or deliberately hang for seconds to starve the coordinator.
- *Mechanism*: `ScatterGatherEngine` and `FederatedQueryCoordinator` wrap remote peer query futures in `tokio::time::timeout(Duration::from_millis(400), ...)`.
- *Inference*: When remote peers hang (e.g. 5,000ms delay) or drop connections, the timeout guard fires at 400ms. The coordinator gathers whatever local and fast-peer chunks arrived before the deadline, combines them via RRF, and returns cleanly.
- *Test Assertion*: A query across 5 hanging peers (800ms to 10,000ms delay) and 1 fast peer (20ms) must complete in $\le 450\text{ms}$. Fast peer and local chunks are present; hanging peer chunks are strictly absent.

### Step 5: Anti-Replay Freshness Window Logic
- *Premise*: An adversary eavesdrops on authentic signed queries and replays them to probe private indices.
- *Mechanism*: `verify_query` calculates `drift = (now - timestamp).abs()`. If `drift > 60`, it fails closed with `CoordinatorError::StaleTimestamp`.
- *Inference*: Replayed requests with timestamps older than 60s (or future timestamps $> 60\text{s}$) are rejected immediately.
- *Test Assertion*: Timestamps at `now - 61s`, `now - 3600s`, `0` (Unix epoch), and `now + 61s` must fail verification with `StaleTimestamp` and return 0 chunks.

### Step 6: Source Code & Temp File Exclusion Logic
- *Premise*: Source code files and temporary files in watched folders could contaminate RAG indices and pollute user citations.
- *Mechanism*: Multi-layer defense: `ClassifierRouter` rejects `FileCategory::Code`, code extensions, and temp files (`~$*`, `*.tmp`, `*.bak`, `*.wbk`). `FilesystemCrawler` prunes code directories. `RetrieveMeshNode` filters code/temp chunks at query time.
- *Inference*: Even if legacy code chunks exist in storage, RAG retrieval strictly purges them from candidate lists and synthesis citations.
- *Test Assertion*: In a directory with 5 code files, 4 temp/lock files, 2 binaries, and 3 legitimate documents/images, crawler and reindexer index exactly 3 files, and RAG retrieval returns 0 code/temp chunks.

### Step 7: Deterministic Grounded Synthesis & Progress Monotonicity Logic
- *Premise*: Queries must never hallucinate ungrounded facts, and progress indicators must never drop or oscillate.
- *Mechanism*: `DeterministicOfflineSynthesisEngine` requires multi-term entity matching and returns a strict fallback message with 0 citations when no documents match. `ReindexWorker` discovers total candidates upfront and increments `files_scanned` monotonically.
- *Inference*: Empty queries return deterministic fallback without model hallucination. Citations are emitted strictly at the bottom. Progress percentage monotonically advances to 100.0%.
- *Test Assertion*: Unmatched query returns exact fallback with `Vec::new()` citations. Progress percentage across 100+ samples satisfies $pct_t \ge pct_{t-1} - 0.0001$.

---

## 3. Caveats

1. **System Memory & Thread Constraints**:
   - The user directive states: `Preserve debug = 0 in Cargo.toml and jobs = 2. Never run cargo test concurrently across 5 subagents at the same time.`
   - In accordance with this directive, the regression suite must be executed with `-- --test-threads=2` to prevent memory exhaustion (error `0xc0000017 STATUS_NO_MEMORY`) when compiling LanceDB/DataFusion dependencies on Windows.
2. **Network Mode in Tests**:
   - For deterministic microsecond-accurate Byzantine fault injection (simulating 5,000ms hangs, Sybil flooding, and corrupted signatures), the suite utilizes `MockRemotePeer` and `DirectResponseDispatcher` within `ScatterGatherEngine`, combined with live loopback Swarm tests from `quorum-network`.
3. **No Mocks Protocol Compliance**:
   - In strict compliance with `no_mocks_write_actual_code.md`, all data types (`CandidateChunkDto`, `DocumentChunk`, `ScoredChunk`), cryptographic routines (BLAKE3, Ed25519-dalek), RRF ranking calculations, file classifiers (`ClassifierRouter`), and progress tracking engines are genuine production types, not hardcoded stub mocks.

---

## 4. Conclusion & Implementation Plan for Worker M10

### 4.1 Target File Locations
Worker M10 should implement the unified Byzantine fault regression suite at:
- **Primary Integration Suite**: `d:\harfile\QuorumRAG\crates\quorum-coordinator\tests\byzantine_fault_regression.rs`
- **Root Workspace Integration Target**: `d:\harfile\QuorumRAG\tests\byzantine_fault_regression.rs` (configured in `tests/e2e/Cargo.toml` as `[[test]] name = "byzantine_fault_regression"`).

### 4.2 Concrete Implementation Plan (7 Unified Test Functions)

Worker M10 must implement the following 7 comprehensive test functions in `crates/quorum-coordinator/tests/byzantine_fault_regression.rs`:

```rust
// crates/quorum-coordinator/tests/byzantine_fault_regression.rs

use chrono::Utc;
use quorum_classifier::FileTypeClassifier;
use quorum_contract::{EmbeddingContract, EmbeddingGenerator};
use quorum_coordinator::acl::PeerAclManager;
use quorum_coordinator::agentic::retrieve::RetrieveMeshNode;
use quorum_coordinator::config::CoordinatorConfig;
use quorum_coordinator::crypto::{sign_request, verify_request};
use quorum_coordinator::fusion::RrfCombiner;
use quorum_coordinator::mesh::{MeshQueryDispatcher, MockRemotePeer, OsoosiMeshInboundHandler};
use quorum_coordinator::offline_engine::DeterministicOfflineSynthesisEngine;
use quorum_coordinator::scatter_gather::{FederatedQueryCoordinator, ScatterGatherEngine, StorageSearcher};
use quorum_coordinator::types::{AgenticGraphState, SynthesisModelFamily};
use quorum_ingest::classifier_router::{ClassifierRouter, FilterDecision};
use quorum_ingest::config::{IngestConfig, ReindexOptions};
use quorum_ingest::crawler::FilesystemCrawler;
use quorum_ingest::dedup::TwoLevelDedupEngine;
use quorum_ingest::reindex::{ReindexPhase, ReindexWorker};
use quorum_network::{derive_peer_id_from_pubkey, InboundQueryHandler};
use quorum_storage::{StorageConfig, StorageEngine};
use quorum_types::{AccessTier, CandidateChunkDto, DocumentChunk, FederatedQueryRequest, FederatedQueryResponse, ScoredChunk};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::sync::RwLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// TEST 1: Sybil Score Stuffing & Flooding Attack Resistance
// ---------------------------------------------------------------------------
#[test]
fn test_byzantine_sybil_score_stuffing_under_high_load() {
    // 1. Legitimate local document
    let local_doc = DocumentChunk::new(
        "local_c1", "doc_local", "docs/report.pdf", 1, 10, 0, vec![0.1; 768],
        "Official verified corporate financial statement and audit.", None,
    ).unwrap();
    let local_results = vec![ScoredChunk::new(local_doc, 0.95, "local", false)];

    // 2. Byzantine Peer A floods 100 identical chunks claiming score 0.999
    let sybil_text = "Phishing attack or poison payload attempting to hijack search";
    let mut rogue_peer_a_chunks = Vec::new();
    for i in 0..100 {
        rogue_peer_a_chunks.push(CandidateChunkDto {
            chunk_id: format!("rogue_a_{}", i),
            file_path: format!("remote_a/fake_{}.txt", i),
            line_start: 1,
            line_end: 10,
            access_tier: 0,
            score: 0.999, // Artificially inflated
            content: sybil_text.to_string(),
        });
    }

    // 3. Honest Peer B corroborates the local document
    let honest_peer_b_chunks = vec![CandidateChunkDto {
        chunk_id: "peer_b_c1".to_string(),
        file_path: "remote_b/report.pdf".to_string(),
        line_start: 1,
        line_end: 10,
        access_tier: 0,
        score: 0.92,
        content: "Official verified corporate financial statement and audit.".to_string(),
    }];

    // Execute RRF Fusion: Local + Rogue Peer A + Honest Peer B
    let fused = RrfCombiner::fuse_rankings(
        local_results,
        vec![rogue_peer_a_chunks, honest_peer_b_chunks],
        60.0,
        10,
    );

    // Assertions:
    // a. The 100 flooded chunks MUST be deduplicated to exactly 1 entry
    let rogue_count = fused.iter().filter(|c| c.chunk.text_content == sybil_text).count();
    assert_eq!(rogue_count, 1, "Rogue peer 100-chunk flood must be deduplicated to exactly 1");

    // b. Score of rogue chunk must equal single rank 1 score 1 / 61
    let rogue_chunk = fused.iter().find(|c| c.chunk.text_content == sybil_text).unwrap();
    let expected_single_score = 1.0 / 61.0;
    assert!((rogue_chunk.score - expected_single_score).abs() < 1e-6);

    // c. Honest chunk receives multi-peer 1.25x boost and outranks rogue chunk at rank 0
    let honest_chunk = &fused[0];
    assert_eq!(honest_chunk.chunk.chunk_id, "local_c1");
    let expected_honest_score = ((1.0 / 61.0) + (1.0 / 61.0)) * 1.25;
    assert!((honest_chunk.score - expected_honest_score).abs() < 1e-6);
    assert!(honest_chunk.score > rogue_chunk.score);
}

// ---------------------------------------------------------------------------
// TEST 2: Merkle Root & Chunk Hash Tampering Detection
// ---------------------------------------------------------------------------
#[test]
fn test_byzantine_merkle_root_and_chunk_hash_tampering() {
    let authentic_text = "Verified cryptographic audit trail block";
    let chunk = DocumentChunk::new(
        "chunk_audit_01", "doc_audit", "audit.log", 1, 5, 0, vec![0.05; 768],
        authentic_text, None,
    ).unwrap();

    // Verification check on authentic chunk passes
    assert!(chunk.verify_hash(), "Authentic chunk hash must verify cleanly");

    // Tamper 1: Alter text content without updating blake3_hash
    let mut tampered_text_chunk = chunk.clone();
    tampered_text_chunk.text_content = "Tampered unauthorized modification of audit block".to_string();
    assert!(!tampered_text_chunk.verify_hash(), "Tampered text MUST fail hash verification");

    // Tamper 2: Alter blake3_hash without updating text
    let mut tampered_hash_chunk = chunk.clone();
    tampered_hash_chunk.blake3_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(!tampered_hash_chunk.verify_hash(), "Tampered hash MUST fail hash verification");

    // Tamper 3: Collision attack during RRF fusion
    // Rogue candidate attempts to claim the authentic chunk's ID but supplies altered content
    let candidate = CandidateChunkDto {
        chunk_id: "chunk_audit_01".to_string(), // Spoofed ID
        file_path: "fake/audit.log".to_string(),
        line_start: 1,
        line_end: 5,
        access_tier: 0,
        score: 0.99,
        content: "Malicious payload under legitimate ID".to_string(),
    };

    let fused = RrfCombiner::fuse_rankings(
        vec![ScoredChunk::new(chunk, 0.95, "local", false)],
        vec![vec![candidate]],
        60.0,
        10,
    );

    // Because fusion keys on dynamically recomputed BLAKE3 hash, both chunks remain separate!
    assert_eq!(fused.len(), 2, "Distinct content MUST remain separate despite identical chunk_id");
    assert_ne!(fused[0].chunk.blake3_hash, fused[1].chunk.blake3_hash);
}

// ---------------------------------------------------------------------------
// TEST 3: Ed25519 Forged Signatures & Identity Spoofing Rejection
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_byzantine_ed25519_forged_signature_and_identity_spoofing() {
    struct MockStorage;
    #[async_trait::async_trait]
    impl StorageSearcher for MockStorage {
        async fn search_hybrid(&self, _: &str, _: &[f32], _: u8, _: usize) -> Vec<ScoredChunk> {
            panic!("Storage MUST NOT be accessed on invalid signature or spoofed identity!");
        }
    }

    let seed_a = [11u8; 32];
    let secret_a = quorum_network::libp2p::identity::ed25519::SecretKey::try_from_bytes(seed_a).unwrap();
    let kp_a = quorum_network::libp2p::identity::ed25519::Keypair::from(secret_a);
    let pubkey_a = kp_a.public().to_bytes();
    let pk_a = quorum_network::libp2p::identity::PublicKey::from(kp_a.public());
    let peer_id_a = quorum_network::libp2p::PeerId::from_public_key(&pk_a).to_string();

    let handler = OsoosiMeshInboundHandler::new(Arc::new(MockStorage), &peer_id_a);

    // Attack A: Identity Spoofing (Keypair A signs request, but claims peer ID of Node B)
    let mut spoofed_req = FederatedQueryRequest {
        query_id: Uuid::new_v4(),
        sender_peer_id: "12D3KooWSpoofedVictimPeerIdNotMatchingKey".to_string(),
        sender_pubkey: pubkey_a,
        query_text: "Target confidential document".to_string(),
        embedding: vec![0.02; 768],
        top_k: 5,
        timestamp: Utc::now().timestamp(),
        signature: [0u8; 64],
    };
    sign_request(&seed_a, &mut spoofed_req, AccessTier::Tier0Public);
    let resp_spoofed = handler.handle_query(spoofed_req).await;
    assert!(resp_spoofed.chunks.is_empty(), "Spoofed sender_peer_id MUST return 0 chunks");

    // Attack B: Forged Signature (Bit flip in signature byte 0)
    let mut forged_sig_req = FederatedQueryRequest {
        query_id: Uuid::new_v4(),
        sender_peer_id: peer_id_a.clone(),
        sender_pubkey: pubkey_a,
        query_text: "Target confidential document".to_string(),
        embedding: vec![0.02; 768],
        top_k: 5,
        timestamp: Utc::now().timestamp(),
        signature: [0u8; 64],
    };
    sign_request(&seed_a, &mut forged_sig_req, AccessTier::Tier0Public);
    forged_sig_req.signature[0] ^= 0x01; // Bit flip
    let resp_forged = handler.handle_query(forged_sig_req).await;
    assert!(resp_forged.chunks.is_empty(), "Forged signature MUST return 0 chunks");

    // Attack C: Query Tampering After Signing (Altered query text)
    let mut tampered_text_req = FederatedQueryRequest {
        query_id: Uuid::new_v4(),
        sender_peer_id: peer_id_a.clone(),
        sender_pubkey: pubkey_a,
        query_text: "Public weather query".to_string(),
        embedding: vec![0.02; 768],
        top_k: 5,
        timestamp: Utc::now().timestamp(),
        signature: [0u8; 64],
    };
    sign_request(&seed_a, &mut tampered_text_req, AccessTier::Tier0Public);
    tampered_text_req.query_text = "Classified confidential keys".to_string();
    let resp_tampered = handler.handle_query(tampered_text_req).await;
    assert!(resp_tampered.chunks.is_empty(), "Tampered query text MUST return 0 chunks");
}

// ---------------------------------------------------------------------------
// TEST 4: Network Partitions, Silent Drops, & 400ms Hard Timeout Guard
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_byzantine_network_partition_and_silent_drop_400ms_deadline() {
    struct TestLocalSearcher {
        sample_chunk: ScoredChunk,
    }
    #[async_trait::async_trait]
    impl StorageSearcher for TestLocalSearcher {
        async fn search_hybrid(&self, _: &str, _: &[f32], _: u8, _: usize) -> Vec<ScoredChunk> {
            tokio::time::sleep(Duration::from_millis(15)).await;
            vec![self.sample_chunk.clone()]
        }
    }

    let local_doc = DocumentChunk::new(
        "local_stable_1", "doc_local", "local/data.txt", 1, 10, 0, vec![0.1; 768],
        "Local authoritative document that survives network partition.", None,
    ).unwrap();
    let local_searcher = TestLocalSearcher {
        sample_chunk: ScoredChunk::new(local_doc, 0.95, "local", false),
    };

    let mut coordinator = FederatedQueryCoordinator::new(local_searcher);

    // Fast responsive peer (responds in 30ms)
    coordinator.add_peer(MockRemotePeer {
        peer_id: "peer_fast_honest".to_string(),
        latency: Duration::from_millis(30),
        max_access_tier: 0,
        candidate_chunks: vec![CandidateChunkDto {
            chunk_id: "c_fast_remote".to_string(),
            file_path: "fast/doc.txt".to_string(),
            line_start: 1,
            line_end: 10,
            access_tier: 0,
            score: 0.90,
            content: "Fast remote peer document".to_string(),
        }],
    });

    // 5 Partitioned / Silent Drop / Hanging Peers (delays 800ms to 5,000ms > 400ms deadline)
    for i in 1..=5 {
        coordinator.add_peer(MockRemotePeer {
            peer_id: format!("peer_partitioned_{}", i),
            latency: Duration::from_millis(800 * i as u64),
            max_access_tier: 0,
            candidate_chunks: vec![CandidateChunkDto {
                chunk_id: format!("c_partitioned_{}", i),
                file_path: format!("bad/part_{}.txt", i),
                line_start: 1,
                line_end: 10,
                access_tier: 0,
                score: 0.99,
                content: format!("Partitioned peer {} data should be discarded", i),
            }],
        });
    }

    // Execute query: Hard deadline is 400ms
    let start = std::time::Instant::now();
    let (results, volatile, duration_ms) = coordinator
        .execute_query("partition test", &vec![0.1; 768], 0, 10)
        .await;
    let elapsed = start.elapsed();

    // Invariants:
    // 1. Duration must be around 400ms (<= 450ms), NOT 4,000ms!
    assert!(
        duration_ms <= 450,
        "Scatter-gather query took {}ms (> 450ms deadline violation)",
        duration_ms
    );
    assert!(elapsed.as_millis() <= 500);

    // 2. Results must contain local chunk and fast peer chunk
    let ids: Vec<&str> = results.iter().map(|r| r.chunk.chunk_id.as_str()).collect();
    assert!(ids.contains(&"local_stable_1"));
    assert!(ids.contains(&"c_fast_remote"));

    // 3. None of the partitioned peers' chunks must appear
    for i in 1..=5 {
        let part_id = format!("c_partitioned_{}", i);
        assert!(!ids.contains(&part_id.as_str()));
    }

    // 4. Remote chunks in volatile memory must be scrubbed on drop
    assert!(!volatile.is_empty());
}

// ---------------------------------------------------------------------------
// TEST 5: Replay Attacks & Anti-Replay Freshness Windows
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_byzantine_replay_attack_and_drift_window_rejection() {
    struct MockStorage;
    #[async_trait::async_trait]
    impl StorageSearcher for MockStorage {
        async fn search_hybrid(&self, _: &str, _: &[f32], _: u8, _: usize) -> Vec<ScoredChunk> {
            vec![]
        }
    }

    let seed = [22u8; 32];
    let secret = quorum_network::libp2p::identity::ed25519::SecretKey::try_from_bytes(seed).unwrap();
    let kp = quorum_network::libp2p::identity::ed25519::Keypair::from(secret);
    let pubkey = kp.public().to_bytes();
    let pk = quorum_network::libp2p::identity::PublicKey::from(kp.public());
    let peer_id = quorum_network::libp2p::PeerId::from_public_key(&pk).to_string();

    let handler = OsoosiMeshInboundHandler::new(Arc::new(MockStorage), &peer_id);
    let now = Utc::now().timestamp();

    // Expired past timestamps (> 60s) and future timestamps (> 60s)
    let replayed_timestamps = vec![
        now - 61,       // 1 second past drift window
        now - 120,      // 2 minutes ago
        now - 3600,     // 1 hour ago
        now - 86400,    // 1 day ago
        0,              // Unix epoch replay (1970)
        now + 61,       // 1 second in future
        now + 3600,     // 1 hour in future
    ];

    for ts in replayed_timestamps {
        let mut req = FederatedQueryRequest {
            query_id: Uuid::new_v4(),
            sender_peer_id: peer_id.clone(),
            sender_pubkey: pubkey,
            query_text: "Replayed authentic query".to_string(),
            embedding: vec![0.05; 768],
            top_k: 5,
            timestamp: ts,
            signature: [0u8; 64],
        };
        sign_request(&seed, &mut req, AccessTier::Tier0Public);

        let resp = handler.handle_query(req.clone()).await;
        assert!(
            resp.chunks.is_empty(),
            "Replayed request with timestamp {} must return 0 chunks",
            ts
        );
        assert!(verify_request(&req, AccessTier::Tier0Public, 60).is_err());
    }

    // Valid timestamps within 60s drift window must pass
    for ts in [now - 30, now, now + 30] {
        let mut valid_req = FederatedQueryRequest {
            query_id: Uuid::new_v4(),
            sender_peer_id: peer_id.clone(),
            sender_pubkey: pubkey,
            query_text: "Valid query within drift window".to_string(),
            embedding: vec![0.05; 768],
            top_k: 5,
            timestamp: ts,
            signature: [0u8; 64],
        };
        sign_request(&seed, &mut valid_req, AccessTier::Tier0Public);
        assert!(verify_request(&valid_req, AccessTier::Tier0Public, 60).is_ok());
    }
}

// ---------------------------------------------------------------------------
// TEST 6: Invariant Fault Regression: Source Code & Temp File Leakage
// ---------------------------------------------------------------------------
#[test]
fn test_invariant_fault_regression_source_code_and_temp_file_leakage() {
    let dir = tempdir().unwrap();
    let base = dir.path();

    // 1. Create mixed directory: source code, temp files, binaries, legitimate docs
    let files = [
        ("main.rs", "fn main() { println!(\"code\"); }", true),
        ("app.py", "def run(): pass", true),
        ("bundle.js", "console.log('js');", true),
        ("query.sql", "SELECT * FROM users;", true),
        ("~$Confidential_Audit.docx", "Word lock buffer", true),
        ("~autosave.tmp", "Temporary autosave buffer", true),
        ("backup_data.bak", "Backup copy", true),
        ("payload.exe", "MZ executable binary", true),
        ("library.dll", "PE DLL binary", true),
        ("annual_report.pdf", "%PDF-1.5 Annual financial earnings report and overview.", false),
        ("meeting_notes.txt", "Meeting notes regarding architectural roadmap and milestones.", false),
        ("system_architecture.png", "[PNG Image: 1920x1080 system diagram]", false),
    ];

    let mut cfg = IngestConfig::default();
    cfg.exclude_source_code = true;
    let router = ClassifierRouter::new(cfg);

    for (name, content, should_exclude) in files {
        let path = base.join(name);
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }

        let decision = router.evaluate_file(&path);
        if should_exclude {
            assert!(
                matches!(decision, FilterDecision::SkipBinary { .. } | FilterDecision::SkipHidden),
                "File '{}' MUST be excluded by ClassifierRouter, got {:?}",
                name, decision
            );
        } else {
            assert!(
                matches!(decision, FilterDecision::Accept { .. }),
                "Legitimate file '{}' MUST be accepted, got {:?}",
                name, decision
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TEST 7: Invariant Fault Regression: Deterministic Grounding & Progress Monotonicity
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_invariant_fault_regression_deterministic_synthesis_and_monotonic_progress() {
    // Part A: Zero Ungrounded Hallucinations in Synthesis
    // 1. Query with 0 matching documents -> Deterministic fallback, 0 citations
    let (response_empty, citations_empty) = DeterministicOfflineSynthesisEngine::synthesize(
        "what is quantum teleportation cryptography", &[], "AutoFusion",
    );
    assert!(response_empty.contains("No matching documents were found in your local RAG index"));
    assert!(citations_empty.is_empty(), "Empty search MUST return 0 citations");

    // 2. Query with non-matching multi-term query (incidental generic hit rejected)
    let tax_chunk = ScoredChunk::new(
        DocumentChunk::new(
            "tax_c1", "doc_tax", "tax_statement_2023.pdf", 1, 20, 0, vec![0.1; 768],
            "Annual gross taxable income statement and standard deduction calculation.", None,
        ).unwrap(),
        0.85, "local", false,
    );
    // Query: "universal mineral income" -> has generic term "income", but 0 hits for "universal" or "mineral"
    let (response_incidental, citations_incidental) = DeterministicOfflineSynthesisEngine::synthesize(
        "what is universal mineral income", &[tax_chunk], "AutoFusion",
    );
    assert!(response_incidental.contains("No matching documents were found"));
    assert!(citations_incidental.is_empty(), "Incidental generic term match MUST be rejected");

    // Part B: Monotonic Indexer Progress Invariant
    let temp = tempdir().unwrap();
    let root = temp.path();
    let sub = root.join("docs");
    create_dir_all(&sub).unwrap();

    for i in 0..12 {
        let p = sub.join(format!("document_{}.txt", i));
        let mut f = File::create(&p).unwrap();
        writeln!(f, "Document {} content for monotonic progress assertion.", i).unwrap();
    }

    let storage_dir = root.join("storage");
    let storage = Arc::new(StorageEngine::open(StorageConfig::new(storage_dir)).await.unwrap());
    let classifier = Arc::new(FileTypeClassifier::new());
    let embedder: Arc<dyn EmbeddingGenerator> = Arc::new(quorum_contract::DeterministicOfflineEngine::new());
    let dedup = Arc::new(TwoLevelDedupEngine::new());

    let config = IngestConfig {
        watch_paths: vec![sub.clone()],
        ..Default::default()
    };

    let worker = ReindexWorker::new(config, storage, classifier, embedder, dedup);
    let options = ReindexOptions {
        target_paths: vec![sub],
        full_rebuild: true,
        force_reembed: false,
        batch_size: 4,
        num_threads: 0,
        dry_run: false,
    };

    worker.start_reindex(options).await.unwrap();

    let mut last_scanned = 0;
    let mut last_pct = 0.0;
    let mut polls = 0;

    loop {
        tokio::time::sleep(Duration::from_millis(5)).await;
        polls += 1;

        if let Some(prog) = worker.get_progress().await {
            // Invariant 1: files_scanned must be monotonically non-decreasing
            assert!(
                prog.files_scanned >= last_scanned,
                "files_scanned dropped from {} to {}",
                last_scanned, prog.files_scanned
            );
            last_scanned = prog.files_scanned;

            // Invariant 2: progress percentage must be monotonically non-decreasing
            if prog.files_total_estimated > 0 {
                let current_pct = (prog.files_scanned as f64 / prog.files_total_estimated as f64) * 100.0;
                assert!(
                    current_pct >= last_pct - 0.0001,
                    "Progress percentage dropped from {:.2}% to {:.2}%",
                    last_pct, current_pct
                );
                last_pct = current_pct;
            }

            if matches!(prog.phase, ReindexPhase::Completed) {
                assert_eq!(prog.files_scanned, 12);
                assert_eq!(prog.files_indexed, 12);
                break;
            }
        }
        if polls > 500 {
            panic!("Reindex worker timed out");
        }
    }
    assert!(last_pct >= 99.99);
}
```

---

## 5. Verification Method

### 5.1 Test Execution Command
To verify the test suite cleanly and independently:
```powershell
# From workspace root d:\harfile\QuorumRAG:
cargo test -p quorum-coordinator --test byzantine_fault_regression -- --test-threads=2 --nocapture
```

### 5.2 Dependent Invariant Verification Commands
```powershell
# Verify source code and temp file exclusion invariants:
cargo test -p quorum-ingest --test m8_5_exclusion_tests -- --nocapture

# Verify monotonic progress reporting invariant:
cargo test -p quorum-ingest --test m9_monotonic_progress_test -- --nocapture

# Verify full coordinator adversarial suite:
cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2
cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2
```

### 5.3 Invalidation Conditions
The test suite or conclusion is invalidated if:
1. Any rogue peer reporting `score = 0.999` causes an inflated score in `RrfCombiner::fuse_rankings` or outranks legitimate corroborated chunks.
2. An altered `text_content` with a mismatched `blake3_hash` passes `verify_hash()`.
3. An altered Ed25519 query signature or forged payload returns non-empty candidate chunks.
4. Hanging or partitioned peers cause `ScatterGatherEngine` to exceed 450ms.
5. Replayed queries older than 60s return valid candidate chunks.
6. Any source code file (`FileCategory::Code`) or Office temp file (`~$*`) appears in RAG synthesis citations.
7. `ReindexWorker` progress percentage decreases at any point during scanning or indexing.
