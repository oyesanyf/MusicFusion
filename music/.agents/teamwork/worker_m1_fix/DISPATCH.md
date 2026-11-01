## 2026-11-01T01:46:52Z
You are a Worker subagent in a Teamwork hierarchy for Milestone 1 Iteration 2 (Remediation).
Your working directory is: d:\harfile\music\.agents\teamwork\worker_m1_fix\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md
and the gate failure review at:
d:\harfile\music\.agents\teamwork\reviewer_m1_1\handoff.md
and the complete remediation plan and exact code diffs at:
d:\harfile\music\.agents\teamwork\explorer_m1_fix3\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

EXCLUSIVE WRITE OWNERSHIP:
You exclusively own:
- crates/music_fusion/src/memory.rs
- crates/music_fusion/tests/fusion_tests.rs
- crates/music_fusion/tests/adversarial_fusion_stress_tests.rs

YOUR MISSION:
Apply the exact thread-safe remediation designed by explorer_m1_fix3:
1. In `crates/music_fusion/src/memory.rs`:
   - Add `pub force_dsp: bool` and `pub force_hf: bool` to `SystemMemory`.
   - In `detect_live()`, read `MUSIC_FORCE_DSP` and `MUSIC_FORCE_HF` from environment.
   - In `simulated()`, default both to `false`.
   - Add builder methods: `pub fn with_force_dsp(mut self, force: bool) -> Self` and `pub fn with_force_hf(mut self, force: bool) -> Self`.
   - In `route_stage()`, check `self.force_dsp`, `self.force_hf`, `engine_mode == "pure_rust"`, etc., without calling `std::env::var` inside `route_stage()`.
2. In `crates/music_fusion/tests/fusion_tests.rs`:
   - Eliminate `std::env::set_var` and `std::env::remove_var`. Use `.with_force_dsp(true)` or `"pure_rust"`.
3. In `crates/music_fusion/tests/adversarial_fusion_stress_tests.rs`:
   - Eliminate `std::env::set_var` and `std::env::remove_var`. Use `.with_force_dsp(true)`.
4. Verification:
   - Run `cargo check --workspace`
   - Run a 10-iteration loop: `cargo test -p music_core -p music_fusion -- --test-threads=16`
   - Run `cargo test --workspace`
   - Run `cargo test --manifest-path tests/Cargo.toml`
   - Verify 100% of tests pass on every single run.
5. Commit:
   - Stage and commit the fix cleanly with git.

Write your completion report into:
d:\harfile\music\.agents\teamwork\worker_m1_fix\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\worker_m1_fix\progress.md.
When finished, send a brief message with your handoff path.
