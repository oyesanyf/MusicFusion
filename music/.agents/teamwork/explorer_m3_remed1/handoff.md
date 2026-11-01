# Milestone 3 Remediation Analysis: Path Security & Windows Reserved Device Names

**Agent**: `explorer_m3_remed1` (Milestone 3 Remediation Explorer 1 — Path Security & Windows Reserved Device Names)  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m3_remed1\`  
**Timestamp**: 2026-11-01T03:56:00Z  
**Recipient**: `4caf3f77-b3ac-4b67-8bda-934a91faafab` (Orchestrator)  
**Target Crate**: `crates/music_pipeline`  
**Status**: Investigation Complete — Ready for Implementation  

---

## 1. Observation

### 1.1 Direct Source Code Observations
1. **`crates/music_pipeline/src/checkpointer.rs` (lines 68–99)**:
   ```rust
   pub fn sanitize_path_component(component: &str) -> Result<String> {
       let comp_lower = component.to_ascii_lowercase();
       if component.is_empty()
           || component.contains('/')
           || component.contains('\\')
           || component.contains(':')
           || component.contains('\0')
           || component.contains("..")
           || comp_lower.contains("%2e")
           || comp_lower.contains("%2f")
           || comp_lower.contains("%5c")
           || comp_lower.contains("%00")
       {
           return Err(PipelineError::SecurityError(format!(
               "Invalid path component: '{}'",
               component
           )));
       }

       let p = Path::new(component);
       for c in p.components() {
           if !matches!(c, Component::Normal(_)) {
               return Err(PipelineError::SecurityError(format!(
                   "Path traversal attempt rejected in component: '{}'",
                   component
               )));
           }
       }

       Ok(component.to_string())
   }
   ```
   - **Finding**: `sanitize_path_component` lacks any validation for Windows reserved DOS device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, `LPT1` through `LPT9`), case-insensitively, standalone or with file extensions.
   - **Finding**: `sanitize_path_component` lacks validation for Windows filesystem quirks such as trailing periods (`.ends_with('.')`) and trailing spaces (`.ends_with(' ')`).

2. **`crates/music_pipeline/tests/graph_tests.rs` (lines 223–236)**:
   ```rust
   let bad_ids = [
       "../escape",
       "..\\escape",
       "c:/windows/system32",
       "foo/bar",
       "foo\\bar",
       "foo:bar",
       "foo\0bar",
       "foo%2ebar",
       "foo%2fbar",
       "foo%5cbar",
       "",
   ];
   ```
   - **Finding**: `bad_ids` omitted all Windows reserved device names (`COM1`, `CON`, `NUL`, etc.) and trailing dot/space quirks.

3. **`crates/music_pipeline/tests/challenger_stress_tests.rs` (lines 323–373)**:
   - Includes 27 Windows reserved device name attack vectors:
     `["COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CON", "PRN", "AUX", "NUL", "com1", "con", "prn", "aux", "nul"]`.

### 1.2 Tool Commands and Empirical Verification
- **Command**:
  ```powershell
  cargo test -p music_pipeline --test challenger_stress_tests
  ```
- **Exit Code**: `1` (Failure)
- **Verbatim Error Output**:
  ```
  running 12 tests
  test stress_test_add_edge_start_to_nonexistent_node_rejects ... ok
  test stress_test_conditional_edge_to_unregistered_branch_destination_rejects ... ok
  test stress_test_edge_to_unregistered_destination_rejects ... ok
  test stress_test_entry_point_nonexistent_node_rejects ... ok
  test stress_test_missing_entry_point_rejects_immediately ... ok
  test stress_test_configurable_max_iterations_boundaries ... ok
  test stress_test_runtime_unregistered_branch_key_fails_closed ... ok
  test stress_test_conditional_ping_pong_loop_hits_limit ... ok
  test stress_test_single_node_self_cycle_hits_default_50_limit ... ok
  test stress_test_multi_node_cycle_hits_default_50_limit ... ok
  test stress_test_checkpoint_path_injection_and_traversal_attacks ... FAILED
  test stress_test_concurrent_multi_song_checkpoint_isolation ... ok

  failures:

  ---- stress_test_checkpoint_path_injection_and_traversal_attacks stdout ----

  thread 'stress_test_checkpoint_path_injection_and_traversal_attacks' (37448) panicked at crates\music_pipeline\tests\challenger_stress_tests.rs:368:5:
  The following malicious components bypassed sanitize_path_component: ["COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CON", "PRN", "AUX", "NUL", "com1", "con", "prn", "aux", "nul"]
  ```

---

## 2. Logic Chain

1. **Root Cause Identification**:
   - `std::path::Path::new("COM1").components()` treats `"COM1"` as `Component::Normal("COM1")`.
   - On the Windows Win32 API layer, creating or opening files/directories named `COM1` or `CON` (or `COM1.wav`, `CON.txt`, etc.) references the legacy MS-DOS device namespace (`\\.\COM1`, `\\.\CON`).
   - If an uncontained `song_id` or `stem_name` bypasses sanitization, `CheckpointManager::save_checkpoint` invokes `std::fs::create_dir_all` or `CheckpointManager::save_stem` invokes `buffer.to_wav_file_with_format` on these names, risking device lockup or file system corruption.

2. **Windows Reserved Device Name Surface**:
   - Exactly 22 reserved device base names exist:
     - 4 console / printer / aux devices: `con`, `prn`, `aux`, `nul`
     - 9 communication ports: `com1` through `com9`
     - 9 line printer ports: `lpt1` through `lpt9`
   - In Windows, these names are reserved:
     - Regardless of casing (`COM1`, `com1`, `CoM1`, etc.).
     - Regardless of file extension (`COM1.wav`, `con.json`, `nul.dat`, etc.). Windows evaluates the base stem before the first dot.
     - With trailing periods or trailing spaces (`foo.`, `foo `), which Windows Win32 APIs strip or alias.

3. **Mathematical Sanitization Invariants**:
   - Step 1: Reject empty string, slashes (`/`, `\`), colons (`:`), null bytes (`\0`), directory traversal (`..`), percent-encoded traversals (`%2e`, `%2f`, `%5c`, `%00`), and components ending in `.` or ` `.
   - Step 2: Extract base file stem before the first dot (`comp_lower.split('.').next().unwrap_or(&comp_lower)`).
   - Step 3: Check if `base_stem` is contained in `WINDOWS_RESERVED_NAMES`. If so, fail closed with `PipelineError::SecurityError(format!("Windows reserved device name rejected: '{}'", component))`.
   - Step 4: Ensure all `Path::new(component).components()` match only `Component::Normal(_)`.
   - Step 5: Return `Ok(component.to_string())`.

4. **Impact on Test Harness**:
   - In `crates/music_pipeline/tests/challenger_stress_tests.rs`:
     `stress_test_checkpoint_path_injection_and_traversal_attacks` tests each component through `sanitize_path_component`, `version_dir`, and `stems_dir`.
     With the 22 reserved names rejected, all 27 failing components return `Err(...)`, `passed_sanitization` remains empty, and the assertion passes cleanly.
   - In `crates/music_pipeline/tests/graph_tests.rs`:
     Expanding `test_checkpoint_manager_path_traversal_defense` to assert all 22 device names in upper/lower case and with extensions permanently closes the self-certifying test hole.

---

## 3. Caveats

- **Scope Boundary**: This investigation is strictly read-only and does NOT edit production code or test files directly. Implementation is delegated to the remediation worker.
- **Valid Song Identifiers**: Names that start with reserved substrings but are valid distinct words (e.g., `"control"`, `"connect"`, `"auxiliary"`, `"nullify"`) have different base stems (`"control" != "con"`) and will continue to be safely accepted as valid IDs.
- **Fail-Closed Execution**: In `crates/music_pipeline/src/graph.rs` (lines 250–254), `CompiledGraph::execute_internal` currently logs a warning if `save_checkpoint` fails. While not breaking tests, fail-closed security dictates that fatal `PipelineError::SecurityError` should immediately abort execution rather than continue synthesis.

---

## 4. Conclusion & Concrete Remediation Plan

### 4.1 Exact Code Changes

#### Change 1: `crates/music_pipeline/src/checkpointer.rs`
**Target lines**: 68–99  
**Action**: Replace `sanitize_path_component` with the following robust implementation:

```rust
    /// Validates path component preventing directory traversal, NT ADS breakouts,
    /// Windows reserved device names, and filesystem quirks.
    pub fn sanitize_path_component(component: &str) -> Result<String> {
        let comp_lower = component.to_ascii_lowercase();
        if component.is_empty()
            || component.contains('/')
            || component.contains('\\')
            || component.contains(':')
            || component.contains('\0')
            || component.contains("..")
            || comp_lower.contains("%2e")
            || comp_lower.contains("%2f")
            || comp_lower.contains("%5c")
            || comp_lower.contains("%00")
            || component.ends_with('.')
            || component.ends_with(' ')
        {
            return Err(PipelineError::SecurityError(format!(
                "Invalid path component: '{}'",
                component
            )));
        }

        // Windows reserved DOS device names (case-insensitive, alone or with any extension)
        const WINDOWS_RESERVED_NAMES: &[&str] = &[
            "con", "prn", "aux", "nul",
            "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
            "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];

        let base_stem = comp_lower.split('.').next().unwrap_or(&comp_lower);
        if WINDOWS_RESERVED_NAMES.contains(&base_stem) {
            return Err(PipelineError::SecurityError(format!(
                "Windows reserved device name rejected: '{}'",
                component
            )));
        }

        let p = Path::new(component);
        for c in p.components() {
            if !matches!(c, Component::Normal(_)) {
                return Err(PipelineError::SecurityError(format!(
                    "Path traversal attempt rejected in component: '{}'",
                    component
                )));
            }
        }

        Ok(component.to_string())
    }
```

#### Change 2: `crates/music_pipeline/tests/graph_tests.rs`
**Target lines**: 218–256  
**Action**: Replace `test_checkpoint_manager_path_traversal_defense` with comprehensive test coverage:

```rust
#[tokio::test]
async fn test_checkpoint_manager_path_traversal_defense() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let bad_ids = [
        // Standard path traversal
        "../escape",
        "..\\escape",
        "../../etc/passwd",
        "c:/windows/system32",
        "C:\\Windows\\System32",
        "foo/bar",
        "foo\\bar",
        "foo:bar",
        "foo\0bar",
        "foo%2ebar",
        "foo%2fbar",
        "foo%5cbar",
        "foo%00bar",
        "",
        // Windows filesystem quirks (trailing dot / space)
        "foo.",
        "foo ",
        "song.",
        "song ",
        // Windows reserved device names (uppercase)
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "COM1",
        "COM2",
        "COM3",
        "COM4",
        "COM5",
        "COM6",
        "COM7",
        "COM8",
        "COM9",
        "LPT1",
        "LPT2",
        "LPT3",
        "LPT4",
        "LPT5",
        "LPT6",
        "LPT7",
        "LPT8",
        "LPT9",
        // Windows reserved device names (lowercase)
        "con",
        "prn",
        "aux",
        "nul",
        "com1",
        "com2",
        "com3",
        "com4",
        "com5",
        "com6",
        "com7",
        "com8",
        "com9",
        "lpt1",
        "lpt2",
        "lpt3",
        "lpt4",
        "lpt5",
        "lpt6",
        "lpt7",
        "lpt8",
        "lpt9",
        // Windows reserved device names with extensions
        "CON.txt",
        "prn.wav",
        "aux.json",
        "nul.dat",
        "com1.wav",
        "COM2.state",
        "LPT1.log",
    ];

    for bad in &bad_ids {
        let err = CheckpointManager::sanitize_path_component(bad);
        assert!(
            err.is_err(),
            "Component '{}' should be rejected by path traversal defense",
            bad
        );
        assert!(
            matches!(err.unwrap_err(), PipelineError::SecurityError(_)),
            "Component '{}' should return PipelineError::SecurityError",
            bad
        );

        let res = checkpointer.version_dir(bad, 1);
        assert!(res.is_err(), "version_dir for '{}' should be rejected", bad);

        let stems_res = checkpointer.stems_dir(bad, 1);
        assert!(stems_res.is_err(), "stems_dir for '{}' should be rejected", bad);
    }

    let good_ids = [
        "valid-song-id",
        "song_12345",
        "test-track-v1",
        "uuid-a1b2-c3d4",
        "roots-reggae-anthem",
        "afrobeats-groove-001",
        "raspy_tenor_vocals",
        "drums",
    ];
    for good in &good_ids {
        assert!(
            CheckpointManager::sanitize_path_component(good).is_ok(),
            "Component '{}' should be accepted",
            good
        );
    }
}
```

#### Secondary Security Hardening Recommendations
1. **`crates/music_pipeline/src/graph.rs` (lines 250–254)**:
   ```rust
   if let Some(cp) = checkpointer {
       if let Err(e) = cp.save_checkpoint(state, &current_node).await {
           if matches!(e, PipelineError::SecurityError(_)) {
               return Err(e);
           }
           log::warn!("Checkpointer warning at node {}: {}", current_node, e);
       }
   }
   ```
2. **`crates/music_pipeline/src/checkpointer.rs` (lines 31–34)**:
   Enable WAL mode and busy timeout on SQLite connection:
   ```rust
   let conn = rusqlite::Connection::open(&db_path)
       .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
   conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")
       .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
   ```

---

## 5. Verification Method

Once implemented by the worker, run the following commands to independently verify:

1. **Verify Challenger Stress Test**:
   ```powershell
   cargo test -p music_pipeline --test challenger_stress_tests -- stress_test_checkpoint_path_injection_and_traversal_attacks
   ```
   *Expected Result*: `test stress_test_checkpoint_path_injection_and_traversal_attacks ... ok` (1 passed, 0 failed).

2. **Verify All Challenger Stress Tests**:
   ```powershell
   cargo test -p music_pipeline --test challenger_stress_tests
   ```
   *Expected Result*: `12 passed; 0 failed; 0 ignored; finished in ~1.5s`.

3. **Verify Pipeline Graph Tests**:
   ```powershell
   cargo test -p music_pipeline --test graph_tests
   ```
   *Expected Result*: `12 passed; 0 failed; 0 ignored`.

4. **Verify Entire Workspace**:
   ```powershell
   cargo test --workspace
   ```
   *Expected Result*: All workspace tests compile and pass with 0 failures.

### Invalidation Conditions
- Any component in `WINDOWS_RESERVED_NAMES` returning `Ok(_)`.
- Any component ending with `.` or ` ` returning `Ok(_)`.
- Any valid song ID or stem name (e.g., `"valid-song-id"`, `"drums"`, `"raspy_tenor_vocals"`) returning `Err(_)`.
