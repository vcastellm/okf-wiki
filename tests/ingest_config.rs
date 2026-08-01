use std::{fs, path::Path, process::Command};

use tempfile::tempdir;

const CUSTOM_CONFIG: &str = "[folders]\nraw = [\"incoming\", \"research\"]\nsources = \"source-pages\"\nnotes = \"field-notes\"\nentities = \"actors\"\nconcepts = \"ideas\"\n";

#[test]
fn ingest_uses_configured_first_raw_folder_and_sources_folder() -> anyhow::Result<()> {
    // Given: a configured bundle and an external source file.
    let bundle = tempdir()?;
    fs::write(bundle.path().join("okf-wiki.toml"), CUSTOM_CONFIG)?;
    fs::create_dir_all(bundle.path().join("incoming"))?;
    fs::create_dir_all(bundle.path().join("research"))?;
    fs::create_dir_all(bundle.path().join("source-pages"))?;
    let source_dir = tempdir()?;
    let source = source_dir.path().join("meeting-notes.txt");
    fs::write(&source, "raw evidence")?;

    // When: ingest copies and indexes the source.
    let output = Command::new(executable())
        .args([
            "ingest",
            source.to_string_lossy().as_ref(),
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "--title",
            "Meeting Notes",
            "--slug",
            "meeting-notes",
            "--no-commit",
        ])
        .output()?;

    // Then: all generated paths use the configured folders and lint succeeds.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Created source-pages/meeting-notes.md"));
    assert!(
        stdout.contains("Source: incoming/meeting-notes.txt  Page: source-pages/meeting-notes.md")
    );
    assert!(bundle.path().join("incoming/meeting-notes.txt").is_file());
    assert!(!bundle.path().join("research/meeting-notes.txt").exists());
    assert!(!bundle.path().join("raw/meeting-notes.txt").exists());
    assert_file_contains(
        &bundle.path().join("source-pages/meeting-notes.md"),
        "sources: [incoming/meeting-notes.txt]",
    )?;
    assert_file_contains(
        &bundle.path().join("source-pages/meeting-notes.md"),
        "Source: `incoming/meeting-notes.txt`",
    )?;
    assert_file_contains(
        &bundle.path().join("source-pages/index.md"),
        "(/source-pages/meeting-notes.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("log.md"),
        "ingest: meeting-notes (incoming/meeting-notes.txt)",
    )?;
    Ok(())
}

#[test]
fn ingest_keeps_default_paths_without_config() -> anyhow::Result<()> {
    // Given: an unconfigured bundle and an external source file.
    let bundle = tempdir()?;
    let source_dir = tempdir()?;
    let source = source_dir.path().join("default-source.txt");
    fs::write(&source, "raw evidence")?;

    // When: ingest runs without a config file.
    let output = Command::new(executable())
        .args([
            "ingest",
            source.to_string_lossy().as_ref(),
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "--slug",
            "default-source",
            "--no-commit",
        ])
        .output()?;

    // Then: the historical default output paths are still used.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Created sources/default-source.md"));
    assert!(stdout.contains("Source: raw/default-source.txt  Page: sources/default-source.md"));
    assert!(bundle.path().join("raw/default-source.txt").is_file());
    assert_file_contains(
        &bundle.path().join("sources/default-source.md"),
        "sources: [raw/default-source.txt]",
    )?;
    Ok(())
}

#[test]
fn ingest_rejects_invalid_config_before_writing_files() -> anyhow::Result<()> {
    // Given: a malformed configured bundle and an external source file.
    let bundle = tempdir()?;
    fs::write(bundle.path().join("okf-wiki.toml"), "[folders]\nraw = []\n")?;
    let source_dir = tempdir()?;
    let source = source_dir.path().join("blocked.txt");
    fs::write(&source, "raw evidence")?;

    // When: ingest attempts to run.
    let output = Command::new(executable())
        .args([
            "ingest",
            source.to_string_lossy().as_ref(),
            "--bundle",
            bundle.path().to_string_lossy().as_ref(),
            "--no-commit",
        ])
        .output()?;

    // Then: it fails before copying raw files or creating pages.
    assert!(!output.status.success());
    for path in ["raw", "incoming", "sources", "source-pages", "log.md"] {
        assert!(!bundle.path().join(path).exists());
    }
    Ok(())
}

fn executable() -> &'static str {
    env!("CARGO_BIN_EXE_okf-wiki")
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_file_contains(path: &Path, needle: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(path)?;
    assert!(
        content.contains(needle),
        "{} missing {needle:?}",
        path.display()
    );
    Ok(())
}
