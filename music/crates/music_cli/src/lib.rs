pub mod browser;
pub mod cli_args;
pub mod mcp_server;
pub mod presets;
pub mod rest_server;

pub use browser::{open_browser_raw, should_suppress_browser, spawn_browser_delayed};
pub use cli_args::CliArgs;
pub use mcp_server::{handle_mcp_request, handle_tool_call, list_tools, run_mcp_server};
pub use presets::{create_hit_state, flagship_reggae_anthem_state, flagship_reggae_anthem_state_with_id};
pub use rest_server::{
    bind_server, create_router, run_server_foreground, start_rest_server, AppState,
    CreateSongRequest, CreateSongResponse, HealthResponse, IterateSongRequest, IterateSongResponse,
    MasterSongRequest, MasterSongResponse, RegenerateStemRequest, RegenerateStemResponse,
};
