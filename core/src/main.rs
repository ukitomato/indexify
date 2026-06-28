// indexify — incremental trigram code-search index.
//
// One binary, several front-ends over the same Tantivy trigram index:
//   indexify init    [--root <path[@enc]>...] configure which folders to index -> settings.json
//   indexify build   [--force]                full (re)build from settings.json roots
//   indexify sync                             incremental catch-up (reuses settings.json roots)
//   indexify search  <query> [--regex] [--json]   query from the shell
//   indexify status  [--json]                 index statistics
//   indexify serve                            NDJSON sidecar for the VSCode extension
//   indexify mcp                              MCP (Model Context Protocol) stdio server
//
// The index lives in `<workspace>/.indexify/` by default (override with --index-dir or
// $INDEXIFY_INDEX_DIR). Files are decoded with their root's encoding at index time, so UTF-8 and
// Shift_JIS folders coexist in one index.

mod cmd;
mod encoding;
mod index;
mod store;
mod watcher;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "indexify", version, about = "Incremental trigram code search — CLI, MCP server, and VSCode sidecar")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Configure which folders to index (writes settings.json). With no --root and a terminal,
    /// prompts interactively; otherwise records the given roots (or the whole workspace).
    Init {
        /// Index directory (default: ./.indexify or $INDEXIFY_INDEX_DIR).
        #[arg(long)]
        index_dir: Option<String>,
        /// Folder to index, optionally with an encoding: --root legacy@shift_jis (repeatable).
        #[arg(long = "root", value_name = "PATH[@ENC]")]
        roots: Vec<String>,
        /// Overwrite an existing settings.json.
        #[arg(long)]
        force: bool,
    },
    /// Build (or rebuild) the index from the roots in settings.json (configure them with `init`).
    Build {
        /// Index directory (default: ./.indexify or $INDEXIFY_INDEX_DIR).
        #[arg(long)]
        index_dir: Option<String>,
        /// Discard the existing index and rebuild from scratch.
        #[arg(long)]
        force: bool,
    },
    /// Incrementally update the index (reindex changed/new files, drop deleted ones).
    Sync {
        #[arg(long)]
        index_dir: Option<String>,
    },
    /// Search the index.
    Search {
        /// Query string (substring by default; regex with --regex).
        query: String,
        #[arg(long)]
        index_dir: Option<String>,
        /// Treat the query as a regular expression.
        #[arg(long)]
        regex: bool,
        /// Maximum number of results.
        #[arg(long, default_value_t = 300)]
        max: usize,
        /// Output results as a JSON array.
        #[arg(long)]
        json: bool,
        /// Skip the automatic incremental sync before searching (faster, may return stale results).
        #[arg(long)]
        no_sync: bool,
        /// Case-sensitive search (default: case-insensitive).
        #[arg(long)]
        case_sensitive: bool,
    },
    /// Show index statistics.
    Status {
        #[arg(long)]
        index_dir: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Run the NDJSON sidecar (used by the VSCode extension).
    Serve {
        #[arg(long)]
        index_dir: Option<String>,
    },
    /// Run the MCP (Model Context Protocol) stdio server.
    Mcp {
        #[arg(long)]
        index_dir: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { index_dir, roots, force } => cmd::init::run(index_dir.as_deref(), &roots, force),
        Command::Build { index_dir, force } => cmd::build::run(index_dir.as_deref(), force),
        Command::Sync { index_dir } => cmd::sync::run(index_dir.as_deref()),
        Command::Search { query, index_dir, regex, max, json, no_sync, case_sensitive } => {
            cmd::search::run(index_dir.as_deref(), &query, regex, max, json, no_sync, case_sensitive)
        }
        Command::Status { index_dir, json } => cmd::status::run(index_dir.as_deref(), json),
        Command::Serve { index_dir } => cmd::serve::run(index_dir.as_deref()),
        Command::Mcp { index_dir } => cmd::mcp::run(index_dir.as_deref()),
    }
}
