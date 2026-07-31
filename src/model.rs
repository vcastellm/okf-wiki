use std::path::PathBuf;

use serde::Serialize;

pub const AUTO_INDEX_MARKER: &str = "<!-- okf:auto-index -->";
pub const AUTO_INDEX_CLOSE: &str = "<!-- /okf:auto-index -->";
pub const FACTUAL_TYPES: &[&str] = &["Source", "Note"];
pub const LIFECYCLE_STATUSES: &[&str] = &["active", "draft", "archived"];
pub const TIMELINE_KINDS: &[&str] = &["decision", "evidence", "reversal", "note"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontmatterValue {
    Scalar(String),
    List(Vec<String>),
}

impl FrontmatterValue {
    pub fn display(&self) -> String {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::List(values) => values.join(", "),
        }
    }

    pub fn as_list(&self) -> Vec<String> {
        match self {
            Self::Scalar(value) => vec![value.clone()],
            Self::List(values) => values.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Frontmatter {
    entries: Vec<(String, FrontmatterValue)>,
}

impl Frontmatter {
    pub fn get(&self, key: &str) -> Option<&FrontmatterValue> {
        self.entries
            .iter()
            .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
    }

    pub fn text(&self, key: &str) -> String {
        self.get(key)
            .map_or_else(String::new, FrontmatterValue::display)
    }

    pub fn set(&mut self, key: impl Into<String>, value: FrontmatterValue) {
        let key = key.into();
        if let Some((_, current)) = self
            .entries
            .iter_mut()
            .find(|(entry_key, _)| *entry_key == key)
        {
            *current = value;
            return;
        }
        self.entries.push((key, value));
    }

    pub fn append_list_item(&mut self, key: &str, item: String) {
        match self
            .entries
            .iter_mut()
            .find(|(entry_key, _)| entry_key == key)
        {
            Some((_, FrontmatterValue::List(items))) => items.push(item),
            Some((_, value)) => *value = FrontmatterValue::List(vec![item]),
            None => self
                .entries
                .push((key.to_owned(), FrontmatterValue::List(vec![item]))),
        }
    }

    pub fn entries(&self) -> &[(String, FrontmatterValue)] {
        &self.entries
    }
}

#[derive(Clone, Debug)]
pub struct Concept {
    pub path: PathBuf,
    pub rel: String,
    pub frontmatter: Frontmatter,
    pub body: String,
    pub links: Vec<String>,
}

impl Concept {
    pub fn is_reserved_file(&self) -> bool {
        matches!(
            self.path.file_name().and_then(|name| name.to_str()),
            Some(name) if name.eq_ignore_ascii_case("index.md")
                || name.eq_ignore_ascii_case("log.md")
                || name.eq_ignore_ascii_case("readme.md")
        )
    }

    pub fn type_tag(&self) -> String {
        let page_type = self.frontmatter.text("type");
        if page_type.is_empty() {
            "page".to_owned()
        } else {
            page_type
        }
    }

    pub fn title_or_stem(&self) -> String {
        let title = self.frontmatter.text("title");
        if title.is_empty() {
            self.path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map_or_else(|| self.rel.clone(), ToOwned::to_owned)
        } else {
            title
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SearchResult {
    pub rel: String,
    pub title: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub description: String,
    pub preview: String,
    pub score: i32,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LintIssue {
    pub file: String,
    pub rule: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LintReport {
    pub errors: Vec<LintIssue>,
    pub warnings: Vec<LintIssue>,
    pub pages: usize,
}

#[derive(Clone, Debug, Default)]
pub struct IndexReport {
    pub changed: usize,
    pub entries: Vec<(PathBuf, usize)>,
}
