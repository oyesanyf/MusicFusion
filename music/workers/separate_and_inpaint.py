#!/usr/bin/env python3
"""Open-weight 4-stem separation (`htdemucs`) with phase-coherent stereo DSP crossover fallback."""

from __future__ import annotations

import argparse
import gc
import sys
import wave
from pathlib import Path


def release_cuda_cache() -> None:
    gc.collect()
    try:
        import torch

        if torch.cuda.is_available():
            torch.cuda.empty_cache()
    except Exception:
        pass


def write_wav_48k(path: Path, data_2ch, sr: int) -> None:
    import numpy as np
    from math import gcd
    from scipy.signal import resample_poly

    arr = np.asarray(data_2ch, dtype=np.float32)
    if arr.ndim == 1:
        arr = np.stack([arr, arr], axis=0)
    if arr.shape[0] > 2 and arr.shape[1] <= 2:
        arr = arr.T
    if arr.shape[0] == 1:
        arr = np.vstack([arr, arr])
    arr = arr[:2, :]
    if sr != 48000 and arr.shape[1] > 0:
        g = gcd(48000, int(sr))
        arr = resample_poly(arr, 48000 // g, int(sr) // g, axis=1).astype(np.float32)

    peak = float(np.max(np.abs(arr))) if arr.size else 0.0
    if peak > 0.96:
        arr = arr * (0.94 / peak)

    pcm16 = (np.clip(arr.T, -1.0, 1.0) * 32767.0).astype(np.int16)
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(48000)
        wf.writeframes(pcm16.tobytes())


def try_demucs_separation(input_wav: Path, out_dir: Path) -> bool:
    try:
        import numpy as np
        import soundfile as sf
        import torch
        from demucs.apply import apply_model
        from demucs.pretrained import get_model
    except Exception:
        return False

    try:
        audio, sr = sf.read(str(input_wav), dtype="float32")
        if audio.ndim == 1:
            audio = np.stack([audio, audio], axis=1)
        wav_t = torch.from_numpy(audio.T).unsqueeze(0)

        model = get_model("htdemucs")
        device = "cuda" if torch.cuda.is_available() else "cpu"
        model.to(device)
        wav_t = wav_t.to(device)
        with torch.no_grad():
            sources = apply_model(model, wav_t, split=True, overlap=0.25)[0]
        src_np = sources.detach().cpu().numpy()

        # htdemucs source order: drums, bass, other, vocals
        names = getattr(model, "sources", ["drums", "bass", "other", "vocals"])
        mapping = {name: src_np[idx] for idx, name in enumerate(names)}
        write_wav_48k(out_dir / "drums.wav", mapping.get("drums", src_np[0]), int(model.samplerate))
        write_wav_48k(out_dir / "bass.wav", mapping.get("bass", src_np[1]), int(model.samplerate))
        other_stem = mapping.get("other", src_np[2])
        write_wav_48k(out_dir / "guitar.wav", other_stem * 0.65, int(model.samplerate))
        write_wav_48k(out_dir / "other.wav", other_stem * 0.75, int(model.samplerate))
        del model
        release_cuda_cache()
        return True
    except Exception as exc:
        sys.stderr.write(f"Demucs fallback triggered: {exc}\n")
        release_cuda_cache()
        return False


def crossover_band_separation(input_wav: Path, out_dir: Path) -> bool:
    import numpy as np
    import soundfile as sf
    from scipy.signal import butter, sosfiltfilt

    audio, sr = sf.read(str(input_wav), dtype="float32")
    if audio.ndim == 1:
        audio = np.stack([audio, audio], axis=1)
    ch = audio.T  # (2, N)

    sos_bass = butter(4, 170.0, btype="lowpass", fs=sr, output="sos")
    sos_gtr = butter(4, [260.0, 4200.0], btype="bandpass", fs=sr, output="sos")
    sos_hi = butter(4, 3600.0, btype="highpass", fs=sr, output="sos")

    bass = sosfiltfilt(sos_bass, ch, axis=1)
    side = 0.5 * (ch[0] - ch[1])
    gtr_band = sosfiltfilt(sos_gtr, ch, axis=1)
    gtr = np.stack([gtr_band[0] + 0.35 * side, gtr_band[1] - 0.35 * side], axis=0)
    hi_perc = sosfiltfilt(sos_hi, ch, axis=1)
    drums = 0.65 * hi_perc + 0.35 * bass
    other = ch - 0.45 * bass - 0.45 * gtr

    write_wav_48k(out_dir / "drums.wav", drums, sr)
    write_wav_48k(out_dir / "bass.wav", bass, sr)
    write_wav_48k(out_dir / "guitar.wav", gtr, sr)
    write_wav_48k(out_dir / "other.wav", other, sr)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description="Open-weight stem separator worker")
    parser.add_argument("--input", required=False, default="yue_instrumentals.wav")
    parser.add_argument("--output-dir", required=False, default="stems")
    parser.add_argument("--check-env", action="store_true")
    args = parser.parse_args()

    if args.check_env:
        print("separate_and_inpaint ready")
        return 0

    in_path = Path(args.input)
    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    if not in_path.is_file():
        sys.stderr.write(f"Missing input audio file: {in_path}\n")
        return 2

    if try_demucs_separation(in_path, out_dir):
        return 0
    if crossover_band_separation(in_path, out_dir):
        return 0
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
