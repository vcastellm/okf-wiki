use std::collections::HashSet;

use serde::Deserialize;

use super::{
    DEFAULT_CONCEPTS, DEFAULT_ENTITIES, DEFAULT_NOTES, DEFAULT_RAW, DEFAULT_SOURCES, FolderName,
    FoldersConfig, RESERVED_NAMES,
};

pub(super) enum ConfigFileError {
    Parse(toml::de::Error),
    Invalid(String),
}

pub(super) fn parse_folders(contents: &str) -> Result<FoldersConfig, ConfigFileError> {
    let file = toml::from_str::<ConfigFile>(contents).map_err(ConfigFileError::Parse)?;
    folders_from_file(file.folders).map_err(ConfigFileError::Invalid)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    folders: FoldersFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FoldersFile {
    #[serde(default = "default_raw")]
    raw: Vec<String>,
    #[serde(default)]
    ignored: Vec<String>,
    #[serde(default = "default_sources")]
    sources: String,
    #[serde(default = "default_notes")]
    notes: String,
    #[serde(default = "default_entities")]
    entities: String,
    #[serde(default = "default_concepts")]
    concepts: String,
}

impl Default for FoldersFile {
    fn default() -> Self {
        Self {
            raw: default_raw(),
            ignored: Vec::new(),
            sources: default_sources(),
            notes: default_notes(),
            entities: default_entities(),
            concepts: default_concepts(),
        }
    }
}

fn folders_from_file(file: FoldersFile) -> Result<FoldersConfig, String> {
    if file.raw.is_empty() {
        return Err("folders.raw must contain at least one folder".to_owned());
    }

    let raw = file
        .raw
        .into_iter()
        .map(parse_folder_name)
        .collect::<Result<Vec<_>, _>>()?;
    let ignored = file
        .ignored
        .into_iter()
        .map(parse_folder_name)
        .collect::<Result<Vec<_>, _>>()?;
    let sources = parse_folder_name(file.sources)?;
    let notes = parse_folder_name(file.notes)?;
    let entities = parse_folder_name(file.entities)?;
    let concepts = parse_folder_name(file.concepts)?;

    reject_duplicates(&raw, [&sources, &notes, &entities, &concepts], &ignored)?;

    Ok(FoldersConfig::from_validated(
        raw, ignored, sources, notes, entities, concepts,
    ))
}

fn parse_folder_name(value: String) -> Result<FolderName, String> {
    validate_folder_name(&value)?;
    Ok(FolderName::from_validated(value))
}

fn validate_folder_name(value: &str) -> Result<(), String> {
    let reserved_key = comparable_name(value);
    if value.is_empty() {
        return Err("folder names must not be empty".to_owned());
    }
    if value == "." || value == ".." || value.starts_with('.') {
        return Err(format!("folder name '{value}' must be visible"));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(format!("folder name '{value}' must be a single component"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "folder name '{value}' must not contain control characters"
        ));
    }
    if RESERVED_NAMES.contains(&reserved_key.as_str()) {
        return Err(format!("folder name '{value}' is reserved"));
    }
    Ok(())
}

fn reject_duplicates(
    raw: &[FolderName],
    managed: [&FolderName; 4],
    ignored: &[FolderName],
) -> Result<(), String> {
    let mut names = HashSet::new();
    for folder in raw.iter().chain(managed).chain(ignored) {
        if !names.insert(comparable_name(folder.as_str())) {
            return Err(format!("folder name '{}' is duplicated", folder.as_str()));
        }
    }
    Ok(())
}

fn comparable_name(value: &str) -> String {
    value.to_lowercase()
}

fn default_raw() -> Vec<String> {
    vec![DEFAULT_RAW.to_owned()]
}

fn default_sources() -> String {
    DEFAULT_SOURCES.to_owned()
}

fn default_notes() -> String {
    DEFAULT_NOTES.to_owned()
}

fn default_entities() -> String {
    DEFAULT_ENTITIES.to_owned()
}

fn default_concepts() -> String {
    DEFAULT_CONCEPTS.to_owned()
}
