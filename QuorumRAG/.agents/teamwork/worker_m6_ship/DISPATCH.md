# DISPATCH — worker_m6_ship

## Objective
Finalize QuorumRAG production release sign-off:
1. Update `PROJECT.md` line 87: change Milestone 6 Status from `IN_PROGRESS` to `DONE`.
2. Inspect git status (`git status -s`) and git log (`git log -n 5 --oneline`).
3. If `scripts/solo_pr_ship.ps1` does not exist in `d:\harfile\QuorumRAG`, copy/adapt it from `d:\harfile\OshoosiClaw\scripts\solo_pr_ship.ps1` if applicable, or perform the standard automatic ship:
   - Stage all remaining files (`git add -A`).
   - Commit: `git commit -m "feat(quorumrag): complete production build, caching recache subsystem, 100% e2e and tier 5 adversarial hardening"`.
   - Push to remote: `git push origin main` (or execute `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "feat(quorumrag): complete production build, caching recache subsystem, 100% e2e and tier 5 adversarial hardening"` if available/applicable).
4. Run `git status -s` to verify clean working tree (no untracked or modified project files).
5. Inspect open PRs or GitHub Actions status if applicable (`gh pr list`, `gh run list`), ensuring clean merge/health.
6. Write `handoff.md` in your working directory `.agents/teamwork/worker_m6_ship/handoff.md` detailing the actions taken, commit hash, and git status.

## Mandatory Constraints
- DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
- Zero Mocks Protocol (`no_mocks_write_actual_code.md`): Zero dummy data, production code only.
- Universal Zero-Touch PATH Protocol (`environment_safety.md`): Never mutate system or user registry environment variables.
- Write your progress heartbeat to `.agents/teamwork/worker_m6_ship/progress.md` with a `Last visited: [timestamp]` header.

## Reference Paths
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\progress.md`

## 2026-11-01T10:53:26Z
**Context**: Milestone 6 Sign-Off and Git Shipping
**Content**: Checking in on status. How is the PROJECT.md update and git commit / push progressing?
**Action**: Provide a brief status update on current step and estimated completion time.
