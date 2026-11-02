# BRIEFING — 2026-11-02T05:29:00Z

## Mission
Investigate normalization in correct_query_spelling in crates/quorum-coordinator/src/offline_engine.rs and formulate a surgical fix strategy for cargo test -p quorum-coordinator --lib.

## 🔒 My Identity
- Archetype: explorer
- Roles: teamwork_explorer
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2
- Original parent: facfe77d-94d3-44be-95df-eec2b9dfe368
- Milestone: M8 Remediation 2

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Do NOT modify source code files
- Zero unverified claims, ground every observation with line numbers and verbatim code
- Zero-touch PATH and environment protection

## Current Parent
- Conversation ID: facfe77d-94d3-44be-95df-eec2b9dfe368
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `crates/quorum-coordinator/src/offline_engine.rs` (lines 810–1252): inspected `correct_spelling`, `correct_query_spelling`, `EXACT_TYPO_MAP`, `VOCABULARY`, tokenization loop, and unit test `test_spellcheck_corrects_chat_box_typos_without_rewriting`.
  - `crates/quorum-daemon/src/handlers/chat.rs` (lines 146–190): inspected `spellcheck_handler` endpoint.
  - Reproducing `cargo test -p quorum-coordinator --lib -- --test-threads=2`: confirmed 12/13 passed, 1 failed at line 1206 (formerly line 1166) with `assertion left == right failed: left: "where did femi Oyesanya go to school", right: "where did Femi Oyesanya go to school"`.
- **Key findings**:
  - `EXACT_TYPO_MAP` has `("femii", "Femi")` and `("femy", "Femi")`, but omits `("femi", "Femi")`.
  - The subsequent Levenshtein distance fallback requires `core_lower.len() >= 6`. Because `"femi"` is length 4 (< 6), it is bypassed entirely.
  - The Levenshtein loop also checks `let already_valid = VOCABULARY.iter().any(|v| v.eq_ignore_ascii_case(&core_lower)); if !already_valid { ... }`. Because `"Femi"` is in `VOCABULARY`, `already_valid` is `true`, meaning the loop intentionally does NOT perform case correction and sets no replacement.
  - Approach Comparison: Adding `("femi", "Femi")` and other lowercase proper nouns to `EXACT_TYPO_MAP` is strictly superior to modifying `VOCABULARY` loops. It is zero-allocation, has zero side effects on common nouns (e.g. avoiding spurious capitalization of `"Transcript"`), and directly fits the existing architecture where all proper noun normalization lives in `EXACT_TYPO_MAP`.
- **Unexplored areas**: None. Root cause, behavioral impact, and fix strategy fully characterized.

## Key Decisions Made
- Recommending Approach A: Surgical addition of `("femi", "Femi")` (and canonical domain names) to `EXACT_TYPO_MAP` in `crates/quorum-coordinator/src/offline_engine.rs`.

## Artifact Index
- DISPATCH.md — dispatch instructions and message logs
- BRIEFING.md — persistent working memory
- progress.md — liveness heartbeat
- handoff.md — self-contained handoff report with 5 components
