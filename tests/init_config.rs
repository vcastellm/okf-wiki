use std::{fs, path::Path, process::Command};

use tempfile::tempdir;

const CUSTOM_CONFIG: &str = "[folders]\nraw = [\"incoming\", \"research\"]\nsources = \"source-pages\"\nnotes = \"field-notes\"\nentities = \"actors\"\nconcepts = \"ideas\"\n";

#[test]
fn init_scaffolds_configured_folders_when_config_preexists() -> anyhow::Result<()> {
    // Given: a target bundle root with a pre-existing custom folder config.
    let bundle = tempdir()?;
    let config_path = bundle.path().join("okf-wiki.toml");
    fs::write(&config_path, CUSTOM_CONFIG)?;

    // When: init scaffolds the bundle.
    let output = Command::new(executable())
        .args([
            "init",
            bundle.path().to_string_lossy().as_ref(),
            "--title",
            "Custom Wiki",
            "--no-git",
        ])
        .output()?;

    // Then: the existing config is preserved and only configured folders are scaffolded.
    assert_success(&output);
    assert_eq!(fs::read_to_string(config_path)?, CUSTOM_CONFIG);
    for directory in [
        "incoming",
        "research",
        "source-pages",
        "field-notes",
        "actors",
        "ideas",
    ] {
        assert!(bundle.path().join(directory).is_dir());
    }
    assert!(!bundle.path().join("raw").exists());
    assert!(bundle.path().join("incoming/.gitkeep").is_file());
    assert!(bundle.path().join("research/.gitkeep").is_file());
    assert_file_contains(
        &bundle.path().join("source-pages/index.md"),
        "title: Source Pages",
    )?;
    assert_file_contains(
        &bundle.path().join("field-notes/index.md"),
        "title: Field Notes",
    )?;
    assert_file_contains(&bundle.path().join("actors/index.md"), "title: Actors")?;
    assert_file_contains(&bundle.path().join("ideas/index.md"), "title: Ideas")?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Notes](/field-notes/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Sources](/source-pages/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Entities](/actors/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Concepts](/ideas/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "Drop source files into `incoming/`, then run INGEST.",
    )?;
    Ok(())
}

#[test]
fn init_keeps_default_folder_paths_without_config() -> anyhow::Result<()> {
    // Given: a target bundle root without a config file.
    let bundle = tempdir()?;

    // When: init scaffolds the bundle.
    let output = Command::new(executable())
        .args(["init", bundle.path().to_string_lossy().as_ref(), "--no-git"])
        .output()?;

    // Then: the historical default folder paths are still used.
    assert_success(&output);
    for directory in ["raw", "sources", "entities", "concepts", "notes"] {
        assert!(bundle.path().join(directory).is_dir());
    }
    assert!(bundle.path().join("raw/.gitkeep").is_file());
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Notes](/notes/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "- [Sources](/sources/index.md)",
    )?;
    assert_file_contains(
        &bundle.path().join("index.md"),
        "Drop source files into `raw/`, then run INGEST.",
    )?;
    Ok(())
}

#[test]
fn init_rejects_invalid_config_before_scaffolding() -> anyhow::Result<()> {
    // Given: a target bundle root whose config is invalid.
    let bundle = tempdir()?;
    fs::write(bundle.path().join("okf-wiki.toml"), "[folders]\nraw = []\n")?;

    // When: init runs against the malformed bundle config.
    let output = Command::new(executable())
        .args(["init", bundle.path().to_string_lossy().as_ref(), "--no-git"])
        .output()?;

    // Then: it fails before creating configured or default content folders.
    assert!(!output.status.success());
    for directory in ["raw", "sources", "entities", "concepts", "notes"] {
        assert!(!bundle.path().join(directory).exists());
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
