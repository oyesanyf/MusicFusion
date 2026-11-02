# BRIEFING — 2026-11-02T14:45:30Z

## Mission
Discover and document authoritative technical specifications for Deliverables 3 & 4: RL Policy (PPO/Bandit + DPO) with Composite Reward & Multi-File Style Blending, and the Mandatory Built-in Legal Originality / Anti-Plagiarism Guard (R_originality_guard) with 4-baseline empirical superiority proof.

## 🔒 My Identity
- Archetype: specification miner
- Roles: Specification Miner, Domain Expert (RL & Audio Signal Processing)
- Working directory: d:\harfile\music\.agents\teamwork\spec_miner_m8
- Original parent: f3dc28a2-6906-4296-bad9-049bc88a16ae
- Milestone: Milestone 8 (Deliverables 3 & 4)

## 🔒 Key Constraints
- Read-only on production code: Do NOT implement anything. Discover and document specifications.
- Prioritize authoritative spec sources over LLM prior knowledge.
- Multi-file style blending across uploaded reference WAVs (up to 20 files) so no single input file is copied.
- Composite Reward Function: R_total = w_1 * R_style(y, x_ref_ensemble) + w_2 * R_harmony(y) + w_3 * R_rhythm(y) + w_4 * R_vocal_clarity(y) + w_5 * R_human + w_6 * R_originality_guard(y, {x_ref_1..x_ref_N}).
- Interactive DPO preference learning wired to Co-Pilot Keep / Revert / Thumbs-Up feedback pairs (y_chosen, y_rejected).
- Built-in Legal Originality Guard: Max N-gram pitch/chroma sequence overlap penalty, waveform cross-correlation & frame-level cosine similarity ceiling, reports verifiable Originality & Legal Compliance Score.
- RL Evaluation Rule: Empirical learning superiority proof over 4 baselines (Disabled, Random, Heuristic, NoLearning) with 95% Confidence Intervals.
- No mocks rule: Genuine mathematical algorithms, real audio math, zero mock constants.

## Current Parent
- Conversation ID: f3dc28a2-6906-4296-bad9-049bc88a16ae
- Updated: 2026-11-02T14:45:30Z

## Task Summary
- **What to build**: Comprehensive technical specification and mathematical formalization for RL Style Blending, Composite Reward, DPO feedback, Legal Originality Guard, and 4-baseline benchmarking.
- **Success criteria**: Exhaustive interface definitions, mathematical equations, data structures, edge cases, error conditions, and verifiable test criteria documented in analysis.md and handoff.md.
- **Interface contracts**: d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md, PROJECT.md
- **Code layout**: d:\harfile\music\PROJECT.md

## Key Decisions Made
- Multi-file style blending: Formulated convex combination with a strict 60% anti-monopoly weight ceiling when N >= 3 to prevent single-track copying.
- Composite reward: 6 normalized components with weights [0.20, 0.15, 0.15, 0.15, 0.15, 0.20] and a steep negative penalty (down to -10.0) upon copyright breach.
- Legal Originality Guard: Triple-metric gate with 12-chromatic transposition sliding N-gram LCS (breach at >= 8 notes), normalized waveform cross-correlation (rho <= 0.65), and frame-level chroma cosine similarity (Sim <= 0.75).
- Interactive DPO: Direct online gradient update based on Bradley-Terry preference probability without auxiliary reward model, wired to WebUI Keep / Revert events.
- RL Benchmarking: Rigorously enforced RULE[...rl_benchmarking_and_evaluation.md] with explicit backend labeling ("SyntheticAudioEnvironment"), 50 matched seeds, 4 baselines (Disabled, Random, Heuristic, NoLearning) vs. LearnedPolicy, and Student's t 95% Confidence Intervals.

## Artifact Index
- d:\harfile\music\.agents\teamwork\spec_miner_m8\DISPATCH.md — Dispatch assignment and instructions
- d:\harfile\music\.agents\teamwork\spec_miner_m8\BRIEFING.md — Persistent context & situational awareness
- d:\harfile\music\.agents\teamwork\spec_miner_m8\progress.md — Liveness heartbeat and milestone tracker
- d:\harfile\music\.agents\teamwork\spec_miner_m8\analysis.md — Technical findings, mathematical equations, Rust data contracts, and mined specifications
- d:\harfile\music\.agents\teamwork\spec_miner_m8\handoff.md — 5-component self-contained hard handoff report

## Loaded Skills
- None explicitly loaded.
