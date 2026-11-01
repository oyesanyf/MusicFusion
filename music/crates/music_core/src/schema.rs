use serde::{Deserialize, Serialize};

/// The 6-stage sequential Hugging Face + Ollama ModelFusion stack for hit song production.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineModelStack {
    /// Stage 0: Multi-model Ollama songwriting, music theory, and arrangement panel
    pub stage0_ollama_panel: Vec<String>,
    pub stage0_judge_model: String,
    pub stage0_writer_model: String,
    /// Stage 1: Rapid diffusion groove & song structure foundation
    pub stage1_groove_foundation: String,
    /// Stage 2: Full-song dual-stem synthesis with 30s In-Context Learning (ICL)
    pub stage2_yue_semantic: String,
    pub stage2_yue_acoustic: String,
    /// Stage 3: Zero-shot soulful raspy 1970s Jamaican tenor voice conversion
    pub stage3_voice_conversion: String,
    /// Stage 4: 4-stem separation & virtuosic guitar solo / polyrhythmic drum regeneration
    pub stage4_stem_separator: String,
    pub stage4_solo_regenerator: String,
    pub stage4_style_transfer: String,
    /// Stage 5: 48 kHz super-resolution & 1977 analog tape reference mastering
    pub stage5_super_resolution: String,
    pub stage5_reference_mastering: String,
}

impl Default for PipelineModelStack {
    fn default() -> Self {
        Self {
            stage0_ollama_panel: vec![
                "flint:latest".to_string(),
                "glm4:9b".to_string(),
                "qwen2.5:7b".to_string(),
                "mistral:latest".to_string(),
            ],
            stage0_judge_model: "qwen2.5:7b".to_string(),
            stage0_writer_model: "qwen2.5:7b".to_string(),
            stage1_groove_foundation: "ACE-Step/ACE-Step-v1.5".to_string(),
            stage2_yue_semantic: "m-a-p/YuE-s1-7B-anneal-en-icl".to_string(),
            stage2_yue_acoustic: "m-a-p/YuE-s2-1B-general".to_string(),
            stage3_voice_conversion: "Plachtaa/Seed-VC".to_string(),
            stage4_stem_separator: "facebook/htdemucs_ft".to_string(),
            stage4_solo_regenerator: "stabilityai/stable-audio-open-1.0".to_string(),
            stage4_style_transfer: "facebook/musicgen-style".to_string(),
            stage5_super_resolution: "haoheliu/versatile_audio_super_resolution".to_string(),
            stage5_reference_mastering: "Matchering-2.0 (1977 Analog Tape Reference)".to_string(),
        }
    }
}

/// Mixing and mastering parameters for all isolated stems and Stage 5 analog mastering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemMixConfig {
    pub lead_vocal_db: f32,
    pub gospel_choir_db: f32,
    pub drums_db: f32,
    pub dub_bass_db: f32,
    pub guitar_db: f32,
    pub organ_brass_db: f32,
    /// 0.0 to 1.0: Studer A800 2-inch analog tape saturation & harmonic warmth
    pub analog_tape_warmth: f32,
    /// 0.0 to 1.0: Roland RE-201 Space Echo dub delay feedback & spring reverb wet mix
    pub tape_echo_wet: f32,
    /// Sub-bass weight boost around 55-85 Hz (in dB)
    pub sub_bass_boost_db: f32,
    /// 1970s vocal rasp & zero-shot Seed-VC tenor timbre intensity (0.0 to 1.0)
    pub vocal_rasp_intensity: f32,
    /// Stereo imaging width (1.0 = normal, 1.25 = wide stadium/analog image)
    pub stereo_width: f32,
    /// Target integrated loudness in LUFS (e.g. -14.0)
    pub target_lufs: f32,
}

impl Default for StemMixConfig {
    fn default() -> Self {
        Self {
            lead_vocal_db: 1.5,
            gospel_choir_db: -1.0,
            drums_db: 0.5,
            dub_bass_db: 1.8,
            guitar_db: 1.2,
            organ_brass_db: -1.5,
            analog_tape_warmth: 0.78,
            tape_echo_wet: 0.34,
            sub_bass_boost_db: 3.2,
            vocal_rasp_intensity: 0.82,
            stereo_width: 1.22,
            target_lufs: -14.0,
        }
    }
}

/// A structural section of a song (e.g., Intro, Verse 1, Chorus, Instrumental Interlude, Guitar Solo, Outro).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongSection {
    pub section_id: String,
    /// Clean YuE tag without descriptive text inside brackets: "[intro]", "[verse]", "[chorus]", "[instrumental]", "[bridge]", "[guitar solo]", "[outro]"
    pub yue_clean_tag: String,
    /// Human-readable section name: "Intro", "Verse 1", "Chorus", "Instrumental Interlude", etc.
    pub display_label: String,
    /// Production cue moved to genre.txt for YuE, or kept inside brackets for Suno/Udio/ACE-Step
    pub production_cue: String,
    /// Lyric lines sung in this section (empty for purely instrumental sections)
    pub lyrics: Vec<String>,
    /// Number of 4/4 bars in this section
    pub bars: u32,
    /// Chord progression per bar or half-bar (e.g. ["A", "E", "F#m", "D"])
    pub chords: Vec<String>,
    /// Virtuosic lead electric guitar solo intensity (0.0 = offbeat rhythm skank only, 1.0 = stadium blues-rock lead solo)
    pub guitar_solo_intensity: f32,
    /// CryBaby wah-pedal filter envelope depth (0.0 to 1.0)
    pub wah_pedal_depth: f32,
    /// Polyrhythmic snare roll & master drum fill intensity (0.0 to 1.0)
    pub drum_fill_complexity: f32,
    /// Nyabinghi hand-drum polyrhythmic layer intensity (0.0 to 1.0)
    pub nyabinghi_polyrhythm: f32,
    /// Lush female gospel backing choir call-and-response level (0.0 to 1.0)
    pub gospel_choir_level: f32,
    /// Full brass horn section swells level (0.0 to 1.0)
    pub brass_swell_level: f32,
    /// Deep melodic dub sub-bass weight (0.0 to 1.0)
    pub dub_sub_bass_weight: f32,
}

/// Complete, versioned, reusable blueprint for a song in MusicFusion Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongBlueprint {
    pub song_id: String,
    pub slug: String,
    pub version: u32,
    pub parent_version: Option<u32>,
    pub title: String,
    pub subtitle: String,
    pub style_of_music: String,
    pub bpm: f64,
    pub key_signature: String,
    pub time_signature: String,
    pub sections: Vec<SongSection>,
    pub stem_mix: StemMixConfig,
    pub pipeline_models: PipelineModelStack,
    pub iteration_notes: String,
    pub created_at: String,
}

impl SongBlueprint {
    /// Total number of bars across the entire song.
    pub fn total_bars(&self) -> u32 {
        self.sections.iter().map(|s| s.bars).sum()
    }

    /// Estimated duration of the song in seconds based on BPM and 4/4 bars.
    pub fn estimated_duration_secs(&self) -> f64 {
        let beats = (self.total_bars() * 4) as f64;
        beats * (60.0 / self.bpm.max(40.0))
    }

    /// Generates the clean `genre.txt` expected by `m-a-p/YuE-s1-7B-anneal-en-cot` and `m-a-p/YuE-s1-7B-anneal-en-icl`.
    /// Moves all descriptive instrumental/vocal cues out of the lyric brackets into `genre.txt`.
    pub fn to_yue_genre_txt(&self) -> String {
        let mut cues: Vec<String> = Vec::new();
        cues.push(self.style_of_music.trim().trim_end_matches('.').to_string());
        cues.push(format!("{} BPM", self.bpm.round() as u32));
        cues.push(format!("Key of {}", self.key_signature));

        for sec in &self.sections {
            let cue = sec.production_cue.trim().trim_end_matches('.');
            if !cue.is_empty() && !cues.iter().any(|c| c.eq_ignore_ascii_case(cue)) {
                cues.push(cue.to_string());
            }
        }
        format!("{}.\n", cues.join(", "))
    }

    /// Generates the clean `lyrics.txt` expected by `YuE` (`infer.py`), with strictly clean structural
    /// tags (`[intro]`, `[verse]`, `[chorus]`, `[instrumental]`, `[bridge]`, `[guitar solo]`, `[outro]`)
    /// separated by double line breaks (`\n\n`) and no descriptive clutter inside the brackets.
    pub fn to_yue_lyrics_txt(&self) -> String {
        let mut blocks: Vec<String> = Vec::new();
        for sec in &self.sections {
            let clean_tag = if sec.yue_clean_tag.starts_with('[') && sec.yue_clean_tag.ends_with(']') {
                sec.yue_clean_tag.to_lowercase()
            } else {
                format!("[{}]", sec.yue_clean_tag.to_lowercase())
            };
            if sec.lyrics.is_empty() {
                blocks.push(clean_tag);
            } else {
                let lines = sec.lyrics.join("\n");
                blocks.push(format!("{}\n{}", clean_tag, lines));
            }
        }
        format!("{}\n", blocks.join("\n\n"))
    }

    /// Generates the full custom-mode prompt for Suno v6, Udio, and ACE-Step 1.5 with rich bracketed
    /// production directions (`[Intro: Warm analog tape hiss...]`, `[Verse 1: Soulful raspy...]`, etc.).
    pub fn to_suno_udio_custom_txt(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Style of Music: {}\n\n", self.style_of_music));
        let mut blocks: Vec<String> = Vec::new();
        for sec in &self.sections {
            let header = if sec.production_cue.trim().is_empty() {
                format!("[{}]", sec.display_label)
            } else {
                format!("[{}: {}]", sec.display_label, sec.production_cue.trim())
            };
            if sec.lyrics.is_empty() {
                blocks.push(header);
            } else {
                blocks.push(format!("{}\n{}", header, sec.lyrics.join("\n")));
            }
        }
        out.push_str(&blocks.join("\n\n"));
        out.push('\n');
        out
    }

    /// Flagship Preset 1: "One Love Shining (One Heart, One Destiny)" — Glorifying Human Love & Compassion
    /// 1977 Roots Reggae Anthem at 76 BPM in A Major, with soulful raspy Jamaican male tenor vocals,
    /// lush call-and-response female gospel choir, virtuosic psychedelic blues-rock guitar solos,
    /// and master polyrhythmic one-drop & nyabinghi drumming.
    pub fn default_one_love_shining_anthem() -> Self {
        Self {
            song_id: "song-one-love-shining-1977".to_string(),
            slug: "one_love_shining_1977".to_string(),
            version: 1,
            parent_version: None,
            title: "One Love Shining (One Heart, One Destiny)".to_string(),
            subtitle: "1977 Roots Reggae & Psychedelic Blues-Rock Fusion Anthem".to_string(),
            style_of_music: "1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, 76 BPM, hypnotic one drop master drumming with complex polyrhythmic snare rolls and deep sub bass, virtuosic soaring psychedelic blues rock electric guitar solos, warm analog tape production, lush gospel backing choir.".to_string(),
            bpm: 76.0,
            key_signature: "A Major".to_string(),
            time_signature: "4/4".to_string(),
            sections: vec![
                SongSection {
                    section_id: "intro".to_string(),
                    yue_clean_tag: "[intro]".to_string(),
                    display_label: "Intro".to_string(),
                    production_cue: "Warm analog tape hiss, intricate syncopated rimshot drum groove, deep melodic dub bassline, crying blues-rock electric guitar lead".to_string(),
                    lyrics: vec![],
                    bars: 4,
                    chords: vec!["A".to_string(), "E".to_string(), "F#m".to_string(), "D".to_string()],
                    guitar_solo_intensity: 0.78,
                    wah_pedal_depth: 0.65,
                    drum_fill_complexity: 0.75,
                    nyabinghi_polyrhythm: 0.55,
                    gospel_choir_level: 0.20,
                    brass_swell_level: 0.25,
                    dub_sub_bass_weight: 0.90,
                },
                SongSection {
                    section_id: "verse_1".to_string(),
                    yue_clean_tag: "[verse]".to_string(),
                    display_label: "Verse 1".to_string(),
                    production_cue: "Soulful raspy Jamaican tenor vocal, intimate and rhythmic, steady offbeat skank guitar and bubbling Hammond organ".to_string(),
                    lyrics: vec![
                        "We walk this earth under the same bright sun,".to_string(),
                        "Bound by a heart where every pulse is one,".to_string(),
                        "No matter where you roam or where you fall,".to_string(),
                        "The gentle touch of mercy heals us all,".to_string(),
                        "A simple smile can chase the dark away,".to_string(),
                        "And turn the coldest night into the day.".to_string(),
                    ],
                    bars: 8,
                    chords: vec![
                        "A".to_string(), "E".to_string(), "F#m".to_string(), "D".to_string(),
                        "A".to_string(), "D".to_string(), "E".to_string(), "A".to_string(),
                    ],
                    guitar_solo_intensity: 0.15,
                    wah_pedal_depth: 0.20,
                    drum_fill_complexity: 0.45,
                    nyabinghi_polyrhythm: 0.40,
                    gospel_choir_level: 0.25,
                    brass_swell_level: 0.10,
                    dub_sub_bass_weight: 0.88,
                },
                SongSection {
                    section_id: "chorus_1".to_string(),
                    yue_clean_tag: "[chorus]".to_string(),
                    display_label: "Chorus".to_string(),
                    production_cue: "Anthemic, powerful, lush call-and-response female gospel backing choir, full brass horn section swells".to_string(),
                    lyrics: vec![
                        "One love shining through the pouring rain,".to_string(),
                        "Soft compassion washing out the pain,".to_string(),
                        "Holding every brother by the hand,".to_string(),
                        "Spreading grace across the weary land,".to_string(),
                        "With every open heart we find our way,".to_string(),
                        "Love is the only light that stays.".to_string(),
                    ],
                    bars: 8,
                    chords: vec![
                        "A".to_string(), "D".to_string(), "A".to_string(), "E".to_string(),
                        "F#m".to_string(), "D".to_string(), "E".to_string(), "A".to_string(),
                    ],
                    guitar_solo_intensity: 0.30,
                    wah_pedal_depth: 0.35,
                    drum_fill_complexity: 0.68,
                    nyabinghi_polyrhythm: 0.60,
                    gospel_choir_level: 0.95,
                    brass_swell_level: 0.90,
                    dub_sub_bass_weight: 0.95,
                },
                SongSection {
                    section_id: "interlude_solo".to_string(),
                    yue_clean_tag: "[instrumental]".to_string(),
                    display_label: "Instrumental Interlude".to_string(),
                    production_cue: "Virtuosic, soaring electric guitar solo with heavy sustain, soulful string bends, and wah pedal phrasing".to_string(),
                    lyrics: vec![],
                    bars: 6,
                    chords: vec![
                        "F#m".to_string(), "D".to_string(), "A".to_string(), "E".to_string(),
                        "F#m".to_string(), "E".to_string(),
                    ],
                    guitar_solo_intensity: 0.94,
                    wah_pedal_depth: 0.90,
                    drum_fill_complexity: 0.82,
                    nyabinghi_polyrhythm: 0.70,
                    gospel_choir_level: 0.30,
                    brass_swell_level: 0.40,
                    dub_sub_bass_weight: 0.92,
                },
                SongSection {
                    section_id: "verse_2".to_string(),
                    yue_clean_tag: "[verse]".to_string(),
                    display_label: "Verse 2".to_string(),
                    production_cue: "Dynamic drum fills, polyrhythmic percussion breakdown with nyabinghi hand drums, passionate vocal delivery".to_string(),
                    lyrics: vec![
                        "To look into the eyes of someone near,".to_string(),
                        "And wipe away the bitter trace of fear,".to_string(),
                        "To share the heavy load and ease the strain,".to_string(),
                        "And walk beside them through the wind and rain,".to_string(),
                        "For human kindness is a sacred spark,".to_string(),
                        "That guides the wandering spirit through the dark.".to_string(),
                    ],
                    bars: 8,
                    chords: vec![
                        "A".to_string(), "E".to_string(), "F#m".to_string(), "D".to_string(),
                        "A".to_string(), "D".to_string(), "E".to_string(), "A".to_string(),
                    ],
                    guitar_solo_intensity: 0.22,
                    wah_pedal_depth: 0.30,
                    drum_fill_complexity: 0.85,
                    nyabinghi_polyrhythm: 0.92,
                    gospel_choir_level: 0.40,
                    brass_swell_level: 0.25,
                    dub_sub_bass_weight: 0.90,
                },
                SongSection {
                    section_id: "bridge".to_string(),
                    yue_clean_tag: "[bridge]".to_string(),
                    display_label: "Bridge".to_string(),
                    production_cue: "Stripped back to just deep bass, complex master snare rolls, and raw, emotional acapella-style lead vocals".to_string(),
                    lyrics: vec![
                        "Love is the breath we share,".to_string(),
                        "The quiet comfort there,".to_string(),
                        "No wall can keep it out,".to_string(),
                        "It conquers every doubt.".to_string(),
                    ],
                    bars: 4,
                    chords: vec!["F#m".to_string(), "D".to_string(), "Bm".to_string(), "E".to_string()],
                    guitar_solo_intensity: 0.10,
                    wah_pedal_depth: 0.15,
                    drum_fill_complexity: 0.96,
                    nyabinghi_polyrhythm: 0.88,
                    gospel_choir_level: 0.65,
                    brass_swell_level: 0.15,
                    dub_sub_bass_weight: 1.0,
                },
                SongSection {
                    section_id: "guitar_solo".to_string(),
                    yue_clean_tag: "[guitar solo]".to_string(),
                    display_label: "Guitar Solo".to_string(),
                    production_cue: "Extended, mind blowing stadium rock guitar solo weaving through the heavy roots reggae rhythm".to_string(),
                    lyrics: vec![],
                    bars: 8,
                    chords: vec![
                        "A".to_string(), "E".to_string(), "F#m".to_string(), "D".to_string(),
                        "A".to_string(), "D".to_string(), "E".to_string(), "A".to_string(),
                    ],
                    guitar_solo_intensity: 1.0,
                    wah_pedal_depth: 0.95,
                    drum_fill_complexity: 0.92,
                    nyabinghi_polyrhythm: 0.85,
                    gospel_choir_level: 0.50,
                    brass_swell_level: 0.60,
                    dub_sub_bass_weight: 0.96,
                },
                SongSection {
                    section_id: "outro".to_string(),
                    yue_clean_tag: "[outro]".to_string(),
                    display_label: "Outro".to_string(),
                    production_cue: "Full band crescendo, uplifting choir harmonies, intricate drum solo flourishes fading out into warm analog tape echo".to_string(),
                    lyrics: vec![
                        "Feel the love surrounding,".to_string(),
                        "Every heart abounding,".to_string(),
                        "One heart, one destiny,".to_string(),
                        "Walking in the light, Free.".to_string(),
                    ],
                    bars: 6,
                    chords: vec![
                        "A".to_string(), "D".to_string(), "A".to_string(), "E".to_string(),
                        "D".to_string(), "A".to_string(),
                    ],
                    guitar_solo_intensity: 0.82,
                    wah_pedal_depth: 0.75,
                    drum_fill_complexity: 0.95,
                    nyabinghi_polyrhythm: 0.90,
                    gospel_choir_level: 1.0,
                    brass_swell_level: 0.95,
                    dub_sub_bass_weight: 0.95,
                },
            ],
            stem_mix: StemMixConfig::default(),
            pipeline_models: PipelineModelStack::default(),
            iteration_notes: "Initial v1 flagship production: One Love Shining (1977 Roots Reggae & Virtuosic Blues-Rock Guitar/Drum Fusion)".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Flagship Preset 2: "Rise Up Together (Break Every Chain)" — Freedom & Unity Anthem
    /// Contains the first set of lyrics from the user's specification.
    pub fn default_rise_up_together_anthem() -> Self {
        let mut bp = Self::default_one_love_shining_anthem();
        bp.song_id = "song-rise-up-together-1977".to_string();
        bp.slug = "rise_up_together_1977".to_string();
        bp.title = "Rise Up Together (Break Every Chain)".to_string();
        bp.subtitle = "1977 Roots Reggae Freedom & Unity Anthem".to_string();
        bp.sections[1].lyrics = vec![
            "They try to build walls across the fertile land,".to_string(),
            "To divide the brother from the sister and the man,".to_string(),
            "They count up the silver and they hoard up the gold,".to_string(),
            "While the fire of the spirit is left out in the cold,".to_string(),
            "But the roots run deep beneath the concrete stone,".to_string(),
            "And the seed of freedom is a truth well known.".to_string(),
        ];
        bp.sections[2].lyrics = vec![
            "Rise up together let the trumpet sound,".to_string(),
            "Break every chain that is holding us down,".to_string(),
            "More than a river when the rains descend,".to_string(),
            "A love like a mountain that will never end,".to_string(),
            "Hear the drumbeat echo through the open sky,".to_string(),
            "We were born to conquer and we will not die.".to_string(),
        ];
        bp.sections[4].lyrics = vec![
            "They sell you the poison and they wrap it in a smile,".to_string(),
            "Keeping the people on the treadmill for a mile,".to_string(),
            "They want us to whisper when we ought to shout,".to_string(),
            "To live in the darkness and to feed on doubt,".to_string(),
            "But the dawn is breaking and the shadows retreat,".to_string(),
            "As the marching millions bring the healing beat.".to_string(),
        ];
        bp.sections[5].lyrics = vec![
            "Hold the line when the pressure is high,".to_string(),
            "Look to the stars in the midnight sky,".to_string(),
            "No weapon formed against the soul can stand,".to_string(),
            "When freedom walks across the ancient land.".to_string(),
        ];
        bp.sections[7].lyrics = vec![
            "Hear the drum beat calling,".to_string(),
            "Hear the walls come falling,".to_string(),
            "One heart, one destiny,".to_string(),
            "Walking in the light, Free.".to_string(),
        ];
        bp.iteration_notes = "Initial v1 preset: Rise Up Together (Freedom & Unity Roots Reggae Anthem)".to_string();
        bp
    }
}
