# Dispatch: Forensic Auditor (Milestone 6 Final Integrity Audit)

## Metadata
- **Agent Name**: auditor_m6_1
- **Role**: Milestone 6 Final Forensic Integrity Auditor
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m6_1\`
- **Project Scope Document**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Original User Request**: `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`

## Mandate & Scope
Perform an exhaustive, uncompromising forensic integrity audit of the QuorumRAG workspace.
The audit is a **BINARY VETO** — any detected mock, dummy facade, fabricated log, or shortcut causes immediate failure (`INTEGRITY VIOLATION`).

## Mandatory Forensic Checks
1. **Zero Mocks Protocol (`RULE[no_mocks_write_actual_code.md]`)**:
   - Verify zero dummy personas, zero hardcoded mockup data, zero facade mocks across all source and test code.
   - Verify all tests exercise real local files, real endpoints, real cryptographic primitives, and genuine LanceDB/Tantivy indexes.
2. **Cryptographic & Deduplication Integrity**:
   - Verify genuine BLAKE3 hashing across document chunks and level-2 deduplication.
   - Verify genuine Ed25519 signing payloads and constant-time signature verification.
3. **Storage & Arrow Encapsulation**:
   - Verify real LanceDB IVF_PQ tables on disk, real Tantivy inverted indexes on disk.
   - Verify zero leaked Arrow dependencies in domain models (`quorum-types`).
4. **Google LangExtract 4-Arm Engine**:
   - Verify genuine 4-arm consensus and strict character interval (`char_interval`) grounding against raw document bytes without hallucinated spans.
5. **Multi-Layer Caching & Recache Subsystem**:
   - Verify genuine BLAKE3 caching engine (`QueryCache` + `EmbeddingCache`), real `cache_epoch` sync, and genuine CLI/REST recache mechanisms.
   - Verify unpinned remote peer chunks undergo genuine `ZeroizeOnDrop` memory scrubbing on eviction/drop.
6. **Feature 42 Live IDE Execution Gate Authenticity**:
   - Verify `scripts/verify_live_ide_execution.ps1` executes against real live daemon on port 3031.
   - Verify it extracts genuine compilable Rust code block (`pub struct RrfCombiner;`) and verifies it via `cargo check` in isolated temporary harness.
   - Verify clean cooperative shutdown via `POST /api/shutdown` with zero orphaned lockfiles.
7. **Zero-Touch PATH & Environment Protection (`RULE[environment_safety.md]`)**:
   - Verify zero modifications to user/system PATH or Windows environment registry.
8. **Compiler & Linter Verification**:
   - Verify workspace compiles cleanly with `cargo check --workspace` and `cargo clippy --workspace --tests -- -D warnings`.

## Output Requirements
Write `handoff.md` in your working directory with:
- Observation (forensic evidence across all 8 checks)
- Logic Chain
- Integrity Forensics Checklist (Check 1 to Check 8)
- Conclusion
- **Verdict**: `CLEAN` or `INTEGRITY VIOLATION`
Notify orchestrator via `send_message`.
