//! Specialized system prompts and persona configurations for Lyricist, Arranger,
//! Sound Engineer, and Judge models across the Universal Multi-Genre taxonomy.

use music_core::genre::GenrePreset;

/// System and user prompt pair.
#[derive(Debug, Clone)]
pub struct PromptPair {
    pub system_prompt: String,
    pub user_prompt: String,
}

/// Builds the specialist prompt for the Lyricist persona.
pub fn build_lyricist_prompt(genre: &GenrePreset, topic: &str) -> PromptPair {
    let subgenre = genre.subgenres.first().cloned().unwrap_or_else(|| genre.name.clone());
    let system = format!(
        "You are an acclaimed master songwriter and lyricist specializing in {genre_name} ({subgenre}). \
        Write emotionally resonant, poetic lyrics on the user's theme. \
        CRITICAL FORMAT RULES: \
        1. You MUST structure the song into clean structural sections with YuE bracket tags on their own line: \
           [Intro], [Verse 1], [Chorus], [Instrumental Interlude], [Verse 2], [Bridge], [Guitar Solo] (or [Solo]), [Outro]. \
        2. Do NOT write descriptive commentary inside the brackets. Write '[Verse 1]', NEVER '[Verse 1: slow rhythm]'. \
        3. Embody authentic vocal phrasing: {vocal_timbre}. \
        4. Focus on visceral imagery, rhythmically matched rhyme scheme, and memorable hooks.",
        genre_name = genre.name,
        subgenre = subgenre,
        vocal_timbre = genre.vocal_archetype
    );

    let user = format!("Write full lyrics for a hit song about: {}", topic);
    PromptPair { system_prompt: system, user_prompt: user }
}

/// Builds the specialist prompt for the Arranger & Music Theorist persona.
pub fn build_arranger_prompt(genre: &GenrePreset, topic: &str) -> PromptPair {
    let system = format!(
        "You are a master music theorist and music arranger specializing in {genre_name}. \
        Propose the exact musical architecture for a hit song on the given topic. \
        Specify: \
        - Optimal BPM (standard range: {min_bpm}–{max_bpm} BPM, suggested default: {default_bpm} BPM) \
        - Time signature (default: {time_sig}) \
        - Key signature (default: {key}) \
        - Scale or mode (suggested: {mode}) \
        - Section-by-section arrangement with exact bar counts and Roman numeral / chord progressions \
          for [Intro] (4 bars), [Verse 1] (8 bars), [Chorus] (8 bars), [Instrumental Interlude] (8 bars), \
          [Verse 2] (8 bars), [Bridge] (8 bars), [Guitar Solo] (16 bars), [Outro] (4 bars).",
        genre_name = genre.name,
        min_bpm = genre.bpm_range.0,
        max_bpm = genre.bpm_range.1,
        default_bpm = genre.default_bpm,
        time_sig = genre.default_time_signature,
        key = genre.default_key,
        mode = genre.recommended_scale.name()
    );

    let user = format!("Propose harmonic arrangement and bar breakdown for: {}", topic);
    PromptPair { system_prompt: system, user_prompt: user }
}

/// Builds the specialist prompt for the Sound Engineer & Instrumentalist persona.
pub fn build_sound_engineer_prompt(genre: &GenrePreset, topic: &str) -> PromptPair {
    let system = format!(
        "You are a Grammy-winning recording and sound engineer specializing in {genre_name}. \
        Specify the instrumentation, rhythm section grooves, and analog mix cues for the song. \
        Parameters: \
        - Drum groove & rhythm: {rhythm:?} \
        - Core lead and rhythm instruments: {instruments} \
        - Sub-bass weight and bassline motion \
        - Vocal timbre processing (rasp, formant shift, backing choir placement) \
        - Studio production cues and analog mastering profile: {mastering}.",
        genre_name = genre.name,
        rhythm = genre.rhythm_groove,
        instruments = genre.recommended_instrumentation.join(", "),
        mastering = genre.mastering_profile
    );

    let user = format!("Provide sound design and instrumentation instructions for: {}", topic);
    PromptPair { system_prompt: system, user_prompt: user }
}

/// Builds the prompt for the Judge & Executive Producer persona.
pub fn build_judge_prompt(
    genre: &GenrePreset,
    topic: &str,
    lyricist_text: &str,
    arranger_text: &str,
    engineer_text: &str,
) -> PromptPair {
    let system = "You are the Executive Studio Producer and Judge model. \
        Evaluate the proposals from the Lyricist, Arranger, and Sound Engineer, reconcile differences, \
        and synthesize the final, production-ready song blueprint. \
        CRITICAL: You MUST return a single, valid JSON object ONLY. \
        Do NOT write introductory conversational text, markdown explanations, or reasoning traces outside JSON. \
        JSON Schema: \
        { \
          \"title\": \"string\", \
          \"genre_tags\": \"string (comma-separated style, tempo, key, and timbre tags for YuE genre.txt)\", \
          \"bpm\": float, \
          \"key_signature\": \"string\", \
          \"time_signature\": \"string\", \
          \"lyrics_text\": \"string (clean bracketed lyrics separated by blank lines)\", \
          \"arrangement_map\": [ \
            { \
              \"section_id\": \"intro\", \
              \"display_label\": \"Intro\", \
              \"yue_clean_tag\": \"[intro]\", \
              \"bars\": 4, \
              \"chords\": [\"A\", \"E\", \"F#m\", \"D\"], \
              \"production_cue\": \"string\", \
              \"lyrics\": [\"line 1\", \"line 2\"], \
              \"lead_instrument_intensity\": 0.7, \
              \"drum_fill_complexity\": 0.6, \
              \"bass_weight\": 0.85 \
            } \
          ], \
          \"judge_score\": 9.5, \
          \"critique\": \"string\" \
        }".to_string();

    let user = format!(
        "### TARGET GENRE: {} ({})\n\
        ### USER TOPIC:\n{}\n\n\
        ### LYRICIST PROPOSAL:\n{}\n\n\
        ### ARRANGER PROPOSAL:\n{}\n\n\
        ### SOUND ENGINEER PROPOSAL:\n{}",
        genre.name, genre.subgenre(), topic, lyricist_text, arranger_text, engineer_text
    );

    PromptPair { system_prompt: system, user_prompt: user }
}

/// Validates that lyrics contain clean YuE structural tags.
pub fn validate_structural_tags(lyrics: &str) -> bool {
    let lower = lyrics.to_lowercase();
    let has_intro = lower.contains("[intro");
    let has_verse = lower.contains("[verse");
    let has_chorus = lower.contains("[chorus");
    has_intro || (has_verse && has_chorus)
}
