use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "osmozzz",
    about = "Local semantic memory for AI agents",
    version,
    long_about = "OSMOzzz indexes your personal data locally and provides\nsemantic search without sending anything to the cloud."
)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index documents from a source
    Index(IndexArgs),
    /// Search indexed documents
    Search(SearchArgs),
    /// Start the UDS bridge daemon for OpenClaw integration
    Serve(ServeArgs),
    /// Show indexing status and statistics
    Status,
    /// Start MCP server (stdio) for Claude Desktop integration
    Mcp,
    /// Start the background watcher daemon (independent of AI tools)
    ///
    /// Watches ~/Desktop and ~/Documents for new/modified files
    /// and indexes them silently into LanceDB.
    /// Run this once at login; MCP and other tools share the same DB.
    Daemon,
    /// Compact the vector database to restore fast search
    ///
    /// Run this after bulk indexing (Gmail, Chrome, files).
    /// Merges thousands of tiny fragment files into one and prunes old versions.
    Compact,
}

#[derive(Args)]
pub struct IndexArgs {
    /// Source to index: chrome | files
    #[arg(short, long)]
    pub source: String,

    /// Path for file source (required when --source=files)
    #[arg(short, long)]
    pub path: Option<String>,

    /// Batch size for embedding (default: 100)
    #[arg(short, long, default_value = "100")]
    pub batch: usize,

    /// Delete all existing docs for this source before re-indexing
    #[arg(long)]
    pub reset: bool,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Number of results to return
    #[arg(short, long, default_value = "5")]
    pub limit: usize,

    /// Output format: text | json
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Filter results by source type: chrome | email | file | markdown | pdf
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Args)]
pub struct ServeArgs {
    /// Socket path (overrides config)
    #[arg(short, long)]
    pub socket: Option<String>,
}
