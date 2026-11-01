use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "music_cli",
    about = "Universal Multi-Genre AI Music Producer Agent (CLI, Web Studio & MCP Server)",
    version = "0.1.0",
    disable_version_flag = true
)]
pub struct CliArgs {
    /// Flagship preset: generate the 1977 Roots Reggae 76 BPM "One Love" Anthem
    #[arg(long)]
    pub create_reggae_anthem: bool,

    /// Universal multi-genre hit generation
    #[arg(long)]
    pub create_hit: bool,

    /// Target musical genre (e.g. 'Roots Reggae', 'Afrobeats', 'Amapiano', 'Hip-Hop', 'Blues Rock', 'Jazz', 'Pop')
    #[arg(long, short = 'g')]
    pub genre: Option<String>,

    /// Creative musical and lyrical prompt describing style, instrumentation, mood
    #[arg(long, short = 'p')]
    pub prompt: Option<String>,

    /// Custom song lyrics with bracketed section tags ([Intro], [Verse 1], [Chorus], etc.)
    #[arg(long, short = 'l')]
    pub lyrics: Option<String>,

    /// Target tempo in Beats Per Minute (40-220)
    #[arg(long)]
    pub bpm: Option<u32>,

    /// Musical key signature (e.g. 'A Major', 'G Minor', 'C Major')
    #[arg(long)]
    pub key: Option<String>,

    /// Time signature (e.g. '4/4', '3/4', '6/8')
    #[arg(long, default_value = "4/4")]
    pub time_signature: String,

    /// Vocal archetype ('raspy_tenor', 'smooth_rnb', 'gritty_rock', 'operatic', etc.)
    #[arg(long)]
    pub vocal_archetype: Option<String>,

    /// Execution engine mode: 'auto', 'pure_rust', or 'hf_hybrid'
    #[arg(long, default_value = "auto")]
    pub engine_mode: String,

    /// Shorthand to force pure-Rust 48 kHz DSP synthesis (no external HuggingFace models)
    #[arg(long)]
    pub pure_rust: bool,

    /// Prompt-based song iteration: describe refinements to create v(N+1) from existing song
    #[arg(long)]
    pub iterate: Option<String>,

    /// Query the dedicated Iteration Co-Pilot LLM for feedback and actionable suggestions
    #[arg(long)]
    pub assist_iteration: bool,

    /// Target song ID for iteration, stem regeneration, or mastering
    #[arg(long)]
    pub song_id: Option<String>,

    /// Target version number for iteration, stem regeneration, or mastering
    #[arg(long)]
    pub version: Option<u32>,

    /// Regenerate a single isolated stem ('drums', 'bass', 'guitar', 'vocals', 'other', 'inpainted_guitar')
    #[arg(long)]
    pub regenerate_stem: Option<String>,

    /// Run analog tape mastering on existing track
    #[arg(long)]
    pub master: bool,

    /// Mastering profile ('analog_tape', 'modern_pop', 'club_sub', 'cinematic', 'acoustic_jazz')
    #[arg(long, default_value = "analog_tape")]
    pub profile: String,

    /// Launch Tokio HTTP REST server and ChatGPT-like Web Studio UI
    #[arg(long)]
    pub serve: bool,

    /// HTTP server bind host
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// HTTP server bind port (falls back through 5050..5060 if busy)
    #[arg(long, default_value_t = 5050)]
    pub port: u16,

    /// Suppress auto-opening the web browser (headless or test mode)
    #[arg(long)]
    pub no_browser: bool,

    /// Open Web Studio UI in browser after CLI song generation finishes
    #[arg(long)]
    pub open_ui: bool,

    /// Run JSON-RPC 2.0 stdio MCP server (never launches browser)
    #[arg(long)]
    pub mcp: bool,

    /// Storage directory for song checkpoints and rendered WAV files
    #[arg(long, default_value = "storage")]
    pub storage_dir: PathBuf,

    /// Ollama endpoint host URL
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    pub ollama_host: String,
}
