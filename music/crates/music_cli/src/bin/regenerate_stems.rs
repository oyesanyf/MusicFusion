//! Helper utility to regenerate all disk sample songs with true melodic singing synthesis
//! and Studer A800 tape mastering, strictly preserving exact frame counts and non-silence invariants.

use std::fs;
use std::path::PathBuf;
use music_dsp::melodic_singing::MelodicSingingSynthesizer;
use music_dsp::vocal_formant::{apply_vocal_timbre, PitchLockVocoder};
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_dsp::tape_mastering::{apply_analog_mastering, ButterworthLowpass12kHz, MasteringProfile};
use music_pipeline::nodes::node5_dsp_judge_gate::mix_buffers;

struct SongTarget {
    song_id: &'static str,
    version: u32,
    bpm: u32,
    root_hz: f32,
    genre: &'static str,
    expected_frames: usize,
}

fn main() -> anyhow::Result<()> {
    let targets = [
        SongTarget {
            song_id: "roots_reggae_anthem",
            version: 1,
            bpm: 76,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "roots_reggae_anthem",
            version: 2,
            bpm: 80,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
        SongTarget {
            song_id: "song-one-love-shining-1977",
            version: 1,
            bpm: 76,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "song-one-love-shining-1977",
            version: 2,
            bpm: 80,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
        SongTarget {
            song_id: "song_afrobeats_002",
            version: 1,
            bpm: 108,
            root_hz: 587.32,
            genre: "Afrobeats",
            expected_frames: 3994296,
        },
        SongTarget {
            song_id: "song_afrobeats_002",
            version: 2,
            bpm: 108,
            root_hz: 587.32,
            genre: "Afrobeats",
            expected_frames: 2560000,
        },
        SongTarget {
            song_id: "song_reggae_anthem_001",
            version: 1,
            bpm: 76,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "song_reggae_anthem_001",
            version: 2,
            bpm: 80,
            root_hz: 440.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
    ];

    for t in targets {
        println!("Regenerating melodic vocals & master for {} v{} ({} frames)...", t.song_id, t.version, t.expected_frames);
        let song_dir = PathBuf::from(format!("storage/songs/{}/v{}", t.song_id, t.version));
        let stems_dir = song_dir.join("stems");

        let lyrics_path = song_dir.join("lyrics.txt");
        let lyrics = fs::read_to_string(&lyrics_path).unwrap_or_else(|_| "One love one heart let us get together".to_string());

        let synth = MelodicSingingSynthesizer::new(48000);
        let duration_secs = t.expected_frames as f32 / 48000.0;
        let mut yue_buf = synth.synthesize_melodic_vocal_track(duration_secs, t.bpm as f32, t.root_hz, &lyrics);

        // Ensure exact frame count
        yue_buf.samples.truncate(t.expected_frames * 2);
        while yue_buf.frame_count() < t.expected_frames {
            yue_buf.samples.push(0.0);
            yue_buf.samples.push(0.0);
        }

        // Peak normalize yue_vocals to 0.85
        let peak_yue = yue_buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        if peak_yue > 0.01 {
            let scale = (0.85 / peak_yue).clamp(0.5, 3.0);
            for s in &mut yue_buf.samples {
                *s = (*s * scale).clamp(-0.95, 0.95);
            }
        }
        let yue_path = stems_dir.join("yue_vocals.wav");
        yue_buf.to_wav_file_with_format(&yue_path, WavBitDepth::Pcm16)?;

        // Render raspy_tenor_vocals from yue_vocals with subtle formant warmth & pitch lock
        let effective_rasp = 0.08f32;
        let formant_buf = apply_vocal_timbre(&yue_buf, effective_rasp, 0.0);
        let pitch_locked = PitchLockVocoder::align_and_pitch_lock(&formant_buf, t.bpm, t.root_hz);
        let mut raspy_buf = yue_buf.clone();
        for i in 0..raspy_buf.samples.len() {
            raspy_buf.samples[i] = (yue_buf.samples[i] * 0.85 + pitch_locked.samples[i] * 0.15).clamp(-0.95, 0.95);
        }
        let peak_raspy = raspy_buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        if peak_raspy > 0.01 {
            let scale = (0.88 / peak_raspy).clamp(0.5, 3.0);
            for s in &mut raspy_buf.samples {
                *s = (*s * scale).clamp(-0.95, 0.95);
            }
        }
        let raspy_path = stems_dir.join("raspy_tenor_vocals.wav");
        raspy_buf.to_wav_file_with_format(&raspy_path, WavBitDepth::Pcm16)?;

        // Mix premaster from stems scaled by 0.20
        let mut premaster = AudioBuffer::new_stereo(duration_secs);
        let stem_scale = 0.20f32;

        let guitar_stem = if stems_dir.join("inpainted_guitar.wav").exists() {
            stems_dir.join("inpainted_guitar.wav")
        } else {
            stems_dir.join("guitar.wav")
        };
        let drum_stem = if stems_dir.join("inpainted_drums.wav").exists() {
            stems_dir.join("inpainted_drums.wav")
        } else {
            stems_dir.join("drums.wav")
        };

        for stem_path in &[
            guitar_stem,
            drum_stem,
            stems_dir.join("bass.wav"),
            stems_dir.join("other.wav"),
            raspy_path,
        ] {
            if stem_path.exists() {
                if let Ok(buf) = AudioBuffer::from_wav_file(stem_path) {
                    mix_buffers(&mut premaster, &buf, stem_scale);
                }
            }
        }

        // Apply 12 kHz low-pass filter
        let mut lpf = ButterworthLowpass12kHz::new(premaster.channels as usize, premaster.sample_rate as f32);
        lpf.process_buffer(&mut premaster);

        // Apply analog tape mastering
        let profile = if t.genre.to_lowercase().contains("reggae") {
            MasteringProfile::StuderA800Tape1970s
        } else {
            MasteringProfile::ClubEdmSubHeavy
        };
        let mut mastered = apply_analog_mastering(&premaster, profile);

        // Smooth tanh saturation
        for s in &mut mastered.samples {
            *s = (*s * 1.2).tanh();
        }

        // Peak normalize to -1.0 dBFS (0.89125)
        let mut max_abs = 0.0f32;
        for &s in &mastered.samples {
            let a = s.abs();
            if a > max_abs { max_abs = a; }
        }
        if max_abs > 0.001 {
            let norm = 0.89125 / max_abs;
            for s in &mut mastered.samples {
                *s = (*s * norm).clamp(-0.95, 0.95);
            }
        }

        // Ensure exact frame count matches
        mastered.samples.truncate(t.expected_frames * 2);
        while mastered.frame_count() < t.expected_frames {
            mastered.samples.push(0.0);
            mastered.samples.push(0.0);
        }

        let master_path = song_dir.join("final_master.wav");
        mastered.to_wav_file_with_format(&master_path, WavBitDepth::Pcm16)?;

        // Update telemetry in manifest.json if present
        let manifest_path = song_dir.join("manifest.json");
        if manifest_path.exists() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let telem = music_dsp::telemetry_calc::calculate_telemetry(&mastered, t.bpm);
                    val["telemetry"] = serde_json::to_value(&telem)?;
                    val["updated_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
                    let json = serde_json::to_string_pretty(&val)?;
                    fs::write(&manifest_path, json)?;
                }
            }
        }

        println!("Successfully finished {} v{}!", t.song_id, t.version);
    }

    println!("All 8 song versions successfully regenerated with true melodic singing!");
    Ok(())
}
