# DISPATCH — spec_miner_survey_1

## Task Assignment
You are Spec Miner 1: P2P Network, Security & Federated Retrieval Spec Miner.
Your mission is to mine and formalize exact protocol and interface specifications for R3 (rust-libp2p Mesh Network & Federated Retrieval) and related parts of R5.

### Required Specification Mining:
1. Complete, compilable `rust-libp2p` Swarm architecture:
   - Identify recommended `libp2p` crate version and feature flags (QUIC via `libp2p-quic`, TCP via `libp2p-tcp`, Noise via `libp2p-noise`, Yamux via `libp2p-yamux`, mDNS via `libp2p-mdns`, Kademlia DHT via `libp2p-kad`, GossipSub via `libp2p-gossipsub`, Request-Response via `libp2p-request-response` or `cbor`/`json`/protobuf codec).
   - Ensure NO mock loops or stubbed traits; define the Swarm behaviour composition (`#[derive(NetworkBehaviour)]`).
2. Federated Query Coordinator:
   - Scatter-gather query protocol: parallel local search and peer query broadcast/request-response with strict 400ms timeout.
   - Ed25519 `access_tier` SQL pre-filtering in LanceDB before vector scanning. Define peer identity verification and ACL rules.
   - Candidate chunk aggregation via Reciprocal Rank Fusion (RRF): mathematical formula, score normalization, deduplication.
   - Volatile chunk security: remote peer chunks held strictly in `ZeroizeOnDrop` memory wrappers during synthesis, ensuring zero data retention after query completion.
3. Multi-node integration test strategy:
   - Setting up in-process or dual-node loopback Swarms with simulated network conditions.
4. Deliver a structured report to `d:\harfile\QuorumRAG\.agents\teamwork\spec_miner_survey_1\handoff.md`.

Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` thoroughly before beginning.

## 2026-10-31T14:58:14Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Priority: MESSAGE_PRIORITY_HIGH
Content: You are Spec Miner 1: P2P Network & Retrieval Spec Miner.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\spec_miner_survey_1\.
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\spec_miner_survey_1\DISPATCH.md.
Mine specifications and interface contracts for R3 (rust-libp2p Swarm with QUIC/TCP, Noise, Yamux, mDNS, Kademlia DHT, GossipSub, Request-Response; 400ms scatter-gather query coordinator, Ed25519 access_tier filtering, RRF chunk fusion, ZeroizeOnDrop volatile memory) and R5 test strategies.
Write your structured findings to d:\harfile\QuorumRAG\.agents\teamwork\spec_miner_survey_1\handoff.md and send a message when done.
