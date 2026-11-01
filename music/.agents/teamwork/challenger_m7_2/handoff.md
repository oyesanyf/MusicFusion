# Milestone 7 Challenger 2 Adversarial Verification Report

**Subprocess Bridge, Sample Songs & Instrument Completeness**  
**Verdict**: **REJECT** (Milestone 7 "ALL INSTRUMENTS MUST BE USED" Invariant Breach on `song_afrobeats_002`)

---

## 1. Observation

### Vector 1: `workers/generate_song.py` & `UvBridge` Verification
- **Existence & Syntax**:
  `workers/generate_song.py` exists (118 lines) and parses into a valid Python Abstract Syntax Tree:
  Command: `python -c "import ast; tree = ast.parse(open('workers/generate_song.py', encoding='utf-8').read()); print(type(tree))"`  
  Result: `<class 'ast.Module'>` (exit code 0).
- **CLI Arguments**:
  Supports `--model`, `--prompt`, `--lyrics_file`, `--bpm`, `--key`, `--duration`, `--output_dir`, `--device`.
  Command: `python workers/generate_song.py --help` exited with code 0.
- **Exit Code Behavior (0, 2, 3)**:
  Empirical multi-trial testing of `main()` in `workers/generate_song.py` verified:
  1. Exit code 0 on successful stem output generation.
  2. Exit code 2 on missing model weights, offline mode, or missing dependencies.
  3. Exit code 3 on CUDA Out Of Memory (`RuntimeError("CUDA out of memory...")`).
- **Defect / Trap Found in `workers/generate_song.py`**:
  Lines 34-40:
  ```python
  try:
      import torch
      import soundfile as sf
  except ImportError as e:
      sys.stderr.write(f"Missing required neural dependencies: {e}\n")
      sys.exit(2)
  ```
  When tested against Windows Python without C-redistributables or missing `shm.dll`:
  ```
  OSError: [WinError 126] The specified module could not be found. Error loading "C:\Python314\Lib\site-packages\torch\lib\shm.dll" or one of its dependencies.
  EXIT_CODE=1
  ```
  `OSError` is not an `ImportError`. It bypasses the `except ImportError:` handler, crashing Python with uncaught exception and exiting with code 1 instead of exit code 2. In `crates/music_pipeline/src/python_bridge.rs`, exit code 1 is caught under wildcard `_ => Err(PipelineError::SubprocessError)`, successfully triggering pure-Rust fallback, but `generate_song.py` violates the spec contract by emitting exit code 1 instead of 2.
- **`UvBridge` Invariant Verification**:
  In `crates/music_pipeline/src/python_bridge.rs`:
  * Windows `CREATE_NO_WINDOW` is defined as `const CREATE_NO_WINDOW: u32 = 0x08000000;` (line 13) and applied to process spawning on lines 108, 146, 170, and 215.
  * Timeouts are enforced using `tokio::time::timeout(timeout_duration, child.wait_with_output()).await` (lines 232-244).
  * Graceful fallback to pure-Rust physical modeling is implemented in `crates/music_pipeline/src/nodes/node2_groove_yue.rs` (lines 152-172).
  * Tokio console logs strictly format:
    - Line 146: `🎛️  [SYNTHESIS ENGINE] Track rendered by Hugging Face GPU weights...`
    - Line 153: `⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: {}. Automatically falling back to pure-Rust physical modeling synthesizer.`
    - Line 176: `🎹  [SYNTHESIS ENGINE] Track rendered by pure-Rust physical modeling synthesizer (Karplus-Strong string algorithm, Reggae one-drop drums & dub bass)`

---

### Vector 2: "ALL INSTRUMENTS MUST BE USED" Empirical Stem Energy Audit
All 56 on-disk stems (7 stems per version across 8 versions in `storage/songs/`) were empirically analyzed using PCM16 sample decoding:

| Song | Version | Stem | Peak (dBFS) | RMS (dBFS) | Zero % | Active (> -60 dBFS)? |
|---|---|---|---|---|---|---|
| `roots_reggae_anthem` | v1 | drums.wav | -12.46 | -41.26 | 9.01% | **PASS** |
| `roots_reggae_anthem` | v1 | bass.wav | -7.13 | -16.46 | 0.01% | **PASS** |
| `roots_reggae_anthem` | v1 | guitar.wav | -20.40 | -42.47 | 0.16% | **PASS** |
| `roots_reggae_anthem` | v1 | other.wav | -11.22 | -25.59 | 0.02% | **PASS** |
| `roots_reggae_anthem` | v1 | yue_vocals.wav | -1.13 | -21.32 | 26.55% | **PASS** |
| `roots_reggae_anthem` | v1 | raspy_tenor_vocals.wav | -0.92 | -21.11 | 26.55% | **PASS** |
| `roots_reggae_anthem` | v1 | final_master.wav | -1.00 | -16.99 | 0.01% | **PASS** |
| `roots_reggae_anthem` | v2 | drums.wav | -12.53 | -41.15 | 8.78% | **PASS** |
| `roots_reggae_anthem` | v2 | bass.wav | -6.60 | -16.46 | 0.00% | **PASS** |
| `roots_reggae_anthem` | v2 | guitar.wav | -19.85 | -42.45 | 0.15% | **PASS** |
| `roots_reggae_anthem` | v2 | other.wav | -12.21 | -25.58 | 0.02% | **PASS** |
| `roots_reggae_anthem` | v2 | yue_vocals.wav | -1.13 | -21.31 | 26.85% | **PASS** |
| `roots_reggae_anthem` | v2 | raspy_tenor_vocals.wav | -0.92 | -21.09 | 26.85% | **PASS** |
| `roots_reggae_anthem` | v2 | final_master.wav | -1.00 | -17.25 | 0.01% | **PASS** |
| `song-one-love-shining-1977` | v1 | drums.wav | -12.46 | -41.26 | 9.01% | **PASS** |
| `song-one-love-shining-1977` | v1 | bass.wav | -7.13 | -16.46 | 0.01% | **PASS** |
| `song-one-love-shining-1977` | v1 | guitar.wav | -20.40 | -42.47 | 0.16% | **PASS** |
| `song-one-love-shining-1977` | v1 | other.wav | -11.22 | -25.59 | 0.02% | **PASS** |
| `song-one-love-shining-1977` | v1 | yue_vocals.wav | -1.13 | -21.32 | 26.55% | **PASS** |
| `song-one-love-shining-1977` | v1 | raspy_tenor_vocals.wav | -0.92 | -21.11 | 26.55% | **PASS** |
| `song-one-love-shining-1977` | v1 | final_master.wav | -1.00 | -16.99 | 0.01% | **PASS** |
| `song-one-love-shining-1977` | v2 | drums.wav | -12.53 | -41.15 | 8.78% | **PASS** |
| `song-one-love-shining-1977` | v2 | bass.wav | -6.60 | -16.46 | 0.00% | **PASS** |
| `song-one-love-shining-1977` | v2 | guitar.wav | -19.85 | -42.45 | 0.15% | **PASS** |
| `song-one-love-shining-1977` | v2 | other.wav | -12.21 | -25.58 | 0.02% | **PASS** |
| `song-one-love-shining-1977` | v2 | yue_vocals.wav | -1.13 | -21.31 | 26.85% | **PASS** |
| `song-one-love-shining-1977` | v2 | raspy_tenor_vocals.wav | -0.92 | -21.09 | 26.85% | **PASS** |
| `song-one-love-shining-1977` | v2 | final_master.wav | -1.00 | -17.44 | 0.01% | **PASS** |
| `song_reggae_anthem_001` | v1 | drums.wav | -12.46 | -41.26 | 9.01% | **PASS** |
| `song_reggae_anthem_001` | v1 | bass.wav | -7.13 | -16.46 | 0.01% | **PASS** |
| `song_reggae_anthem_001` | v1 | guitar.wav | -20.40 | -42.47 | 0.16% | **PASS** |
| `song_reggae_anthem_001` | v1 | other.wav | -11.22 | -25.59 | 0.02% | **PASS** |
| `song_reggae_anthem_001` | v1 | yue_vocals.wav | -1.13 | -21.32 | 26.55% | **PASS** |
| `song_reggae_anthem_001` | v1 | raspy_tenor_vocals.wav | -0.92 | -21.11 | 26.55% | **PASS** |
| `song_reggae_anthem_001` | v1 | final_master.wav | -1.00 | -16.99 | 0.01% | **PASS** |
| `song_reggae_anthem_001` | v2 | drums.wav | -12.53 | -41.15 | 8.78% | **PASS** |
| `song_reggae_anthem_001` | v2 | bass.wav | -6.60 | -16.46 | 0.00% | **PASS** |
| `song_reggae_anthem_001` | v2 | guitar.wav | -19.85 | -42.45 | 0.15% | **PASS** |
| `song_reggae_anthem_001` | v2 | other.wav | -12.21 | -25.58 | 0.02% | **PASS** |
| `song_reggae_anthem_001` | v2 | yue_vocals.wav | -1.13 | -21.31 | 26.85% | **PASS** |
| `song_reggae_anthem_001` | v2 | raspy_tenor_vocals.wav | -0.92 | -21.09 | 26.85% | **PASS** |
| `song_reggae_anthem_001` | v2 | final_master.wav | -1.00 | -17.25 | 0.01% | **PASS** |
| `song_afrobeats_002` | v1 | **drums.wav** | -34.50 | **-68.79** | 8.82% | **FAIL** (< -60 dBFS) |
| `song_afrobeats_002` | v1 | bass.wav | -5.43 | -15.68 | 0.01% | **PASS** |
| `song_afrobeats_002` | v1 | guitar.wav | -19.34 | -42.40 | 0.15% | **PASS** |
| `song_afrobeats_002` | v1 | other.wav | -9.70 | -24.98 | 0.02% | **PASS** |
| `song_afrobeats_002` | v1 | yue_vocals.wav | -1.04 | -21.28 | 27.30% | **PASS** |
| `song_afrobeats_002` | v1 | raspy_tenor_vocals.wav | -0.92 | -21.17 | 27.29% | **PASS** |
| `song_afrobeats_002` | v1 | final_master.wav | -1.00 | -16.60 | 0.01% | **PASS** |
| `song_afrobeats_002` | v2 | **drums.wav** | -34.66 | **-68.67** | 8.64% | **FAIL** (< -60 dBFS) |
| `song_afrobeats_002` | v2 | bass.wav | -5.03 | -15.51 | 0.01% | **PASS** |
| `song_afrobeats_002` | v2 | guitar.wav | -19.22 | -42.24 | 0.15% | **PASS** |
| `song_afrobeats_002` | v2 | other.wav | -9.03 | -24.82 | 0.02% | **PASS** |
| `song_afrobeats_002` | v2 | yue_vocals.wav | -1.04 | -21.46 | 28.35% | **PASS** |
| `song_afrobeats_002` | v2 | raspy_tenor_vocals.wav | -0.92 | -21.36 | 28.33% | **PASS** |
| `song_afrobeats_002` | v2 | final_master.wav | -1.00 | -16.35 | 0.01% | **PASS** |

- **Verification failure in test suite**:
  Running `python tests/audit_stems_non_silence.py`:
  ```
  AssertionError: SILENT STEM DETECTED: song_afrobeats_002/v1/drums.wav rms=-68.52996626651088
  ```
- **Root Cause Analysis**:
  In `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`:
  ```rust
  // Line 153-154:
  // 3. Drums: Highpass at 3500 Hz + punch band
  let mut drums_hp = StemFilter::highpass(3500.0, 0.707, channels, sr);
  ```
  `separate_stems_pure_rust` applies a 3500 Hz highpass filter to `yue_instrumentals.wav` to produce `drums.wav`. While Reggae drums feature high-frequency >6000 Hz metallic sizzle (`HiHat16thMetallic`), Afrobeats percussion (`UniversalDrumSynthesizer` with `RhythmGroove::AfrobeatTresillo`) consists of low/mid instruments: Amapiano log drum, shaker, clave, and soft kick. The 3500 Hz highpass filter almost completely removes this acoustic energy, attenuating `drums.wav` to -68.79 dBFS. (Note that `inpainted_drums.wav` is active at -21.41 dBFS because drum fills are spliced in during Node 4, but the raw `drums.wav` stem is effectively inaudible).

---

### Vector 3: Sample Song Integrity
Command: `python tests/validate_disk_songs.py`
Output:
```
=========================================================================
RESULT: 100% EMPIRICAL VALIDATION PASSED!
Checked 8 song versions (4 songs * 2 versions).
Checked 88 total WAV files (11 files per version).
All true-peaks <= 0.96605, zero clipping, all multi-section lyrics complete, zero UTF-8 BOM.
=========================================================================
```
- All 88 files exist with valid 48 kHz stereo 16-bit PCM WAV headers.
- True-peak on all 8 `final_master.wav` files is strictly 0.89124 (-1.00 dBFS), below the 0.96605 ceiling (-0.30 dBFS).
- Zero clipped samples across all stems and masters.
- Section completeness: All 7 canonical sections (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Verse 2]`, `[Guitar Solo]`, `[Bridge]`, `[Outro]`) verified across `lyrics.txt`, `manifest.json`, and `state.json`.

---

### Vector 4: Workspace Test Suites
1. `cargo test --workspace` passed 100% (exit code 0):
   - `music_core`: 27/27 passed
   - `music_dsp`: 104/104 passed
   - `music_fusion`: 43/43 passed
   - `music_pipeline`: 60/60 passed
   - `music_cli`: 39/39 passed
2. `cargo test --manifest-path tests/Cargo.toml` passed 100%: 191/191 passed.

---

## 2. Logic Chain

1. The critical requirement reinforcement from the user and orchestrator explicitly states:
   *"all instruments must be used... no instrument stem may be dropped, muted, or left silent in either the individual stems or the summed master mix!"*
2. The challenger dispatch specifically mandated:
   *"Verify every single stem has non-trivial RMS energy (> -60 dBFS) and non-zero samples (no muted, dropped, or silent stems)."*
3. Our empirical audio analysis showed that in `song_afrobeats_002`:
   - `v1/stems/drums.wav` has RMS = **-68.79 dBFS**.
   - `v2/stems/drums.wav` has RMS = **-68.67 dBFS**.
   Both fall below the -60.0 dBFS threshold, breaching the non-silent stem requirement.
4. Furthermore, the repository's test `tests/audit_stems_non_silence.py` asserts `st["rms_db"] > -50.0` across all stems and crashes with an `AssertionError` on `song_afrobeats_002/v1/drums.wav`.
5. The root cause is mathematical: `StemFilter::highpass(3500.0, 0.707)` in `node4_stem_inpainting.rs` only passes frequencies above 3.5 kHz, which discards Afrobeats shaker, clave, and log drum fundamentals.
6. In `workers/generate_song.py`, catching only `ImportError` on line 38 allows Windows `OSError` (e.g. `shm.dll` loading failures) to escape as exit code 1 instead of exit code 2.

---

## 3. Caveats
- `final_master.wav` across all 8 song versions (including `song_afrobeats_002`) has full acoustic energy (-16.35 dBFS to -17.44 dBFS) and zero clipping. The issue is isolated to the decomposed `drums.wav` stem in `song_afrobeats_002`.
- All 6 Reggae song versions (`roots_reggae_anthem`, `song-one-love-shining-1977`, `song_reggae_anthem_001` across v1 & v2) fully satisfy all criteria with robust RMS levels (> -42.5 dBFS on all stems).
- Pure-Rust physical modeling, headroom scaling (0.20), Butterworth 12 kHz low-pass filtering, `tanh` saturation, and hound PCM16 serialization are 100% verified and operating correctly.

---

## 4. Conclusion
While the physical modeling synthesis, mastering headroom, UvBridge process flags, and sample song serialization meet the core Milestone 7 objectives, the explicit invariant:
**"every single stem has non-trivial RMS energy (> -60 dBFS)"**
is violated on `storage/songs/song_afrobeats_002/v1/stems/drums.wav` (-68.79 dBFS) and `v2/stems/drums.wav` (-68.67 dBFS).

**Explicit Verdict**: **REJECT**

### Required Remediation for worker_m7:
1. In `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`:
   Update `separate_stems_pure_rust` (lines 153-166) so that the drum stem does not rely solely on a 3500 Hz highpass filter when the underlying rhythm has low/mid percussion (or mix `draft_groove` into `drums.wav` with appropriate scaling).
2. In `workers/generate_song.py`:
   Change `except ImportError as e:` on line 38 to `except Exception as e:` so that Windows DLL/`OSError` import failures cleanly exit with code 2 instead of code 1.
3. Regenerate `storage/songs/song_afrobeats_002` (v1 and v2) and confirm `python tests/audit_stems_non_silence.py` passes with zero assertion errors.

---

## 5. Verification Method

To independently verify these findings:

1. **Verify the failing `drums.wav` RMS on `song_afrobeats_002`**:
   ```powershell
   python tests/audit_stems_non_silence.py
   ```
   *Expected result*: Fails with `AssertionError: SILENT STEM DETECTED: song_afrobeats_002/v1/drums.wav rms=-68.52996626651088`.

2. **Verify exact stem RMS levels across all 8 versions**:
   ```powershell
   python -c "
   import wave, struct, math
   for song in ['roots_reggae_anthem', 'song-one-love-shining-1977', 'song_afrobeats_002', 'song_reggae_anthem_001']:
       for ver in ['v1', 'v2']:
           p = f'storage/songs/{song}/{ver}/stems/drums.wav'
           w = wave.open(p, 'rb')
           raw = w.readframes(w.getnframes())
           samples = [struct.unpack('<h', raw[i:i+2])[0] / 32768.0 for i in range(0, len(raw), 2)]
           rms = math.sqrt(sum(s*s for s in samples) / len(samples))
           db = 20 * math.log10(rms)
           print(f'{song}/{ver}/drums.wav RMS: {db:.2f} dBFS')
   "
   ```
   *Observation*: `song_afrobeats_002` reports `-68.79 dBFS` and `-68.67 dBFS`, failing the `> -60 dBFS` invariant.

3. **Verify sample song integrity suite**:
   ```powershell
   python tests/validate_disk_songs.py
   ```

4. **Verify full workspace tests**:
   ```powershell
   cargo test --workspace
   cargo test --manifest-path tests/Cargo.toml
   ```
