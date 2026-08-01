mod boundary;

use std::{error::Error, fmt, fs, io, path::Path, path::PathBuf};

pub const CONFIG_FILE_NAME: &str = "okf-wiki.toml";

pub(super) const DEFAULT_RAW: &str = "raw";
pub(super) const DEFAULT_SOURCES: &str = "sources";
pub(super) const DEFAULT_NOTES: &str = "notes";
pub(super) const DEFAULT_ENTITIES: &str = "entities";
pub(super) const DEFAULT_CONCEPTS: &str = "concepts";
pub(super) const RESERVED_NAMES: [&str; 4] = ["index.md", "log.md", "readme.md", CONFIG_FILE_NAME];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FolderName(String);

impl FolderName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn from_validated(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WikiConfig {
    folders: FoldersConfig,
}

impl WikiConfig {
    pub fn load(bundle_root: &Path) -> Result<Self, ConfigError> {
        let path = bundle_root.join(CONFIG_FILE_NAME);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(ConfigError::Io {
                    path,
                    source: error,
                });
            }
        };

        let folders = boundary::parse_folders(&contents).map_err(|error| match error {
            boundary::ConfigFileError::Parse(source) => ConfigError::Parse {
                path: path.clone(),
                source,
            },
            boundary::ConfigFileError::Invalid(message) => ConfigError::Invalid { path, message },
        })?;

        Ok(Self { folders })
    }

    pub fn folders(&self) -> &FoldersConfig {
        &self.folders
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldersConfig {
    raw: Vec<FolderName>,
    sources: FolderName,
    notes: FolderName,
    entities: FolderName,
    concepts: FolderName,
}

impl FoldersConfig {
    pub fn raw(&self) -> &[FolderName] {
        &self.raw
    }

    pub fn sources(&self) -> &FolderName {
        &self.sources
    }

    pub fn notes(&self) -> &FolderName {
        &self.notes
    }

    pub fn entities(&self) -> &FolderName {
        &self.entities
    }

    pub fn concepts(&self) -> &FolderName {
        &self.concepts
    }

    pub(super) fn from_validated(
        raw: Vec<FolderName>,
        sources: FolderName,
        notes: FolderName,
        entities: FolderName,
        concepts: FolderName,
    ) -> Self {
        Self {
            raw,
            sources,
            notes,
            entities,
            concepts,
        }
    }
}

impl Default for FoldersConfig {
    fn default() -> Self {
        Self {
            raw: vec![FolderName(DEFAULT_RAW.to_owned())],
            sources: FolderName(DEFAULT_SOURCES.to_owned()),
            notes: FolderName(DEFAULT_NOTES.to_owned()),
            entities: FolderName(DEFAULT_ENTITIES.to_owned()),
            concepts: FolderName(DEFAULT_CONCEPTS.to_owned()),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Invalid {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "failed to parse {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(formatter, "invalid {}: {message}", path.display())
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}
