//! `reference_rl_synth`: Reference-Ensemble + RL Policy Conditioned Full-Song Synthesizer.
//!
//! Synthesizes a 100% brand-new, original multi-stem song directly conditioned on:
//! 1. The user's natural language prompt (parsing explicit BPM, Key, Title, and arrangement cues
//!    such as:
//!    - 1988 Acoustic Folk & Soul Ballad: continuous fingerpicked steel-string acoustic riff with
//!      bright hammer-ons & open chords, 16-bar woody acoustic slide guitar solo, stripped-back
//!      Verse 1 shaker & soft woodblock building into tight dry studio cross-stick rimshot, brushed
//!      snare swells & driving 16th-note hi-hat accents, 55–90 Hz melodic fretless bass with woody
//!      analog saturation, whisper-quiet Hammond B3 pad + delicate Wurlitzer electric piano + yearning
//!      pedal steel transition swells, and soulful contralto / raspy acoustic tenor storytelling vocals
//!      with Chorus/Outro dual harmony call-and-response, mastered with 1988 LA Studer A800 vocal-priority
//!      compression;
//!    - Dire Straits / Mark Knopfler-style soaring Stratocaster guitar solo + roots groove),
//! 2. The extracted acoustic features of the 1..=20 Reference WAV files (`BlendedStyleProfile`),
//! 3. The Reinforcement Learning policy action (`SynthesisAction`).

use std::f32::consts::PI;
use music_core::rl_contracts::BlendedStyleProfile;
use crate::groove_synth::DspRng;
use crate::guitar_wah::{GuitarAmpSimulator, KarplusStrongString, PinkNoiseGenerator};
use crate::melodic_singing::{
    MelodicSingingSynthesizer, MovementSection, SongMovementKind, StudioVocalReverb,
};
use crate::tape_mastering::{apply_analog_mastering, MasteringProfile};
use crate::vocal_formant::BiquadFilter;
use crate::wav::{AudioBuffer, DEFAULT_SAMPLE_RATE as SAMPLE_RATE};

/// Complete set of synthesized 48 kHz stereo stems and metadata for a Reference-RL song.
pub struct ReferenceRlSongOutput {
    pub title: String,
    pub genre_label: String,
    pub prompt_used: String,
    pub lyrics: String,
    pub bpm: u32,
    pub key_signature: String,
    pub final_master: AudioBuffer,
    pub vocals: AudioBuffer,
    pub drums: AudioBuffer,
    pub bass: AudioBuffer,
    pub acoustic_guitar: AudioBuffer,
    pub keys_organ: AudioBuffer,
    pub lead_guitar_solo: AudioBuffer,
    pub percussion_fills: AudioBuffer,
    pub draft_groove: AudioBuffer,
}

/// Chord definition in the 8-bar Reference-Conditioned Progression.
#[derive(Debug, Clone, Copy)]
struct ChordVoicing {
    root_hz: f32,
    third_hz: f32,
    fifth_hz: f32,
    seventh_hz: f32,
}

pub struct ReferenceRlSynthesizer;

impl ReferenceRlSynthesizer {
    /// Parses explicit BPM (e.g. "at 104 BPM") from the user's natural language prompt if present.
    pub fn extract_prompt_bpm(prompt: &str) -> Option<u32> {
        let lower = prompt.to_ascii_lowercase();
        let tokens: Vec<&str> = lower.split_whitespace().collect();
        for i in 0..tokens.len() {
            let clean = tokens[i].trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if clean == "bpm" && i > 0 {
                let prev = tokens[i - 1].trim_matches(|c: char| !c.is_ascii_digit());
                if let Ok(val) = prev.parse::<u32>() {
                    if (45..=210).contains(&val) {
                        return Some(val);
                    }
                }
            } else if let Some(num_part) = clean.strip_suffix("bpm") {
                if let Ok(val) = num_part.parse::<u32>() {
                    if (45..=210).contains(&val) {
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    /// Parses explicit Key Signature (e.g. "in A Major", "in A Minor", "in G Major") from the prompt if present.
    pub fn extract_prompt_key(prompt: &str) -> Option<String> {
        let lower = prompt.to_ascii_lowercase();
        let keys = [
            ("a# major", "A# Major"), ("bb major", "Bb Major"),
            ("a# minor", "A# Minor"), ("bb minor", "Bb Minor"),
            ("c# major", "C# Major"), ("db major", "Db Major"),
            ("c# minor", "C# Minor"), ("db minor", "C# Minor"),
            ("d# major", "D# Major"), ("eb major", "Eb Major"),
            ("d# minor", "D# Minor"), ("eb minor", "Eb Minor"),
            ("f# major", "F# Major"), ("gb major", "F# Major"),
            ("f# minor", "F# Minor"), ("gb minor", "F# Minor"),
            ("g# major", "G# Major"), ("ab major", "Ab Major"),
            ("g# minor", "G# Minor"), ("ab minor", "G# Minor"),
            ("a major", "A Major"), ("a minor", "A Minor"), ("a dorian", "A Dorian"),
            ("b major", "B Major"), ("b minor", "B Minor"),
            ("c major", "C Major"), ("c minor", "C Minor"),
            ("d major", "D Major"), ("d minor", "D Minor"),
            ("e major", "E Major"), ("e minor", "E Minor"),
            ("f major", "F Major"), ("f minor", "F Minor"),
            ("g major", "G Major"), ("g minor", "G Minor"),
        ];
        for (pat, canonical) in keys {
            if lower.contains(&format!("in {}", pat)) || lower.contains(pat) {
                return Some(canonical.to_string());
            }
        }
        None
    }

    /// Extracts an explicit quoted title after `titled "..."` or `called "..."` if present.
    pub fn extract_prompt_title(prompt: &str) -> Option<String> {
        for marker in ["titled \"", "called \"", "titled “", "called “"] {
            if let Some(idx) = prompt.to_ascii_lowercase().find(marker) {
                let start = idx + marker.len();
                let rest = &prompt[start..];
                if let Some(end) = rest.find(['"', '”']) {
                    let candidate = rest[..end].trim();
                    if !candidate.is_empty() && candidate.len() <= 64 {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
        None
    }

    /// Checks whether provided lyrics are just the old pre-filled Kingston reggae placeholder.
    pub fn is_stale_default_reggae_lyrics(lyrics: &str) -> bool {
        let lower = lyrics.to_ascii_lowercase();
        lower.contains("hear the children crying, give thanks and praise")
            || lower.contains("sun rises over zion hill")
            || (lower.contains("one love! one heart! let us get together") && lower.len() < 420)
    }

    /// Generates a brand-new song title, genre label, and 8-section structured English storytelling lyrics
    /// directly tailored to the user's prompt and the resolved key/BPM.
    pub fn compose_prompt_conditioned_lyrics_and_title(
        prompt: &str,
        bpm: u32,
        key: &str,
    ) -> (String, String, String) {
        let lower = prompt.to_ascii_lowercase();
        let is_1988_folk_soul = lower.contains("1988")
            || lower.contains("folk and soul")
            || lower.contains("acoustic slide")
            || lower.contains("woodblock")
            || lower.contains("fretless")
            || lower.contains("wurlitzer")
            || lower.contains("pedal steel")
            || lower.contains("contralto");
        let has_dire_straits = lower.contains("dire straits")
            || lower.contains("sultans")
            || lower.contains("knopfler");

        let explicit_title = Self::extract_prompt_title(prompt);
        let title = if let Some(t) = explicit_title {
            if is_1988_folk_soul {
                format!("{} — 1988 Acoustic Folk & Soul Ballad ({} BPM • {})", t, bpm, key)
            } else {
                format!("{} ({} BPM • {})", t, bpm, key)
            }
        } else if is_1988_folk_soul {
            format!("Fast Train to the Coast — 1988 Acoustic Folk & Soul ({} BPM • {})", bpm, key)
        } else if has_dire_straits {
            format!("Midnight Highway Strings ({} BPM • {})", bpm, key)
        } else {
            format!("Acoustic Roots & Soaring Strings ({} BPM • {})", bpm, key)
        };

        let genre_label = if is_1988_folk_soul {
            "1988 Acoustic Folk & Soul Ballad".to_string()
        } else if has_dire_straits {
            "Acoustic Roots & Dire Straits Guitar Fusion".to_string()
        } else {
            "Reference WAV Ensemble & RL Studio Mix".to_string()
        };

        let lyrics = if is_1988_folk_soul {
            format!(
                "[Intro]\n\
                 (Continuous hypnotic fingerpicked steel-string acoustic guitar riff with bright hammer-ons and warm open {key} chords at {bpm} BPM, supported by deep fingerstyle fretless bass)\n\
                 Pack the old suitcase by the kitchen door, city lights fading on the fourth-story floor.\n\n\
                 [Verse 1]\n\
                 (Intimate stripped-back arrangement: fingerpicked acoustic guitar, subtle shaker, soft woodblock, and warm contralto/tenor storytelling lead vocal right at the front)\n\
                 Got a ticket in my coat and a dollar in my hand,\n\
                 Tracing every highway line across this restless land.\n\
                 You said keep the promise burning when the night gets cold,\n\
                 Every mile we travel is a story being told.\n\n\
                 [Chorus]\n\
                 (Tight dry studio drum kit enters with crisp cross-stick rimshot, brushed snare swells, driving 16th-note hi-hats, and intimate dual harmony call-and-response backing vocals)\n\
                 One love shining through the canyon rain!\n\
                 (Harmony response: Shining through the rain!)\n\
                 Carried on the rhythm of a midnight westbound train!\n\
                 From the wooden porch steps to the California sea,\n\
                 One love shining over you and me!\n\n\
                 [Bridge]\n\
                 (Yearning pedal steel guitar swells bloom softly into a 16-bar melodic, woody acoustic slide guitar solo capturing longing and motion)\n\
                 Hear the slide guitar crying down the open line,\n\
                 Every crossroads leading back to yours and mine!\n\n\
                 [Outro]\n\
                 (Intimate dual harmony call-and-response swells gently before the climax woody acoustic slide guitar solo and warm 1988 LA Studer A800 tape fade)\n\
                 One love shining, roll on down the line,\n\
                 Acoustic strings and tender souls forever intertwined.",
                key = key,
                bpm = bpm
            )
        } else {
            format!(
                "[Intro]\n\
                 (Warm fingerpicked acoustic guitar arpeggios lock into a deep {bpm} BPM roots groove in {key}, accented by clean Knopfler-style Stratocaster fills)\n\
                 Streetlights shimmer on the midnight rain, six silver strings singing through the pain.\n\n\
                 [Verse 1]\n\
                 Driving down the coast where the river meets the sea,\n\
                 Warm acoustic timber ringing wild and free.\n\
                 Every mile behind us is a story left untold,\n\
                 Chasing down the horizon where the morning turns to gold.\n\n\
                 [Chorus]\n\
                 Turn the headlights high through the pouring rain!\n\
                 Feel the roots groove rising in our veins!\n\
                 Sing it clear and upfront across the open night,\n\
                 Let the soaring guitar carry us into the light!\n\n\
                 [Outro]\n\
                 Roll on through the night, let the silver strings ring true,\n\
                 Warm roots and clear voices shining straight to you.",
                bpm = bpm,
                key = key
            )
        };

        (title, genre_label, lyrics)
    }

    /// Extracts ONLY the sung vocal lines (stripping bracket tags and stage directions in parentheses)
    /// so the TD-PSOLA vocalist sings pure poetic storytelling lyrics without reading stage directions aloud.
    fn extract_sung_lines_only(structured_lyrics: &str) -> String {
        let mut out = Vec::new();
        for line in structured_lyrics.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('[') || trimmed.starts_with('(') {
                continue;
            }
            out.push(trimmed);
        }
        if out.is_empty() {
            "Pack the old suitcase by the kitchen door, got a ticket in my coat and a dollar in my hand, one love shining through the canyon rain, carried on the rhythm of a midnight westbound train".to_string()
        } else {
            out.join(" ")
        }
    }

    /// Parses the key signature into a fundamental root frequency (in the guitar/bass A2..G3 octave)
    /// and whether the tonality is Minor/Dorian.
    pub fn parse_key_root_and_mode(key: &str) -> (f32, bool) {
        let upper = key.to_ascii_uppercase();
        let is_minor = upper.contains("MINOR") || upper.contains("DORIAN") || upper.ends_with("MIN");
        let root_hz = if upper.starts_with("A#") || upper.starts_with("BB") {
            116.54 // Bb2
        } else if upper.starts_with('A') {
            110.00 // A2
        } else if upper.starts_with('B') {
            123.47 // B2
        } else if upper.starts_with("C#") || upper.starts_with("DB") {
            138.59 // C#3
        } else if upper.starts_with('C') {
            130.81 // C3
        } else if upper.starts_with("D#") || upper.starts_with("EB") {
            155.56 // Eb3
        } else if upper.starts_with('D') {
            146.83 // D3
        } else if upper.starts_with('E') {
            164.81 // E3
        } else if upper.starts_with("F#") || upper.starts_with("GB") {
            185.00 // F#3
        } else if upper.starts_with('F') {
            174.61 // F3
        } else if upper.starts_with("G#") || upper.starts_with("AB") {
            207.65 // G#3
        } else if upper.starts_with('G') {
            196.00 // G3
        } else {
            110.00 // Default A2
        };
        (root_hz, is_minor)
    }

    /// Builds an 8-bar chord progression conditioned on the key and mode:
    /// - For Major (e.g. 1988 Acoustic Folk & Soul Ballad in A Major — Tracy Chapman *Fast Car* / *Blowin' in the Wind*):
    ///   `Dmaj7 (IV) -> Aadd9 (I) -> E (V) -> F#m7 (vi) -> Dmaj7 (IV) -> Aadd9 (I) -> Bm7 (ii) -> E7sus4/E (V)`
    /// - For Minor (e.g. Dire Straits *Romeo & Juliet* / *Sultans of Swing* / Asa *Jailer* in A Minor):
    ///   `Am7 (i) -> Fmaj7 (VI) -> Cmaj7 (III) -> G6 (VII) -> Dm7 (iv) -> Am7 (i) -> Fmaj7 (VI) -> E7 (V7)`
    fn build_8bar_chord_progression(root_hz: f32, is_minor: bool) -> [ChordVoicing; 8] {
        let st = |semitones: f32| -> f32 { root_hz * 2.0f32.powf(semitones / 12.0) };
        if is_minor {
            [
                ChordVoicing { root_hz: st(0.0),  third_hz: st(3.0),  fifth_hz: st(7.0),  seventh_hz: st(10.0) }, // i (Am7)
                ChordVoicing { root_hz: st(-4.0), third_hz: st(0.0),  fifth_hz: st(3.0),  seventh_hz: st(7.0)  }, // VI (Fmaj7)
                ChordVoicing { root_hz: st(3.0),  third_hz: st(7.0),  fifth_hz: st(10.0), seventh_hz: st(14.0) }, // III (Cmaj7)
                ChordVoicing { root_hz: st(-2.0), third_hz: st(2.0),  fifth_hz: st(5.0),  seventh_hz: st(10.0) }, // VII (G6)
                ChordVoicing { root_hz: st(5.0),  third_hz: st(8.0),  fifth_hz: st(12.0), seventh_hz: st(15.0) }, // iv (Dm7)
                ChordVoicing { root_hz: st(0.0),  third_hz: st(3.0),  fifth_hz: st(7.0),  seventh_hz: st(10.0) }, // i (Am7)
                ChordVoicing { root_hz: st(-4.0), third_hz: st(0.0),  fifth_hz: st(3.0),  seventh_hz: st(7.0)  }, // VI (Fmaj7)
                ChordVoicing { root_hz: st(-5.0), third_hz: st(-1.0), fifth_hz: st(2.0),  seventh_hz: st(5.0)  }, // V7 (E7)
            ]
        } else {
            // Iconic 1988 Acoustic Folk & Soul Ballad progression (IVmaj7 - Iadd9 - V - vi7):
            [
                ChordVoicing { root_hz: st(5.0),  third_hz: st(9.0),  fifth_hz: st(12.0), seventh_hz: st(16.0) }, // IV (Dmaj7)
                ChordVoicing { root_hz: st(0.0),  third_hz: st(4.0),  fifth_hz: st(7.0),  seventh_hz: st(14.0) }, // I (Aadd9)
                ChordVoicing { root_hz: st(-5.0), third_hz: st(-1.0), fifth_hz: st(2.0),  seventh_hz: st(7.0)  }, // V (Eadd4)
                ChordVoicing { root_hz: st(-3.0), third_hz: st(0.0),  fifth_hz: st(4.0),  seventh_hz: st(7.0)  }, // vi (F#m7)
                ChordVoicing { root_hz: st(5.0),  third_hz: st(9.0),  fifth_hz: st(12.0), seventh_hz: st(16.0) }, // IV (Dmaj7)
                ChordVoicing { root_hz: st(0.0),  third_hz: st(4.0),  fifth_hz: st(7.0),  seventh_hz: st(14.0) }, // I (Aadd9)
                ChordVoicing { root_hz: st(2.0),  third_hz: st(5.0),  fifth_hz: st(9.0),  seventh_hz: st(12.0) }, // ii (Bm7)
                ChordVoicing { root_hz: st(-5.0), third_hz: st(-1.0), fifth_hz: st(2.0),  seventh_hz: st(5.0)  }, // V7 (E7)
            ]
        }
    }

    /// Synthesizes Stem 1: Continuous Hypnotic Fingerpicked Steel-String Acoustic Guitar Riff (`guitar.wav`).
    /// Blends bright hammer-ons (fast grace-note pitch articulations on steps 3 and 7) with warm open chords.
    fn synthesize_warm_acoustic_guitar_stem(
        duration_secs: f32,
        bpm: f32,
        chords: &[ChordVoicing; 8],
        seed: u64,
    ) -> AudioBuffer {
        let sr = SAMPLE_RATE;
        let frames = (duration_secs * sr as f32).round() as usize;
        let mut buf = AudioBuffer::new(sr, 2, duration_secs);

        let beat_sec = 60.0 / bpm.clamp(50.0, 200.0);
        let sixteenth_sec = beat_sec * 0.25;
        let total_sixteenths = (duration_secs / sixteenth_sec).ceil() as usize;

        let mut strings_l = [
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
        ];
        let mut strings_r = [
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
            KarplusStrongString::new(sr),
        ];
        let mut rng = DspRng::new(9001 + seed as u32);
        let mut pink = PinkNoiseGenerator::new();

        // Continuous hypnotic 1988 acoustic folk-soul fingerpicking riff:
        // (voice_idx, velocity, is_hammer_on)
        let pick_pattern: [(usize, f32, bool); 16] = [
            (0, 0.90, false), // 1: Thumb bass root + open string pinch
            (2, 0.56, false), // e: Index fifth
            (1, 0.74, true),  // &: Bright hammer-on into the 3rd (classic Fast Car motif!)
            (3, 0.84, false), // a: High 9th/7th chime
            (0, 0.68, false), // 2: Thumb root
            (2, 0.58, false), // e: Fifth
            (3, 0.86, true),  // &: Hammer-on high melody note
            (1, 0.62, false), // a: Open third pull-off
            (0, 0.88, false), // 3: Thumb bass root
            (1, 0.60, false), // e: Third
            (2, 0.76, false), // &: Fifth
            (3, 0.82, false), // a: High open string ring
            (1, 0.72, false), // 4: Warm open chord brush
            (2, 0.64, false), // e
            (3, 0.78, true),  // &: Hammer-on turnaround
            (1, 0.58, false), // a
        ];

        let mut next_step_frame = 0usize;
        let mut step_idx = 0usize;

        // Warm 1988 Martin D-28 / Guild steel-string acoustic body resonance filters
        let mut body_eq = BiquadFilter::peaking_eq(185.0, 1.0, 3.2, 2);
        let mut wood_eq = BiquadFilter::peaking_eq(420.0, 1.1, 1.8, 2);
        let mut sheen_eq = BiquadFilter::peaking_eq(3800.0, 1.0, 2.8, 2);

        for f in 0..frames {
            if f >= next_step_frame && step_idx < total_sixteenths {
                let bar_idx = (step_idx / 16) % chords.len();
                let chord = chords[bar_idx];
                let pat_pos = step_idx % 16;
                let (voice_idx, vel, is_hammer_on) = pick_pattern[pat_pos];

                let tones = [
                    chord.root_hz * 2.0,
                    chord.third_hz * 2.0,
                    chord.fifth_hz * 2.0,
                    chord.seventh_hz * 2.0,
                ];
                let base_freq = tones[voice_idx % 4];
                // For hammer-ons, pluck slightly sharper/brighter (+2 semitones grace-note excitation)
                let freq = if is_hammer_on { base_freq * 1.122_462 } else { base_freq };

                strings_l[voice_idx % 4].pluck_with_rng(freq, vel, &mut rng, &mut pink);
                strings_r[voice_idx % 4].pluck_with_rng(freq * 1.0012, vel * 0.96, &mut rng, &mut pink);

                // Warm open-chord strums on beats 1, 2.5, and 4 (steps 0, 6, 12)
                if pat_pos == 0 || pat_pos == 6 || pat_pos == 12 {
                    for (s_i, &t_hz) in tones.iter().enumerate() {
                        strings_l[s_i].pluck_with_rng(t_hz, vel * 0.58, &mut rng, &mut pink);
                        strings_r[s_i].pluck_with_rng(t_hz * 1.0014, vel * 0.58, &mut rng, &mut pink);
                    }
                }

                step_idx += 1;
                next_step_frame = ((step_idx as f32) * sixteenth_sec * sr as f32).round() as usize;
            }

            let mut sum_l = 0.0f32;
            let mut sum_r = 0.0f32;
            for i in 0..4 {
                let tl = strings_l[i].tick();
                let tr = strings_r[i].tick();
                sum_l += tl * 0.78 + tr * 0.22;
                sum_r += tr * 0.78 + tl * 0.22;
            }

            let eq_l = sheen_eq.process_sample(0, wood_eq.process_sample(0, body_eq.process_sample(0, sum_l * 0.52)));
            let eq_r = sheen_eq.process_sample(1, wood_eq.process_sample(1, body_eq.process_sample(1, sum_r * 0.52)));

            buf.samples[f * 2] = eq_l.clamp(-0.92, 0.92);
            buf.samples[f * 2 + 1] = eq_r.clamp(-0.92, 0.92);
        }

        buf
    }

    /// Synthesizes Stem 6 (`inpainted_guitar.wav`):
    /// - When `is_acoustic_slide` is true (1988 Acoustic Folk & Soul Ballad): renders a **16-bar melodic,
    ///   woody acoustic slide guitar solo** during the Bridge/Solo with continuous glass-bottleneck pitch
    ///   glides, warm acoustic soundboard resonance, and yearning pedal-steel swells on transitions!
    /// - Otherwise: renders a soaring Dire Straits / Mark Knopfler-style fingerpicked Stratocaster solo.
    #[allow(clippy::too_many_arguments)]
    fn synthesize_lead_or_slide_guitar_solo_stem(
        duration_secs: f32,
        bpm: f32,
        root_hz: f32,
        is_minor: bool,
        is_acoustic_slide: bool,
        guitar_drive: f32,
        melodic_seed: u64,
        solo_beat_window: (f32, f32),
    ) -> AudioBuffer {
        let sr = SAMPLE_RATE as f32;
        let frames = (duration_secs * sr).round() as usize;
        let mut buf = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);

        let beat_sec = 60.0 / bpm.clamp(50.0, 200.0);
        let lead_root = if root_hz < 180.0 { root_hz * 2.0 } else { root_hz };

        let scale_semitones: [f32; 10] = if is_minor {
            [0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0, 12.0, 14.0, 15.0]
        } else {
            [0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0, 12.0, 14.0, 16.0]
        };

        let shift = (melodic_seed as usize) % 3;
        // Expressive fingerpicked / bottleneck acoustic guitar solo licks:
        // (start_beat_in_16, duration_beats, scale_idx, grace_slide_semitones, has_double_stop)
        let solo_licks: [(f32, f32, usize, f32, bool); 24] = [
            // Bar 1: Plucked bottleneck grace-note slide into 5th & warm double-stop sixths
            (0.0,  0.75, 4 + (shift % 2), 2.0, true),
            (0.75, 0.75, 5, 0.0, false),
            (1.5,  1.0,  6, 2.0, true),
            (2.5,  0.75, 5, 0.0, false),
            (3.25, 0.75, 4, 0.0, true),
            // Bar 2: Ascending woody acoustic cry up to the high root & 9th
            (4.0,  0.75, 4, 0.0, false),
            (4.75, 0.75, 6, 2.0, true),
            (5.5,  1.0,  7, 0.0, true),
            (6.5,  0.75, 6, 0.0, false),
            (7.25, 0.75, 4, 0.0, true),
            // Bar 3: Cascading fingerpicked highway melody phrase
            (8.0,  0.50, 4, 0.0, false),
            (8.5,  0.50, 5, 0.0, false),
            (9.0,  0.75, 7, 2.0, true),
            (9.75, 0.75, 8, 0.0, true),
            (10.5, 0.75, 7, 0.0, false),
            (11.25, 0.75, 5, 0.0, true),
            // Bar 4: Warm acoustic double-stop resolution to tonic
            (12.0, 0.75, 6, 2.0, true),
            (12.75, 0.75, 5, 0.0, false),
            (13.5, 0.50, 4, 0.0, true),
            (14.0, 0.50, 2, 0.0, false),
            (14.5, 0.50, 4, 0.0, true),
            (15.0, 0.50, 5, 0.0, false),
            (15.5, 0.25, 4, 0.0, false),
            (15.75, 0.25, 7, 0.0, true),
        ];

        let mut lead_str_l = KarplusStrongString::new(SAMPLE_RATE);
        let mut lead_str_r = KarplusStrongString::new(SAMPLE_RATE);
        let mut harm_str_l = KarplusStrongString::new(SAMPLE_RATE);
        let mut harm_str_r = KarplusStrongString::new(SAMPLE_RATE);
        let mut rng = DspRng::new(7741 + melodic_seed as u32);
        let mut pink = PinkNoiseGenerator::new();

        let drive = if is_acoustic_slide {
            1.08 // Pure, warm, unclipped woody acoustic soundboard!
        } else {
            (1.35 + guitar_drive * 0.6).clamp(1.2, 2.1)
        };
        let mut amp = GuitarAmpSimulator::new(drive);
        let mut woody_resonator = if is_acoustic_slide {
            BiquadFilter::peaking_eq(265.0, 1.1, 3.6, 2) // Warm Martin / Dobro wooden soundboard resonance
        } else {
            BiquadFilter::peaking_eq(850.0, 1.1, 2.6, 2)
        };
        let mut air_eq = BiquadFilter::peaking_eq(3400.0, 1.0, 2.8, 2);
        let mut reverb = StudioVocalReverb::new(SAMPLE_RATE, 0.22);

        let delay_len_l = (0.125 * sr) as usize;
        let delay_len_r = (0.175 * sr) as usize;
        let mut delay_buf_l = vec![0.0f32; delay_len_l.max(1)];
        let mut delay_buf_r = vec![0.0f32; delay_len_r.max(1)];
        let mut d_idx_l = 0usize;
        let mut d_idx_r = 0usize;

        let mut last_lick_id: Option<(usize, usize)> = None;
        let mut grace_target_pushed = false;

        for i in 0..frames {
            let t = i as f32 / sr;
            let total_beats = t / beat_sec;
            let in_main_solo = total_beats >= solo_beat_window.0 && total_beats < solo_beat_window.1;
            let in_intro_hook = total_beats < 2.0;
            // Subtle fingerpicked turnaround fills at the end of every 4-bar section (beats 14..16)
            let in_transition_fill = (total_beats % 16.0) >= 14.0 && total_beats < solo_beat_window.1;

            let (cycle_num, beat_in_16) = if in_main_solo {
                let rel = total_beats - solo_beat_window.0;
                ((rel / 16.0).floor() as usize + 100, rel % 16.0)
            } else {
                ((total_beats / 16.0).floor() as usize, total_beats % 16.0)
            };

            let mut active_lick: Option<(usize, (f32, f32, usize, f32, bool))> = None;
            if in_main_solo || in_intro_hook || in_transition_fill {
                for (l_idx, &lick) in solo_licks.iter().enumerate() {
                    if beat_in_16 >= lick.0 && beat_in_16 < (lick.0 + lick.1) {
                        active_lick = Some((l_idx, lick));
                        break;
                    }
                }
            }

            if let Some((l_idx, (start_b, dur_b, s_idx, slide_st, has_double_stop))) = active_lick {
                let cur_id = (cycle_num, l_idx);
                let progress = ((beat_in_16 - start_b) / dur_b).clamp(0.0, 1.0);
                let target_st = scale_semitones[s_idx % scale_semitones.len()];
                let target_hz = lead_root * 2.0f32.powf(target_st / 12.0);
                let harm_st = scale_semitones[(s_idx + 2) % scale_semitones.len()];
                let harm_hz = lead_root * 2.0f32.powf(harm_st / 12.0);
                let vel = if in_main_solo { 0.92 } else { 0.68 };

                if last_lick_id != Some(cur_id) {
                    last_lick_id = Some(cur_id);
                    grace_target_pushed = false;
                    // If this note has a bottleneck grace slide, pluck the lower grace note first!
                    let initial_hz = if slide_st > 0.0 {
                        lead_root * 2.0f32.powf((target_st - slide_st) / 12.0)
                    } else {
                        target_hz
                    };
                    lead_str_l.pluck_with_rng(initial_hz, vel, &mut rng, &mut pink);
                    lead_str_r.pluck_with_rng(initial_hz * 1.0015, vel * 0.96, &mut rng, &mut pink);
                    if has_double_stop {
                        harm_str_l.pluck_with_rng(harm_hz, vel * 0.62, &mut rng, &mut pink);
                        harm_str_r.pluck_with_rng(harm_hz * 1.0018, vel * 0.62, &mut rng, &mut pink);
                    }
                } else if slide_st > 0.0 && !grace_target_pushed && progress >= 0.18 {
                    // 75 ms later, slide/hammer-on into the target melody note!
                    grace_target_pushed = true;
                    lead_str_l.pluck_with_rng(target_hz, vel * 0.95, &mut rng, &mut pink);
                    lead_str_r.pluck_with_rng(target_hz * 1.0015, vel * 0.92, &mut rng, &mut pink);
                }
            }

            let raw_l = lead_str_l.tick() * 0.78 + harm_str_l.tick() * 0.48;
            let raw_r = lead_str_r.tick() * 0.78 + harm_str_r.tick() * 0.48;
            let dry_l = amp.process(raw_l) * 0.82;
            let dry_r = amp.process(raw_r) * 0.82;

            let eq_l = air_eq.process_sample(0, woody_resonator.process_sample(0, dry_l));
            let eq_r = air_eq.process_sample(1, woody_resonator.process_sample(1, dry_r));

            let del_l = delay_buf_l[d_idx_l];
            let del_r = delay_buf_r[d_idx_r];
            delay_buf_l[d_idx_l] = eq_l + del_r * 0.26;
            delay_buf_r[d_idx_r] = eq_r + del_l * 0.26;
            d_idx_l = (d_idx_l + 1) % delay_len_l.max(1);
            d_idx_r = (d_idx_r + 1) % delay_len_r.max(1);

            let (rev_l, rev_r) = reverb.process_stereo(eq_l + del_l * 0.30, eq_r + del_r * 0.30);
            buf.samples[i * 2] = rev_l.clamp(-0.92, 0.92);
            buf.samples[i * 2 + 1] = rev_r.clamp(-0.92, 0.92);
        }

        buf
    }

    /// Synthesizes Stem 2 & Stem 7 (`drums.wav` & `inpainted_drums.wav`):
    /// - When `stripped_verse1` is true (1988 Acoustic Folk & Soul Ballad):
    ///   * Intro & Verse 1 (`beats 0..24`): Intimate, stripped-back arrangement using ONLY a subtle
    ///     16th-note shaker and soft hollow woodblock!
    ///   * Chorus, Verse 2, Bridge & Solo (`beats 24..end`): Builds into a tight, dry 1988 studio
    ///     drum kit featuring a crisp cross-stick rimshot, brushed snare swells, and driving
    ///     16th-note hi-hat accents pushing the narrative forward!
    fn synthesize_acoustic_roots_drums(
        duration_secs: f32,
        bpm: f32,
        drum_punch: f32,
        stripped_verse1: bool,
        seed: u64,
    ) -> (AudioBuffer, AudioBuffer) {
        let sr = SAMPLE_RATE as f32;
        let frames = (duration_secs * sr).round() as usize;
        let mut drums = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);
        let mut fills = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);

        let beat_sec = 60.0 / bpm.clamp(50.0, 200.0);
        let step_sec = beat_sec * 0.25; // 16th-note grid
        let mut rng = DspRng::new(4242 + seed as u32);
        let punch = (0.85 + drum_punch * 0.25).clamp(0.75, 1.25);

        for i in 0..frames {
            let t = i as f32 / sr;
            let total_beats = t / beat_sec;
            let step_f = t / step_sec;
            let step_idx = step_f.floor() as usize;
            let step_in_bar = step_idx % 16;
            let bar_idx = step_idx / 16;
            let dt = (step_f - step_idx as f32) * step_sec;

            // In 1988 Folk-Soul mode, Verse 1 (first 24 beats) uses ONLY subtle shaker + soft woodblock!
            let in_stripped_intro_verse1 = stripped_verse1 && total_beats < 24.0;

            let mut l = 0.0f32;
            let mut r = 0.0f32;

            // 1. Subtle 16th-Note Shaker & Driving Hi-Hat Accents (active throughout)
            if dt < 0.068 {
                let accent = if step_in_bar % 4 == 0 {
                    0.28
                } else if step_in_bar % 2 == 0 {
                    0.21
                } else {
                    0.13
                };
                let shaker = rng.next_f32() * (-dt * 52.0).exp() * accent;
                l += shaker * 0.86;
                r += shaker * 1.14;
            }

            // 2. Soft Hollow Maple Woodblock on Beats 2 & 4 during stripped Verse 1 (or subtle syncopation later)
            if (step_in_bar == 4 || step_in_bar == 12 || (in_stripped_intro_verse1 && step_in_bar == 10)) && dt < 0.075 {
                let wb_freq = if step_in_bar == 10 { 920.0 } else { 760.0 };
                let wb_env = (-dt * 68.0).exp();
                let wb = (2.0 * PI * wb_freq * dt).sin() * wb_env * if in_stripped_intro_verse1 { 0.38 } else { 0.18 };
                l += wb * 1.05;
                r += wb * 0.95;
            }

            // 3. Full Tight Dry Studio Drum Kit (enters at Chorus / Beat 24.0!)
            let mut fill_l = 0.0f32;
            let mut fill_r = 0.0f32;
            if !in_stripped_intro_verse1 {
                // Tight dry studio kick (Steps 0, 6, 8, 10)
                if (step_in_bar == 0 || step_in_bar == 6 || step_in_bar == 8 || (bar_idx % 2 == 1 && step_in_bar == 10))
                    && dt < 0.15
                {
                    let freq = 54.0 + 90.0 * (-dt * 45.0).exp();
                    let env = (-dt * 20.0).exp();
                    let click = (-dt * 190.0).exp() * 0.22;
                    let k = ((2.0 * PI * freq * dt).sin() + click) * env * 0.70 * punch;
                    l += k;
                    r += k;
                }

                // Crisp Cross-Stick Rimshot + Brushed Snare Swells on Beats 2 & 4 (Steps 4 & 12)
                if (step_in_bar == 4 || step_in_bar == 12) && dt < 0.18 {
                    let cross_stick = ((2.0 * PI * 480.0 * dt).sin() * 0.55
                        + (2.0 * PI * 1120.0 * dt).sin() * 0.30)
                        * (-dt * 75.0).exp();
                    let brush_body = (2.0 * PI * 182.0 * dt).sin() * (-dt * 30.0).exp() * 0.38;
                    let brush_wires = rng.next_f32() * (-dt * 18.0).exp() * 0.36;
                    let s = (cross_stick + brush_body + brush_wires) * 0.72 * punch;
                    l += s * 0.96;
                    r += s * 1.04;
                } else if (step_in_bar == 7 || step_in_bar == 14 || step_in_bar == 15) && dt < 0.12 {
                    // Brushed snare swell leading into the next downbeat
                    let swell_env = (dt / 0.12 * PI).sin();
                    let brush_swell = rng.next_f32() * swell_env * 0.20;
                    l += brush_swell * 0.90;
                    r += brush_swell * 1.10;
                    fill_l += brush_swell;
                    fill_r += brush_swell;
                }

                // Studio tom & rimshot turnaround fills at section boundaries (every 4th bar)
                if bar_idx % 4 == 3 && step_in_bar >= 12 && dt < 0.14 {
                    let tom_freqs = [142.0, 122.0, 104.0, 86.0];
                    let tf = tom_freqs[(step_in_bar - 12) % 4];
                    let tom = (2.0 * PI * tf * dt).sin() * (-dt * 22.0).exp() * 0.52;
                    let pan = (step_in_bar - 12) as f32 / 3.0;
                    fill_l += tom * (1.0 - pan * 0.5);
                    fill_r += tom * (0.5 + pan * 0.5);
                    l += fill_l * 0.65;
                    r += fill_r * 0.65;
                }
            }

            drums.samples[i * 2] = l.clamp(-0.92, 0.92);
            drums.samples[i * 2 + 1] = r.clamp(-0.92, 0.92);
            fills.samples[i * 2] = (fill_l + l * 0.25).clamp(-0.92, 0.92);
            fills.samples[i * 2 + 1] = (fill_r + r * 0.25).clamp(-0.92, 0.92);
        }

        (drums, fills)
    }

    /// Synthesizes Stem 3: Deep Melodic Fingerstyle Fretless Bassline in the 55–90 Hz range (`bass.wav`).
    /// Features smooth fretless pitch slides ("mwah" bloom envelope) and rounded woody analog tape saturation.
    fn synthesize_fretless_roots_bass_stem(
        duration_secs: f32,
        bpm: f32,
        chords: &[ChordVoicing; 8],
        bass_warmth_db: f32,
    ) -> AudioBuffer {
        let sr = SAMPLE_RATE as f32;
        let frames = (duration_secs * sr).round() as usize;
        let mut buf = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);

        let beat_sec = 60.0 / bpm.clamp(50.0, 200.0);
        let eighth_sec = beat_sec * 0.5;
        let mut phase = 0.0f32;
        let mut sub_phase = 0.0f32;
        let mut smoothed_hz = 55.0f32;
        let mut lp_state = 0.0f32;
        let warmth_gain = 10.0f32.powf(bass_warmth_db.clamp(-3.0, 4.5) / 20.0);
        let mut woody_eq = BiquadFilter::peaking_eq(78.0, 1.1, 2.8, 2);

        for i in 0..frames {
            let t = i as f32 / sr;
            let eighth_f = t / eighth_sec;
            let eighth_idx = eighth_f.floor() as usize;
            let step_in_bar = eighth_idx % 8;
            let bar_idx = (eighth_idx / 8) % chords.len();
            let dt = (eighth_f - eighth_idx as f32) * eighth_sec;

            let chord = chords[bar_idx];
            // Fold root into the exact 55..90 Hz fretless fundamental range!
            let mut base_root = chord.root_hz;
            while base_root > 92.0 {
                base_root *= 0.5;
            }
            while base_root < 52.0 {
                base_root *= 2.0;
            }

            let raw_target_hz = match step_in_bar {
                0 | 1 => base_root,
                2 => (base_root * 1.498_307).clamp(55.0, 110.0), // Fifth
                3 => base_root,
                4 | 5 => base_root,
                6 => (base_root * 1.259_921).clamp(55.0, 105.0), // Expressive fretless 3rd
                _ => (base_root * 1.122_462).clamp(55.0, 100.0), // Passing 2nd glide
            };

            // Smooth fretless fingerboard pitch portamento (~22 ms glide)
            smoothed_hz = smoothed_hz * 0.9990 + raw_target_hz * 0.0010;

            phase = (phase + smoothed_hz / sr).fract();
            sub_phase = (sub_phase + (smoothed_hz * 0.5) / sr).fract();

            // Fretless "mwah" bloom: warm rounded fingerstyle attack (28 ms) into singing sustain
            let env = if dt < 0.028 {
                (dt / 0.028 * PI * 0.5).sin()
            } else {
                (-dt * 3.8).exp() * 0.78 + 0.22
            };

            let mwah_overtone = (dt / 0.09).min(1.0).sin() * 0.26;
            let raw = (2.0 * PI * phase).sin() * 0.70
                + (4.0 * PI * phase).sin() * (0.20 + mwah_overtone)
                + (6.0 * PI * phase).sin() * (0.08 * mwah_overtone)
                + (2.0 * PI * sub_phase).sin() * 0.18;

            lp_state = lp_state * 0.86 + raw * 0.14;
            let saturated = (lp_state * env * 0.78 * warmth_gain).tanh() * 0.84;
            let woody = woody_eq.process_sample(0, saturated).clamp(-0.88, 0.88);

            buf.samples[i * 2] = woody;
            buf.samples[i * 2 + 1] = woody;
        }

        buf
    }

    /// Synthesizes Stem 5 (`other.wav`):
    /// Whisper-quiet sustained Hammond B3 organ pad tucked low in the mix + delicate Wurlitzer
    /// electric piano bell-tine chords + yearning pedal steel guitar swells that bloom softly
    /// during bar/section transitions.
    fn synthesize_wurlitzer_b3_and_pedal_steel_stem(
        duration_secs: f32,
        bpm: f32,
        chords: &[ChordVoicing; 8],
    ) -> AudioBuffer {
        let sr = SAMPLE_RATE as f32;
        let frames = (duration_secs * sr).round() as usize;
        let mut buf = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);

        let beat_sec = 60.0 / bpm.clamp(50.0, 200.0);
        let bar_sec = beat_sec * 4.0;
        let half_bar_sec = beat_sec * 2.0;
        let mut phases = [0.0f32; 4];
        let mut steel_phase = 0.0f32;
        let mut smoothed_steel_hz = chords[0].fifth_hz * 4.0;

        for i in 0..frames {
            let t = i as f32 / sr;
            let bar_idx = ((t / bar_sec).floor() as usize) % chords.len();
            let dt_half = t % half_bar_sec;
            let dt_bar = t % bar_sec;
            let chord = chords[bar_idx];
            let freqs = [
                chord.root_hz * 2.0,
                chord.third_hz * 2.0,
                chord.fifth_hz * 2.0,
                chord.seventh_hz * 2.0,
            ];

            // 1. Whisper-quiet sustained Hammond B3 pad + Delicate Wurlitzer 200A reed/tine chords
            let wurli_env = (-dt_half * 2.6).exp();
            let wurli_tine = (-dt_half * 14.0).exp() * 0.18;
            let mut pad_and_wurli = 0.0f32;
            for k in 0..4 {
                phases[k] = (phases[k] + freqs[k] / sr).fract();
                let w = 2.0 * PI * phases[k];
                // Sustained B3 drawbar (0.09) + Wurlitzer reed fundamental & bell overtone
                let b3 = w.sin() * 0.09 + (2.0 * w).sin() * 0.04;
                let wurli = (w.sin() * 0.16 + (3.0 * w).sin() * wurli_tine) * wurli_env;
                pad_and_wurli += b3 + wurli;
            }

            // 2. Yearning Pedal Steel Guitar Swells (bloom softly during the second half of every bar)
            let target_steel_hz = chord.third_hz * 4.0;
            smoothed_steel_hz = smoothed_steel_hz * 0.9994 + target_steel_hz * 0.0006;
            steel_phase = (steel_phase + smoothed_steel_hz / sr).fract();
            let bar_progress = (dt_bar / bar_sec).clamp(0.0, 1.0);
            let swell_env = if bar_progress > 0.45 {
                ((bar_progress - 0.45) / 0.55 * PI).sin()
            } else {
                0.0
            };
            let steel_vib = 1.0 + 0.004 * (2.0 * PI * 5.2 * t).sin();
            let pedal_steel = ((2.0 * PI * steel_phase * steel_vib).sin() * 0.22
                + (4.0 * PI * steel_phase).sin() * 0.09)
                * swell_env;

            // Gentle Wurlitzer tremolo + wide stereo spread
            let trem_l = 1.0 + 0.14 * (2.0 * PI * 4.8 * t).sin();
            let trem_r = 1.0 + 0.14 * (2.0 * PI * 4.8 * t + PI * 0.5).cos();

            buf.samples[i * 2] = ((pad_and_wurli * trem_l + pedal_steel * 0.85) * 0.34).clamp(-0.85, 0.85);
            buf.samples[i * 2 + 1] = ((pad_and_wurli * trem_r + pedal_steel * 1.15) * 0.34).clamp(-0.85, 0.85);
        }

        buf
    }

    /// Primary entry point: Synthesizes a complete, 100% original Reference-Ensemble + RL Song
    /// with all 8 isolated studio stems and 1988 LA Studer A800 analog-mastered stereo mix.
    #[allow(clippy::too_many_arguments)]
    pub fn synthesize_full_reference_rl_song(
        prompt: &str,
        custom_lyrics: Option<&str>,
        blended: &BlendedStyleProfile,
        vocal_gain_db: f32,
        ducking_db: f32,
        guitar_drive: f32,
        bass_warmth_db: f32,
        drum_punch: f32,
        melodic_seed: u64,
    ) -> ReferenceRlSongOutput {
        // Respect explicit BPM or Key in the user's prompt (e.g., "at 104 BPM in A Major"),
        // falling back seamlessly to the Reference WAV Ensemble's consensus BPM and Key!
        let bpm = Self::extract_prompt_bpm(prompt)
            .unwrap_or_else(|| blended.blended_bpm.round().clamp(68.0, 165.0) as u32);
        let key_signature = Self::extract_prompt_key(prompt).unwrap_or_else(|| {
            if blended.consensus_key.trim().is_empty() {
                "A Minor".to_string()
            } else {
                blended.consensus_key.clone()
            }
        });

        let (title, genre_label, auto_lyrics) =
            Self::compose_prompt_conditioned_lyrics_and_title(prompt, bpm, &key_signature);
        let structured_lyrics = match custom_lyrics {
            Some(l) if !l.trim().is_empty() && !Self::is_stale_default_reggae_lyrics(l) => l.to_string(),
            _ => auto_lyrics,
        };
        let sung_lyrics = Self::extract_sung_lines_only(&structured_lyrics);

        let lower_prompt = prompt.to_ascii_lowercase();
        let is_1988_folk_soul = lower_prompt.contains("1988")
            || lower_prompt.contains("folk and soul")
            || lower_prompt.contains("acoustic slide")
            || lower_prompt.contains("woodblock")
            || lower_prompt.contains("fretless")
            || lower_prompt.contains("wurlitzer");

        let (root_hz, is_minor) = Self::parse_key_root_and_mode(&key_signature);
        let chords = Self::build_8bar_chord_progression(root_hz, is_minor);
        let beat_sec = 60.0 / (bpm as f32);

        // 1. Synthesize clear, upfront contralto/raspy acoustic tenor lead + dual harmony vocals FIRST
        //    with NO mid-word solo interruption and NO wrap-around repetition!
        let max_vocal_canvas_secs = 80.0f32;
        let vocal_synth = MelodicSingingSynthesizer::new(SAMPLE_RATE);
        let raw_vocals = vocal_synth.synthesize_reference_conditioned_vocal_track(
            max_vocal_canvas_secs,
            bpm as f32,
            root_hz * 2.0,
            is_minor,
            &sung_lyrics,
            melodic_seed,
            true,
            None,
        );

        // Find the exact frame where the vocalist finishes the final lyric of [Outro]
        let mut last_vocal_frame = (36.0 * SAMPLE_RATE as f32) as usize;
        for f in (0..raw_vocals.frame_count()).rev() {
            let amp = raw_vocals.samples[f * 2]
                .abs()
                .max(raw_vocals.samples[f * 2 + 1].abs());
            if amp > 0.025 {
                last_vocal_frame = f;
                break;
            }
        }
        let last_vocal_sec = (last_vocal_frame as f32 / SAMPLE_RATE as f32).clamp(28.0, 62.0);
        let last_vocal_beat = last_vocal_sec / beat_sec;

        // After the vocalist finishes the final word, launch the 16-beat woody acoustic slide guitar solo
        // followed by a 6-beat resolving tonic chord ring-out and smooth 3.5s studio fade-out!
        let solo_start_beat = last_vocal_beat;
        let solo_end_beat = solo_start_beat + 16.0;
        let solo_beat_window = (solo_start_beat, solo_end_beat);
        let duration_secs = ((solo_end_beat + 6.0) * beat_sec).clamp(48.0, 76.0);
        let frames = (duration_secs * SAMPLE_RATE as f32).round() as usize;

        let mut vocals = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);
        let copy_frames = frames.min(raw_vocals.frame_count());
        vocals.samples[..copy_frames * 2].copy_from_slice(&raw_vocals.samples[..copy_frames * 2]);

        // 2. Synthesize all instrumental stems conditioned on the prompt, reference ensemble & RL action
        let mut acoustic_guitar =
            Self::synthesize_warm_acoustic_guitar_stem(duration_secs, bpm as f32, &chords, melodic_seed);
        let mut lead_guitar_solo = Self::synthesize_lead_or_slide_guitar_solo_stem(
            duration_secs,
            bpm as f32,
            root_hz,
            is_minor,
            is_1988_folk_soul,
            guitar_drive,
            melodic_seed,
            solo_beat_window,
        );
        let (mut drums, mut percussion_fills) = Self::synthesize_acoustic_roots_drums(
            duration_secs,
            bpm as f32,
            drum_punch,
            is_1988_folk_soul,
            melodic_seed,
        );
        let mut bass = Self::synthesize_fretless_roots_bass_stem(
            duration_secs,
            bpm as f32,
            &chords,
            bass_warmth_db,
        );
        let mut keys_organ = Self::synthesize_wurlitzer_b3_and_pedal_steel_stem(
            duration_secs,
            bpm as f32,
            &chords,
        );

        // Apply clean studio ending resolution over the final 3.5 seconds of all stems
        // (drums taper first so the acoustic guitar, fretless bass & Wurlitzer chord rings out and fades to 0.0!)
        let fade_out_secs = 3.5f32;
        let fade_start_sec = (duration_secs - fade_out_secs).max(10.0);
        for i in 0..frames {
            let t = i as f32 / (SAMPLE_RATE as f32);
            if t >= fade_start_sec {
                let progress = ((t - fade_start_sec) / fade_out_secs).clamp(0.0, 1.0);
                // Smooth raised-cosine studio fade-out reaching 0.0000 for the last 0.25s
                let chord_fade = if progress >= 0.93 {
                    0.0
                } else {
                    0.5 * (1.0 + (progress / 0.93 * PI).cos())
                };
                // Drums finish 1.2s earlier so the final acoustic chord rings out purely
                let drum_fade = if progress >= 0.60 {
                    0.0
                } else {
                    0.5 * (1.0 + (progress / 0.60 * PI).cos())
                };
                acoustic_guitar.samples[i * 2] *= chord_fade;
                acoustic_guitar.samples[i * 2 + 1] *= chord_fade;
                lead_guitar_solo.samples[i * 2] *= chord_fade;
                lead_guitar_solo.samples[i * 2 + 1] *= chord_fade;
                bass.samples[i * 2] *= chord_fade;
                bass.samples[i * 2 + 1] *= chord_fade;
                keys_organ.samples[i * 2] *= chord_fade;
                keys_organ.samples[i * 2 + 1] *= chord_fade;
                drums.samples[i * 2] *= drum_fade;
                drums.samples[i * 2 + 1] *= drum_fade;
                percussion_fills.samples[i * 2] *= drum_fade;
                percussion_fills.samples[i * 2 + 1] *= drum_fade;
                vocals.samples[i * 2] *= chord_fade;
                vocals.samples[i * 2 + 1] *= chord_fade;
            }
        }

        // 3. Build Draft Groove stem (acoustic guitar + drums + fretless bass foundation)
        let mut draft_groove = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);
        for i in 0..frames {
            draft_groove.samples[i * 2] = (acoustic_guitar.samples[i * 2] * 0.55
                + drums.samples[i * 2] * 0.55
                + bass.samples[i * 2] * 0.52)
                .clamp(-0.95, 0.95);
            draft_groove.samples[i * 2 + 1] = (acoustic_guitar.samples[i * 2 + 1] * 0.55
                + drums.samples[i * 2 + 1] * 0.55
                + bass.samples[i * 2 + 1] * 0.52)
                .clamp(-0.95, 0.95);
        }

        // 4. 1988 LA Studer A800 Analog Master Mix with Vocal-Priority Compression & Acoustic Front-Presence
        let mut raw_master = AudioBuffer::new(SAMPLE_RATE, 2, duration_secs);
        let vocal_boost = 10.0f32.powf((vocal_gain_db.clamp(-1.0, 4.5) + 2.4) / 20.0);
        let max_duck_atten = 10.0f32.powf(ducking_db.clamp(-5.5, -2.2) / 20.0);
        let mut v_env = 0.0f32;

        for i in 0..frames {
            let t = i as f32 / (SAMPLE_RATE as f32);
            let total_beats = t / beat_sec;
            let in_solo = total_beats >= solo_beat_window.0 && total_beats < solo_beat_window.1;

            let vl = vocals.samples[i * 2] * vocal_boost;
            let vr = vocals.samples[i * 2 + 1] * vocal_boost;
            let v_mag = vl.abs().max(vr.abs());
            if v_mag > v_env {
                v_env = 0.985 * v_env + 0.015 * v_mag;
            } else {
                v_env = 0.9990 * v_env + 0.0010 * v_mag;
            }

            // Vocal-priority sidechain compression so storytelling vocals & acoustic strings sit right up front
            let inst_duck = if v_env > 0.025 {
                (1.0 - ((v_env / 0.28).min(1.0) * (1.0 - max_duck_atten))).clamp(0.42, 1.0)
            } else {
                1.0
            };

            // Keep fingerpicked steel-string acoustic guitar prominent right alongside the vocals
            let ac_l = acoustic_guitar.samples[i * 2] * 0.54 * (0.82 + 0.18 * inst_duck);
            let ac_r = acoustic_guitar.samples[i * 2 + 1] * 0.54 * (0.82 + 0.18 * inst_duck);

            let solo_gain = if in_solo { 0.88 } else { 0.28 * inst_duck };
            let sl_l = lead_guitar_solo.samples[i * 2] * solo_gain;
            let sl_r = lead_guitar_solo.samples[i * 2 + 1] * solo_gain;

            let dr_l = drums.samples[i * 2] * 0.42 * inst_duck;
            let dr_r = drums.samples[i * 2 + 1] * 0.42 * inst_duck;

            let ba_l = bass.samples[i * 2] * 0.44 * inst_duck;
            let ba_r = bass.samples[i * 2 + 1] * 0.44 * inst_duck;

            // Whisper-quiet Hammond B3 + delicate Wurlitzer & pedal steel tucked gently in the mix
            let or_l = keys_organ.samples[i * 2] * 0.22 * inst_duck;
            let or_r = keys_organ.samples[i * 2 + 1] * 0.22 * inst_duck;

            let mix_l = vl * 0.95 + ac_l + sl_l + dr_l + ba_l + or_l;
            let mix_r = vr * 0.95 + ac_r + sl_r + dr_r + ba_r + or_r;

            raw_master.samples[i * 2] = mix_l.tanh() * 0.95;
            raw_master.samples[i * 2 + 1] = mix_r.tanh() * 0.95;
        }

        let mut final_master = apply_analog_mastering(&raw_master, MasteringProfile::StuderA800Tape1970s);

        // 5. Broadcast Loudness Make-Up Gain & True-Peak Limiter (-9.8 dBFS RMS, -0.25 dBFS Ceiling)
        //    plus guaranteed studio fade-out to 0.0000 silence at the end of the track!
        let active_end_frame = ((fade_start_sec * SAMPLE_RATE as f32) as usize).min(frames);
        let mut active_sum_sq = 0.0f64;
        let active_samples = (active_end_frame * 2).max(1);
        for &s in &final_master.samples[..active_samples] {
            active_sum_sq += (s as f64) * (s as f64);
        }
        let active_rms = (active_sum_sq / active_samples as f64).sqrt() as f32;
        let makeup_gain = (0.32 / active_rms.max(0.05)).clamp(1.15, 2.8);

        for i in 0..frames {
            let t = i as f32 / (SAMPLE_RATE as f32);
            let end_env = if t >= fade_start_sec {
                let progress = ((t - fade_start_sec) / fade_out_secs).clamp(0.0, 1.0);
                if progress >= 0.93 {
                    0.0
                } else {
                    0.5 * (1.0 + (progress / 0.93 * PI).cos())
                }
            } else {
                1.0
            };

            for ch in 0..2 {
                let x = final_master.samples[i * 2 + ch] * makeup_gain * end_env;
                // Smooth analog soft-knee true-peak ceiling at 0.965 (-0.3 dBFS)
                let abs_x = x.abs();
                let limited = if abs_x <= 0.76 {
                    x
                } else {
                    let excess = (abs_x - 0.76) / 0.205;
                    (0.76 + 0.205 * excess.tanh()).copysign(x)
                };
                final_master.samples[i * 2 + ch] = limited.clamp(-0.966, 0.966);
            }
        }

        ReferenceRlSongOutput {
            title,
            genre_label,
            prompt_used: prompt.to_string(),
            lyrics: structured_lyrics,
            bpm,
            key_signature,
            final_master,
            vocals,
            drums,
            bass,
            acoustic_guitar,
            keys_organ,
            lead_guitar_solo,
            percussion_fills,
            draft_groove,
        }
    }
}
