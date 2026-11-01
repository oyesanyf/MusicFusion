#!/usr/bin/env python3
"""
tests/audit_stems_non_silence.py

Milestone 7 Stem Non-Silence & RMS Energy Audit:
Verifies the 'ALL INSTRUMENTS MUST BE USED' invariant across all 8 song versions:
1. roots_reggae_anthem (v1, v2)
2. song-one-love-shining-1977 (v1, v2)
3. song_afrobeats_002 (v1, v2)
4. song_reggae_anthem_001 (v1, v2)

Asserts:
- Every single stem is active and non-silent (RMS > -50.0 dBFS, peak <= 1.0)
- Zero muted, dropped, or silent stems
"""

import os
import sys
import wave
import array
import math

if sys.stdout.encoding != 'utf-8':
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except Exception:
        pass

SONGS = [
    "roots_reggae_anthem",
    "song-one-love-shining-1977",
    "song_afrobeats_002",
    "song_reggae_anthem_001"
]

VERSIONS = ["v1", "v2"]

AUDIT_STEMS = [
    ("drums.wav", "stems/drums.wav"),
    ("bass.wav", "stems/bass.wav"),
    ("guitar.wav", "stems/guitar.wav"),
    ("other.wav", "stems/other.wav"),
    ("yue_vocals.wav", "stems/yue_vocals.wav"),
    ("raspy_tenor_vocals.wav", "stems/raspy_tenor_vocals.wav"),
    ("final_master.wav", "final_master.wav"),
]

SILENCE_THRESHOLD_DB = -50.0 # Strict non-silence threshold (> -60.0 dBFS invariant)

def compute_wav_stats(filepath):
    with wave.open(filepath, "rb") as w:
        nchannels = w.getnchannels()
        sampwidth = w.getsampwidth()
        nframes = w.getnframes()
        sr = w.getframerate()
        raw = w.readframes(nframes)

    if sampwidth == 2:
        arr = array.array("h")
        arr.frombytes(raw)
        total_samples = len(arr)
        if total_samples == 0:
            return {"channels": nchannels, "sample_rate": sr, "duration_secs": 0.0,
                    "peak": 0.0, "peak_db": -100.0, "rms": 0.0, "rms_db": -100.0, "zero_pct": 100.0}

        peak_int = max(abs(x) for x in arr)
        peak = peak_int / 32768.0
        sum_sq = sum(x * x for x in arr) / (32768.0 * 32768.0)
        rms = math.sqrt(sum_sq / total_samples)
        zero_pct = (sum(1 for x in arr if x == 0) / total_samples) * 100.0
    else:
        raise ValueError(f"Unsupported sample width: {sampwidth} in {filepath}")

    rms_db = 20.0 * math.log10(rms) if rms > 1e-9 else -100.0
    peak_db = 20.0 * math.log10(peak) if peak > 1e-9 else -100.0

    return {
        "channels": nchannels,
        "sample_rate": sr,
        "duration_secs": nframes / float(sr),
        "peak": peak,
        "peak_db": peak_db,
        "rms": rms,
        "rms_db": rms_db,
        "zero_pct": zero_pct
    }

def main():
    base_dir = "storage/songs"
    if not os.path.exists(base_dir):
        base_dir = "d:/harfile/music/storage/songs"

    print("=========================================================================")
    print("RUNNING STEM NON-SILENCE & RMS ENERGY AUDIT")
    print(f"Base Directory: {base_dir}")
    print(f"Non-Silence Threshold: > {SILENCE_THRESHOLD_DB} dBFS")
    print("=========================================================================\n")

    total_audited = 0
    failures = []

    header = f"{'Song':<28} | {'Ver':<3} | {'Stem':<22} | {'Peak (dBFS)':<11} | {'RMS (dBFS)':<10} | {'Zero %':<7} | {'Status'}"
    print(header)
    print("-" * len(header))

    for song in SONGS:
        for ver in VERSIONS:
            ver_dir = os.path.join(base_dir, song, ver)
            assert os.path.exists(ver_dir), f"Directory missing: {ver_dir}"

            for stem_name, rel_path in AUDIT_STEMS:
                full_path = os.path.join(ver_dir, rel_path)
                assert os.path.exists(full_path), f"Stem missing: {full_path}"
                st = compute_wav_stats(full_path)
                total_audited += 1

                is_pass = (st["rms_db"] > SILENCE_THRESHOLD_DB) and (st["peak"] <= 1.0001)
                status_str = "PASS" if is_pass else "FAIL"

                print(
                    f"{song:<28} | {ver:<3} | {stem_name:<22} | "
                    f"{st['peak_db']:>11.2f} | {st['rms_db']:>10.2f} | "
                    f"{st['zero_pct']:>6.2f}% | {status_str}",
                    flush=True
                )

                if not is_pass:
                    failures.append((song, ver, stem_name, st["rms_db"]))

    print("\n" + "=" * 73)
    if failures:
        print(f"AUDIT FAILED: {len(failures)} silent or clipped stems detected!")
        for f in failures:
            print(f"  FAILED: {f[0]}/{f[1]}/{f[2]} rms={f[3]:.2f} dBFS")
        first = failures[0]
        raise AssertionError(f"SILENT STEM DETECTED: {first[0]}/{first[1]}/{first[2]} rms={first[3]}")
    else:
        print(f"AUDIT PASSED: All {total_audited} stems have robust RMS energy (> {SILENCE_THRESHOLD_DB} dBFS) and zero clipping!")
    print("=========================================================================\n")

if __name__ == "__main__":
    main()
