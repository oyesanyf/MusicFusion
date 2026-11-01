use serde::{Deserialize, Serialize};
use crate::schema::StemMixConfig;

/// Broad genre family categories spanning world music.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenreCategory {
    RootsReggaeDub,
    AfrobeatHighlifeAmapiano,
    RnbSoulGospel,
    HipHopTrap,
    BluesRockMetal,
    JazzFusion,
    ClassicalCinematic,
    PopElectronic,
    LatinCaribbean,
    FolkCountryWorld,
    ExperimentalHybrid,
}

impl GenreCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            GenreCategory::RootsReggaeDub => "Roots Reggae & Dub",
            GenreCategory::AfrobeatHighlifeAmapiano => "Afrobeats, Highlife & Amapiano",
            GenreCategory::RnbSoulGospel => "R&B, Soul & Gospel",
            GenreCategory::HipHopTrap => "Hip-Hop & Trap",
            GenreCategory::BluesRockMetal => "Blues, Rock & Metal",
            GenreCategory::JazzFusion => "Jazz & Fusion",
            GenreCategory::ClassicalCinematic => "Classical & Cinematic",
            GenreCategory::PopElectronic => "Pop & Electronic",
            GenreCategory::LatinCaribbean => "Latin & Caribbean",
            GenreCategory::FolkCountryWorld => "Folk, Country & World",
            GenreCategory::ExperimentalHybrid => "Experimental & Hybrid",
        }
    }
}

/// Musical modal scales across Western and global traditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalScale {
    Ionian,             // Major
    Dorian,             // Minor with natural 6th
    Phrygian,           // Minor with flat 2nd
    Lydian,             // Major with raised 4th
    Mixolydian,         // Major with flat 7th
    Aeolian,            // Natural Minor
    Locrian,            // Diminished
    HarmonicMinor,      // Minor with raised 7th
    MelodicMinor,       // Minor with raised 6th and 7th
    MajorPentatonic,    // 1 2 3 5 6
    MinorPentatonic,    // 1 b3 4 5 b7
    BluesScale,         // Minor Pentatonic + b5 blue note
    ArabicHijaz,        // Phrygian Dominant (1 b2 3 4 5 b6 b7)
    JapaneseHirajoshi,  // 1 2 b3 5 b6
    SpanishGypsy,       // Phrygian Dominant with harmonic flares
    RagaBhairav,        // Indian Classical morning raga
}

impl ModalScale {
    pub fn name(&self) -> &'static str {
        match self {
            ModalScale::Ionian => "Major (Ionian)",
            ModalScale::Dorian => "Dorian",
            ModalScale::Phrygian => "Phrygian",
            ModalScale::Lydian => "Lydian",
            ModalScale::Mixolydian => "Mixolydian",
            ModalScale::Aeolian => "Natural Minor (Aeolian)",
            ModalScale::Locrian => "Locrian",
            ModalScale::HarmonicMinor => "Harmonic Minor",
            ModalScale::MelodicMinor => "Melodic Minor",
            ModalScale::MajorPentatonic => "Major Pentatonic",
            ModalScale::MinorPentatonic => "Minor Pentatonic",
            ModalScale::BluesScale => "Blues Scale",
            ModalScale::ArabicHijaz => "Arabic Maqam Hijaz",
            ModalScale::JapaneseHirajoshi => "Japanese Hirajoshi",
            ModalScale::SpanishGypsy => "Spanish Gypsy (Phrygian Dominant)",
            ModalScale::RagaBhairav => "Raga Bhairav",
        }
    }
}

/// Rhythmic groove foundation patterns for drum and rhythm synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RhythmGroove {
    OneDrop,            // Reggae: kick & snare on beat 3, 16th-note hi-hat, offbeat skank
    Steppers,           // Reggae: kick on 1, 2, 3, 4 with syncopated dub accents
    FourOnTheFloor,     // House/EDM: kick on 1, 2, 3, 4, offbeat open hat
    BoomBap,            // Hip-Hop: swung 16th hats, kick on 1 and offbeats, snare on 2 and 4
    Trap808,            // Trap: pitch-sliding 808 sub kick, rapid 32nd triplet hi-hat rolls
    AfrobeatTresillo,   // Afrobeats: 3:2 Tresillo syncopation, polyrhythmic shekere & congas
    AmapianoLogDrum,    // Amapiano: syncopated percussive bass log-drums, shaker shuffle
    HighlifeSyncopation,// Highlife: interlocking acoustic guitars, two-finger groove, clave bell
    RockBackbeat,       // Rock: heavy kick on 1 & 3, snare on 2 & 4, driving 8th-note cymbals
    MetalDoubleKick,    // Metal: rapid double-kick patterns, blast beats, tight palm-mute gallop
    JazzSwingRide,      // Jazz: triplet swing ride cymbal ding-ding-a-ding, brushed snare comping
    BossaNovaClave,     // Bossa Nova: nylon string syncopated chords, gentle rimshot clave
    SalsaCascara,       // Salsa: 2:3 or 3:2 son clave, cascara timbales, piano montuno
    CinematicPercussion,// Orchestral: heavy taiko hits, orchestral bass drum, timpani rolls
    WaltzThreeFour,     // 3/4 time: strong 1, light 2 and 3 waltz pulse
    GospelShuffle,      // Gospel: 6/8 or 12/8 swung triplet shuffle, tambourine on 2 and 4
}

/// Complete configuration preset for a musical genre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenrePreset {
    pub id: String,
    pub name: String,
    pub category: GenreCategory,
    pub subgenres: Vec<String>,
    pub default_bpm: u32,
    pub bpm_range: (u32, u32),
    pub default_time_signature: String,
    pub default_key: String,
    pub recommended_scale: ModalScale,
    pub rhythm_groove: RhythmGroove,
    pub description: String,
    pub style_tags: Vec<String>,
    pub recommended_instrumentation: Vec<String>,
    pub vocal_archetype: String,
    pub mastering_profile: String,
    pub stem_mix: StemMixConfig,
}

impl GenrePreset {
    pub fn roots_reggae_1977() -> Self {
        GenreTaxonomy::flagship_reggae_anthem()
    }

    pub fn afrobeats() -> Self {
        GenreTaxonomy::afrobeats_hit()
    }

    pub fn genre_name(&self) -> &str {
        &self.name
    }

    pub fn subgenre(&self) -> &str {
        self.subgenres.first().map(|s| s.as_str()).unwrap_or(&self.name)
    }

    pub fn min_bpm(&self) -> f64 {
        self.bpm_range.0 as f64
    }

    pub fn max_bpm(&self) -> f64 {
        self.bpm_range.1 as f64
    }

    pub fn time_signature(&self) -> &str {
        &self.default_time_signature
    }

    pub fn vocal_timbre_archetype(&self) -> &str {
        &self.vocal_archetype
    }

    pub fn rhythm_style(&self) -> String {
        format!("{:?}", self.rhythm_groove)
    }

    pub fn preferred_scale_or_mode(&self) -> &'static str {
        self.recommended_scale.name()
    }
}

/// Universal taxonomy of all human musical genres and presets.
#[derive(Debug, Default, Clone)]
pub struct GenreTaxonomy;

pub type GenreRegistry = GenreTaxonomy;

impl GenreTaxonomy {
    pub fn resolve(&self, query: &str) -> GenrePreset {
        Self::find_or_create(query)
    }

    pub fn get_genre(name: &str) -> Option<GenrePreset> {
        Self::find_preset(name)
    }

    pub fn get_or_default(name: &str) -> GenrePreset {
        Self::find_or_create(name)
    }

    pub fn list_all_genres() -> Vec<GenrePreset> {
        Self::all_presets()
    }

    /// Returns the comprehensive list of 20+ world music presets.
    pub fn all_presets() -> Vec<GenrePreset> {
        vec![
            Self::flagship_reggae_anthem(),
            Self::afrobeats_hit(),
            Self::amapiano_groove(),
            Self::highlife_golden(),
            Self::classic_soul_rnb(),
            Self::traditional_gospel(),
            Self::boom_bap_hip_hop(),
            Self::modern_trap(),
            Self::delta_blues(),
            Self::psychedelic_rock(),
            Self::heavy_metal(),
            Self::bebop_jazz(),
            Self::cinematic_orchestral(),
            Self::modern_pop(),
            Self::club_house_edm(),
            Self::latin_salsa(),
            Self::bossa_nova(),
            Self::americana_country(),
            Self::arabic_maqam_fusion(),
            Self::ambient_chillout(),
        ]
    }

    /// Primary Flagship Preset: 1977 Roots Reggae "One Love" Anthem at 76 BPM in A Major.
    pub fn flagship_reggae_anthem() -> GenrePreset {
        GenrePreset {
            id: "roots_reggae_1977".to_string(),
            name: "1977 Roots Reggae Anthem".to_string(),
            category: GenreCategory::RootsReggaeDub,
            subgenres: vec![
                "Roots Reggae".to_string(),
                "Dub".to_string(),
                "Rocksteady".to_string(),
                "Nyabinghi".to_string(),
            ],
            default_bpm: 76,
            bpm_range: (68, 84),
            default_time_signature: "4/4".to_string(),
            default_key: "A Major".to_string(),
            recommended_scale: ModalScale::Mixolydian,
            rhythm_groove: RhythmGroove::OneDrop,
            description: "1977 roots reggae anthem with soulful raspy Jamaican male tenor vocals, hypnotic one drop drumming, virtuosic blues-rock guitar solo, call-and-response gospel choir, and warm analog tape production.".to_string(),
            style_tags: vec![
                "1977 roots reggae anthem".to_string(),
                "soulful raspy Jamaican male tenor vocals".to_string(),
                "spiritual and uplifting".to_string(),
                "76 BPM".to_string(),
                "hypnotic one drop master drumming".to_string(),
                "complex polyrhythmic snare rolls".to_string(),
                "deep sub bass".to_string(),
                "virtuosic soaring psychedelic blues rock electric guitar solos".to_string(),
                "warm analog tape production".to_string(),
                "lush gospel backing choir".to_string(),
            ],
            recommended_instrumentation: vec![
                "One-drop drum kit with rimshot snare".to_string(),
                "Fender Jazz Bass with flatwound strings".to_string(),
                "Fender Stratocaster electric guitar with CryBaby wah".to_string(),
                "Hammond B3 organ with Leslie rotating speaker".to_string(),
                "Nyabinghi hand drums (funde and kete)".to_string(),
                "Brass horn section (tenor sax, trumpet, trombone)".to_string(),
                "Female gospel backing choir".to_string(),
            ],
            vocal_archetype: "Soulful Raspy Jamaican Male Tenor".to_string(),
            mastering_profile: "1977 Studer A800 2-Inch Analog Tape Reference".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn afrobeats_hit() -> GenrePreset {
        GenrePreset {
            id: "afrobeats".to_string(),
            name: "Modern Afrobeats Hit".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Afrobeats".to_string(), "Afro-Fusion".to_string(), "Afro-Pop".to_string()],
            default_bpm: 105,
            bpm_range: (96, 115),
            default_time_signature: "4/4".to_string(),
            default_key: "F# Minor".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::AfrobeatTresillo,
            description: "Contemporary West African Afrobeats with infectious 3:2 syncopation, warm basslines, bright marimba/electric piano chords, and vocal hooks.".to_string(),
            style_tags: vec![
                "modern afrobeats hit".to_string(),
                "infectious 3:2 syncopation".to_string(),
                "warm punchy 808 bass".to_string(),
                "clean muted electric guitar riffs".to_string(),
                "shaker rolls and conga fills".to_string(),
            ],
            recommended_instrumentation: vec![
                "Syncopated kick and snare rim".to_string(),
                "Shekere and shaker loop".to_string(),
                "Sub bass".to_string(),
                "Clean stratocaster riffs".to_string(),
                "Bright Rhodes/Marimba".to_string(),
            ],
            vocal_archetype: "Smooth West African Melodic Tenor/Baritone".to_string(),
            mastering_profile: "Modern Punchy Streaming Pop Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.0,
                gospel_choir_db: -2.0,
                drums_db: 1.5,
                dub_bass_db: 2.0,
                guitar_db: 0.5,
                organ_brass_db: -1.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 2.8,
                vocal_rasp_intensity: 0.20,
                stereo_width: 1.15,
                target_lufs: -12.0,
            },
        }
    }

    pub fn amapiano_groove() -> GenrePreset {
        GenrePreset {
            id: "amapiano".to_string(),
            name: "South African Amapiano".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Amapiano".to_string(), "Kwaito".to_string(), "Deep House".to_string()],
            default_bpm: 113,
            bpm_range: (108, 118),
            default_time_signature: "4/4".to_string(),
            default_key: "C Minor".to_string(),
            recommended_scale: ModalScale::Aeolian,
            rhythm_groove: RhythmGroove::AmapianoLogDrum,
            description: "Deep South African Amapiano featuring syncopated pitch-bent percussive log drums, jazzy Rhodes piano chords, shaker shuffle, and soulful toplines.".to_string(),
            style_tags: vec![
                "south african amapiano".to_string(),
                "pitch bent sub log drums".to_string(),
                "jazzy rhodes piano chords".to_string(),
                "laid-back shaker shuffle".to_string(),
                "soulful vocal chants".to_string(),
            ],
            recommended_instrumentation: vec![
                "Log drum synthesizer".to_string(),
                "Fender Rhodes electric piano".to_string(),
                "Shaker shuffle".to_string(),
                "Congas and rimshot".to_string(),
            ],
            vocal_archetype: "Soulful Chanted Topline with Harmonies".to_string(),
            mastering_profile: "Club Sound System Deep Sub Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: 0.0,
                drums_db: 1.8,
                dub_bass_db: 3.5,
                guitar_db: -2.0,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.25,
                sub_bass_boost_db: 4.5,
                vocal_rasp_intensity: 0.15,
                stereo_width: 1.20,
                target_lufs: -11.0,
            },
        }
    }

    pub fn highlife_golden() -> GenrePreset {
        GenrePreset {
            id: "highlife".to_string(),
            name: "Classic Highlife".to_string(),
            category: GenreCategory::AfrobeatHighlifeAmapiano,
            subgenres: vec!["Highlife".to_string(), "Guitar Band Highlife".to_string(), "Palm-Wine".to_string()],
            default_bpm: 118,
            bpm_range: (110, 126),
            default_time_signature: "4/4".to_string(),
            default_key: "G Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::HighlifeSyncopation,
            description: "Joyful Ghanaian & Nigerian Highlife with interlocking dual clean electric guitars, brass fanfare, clave bell, and harmonized vocals.".to_string(),
            style_tags: vec![
                "classic highlife".to_string(),
                "interlocking guitar melodies".to_string(),
                "brass fanfare".to_string(),
                "clave bell groove".to_string(),
                "harmonized joyful vocals".to_string(),
            ],
            recommended_instrumentation: vec![
                "Twin clean electric guitars".to_string(),
                "Brass horns".to_string(),
                "Bass guitar".to_string(),
                "Clave bell and congas".to_string(),
            ],
            vocal_archetype: "Harmonized Melodic Tenor Duo".to_string(),
            mastering_profile: "Vintage Warm Brass Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn classic_soul_rnb() -> GenrePreset {
        GenrePreset {
            id: "soul_rnb".to_string(),
            name: "Classic Soul & R&B".to_string(),
            category: GenreCategory::RnbSoulGospel,
            subgenres: vec!["Soul".to_string(), "Motown".to_string(), "Neo-Soul".to_string(), "R&B".to_string()],
            default_bpm: 84,
            bpm_range: (70, 96),
            default_time_signature: "4/4".to_string(),
            default_key: "E Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::GospelShuffle,
            description: "Deep, heartfelt 1960s/70s soul with expressive vocal delivery, lush strings, vibrant horns, rhythmic bass, and tambourine groove.".to_string(),
            style_tags: vec![
                "classic deep soul".to_string(),
                "passionate vocal delivery".to_string(),
                "lush orchestral strings".to_string(),
                "punchy brass swells".to_string(),
                "motown bassline".to_string(),
            ],
            recommended_instrumentation: vec![
                "Drums with tambourine".to_string(),
                "P-Bass".to_string(),
                "Wurlitzer/Rhodes".to_string(),
                "Horn section".to_string(),
                "String quartet".to_string(),
            ],
            vocal_archetype: "Passionate Gritty Soul Belter".to_string(),
            mastering_profile: "1960s Motown Analog Tape Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn traditional_gospel() -> GenrePreset {
        GenrePreset {
            id: "gospel".to_string(),
            name: "Uplifting Gospel Anthem".to_string(),
            category: GenreCategory::RnbSoulGospel,
            subgenres: vec!["Gospel".to_string(), "Praise & Worship".to_string(), "Spirituals".to_string()],
            default_bpm: 72,
            bpm_range: (60, 88),
            default_time_signature: "6/8".to_string(),
            default_key: "Ab Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::GospelShuffle,
            description: "Soaring, inspirational gospel anthem featuring mass choir call-and-response, acoustic grand piano, Hammond B3 organ swells, and dramatic crescendos.".to_string(),
            style_tags: vec![
                "uplifting gospel anthem".to_string(),
                "mass choir harmonies".to_string(),
                "hammond b3 organ swells".to_string(),
                "grand piano flourishes".to_string(),
                "dynamic crescendo".to_string(),
            ],
            recommended_instrumentation: vec![
                "Grand piano".to_string(),
                "Hammond B3 organ".to_string(),
                "Drums with marching snare accents".to_string(),
                "Bass guitar".to_string(),
                "Mass choir".to_string(),
            ],
            vocal_archetype: "Powerful Gospel Soloist & Choir".to_string(),
            mastering_profile: "Dynamic High-Headroom Choral Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.0,
                gospel_choir_db: 3.0,
                drums_db: 0.0,
                dub_bass_db: 1.0,
                guitar_db: -1.0,
                organ_brass_db: 1.5,
                analog_tape_warmth: 0.65,
                tape_echo_wet: 0.30,
                sub_bass_boost_db: 2.0,
                vocal_rasp_intensity: 0.50,
                stereo_width: 1.30,
                target_lufs: -14.0,
            },
        }
    }

    pub fn boom_bap_hip_hop() -> GenrePreset {
        GenrePreset {
            id: "boom_bap".to_string(),
            name: "90s Golden Era Boom-Bap".to_string(),
            category: GenreCategory::HipHopTrap,
            subgenres: vec!["Boom-Bap".to_string(), "East Coast Hip-Hop".to_string(), "Lo-Fi Hip-Hop".to_string()],
            default_bpm: 92,
            bpm_range: (84, 98),
            default_time_signature: "4/4".to_string(),
            default_key: "D Minor".to_string(),
            recommended_scale: ModalScale::MinorPentatonic,
            rhythm_groove: RhythmGroove::BoomBap,
            description: "Gritty 1990s New York boom-bap with crisp chopped vinyl drum breaks, filtered jazz upright bass, vinyl crackle, and rhyming cadence.".to_string(),
            style_tags: vec![
                "90s boom-bap hip hop".to_string(),
                "hard hitting drum break".to_string(),
                "filtered jazz sample".to_string(),
                "upright bass chop".to_string(),
                "rhythmic lyric flow".to_string(),
            ],
            recommended_instrumentation: vec![
                "Sampled acoustic drum break".to_string(),
                "Upright acoustic bass".to_string(),
                "Chopped jazz piano/horns".to_string(),
                "Turntable scratch accents".to_string(),
            ],
            vocal_archetype: "Crisp Articulate Rapping Cadence".to_string(),
            mastering_profile: "Punchy Vinyl & Tape Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 3.0,
                gospel_choir_db: -6.0,
                drums_db: 2.5,
                dub_bass_db: 2.0,
                guitar_db: -1.0,
                organ_brass_db: -1.0,
                analog_tape_warmth: 0.85,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 3.0,
                vocal_rasp_intensity: 0.40,
                stereo_width: 1.05,
                target_lufs: -11.5,
            },
        }
    }

    pub fn modern_trap() -> GenrePreset {
        GenrePreset {
            id: "trap".to_string(),
            name: "Modern 808 Trap".to_string(),
            category: GenreCategory::HipHopTrap,
            subgenres: vec!["Trap".to_string(), "Drill".to_string(), "Dark Trap".to_string()],
            default_bpm: 140,
            bpm_range: (130, 160),
            default_time_signature: "4/4".to_string(),
            default_key: "F Minor".to_string(),
            recommended_scale: ModalScale::Phrygian,
            rhythm_groove: RhythmGroove::Trap808,
            description: "Dark, hard-hitting Atlanta trap with sliding 808 sub-bass, rapid-fire triplet hi-hat rolls, atmospheric bells, and aggressive melodic cadence.".to_string(),
            style_tags: vec![
                "modern trap hit".to_string(),
                "distorted 808 bass slides".to_string(),
                "triplet hi-hat rolls".to_string(),
                "minor key bells and synths".to_string(),
            ],
            recommended_instrumentation: vec![
                "808 Sub-bass".to_string(),
                "Trap drum kit with snappy snare".to_string(),
                "Glockenspiel/Bells synth".to_string(),
                "Reverse reverb pads".to_string(),
            ],
            vocal_archetype: "Autotuned Melodic Trap Flow".to_string(),
            mastering_profile: "Loud Modern Commercial Trap Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.5,
                gospel_choir_db: -4.0,
                drums_db: 2.0,
                dub_bass_db: 4.0,
                guitar_db: -3.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 5.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.25,
                target_lufs: -9.0,
            },
        }
    }

    pub fn delta_blues() -> GenrePreset {
        GenrePreset {
            id: "delta_blues".to_string(),
            name: "Delta & Electric Blues".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Delta Blues".to_string(), "Chicago Blues".to_string(), "Blues Rock".to_string()],
            default_bpm: 78,
            bpm_range: (65, 96),
            default_time_signature: "4/4".to_string(),
            default_key: "E Blues".to_string(),
            recommended_scale: ModalScale::BluesScale,
            rhythm_groove: RhythmGroove::RockBackbeat,
            description: "Raw, soulful American blues with overdriven valve-amp electric guitar bends, crying harmonica, walking bass, and gravelly vocals.".to_string(),
            style_tags: vec![
                "raw electric blues".to_string(),
                "soaring guitar bends".to_string(),
                "valve amplifier saturation".to_string(),
                "blues harp harmonica".to_string(),
                "gravelly emotional vocal".to_string(),
            ],
            recommended_instrumentation: vec![
                "Gibson Les Paul / Stratocaster through tube amp".to_string(),
                "Blues harp (harmonica)".to_string(),
                "Upright or electric bass".to_string(),
                "Minimal drum kit".to_string(),
            ],
            vocal_archetype: "Raw Gravelly Blues Baritone/Tenor".to_string(),
            mastering_profile: "1950s Tube Amp & Tape Saturation Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn psychedelic_rock() -> GenrePreset {
        GenrePreset {
            id: "psychedelic_rock".to_string(),
            name: "1970s Stadium Psychedelic Rock".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Psychedelic Rock".to_string(), "Classic Rock".to_string(), "Progressive Rock".to_string()],
            default_bpm: 116,
            bpm_range: (95, 132),
            default_time_signature: "4/4".to_string(),
            default_key: "D Major".to_string(),
            recommended_scale: ModalScale::Mixolydian,
            rhythm_groove: RhythmGroove::RockBackbeat,
            description: "Expansive 1970s stadium psychedelic rock with virtuosic soaring guitar solos, phaser and wah effects, driving bass, and soaring rock vocals.".to_string(),
            style_tags: vec![
                "1970s stadium rock".to_string(),
                "virtuosic electric guitar solo".to_string(),
                "crybaby wah pedal".to_string(),
                "heavy drum fills".to_string(),
                "soaring vocal belts".to_string(),
            ],
            recommended_instrumentation: vec![
                "Lead electric guitar with overdrive and delay".to_string(),
                "Rhythm electric guitar".to_string(),
                "Rickenbacker / Precision bass".to_string(),
                "Rock drum kit with heavy crash cymbals".to_string(),
                "Mellotron / analog synth pads".to_string(),
            ],
            vocal_archetype: "Soaring Rock Tenor with Natural Grit".to_string(),
            mastering_profile: "1970s Stadium Analog Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -1.0,
                drums_db: 2.0,
                dub_bass_db: 1.5,
                guitar_db: 3.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.85,
                tape_echo_wet: 0.35,
                sub_bass_boost_db: 2.0,
                vocal_rasp_intensity: 0.70,
                stereo_width: 1.25,
                target_lufs: -12.0,
            },
        }
    }

    pub fn heavy_metal() -> GenrePreset {
        GenrePreset {
            id: "heavy_metal".to_string(),
            name: "Symphonic Heavy Metal".to_string(),
            category: GenreCategory::BluesRockMetal,
            subgenres: vec!["Heavy Metal".to_string(), "Thrash Metal".to_string(), "Symphonic Metal".to_string()],
            default_bpm: 138,
            bpm_range: (110, 180),
            default_time_signature: "4/4".to_string(),
            default_key: "E Minor".to_string(),
            recommended_scale: ModalScale::HarmonicMinor,
            rhythm_groove: RhythmGroove::MetalDoubleKick,
            description: "High-octane heavy metal featuring thunderous double-kick drumming, palm-muted high-gain guitar riffs, sweeping neo-classical solos, and operatic rock vocals.".to_string(),
            style_tags: vec![
                "symphonic heavy metal".to_string(),
                "rapid double kick drums".to_string(),
                "high gain palm muted riffs".to_string(),
                "sweeping arpeggio guitar solos".to_string(),
                "operatic vocal range".to_string(),
            ],
            recommended_instrumentation: vec![
                "Down-tuned high-gain guitars".to_string(),
                "Heavy 5-string bass".to_string(),
                "Double-bass drum kit".to_string(),
                "Orchestral choir and strings".to_string(),
            ],
            vocal_archetype: "Powerful Operatic Rock Tenor or Aggressive Grunt".to_string(),
            mastering_profile: "High-Energy Wall of Sound Metal Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: 0.5,
                drums_db: 2.5,
                dub_bass_db: 1.5,
                guitar_db: 3.5,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 2.5,
                vocal_rasp_intensity: 0.85,
                stereo_width: 1.30,
                target_lufs: -10.0,
            },
        }
    }

    pub fn bebop_jazz() -> GenrePreset {
        GenrePreset {
            id: "jazz".to_string(),
            name: "Modal Jazz & Bebop".to_string(),
            category: GenreCategory::JazzFusion,
            subgenres: vec!["Bebop".to_string(), "Modal Jazz".to_string(), "Hard Bop".to_string(), "Fusion".to_string()],
            default_bpm: 180,
            bpm_range: (120, 240),
            default_time_signature: "4/4".to_string(),
            default_key: "Bb Major".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::JazzSwingRide,
            description: "Sophisticated jazz featuring complex harmonic substitutions, swinging triplet ride cymbal, acoustic walking upright bass, and virtuoso saxophone/trumpet improvisation.".to_string(),
            style_tags: vec![
                "acoustic modal jazz".to_string(),
                "swinging ride cymbal".to_string(),
                "walking upright bass".to_string(),
                "virtuosic saxophone solo".to_string(),
                "rich extended jazz chords".to_string(),
            ],
            recommended_instrumentation: vec![
                "Tenor saxophone".to_string(),
                "Acoustic upright bass".to_string(),
                "Steinway grand piano".to_string(),
                "Jazz drum kit with coated heads and brushes".to_string(),
            ],
            vocal_archetype: "Warm Intimate Jazz Crooner or Scat Soloist".to_string(),
            mastering_profile: "Audiophile High-Dynamic Range Natural Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: -6.0,
                drums_db: 0.0,
                dub_bass_db: 0.5,
                guitar_db: 0.0,
                organ_brass_db: 1.5,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.10,
                sub_bass_boost_db: 1.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.15,
                target_lufs: -16.0,
            },
        }
    }

    pub fn cinematic_orchestral() -> GenrePreset {
        GenrePreset {
            id: "cinematic".to_string(),
            name: "Epic Cinematic Orchestral Score".to_string(),
            category: GenreCategory::ClassicalCinematic,
            subgenres: vec!["Cinematic Score".to_string(), "Orchestral".to_string(), "Film Trailer".to_string()],
            default_bpm: 80,
            bpm_range: (50, 140),
            default_time_signature: "4/4".to_string(),
            default_key: "D Minor".to_string(),
            recommended_scale: ModalScale::HarmonicMinor,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Grand orchestral soundtrack with massive brass swells, sweeping string ensembles, thundering Taiko percussion, French horn themes, and choral chants.".to_string(),
            style_tags: vec![
                "epic cinematic score".to_string(),
                "sweeping symphonic strings".to_string(),
                "massive brass horns".to_string(),
                "thunderous taiko drums".to_string(),
                "choral chanting".to_string(),
            ],
            recommended_instrumentation: vec![
                "Full symphonic string section (violins, violas, cellos, basses)".to_string(),
                "French horn section & tubas".to_string(),
                "Taiko drums and orchestral bass drum".to_string(),
                "Timpani and concert cymbals".to_string(),
            ],
            vocal_archetype: "Epic Operatic Soloist / Choral Ensemble".to_string(),
            mastering_profile: "Ultra-Wide Dynamic Cinematic Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.0,
                gospel_choir_db: 2.5,
                drums_db: 2.0,
                dub_bass_db: 2.5,
                guitar_db: -4.0,
                organ_brass_db: 3.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.35,
                sub_bass_boost_db: 3.5,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.40,
                target_lufs: -15.0,
            },
        }
    }

    pub fn modern_pop() -> GenrePreset {
        GenrePreset {
            id: "pop".to_string(),
            name: "Global Radio Pop Hit".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["Dance-Pop".to_string(), "Synthpop".to_string(), "Electropop".to_string()],
            default_bpm: 122,
            bpm_range: (112, 130),
            default_time_signature: "4/4".to_string(),
            default_key: "C Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: "Radio-ready dance-pop anthem with shimmering synthesizers, irresistible melodic earworm vocal hooks, punchy drums, and pristine modern production.".to_string(),
            style_tags: vec![
                "global radio pop hit".to_string(),
                "catchy vocal earworm".to_string(),
                "shimmering synth pads".to_string(),
                "driving four on the floor beat".to_string(),
            ],
            recommended_instrumentation: vec![
                "Synthesizer plucks and leads".to_string(),
                "Electronic drum samples".to_string(),
                "Sidechained sub-bass".to_string(),
                "Acoustic guitar strums in verses".to_string(),
            ],
            vocal_archetype: "Pristine Modern Pop Soprano/Tenor".to_string(),
            mastering_profile: "Bright Punchy Commercial Streaming Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 2.5,
                gospel_choir_db: -1.0,
                drums_db: 1.5,
                dub_bass_db: 2.0,
                guitar_db: 0.0,
                organ_brass_db: 0.0,
                analog_tape_warmth: 0.40,
                tape_echo_wet: 0.20,
                sub_bass_boost_db: 2.5,
                vocal_rasp_intensity: 0.05,
                stereo_width: 1.20,
                target_lufs: -10.5,
            },
        }
    }

    pub fn club_house_edm() -> GenrePreset {
        GenrePreset {
            id: "house_edm".to_string(),
            name: "Peak-Time House & EDM".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["House".to_string(), "Tech House".to_string(), "EDM".to_string(), "Synthwave".to_string()],
            default_bpm: 126,
            bpm_range: (124, 132),
            default_time_signature: "4/4".to_string(),
            default_key: "G Minor".to_string(),
            recommended_scale: ModalScale::Aeolian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: "Peak-time club house music driven by a pumping 4/4 kick, rolling basslines, classic 909 open hi-hats, uplifting piano stabs, and soulful vocal chops.".to_string(),
            style_tags: vec![
                "peak time club house".to_string(),
                "pumping four on the floor kick".to_string(),
                "rolling bassline".to_string(),
                "classic 909 open hi-hat".to_string(),
                "uplifting house piano chords".to_string(),
            ],
            recommended_instrumentation: vec![
                "909 Drum machine".to_string(),
                "Analog saw-wave synth bass".to_string(),
                "Korg M1 piano stabs".to_string(),
                "White noise risers and crash cymbals".to_string(),
            ],
            vocal_archetype: "Soulful Powerful House Diva Topline".to_string(),
            mastering_profile: "Club Sound System Maximum Energy Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -0.5,
                drums_db: 2.5,
                dub_bass_db: 3.5,
                guitar_db: -4.0,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.50,
                tape_echo_wet: 0.25,
                sub_bass_boost_db: 4.0,
                vocal_rasp_intensity: 0.10,
                stereo_width: 1.25,
                target_lufs: -9.5,
            },
        }
    }

    pub fn latin_salsa() -> GenrePreset {
        GenrePreset {
            id: "salsa".to_string(),
            name: "Classic Fania-Style Salsa".to_string(),
            category: GenreCategory::LatinCaribbean,
            subgenres: vec!["Salsa Dura".to_string(), "Mambo".to_string(), "Son Cubano".to_string()],
            default_bpm: 186,
            bpm_range: (160, 210),
            default_time_signature: "4/4".to_string(),
            default_key: "A Minor".to_string(),
            recommended_scale: ModalScale::SpanishGypsy,
            rhythm_groove: RhythmGroove::SalsaCascara,
            description: "Electrifying New York/Puerto Rican salsa dura with syncopated piano montuno, punchy horn section, driving timbales cascara, baby bass tumbao, and sonero vocals.".to_string(),
            style_tags: vec![
                "classic salsa dura".to_string(),
                "piano montuno".to_string(),
                "punchy horn section".to_string(),
                "timbales cascara and bongo bell".to_string(),
                "sonero call and response".to_string(),
            ],
            recommended_instrumentation: vec![
                "Timbales, congas, and bongo".to_string(),
                "Baby bass (Ampeg electric upright)".to_string(),
                "Acoustic grand piano".to_string(),
                "Trombone and trumpet section".to_string(),
            ],
            vocal_archetype: "Dynamic Improvising Sonero Vocalist".to_string(),
            mastering_profile: "Bright Dynamic Latin Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn bossa_nova() -> GenrePreset {
        GenrePreset {
            id: "bossa_nova".to_string(),
            name: "Intimate Brazilian Bossa Nova".to_string(),
            category: GenreCategory::LatinCaribbean,
            subgenres: vec!["Bossa Nova".to_string(), "Samba".to_string(), "MPB".to_string()],
            default_bpm: 128,
            bpm_range: (115, 140),
            default_time_signature: "2/4".to_string(),
            default_key: "F Major".to_string(),
            recommended_scale: ModalScale::Lydian,
            rhythm_groove: RhythmGroove::BossaNovaClave,
            description: "Gentle, intimate Brazilian Bossa Nova with nylon-string classical guitar syncopation, whispering romantic vocals, soft brush drumming, and flute flourishes.".to_string(),
            style_tags: vec![
                "intimate bossa nova".to_string(),
                "nylon-string guitar syncopation".to_string(),
                "soft brushed snare".to_string(),
                "gentle whispering vocals".to_string(),
                "flute and piano embellishments".to_string(),
            ],
            recommended_instrumentation: vec![
                "Nylon-string acoustic guitar".to_string(),
                "Upright acoustic bass".to_string(),
                "Brushed snare and gentle rimshot".to_string(),
                "Concert flute".to_string(),
            ],
            vocal_archetype: "Intimate Whispering Soft Baritone/Alto".to_string(),
            mastering_profile: "Intimate Audiophile Acoustic Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 1.5,
                gospel_choir_db: -8.0,
                drums_db: -1.0,
                dub_bass_db: 0.0,
                guitar_db: 2.0,
                organ_brass_db: 0.5,
                analog_tape_warmth: 0.60,
                tape_echo_wet: 0.15,
                sub_bass_boost_db: 1.0,
                vocal_rasp_intensity: 0.05,
                stereo_width: 1.10,
                target_lufs: -15.0,
            },
        }
    }

    pub fn americana_country() -> GenrePreset {
        GenrePreset {
            id: "americana".to_string(),
            name: "Acoustic Bluegrass & Americana".to_string(),
            category: GenreCategory::FolkCountryWorld,
            subgenres: vec!["Americana".to_string(), "Bluegrass".to_string(), "Country".to_string(), "Folk".to_string()],
            default_bpm: 104,
            bpm_range: (80, 130),
            default_time_signature: "4/4".to_string(),
            default_key: "G Major".to_string(),
            recommended_scale: ModalScale::MajorPentatonic,
            rhythm_groove: RhythmGroove::HighlifeSyncopation,
            description: "Heartfelt acoustic Americana featuring fingerpicked Martin dreadnought guitar, bluegrass fiddle, weeping pedal steel guitar, and authentic harmony singing.".to_string(),
            style_tags: vec![
                "acoustic americana bluegrass".to_string(),
                "fingerpicked acoustic guitar".to_string(),
                "bluegrass fiddle solo".to_string(),
                "weeping pedal steel guitar".to_string(),
                "two-part vocal harmony".to_string(),
            ],
            recommended_instrumentation: vec![
                "Acoustic guitar (steel string)".to_string(),
                "Fiddle / Violin".to_string(),
                "Pedal steel guitar".to_string(),
                "Upright bass".to_string(),
                "Banjo / Mandolin".to_string(),
            ],
            vocal_archetype: "Warm Honest Acoustic Storyteller".to_string(),
            mastering_profile: "Natural Room Acoustic Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn arabic_maqam_fusion() -> GenrePreset {
        GenrePreset {
            id: "arabic_maqam".to_string(),
            name: "Middle Eastern Maqam Fusion".to_string(),
            category: GenreCategory::FolkCountryWorld,
            subgenres: vec!["Arabic Maqam".to_string(), "Oriental Fusion".to_string(), "Sufi".to_string()],
            default_bpm: 96,
            bpm_range: (80, 120),
            default_time_signature: "4/4".to_string(),
            default_key: "D Hijaz".to_string(),
            recommended_scale: ModalScale::ArabicHijaz,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Mystical Middle Eastern orchestral fusion with virtuosic Oud melodies, Ney flute, microtonal string runs, Darbuka / Riq percussion, and emotional vocal ornamentation.".to_string(),
            style_tags: vec![
                "middle eastern maqam fusion".to_string(),
                "virtuosic oud solos".to_string(),
                "breathy ney flute".to_string(),
                "darbuka and riq frame drums".to_string(),
                "microtonal vocal ornamentation".to_string(),
            ],
            recommended_instrumentation: vec![
                "Oud (Middle Eastern lute)".to_string(),
                "Ney flute".to_string(),
                "Qanun (plucked zither)".to_string(),
                "Darbuka & Riq percussion".to_string(),
                "Orchestral strings in unison".to_string(),
            ],
            vocal_archetype: "Expressive Microtonal Arabic Vocalist".to_string(),
            mastering_profile: "Rich Spatial Acoustic Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }

    pub fn ambient_chillout() -> GenrePreset {
        GenrePreset {
            id: "ambient".to_string(),
            name: "Deep Atmospheric Ambient".to_string(),
            category: GenreCategory::PopElectronic,
            subgenres: vec!["Ambient".to_string(), "Chillout".to_string(), "Drone".to_string(), "Space Music".to_string()],
            default_bpm: 60,
            bpm_range: (40, 80),
            default_time_signature: "4/4".to_string(),
            default_key: "C# Minor".to_string(),
            recommended_scale: ModalScale::Dorian,
            rhythm_groove: RhythmGroove::CinematicPercussion,
            description: "Meditative, immersive soundscape with lush analog synthesizer drones, gentle sub-bass pulses, ethereal vocal textures, and vast spatial reverbs.".to_string(),
            style_tags: vec![
                "deep atmospheric ambient".to_string(),
                "lush analog synthesizer drones".to_string(),
                "gentle sub bass pulse".to_string(),
                "ethereal vocal choir texture".to_string(),
                "vast spatial reverb".to_string(),
            ],
            recommended_instrumentation: vec![
                "Analog polyphonic synthesizers (Prophet, Juno)".to_string(),
                "Bowed electric guitar / E-Bow".to_string(),
                "Sub-bass drone oscillator".to_string(),
                "Reverb-drenched vocal pads".to_string(),
            ],
            vocal_archetype: "Ethereal Wordless Ambient Vocalist".to_string(),
            mastering_profile: "Expansive Immersive Reverb Master".to_string(),
            stem_mix: StemMixConfig {
                lead_vocal_db: 0.0,
                gospel_choir_db: 2.0,
                drums_db: -4.0,
                dub_bass_db: 2.5,
                guitar_db: 0.5,
                organ_brass_db: 1.0,
                analog_tape_warmth: 0.70,
                tape_echo_wet: 0.50,
                sub_bass_boost_db: 3.5,
                vocal_rasp_intensity: 0.0,
                stereo_width: 1.45,
                target_lufs: -18.0,
            },
        }
    }

    /// Case-insensitive search across presets by id, name, or subgenre.
    pub fn find_preset(query: &str) -> Option<GenrePreset> {
        let q = query.trim().to_lowercase();
        let presets = Self::all_presets();

        // 1. Direct ID match
        if let Some(p) = presets.iter().find(|p| p.id.to_lowercase() == q) {
            return Some(p.clone());
        }
        // 2. Direct name match
        if let Some(p) = presets.iter().find(|p| p.name.to_lowercase() == q) {
            return Some(p.clone());
        }
        // 3. Subgenre match
        if let Some(p) = presets.iter().find(|p| {
            p.subgenres.iter().any(|s| s.to_lowercase() == q)
        }) {
            return Some(p.clone());
        }
        // 4. Substring contains match
        if let Some(p) = presets.iter().find(|p| {
            p.name.to_lowercase().contains(&q) || q.contains(&p.id.to_lowercase())
        }) {
            return Some(p.clone());
        }

        None
    }

    /// Intelligent fallback: Finds an existing preset, or dynamically constructs a complete, valid
    /// `GenrePreset` for ANY genre name known to humanity (e.g. "K-Pop", "Celtic Punk", "Flamenco Trap").
    pub fn find_or_create(query: &str) -> GenrePreset {
        if let Some(preset) = Self::find_preset(query) {
            return preset;
        }

        let q = query.trim().to_lowercase();
        let slug = q.replace(|c: char| !c.is_alphanumeric(), "_");

        // Keyword heuristics for auto-constructing presets
        if q.contains("reggae") || q.contains("dub") || q.contains("rasta") || q.contains("ska") {
            let mut p = Self::flagship_reggae_anthem();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("afro") || q.contains("amapiano") || q.contains("highlife") || q.contains("juju") {
            let mut p = Self::afrobeats_hit();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("soul") || q.contains("r&b") || q.contains("gospel") || q.contains("funk") {
            let mut p = Self::classic_soul_rnb();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("hip") || q.contains("hop") || q.contains("rap") || q.contains("trap") || q.contains("drill") {
            let mut p = Self::boom_bap_hip_hop();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("house") || q.contains("edm") || q.contains("techno") || q.contains("dance") || q.contains("trance") || q.contains("synth") || q.contains("cyber") || q.contains("electro") {
            let mut p = Self::club_house_edm();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("metal") || q.contains("rock") || q.contains("punk") || q.contains("grunge") {
            let mut p = Self::psychedelic_rock();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("jazz") || q.contains("swing") || q.contains("bebop") || q.contains("jazz fusion") {
            let mut p = Self::bebop_jazz();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("classic") || q.contains("orchestr") || q.contains("cinematic") || q.contains("symphon") {
            let mut p = Self::cinematic_orchestral();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("latin") || q.contains("salsa") || q.contains("bossa") || q.contains("cumbia") || q.contains("reggaeton") {
            let mut p = Self::latin_salsa();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }
        if q.contains("country") || q.contains("folk") || q.contains("bluegrass") || q.contains("acoustic") {
            let mut p = Self::americana_country();
            p.id = slug;
            p.name = query.to_string();
            return p;
        }

        // Generic intelligent fallback for any experimental or novel genre prompt
        GenrePreset {
            id: slug,
            name: query.to_string(),
            category: GenreCategory::ExperimentalHybrid,
            subgenres: vec![query.to_string()],
            default_bpm: 120,
            bpm_range: (60, 180),
            default_time_signature: "4/4".to_string(),
            default_key: "C Major".to_string(),
            recommended_scale: ModalScale::Ionian,
            rhythm_groove: RhythmGroove::FourOnTheFloor,
            description: format!("Universal production profile for {}.", query),
            style_tags: vec![query.to_string(), "120 BPM".to_string(), "4/4 time".to_string()],
            recommended_instrumentation: vec![
                "Drums and percussion".to_string(),
                "Bass".to_string(),
                "Lead instrument".to_string(),
                "Harmony chords".to_string(),
            ],
            vocal_archetype: "Versatile Melodic Vocalist".to_string(),
            mastering_profile: "Balanced Dynamic Full-Spectrum Master".to_string(),
            stem_mix: StemMixConfig::default(),
        }
    }
}
