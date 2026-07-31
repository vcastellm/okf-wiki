use clap::Parser;

fn main() -> anyhow::Result<()> {
    okf_wiki::run(okf_wiki::cli::Cli::parse())
}
