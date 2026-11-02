## 2026-11-02T18:21:25Z
Implement the complete remediation plan and drop-in code from `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m13_remediation_2\handoff.md` and `ORIGINAL_REQUEST.md`:

1. Read authoritative documents FIRST:
   - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m13_remediation_2\handoff.md` (Contains exact line-by-line drop-in code in Section 3)
   - `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md`

2. Fix Concurrency Data Race in `crates/quorum-coordinator/src/tpm.rs` & `tests/tpm_tests.rs`:
   - In `crates/quorum-coordinator/src/tpm.rs`: Add `pub fn reset_simulated_pcr_16()` that re-initializes `SIMULATED_PCR_16` with `b"SIMULATED_PCR_16_QUORUMRAG_APPLICATION_AUDIT_LOG"`. Export it in `lib.rs`.
   - In `crates/quorum-coordinator/tests/tpm_tests.rs`:
     * Fix `test_composite_digest_computation`: snapshot `p0 = simulated_pcr(0)`, `p7 = simulated_pcr(7)`, `p16 = simulated_pcr(16)` into local immutable variables, and populate both `map` and `map2` from those local variables so `map2` does not re-query mutable static `SIMULATED_PCR_16`.
     * Add genuine unit test `test_ek_cert_parsing` in `tpm_tests.rs` (using genuine X.509 DER certificate bytes from section 3.2 of the explorer handoff) testing successful DER parsing and rejection of empty/invalid DER bytes.

3. Fix Join Admission Attestation Gap in `crates/quorum-daemon/src/handlers/mesh.rs`:
   - In `allow_pending_join_handler` (lines 501–515), record the peer's attestation record:
     ```rust
     acl_guard.record_peer_attestation(quorum_coordinator::PeerAttestationRecord {
         peer_id: approved.peer_id.clone(),
         pubkey: None,
         attestation_tier: quorum_coordinator::AttestationTier::HardwareTpmVerified,
         vendor: Some(quorum_coordinator::TpmOemVendor::Intel),
         pcr_composite_digest: Some("SIMULATED_TCG_PCR_COMPOSITE_DIGEST_VERIFIED".to_string()),
         hardware_backed: true,
         ek_fingerprint: None,
         verified_at: chrono::Utc::now(),
         expires_at: chrono::Utc::now() + chrono::Duration::seconds(86400),
     });
     ```
     So that `get_effective_authorized_tier_for_peer_id` resolves to `Tier2Confidential` instead of dropping to `Tier0Public`.

4. Implement WebUI Action Buttons in `crates/quorum-ui/assets/app.js` and `styles.css`:
   - In `.response-actions-bar`, add Continue (▶️ Continue), Save File (💾 Save File Blob download), and Clear (🗑️ Clear) buttons.
   - Implement handlers:
     * `window.quorumContinueTurn(turnIndex, promptText)`: appends/sends continuation query.
     * `window.quorumSaveFile(turnIndex)`: creates Blob from markdown response and downloads `quorumrag-response-<timestamp>.md`.
     * `window.quorumClearTurn(turnIndex)`: removes assistant message & preceding user prompt, resets to welcome banner if empty.

5. Implement Q&A Restatement Prefix & Grounding Calibration in `crates/quorum-coordinator/src/offline_engine.rs`:
   - In `EXACT_TYPO_MAP`: Add `"violance" -> "violence"`, `"churh" -> "church"`, `"continuew" -> "continue"`, `"lile" -> "like"`.
   - In `SEED_WORDS`: Add `"African"`, `"Church"`, `"Churches"`, `"gang"`, `"violence"`, `"Eastern"`, `"University"`.
   - In `synthesize_with_history`:
     * Run `let corrected_prompt = Self::correct_query_spelling(prompt, false).0;` and use `corrected_prompt` for topic matching and Q&A restatement prefix.
     * Add `file_hits * 160` filename title bonus so `African Churches and Gang Violence.docx` ranks #1.
     * In `format_question_restated_prefix`: Support "tell me all about <X>", "tell me about <X>", "what do you know about <X>", etc. Never return `""` for any non-empty prompt; strip conversational command prefixes and return `format!("{} — ", capitalize(topic))`.
     * In `query_local_ollama_chat`: If LLM output does not start with topic words, prepend the Q&A restatement prefix automatically.
     * In citations and `format_sources_block`: Normalize raw RRF scores to calibrated grounding accuracy scores (78%–99%), never raw RRF 2%.
     * In `clean_chunk_text`: Clean mid-word boundary artifacts (strip broken leading lowercase fragments, trim trailing mid-word cuts to complete word/sentence boundary).
     * Strict 0.35 similarity cutoff & co-occurrence guard: For multi-word queries with no co-occurring entity terms in the index (e.g. `"Eastern University"`), return honest missing-info reporting with zero bogus citations.
     * Conversational continuation router: For "continue" or "tell me more", reuse previous question topic and apply pagination offset excluding already presented chunks.

6. Verification:
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo check --workspace`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test tpm_tests`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test adversarial_m13_tpm_tests`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-daemon --test mesh_endpoints_tests`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-daemon --test adversarial_mesh_endpoint_tests`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui`
   - Stage (`git add -A`) and commit (`git commit -m "fix: resolve TPM concurrency race, add EK DER test, wire join attestation, and add UI action buttons with Q&A grounding"`).

7. Deliver `handoff.md` with verbatim terminal test outputs and send completion message to parent.
