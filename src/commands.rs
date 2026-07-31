mod archive;
mod diff;
mod ingest;
mod init;
mod manage;
mod overview;
mod search;
mod support;
mod update;
mod wire;

use anyhow::Result;

use crate::{
    cli::{Cli, Command},
    frontmatter::now_iso,
};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init(args) => init::run(args),
        Command::Ingest(args) => ingest::run(args),
        Command::Update(args) => update::run(args, false),
        Command::Truth(args) => update::run(args, true),
        Command::Archive(args) => archive::run(args),
        Command::Diff(args) => diff::run(args),
        Command::Lint(args) => manage::lint(args),
        Command::Search(args) => search::run(args),
        Command::Status(args) => overview::status(args),
        Command::Index(args) => manage::index(args),
        Command::Now => {
            println!("{}", now_iso());
            Ok(())
        }
        Command::Dir(args) => overview::directory(args),
        Command::Wire(args) => wire::run(args),
    }
}
