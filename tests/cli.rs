use std::process::Command;

use clap::Parser;
use okf_wiki::cli::Cli;

#[test]
fn parses_every_supported_subcommand() -> anyhow::Result<()> {
    // Given: one valid minimal invocation per supported command.
    let invocations: &[&[&str]] = &[
        &["okf-wiki", "init", "/tmp/wiki"],
        &["okf-wiki", "ingest", "source.txt"],
        &["okf-wiki", "update", "notes/page.md"],
        &["okf-wiki", "truth", "notes/page.md"],
        &["okf-wiki", "archive", "notes/page.md"],
        &["okf-wiki", "diff"],
        &["okf-wiki", "lint", "/tmp/wiki"],
        &["okf-wiki", "search", "authentication"],
        &["okf-wiki", "status"],
        &["okf-wiki", "index", "/tmp/wiki"],
        &["okf-wiki", "now"],
        &["okf-wiki", "dir"],
        &["okf-wiki", "wire", "--agent", "claude"],
    ];

    // When: clap parses each invocation.
    let parsed = invocations
        .iter()
        .map(|arguments| Cli::try_parse_from(*arguments))
        .collect::<Result<Vec<_>, _>>();

    // Then: every command is accepted by the Rust CLI.
    assert_eq!(parsed?.len(), invocations.len());
    Ok(())
}

#[test]
fn help_lists_the_full_command_surface() -> anyhow::Result<()> {
    // Given: the compiled binary.
    let executable = env!("CARGO_BIN_EXE_okf-wiki");

    // When: users ask for its help text.
    let output = Command::new(executable).arg("--help").output()?;

    // Then: help succeeds and advertises the expected command surface.
    let stdout = String::from_utf8(output.stdout)?;
    assert!(output.status.success());
    for command in [
        "init", "ingest", "update", "truth", "archive", "diff", "lint", "search", "status",
        "index", "now", "dir", "wire",
    ] {
        assert!(stdout.contains(command));
    }
    Ok(())
}
