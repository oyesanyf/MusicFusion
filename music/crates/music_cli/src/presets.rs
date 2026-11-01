use anyhow::Result;
use music_core::genre::GenreTaxonomy;
use music_core::state::MusicProductionState;
use crate::cli_args::CliArgs;

/// Flagship 1977 Roots Reggae "One Love" Anthem state builder.
pub fn flagship_reggae_anthem_state(
    bpm_override: Option<u32>,
    key_override: Option<&str>,
    pure_rust: bool,
) -> MusicProductionState {
    let mut state = MusicProductionState::default_reggae_anthem();
    if let Some(bpm) = bpm_override {
        state.bpm = bpm;
    }
    if let Some(key) = key_override {
        state.key_signature = key.to_string();
    }
    if pure_rust {
        state.engine_mode = "pure_rust".to_string();
    }
    state
}

/// Universal multi-genre hit generation state builder.
pub fn create_hit_state(args: &CliArgs) -> Result<MusicProductionState> {
    let genre_query = args.genre.as_deref().unwrap_or("roots_reggae");
    let preset = GenreTaxonomy::get_or_default(genre_query);

    let song_id = format!("{}-{}", preset.id, &uuid::Uuid::new_v4().to_string()[..8]);
    let bpm = args.bpm.unwrap_or(preset.default_bpm);
    let key = args.key.clone().unwrap_or_else(|| preset.default_key.clone());

    let mut state = MusicProductionState::new(&song_id, &preset.name, bpm, &key);
    state.time_signature = args.time_signature.clone();

    if args.pure_rust || args.engine_mode == "pure_rust" {
        state.engine_mode = "pure_rust".to_string();
    } else {
        state.engine_mode = args.engine_mode.clone();
    }

    if let Some(prompt) = &args.prompt {
        state.style_prompt = prompt.clone();
    } else {
        state.style_prompt = format!("{} hit in {}, tempo {} BPM, {}", preset.name, key, bpm, preset.description);
    }

    if let Some(lyrics) = &args.lyrics {
        state.lyrics = lyrics.clone();
    }

    Ok(state)
}
