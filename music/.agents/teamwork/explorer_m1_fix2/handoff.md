# Handoff Report: Thread Safety, Race Prevention, and Test Environment Isolation

**Author**: Explorer Subagent (`explorer_m1_fix2`)  
**Parent**: `4caf3f77-b3ac-4b67-8bda-934a91faafab`  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m1_fix2\`  
**Scope**: Workspace-wide test isolation analysis across `music_core`, `music_fusion`, `music_dsp`, `music_pipeline`, `music_cli`, and integration suites in `d:\harfile\music`.

---

## 1. Observation

### 1.1 Direct Reproduction of Test Failure
Running standard multi-threaded test execution via:
```powershell
cargo test -p music_core -p music_fusion
```
produces an intermittent panic in `fusion_tests.rs`:
```
running 12 tests
test test_dynamic_available_memory_law_tiers ... ok
test test_judge_json_parsing_with_markdown_fences ... ok
test test_judge_json_parsing_with_think_tags ... ok
test test_judge_json_fail_safe_fallback ... ok
test test_stage_routing_force_dsp_override ... ok
test test_specialized_model_gating ... ok
test test_stage_routing_gpu_vs_dsp ... FAILED
test test_missing_model_guidance ... ok
test test_probe_health ... ok
test test_list_installed_models ... ok
test test_producer_panel_offline_fallback ... ok
test test_live_memory_detection ... ok

failures:

---- test_stage_routing_gpu_vs_dsp stdout ----

thread 'test_stage_routing_gpu_vs_dsp' (43180) panicked at crates\music_fusion\tests\fusion_tests.rs:58:5:
assertion `left == right` failed
  left: PureRustDsp
 right: HuggingFaceGpu
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    test_stage_routing_gpu_vs_dsp

test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s
```

### 1.2 Identified Locations of Process Environment Mutation
A comprehensive workspace-wide regex/grep search (`std::env::set_var`, `std::env::remove_var`, `std::env::var`) revealed exact locations:

1. **`crates/music_fusion/tests/fusion_tests.rs` (lines 73–81)**:
   ```rust
   #[test]
   fn test_stage_routing_force_dsp_override() {
       std::env::set_var("MUSIC_FORCE_DSP", "1");
       let mem = SystemMemory::simulated(64.0, 48.0, 16, Some("RTX 4090".into()), 24.0, 20.0);
       assert_eq!(
           mem.route_stage(PipelineStageKind::Stage1Groove, "auto"),
           ExecutionRoute::PureRustDsp
       );
       std::env::remove_var("MUSIC_FORCE_DSP");
   }
   ```

2. **`crates/music_fusion/tests/adversarial_fusion_stress_tests.rs` (lines 273–286)**:
   ```rust
   // MUSIC_FORCE_DSP=1 env override forces PureRustDsp regardless of engine_mode
   std::env::set_var("MUSIC_FORCE_DSP", "1");
   for stage in &all_stages {
       assert_eq!(
           mem_rtx4090.route_stage(*stage, "auto"),
           ExecutionRoute::PureRustDsp
       );
       assert_eq!(
           mem_rtx4090.route_stage(*stage, "hf_hybrid"),
           ExecutionRoute::PureRustDsp
       );
   }
   std::env::remove_var("MUSIC_FORCE_DSP");
   ```

3. **`crates/music_fusion/src/memory.rs` (lines 223–245)**:
   ```rust
   pub fn route_stage(&self, stage: PipelineStageKind, engine_mode: &str) -> ExecutionRoute {
       if engine_mode == "pure_rust" || std::env::var("MUSIC_FORCE_DSP").is_ok() {
           return ExecutionRoute::PureRustDsp;
       }

       let required_vram = stage.required_vram_gb();
       let required_ram = stage.required_ram_gb();

       // 1. Discrete GPU with sufficient VRAM budget
       if self.has_usable_gpu() && self.gpu_budget_gb() >= required_vram {
           return ExecutionRoute::HuggingFaceGpu;
       }

       // 2. Forced HF or hybrid engine with sufficient RAM budget
       if (engine_mode == "hf_hybrid" || std::env::var("MUSIC_FORCE_HF").is_ok())
           && self.ram_budget_gb() >= required_ram
       {
           return ExecutionRoute::HuggingFaceCpu;
       }

       // 3. Default safe fail-closed path: Route to internal pure-Rust 48 kHz DSP engine
       ExecutionRoute::PureRustDsp
   }
   ```

4. **`crates/music_fusion/src/ollama_client.rs` (line 125)**:
   ```rust
   pub fn from_env() -> Self {
       let ep = std::env::var("LOCAL_OLLAMA_ENDPOINT")
           .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
       Self::new(ep)
   }
   ```
   *Note*: No test calls `set_var("LOCAL_OLLAMA_ENDPOINT", ...)`. Tests construct clients directly via `OllamaClient::new(endpoint)`, which is already isolated and thread-safe.

5. **`crates/music_core/tests/` (`core_tests.rs` and `adversarial_stress_tests.rs`)**:
   - Total tests: 27 (7 unit + 20 stress).
   - Zero environment calls, zero mutable statics, zero file I/O collisions. 100% deterministic and thread-safe.

6. **`tests/` E2E suite (`tests/common/mod.rs` and 191 E2E tests)**:
   - Zero environment variable mutations. All audio processing is purely in-memory IEEE float WAV buffers.

---

## 2. Logic Chain

1. **Cargo Test Execution Model**: By default, Cargo compiles integration test files (e.g. `tests/fusion_tests.rs` and `tests/adversarial_fusion_stress_tests.rs`) into standalone executables and runs all test functions concurrently across a thread pool (typically equal to logical CPU cores, e.g. 16 or 24 threads).
2. **Global Process State**: In standard OS runtimes (Windows PEB and POSIX `environ`), environment variables are process-wide. A call to `std::env::set_var` alters the environment table for all threads in the running process. Furthermore, standard library documentation explicitly notes that `std::env::set_var` is not thread-safe and is marked `unsafe` in modern Rust editions.
3. **The Race Condition**:
   - Thread 1 begins running `test_stage_routing_force_dsp_override` and executes `std::env::set_var("MUSIC_FORCE_DSP", "1")`.
   - Thread 2 concurrently executes `test_stage_routing_gpu_vs_dsp`, which evaluates:
     ```rust
     mem_gpu.route_stage(PipelineStageKind::Stage2YuE, "auto")
     ```
   - Inside `route_stage`, line 224 evaluates `std::env::var("MUSIC_FORCE_DSP").is_ok()`. Because Thread 1 set the variable in the shared process environment, the expression evaluates to `true`.
   - `route_stage` returns `ExecutionRoute::PureRustDsp` early, bypassing GPU VRAM checks.
   - Thread 2 executes `assert_eq!(left, right)` where `left = PureRustDsp` and `right = HuggingFaceGpu`, failing with a panic.
   - When run sequentially (`--test-threads=1`), Thread 1 and Thread 2 never overlap in time, which is why the bug did not appear under serialized test runs.
4. **Secondary Collision Hazard**:
   - In `adversarial_fusion_stress_tests.rs:274`, `test_stress_pure_rust_and_env_force_overrides` similarly calls `std::env::set_var("MUSIC_FORCE_DSP", "1")`.
   - Sibling test `test_stress_execution_routing_all_stages_under_auto_mode` runs across all 5 stages expecting `HuggingFaceGpu` under high VRAM. If scheduled concurrently with `test_stress_pure_rust_and_env_force_overrides`, it faces the identical race condition.

---

## 3. Evaluation of Design Patterns

| Pattern | Mechanism | Pros | Cons | Verdict |
|---|---|---|---|---|
| **A. Global Shared Mutex (`ENV_MUTEX`)** | `static ENV_MUTEX: Mutex<()>` acquired by tests touching env | Allows keeping `set_var` in tests | Fragile: EVERY test calling `route_stage` (dozens of tests) must acquire the mutex, or un-synchronized tests will still race. High serialization penalty. | ❌ Rejected |
| **B. CLI Flag Serializer (`--test-threads=1`)** | Force serial execution via `cargo test -- --test-threads=1` | Zero code changes | Fails project requirement (must pass under default `cargo test`). Hides concurrency bugs rather than fixing them. Slows CI. | ❌ Rejected |
| **C. Typed Functional Isolation (`RouteOverrides`)** | Introduce `RouteOverrides` struct with `route_stage_with_overrides`; leave `route_stage` calling live env for runtime compatibility; tests pass typed overrides without mutating process env | 100% thread-safe; zero locks; deterministic on 1 to 128 threads; 100% backward compatible; complies with `RULE[environment_safety.md]` | None | **✅ Recommended** |

---

## 4. Proposed Solution & Code Changes

### 4.1 Target File 1: `crates/music_fusion/src/memory.rs`
Introduce a typed `RouteOverrides` configuration struct and a pure `route_stage_with_overrides` method. `route_stage` delegates to it using `RouteOverrides::from_env()`:

```rust
// ----------------------------------------------------------------------------
// ADDITION in crates/music_fusion/src/memory.rs
// ----------------------------------------------------------------------------

/// Optional routing overrides configured programmatically or read from environment variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RouteOverrides {
    pub force_dsp: bool,
    pub force_hf: bool,
}

impl RouteOverrides {
    /// Inspect environment variables `MUSIC_FORCE_DSP` and `MUSIC_FORCE_HF`.
    pub fn from_env() -> Self {
        Self {
            force_dsp: std::env::var("MUSIC_FORCE_DSP").is_ok(),
            force_hf: std::env::var("MUSIC_FORCE_HF").is_ok(),
        }
    }

    /// Default: no manual overrides.
    pub fn none() -> Self {
        Self::default()
    }

    /// Explicitly force internal pure-Rust DSP engine.
    pub fn force_dsp() -> Self {
        Self {
            force_dsp: true,
            force_hf: false,
        }
    }

    /// Explicitly force Hugging Face CPU/GPU execution.
    pub fn force_hf() -> Self {
        Self {
            force_dsp: false,
            force_hf: true,
        }
    }
}
```

Refactor `route_stage` (lines 223–245):
```rust
    /// Decide execution routing with explicit overrides (thread-safe, zero process environment mutation).
    pub fn route_stage_with_overrides(
        &self,
        stage: PipelineStageKind,
        engine_mode: &str,
        overrides: RouteOverrides,
    ) -> ExecutionRoute {
        if engine_mode == "pure_rust" || overrides.force_dsp {
            return ExecutionRoute::PureRustDsp;
        }

        let required_vram = stage.required_vram_gb();
        let required_ram = stage.required_ram_gb();

        // 1. Discrete GPU with sufficient VRAM budget
        if self.has_usable_gpu() && self.gpu_budget_gb() >= required_vram {
            return ExecutionRoute::HuggingFaceGpu;
        }

        // 2. Forced HF or hybrid engine with sufficient RAM budget
        if (engine_mode == "hf_hybrid" || overrides.force_hf)
            && self.ram_budget_gb() >= required_ram
        {
            return ExecutionRoute::HuggingFaceCpu;
        }

        // 3. Default safe fail-closed path: Route to internal pure-Rust 48 kHz DSP engine
        ExecutionRoute::PureRustDsp
    }

    /// Decide execution routing for a music pipeline stage based on live hardware resources.
    /// Supports manual override via `MUSIC_FORCE_DSP=1` or `MUSIC_FORCE_HF=1` in the environment.
    pub fn route_stage(&self, stage: PipelineStageKind, engine_mode: &str) -> ExecutionRoute {
        self.route_stage_with_overrides(stage, engine_mode, RouteOverrides::from_env())
    }
```

Export `RouteOverrides` in `crates/music_fusion/src/lib.rs`:
```rust
pub use memory::{
    create_hidden_command, ExecutionRoute, PipelineStageKind, RouteOverrides, StageType, SystemMemory,
};
```

---

### 4.2 Target File 2: `crates/music_fusion/tests/fusion_tests.rs`
Update `test_stage_routing_force_dsp_override` (lines 72–81) to eliminate `std::env::set_var`:

**Before**:
```rust
#[test]
fn test_stage_routing_force_dsp_override() {
    std::env::set_var("MUSIC_FORCE_DSP", "1");
    let mem = SystemMemory::simulated(64.0, 48.0, 16, Some("RTX 4090".into()), 24.0, 20.0);
    assert_eq!(
        mem.route_stage(PipelineStageKind::Stage1Groove, "auto"),
        ExecutionRoute::PureRustDsp
    );
    std::env::remove_var("MUSIC_FORCE_DSP");
}
```

**After**:
```rust
#[test]
fn test_stage_routing_force_dsp_override() {
    let mem = SystemMemory::simulated(64.0, 48.0, 16, Some("RTX 4090".into()), 24.0, 20.0);
    // 1. Engine mode "pure_rust" forces PureRustDsp even with 20 GB free VRAM
    assert_eq!(
        mem.route_stage(PipelineStageKind::Stage1Groove, "pure_rust"),
        ExecutionRoute::PureRustDsp
    );
    // 2. Explicit RouteOverrides::force_dsp() forces PureRustDsp under "auto" mode without mutating global process state
    assert_eq!(
        mem.route_stage_with_overrides(
            PipelineStageKind::Stage1Groove,
            "auto",
            music_fusion::RouteOverrides::force_dsp()
        ),
        ExecutionRoute::PureRustDsp
    );
}
```

---

### 4.3 Target File 3: `crates/music_fusion/tests/adversarial_fusion_stress_tests.rs`
Update `test_stress_pure_rust_and_env_force_overrides` (lines 253–286) to eliminate `std::env::set_var`:

**Before**:
```rust
    // MUSIC_FORCE_DSP=1 env override forces PureRustDsp regardless of engine_mode
    std::env::set_var("MUSIC_FORCE_DSP", "1");
    for stage in &all_stages {
        assert_eq!(
            mem_rtx4090.route_stage(*stage, "auto"),
            ExecutionRoute::PureRustDsp
        );
        assert_eq!(
            mem_rtx4090.route_stage(*stage, "hf_hybrid"),
            ExecutionRoute::PureRustDsp
        );
    }
    std::env::remove_var("MUSIC_FORCE_DSP");
```

**After**:
```rust
    // RouteOverrides::force_dsp() forces PureRustDsp regardless of engine_mode without mutating global process state
    let force_dsp = music_fusion::RouteOverrides::force_dsp();
    for stage in &all_stages {
        assert_eq!(
            mem_rtx4090.route_stage_with_overrides(*stage, "auto", force_dsp),
            ExecutionRoute::PureRustDsp
        );
        assert_eq!(
            mem_rtx4090.route_stage_with_overrides(*stage, "hf_hybrid", force_dsp),
            ExecutionRoute::PureRustDsp
        );
    }
```

---

## 5. Caveats
- No other test suites in `crates/music_core/tests/` or `tests/` mutate shared environment or process state.
- No production APIs are broken: `SystemMemory::route_stage(stage, engine_mode)` retains its exact signature and behavior for all callers in `music_pipeline` and `music_cli`.
- `RouteOverrides::from_env()` ensures that deployment scripts or CLI invocations utilizing `MUSIC_FORCE_DSP=1` or `MUSIC_FORCE_HF=1` in external production environments continue to function seamlessly.

---

## 6. Conclusion
The root cause of the Milestone 1 gate failure is an environment variable race condition caused by `std::env::set_var` in `fusion_tests.rs` and `adversarial_fusion_stress_tests.rs`. By decoupling override routing into a typed parameter (`RouteOverrides`) and delegating `route_stage` to it, we completely eliminate global state mutation from all test suites while preserving 100% runtime compatibility. This guarantees 100% deterministic test execution under `cargo test -p music_core -p music_fusion` and `cargo test --workspace` across any number of parallel test threads.

---

## 7. Verification Method

Once the changes are applied by the implementer:
1. Run standard multi-threaded test suite:
   ```powershell
   cargo test -p music_core -p music_fusion
   ```
2. Run stress loop across 10 consecutive executions to verify zero flakiness:
   ```powershell
   cmd /c "for /l %x in (1, 1, 10) do cargo test -p music_fusion --test fusion_tests"
   ```
3. Run full workspace check and E2E suite:
   ```powershell
   cargo check --workspace
   cargo test --workspace
   cargo test --manifest-path tests/Cargo.toml
   ```
4. Invalidation condition: Any panic or non-zero exit code across any of the 10 loop runs.
