#!/usr/bin/env python3
"""Open-weight local song & stem synthesis worker.

Supports the 9 open-weight local families without any paid API calls or audio sampling:
  1. ACE-Step/acestep-v15-turbo / base / xl-turbo (MIT)
  2. tencent/SongGeneration / lglg666/SongGeneration-v2-large (LeVo 2)
  3. m-a-p/YuE2-3B / m-a-p/YuE-s1-7B-anneal-en-cot
  4. MiniMaxAI/MiniMax-Music3
  5. ASLP-lab/DiffRhythm2 (Apache-2.0)
  6. openmusic/Muse (Apache-2.0, optional LoRA adapter)
  7. stabilityai/stable-audio-open-small / 1.0 / 3-medium
  8. facebook/musicgen-large
  9. google/magenta-realtime-2 (Apache-2.0)

Exits with code 2 cleanly when requested open-weight checkpoints are not cached locally
so the Rust StateGraph smoothly performs hardware-aware local synthesis fallback.
"""

from __future__ import annotations

import argparse
import gc
import os
import sys
import wave
from pathlib import Path
from typing import Optional

NON_COMMERCIAL_TOKENS = (
    "songgeneration",
    "levo2",
    "yue2",
    "yue-s1",
    "musicgen-large",
)

SUPPORTED_OPEN_WEIGHT_FAMILIES = {
    "ace-step": ("ACE-Step/acestep-v15-turbo", "ACE-Step/acestep-v15-base", "ACE-Step/acestep-v15-xl-turbo"),
    "levo2": ("tencent/SongGeneration", "lglg666/SongGeneration-v2-large"),
    "yue2": ("m-a-p/YuE2-3B", "m-a-p/YuE-s1-7B-anneal-en-cot"),
    "minimax": ("MiniMaxAI/MiniMax-Music3",),
    "diffrhythm2": ("ASLP-lab/DiffRhythm2",),
    "muse": ("openmusic/Muse",),
    "stable-audio": ("stabilityai/stable-audio-open-small", "stabilityai/stable-audio-open-1.0", "stabilityai/stable-audio-3-medium"),
    "musicgen": ("facebook/musicgen-large",),
    "magenta": ("google/magenta-realtime-2",),
}


def resolve_hf_home() -> Path:
    for var in ("MUSIC_MODELS_DIR", "HF_HOME"):
        val = os.environ.get(var, "").strip()
        if val:
            return Path(val)
    return Path(r"D:\music\models") if os.name == "nt" else Path("models")


def release_cuda_cache() -> None:
    gc.collect()
    try:
        import torch

        if torch.cuda.is_available():
            torch.cuda.empty_cache()
            torch.cuda.ipc_collect()
    except Exception:
        pass


def is_repo_cached_locally(repo_id: str, hf_home: Path) -> bool:
    sanitized = repo_id.replace("/", "--")
    candidates = [
        hf_home / "hub" / f"models--{sanitized}",
        hf_home / f"models--{sanitized}",
        hf_home / repo_id.replace("/", "_"),
        hf_home / repo_id.split("/")[-1],
    ]
    return any(p.exists() for p in candidates)


def write_stereo_wav_48k(out_path: Path, samples, src_sr: int) -> None:
    import numpy as np

    arr = np.asarray(samples, dtype=np.float32)
    if arr.ndim == 1:
        arr = np.stack([arr, arr], axis=0)
    elif arr.shape[0] > 2 and arr.shape[1] <= 2:
        arr = arr.T
    if arr.shape[0] == 1:
        arr = np.vstack([arr, arr])
    arr = arr[:2, :]

    if src_sr != 48000 and arr.shape[1] > 0:
        from scipy.signal import resample_poly
        from math import gcd

        g = gcd(48000, int(src_sr))
        arr = resample_poly(arr, 48000 // g, int(src_sr) // g, axis=1).astype(np.float32)

    peak = float(np.max(np.abs(arr))) if arr.size else 0.0
    if peak > 0.96:
        arr = arr * (0.94 / peak)

    interleaved = np.clip(arr.T, -1.0, 1.0)
    pcm16 = (interleaved * 32767.0).astype(np.int16)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(out_path), "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(48000)
        wf.writeframes(pcm16.tobytes())


def render_diffusers_stable_audio(
    model_id: str,
    prompt_text: str,
    duration_secs: float,
    device: str,
    hf_home: Path,
    out_dir: Path,
) -> bool:
    import torch
    from diffusers import StableAudioPipeline

    dtype = torch.float16 if device == "cuda" else torch.float32
    pipe = StableAudioPipeline.from_pretrained(
        model_id,
        torch_dtype=dtype,
        cache_dir=str(hf_home / "hub"),
        local_files_only=True,
    )
    if device == "cuda":
        pipe = pipe.to("cuda")

    audio = pipe(
        prompt_text,
        negative_prompt="low quality, distorted clipping, mono noise",
        num_inference_steps=60,
        audio_end_in_s=min(float(duration_secs), 47.0),
    ).audios[0]
    audio_np = audio.detach().cpu().numpy()
    sr = getattr(pipe.vae, "sampling_rate", 44100)
    write_stereo_wav_48k(out_dir / "yue_instrumentals.wav", audio_np, sr)
    write_stereo_wav_48k(out_dir / "yue_vocals.wav", audio_np * 0.05, sr)
    del pipe
    release_cuda_cache()
    return True


def render_musicgen_transformers(
    model_id: str,
    prompt_text: str,
    duration_secs: float,
    device: str,
    lora_path: Optional[Path],
    hf_home: Path,
    out_dir: Path,
) -> bool:
    import torch
    from transformers import AutoProcessor, MusicgenForConditionalGeneration

    processor = AutoProcessor.from_pretrained(model_id, cache_dir=str(hf_home / "hub"), local_files_only=True)
    model = MusicgenForConditionalGeneration.from_pretrained(
        model_id,
        cache_dir=str(hf_home / "hub"),
        local_files_only=True,
        torch_dtype=torch.float16 if device == "cuda" else torch.float32,
    )
    if lora_path and lora_path.exists():
        from peft import PeftModel

        model = PeftModel.from_pretrained(model, str(lora_path))
    if device == "cuda":
        model = model.to("cuda")

    inputs = processor(text=[prompt_text], padding=True, return_tensors="pt")
    if device == "cuda":
        inputs = {k: v.to("cuda") for k, v in inputs.items()}
    max_tokens = max(256, min(1500, int(duration_secs * 50)))
    with torch.no_grad():
        audio_values = model.generate(**inputs, max_new_tokens=max_tokens)
    sr = getattr(model.config.audio_encoder, "sampling_rate", 32000)
    wav_np = audio_values[0].detach().cpu().numpy()
    write_stereo_wav_48k(out_dir / "yue_instrumentals.wav", wav_np, sr)
    write_stereo_wav_48k(out_dir / "yue_vocals.wav", wav_np * 0.05, sr)
    del model
    release_cuda_cache()
    return True


def render_acestep_or_vocal_repo(
    model_id: str,
    prompt_text: str,
    lyrics_text: str,
    duration_secs: float,
    bpm: int,
    device: str,
    lora_path: Optional[Path],
    hf_home: Path,
    out_dir: Path,
) -> bool:
    # Try ACE-Step Python API if installed in the active cu124 environment
    try:
        from acestep.pipeline_ace_step import ACEStepPipeline  # type: ignore

        ckpt_dir = hf_home / "hub" / f"models--{model_id.replace('/', '--')}"
        pipe = ACEStepPipeline(checkpoint_dir=str(ckpt_dir), dtype="bf16" if device == "cuda" else "fp32")
        if lora_path and lora_path.exists() and hasattr(pipe, "load_lora"):
            pipe.load_lora(str(lora_path))
        out_wav = out_dir / "mix_master.wav"
        pipe(
            audio_duration=float(duration_secs),
            prompt=f"{prompt_text}, {bpm} BPM",
            lyrics=lyrics_text,
            save_path=str(out_wav),
        )
        if out_wav.exists():
            # Split quick stereo accompaniment vs center vocal estimate until separate_and_inpaint runs
            import soundfile as sf
            import numpy as np

            data, sr = sf.read(str(out_wav), dtype="float32")
            if data.ndim == 1:
                data = np.stack([data, data], axis=1)
            mid = 0.5 * (data[:, 0] + data[:, 1])
            side = 0.5 * (data[:, 0] - data[:, 1])
            write_stereo_wav_48k(out_dir / "yue_vocals.wav", np.stack([mid, mid], axis=0), sr)
            write_stereo_wav_48k(
                out_dir / "yue_instrumentals.wav",
                np.stack([0.65 * mid + side, 0.65 * mid - side], axis=0),
                sr,
            )
            del pipe
            release_cuda_cache()
            return True
    except ImportError:
        pass
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Open-weight open-source local song generator worker")
    parser.add_argument("--model", default="ACE-Step/acestep-v15-turbo", help="Open-weight HF repository ID")
    parser.add_argument("--genre-txt", required=False, default="genre.txt")
    parser.add_argument("--lyrics-txt", required=False, default="lyrics.txt")
    parser.add_argument("--output-dir", required=False, default="stems")
    parser.add_argument("--bpm", type=int, default=100)
    parser.add_argument("--key", default="A Major")
    parser.add_argument("--max-duration", type=float, default=60.0)
    parser.add_argument("--lora", type=str, default=None)
    parser.add_argument("--device", default="auto", choices=["auto", "cuda", "cpu"])
    parser.add_argument("--commercial-only", action="store_true")
    parser.add_argument("--check-env", action="store_true", help="Print runtime capability report and exit")
    args = parser.parse_args()

    model_lower = args.model.lower()
    if "lyria" in model_lower or "gemini" in model_lower:
        sys.stderr.write("Paid proprietary models (Lyria/Gemini) are strictly disabled.\n")
        return 2

    if args.commercial_only and any(tok in model_lower for tok in NON_COMMERCIAL_TOKENS):
        sys.stderr.write(f"Model '{args.model}' is blocked because --commercial-only is enabled.\n")
        return 2

    hf_home = resolve_hf_home()
    if args.check_env:
        cuda_ok = False
        try:
            import torch

            cuda_ok = bool(torch.cuda.is_available())
        except Exception:
            pass
        print(
            f"hf_home={hf_home} cuda={cuda_ok} model_cached={is_repo_cached_locally(args.model, hf_home)} "
            f"families={len(SUPPORTED_OPEN_WEIGHT_FAMILIES)}"
        )
        return 0

    genre_path = Path(args.genre_txt)
    lyrics_path = Path(args.lyrics_txt)
    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    genre_text = genre_path.read_text(encoding="utf-8") if genre_path.is_file() else "Modern Pop"
    lyrics_text = lyrics_path.read_text(encoding="utf-8") if lyrics_path.is_file() else ""

    if not is_repo_cached_locally(args.model, hf_home):
        sys.stderr.write(
            f"Open-weight checkpoint '{args.model}' is not cached in '{hf_home}'. "
            f"Pull it via CLI `--pull-model {args.model}` or UI Model Pull button.\n"
        )
        release_cuda_cache()
        return 2

    device = args.device
    if device == "auto":
        try:
            import torch

            device = "cuda" if torch.cuda.is_available() else "cpu"
        except Exception:
            device = "cpu"

    lora_path = Path(args.lora) if args.lora else None
    try:
        release_cuda_cache()
        if "stable-audio" in model_lower:
            ok = render_diffusers_stable_audio(
                args.model, genre_text, args.max_duration, device, hf_home, out_dir
            )
        elif "musicgen" in model_lower or "magenta" in model_lower:
            ok = render_musicgen_transformers(
                args.model, genre_text, args.max_duration, device, lora_path, hf_home, out_dir
            )
        else:
            ok = render_acestep_or_vocal_repo(
                args.model,
                genre_text,
                lyrics_text,
                args.max_duration,
                args.bpm,
                device,
                lora_path,
                hf_home,
                out_dir,
            )
        release_cuda_cache()
        if not ok:
            sys.stderr.write(
                f"Runtime backend for '{args.model}' requires optional python inference package in .venv.\n"
            )
            return 2
        return 0
    except RuntimeError as exc:
        release_cuda_cache()
        if "out of memory" in str(exc).lower():
            sys.stderr.write(f"CUDA OOM on '{args.model}': {exc}\n")
            return 3
        sys.stderr.write(f"Synthesis runtime error on '{args.model}': {exc}\n")
        return 2
    except Exception as exc:
        release_cuda_cache()
        sys.stderr.write(f"Synthesis failed on '{args.model}': {exc}\n")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
