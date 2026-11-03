# Empirical Challenge Report: Milestone M11 - Preferences Contract & Invariants

**Agent**: Challenger M11 2 (`teamwork_challenger`)  
**Roles**: critic, specialist  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\`  
**Date**: 2026-11-03T00:32:00Z  
**Verdict**: **APPROVE**  

---

## Challenge Summary

**Overall risk assessment**: **LOW**

The Milestone M11 implementation introduces strongly typed configurations, serde alias mapping, WebUI interactive toggles, and Axum daemon routing for Deep Research and Iterative Section RAG. 
An adversarial integration test suite (`crates/quorum-daemon/tests/preferences_contract_challenge.rs`) was constructed and executed to independently stress-test the HTTP routing contract, payload serialization round-trips, serde alias deserialization, default value invariants, and failure modes under malformed and mismatched payloads. All tests passed empirically with zero failures.

---

## Challenges & Stress Scenarios Evaluated

### [Medium] Challenge 1: Route Dual-Method Binding and Alias Parity
- **Assumption challenged**: `/api/preferences` must correctly bind both `GET` and `POST` methods, routing to `get_settings_handler` and `update_settings_handler`, and maintaining state synchronization with `/api/settings`.
- **Attack scenario**: Calling `GET /api/preferences` might return 404/405 if only `POST` or `/api/settings` was wired; or updates to `/api/preferences` might fail to persist to the global controller and subsequent reads from `/api/settings` would return stale configuration.
- **Empirical test**: 
  - Sent `GET /api/preferences` → Returned HTTP 200 OK with valid JSON containing full `ui` preferences matching `GET /api/settings`.
  - Sent `POST /api/preferences` with custom settings → Returned HTTP 200 OK with updated configuration.
  - Followed up with `GET /api/preferences` and `GET /api/settings` → Both confirmed live update persistence.
- **Result**: **PASS**

### [Medium] Challenge 2: Serde Alias Tolerance and Flattened Property Fallback
- **Assumption challenged**: Client requests using alternative naming conventions (`agentic_loop_chaining`, `agentic_loop_chunk_size`, `deep_research`, `parent_doc_retrieval`, `map_reduce`) must deserialize into canonical Rust struct fields rather than being dropped or swallowed into the `#[serde(flatten)] pub extra: HashMap<String, Value>` container.
- **Attack scenario**: If aliases were omitted or mismatched, sending `{"ui": {"deep_research": true}}` would leave `deep_research_iterative_rag` unpopulated or default, silently ignoring user preferences.
- **Empirical test**: Executed `test_api_preferences_get_and_post_lifecycle` and `dlp_mesh::tests::test_ui_preferences_config_serde_aliases` with payloads containing only aliases. Confirmed all fields mapped to canonical struct fields with exact Boolean and numeric values.
- **Result**: **PASS**

### [High] Challenge 3: Invariant Conformance & 10-Budget Serialization Preservation
- **Assumption challenged**: `UiPreferencesConfig` defaults must strictly satisfy all 7 mandatory requirements without deviation:
  - `max_output_tokens` == 262144
  - `agentic_loop_enabled` == true
  - `agentic_chunk_size` == 4096
  - `agentic_strategy` == "auto_continuation"
  - `deep_research_iterative_rag` == true
  - `parent_document_retrieval` == true
  - `map_reduce_fact_extraction` == true
  Furthermore, all 10 token budget dropdown settings (256, 512, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144) must serialize and deserialize without truncation.
- **Attack scenario**: Default configuration might specify an outdated token budget (e.g. 1600 or 4096) or toggle disabled states, breaking the user requirement for 256k agentic loop execution.
- **Empirical test**: Executed `test_ui_preferences_strict_defaults_invariants` verifying exact values and full round-trip for all 10 budgets.
- **Result**: **PASS**

### [Low] Challenge 4: Adversarial Malformed Body and Type Mismatches
- **Assumption challenged**: Malformed JSON bodies or string-to-number type mismatches must fail gracefully with 400 Bad Request or 422 Unprocessable Entity, never causing an unhandled server panic or process crash.
- **Attack scenario**: Sending malformed bytes `{"ui": { broken json` or `"max_output_tokens": "invalid_string_tokens"` to `POST /api/preferences`.
- **Empirical test**: Executed `test_api_preferences_adversarial_malformed_and_type_mismatch`. Returned 400 Bad Request / 422 Unprocessable Entity; server remained completely healthy. Partial update with empty `{"ui": {}}` succeeded cleanly without resetting unmentioned keys.
- **Result**: **PASS**

---

## Stress Test Results

| Scenario | Expected Behavior | Actual Behavior | Result |
|---|---|---|---|
| `GET /api/preferences` | 200 OK, JSON containing `status: "ok"` and `ui` | 200 OK, full `ui` returned with default invariants | **PASS** |
| `POST /api/preferences` (canonical) | 200 OK, updates persisted and verified via subsequent GET | 200 OK, `max_output_tokens: 65536`, `deep_research: false` persisted | **PASS** |
| `POST /api/preferences` (aliases) | 200 OK, aliases (`agentic_loop_chaining`, `deep_research`, etc.) deserialized | 200 OK, canonical struct fields populated correctly | **PASS** |
| Strict Invariants Verification | All 7 invariants exact; 10 budget levels round-trip | All 7 invariants verified; all 10 budgets round-trip | **PASS** |
| Malformed JSON POST | 400 Bad Request / 422 Unprocessable Entity | 400 Bad Request / 422 returned, zero panics | **PASS** |
| Type Mismatch POST | 400 Bad Request / 422 Unprocessable Entity | 400 Bad Request / 422 returned, zero panics | **PASS** |
| Empty Object `{"ui": {}}` | 200 OK, populates default struct cleanly | 200 OK, successful response | **PASS** |

---

## Test Execution Logs

### 1. Standalone Integration Test Suite (`crates/quorum-daemon/tests/preferences_contract_challenge.rs`)
```
running 3 tests
test test_ui_preferences_strict_defaults_invariants ... ok
test test_api_preferences_adversarial_malformed_and_type_mismatch ... ok
test test_api_preferences_get_and_post_lifecycle ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

### 2. Coordinator Unit Tests (`cargo test -p quorum-coordinator --lib dlp_mesh`)
```
running 5 tests
test dlp_mesh::tests::test_mesh_shared_folder_enforcement ... ok
test dlp_mesh::tests::test_ui_preferences_config_defaults ... ok
test dlp_mesh::tests::test_ui_preferences_config_serde_aliases ... ok
test dlp_mesh::tests::test_ui_preferences_config_serde_canonical_and_defaults ... ok
test dlp_mesh::tests::test_phi_redaction_and_blocking_modes ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.09s
```

### 3. Daemon Route & Health Tests (`cargo test -p quorum-daemon --test daemon_tests`)
```
running 2 tests
test test_cli_parsing_default_and_flags ... ok
test test_daemon_server_routes_and_health ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
```

### 4. Workspace Check (`cargo check --workspace`)
```
Finished `dev` profile [unoptimized] target(s) in 1.45s
```

---

## Unchallenged Areas

- Live Ollama token streaming generation across recursive multi-hop hops up to 262k tokens: Evaluated under Milestone M12 / M13 / M14 scope and runtime worker processes.
