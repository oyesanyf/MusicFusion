//! # `killswitch` Operator Command Line Tool
//!
//! Provides the primary operational interface for querying status, engaging/disarming
//! containment, managing services, inspecting OpenLineage DLP, and stepping the LangGraph.

use anyhow::Result;
use clap::{Parser, Subcommand};
use killswitch_cli::client::KillSwitchClient;
use killswitch_cli::commands::*;

#[derive(Parser, Debug)]
#[command(name = "killswitch")]
#[command(author = "KillSwitch Security Team")]
#[command(version = "0.1.0")]
#[command(about = "Pure Rust Standalone KillSwitch Endpoint Security Agent CLI", long_about = None)]
struct Cli {
    /// Local HTTP control server host
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Local HTTP control server port
    #[arg(short = 'p', long, default_value_t = 3030)]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query operational containment and security agent status
    Status,

    /// Engage emergency endpoint isolation and network containment
    Isolate {
        #[arg(short, long, default_value = "admin_operator")]
        operator: String,

        #[arg(short, long, default_value = "Manual containment requested")]
        reason: String,

        #[arg(short, long)]
        level: Option<String>,

        #[arg(short, long)]
        token: Option<String>,
    },

    /// Disarm containment and restore network connectivity
    Restore {
        #[arg(short, long, default_value = "admin_operator")]
        operator: String,

        #[arg(short, long, default_value = "Threat neutralized; operator restore")]
        reason: String,

        #[arg(short, long)]
        token: Option<String>,
    },

    /// Inspect or control the native OS background service
    Service {
        #[arg(default_value = "status")]
        action: String,
    },

    /// Inspect and evaluate DLP OpenLineage data provenance
    Lineage {
        #[arg(default_value = "status")]
        action: String,

        #[arg(short, long)]
        path: Option<String>,

        #[arg(short, long)]
        sha256: Option<String>,

        #[arg(short, long)]
        classification: Option<String>,

        #[arg(long)]
        policy: Option<String>,

        #[arg(long)]
        pid: Option<u32>,

        #[arg(long)]
        url: Option<String>,
    },

    /// Inspect or step the 10-node cyclic LangGraph orchestrator
    Graph {
        #[arg(default_value = "state")]
        action: String,

        #[arg(long)]
        node: Option<String>,
    },

    /// Run the 6-phase in-memory self-diagnostics
    Verify,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = KillSwitchClient::new(&cli.host, cli.port);

    match cli.command {
        Commands::Status => handle_status(&client).await?,
        Commands::Isolate { operator, reason, level, token } => {
            handle_isolate(&client, &operator, &reason, level.as_deref(), token.as_deref()).await?
        }
        Commands::Restore { operator, reason, token } => {
            handle_restore(&client, &operator, &reason, token.as_deref()).await?
        }
        Commands::Service { action } => {
            handle_service(&client, Some(&action)).await?
        }
        Commands::Lineage { action, path, sha256, classification, policy, pid, url } => {
            let path_or_url = url.as_deref().or(path.as_deref());
            handle_lineage(
                &client,
                &action,
                path_or_url,
                sha256.as_deref(),
                classification.as_deref(),
                policy.as_deref(),
                pid,
            ).await?
        }
        Commands::Graph { action, node } => {
            handle_graph(&client, &action, node.as_deref()).await?
        }
        Commands::Verify => {
            handle_verify().await?
        }
    }

    Ok(())
}
