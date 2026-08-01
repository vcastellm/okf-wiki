pub mod bundle;
pub mod cli;
pub mod commands;
pub mod config;
pub mod frontmatter;
pub mod indexer;
pub mod lint;
pub(crate) mod lint_relations;
pub(crate) mod lint_timeline;
pub mod model;
pub mod search;
pub mod sections;

use anyhow::Result;

pub fn run(cli: cli::Cli) -> Result<()> {
    commands::run(cli)
}
