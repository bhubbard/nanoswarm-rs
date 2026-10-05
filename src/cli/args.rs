// ============================================================================
// cli/args.rs — Command-line argument definitions for NanoSwarm
// ============================================================================

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "nanoswarm",
    author = "Brandon Hubbard",
    version,
    about = "🐝 NanoSwarm: 100% on-device autonomous AI agent swarm in native Rust",
    long_about = "Run a swarm of autonomous local AI agents in ~25MB of RAM on Apple Silicon. \
                  Zero cloud fees, zero weights to download, zero data leakage."
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// Inference engine backend: 'default' (Apple Intelligence FoundationModels) or 'mlx'
    #[arg(long, default_value = "default", global = true)]
    pub engine: String,

    /// Optional LoRA model adapter path
    #[arg(long, global = true)]
    pub adapter: Option<String>,

    /// Maximum turns for sequential swarms
    #[arg(long, default_value_t = 8, global = true)]
    pub max_turns: usize,

    /// Emit report in structured JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose diagnostic logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a multi-agent security, performance, and dead code audit on a codebase
    Audit {
        /// Target directory to audit (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Run audit agents in parallel fan-out mode
        #[arg(long, default_value_t = true)]
        parallel: bool,
    },

    /// Run an autonomous engineering loop (Architect -> Coder -> Verifier)
    Dev {
        /// Feature goal or bugfix instructions
        goal: String,
    },

    /// Inspect local git state and generate conventional commits and PR descriptions
    Triage,

    /// Run a general goal-directed swarm
    Run {
        /// High-level goal for the swarm
        goal: String,

        /// Optional additional context string or file path
        #[arg(short, long)]
        context: Option<String>,
    },

    /// List all available built-in swarm templates and agent archetypes
    ListSwarms,
}
