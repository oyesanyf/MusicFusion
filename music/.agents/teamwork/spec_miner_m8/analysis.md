# Comprehensive Technical Specification & Analysis: Deliverables 3 & 4
**Module**: RL Policy (PPO / Contextual Bandit + DPO), Composite Reward Function, and Built-in Legal Originality / Anti-Plagiarism Guard  
**Subagent**: `spec_miner_m8`  
**Workspace**: `d:\harfile\music`  
**Date**: 2026-11-02  

---

## 1. Executive Summary & Architectural Scope

This document specifies the authoritative mathematical formulations, algorithms, data structures, and verification protocols for **Deliverables 3 & 4** of the Reference WAV Importer and RL Pipeline:

1. **Multi-File Style Blending (Up to 20 Reference WAVs)**:
   Extracts high-dimensional acoustic profiles across $N \in [1, 20]$ uploaded WAV files and synthesizes a non-monopolized convex ensemble style distribution $\mathbf{f}_{ensemble}$, ensuring no candidate song copies or mimics a single source file.
2. **Composite Reward Function ($R_{total}$)**:
   A 6-component multi-objective reward formulation:
   $$R_{total} = w_1 R_{style}(y, x_{ref\_ensemble}) + w_2 R_{harmony}(y) + w_3 R_{rhythm}(y) + w_4 R_{vocal\_clarity}(y) + w_5 R_{human} + w_6 R_{originality\_guard}(y, \{x_{ref\_1..N}\})$$
3. **Interactive DPO Preference Learning ($R_{human}$)**:
   Direct Preference Optimization (DPO) wired to user Keep / Revert / Thumbs-Up interaction feedback pairs $(y_{chosen}, y_{rejected})$ in the WebUI Co-Pilot, executing online parameter updates without training a separate reward model.
4. **Mandatory Built-in Legal Originality / Anti-Plagiarism Guard ($R_{originality\_guard}$)**:
   An unyielding anti-plagiarism guard computing:
   - Sliding N-gram pitch/chroma sequence overlap with 12 chromatic transpositions (penalizing contiguous melodic hook duplication $\ge 8$ notes).
   - Normalized waveform cross-correlation ($\rho_{max} \le 0.65$).
   - Frame-level chroma cosine similarity ceiling ($\text{Sim}_{chroma} \le 0.75$).
   - Emits a verifiable `LegalOriginalityReport` (e.g. 100% Original / Max Single-File Overlap < 15%) for API and WebUI inspection.
5. **Standardized RL Benchmark & Empirical Learning Superiority Proof**:
   Strict enforcement of `RULE[...rl_benchmarking_and_evaluation.md]`:
   - Explicit backend labeling: `SyntheticAudioEnvironment` vs. `LocalPureRustAudioDSPEnvironment`.
   - Proof of empirical superiority over 4 baselines: `Disabled`, `Random`, `Heuristic`, `NoLearning`.
   - Matched episode seeds ($N=50$) reporting 95% Confidence Intervals ($\bar{x} \pm t_{\alpha/2, n-1} \cdot \frac{s}{\sqrt{n}}$) across cumulative reward, originality score, and breach frequency.
6. **Zero-Mocks Enforcement**:
   Strict adherence to `RULE[...no_mocks_write_actual_code.md]`: genuine Radix-2 FFT, continuous autocorrelation, exact dynamic time warping / sliding window cross-correlation, and real audio math.

---

## 2. Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Blending | Multi-File Ensemble Feature Blending | Computes convex combination and variance across up to 20 reference WAV feature vectors | $\mathbf{f}_1 \dots \mathbf{f}_N$, weights $\alpha_1 \dots \alpha_N$ | Blended `BlendedStyleProfile` ($\boldsymbol{\mu}, \boldsymbol{\sigma}^2, \mathbf{c}_{ens}, \mathbf{e}_{ens}$) | Clamps $N \in [1, 20]$; rejects empty input with `EmptyReferencePool` | `ORIGINAL_REQUEST.md` §3 |
| 2 | Blending | Anti-Monopoly Weight Distribution Ceiling | Prevents any single file from exceeding 60% influence when $N \ge 3$ | Raw weights $\alpha_i$ | Normalized bounded weights $\sum \alpha_i = 1.0, \alpha_i \le 0.60$ | Auto-normalizes with logged warning | `ORIGINAL_REQUEST.md` §3 ("no single input file is copied") |
| 3 | Reward | Style Alignment Reward $R_{style}$ | Cosine similarity between candidate embedding/chroma/BPM and ensemble distribution | Candidate $y$, $\mathbf{f}_{ensemble}$ | Scalar $R_{style} \in [0.0, 1.0]$ | Clamped to $[0.0, 1.0]$; NaN/inf defaults to $0.0$ | `ORIGINAL_REQUEST.md` §3 |
| 4 | Reward | Harmonic Consonance Reward $R_{harmony}$ | Measures pitch chroma alignment to designated musical key scale and penalizes harsh dissonance | Candidate chroma $\mathbf{c}_y$, target scale profile $\mathbf{p}_{scale}$ | Scalar $R_{harmony} \in [0.0, 1.0]$ | Defaults to $0.5$ on silent buffer | `PROJECT.md` §Interface Contracts |
| 5 | Reward | Rhythmic Stability Reward $R_{rhythm}$ | Measures tempo drift score via onset autocorrelation flux vs. expected BPM grid | Candidate buffer $y$, target BPM | Scalar $R_{rhythm} \in [0.0, 1.0]$ | Severe drift (>0.30) heavily penalized; clipping subtracts 0.5 | `crates/music_dsp/src/telemetry_calc.rs` |
| 6 | Reward | Vocal Clarity Reward $R_{vocal\_clarity}$ | Formant band ratio (1–4 kHz) combined with SNR and crest factor | Candidate vocal/mix buffer $y$ | Scalar $R_{vocal\_clarity} \in [0.0, 1.0]$ | Digital silence returns 0.0 | `ORIGINAL_REQUEST.md` §3 |
| 7 | Reward | Interactive DPO Preference Reward $R_{human}$ | Direct preference score from Keep (+1.0), Revert (-1.0), or Bradley-Terry model | User feedback pair $(y_w, y_l)$ | Scalar $R_{human} \in [0.0, 1.0]$ | Neutral state defaults to 0.50 | `ORIGINAL_REQUEST.md` §3 |
| 8 | Originality | Transposition-Invariant Melodic N-Gram Overlap | Sliding window comparison of dominant pitch sequences across all 12 chromatic semitone shifts | Pitch tokens $S_y$, $\{S_{ref\_i}\}$ | Max LCS length $LCS_{max}$, Overlap ratio | Sequence length $\ge 8$ triggers breach flag | `ORIGINAL_REQUEST.md` §3.2 |
| 9 | Originality | Waveform Normalized Cross-Correlation Ceiling | Peak sliding dot product between candidate and reference waveforms | Normalized waveforms $y(t), x_{ref\_i}(t)$ | Max correlation $\rho_{max} \in [0.0, 1.0]$ | If $\rho_{max} > 0.65$, triggers plagiarism rejection | `ORIGINAL_REQUEST.md` §3.2 |
| 10 | Originality | Frame-Level Chroma Cosine Similarity Ceiling | Time-warped or sliding cosine similarity between 12-bin chroma sequences | Frame chromagrams $\mathbf{C}_y, \mathbf{C}_{ref\_i}$ | Max chroma similarity $\text{Sim}_{chroma} \in [0.0, 1.0]$ | If $\text{Sim}_{chroma} > 0.75$, triggers plagiarism rejection | `ORIGINAL_REQUEST.md` §3.2 |
| 11 | Originality | Legal Originality & Compliance Score Report | Verifiable compliance artifact summarizing originality percentage and per-file audit | All similarity metrics | `LegalOriginalityReport` struct & JSON | Status: `COMPLIANT_100_PCT_ORIGINAL`, `COMPLIANT_STYLE_BLENDED`, or `REJECTED_POTENTIAL_COPY` | `ORIGINAL_REQUEST.md` §3.2 |
| 12 | RL Policy | Contextual Bandit Policy Optimizer | Softmax/LinUCB policy sampling synthesis tweaks conditioned on ensemble context | Context state vector $\mathbf{s}$, action space $\mathcal{A}$ | Selected parameter tweak action $\mathbf{a}$ | Parameter values clamped to safe DSP bounds | `ORIGINAL_REQUEST.md` §3 |
| 13 | RL Policy | Proximal Policy Optimization (PPO) Engine | Actor-Critic policy gradient with clipped surrogate objective $L^{CLIP}(\theta)$ | State transitions $(s, a, r, s')$, advantages $\hat{A}$ | Updated policy weights $\theta$ | Gradient clipping at norm 1.0; ratio clipped to $[1-\epsilon, 1+\epsilon]$ | `ORIGINAL_REQUEST.md` §3 |
| 14 | DPO | Online Direct Preference Optimization | Updates policy weights using gradient of Bradley-Terry preference loss without reward model | Preference dataset $\mathcal{D} = \{(x, y_w, y_l)\}$ | Policy weight update $\Delta \theta$ | Clamped learning rate $\eta \in [10^{-4}, 10^{-2}]$ | `ORIGINAL_REQUEST.md` §3 |
| 15 | Benchmarking | 4-Baseline Empirical Learning Superiority Proof | Runs 50 matched-seed trials comparing Learned Policy against Disabled, Random, Heuristic, NoLearning | 50 matched seed scenarios | Cumulative reward, originality %, breach frequency with 95% CIs | Fails test if Learned does not achieve statistically superior cumulative reward | `RULE[...rl_benchmarking_and_evaluation.md]` |
| 16 | Benchmarking | Explicit Backend Labeling | Explicit metadata tag distinguishing local pure-Rust synthetic environment from external frameworks | Benchmark configuration | Environment metadata `"SyntheticAudioEnvironment"` | Prohibits reporting synthetic metrics as external package results | `RULE[...rl_benchmarking_and_evaluation.md]` §1 |
| 17 | API/UI | Co-Pilot Keep / Revert Preference Wire | REST endpoint capturing user Keep/Revert actions to generate DPO training tuples | `POST /api/feedback/preference` | Stored preference pair `(y_chosen, y_rejected)` | Rejects invalid version numbers with 400 Bad Request | `crates/music_cli/src/rest_server.rs` |
| 18 | API/UI | Originality Card WebUI Presentation | Front-end card rendering compliance badge, originality percentage, and max single-file overlap | `LegalOriginalityReport` | Interactive WebUI badge & breakdown | Green badge for $\ge 90\%$, amber for $70-89\%$, red for $<70\%$ | `crates/music_cli/webui/app.js` |

---

## 3. Mathematical Formalization

### 3.1 Multi-File Style Blending (Up to 20 Reference WAVs)

Let $\mathcal{X}_{ref} = \{x_{ref\_1}, x_{ref\_2}, \dots, x_{ref\_N}\}$ be the set of uploaded reference audio tracks, where $1 \le N \le 20$.  
For each track $i \in \{1, \dots, N\}$, the Stage 1 acoustic analyzer extracts an acoustic feature tuple:
$$\mathbf{f}_i = \left( BPM_i, \, \mathbf{c}_i, \, C_i, \, DR_i, \, V_i, \, \mathbf{e}_i \right)$$
where:
- $BPM_i \in [40.0, 240.0]$: Estimated tempo.
- $\mathbf{c}_i \in \mathbb{R}^{12}$: 12-dimensional normalized pitch chroma vector ($\|\mathbf{c}_i\|_2 = 1$).
- $C_i \in [20.0, 20000.0]$: Spectral centroid in Hz ($C_i = \frac{\sum_k f_k |X_i(k)|}{\sum_k |X_i(k)|}$).
- $DR_i \in [0.0, 40.0]$: Dynamic range / crest factor in dB.
- $V_i \in [0.0, 1.0]$: Vocal presence ratio ($V_i = \frac{\int_{1000}^{4000} |X_i(f)|^2 df}{\int_{20}^{20000} |X_i(f)|^2 df}$).
- $\mathbf{e}_i \in \mathbb{R}^D$: Joint acoustic style embedding vector ($D = 64$).

#### Convex Combination & Anti-Monopoly Weights
The user may supply preference weights $\mathbf{w} = [\alpha_1, \dots, \alpha_N]$, or uniform weights $\alpha_i = \frac{1}{N}$ are assigned by default.  
To prevent single-file monopolization when blending multiple tracks ($N \ge 3$), weights are strictly capped at $\alpha_{cap} = 0.60$:
$$\alpha_i \leftarrow \frac{\min(\alpha_i, 0.60)}{\sum_{j=1}^N \min(\alpha_j, 0.60)}$$

The blended ensemble profile $x_{ref\_ensemble}$ is defined by:
1. **Ensemble Tempo**:
   $$BPM_{ensemble} = \sum_{i=1}^N \alpha_i BPM_i$$
2. **Ensemble Chroma**:
   $$\mathbf{c}_{ensemble} = \frac{\sum_{i=1}^N \alpha_i \mathbf{c}_i}{\left\| \sum_{i=1}^N \alpha_i \mathbf{c}_i \right\|_2}$$
3. **Ensemble Spectral Centroid**:
   $$C_{ensemble} = \sum_{i=1}^N \alpha_i C_i$$
4. **Ensemble Vocal Presence**:
   $$V_{ensemble} = \sum_{i=1}^N \alpha_i V_i$$
5. **Ensemble Style Embedding**:
   $$\mathbf{e}_{ensemble} = \frac{\sum_{i=1}^N \alpha_i \mathbf{e}_i}{\left\| \sum_{i=1}^N \alpha_i \mathbf{e}_i \right\|_2}$$
6. **Ensemble Style Variance / Dispersion**:
   $$\sigma_{style}^2 = \sum_{i=1}^N \alpha_i \left( 1.0 - \frac{\mathbf{e}_i \cdot \mathbf{e}_{ensemble}}{\|\mathbf{e}_i\|_2 \|\mathbf{e}_{ensemble}\|_2} \right)$$
   A non-zero $\sigma_{style}^2$ establishes the breadth of the genre subspace spanned by the references.

---

### 3.2 Composite Reward Function ($R_{total}$)

The total reward evaluates any candidate song or generation step $y$:
$$R_{total}(y) = w_1 R_{style}(y, x_{ref\_ensemble}) + w_2 R_{harmony}(y) + w_3 R_{rhythm}(y) + w_4 R_{vocal\_clarity}(y) + w_5 R_{human} + w_6 R_{originality\_guard}(y, \{x_{ref\_1..N}\})$$

**Canonical Weights**:
$$w_1 = 0.20, \quad w_2 = 0.15, \quad w_3 = 0.15, \quad w_4 = 0.15, \quad w_5 = 0.15, \quad w_6 = 0.20$$
Normalized: $\sum_{j=1}^6 w_j = 1.00$.

#### 1. Style Alignment Reward: $R_{style}(y, x_{ref\_ensemble}) \in [0.0, 1.0]$
$$R_{style} = 0.50 \cdot \max\left(0.0, \, \frac{\mathbf{e}_y \cdot \mathbf{e}_{ensemble}}{\|\mathbf{e}_y\|_2 \|\mathbf{e}_{ensemble}\|_2}\right) + 0.30 \cdot \max\left(0.0, \, \mathbf{c}_y \cdot \mathbf{c}_{ensemble}\right) + 0.20 \cdot \exp\left(-\frac{|BPM_y - BPM_{ensemble}|}{15.0}\right)$$

#### 2. Harmonic Consonance Reward: $R_{harmony}(y) \in [0.0, 1.0]$
Let $\mathbf{p}_{scale} \in \mathbb{R}^{12}$ be the normalized Krumhansl-Schmuckler pitch profile for the target key and scale mode (e.g. A Major: tonic at bin 9, dominant at bin 4, etc.).
$$R_{harmony} = 0.70 \cdot \max\left(0.0, \, \mathbf{c}_y \cdot \mathbf{p}_{scale}\right) + 0.30 \cdot \left(1.0 - D_{dissonance}(y)\right)$$
where $D_{dissonance}(y) \in [0.0, 1.0]$ represents the proportion of chroma energy occurring on non-harmonic minor seconds and tritones relative to the root note.

#### 3. Rhythmic Stability Reward: $R_{rhythm}(y) \in [0.0, 1.0]$
Leveraging the autocorrelation onset drift algorithm from `crates/music_dsp/src/telemetry_calc.rs`:
$$R_{rhythm} = \exp\left(-3.5 \cdot \text{tempo\_drift\_score}(y)\right) \cdot (1.0 - P_{clipping})$$
where:
- $\text{tempo\_drift\_score}(y) \in [0.002, 1.0]$: 0.002 represents perfect sample-accurate beat alignment; >0.30 represents severe drift.
- $P_{clipping} = 0.50$ if any sample clips ($\ge 1.0$ or $\le -1.0$), else $0.0$.

#### 4. Vocal Clarity Reward: $R_{vocal\_clarity}(y) \in [0.0, 1.0]$
Evaluates vocal presence, intelligibility, and signal quality:
$$R_{vocal\_clarity} = 0.40 \cdot \min\left(1.0, \, \frac{V_y}{0.35}\right) + 0.30 \cdot \frac{\text{clamp}(\text{SNR}_{db}, 20.0, 90.0) - 20.0}{70.0} + 0.30 \cdot \frac{\text{clamp}(DR_{db}, 6.0, 18.0) - 6.0}{12.0}$$

#### 5. Human Preference Reward: $R_{human} \in [0.0, 1.0]$
Represents human reinforcement learning:
- $R_{human} = 1.00$ if the candidate was marked "Keep" or given "Thumbs-Up" in the WebUI.
- $R_{human} = 0.00$ if the candidate was marked "Revert" or given "Thumbs-Down".
- $R_{human} = 0.50$ for neutral / unrated candidates, or computed via implicit model score $\sigma\left(\beta \log \frac{\pi_\theta(y \mid x)}{\pi_{ref}(y \mid x)}\right)$.

#### 6. Built-in Legal Originality Guard: $R_{originality\_guard} \in [-10.0, 1.0]$
See Section 3.3 for full specification.

---

### 3.3 Built-in Legal Originality & Anti-Plagiarism Guard

The Legal Originality Guard is a fail-closed gate that evaluates candidate $y$ against EVERY individual reference track $x_{ref\_i}$ ($i = 1, \dots, N$).

#### Similarity Metric 1: Transposition-Invariant Melodic Pitch N-Gram Overlap
1. Both candidate $y$ and reference $x_{ref\_i}$ are converted to beat-synchronous dominant pitch class sequences:
   $$S_y = [p_1, p_2, \dots, p_T], \quad p_t = \arg\max_{k \in \{0..11\}} c_k(t) \in \{0, \dots, 11\}$$
   $$S_{ref\_i} = [q_1, q_2, \dots, q_{M_i}], \quad q_m = \arg\max_{k \in \{0..11\}} c_{i,k}(m) \in \{0, \dots, 11\}$$
2. In music copyright litigation, transposition does not absolve copying. Therefore, we compute the Longest Common Contiguous Substring across all 12 chromatic transpositions:
   $$LCS_{transposed}(S_y, S_{ref\_i}) = \max_{\Delta \in \{0..11\}} \max \left\{ k : \exists u, v \text{ s.t. } S_y[u..u+k] = (S_{ref\_i}[v..v+k] + \Delta) \bmod 12 \right\}$$
3. Pitch overlap metric:
   $$Overlap_{pitch}(y, x_{ref\_i}) = \frac{LCS_{transposed}(S_y, S_{ref\_i})}{N_{hook\_limit}}$$
   where $N_{hook\_limit} = 8$ notes (representing a standard 2-bar melodic hook).  
   - If $LCS \ge 6$: Amber warning flag.  
   - If $LCS \ge 8$: Hard infringement breach ($Overlap_{pitch} \ge 1.0$).

#### Similarity Metric 2: Waveform Normalized Cross-Correlation Ceiling
To detect sample theft or unauthorized audio stem reuse:
$$\rho_{max}(y, x_{ref\_i}) = \max_{\tau} \frac{\left| \sum_t y(t) \cdot x_{ref\_i}(t + \tau) \right|}{\sqrt{\sum_t y(t)^2 \cdot \sum_t x_{ref\_i}(t + \tau)^2}}$$
Ceiling: $\Theta_{xcorr} = 0.65$.  
If $\rho_{max} > 0.65$, direct waveform duplication is detected.

#### Similarity Metric 3: Frame-Level Chroma Cosine Similarity Ceiling
To detect chord progression and harmonic arrangement cloning:
$$\text{Sim}_{chroma}(y, x_{ref\_i}) = \max_{\tau} \frac{1}{W} \sum_{w=0}^{W-1} \frac{\mathbf{c}_y(t_0 + w) \cdot \mathbf{c}_{ref\_i}(\tau + w)}{\|\mathbf{c}_y(t_0 + w)\|_2 \|\mathbf{c}_{ref\_i}(\tau + w)\|_2}$$
over sliding 4-bar analysis windows ($W = 16$ beats).  
Ceiling: $\Theta_{chroma} = 0.75$.  
If $\text{Sim}_{chroma} > 0.75$, harmonic sequence copying is detected.

#### Overall Similarity and Penalty Function
For each reference file $i \in \{1, \dots, N\}$, define:
$$O_i(y) = \max\left( \frac{LCS_{transposed}(S_y, S_{ref\_i})}{8.0}, \, \frac{\rho_{max}(y, x_{ref\_i})}{0.65}, \, \frac{\text{Sim}_{chroma}(y, x_{ref\_i})}{0.75} \right)$$
Let $O_{max}(y) = \max_{i=1\dots N} O_i(y)$.

The Legal Originality Reward is defined piecewise:
$$R_{originality\_guard}(y) = \begin{cases}
1.0 - 0.5 \cdot O_{max}(y) & \text{if } O_{max}(y) \le 0.70 \quad (\text{Compliant}) \\
-10.0 \cdot \left(\frac{O_{max}(y) - 0.70}{0.30}\right) & \text{if } O_{max}(y) > 0.70 \quad (\text{Plagiarism Breach})
\end{cases}$$

When $O_{max}(y) > 0.70$, $R_{originality\_guard}$ plunges into large negative values (down to $-10.0$), overwhelmingly forcing the RL agent to reject candidate $y$.

#### Verifiable Originality & Legal Compliance Score
The system outputs a public compliance report:
$$\text{OriginalityScorePct} = \max\left(0.0, \, (1.0 - O_{max}(y)) \times 100\%\right)$$
$$\text{MaxSingleFileOverlapPct} = O_{max}(y) \times 100\%$$
Status Determination:
- `COMPLIANT_100_PCT_ORIGINAL`: $O_{max}(y) \le 0.30$ (Originality $\ge 70\%$, 0 copied phrases, clean cross-correlation).
- `COMPLIANT_STYLE_BLENDED`: $0.30 < O_{max}(y) \le 0.70$ (Style matches ensemble distribution without copying phrases).
- `REJECTED_POTENTIAL_COPY`: $O_{max}(y) > 0.70$ (Immediate generation abort / candidate discard).

---

### 3.4 Interactive DPO ($R_{human}$) Preference Learning

Direct Preference Optimization (DPO) optimizes the RL generation policy directly on preference pairs without requiring an auxiliary reward model.

#### Preference Tuple Extraction:
From user actions in `crates/music_cli/webui/app.js`:
1. **Keep / Thumbs-Up on Iterated Version $v$**:
   $$\text{Tuple}: \left( \mathbf{s}, \, y_{chosen} = y_v, \, y_{rejected} = y_{v-1} \right)$$
2. **Revert from Version $v$ to $v-1$**:
   $$\text{Tuple}: \left( \mathbf{s}, \, y_{chosen} = y_{v-1}, \, y_{rejected} = y_v \right)$$

#### Online Parameter Update:
In our Contextual Bandit parameterization, action $\mathbf{a} \in \mathbb{R}^K$ has policy density:
$$\pi_\theta(\mathbf{a} \mid \mathbf{s}) \propto \exp\left( \theta^\top \boldsymbol{\phi}(\mathbf{s}, \mathbf{a}) \right)$$
The Bradley-Terry preference probability is:
$$P(y_w \succ y_l \mid \mathbf{s}) = \sigma\left( \beta \left( \log \frac{\pi_\theta(y_w \mid \mathbf{s})}{\pi_{ref}(y_w \mid \mathbf{s})} - \log \frac{\pi_\theta(y_l \mid \mathbf{s})}{\pi_{ref}(y_l \mid \mathbf{s})} \right) \right)$$
Let $\Delta \boldsymbol{\phi} = \boldsymbol{\phi}(\mathbf{s}, y_w) - \boldsymbol{\phi}(\mathbf{s}, y_l)$. Then the analytical gradient is:
$$\nabla_\theta \mathcal{L}_{DPO}(\theta) = -\beta \left( 1.0 - \sigma\left( \beta (\theta - \theta_{ref})^\top \Delta \boldsymbol{\phi} \right) \right) \Delta \boldsymbol{\phi}$$
The online stochastic update with learning rate $\eta = 0.01$ and temperature $\beta = 0.1$:
$$\theta \leftarrow \theta + \eta \beta \left( 1.0 - \hat{p} \right) \Delta \boldsymbol{\phi}$$
This allows instantaneous online learning in pure Rust (< 1 ms per Keep/Revert action), with zero GPU overhead.

---

### 3.5 RL Benchmark: 4-Baseline Empirical Learning Superiority Proof

Per `RULE[...rl_benchmarking_and_evaluation.md]`:

#### 1. Explicit Backend Labeling
The test harness must explicitly declare:
```rust
pub const RL_BENCHMARK_BACKEND: &str = "SyntheticAudioEnvironment";
```
Metric logs and reports must state: `"Backend: SyntheticAudioEnvironment (Local Pure-Rust DSP)"`.

#### 2. The 4 Baselines vs. Learned Policy
Evaluated across $n = 50$ identical matched seeds ($seed = 1000 \dots 1049$):
1. **`Disabled`**: Policy is inactive; emits fixed neutral defaults ($\Delta BPM = 0$, baseline rasp, no adaptation).
2. **`Random`**: Action sampled uniformly at random: $\mathbf{a} \sim \mathcal{U}(\mathcal{A})$.
3. **`Heuristic`**: Deterministic fixed-rule table mapping genre/BPM to static adjustments without learning.
4. **`NoLearning`**: Initialized policy $\pi_{\theta_0}$ evaluated with frozen weights ($\eta = 0$).
5. **`LearnedPolicy`**: Online PPO / Contextual Bandit + DPO policy updated across training episodes.

#### 3. Statistical Inferential Metrics (95% Confidence Intervals)
For each baseline policy $p \in \{\text{Disabled}, \text{Random}, \text{Heuristic}, \text{NoLearning}, \text{Learned}\}$:
- Sample Mean: $\bar{R}_p = \frac{1}{n} \sum_{k=1}^n R_{total}^{(k)}$
- Sample Standard Deviation: $s_p = \sqrt{\frac{1}{n-1} \sum_{k=1}^n (R_{total}^{(k)} - \bar{R}_p)^2}$
- Standard Error: $SE_p = \frac{s_p}{\sqrt{n}}$
- 95% Confidence Interval:
  $$CI_{95}(p) = \left[ \bar{R}_p - t_{0.025, 49} \cdot SE_p, \quad \bar{R}_p + t_{0.025, 49} \cdot SE_p \right], \quad t_{0.025, 49} \approx 2.0096$$

#### Superiority Invariant:
$$\bar{R}_{\text{Learned}} > \bar{R}_{\text{Heuristic}} > \bar{R}_{\text{NoLearning}} > \bar{R}_{\text{Random}} > \bar{R}_{\text{Disabled}}$$
with breach frequency $\bar{B}_{\text{Learned}} = 0.0\%$.

---

## 4. Edge Cases Discovered & Handled

| # | Feature | Input Scenario | Observed / Specified Behavior |
|---|---------|----------------|-------------------------------|
| 1 | Blending | Single reference WAV ($N = 1$) | Weight $\alpha_1 = 1.00$. Ensemble profile matches the single track, but Originality Guard still enforces $O_{max} \le 0.70$ (no melodic hook cloning or waveform copying). |
| 2 | Blending | 20 Reference WAVs with conflicting tempos (e.g. 10 at 76 BPM, 10 at 160 BPM) | Weighted tempo blending yields average, but variance $\sigma_{bpm}^2$ is tracked; reward function encourages style alignment within the multi-modal convex hull. |
| 3 | Blending | Attempted upload of 21 reference WAVs | Strict capping at 20 files; HTTP 400 Bad Request or UI warning rejects file #21. |
| 4 | Blending | Digital silence reference WAV ($0.0$ samples) | Feature extractor detects energy $< -90$ dBFS; flags invalid reference and excludes from ensemble weights with zero division protection. |
| 5 | Originality | Exact melodic hook transposition (e.g., played 5 semitones higher in F Major instead of C Major) | $LCS_{transposed}$ scans all 12 chromatic shifts $\Delta \in \{0..11\}$; matches identically and flags $LCS \ge 8$ breach! |
| 6 | Originality | Exact waveform loop or sample reuse | Normalized cross-correlation $\rho_{max} > 0.65$ triggers instant plagiarism rejection. |
| 7 | Originality | Inverted phase / polarity-flipped waveform ($-x(t)$) | Absolute cross-correlation $|\rho_{max}| > 0.65$ detects flipped phase copying. |
| 8 | Originality | All reference WAVs identical duplicates of each other | Anti-monopoly and similarity check detects $O(y, x_i)$ uniformly across all duplicates; single-file overlap reported accurately. |
| 9 | DPO | Rapid repetitive user "Revert" actions | DPO receives negative preference updates for reverted states; policy gradient stably penalizes the action subspace without divergence. |
| 10 | RL Benchmark | Seed with adversarial extreme parameters | Softmax action clamping and brickwall limiter guarantee zero floating point overflow or clipping. |

---

## 5. Authoritative Rust Data Structures & Interface Contracts

The following types should be placed in `crates/music_core/src/rl_contracts.rs` (or equivalent core module):

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// High-dimensional acoustic feature profile extracted from a reference WAV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcousticFeatureVector {
    pub file_name: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub bpm: f32,
    pub chroma_12bin: [f32; 12],
    pub key_signature: String,
    pub spectral_centroid_hz: f32,
    pub dynamic_range_db: f32,
    pub vocal_presence_ratio: f32,
    pub style_embedding: Vec<f32>, // 64 dimensions
    pub pitch_token_sequence: Vec<u8>, // Beat-synchronous dominant pitch class (0..11)
}

/// Ensemble blended style distribution across up to 20 reference WAVs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlendedStyleProfile {
    pub reference_count: usize,
    pub file_weights: HashMap<String, f32>,
    pub blended_bpm: f32,
    pub blended_chroma: [f32; 12],
    pub blended_spectral_centroid_hz: f32,
    pub blended_vocal_presence: f32,
    pub blended_style_embedding: Vec<f32>,
    pub style_variance: f32,
}

/// Legal compliance status classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    Compliant100PctOriginal,
    CompliantStyleBlended,
    RejectedPotentialCopy,
}

/// Single-file similarity audit against one reference WAV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SingleFileOriginalityAudit {
    pub reference_file: String,
    pub pitch_sequence_overlap_notes: usize,
    pub pitch_sequence_overlap_ratio: f32,
    pub waveform_cross_correlation: f32,
    pub chroma_cosine_similarity: f32,
    pub composite_similarity_score: f32,
    pub is_breached: bool,
}

/// Verifiable Legal Originality & Anti-Plagiarism Report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalOriginalityReport {
    pub originality_score_pct: f32, // e.g. 98.4%
    pub max_single_file_overlap_pct: f32, // e.g. 14.2%
    pub most_similar_reference_file: String,
    pub max_melodic_sequence_notes: usize,
    pub max_waveform_cross_correlation: f32,
    pub max_chroma_cosine_similarity: f32,
    pub compliance_status: ComplianceStatus,
    pub per_file_breakdown: Vec<SingleFileOriginalityAudit>,
}

/// Breakdown of the 6 components of the Composite Reward Function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeRewardBreakdown {
    pub r_style: f32,
    pub r_harmony: f32,
    pub r_rhythm: f32,
    pub r_vocal_clarity: f32,
    pub r_human: f32,
    pub r_originality_guard: f32,
    pub r_total: f32,
    pub originality_report: LegalOriginalityReport,
}

/// Feedback pair for online DPO preference learning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpoPreferencePair {
    pub song_id: String,
    pub chosen_version: u32,
    pub rejected_version: u32,
    pub context_vector: Vec<f32>,
    pub chosen_action: Vec<f32>,
    pub rejected_action: Vec<f32>,
    pub feedback_type: String, // "keep", "revert", "thumbs_up", "thumbs_down"
}

/// 95% Confidence Interval container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceInterval95 {
    pub mean: f32,
    pub standard_deviation: f32,
    pub standard_error: f32,
    pub lower_bound: f32,
    pub upper_bound: f32,
}

/// Empirical baseline evaluation summary for RL benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaselineBenchmarkSummary {
    pub baseline_name: String, // "Disabled", "Random", "Heuristic", "NoLearning", "Learned"
    pub backend: String, // "SyntheticAudioEnvironment"
    pub episodes: usize,
    pub reward_ci95: ConfidenceInterval95,
    pub originality_ci95: ConfidenceInterval95,
    pub breach_frequency_ci95: ConfidenceInterval95,
}
```

---

## 6. Implementation Guidelines for Workers

1. **`crates/music_core`**: Add the data types above so all crates share authoritative schemas.
2. **`crates/music_dsp`**:
   - Implement `melodic_ngram_overlap(y_seq, ref_seq) -> usize` with 12 chromatic transpositions.
   - Implement `waveform_max_cross_correlation(y, ref) -> f32`.
   - Implement `chroma_sliding_cosine_similarity(y_chroma, ref_chroma) -> f32`.
   - Implement `check_originality_guard(y, &[x_ref]) -> LegalOriginalityReport`.
3. **`crates/music_fusion` / `crates/music_pipeline`**:
   - Implement `MultiFileStyleBlender` with the 60% anti-monopoly ceiling.
   - Implement `CompositeRewardCalculator` computing the 6-term formula.
   - Implement `DpoPreferenceLearner` with online gradient updates.
   - Implement `RlPolicyOptimizer` (Contextual Bandit / PPO).
4. **`crates/music_cli`**:
   - Connect `POST /api/reference-wavs/generate-rl` to run the RL pipeline and include the `LegalOriginalityReport` in the JSON response.
   - Connect `POST /api/feedback/preference` to feed `DpoPreferenceLearner`.
   - Update WebUI to display the Originality & Compliance badge.
5. **`tests/`**:
   - Write `rl_benchmarking_tests.rs` evaluating the 4 baselines over 50 matched seeds with 95% CIs.
   - Write `originality_guard_adversarial_tests.rs` asserting rejection of copied melodic hooks ($\ge 8$ notes) and duplicated waveforms ($\rho > 0.65$).
