use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// QuorumRAG Command-Line Interface.
#[derive(Parser, Debug, Clone)]
#[command(name = "quorumrag")]
#[command(author = "QuorumRAG Team")]
#[command(version = "0.1.0")]
#[command(about = "QuorumRAG: Local-First P2P Retrieval-Augmented Generation Daemon & ChatGPT Web UI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Port to bind the web server and UI on
    #[arg(short, long, default_value_t = 3030)]
    pub port: u16,

    /// Host interface to bind on
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Root directory for LanceDB and Tantivy persistent data
    #[arg(long, default_value = "./data")]
    pub data_dir: PathBuf,

    /// Directory to index and watch for changes
    #[arg(long)]
    pub watch_dir: Option<PathBuf>,

    /// Do not auto-open the default browser upon server startup (for headless or CI test runs)
    #[arg(long, default_value_t = false)]
    pub no_browser: bool,

    /// Port for libp2p P2P swarm (TCP & QUIC)
    #[arg(long, default_value_t = 3031)]
    pub p2p_port: u16,

    /// Run in local-only mode (disable remote peer networking)
    #[arg(long, default_value_t = false)]
    pub local_only: bool,

    /// Default synthesis model family: AutoFusion, Glm4Agentic, Qwen25Code, GemmaProse
    #[arg(long, default_value = "AutoFusion")]
    pub model_family: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Run standalone reindex worker process on designated directories
    Reindex {
        /// Target directory path to reindex (defaults to configured watch_dir or current working dir)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Full rebuild: bypass mtime and Level-1 BLAKE3 catalogs, re-verifying all files
        #[arg(short, long, default_value_t = false)]
        full: bool,

        /// Force re-embedding all chunks even if Level-2 chunk BLAKE3 cache matches
        #[arg(long, default_value_t = false)]
        force_reembed: bool,

        /// Micro-batch size for writes to storage
        #[arg(long, default_value_t = 64)]
        batch_size: usize,
    },

    /// Run standalone drift healing process repairing all 5 drift types
    FixDrift {
        /// Dry run mode: detect and report drift anomalies without mutating storage
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Target specific drift category: merkle_state, hybrid_index_desync, vector_contract, acl_policy, storage_compaction (default: all 5)
        #[arg(long)]
        drift_type: Option<String>,
    },
}
