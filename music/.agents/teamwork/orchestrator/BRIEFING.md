# BRIEFING — 2026-11-02T14:35:00Z

## Mission
Build, test, and ship the 20-file Reference WAV Importer, Multi-File RVQ Tokenizer, and RL Pipeline with built-in Legal Originality / Anti-Plagiarism Guard in `d:\harfile\music`.

## 🔒 My Identity
- Archetype: orchestrator
- Roles: orchestrator, user_liaison, human_reporter, successor
- Working directory: d:\harfile\music\.agents\teamwork\orchestrator\
- Original parent: 1a5adaba-285b-4ff5-91cf-814aea69f711
- Original parent conversation ID: 1a5adaba-285b-4ff5-91cf-814aea69f711

## 🔒 My Workflow
- **Pattern**: Project Pattern (Dual Track: Implementation + E2E Testing)
- **Scope document**: d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
1. **Decompose**:
   - Milestone 8: 20-File Reference WAV Importer & RL Pipeline with Legal Originality Guard
     - Phase 0: Survey & Technical Spec Mining (3 Explorers / Spec Miner)
     - Phase 1: Architectural Decomposition & Scope Definition
     - Phase 2: Implementation (Worker)
     - Phase 3: Comprehensive Verification Gate (2 Reviewers, 2 Challengers, 1 Forensic Auditor)
     - Phase 4: WebUI & Service Live Verification
2. **Dispatch & Execute**: Direct iteration loop (Explorer -> Worker -> Reviewers/Challengers/Auditor -> Gate)
3. **On failure**: Retry -> Replace -> Skip -> Redistribute -> Redesign
4. **Succession**: Track cumulative spawns (threshold 16 spawns per orchestrator generation)
- **Work items**:
  1. Survey & Spec Mining (explorer_m8_1, explorer_m8_2, spec_miner_m8) [in-progress]
  2. Scope Document & Feature Inventory [pending]
  3. Worker Implementation (Multi-File WAV Importer, RVQ Tokenizer, RL Policy + Originality Guard, REST + WebUI) [pending]
  4. Dual-Track E2E Test Writer / Suite [pending]
  5. Verification Gate (Reviewers, Challengers, Forensic Auditor) [pending]
  6. Solo PR Ship & Service Deployment [pending]
- **Current phase**: Phase 0: Survey & Specification Investigation
- **Current focus**: Launching parallel technical exploration & spec mining
- **Current iteration**: 1 / 32 (Milestone 8)

## 🔒 Key Constraints
- Strict dispatch-only orchestrator: NEVER write source code directly, NEVER run tests directly, delegate all exploration and implementation to subagents.
- Mandatory Originality / Anti-Plagiarism Guard (`R_originality_guard`):
  1. Max N-gram pitch/chroma sequence overlap penalty (no copied melodic phrases/hooks).
  2. Waveform cross-correlation & frame-level cosine similarity ceiling vs every individual uploaded reference WAV.
  3. Verifiable Originality & Legal Compliance Score in API and WebUI.
- Up to 20 reference WAV files supported with native file picker (`<input type="file" multiple accept=".wav,audio/wav">`) and drag-and-drop. Strictly enforce 20-file cap.
- Residual Vector Quantization (RVQ) discrete codebook tokenizer + acoustic analyzer (BPM/beat grid, Krumhansl-Schmuckler 12-bin chroma, timbre envelope, formant ratio 1-4 kHz, joint style embedding).
- RL Policy (PPO / Contextual Bandit + DPO) with empirical learning proof over 4 baselines (`Disabled`, `Random`, `Heuristic`, `NoLearning`) with 95% Confidence Intervals.
- Production code only: NO MOCKS, NO FAKE DATA, real WAV analysis, genuine audio math.
- Automatic Git commit & solo PR shipping via `scripts/solo_pr_ship.ps1`.
- Binary veto on Forensic Auditor integrity violations.

## Current Parent
- Conversation ID: 1a5adaba-285b-4ff5-91cf-814aea69f711
- Updated: 2026-11-02T14:35:00Z

## Team Roster
| Agent | Type | Work Item | Status | Conv ID |
|-------|------|-----------|--------|---------|
| explorer_m8_1 | teamwork_explorer | Survey WebUI & REST server for 1-20 WAV upload | completed | a4cebeec-7380-48e4-ae62-c9120ec0674b |
| explorer_m8_2 | teamwork_explorer | Survey DSP feature extraction, RVQ codebook & AudioBuffer | completed | e10410b0-80c8-4515-bbc4-92f190cf2c38 |
| spec_miner_m8 | teamwork_spec_miner | Survey RL Policy (PPO/Bandit+DPO), Composite Reward & Originality Guard specs | completed | a1d8352d-6b90-4ec7-8739-20114e257fd6 |
| worker_m8 | teamwork_worker | Milestone 8 Implementation (WAV Importer, RVQ, RL & Originality Guard) | completed | 74d7d47e-5503-45b7-9e3a-36fe0eafa454 |
| reviewer_m8_1 | teamwork_reviewer | Review DSP & RVQ Tokenizer | completed | 5147d80a-a7d4-41c8-92c4-3ed58f2dcbd5 |
| reviewer_m8_2 | teamwork_reviewer | Review RL, Originality Guard, REST & WebUI | completed | aee09b36-efb1-4145-aa57-192cb19bdf29 |
| challenger_m8_1 | teamwork_challenger | Empirical Challenge: DSP, RVQ & Originality Guard | completed | c3ea442a-5a17-4cc5-90a9-db3e0d94bf61 |
| challenger_m8_2 | teamwork_challenger | Empirical Challenge: REST, 20-File Cap, SSRF & RL Benchmark | completed | bc2968ff-e4fa-4312-9279-73ecb74e068e |
| auditor_m8 | teamwork_auditor | Forensic Integrity Audit (Zero Mocks, Bridgeport & ELVIS Safeguards) | completed | 6c28891e-2715-4b4b-a617-f0c8d16f8bab |
| worker_m8_ship | teamwork_worker | Milestone 8 Shipper & Live Server Deployer | in-progress | 3d677b92-37cc-4232-a578-ecfa06581432 |

## Succession Status
- Succession required: no
- Spawn count: 10 / 16 (Milestone 8 generation)
- Pending subagents: 3d677b92-37cc-4232-a578-ecfa06581432
- Predecessor: none
- Successor: not yet spawned

## Active Timers
- Heartbeat cron: f3dc28a2-6906-4296-bad9-049bc88a16ae/task-40
- Safety timer: none

## Artifact Index
- d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md — Authoritative User Request
- d:\harfile\music\PROJECT.md — Global Project Architecture & Milestones
- d:\harfile\music\.agents\teamwork\orchestrator\DISPATCH.md — Complete Dispatch History
- d:\harfile\music\.agents\teamwork\orchestrator\BRIEFING.md — Persistent Working Memory
- d:\harfile\music\.agents\teamwork\orchestrator\progress.md — Orchestrator Liveness & Progress
- d:\harfile\music\.agents\teamwork\orchestrator\GATE_STATUS.md — Gate Verdict Matrix
