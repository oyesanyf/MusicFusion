//! Complete Studio Multi-Stem & Master Regenerator (`regenerate_stems`).
//!
//! Regenerates ALL 10 stems + `final_master.wav` across all 8 song versions in `storage/songs/`
//! so every track is a complete, full-band studio song from 00:00 to the end:
//! - `draft_groove.wav`, `drums.wav`, `inpainted_drums.wav`: Full-length drums & percussion from 00:00
//! - `bass.wav`: Syncopated 4-bar I-IV-vi-V Dub Sub-Bass from 00:00
//! - `guitar.wav`, `inpainted_guitar.wav`: Karplus-Strong offbeat skank + plucked fills + layered wah solo from 00:00
//! - `other.wav`: Hammond B3 rotary organ bubble + 3-piece Brass Horn section from 00:00
//! - `yue_instrumentals.wav`: Complete instrumental accompaniment mix from 00:00
//! - `yue_vocals.wav`, `raspy_tenor_vocals.wav`: Lyric-driven LF-glottal & dynamic formant singing voice in natural human tenor register
//! - `final_master.wav`: Balanced 12 kHz anti-aliased Studer A800 analog tape master (-1.0 dBFS peak)

use std::fs;
use std::path::PathBuf;
use music_dsp::crossfader::splice_stem;
use music_dsp::groove_synth::{DubSubBass, HammondOrganBubble, UniversalDrumSynthesizer};
use music_dsp::guitar_wah::{synthesize_lead_solo, synthesize_reggae_skank_track};
use music_dsp::melodic_singing::MelodicSingingSynthesizer;
use music_dsp::tape_mastering::{apply_analog_mastering, ButterworthLowpass12kHz, MasteringProfile};
use music_dsp::vocal_formant::{apply_vocal_timbre, PitchLockVocoder};
use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_pipeline::nodes::node5_dsp_judge_gate::mix_buffers;

struct SongTarget {
    song_id: &'static str,
    version: u32,
    bpm: u32,
    root_hz: f32,
    genre: &'static str,
    expected_frames: usize,
}

fn normalize_stem(buf: &mut AudioBuffer, target_peak: f32) {
    let max_p = buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    if max_p > 0.001 {
        let gain = (target_peak / max_p).clamp(0.25, 4.0);
        for s in &mut buf.samples {
            *s = (*s * gain).clamp(-0.95, 0.95);
        }
    }
}

fn enforce_exact_frames(buf: &mut AudioBuffer, expected_frames: usize) {
    buf.samples.truncate(expected_frames * 2);
    while buf.frame_count() < expected_frames {
        buf.samples.push(0.0);
        buf.samples.push(0.0);
    }
}

fn main() -> anyhow::Result<()> {
    let targets = [
        SongTarget {
            song_id: "roots_reggae_anthem",
            version: 1,
            bpm: 76,
            root_hz: 220.0, // Natural Tenor A3
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "roots_reggae_anthem",
            version: 2,
            bpm: 80,
            root_hz: 220.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
        SongTarget {
            song_id: "song-one-love-shining-1977",
            version: 1,
            bpm: 76,
            root_hz: 220.0,
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "song-one-love-shining-1977",
            version: 2,
            bpm: 80,
            root_hz: 220.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
        SongTarget {
            song_id: "song_afrobeats_002",
            version: 1,
            bpm: 108,
            root_hz: 196.0, // Natural Tenor G3
            genre: "Afrobeats",
            expected_frames: 3994296,
        },
        SongTarget {
            song_id: "song_afrobeats_002",
            version: 2,
            bpm: 108,
            root_hz: 196.0,
            genre: "Afrobeats",
            expected_frames: 2560000,
        },
        SongTarget {
            song_id: "song_reggae_anthem_001",
            version: 1,
            bpm: 76,
            root_hz: 220.0,
            genre: "Roots Reggae",
            expected_frames: 3637895,
        },
        SongTarget {
            song_id: "song_reggae_anthem_001",
            version: 2,
            bpm: 80,
            root_hz: 220.0,
            genre: "Roots Reggae",
            expected_frames: 3544615,
        },
    ];

    for t in targets {
        println!(
            "Regenerating COMPLETE 10-stem studio arrangement & master for {} v{} ({} frames)...",
            t.song_id, t.version, t.expected_frames
        );
        let song_dir = PathBuf::from(format!("storage/songs/{}/v{}", t.song_id, t.version));
        let stems_dir = song_dir.join("stems");
        fs::create_dir_all(&stems_dir)?;

        let duration_secs = t.expected_frames as f32 / 48_000.0;

        // 1. Synthesize Full-Length Drum & Percussion Groove from 00:00 to end
        let mut draft_groove = UniversalDrumSynthesizer::generate_groove(t.genre, t.bpm, duration_secs);
        enforce_exact_frames(&mut draft_groove, t.expected_frames);
        normalize_stem(&mut draft_groove, 0.88);
        draft_groove.to_wav_file_with_format(&stems_dir.join("draft_groove.wav"), WavBitDepth::Pcm16)?;

        let drums_buf = draft_groove.clone();
        drums_buf.to_wav_file_with_format(&stems_dir.join("drums.wav"), WavBitDepth::Pcm16)?;

        // 2. Synthesize Full-Length Syncopated 4-Bar Dub Sub-Bass from 00:00 to end
        let mut bass_buf = DubSubBass::new(48_000).synthesize(duration_secs, t.bpm);
        enforce_exact_frames(&mut bass_buf, t.expected_frames);
        normalize_stem(&mut bass_buf, 0.86);
        bass_buf.to_wav_file_with_format(&stems_dir.join("bass.wav"), WavBitDepth::Pcm16)?;

        // 3. Synthesize Full-Length Rhythm Guitar Skank + Counter-Pick from 00:00 to end
        let mut guitar_buf = synthesize_reggae_skank_track(duration_secs, 48_000, t.bpm);
        enforce_exact_frames(&mut guitar_buf, t.expected_frames);
        normalize_stem(&mut guitar_buf, 0.82);
        guitar_buf.to_wav_file_with_format(&stems_dir.join("guitar.wav"), WavBitDepth::Pcm16)?;

        // 4. Synthesize Full-Length Hammond B3 Organ Bubble + Jamaican Brass Section from 00:00 to end
        let mut other_buf = HammondOrganBubble::new(48_000).synthesize(duration_secs, t.bpm, t.root_hz);
        enforce_exact_frames(&mut other_buf, t.expected_frames);
        normalize_stem(&mut other_buf, 0.82);
        other_buf.to_wav_file_with_format(&stems_dir.join("other.wav"), WavBitDepth::Pcm16)?;

        // 5. Synthesize Layered Lead Guitar Solo & Drum Fills for inpainted_guitar.wav & inpainted_drums.wav
        let total_bars = ((duration_secs * t.bpm as f32) / (4.0 * 60.0)).floor() as u32;
        let start_bar = (total_bars / 2).max(4);
        let solo_bars = (total_bars.saturating_sub(start_bar)).clamp(4, 12);

        let mut solo_lead = synthesize_lead_solo(solo_bars, t.bpm, "A Major");
        normalize_stem(&mut solo_lead, 0.82);
        let samples_per_bar = ((60.0 / t.bpm as f32) * 4.0 * 48_000.0).round() as usize;
        let solo_start_frame = (start_bar as usize) * samples_per_bar;
        for f in 0..solo_lead.frame_count() {
            let base_f = solo_start_frame + f;
            if base_f < guitar_buf.frame_count() {
                for c in 0..2 {
                    let rhythm_val = guitar_buf.samples[base_f * 2 + c];
                    let lead_val = solo_lead.samples[f * 2 + c];
                    solo_lead.samples[f * 2 + c] = (lead_val * 0.78 + rhythm_val * 0.68).clamp(-0.95, 0.95);
                }
            }
        }
        let mut inpainted_guitar = splice_stem(&guitar_buf, &solo_lead, start_bar, t.bpm, 4);
        enforce_exact_frames(&mut inpainted_guitar, t.expected_frames);
        normalize_stem(&mut inpainted_guitar, 0.84);
        inpainted_guitar.to_wav_file_with_format(&stems_dir.join("inpainted_guitar.wav"), WavBitDepth::Pcm16)?;

        let solo_dur = (solo_bars as f32 * 4.0 * 60.0) / t.bpm as f32;
        let mut drum_fills = UniversalDrumSynthesizer::generate_groove(t.genre, t.bpm, solo_dur);
        normalize_stem(&mut drum_fills, 0.88);
        let mut inpainted_drums = splice_stem(&drums_buf, &drum_fills, start_bar, t.bpm, 4);
        enforce_exact_frames(&mut inpainted_drums, t.expected_frames);
        normalize_stem(&mut inpainted_drums, 0.88);
        inpainted_drums.to_wav_file_with_format(&stems_dir.join("inpainted_drums.wav"), WavBitDepth::Pcm16)?;

        // 6. Synthesize Complete Instrumental Mix (yue_instrumentals.wav) from 00:00 to end
        let mut yue_inst = AudioBuffer::new_stereo(duration_secs);
        enforce_exact_frames(&mut yue_inst, t.expected_frames);
        for i in 0..yue_inst.samples.len() {
            let mixed = inpainted_drums.samples[i] * 0.34
                + bass_buf.samples[i] * 0.30
                + inpainted_guitar.samples[i] * 0.22
                + other_buf.samples[i] * 0.22;
            yue_inst.samples[i] = (mixed * 1.1).tanh() * 0.90;
        }
        yue_inst.to_wav_file_with_format(&stems_dir.join("yue_instrumentals.wav"), WavBitDepth::Pcm16)?;

        // 7. Synthesize Lyric-Driven Singing Vocal Stems (yue_vocals.wav & raspy_tenor_vocals.wav)
        let lyrics_path = song_dir.join("lyrics.txt");
        let lyrics = fs::read_to_string(&lyrics_path)
            .unwrap_or_else(|_| "One love one heart let us get together and feel alright".to_string());

        let synth = MelodicSingingSynthesizer::new(48_000);
        let mut yue_buf = synth.synthesize_melodic_vocal_track(duration_secs, t.bpm as f32, t.root_hz, &lyrics);
        enforce_exact_frames(&mut yue_buf, t.expected_frames);
        normalize_stem(&mut yue_buf, 0.84);
        yue_buf.to_wav_file_with_format(&stems_dir.join("yue_vocals.wav"), WavBitDepth::Pcm16)?;

        let formant_buf = apply_vocal_timbre(&yue_buf, 0.08, 0.0);
        let pitch_locked = PitchLockVocoder::align_and_pitch_lock(&formant_buf, t.bpm, t.root_hz);
        let mut raspy_buf = yue_buf.clone();
        for i in 0..raspy_buf.samples.len() {
            raspy_buf.samples[i] = (yue_buf.samples[i] * 0.85 + pitch_locked.samples[i] * 0.15).clamp(-0.95, 0.95);
        }
        normalize_stem(&mut raspy_buf, 0.85);
        raspy_buf.to_wav_file_with_format(&stems_dir.join("raspy_tenor_vocals.wav"), WavBitDepth::Pcm16)?;

        // 8. Mix Balanced Full-Band Premaster from 00:00 to end
        let mut premaster = AudioBuffer::new_stereo(duration_secs);
        enforce_exact_frames(&mut premaster, t.expected_frames);
        mix_buffers(&mut premaster, &inpainted_drums, 0.22);
        mix_buffers(&mut premaster, &bass_buf, 0.22);
        mix_buffers(&mut premaster, &inpainted_guitar, 0.18);
        mix_buffers(&mut premaster, &other_buf, 0.18);
        mix_buffers(&mut premaster, &raspy_buf, 0.20);

        // Apply 12 kHz 2nd-order Butterworth low-pass anti-aliasing filter
        let mut lpf = ButterworthLowpass12kHz::new(premaster.channels as usize, premaster.sample_rate as f32);
        lpf.process_buffer(&mut premaster);

        // Apply Studer A800 / Club Analog Mastering
        let profile = if t.genre.to_lowercase().contains("reggae") {
            MasteringProfile::StuderA800Tape1970s
        } else {
            MasteringProfile::ClubEdmSubHeavy
        };
        let mut mastered = apply_analog_mastering(&premaster, profile);

        for s in &mut mastered.samples {
            *s = (*s * 1.2).tanh();
        }

        // Peak normalize to -1.0 dBFS (0.89125)
        let mut max_abs = 0.0f32;
        for &s in &mastered.samples {
            let a = s.abs();
            if a > max_abs {
                max_abs = a;
            }
        }
        if max_abs > 0.001 {
            let norm = 0.89125 / max_abs;
            for s in &mut mastered.samples {
                *s = (*s * norm).clamp(-0.89125, 0.89125);
            }
        }

        enforce_exact_frames(&mut mastered, t.expected_frames);

        let master_path = song_dir.join("final_master.wav");
        mastered.to_wav_file_with_format(&master_path, WavBitDepth::Pcm16)?;

        // Update telemetry in manifest.json
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

        println!("Successfully regenerated all 10 stems + final_master.wav for {} v{}!", t.song_id, t.version);
    }

    println!("All 8 song versions (88 WAV files) regenerated with complete full-band arrangement & singing vocals!");
    Ok(())
}
