import os
import struct
import math
import array
import sys

def inspect_wav(file_path):
    assert os.path.exists(file_path), f"File {file_path} does not exist"
    file_size = os.path.getsize(file_path)
    assert file_size > 44, f"File {file_path} too small ({file_size} bytes)"

    with open(file_path, 'rb') as f:
        riff_header = f.read(12)
        riff_id, riff_size, wave_id = struct.unpack('<4sI4s', riff_header)
        assert riff_id == b'RIFF', f"Invalid RIFF ID: {riff_id}"
        assert wave_id == b'WAVE', f"Invalid WAVE ID: {wave_id}"

        fmt_found = False
        data_found = False
        audio_format = 0
        channels = 0
        sample_rate = 0
        bits_per_sample = 0
        data_bytes = b''
        data_size = 0

        while True:
            chunk_header = f.read(8)
            if len(chunk_header) < 8:
                break
            chunk_id, chunk_size = struct.unpack('<4sI', chunk_header)

            if chunk_id == b'fmt ':
                fmt_found = True
                fmt_data = f.read(chunk_size)
                audio_format, channels, sample_rate, byte_rate, block_align, bits_per_sample = struct.unpack(
                    '<HHIIHH', fmt_data[:16]
                )
            elif chunk_id == b'data':
                data_found = True
                data_size = chunk_size
                data_bytes = f.read(chunk_size)
                if chunk_size % 2 == 1:
                    f.read(1)
            else:
                f.seek(chunk_size, 1)

        assert fmt_found, f"Missing fmt chunk in {file_path}"
        assert data_found, f"Missing data chunk in {file_path}"
        assert sample_rate in [44100, 48000], f"Unexpected sample rate {sample_rate} in {file_path}"
        assert channels in [1, 2], f"Unexpected channels {channels} in {file_path}"

        if audio_format == 3 and bits_per_sample == 32:
            arr = array.array('f')
            arr.frombytes(data_bytes)
            num_samples = len(arr)
            min_val = min(arr)
            max_val = max(arr)
            peak = max(abs(min_val), abs(max_val))
            # Sample every 100th element for fast RMS approximation or compute over array
            # Actually with array, slice or fast sum
            step = 100
            sampled = arr[::step]
            sum_sq = sum(s * s for s in sampled)
            rms = math.sqrt(sum_sq / len(sampled))
        elif audio_format == 1 and bits_per_sample == 16:
            arr = array.array('h')
            arr.frombytes(data_bytes)
            num_samples = len(arr)
            min_val = min(arr) / 32768.0
            max_val = max(arr) / 32768.0
            peak = max(abs(min_val), abs(max_val))
            step = 100
            sampled = arr[::step]
            sum_sq = sum((s / 32768.0) ** 2 for s in sampled)
            rms = math.sqrt(sum_sq / len(sampled))
        else:
            raise ValueError(f"Unsupported format: audio_format={audio_format}, bits={bits_per_sample}")

        peak_dbfs = 20 * math.log10(peak) if peak > 1e-9 else -120.0
        rms_dbfs = 20 * math.log10(rms) if rms > 1e-9 else -120.0

        return {
            "path": file_path,
            "size_bytes": file_size,
            "sample_rate": sample_rate,
            "channels": channels,
            "bits_per_sample": bits_per_sample,
            "audio_format": "IEEE_FLOAT" if audio_format == 3 else "PCM",
            "num_samples": num_samples,
            "duration_secs": num_samples / (channels * sample_rate),
            "peak": peak,
            "peak_dbfs": peak_dbfs,
            "rms": rms,
            "rms_dbfs": rms_dbfs,
            "min_val": min_val,
            "max_val": max_val,
            "is_silent": peak < 1e-5,
            "ceiling_ok": peak <= 0.966001,
        }

def main():
    songs_dir = r"d:\harfile\music\storage\songs"
    assert os.path.exists(songs_dir), f"Songs dir does not exist: {songs_dir}"

    inspected = []
    failed_assertions = []

    for root, dirs, files in os.walk(songs_dir):
        for f in files:
            if f.endswith(".wav"):
                wav_path = os.path.join(root, f)
                info = inspect_wav(wav_path)
                inspected.append(info)
                rel = os.path.relpath(wav_path, songs_dir)
                print(f"[{rel}]")
                print(f"  {info['sample_rate']}Hz, {info['channels']}ch, {info['bits_per_sample']}-bit {info['audio_format']} | {info['duration_secs']:.2f}s | Peak: {info['peak']:.4f} ({info['peak_dbfs']:.2f} dBFS) | RMS: {info['rms_dbfs']:.2f} dBFS")
                print(f"  Non-silent: {not info['is_silent']} | Ceiling <= 0.966: {info['ceiling_ok']}")

                if info['is_silent']:
                    failed_assertions.append(f"SILENT FILE: {wav_path}")
                if "final_master" in f and not info['ceiling_ok']:
                    failed_assertions.append(f"CEILING BREACH on final master: {wav_path} (peak={info['peak']:.5f} > 0.966)")

    print(f"\nTotal WAV files inspected: {len(inspected)}")
    if failed_assertions:
        print("\nERRORS DETECTED:")
        for err in failed_assertions:
            print("  - " + err)
        sys.exit(1)
    else:
        print("\nALL AUDIO FILES PASSED INTEGRITY & CEILING CHECKS EMPIRICALLY!")

if __name__ == "__main__":
    main()
