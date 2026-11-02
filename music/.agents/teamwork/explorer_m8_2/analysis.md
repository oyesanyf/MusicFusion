# Analysis: Deliverable 2 — Stage 1 Reference WAV Tokenization (RVQ) & Multi-File Feature Extraction

**Agent**: `explorer_m8_2`  
**Date**: 2026-11-02  
**Target Crates**: `crates/music_dsp`, `crates/music_core` (with downstream alignment for `crates/music_pipeline` and `crates/music_cli`)  
**Scope**: Deliverable 2 from `ORIGINAL_REQUEST.md` (Stage 1: Reference WAV Tokenization & Multi-File Feature Extraction across up to 20 reference WAVs)

---

## 1. Executive Summary

This investigation provides the architectural specifications, mathematical formulations, and pure-Rust implementation contracts for **Deliverable 2**:
1. **Acoustic Analyzer**: A high-precision digital signal processing engine capable of ingesting between 1 and 20 `.wav` files, extracting 5 distinct feature families per file, and synthesizing a weighted **Ensemble-Blended Audio Style Profile** and 64-dimensional **Joint Style Embedding Vector**.
2. **Residual Vector Quantization (RVQ) Discrete Codebook Tokenizer**: A pure-Rust multi-stage discrete tokenizer ($M \in \{4, 8\}$ stages, codebook size $K \in \{256, 1024\}$, feature dimension $D \in \{32, 64\}$) that tokenizes acoustic frames into discrete code indices, guarantees monotonic residual norm reduction, and directly supplies the token matrices required for the **Legal Originality / Anti-Plagiarism Guard** in Stage 2 & 3.

All algorithms are specified in 100% pure Rust without external C or Python dependencies, conforming to the workspace design principles in `PROJECT.md`.

---

## 2. Current State of Codebase & Identified Gaps

### 2.1 Existing DSP & Core Infrastructure (`crates/music_dsp`, `crates/music_core`)

1. **`AudioBuffer` (`crates/music_dsp/src/wav.rs`)**:
   - Manages interleaved 48 kHz stereo/mono audio (`sample_rate: u32`, `channels: u16`, `samples: Vec<f32>`).
   - Pure-Rust RIFF/WAVE encoder and decoder (`from_wav_bytes`, `to_wav_bytes_with_format`) supporting 16-bit PCM, 24-bit PCM, and 32-bit float.
   - Hound WAV integration (`to_wav_file_hound_pcm16`).
   - Utility operations: linear interpolation resampling (`resample_linear`), channel downmix/upmix (`to_mono`, `to_stereo`), slicing (`slice`), gain scaling (`scale_gain`, `scale_db`), normalization (`normalize`), broadcast clamping (`clamp_broadcast`), peak and RMS calculation.
2. **Spectral Telemetry Engine (`crates/music_dsp/src/telemetry_calc.rs`)**:
   - `fft_radix2(real: &mut [f32], imag: &mut [f32])`: in-place Radix-2 Cooley-Tukey decimation-in-time FFT. Requires $N = 2^k$.
   - `calculate_spectral_flatness`: 1024-point Hann-windowed FFT calculating Wiener entropy.
   - `calculate_tempo_drift`: 10ms-hop half-wave rectified onset flux autocorrelation against an expected BPM grid.
   - `calculate_snr`: frame energy sorted percentile analysis (signal 80th vs noise 10th percentile).
   - `calculate_phase_alignment`: normalized stereo channel dot product.
3. **Core Domain Model (`crates/music_core/src/state.rs`, `telemetry.rs`)**:
   - `MusicProductionState` currently tracks single optional files: `vocal_ref_wav: Option<PathBuf>`, `master_ref_wav: Option<PathBuf>`.
   - `StemTelemetry` captures audio quality metrics (`peak_dbfs`, `rms_dbfs`, `dynamic_range_db`, `tempo_drift_score`, `clipping`, `snr_db`, `spectral_flatness`, `headroom_db`, `phase_alignment_score`).

### 2.2 Critical Gaps to Bridge for Deliverable 2

| Feature Requirement | Current Codebase Status | Architectural Need |
|---|---|---|
| **Multi-WAV Importer (1..=20 files)** | Only single `vocal_ref_wav` / `master_ref_wav` in `MusicProductionState` | `ReferenceWavMetadata` and `Vec<ReferenceWavMetadata>` with strict 20-file cap enforcement |
| **BPM / Beat Grid & Onset Envelope** | Only relative `calculate_tempo_drift` against known BPM | Blind tempo estimation, autocorrelation peak-picking with octave prior, downbeat alignment, and frame-rate onset envelope |
| **12-Bin Chroma & Krumhansl Key** | No chroma or key detection algorithms | Filterbank/STFT chroma mapping across C2..C7, Krumhansl-Schmuckler 24-key Pearson correlation |
| **Spectral Timbre Envelope & Centroid** | Only spectral flatness | 8-band sub-band energy decomposition, spectral centroid $f_c$, crest factor (linear & dB), dynamic range |
| **Vocal Presence Profile** | Only fixed `vocal_headroom_db = 3.5` | Dynamic 1–4 kHz formant band ratio ($VPR$), temporal vocal activity detection curve, formant resonance tracking |
| **Joint Style Embedding & Blending** | None | 64-dimensional normalized dense embedding vector per file, weighted ensemble blending, dispersion metric |
| **Residual Vector Quantization (RVQ)** | None | Multi-stage codebook tokenizer ($M \in \{4, 8\}$, $K \in \{256, 1024\}$), deterministic basis codebook, nearest-neighbor vector search, token sequence representation |
| **Anti-Plagiarism / N-gram Foundation** | None | Rolling N-gram token hashing and frame-level cosine distance contracts |

---

## 3. Acoustic Analyzer Architecture

The Acoustic Analyzer consists of `AcousticAnalyzer`, `FileAcousticFeatures`, and `EnsembleAcousticFeatures`.

```
                    ┌──────────────────────────────┐
                    │ Up to 20 Reference WAV Files │
                    └──────────────┬───────────────┘
                                   │
              ┌────────────────────┴────────────────────┐
              ▼                                         ▼
   ┌───────────────────────┐                 ┌───────────────────────┐
   │   Reference WAV #1    │       ...       │   Reference WAV #N    │
   └──────────┬────────────┘                 └──────────┬────────────┘
              │                                         │
              ├─ Resample 48 kHz & Mono                 ├─ Resample 48 kHz & Mono
              ├─ STFT (1024-point, 10ms hop)            ├─ STFT (1024-point, 10ms hop)
              ├─ 1. BPM / Beat Grid & Onset             ├─ 1. BPM / Beat Grid & Onset
              ├─ 2. 12-Bin Chroma & Krumhansl Key       ├─ 2. 12-Bin Chroma & Krumhansl Key
              ├─ 3. 8-Band Timbre & Centroid            ├─ 3. 8-Band Timbre & Centroid
              ├─ 4. Vocal Presence (1-4 kHz Formant)    ├─ 4. Vocal Presence (1-4 kHz Formant)
              └─ 5. 64-Dim Style Embedding e_1          └─ 5. 64-Dim Style Embedding e_N
                                   │
                                   ▼
                    ┌──────────────────────────────┐
                    │ Multi-File Ensemble Blender  │
                    │  (Weights w_1..w_N, Sum=1.0) │
                    └──────────────┬───────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
     Blended Style Vector    Consensus Key       Blended Tempo
      e_ensemble in R^64      & Scale Profile      & Beat Grid
```

### 3.1 Feature 1: BPM / Beat Grid & Rhythmic Onset Envelope

#### Mathematical Formulation
1. **Short-Time Fourier Transform (STFT)**:
   - Sample rate: $f_s = 48{,}000\text{ Hz}$.
   - Window size: $N_{\text{fft}} = 1024$ samples ($\approx 21.33\text{ ms}$).
   - Hop size: $H = 480$ samples ($10.0\text{ ms}$, resulting in a $100\text{ Hz}$ frame rate).
   - Window function: Periodic Hann window $w[n] = 0.5 \left(1 - \cos\left(\frac{2\pi n}{N_{\text{fft}}}\right)\right)$.
   - Complex spectrum: $X[m, k] = \text{FFT}(x[m \cdot H + n] \cdot w[n])$ using `fft_radix2`.
   - Magnitude spectrum: $|X[m, k]| = \sqrt{\text{real}[m, k]^2 + \text{imag}[m, k]^2}$.

2. **Onset Detection Function (ODF)**:
   - **Spectral Flux (Half-Wave Rectified)**:
     $$\text{SF}[m] = \sum_{k=1}^{N_{\text{fft}}/2} \max(0, |X[m, k]| - |X[m-1, k]|)$$
   - **High-Frequency Content (HFC)**:
     $$\text{HFC}[m] = \sum_{k=1}^{N_{\text{fft}}/2} \frac{k}{N_{\text{fft}}/2} \cdot |X[m, k]|^2$$
   - **Combined Transient Envelope**:
     $$O[m] = 0.6 \cdot \frac{\text{SF}[m]}{\max(\text{SF}) + \epsilon} + 0.4 \cdot \frac{\text{HFC}[m]}{\max(\text{HFC}) + \epsilon}$$
   - Apply local adaptive threshold subtraction: $\widetilde{O}[m] = \max(0, O[m] - \text{median}_{[-10, +10]}(O))$.

3. **BPM Autocorrelation & Octave Prior**:
   - Lag range corresponding to $40\text{ BPM} \le \text{BPM} \le 240\text{ BPM}$:
     $$\tau_{\min} = \left\lfloor \frac{60 \times 100}{240} \right\rfloor = 25\text{ hops}, \quad \tau_{\max} = \left\lceil \frac{60 \times 100}{40} \right\rceil = 150\text{ hops}$$
   - Normalized autocorrelation:
     $$R[\tau] = \frac{\sum_{m=0}^{M - \tau - 1} \widetilde{O}[m] \cdot \widetilde{O}[m + \tau]}{\sqrt{\sum_{m} \widetilde{O}[m]^2 \sum_m \widetilde{O}[m+\tau]^2} + \epsilon}$$
   - Log-Gaussian tempo prior centered at $\mu = 120\text{ BPM}$ ($\tau_0 = 50\text{ hops}$, $\sigma = 0.5$ octaves):
     $$W[\tau] = \exp\left(-\frac{1}{2}\left(\frac{\log_2(\tau / 50)}{\sigma}\right)^2\right)$$
   - Optimal lag: $\tau^* = \arg\max_{\tau \in [\tau_{\min}, \tau_{\max}]} (R[\tau] \cdot W[\tau])$.
   - Parabolic sub-bin interpolation:
     $$\delta = \frac{R[\tau^* - 1] - R[\tau^* + 1]}{2 (R[\tau^* - 1] - 2 R[\tau^*] + R[\tau^* + 1])}, \quad \tau_{\text{refined}} = \tau^* + \delta$$
     $$\text{BPM} = \frac{60 \times 100}{\tau_{\text{refined}}}$$

4. **Beat Grid & Downbeat Tracking**:
   - Beat interval: $\Delta t_{\text{beat}} = \frac{60}{\text{BPM}}\text{ seconds} = \tau_{\text{refined}}\text{ hops}$.
   - Optimal phase offset:
     $$m_0 = \arg\max_{m \in [0, \lfloor \tau^* \rfloor)} \sum_{k=0}^{\lfloor (M - m) / \tau^* \rfloor} \widetilde{O}[m + k \cdot \lfloor \tau^* \rfloor]$$
   - Beat timestamps: $t_k = \frac{m_0 \cdot H}{f_s} + k \cdot \Delta t_{\text{beat}}$.

---

### 3.2 Feature 2: Krumhansl-Schmuckler 12-Bin Chroma Vector, Key & Scale Profile

#### Mathematical Formulation
1. **Chroma Filterbank Mapping**:
   - 12 Pitch Classes: $C=0, C\sharp=1, D=2, D\sharp=3, E=4, F=5, F\sharp=6, G=7, G\sharp=8, A=9, A\sharp=10, B=11$.
   - For FFT bin $k \in [1, N_{\text{fft}}/2]$, frequency $f_k = k \cdot \frac{f_s}{N_{\text{fft}}}$.
   - Musical frequency bounds: $65.4\text{ Hz}$ (C2) to $2093.0\text{ Hz}$ (C7).
   - Exact fractional MIDI pitch:
     $$p(f_k) = 69 + 12 \log_2\left(\frac{f_k}{440}\right)$$
   - Pitch class $c = \lfloor p(f_k) \rceil \pmod{12}$.
   - Bin weight with octave-tapering Gaussian window:
     $$w_{\text{oct}}(f_k) = \exp\left(-\frac{1}{2} \left(\frac{\log_2(f_k / 440)}{1.5}\right)^2\right)$$
   - Frame chroma accumulator:
     $$\mathbf{c}[m, c] = \sum_{k: \lfloor p(f_k) \rceil \equiv c} |X[m, k]|^2 \cdot w_{\text{oct}}(f_k)$$
   - Normalize frame chroma: $\mathbf{c}[m] \leftarrow \frac{\mathbf{c}[m]}{\sum_{c=0}^{11} \mathbf{c}[m, c] + \epsilon}$.
   - Global 12-bin chroma vector:
     $$\mathbf{v}_{\text{chroma}} = \frac{1}{M} \sum_{m=0}^{M-1} \mathbf{c}[m], \quad \sum_{i=0}^{11} v_{\text{chroma}}[i] = 1.0$$

2. **Krumhansl-Schmuckler Key Finding**:
   - Ground-truth key profiles:
     - **Major Profile**: $\mathbf{P}_{\text{maj}} = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88]$
     - **Minor Profile**: $\mathbf{P}_{\text{min}} = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17]$
   - Mean-centered profiles: $\widetilde{P}_i = P_i - \frac{1}{12} \sum_{j=0}^{11} P_j$.
   - Mean-centered chroma: $\widetilde{v}_i = v_i - \frac{1}{12} \sum_{j=0}^{11} v_j$.
   - For root $r \in \{0, \dots, 11\}$ and mode $m \in \{\text{Major}, \text{Minor}\}$:
     $$r_{\text{corr}}(r, m) = \frac{\sum_{i=0}^{11} \widetilde{v}_i \cdot \widetilde{P}_{(i - r) \pmod{12}}^{(m)}}{\sqrt{\sum_{i=0}^{11} \widetilde{v}_i^2} \cdot \sqrt{\sum_{i=0}^{11} (\widetilde{P}_i^{(m)})^2} + \epsilon}$$
   - Key selection: $(r^*, m^*) = \arg\max_{r, m} r_{\text{corr}}(r, m)$.
   - Key Confidence:
     $$\text{Confidence}_{\text{key}} = \max\left(0.0, \frac{r_{\text{best}} - r_{\text{second}}}{\max(0.05, 1.0 - r_{\text{second}})}\right)$$
   - Scale Profile: Determines modal affinity (Ionian, Aeolian, Dorian, Mixolydian, Major Pentatonic, Minor Pentatonic) via diatonic tone collection overlap.

---

### 3.3 Feature 3: Spectral Timbre Envelope, Centroid, Crest Factor, Dynamic Range

1. **8-Band Sub-Band Timbre Envelope**:
   - Sub-band definitions:
     - Band 0: Sub-bass ($20\text{ Hz} - 60\text{ Hz}$)
     - Band 1: Bass ($60\text{ Hz} - 250\text{ Hz}$)
     - Band 2: Low-mid ($250\text{ Hz} - 500\text{ Hz}$)
     - Band 3: Mid ($500\text{ Hz} - 1000\text{ Hz}$)
     - Band 4: Upper-mid ($1000\text{ Hz} - 2000\text{ Hz}$)
     - Band 5: High-mid ($2000\text{ Hz} - 4000\text{ Hz}$)
     - Band 6: Presence ($4000\text{ Hz} - 8000\text{ Hz}$)
     - Band 7: Brilliance ($8000\text{ Hz} - 20000\text{ Hz}$)
   - Energy in band $b$:
     $$E_b = \sum_{k: f_{\min, b} \le f_k < f_{\max, b}} |X[k]|^2$$
   - Normalized timbre vector: $\mathbf{t} = [E_0, \dots, E_7] / \left(\sum_{b=0}^7 E_b + \epsilon\right)$.

2. **Spectral Centroid ($f_c$)**:
   - Per-frame spectral center of gravity:
     $$f_c[m] = \frac{\sum_{k=1}^{N_{\text{fft}}/2} f_k \cdot |X[m, k]|}{\sum_{k=1}^{N_{\text{fft}}/2} |X[m, k]| + \epsilon}$$
   - Global centroid: mean $\bar{f}_c$ and standard deviation $\sigma_{f_c}$ across all non-silent frames.

3. **Crest Factor & Dynamic Range**:
   - Peak level: $x_{\text{peak}} = \max_n |x[n]|$.
   - RMS level: $x_{\text{rms}} = \sqrt{\frac{1}{N} \sum_{n=0}^{N-1} x[n]^2}$.
   - Linear Crest Factor: $CF = \frac{x_{\text{peak}}}{x_{\text{rms}} + \epsilon}$.
   - Decibel Crest Factor: $CF_{\text{dB}} = 20 \log_{10}(CF)$.
   - Dynamic Range: Difference between $95\text{th}$ percentile energy block and $10\text{th}$ percentile noise floor block in dBFS.

---

### 3.4 Feature 4: Vocal Presence Profile (1–4 kHz Formant Band Ratio)

1. **Vocal Formant Band Ratio ($VPR$)**:
   - Band: $1{,}000\text{ Hz} \le f \le 4{,}000\text{ Hz}$ (covers vocal formants F1, F2, F3, and the singer's formant cluster).
   - Frame-level ratio:
     $$VPR[m] = \frac{\sum_{k: 1000 \le f_k \le 4000} |X[m, k]|^2}{\sum_{k: 20 \le f_k \le 20000} |X[m, k]|^2 + \epsilon}$$
   - Mean Vocal Presence: $\overline{VPR} = \frac{1}{M} \sum_{m=0}^{M-1} VPR[m]$.

2. **Vocal Activity & Temporal Presence**:
   - A frame is deemed actively vocal if:
     $$VPR[m] \ge 0.28 \quad \text{AND} \quad 10 \log_{10}\left(\sum_k |X[m, k]|^2\right) \ge -42\text{ dBFS}$$
   - Vocal Activity Ratio:
     $$\text{VAR} = \frac{\text{Count}(m: \text{frame is vocal})}{M}$$
   - Formant Peak Location:
     $$f_{\text{vocal\_peak}} = \arg\max_{f_k \in [1000, 4000]} |X_{\text{avg}}[k]|$$
     Indicates vocal archetype (e.g. Jamaican tenor $\approx 1500-2600\text{ Hz}$ vs. Soprano $\approx 1850-2900\text{ Hz}$).

---

### 3.5 Feature 5: Joint Audio Style Embedding Vector ($\mathbb{R}^{64}$)

Concatenates the normalized multi-domain features into an invariant dense style vector:

| Index Range | Feature Description | Normalization / Encoding |
|---|---|---|
| `[0..11]` | Mean 12-bin Chroma Vector $\mathbf{v}_{\text{chroma}}$ | L1 normalized ($\sum = 1.0$) |
| `[12]` | Estimated Tempo (BPM) | $\frac{\text{BPM} - 40}{200} \in [0.0, 1.0]$ |
| `[13]` | Beat Regularity / Confidence | Autocorrelation peak height $\in [0.0, 1.0]$ |
| `[14]` | Key Mode | $+1.0$ (Major), $-1.0$ (Minor), $0.0$ (Ambiguous) |
| `[15]` | Key Root X-coordinate | $\cos(2\pi \cdot r^* / 12)$ |
| `[16]` | Key Root Y-coordinate | $\sin(2\pi \cdot r^* / 12)$ |
| `[17..24]` | 8-Band Timbre Envelope | L1 normalized ($\sum = 1.0$) |
| `[25]` | Spectral Centroid | $\frac{\bar{f}_c}{10000} \in [0.0, 1.0]$ |
| `[26]` | Spectral Flatness | Wiener entropy $\in [0.0, 1.0]$ |
| `[27]` | Crest Factor | $\frac{CF_{\text{dB}}}{30.0} \in [0.0, 1.0]$ |
| `[28]` | Dynamic Range | $\frac{DR_{\text{dB}}}{30.0} \in [0.0, 1.0]$ |
| `[29]` | Mean Vocal Presence Ratio ($VPR$) | Clamped to $[0.0, 1.0]$ |
| `[30]` | Vocal Activity Ratio ($VAR$) | Percentage $\in [0.0, 1.0]$ |
| `[31]` | RMS Loudness | $\frac{\text{RMS}_{\text{dBFS}} + 90}{90} \in [0.0, 1.0]$ |
| `[32..43]` | Chroma Temporal Variance | Frame-by-frame variance per chroma bin |
| `[44..51]` | Timbre Temporal Variance | Frame-by-frame variance per sub-band |
| `[52]` | Spectral Flux Dynamism | Variance of onset detection function |
| `[53]` | Vocal Presence Variance | Standard deviation of $VPR[m]$ |
| `[54..63]` | Higher Spectral Moments | Skewness, Kurtosis, 85% Roll-off, Zero-Crossing Rate |

**Vector Normalization**:
$$\mathbf{e} \leftarrow \frac{\mathbf{e}}{||\mathbf{e}||_2 + \epsilon}$$

---

### 3.6 Multi-File Ensemble Blending Engine (Up to 20 Files)

When ingesting $N$ reference WAV files ($1 \le N \le 20$):
1. **Cap Enforcement**: If $N > 20$, return `DspError::ParameterError("Exceeded maximum of 20 reference WAV files")`.
2. **User / Equal Weights**: Weights $w_i \ge 0$, normalized so $\sum_{i=1}^N w_i = 1.0$.
3. **Blended Style Embedding**:
   $$\mathbf{e}_{\text{blended}} = \frac{\sum_{i=1}^N w_i \mathbf{e}_i}{\left|\left|\sum_{i=1}^N w_i \mathbf{e}_i\right|\right|_2}$$
4. **Blended Tempo Harmonization**:
   - Tempo octave alignment: If reference $i$ is $75\text{ BPM}$ and reference $j$ is $150\text{ BPM}$, fold them to a common tempo octave $[60, 120]$ or $[80, 160]$ using metric modulation.
   - Weighted harmonic/arithmetic tempo:
     $$\text{BPM}_{\text{blended}} = \sum_{i=1}^N w_i \cdot \text{BPM}_{\text{aligned}, i}$$
5. **Consensus Key & Blended Chroma**:
   - Blended Chroma: $\bar{\mathbf{v}}_{\text{chroma}} = \sum_{i=1}^N w_i \mathbf{v}_{\text{chroma}, i}$.
   - Re-run Krumhansl-Schmuckler on $\bar{\mathbf{v}}_{\text{chroma}}$ to derive the harmonious blended consensus key.
6. **Blended Timbre & Vocal Presence**:
   $$\bar{\mathbf{t}}_{\text{timbre}} = \sum_{i=1}^N w_i \mathbf{t}_i, \quad \overline{VPR}_{\text{blended}} = \sum_{i=1}^N w_i \overline{VPR}_i$$
7. **Style Dispersion / Diversity Index**:
   $$\Delta_{\text{diversity}} = 1.0 - \frac{1}{N} \sum_{i=1}^N (\mathbf{e}_i \cdot \mathbf{e}_{\text{blended}})$$
   - $\Delta = 0.0$: All uploaded reference files share identical acoustic and harmonic styles.
   - $\Delta > 0.15$: Diverse/hybrid multi-genre blend.

---

## 4. Residual Vector Quantization (RVQ) Tokenizer Architecture

The RVQ Tokenizer converts continuous acoustic feature frames into discrete token matrices, matching the multi-stage codebook design used in modern neural audio codecs (e.g. EnCodec, SoundStream, DAC, and YuE acoustic tokenizers).

```
 Input Frame x in R^D ───►[ Stage 1 Codebook C_1 ]───► Token k_1 in {0..K-1}
                                  │
                          Residual r_1 = x - q_1
                                  ▼
                         [ Stage 2 Codebook C_2 ]───► Token k_2 in {0..K-1}
                                  │
                          Residual r_2 = r_1 - q_2
                                  ▼
                         [ Stage 3 Codebook C_3 ]───► Token k_3 in {0..K-1}
                                  │
                          Residual r_3 = r_2 - q_3
                                  ▼
                         [ Stage 4 Codebook C_4 ]───► Token k_4 in {0..K-1}
                                  │
                          Final Residual r_4
```

### 4.1 Mathematical Formulation of RVQ

Let $\mathbf{x} \in \mathbb{R}^D$ be the acoustic feature vector of an audio frame (e.g. $D = 32$ or $D = 64$).
Let $M$ be the number of quantizer stages (e.g. $M = 4$ or $M = 8$).
Let each stage have a codebook $\mathcal{C}_m = \{\mathbf{c}_{m, 0}, \mathbf{c}_{m, 1}, \dots, \mathbf{c}_{m, K-1}\}$ of size $K$ (e.g. $K = 256$, where indices fit in a `u8`/`u16`).

#### Quantization Loop
1. Initialize residual: $\mathbf{r}_0 = \mathbf{x}$.
2. For $m = 1, 2, \dots, M$:
   - Nearest Neighbor Search:
     $$k_m = \arg\min_{k \in \{0, \dots, K-1\}} ||\mathbf{r}_{m-1} - \mathbf{c}_{m, k}||^2$$
   - Quantized Codevector:
     $$\mathbf{q}_m = \mathbf{c}_{m, k_m}$$
   - Compute Next Residual:
     $$\mathbf{r}_m = \mathbf{r}_{m-1} - \mathbf{q}_m$$

#### Discrete Token Representation
An audio file spanning $T$ frames is mapped to a discrete token matrix:
$$\mathbf{T} \in \{0, \dots, K-1\}^{T \times M}$$
Each row $\mathbf{T}[t, :] = [k_{t, 1}, k_{t, 2}, \dots, k_{t, M}]$ compactly characterizes frame $t$.

#### Reconstruction & Residual Norm Property
The reconstructed acoustic vector is:
$$\hat{\mathbf{x}} = \sum_{m=1}^M \mathbf{q}_m = \sum_{m=1}^M \mathbf{c}_{m, k_m}$$
The total quantization error is the final residual: $\mathbf{e}_{\text{recon}} = \mathbf{x} - \hat{\mathbf{x}} = \mathbf{r}_M$.

**Theorem (Monotonic Error Bound)**:
Since the zero codevector $\mathbf{0}$ or minimum-distance codevector is chosen at each stage:
$$||\mathbf{r}_M|| \le ||\mathbf{r}_{M-1}|| \le \dots \le ||\mathbf{r}_1|| \le ||\mathbf{r}_0||$$
Each additional RVQ stage monotonically reduces (or preserves) the quantization error.

---

### 4.2 Acoustic Frame Feature Vector ($D = 32$)

For each 10ms frame, extract a 32-dimensional perceptual acoustic descriptor:
- **`[0..15]` (16 bins)**: Log Mel-scale filterbank energies ($20\text{ Hz} - 20{,}000\text{ Hz}$).
- **`[16..27]` (12 bins)**: Instantaneous 12-bin Pitch Class Chroma vector.
- **`[28]` (1 bin)**: Normalized Spectral Centroid.
- **`[29]` (1 bin)**: Spectral Flatness Measure.
- **`[30]` (1 bin)**: Normalized Frame RMS Energy.
- **`[31]` (1 bin)**: Spectral Flux / Transient Onset Magnitude.

---

### 4.3 Deterministic Codebook Initialization in Pure Rust

To guarantee zero external dependencies and reproducible execution without loading 5 GB external model weights:
- **Stage 1 ($\mathcal{C}_1$)**: Macro-energy and spectral tilt. Orthogonal DCT-II basis modulated across $K=256$ centroids.
- **Stage 2 ($\mathcal{C}_2$)**: Harmonic chroma and pitch-class residuals. 12-bin rotation lattice codevectors.
- **Stage 3 ($\mathcal{C}_3$)**: Formant presence (1–4 kHz) and vocal timbre residuals.
- **Stage 4 ($\mathcal{C}_4$)**: High-frequency micro-transients and noise texture.

#### High-Performance Nearest Neighbor Search
Expand the squared Euclidean distance:
$$||\mathbf{r} - \mathbf{c}_k||^2 = ||\mathbf{r}||^2 + ||\mathbf{c}_k||^2 - 2 (\mathbf{r} \cdot \mathbf{c}_k)$$
Precompute $E_k = ||\mathbf{c}_k||^2$ during codebook construction. The search reduces to finding:
$$k^* = \arg\max_{k \in \{0, \dots, K-1\}} \left(2 (\mathbf{r} \cdot \mathbf{c}_k) - E_k\right)$$
This enables auto-vectorized SIMD dot-products in pure Rust.

---

## 5. Bridge to Deliverable 3: Legal Originality & Anti-Plagiarism Guard

The features and tokens produced in Stage 1 directly empower the Stage 2 & 3 Composite Reward and Anti-Plagiarism Guard:

```
Reference WAVs (up to 20)          Generated Candidate Song y
       │                                     │
       ▼                                     ▼
Stage 1: RVQ Tokenizer &              Stage 1: RVQ Tokenizer &
Acoustic Feature Extraction          Acoustic Feature Extraction
       │                                     │
  Tokens T_ref,i                        Tokens T_y
  Features F_ref,i                      Features F_y
  Style Embeddings e_ref,i              Style Embedding e_y
  Ensemble Embedding e_ens                   │
       │                                     │
       └──────────────────┬──────────────────┘
                          │
                          ▼
        ┌───────────────────────────────────┐
        │ Stage 2 & 3 Anti-Plagiarism Guard │
        └─────────────────┬─────────────────┘
                          │
         ┌────────────────┴────────────────┐
         ▼                                 ▼
1. Max N-Gram Pitch/Chroma         2. Waveform & Frame-Level
   Token Overlap Check                Cosine Similarity Ceiling
   Overlap(T_y, T_ref,i) <= 15%        cos_sim(F_y, F_ref,i) <= 0.70
         │                                 │
         └────────────────┬────────────────┘
                          ▼
            R_originality_guard Penalty &
          Verifiable Originality Score (%)
```

### 5.1 N-Gram Token Sequence Overlap
- Let $N = 16$ frames ($\approx 160\text{ ms}$, corresponding to a 16th-note musical phrase).
- Extract rolling 64-bit hashes of Stage 1 & Stage 2 tokens:
  $$H_y[t] = \text{FNV64}\Big(\mathbf{T}_y[t .. t + N, 0..2]\Big)$$
- Compare set of hashes $\mathcal{H}_y$ against $\mathcal{H}_{\text{ref}, i}$ for each reference WAV:
  $$\text{OverlapRatio}(y, x_{\text{ref}, i}) = \frac{|\mathcal{H}_y \cap \mathcal{H}_{\text{ref}, i}|}{|\mathcal{H}_y|}$$
- If $\max_i \text{OverlapRatio}(y, x_{\text{ref}, i}) > 0.15$ (15% phrase overlap), the candidate triggers the plagiarism penalty.

### 5.2 Frame-Level Cosine Similarity Ceiling
- For candidate frame features $\mathbf{f}_y[t]$ and reference frame features $\mathbf{f}_{\text{ref}, i}[t']$:
  $$S(t, t') = \frac{\mathbf{f}_y[t] \cdot \mathbf{f}_{\text{ref}, i}[t']}{||\mathbf{f}_y[t]|| \cdot ||\mathbf{f}_{\text{ref}, i}[t']||}$$
- Peak cross-similarity to single reference $i$: $\rho_{\max, i} = \max_\tau \frac{1}{T} \sum_t S(t, t + \tau)$.
- Anti-plagiarism condition: $\forall i \in \{1, \dots, N\}, \quad \rho_{\max, i} \le 0.70$.
- Style alignment reward: $R_{\text{style}} = \mathbf{e}_y \cdot \mathbf{e}_{\text{ensemble}}$ (rewarding closeness to the blended ensemble while penalizing closeness to any individual input).

### 5.3 Verifiable Originality Score
$$\text{OriginalityScore} = \left(1.0 - \max_{i=1}^N \max\Big(\text{OverlapRatio}(y, x_{\text{ref}, i}), \; \rho_{\max, i}\Big)\right) \times 100\%$$

---

## 6. Concrete Rust Interface Contracts

### 6.1 Proposed Data Structures in `crates/music_dsp/src/acoustic_analyzer.rs`

```rust
use serde::{Deserialize, Serialize};
use crate::error::DspError;
use crate::wav::AudioBuffer;

/// Rhythmic beat grid and onset timing characteristics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BpmBeatGrid {
    pub bpm: f32,
    pub beat_interval_secs: f32,
    pub downbeat_offset_secs: f32,
    pub beat_times: Vec<f32>,
    pub tempo_confidence: f32,
    pub onset_envelope: Vec<f32>,
}

/// Krumhansl-Schmuckler 12-bin pitch class profile, musical key, and scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromaKeyProfile {
    pub chroma_12bin: [f32; 12],
    pub key_root: String,
    pub key_mode: String,
    pub key_signature: String,
    pub key_confidence: f32,
    pub scale_profile: String,
}

/// Multi-band spectral timbre and dynamic envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralTimbreProfile {
    pub sub_bass_ratio: f32,     // 20-60 Hz
    pub bass_ratio: f32,         // 60-250 Hz
    pub low_mid_ratio: f32,      // 250-500 Hz
    pub mid_ratio: f32,          // 500-1000 Hz
    pub upper_mid_ratio: f32,    // 1000-2000 Hz
    pub high_mid_ratio: f32,     // 2000-4000 Hz
    pub presence_ratio: f32,     // 4000-8000 Hz
    pub brilliance_ratio: f32,   // 8000-20000 Hz
    pub spectral_centroid_hz: f32,
    pub spectral_flatness: f32,
    pub crest_factor_linear: f32,
    pub crest_factor_db: f32,
    pub dynamic_range_db: f32,
}

/// Vocal presence profile within the 1-4 kHz formant resonance cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VocalPresenceProfile {
    pub vocal_formant_ratio: f32,
    pub vocal_activity_percentage: f32,
    pub formant_peak_hz: f32,
    pub vocal_detected: bool,
}

/// Complete acoustic features extracted from a single reference WAV file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileAcousticFeatures {
    pub filename: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
    pub beat_grid: BpmBeatGrid,
    pub chroma_key: ChromaKeyProfile,
    pub timbre: SpectralTimbreProfile,
    pub vocal_presence: VocalPresenceProfile,
    pub style_embedding_64: [f32; 64],
}

/// Ensemble-blended acoustic features synthesized across up to 20 reference WAV files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleAcousticFeatures {
    pub file_count: usize,
    pub per_file_features: Vec<FileAcousticFeatures>,
    pub blended_bpm: f32,
    pub consensus_key_signature: String,
    pub consensus_scale: String,
    pub blended_chroma_12bin: [f32; 12],
    pub blended_timbre: SpectralTimbreProfile,
    pub blended_vocal_presence: VocalPresenceProfile,
    pub joint_style_embedding_64: [f32; 64],
    pub style_dispersion_index: f32,
}

/// Acoustic Analyzer engine contract.
pub struct AcousticAnalyzer;

impl AcousticAnalyzer {
    pub fn analyze_file(filename: &str, buffer: &AudioBuffer) -> Result<FileAcousticFeatures, DspError>;
    pub fn analyze_ensemble(
        files: &[(&str, &AudioBuffer)],
        weights: Option<&[f32]>,
    ) -> Result<EnsembleAcousticFeatures, DspError>;
}
```

---

### 6.2 Proposed Data Structures in `crates/music_dsp/src/rvq.rs`

```rust
use serde::{Deserialize, Serialize};
use crate::error::DspError;
use crate::wav::AudioBuffer;

/// Discrete token sequence generated by the Residual Vector Quantization tokenizer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RvqTokenSequence {
    pub sample_rate: u32,
    pub hop_size: usize,
    pub frame_rate_hz: f32,
    pub num_stages: usize,    // M = 4 or 8
    pub codebook_size: usize, // K = 256 or 1024
    pub feature_dim: usize,   // D = 32
    pub frame_count: usize,   // T
    pub tokens: Vec<u16>,     // Flattened row-major [T x M]
}

impl RvqTokenSequence {
    pub fn frame_tokens(&self, frame_idx: usize) -> Option<&[u16]>;
    pub fn stage_tokens(&self, stage_idx: usize) -> Vec<u16>;
    pub fn rolling_ngram_hashes(&self, n: usize, stage_idx: usize) -> Vec<u64>;
    pub fn compute_ngram_overlap(&self, other: &Self, n: usize) -> f32;
}

/// Multi-stage Residual Vector Quantizer.
#[derive(Debug, Clone)]
pub struct RvqTokenizer {
    pub num_stages: usize,
    pub codebook_size: usize,
    pub feature_dim: usize,
    codebooks: Vec<Vec<f32>>,     // [M x K x D]
    codebook_norms: Vec<Vec<f32>>, // [M x K] precomputed norms for fast Euclidean search
}

impl RvqTokenizer {
    pub fn new_deterministic(num_stages: usize, codebook_size: usize, feature_dim: usize) -> Self;
    pub fn default_4stage_k256() -> Self;
    pub fn default_8stage_k1024() -> Self;
    pub fn tokenize(&self, buffer: &AudioBuffer) -> Result<RvqTokenSequence, DspError>;
    pub fn decode_features(&self, tokens: &RvqTokenSequence) -> Result<Vec<[f32; 32]>, DspError>;
    pub fn synthesize_audio(&self, tokens: &RvqTokenSequence) -> Result<AudioBuffer, DspError>;
}
```

---

### 6.3 Extensions to `crates/music_core/src/state.rs`

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Metadata and analysis summary for an imported reference WAV file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceWavMetadata {
    pub id: String,
    pub filename: String,
    pub path: PathBuf,
    pub duration_secs: f32,
    pub size_bytes: u64,
    pub sha256_hash: String,
    pub bpm: f32,
    pub key_signature: String,
    pub vocal_presence: f32,
}

// In MusicProductionState:
// pub reference_wavs: Vec<ReferenceWavMetadata>,
// pub ensemble_blended_style: Option<EnsembleAcousticFeaturesSummary>,
```

---

## 7. Recommended Implementation Path & Test Strategy

1. **Step 1: Module Creation in `crates/music_dsp`**:
   - Create `crates/music_dsp/src/acoustic_analyzer.rs`.
   - Create `crates/music_dsp/src/rvq.rs`.
   - Re-export `AcousticAnalyzer`, `FileAcousticFeatures`, `EnsembleAcousticFeatures`, `RvqTokenizer`, `RvqTokenSequence` in `crates/music_dsp/src/lib.rs`.
2. **Step 2: Core State Integration in `crates/music_core`**:
   - Add `ReferenceWavMetadata` and `EnsembleAcousticFeaturesSummary` in `crates/music_core/src/state.rs`.
   - Ensure backward compatibility: defaults allow `reference_wavs` to be an empty `Vec`.
3. **Step 3: Verification Test Suite**:
   - `test_acoustic_analyzer_pure_tone`: verify 440 Hz A4 sine wave produces exact A Major/Minor chroma bin peak.
   - `test_krumhansl_major_minor_detection`: verify synthetic C Major chord `[C, E, G]` and A Minor chord `[A, C, E]` correctly resolve to "C Major" and "A Minor" with high confidence.
   - `test_bpm_autocorrelation_beat_grid`: verify synthetic 120 BPM and 76 BPM drum beat clicks resolve to exact BPM $\pm 1$ BPM.
   - `test_vocal_presence_detection`: verify bandpass-filtered 2 kHz tone returns $> 0.85$ vocal presence, while 60 Hz sub-bass sine returns $< 0.05$.
   - `test_ensemble_blending_20_files`: verify ensemble blending of 20 distinct files succeeds, while 21 files strictly returns `DspError::ParameterError`.
   - `test_rvq_monotonic_residual_reduction`: verify $||r_4|| < ||r_3|| < ||r_2|| < ||r_1|| < ||r_0||$ for all tested frames.
   - `test_rvq_ngram_plagiarism_detection`: verify identical audio produces 100% N-gram overlap, while unrelated audio produces $< 5\%$ overlap.
