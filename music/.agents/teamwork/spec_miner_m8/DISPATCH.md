## 2026-11-02T14:36:05Z
You are spec_miner_m8 in the orchestrator team for d:\harfile\music.
Working directory: d:\harfile\music\.agents\teamwork\spec_miner_m8\
Scope document: d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
You MUST read d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md immediately.

Focus on Deliverables 3 & 4:
1. RL Policy (PPO / Contextual Bandit + DPO) & Composite Reward:
   - Multi-file style blending across the uploaded reference WAVs (up to 20 files) so no single input file is copied.
   - Composite Reward Function:
     R_total = w_1 * R_style(y, x_ref_ensemble) + w_2 * R_harmony(y) + w_3 * R_rhythm(y) + w_4 * R_vocal_clarity(y) + w_5 * R_human + w_6 * R_originality_guard(y, {x_ref_1..x_ref_N})
   - Interactive DPO (R_human) preference learning wired to Co-Pilot Keep / Revert / Thumbs-Up feedback pairs (y_chosen, y_rejected).
2. Mandatory Built-in Legal Originality / Anti-Plagiarism Guard (R_originality_guard):
   - Computes mathematical similarity checks between generated song/melody and every individual uploaded reference WAV:
     1. Max N-gram pitch/chroma sequence overlap penalty (ensuring no copied melodic phrases/hooks).
     2. Waveform cross-correlation & frame-level cosine similarity ceiling (penalizing/rejecting any candidate whose frame-level similarity or chroma sequence correlation to any single input file exceeds a strict originality threshold, while rewarding matching the blended ensemble style distribution).
     3. Reports a verifiable Originality & Legal Compliance Score (e.g. 100% Original / Max Single-File Overlap < threshold) in the API response and WebUI card.
3. Strict User Rules to enforce:
   - RL evaluation rule (RULE[...rl_benchmarking_and_evaluation.md]): Empirical learning superiority proof over 4 baselines: Disabled, Random, Heuristic, NoLearning with 95% Confidence Intervals.
   - No mocks rule (RULE[...no_mocks_write_actual_code.md]): Genuine algorithms, real audio math, zero mock constants.
   - Clean git commit & solo PR shipping.

Write your findings and actionable technical specifications to:
d:\harfile\music\.agents\teamwork\spec_miner_m8\analysis.md
d:\harfile\music\.agents\teamwork\spec_miner_m8\handoff.md
Update your progress.md. When done, use send_message to report your completion and results.
