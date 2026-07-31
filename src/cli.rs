use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "okf-wiki",
    version,
    about = "Persistent markdown knowledge base in OKF format",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Ingest(IngestArgs),
    Update(UpdateArgs),
    Truth(UpdateArgs),
    Archive(ArchiveArgs),
    Diff(DiffArgs),
    Lint(LintArgs),
    Search(SearchArgs),
    Status(StatusArgs),
    Index(IndexArgs),
    Now,
    Wire(WireArgs),
}

#[derive(Clone, Debug, Args)]
pub struct BundleArgs {
    #[arg(long)]
    pub bundle: PathBuf,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    pub bundle_dir: PathBuf,
    #[arg(long, default_value = "Wiki")]
    pub title: String,
    #[arg(long)]
    pub no_git: bool,
    #[arg(long)]
    pub from_readme: bool,
    #[arg(long, default_value = "README.md")]
    pub readme_path: PathBuf,
}

#[derive(Debug, Args)]
pub struct IngestArgs {
    pub source: PathBuf,
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub slug: Option<String>,
    #[arg(long)]
    pub no_commit: bool,
    #[arg(long)]
    pub no_lint: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    pub page: PathBuf,
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long)]
    pub message: Option<String>,
    #[arg(long)]
    pub no_commit: bool,
    #[arg(long)]
    pub no_lint: bool,
    #[arg(long)]
    pub no_timestamp: bool,
    #[arg(long, value_enum)]
    pub kind: Option<TimelineKind>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub truth: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TimelineKind {
    Decision,
    Evidence,
    Reversal,
    Note,
}

impl TimelineKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Evidence => "evidence",
            Self::Reversal => "reversal",
            Self::Note => "note",
        }
    }
}

#[derive(Debug, Args)]
pub struct ArchiveArgs {
    pub page: PathBuf,
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long)]
    pub reversal_summary: Option<String>,
    #[arg(long)]
    pub message: Option<String>,
    #[arg(long)]
    pub no_commit: bool,
    #[arg(long)]
    pub no_lint: bool,
}

#[derive(Debug, Args)]
pub struct DiffArgs {
    pub page: Option<PathBuf>,
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long, default_value_t = 1)]
    pub previous: usize,
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(Debug, Args)]
pub struct LintArgs {
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value_t = 90)]
    pub stale_days: i64,
    #[arg(long)]
    pub strict_frontmatter: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: Vec<String>,
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long, default_value_t = 10)]
    pub max_results: usize,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub toc: bool,
    #[arg(long)]
    pub include_archived: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub location: BundleArgs,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    pub bundle_dir: PathBuf,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct WireArgs {
    #[arg(long, value_enum, required = true, num_args = 1..)]
    pub agent: Vec<Agent>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum Agent {
    Claude,
    Codex,
    Cursor,
    Copilot,
    Windsurf,
    All,
}
