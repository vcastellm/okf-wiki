use std::fs;

use okf_wiki::config::{FolderName, WikiConfig};
use tempfile::tempdir;

fn raw_names(raw: &[FolderName]) -> Vec<&str> {
    raw.iter().map(FolderName::as_str).collect()
}

#[test]
fn loads_default_folders_when_config_file_is_absent() -> anyhow::Result<()> {
    // Given: a bundle root without okf-wiki.toml.
    let bundle = tempdir()?;

    // When: its wiki config is loaded.
    let config = WikiConfig::load(bundle.path())?;

    // Then: every folder uses the current built-in default.
    assert_eq!(raw_names(config.folders().raw()), vec!["raw"]);
    assert_eq!(config.folders().sources().as_str(), "sources");
    assert_eq!(config.folders().notes().as_str(), "notes");
    assert_eq!(config.folders().entities().as_str(), "entities");
    assert_eq!(config.folders().concepts().as_str(), "concepts");
    Ok(())
}

#[test]
fn preserves_defaults_for_partial_config_file() -> anyhow::Result<()> {
    // Given: a config file overriding only ordered raw folders.
    let bundle = tempdir()?;
    fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nraw = [\"raw\", \"research\"]\n",
    )?;

    // When: its wiki config is loaded.
    let config = WikiConfig::load(bundle.path())?;

    // Then: the raw order is preserved and managed folders keep defaults.
    assert_eq!(raw_names(config.folders().raw()), vec!["raw", "research"]);
    assert_eq!(config.folders().sources().as_str(), "sources");
    assert_eq!(config.folders().notes().as_str(), "notes");
    assert_eq!(config.folders().entities().as_str(), "entities");
    assert_eq!(config.folders().concepts().as_str(), "concepts");
    Ok(())
}

#[test]
fn loads_every_supported_folder_override() -> anyhow::Result<()> {
    // Given: a config file using the supported folder schema.
    let bundle = tempdir()?;
    fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nraw = [\"raw\", \"research\"]\nsources = \"citations\"\nnotes = \"memos\"\nentities = \"actors\"\nconcepts = \"ideas\"\n",
    )?;

    // When: its wiki config is loaded.
    let config = WikiConfig::load(bundle.path())?;

    // Then: typed accessors expose every configured value immutably.
    assert_eq!(raw_names(config.folders().raw()), vec!["raw", "research"]);
    assert_eq!(config.folders().sources().as_str(), "citations");
    assert_eq!(config.folders().notes().as_str(), "memos");
    assert_eq!(config.folders().entities().as_str(), "actors");
    assert_eq!(config.folders().concepts().as_str(), "ideas");
    Ok(())
}

#[test]
fn rejects_invalid_config_with_path_context() -> anyhow::Result<()> {
    for body in [
        "unexpected = true\n",
        "[folders]\nraw = []\n",
        "[folders]\nraw = [\"\"]\n",
        "[folders]\nsources = \".\"\n",
        "[folders]\nnotes = \"..\"\n",
        "[folders]\nraw = [\"raw/file\"]\n",
        "[folders]\nraw = [\"raw\\\\file\"]\n",
        "[folders]\nraw = [\".hidden\"]\n",
        "[folders]\nraw = [\"raw\", \"notes\"]\n",
        "[folders]\nraw = [\"raw\", \"RAW\"]\n",
        "[folders]\nnotes = \"README.md\"\n",
    ] {
        // Given: a config file containing strict invalid input.
        let bundle = tempdir()?;
        let config_path = bundle.path().join("okf-wiki.toml");
        fs::write(&config_path, body)?;

        // When: its wiki config is loaded.
        let error = WikiConfig::load(bundle.path()).unwrap_err();

        // Then: loading fails and the message identifies the config path.
        assert!(
            error
                .to_string()
                .contains(&config_path.display().to_string())
        );
    }
    Ok(())
}
