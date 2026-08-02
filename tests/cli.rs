use std::process::Command;

use clap::{CommandFactory, Parser};
use okf_wiki::cli::Cli;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn parses_every_supported_subcommand() -> anyhow::Result<()> {
    // Given: one valid minimal invocation per supported command.
    let invocations: &[&[&str]] = &[
        &["okf-wiki", "init", "."],
        &["okf-wiki", "ingest", "source.txt", "--bundle", "."],
        &["okf-wiki", "update", "notes/page.md", "--bundle", "."],
        &["okf-wiki", "truth", "notes/page.md", "--bundle", "."],
        &["okf-wiki", "archive", "notes/page.md", "--bundle", "."],
        &["okf-wiki", "diff", "--bundle", "."],
        &["okf-wiki", "lint", "--bundle", "."],
        &["okf-wiki", "search", "authentication", "--bundle", "."],
        &["okf-wiki", "status", "--bundle", "."],
        &["okf-wiki", "index", "."],
        &["okf-wiki", "now"],
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
fn bundle_scoped_commands_require_explicit_bundle() -> anyhow::Result<()> {
    // Given: bundle-scoped commands without their required bundle option.
    let invocations: &[&[&str]] = &[
        &["okf-wiki", "ingest", "source.txt"],
        &["okf-wiki", "update", "notes/page.md"],
        &["okf-wiki", "truth", "notes/page.md"],
        &["okf-wiki", "archive", "notes/page.md"],
        &["okf-wiki", "diff"],
        &["okf-wiki", "lint"],
        &["okf-wiki", "search", "authentication"],
        &["okf-wiki", "status"],
        &["okf-wiki", "dir"],
    ];

    // When: clap parses each incomplete invocation.
    let rejected = invocations
        .iter()
        .filter(|arguments| Cli::try_parse_from(**arguments).is_err())
        .count();

    // Then: every bundle-scoped command is rejected without --bundle.
    assert_eq!(rejected, invocations.len());
    Ok(())
}

#[test]
fn dir_is_rejected_as_a_cli_subcommand() {
    // Given: the removed dir subcommand.
    let arguments = ["okf-wiki", "dir", "--bundle", "."];

    // When: clap parses the removed command.
    let parsed = Cli::try_parse_from(arguments);

    // Then: parsing fails.
    assert!(parsed.is_err());
}

#[test]
fn help_lists_the_full_command_surface() {
    // Given: the compiled binary.
    let command = Cli::command();

    // When: clap exposes the top-level subcommands structurally.
    let subcommands = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .collect::<Vec<_>>();

    // Then: the supported command surface includes the expected commands and excludes dir.
    for command in [
        "init", "ingest", "update", "truth", "archive", "diff", "lint", "search", "status",
        "index", "now", "wire",
    ] {
        assert!(subcommands.iter().any(|subcommand| subcommand == command));
    }
    assert!(!subcommands.iter().any(|subcommand| subcommand == "dir"));
}

#[test]
fn status_bundle_dot_uses_command_current_dir() -> anyhow::Result<()> {
    // Given: an executable launched with a temporary directory as its current directory.
    let bundle = tempdir()?;
    let executable = env!("CARGO_BIN_EXE_okf-wiki");
    let expected_root = bundle.path().canonicalize()?;

    // When: status resolves the explicit relative bundle path.
    let output = Command::new(executable)
        .args(["status", "--bundle", "."])
        .current_dir(bundle.path())
        .output()?;

    // Then: status reports the temporary current directory as its bundle root.
    let stdout = String::from_utf8(output.stdout)?;
    assert!(output.status.success());
    assert!(stdout.contains(&format!("OKF status: {}", expected_root.display())));
    Ok(())
}

#[test]
fn status_json_counts_raw_files_and_excludes_ignored_pages() -> anyhow::Result<()> {
    // Given: a bundle with raw roots, ignored roots, and one managed page.
    let bundle = tempdir()?;
    std::fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nraw = [\"incoming\", \"research\"]\nignored = [\"scratch\", \"vendor\"]\n",
    )?;
    std::fs::create_dir_all(bundle.path().join("incoming/nested"))?;
    std::fs::create_dir_all(bundle.path().join("research"))?;
    std::fs::create_dir_all(bundle.path().join("scratch/nested/deeper"))?;
    std::fs::create_dir_all(bundle.path().join("vendor/package/docs"))?;
    std::fs::create_dir_all(bundle.path().join("notes"))?;
    std::fs::write(bundle.path().join("incoming/a.md"), "a")?;
    std::fs::write(bundle.path().join("incoming/.hidden.md"), "hidden")?;
    std::fs::write(bundle.path().join("incoming/nested/deep.md"), "deep")?;
    std::fs::write(bundle.path().join("research/b.md"), "b")?;
    std::fs::write(bundle.path().join("research/c.txt"), "c")?;
    std::fs::write(
        bundle.path().join("scratch/nested/deeper/draft.md"),
        "---\ntype: Note\ntitle: Scratch Draft\n---\n\nIgnored.\n",
    )?;
    std::fs::write(
        bundle.path().join("vendor/package/docs/readme.md"),
        "---\ntype: Note\ntitle: Vendor Readme\n---\n\nIgnored.\n",
    )?;
    std::fs::write(
        bundle.path().join("notes/page.md"),
        "---\ntype: Note\ntitle: Page\n---\n\nVisible.\n",
    )?;

    // When: status is requested as machine-readable JSON.
    let executable = env!("CARGO_BIN_EXE_okf-wiki");
    let output = Command::new(executable)
        .args([
            "status",
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "--json",
        ])
        .output()?;

    // Then: the raw count is raw-root-only and page count excludes ignored descendants.
    assert!(output.status.success());
    let body: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(body["raw_files"], 3);
    assert_eq!(body["page_count"], 1);
    Ok(())
}

#[test]
fn status_fails_before_output_when_config_is_malformed() -> anyhow::Result<()> {
    // Given: a bundle with an invalid config that would otherwise have raw files to count.
    let bundle = tempdir()?;
    std::fs::write(bundle.path().join("okf-wiki.toml"), "[folders]\nraw = []\n")?;
    std::fs::create_dir_all(bundle.path().join("raw"))?;
    std::fs::write(bundle.path().join("raw/a.md"), "a")?;

    // When: status is run through the compiled CLI.
    let executable = env!("CARGO_BIN_EXE_okf-wiki");
    let output = Command::new(executable)
        .args([
            "status",
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "--json",
        ])
        .output()?;

    // Then: command execution fails before any status JSON is printed.
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn diff_prints_no_diff_for_page_without_history() -> anyhow::Result<()> {
    // Given: a git-backed bundle with a page that has never been committed.
    let bundle = tempdir()?;
    std::fs::create_dir_all(bundle.path().join("notes"))?;
    std::fs::write(
        bundle.path().join("notes/page.md"),
        "---\ntype: Note\n---\n\nBody.\n",
    )?;
    let git_init = Command::new("git")
        .args(["-C", bundle.path().to_string_lossy().as_ref(), "init", "-q"])
        .status()?;
    assert!(git_init.success());

    // When: diffing the page through the compiled CLI.
    let executable = env!("CARGO_BIN_EXE_okf-wiki");
    let output = Command::new(executable)
        .args([
            "diff",
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "notes/page.md",
        ])
        .output()?;

    // Then: the command succeeds and only reports the empty diff marker.
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "(no diff)\n");
    Ok(())
}
