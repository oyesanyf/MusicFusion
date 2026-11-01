Build QuorumRAG, a production-grade local-first peer-to-peer retrieval-augmented generation daemon and simple ChatGPT-like web interface, adapting useful code from `d:\harfile` (including `d:\harfile\fusion` and IDE interface code) with Gemma Embedding contract and embedded LanceDB hybrid storage.

Working directory: d:\harfile\QuorumRAG
Integrity mode: development

## Requirements

### R1. High-Capacity Ingestion & Isolated Storage Engine
Build a high-throughput filesystem ingestion engine and embedded LanceDB storage layer capable of scaling to 30TB+ drives. Encapsulate all `lancedb` and `arrow-array` `RecordBatch` conversions strictly inside the storage module so raw Arrow types never leak into ingestion, networking, or UI layers. Ingestion must use `notify` and `ignore` with configurable binary and extension filters, file size limits, two-level BLAKE3 chunk deduplication (skipping redundant embeddings for unchanged or moved files), micro-batched writes, scheduled IVF_PQ and Tantivy full-text index compaction, and configurable `.lance` index storage paths.

### R2. Gemma Embedding Contract & Offline Execution
Implement a deterministic mesh-wide embedding contract using Gemma Embedding (`EmbeddingGemma` / `google/embeddinggemma-300m` or local Gemma embedding runtime) operating strictly offline without external network dependencies. Compute a deterministic BLAKE3 contract hash over embedding dimensions, normalization settings, and distance metrics, rejecting any peer during network handshake whose contract hash diverges.

### R3. rust-libp2p Mesh Network & Federated Retrieval
Implement a complete, compilable `rust-libp2p` Swarm with QUIC/TCP transports, Noise encryption, Yamux multiplexing, mDNS LAN discovery, Kademlia DHT, GossipSub query broadcast, and Request-Response chunk streaming without stubbed handlers or mock loops. The query coordinator must dispatch parallel local and mesh queries with a 400ms timeout, enforce Ed25519 `access_tier` SQL pre-filtering inside LanceDB before vector scanning, aggregate candidate chunks via Reciprocal Rank Fusion (RRF), and hold remote peer chunks strictly in volatile `ZeroizeOnDrop` memory during synthesis.

### R4. ChatGPT-like Web Interface & End-to-End Wiring
Provide a clean, responsive ChatGPT-like web chat interface adapted from existing interface code in `d:\harfile\fusion` and `d:\harfile`. Wire the web UI directly to the local daemon and federated query coordinator, supporting real-time query submission, streaming responses, and structured document source citations (peer ID, file path, line numbers, and score).

### R5. Automated Verification & Test Harness
Provide comprehensive, automated test suites verifying all core components end-to-end, including Arrow type boundary isolation, BLAKE3 deduplication and ingestion filtering, Gemma embedding contract verification, multi-node libp2p Swarm discovery and federated RRF query execution under simulated network latency, and web UI API endpoints.

## Acceptance Criteria

### Storage & Ingestion
- [ ] Raw `arrow-array` and `lancedb` types are strictly confined to the storage crate and do not appear in the public APIs or signatures of the ingestion, networking, or UI crates.
- [ ] Ingestion pipeline filters excluded files, enforces file size caps, computes two-level BLAKE3 hashes, and avoids recomputing embeddings for duplicate or moved chunks.
- [ ] LanceDB table initialization, vector indexing (IVF_PQ), Tantivy FTS indexing, and compaction routines execute successfully with configurable data paths.

### Embedding Contract & Offline Verification
- [ ] Gemma Embedding integration generates deterministic dense vectors locally without outbound network access.
- [ ] Contract hash function derives a deterministic BLAKE3 digest and rejects handshakes with mismatched configurations.

### Networking & Federated Search
- [ ] `rust-libp2p` Swarm compiles cleanly with QUIC/TCP, Noise, Yamux, mDNS, Kademlia, GossipSub, and Request-Response protocols without stubbed placeholder traits.
- [ ] Multi-node integration test validates peer discovery via mDNS and federated scatter-gather query execution across two nodes within 400ms.
- [ ] LanceDB pre-filtering strictly blocks unauthorized directories based on Ed25519 peer identity and access tier rules.
- [ ] RRF combiner correctly normalizes and merges local and remote candidate chunks, and remote chunks are zeroized on drop from memory.

### Web Interface & Integration
- [ ] Local web server launches and serves the ChatGPT-like chat interface in a browser.
- [ ] Submitting a prompt in the web interface returns streaming synthesis results with clickable or structured source citations referencing retrieved chunks.
- [ ] All unit and integration tests compile and pass via `cargo test`.
