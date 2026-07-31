use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDateTime, Utc};

use crate::model::{Frontmatter, FrontmatterValue};

pub fn parse_frontmatter(text: &str) -> (Frontmatter, &str) {
    let Some(first_end) = text.find('\n') else {
        return (Frontmatter::default(), text);
    };
    if text[..first_end].trim() != "---" {
        return (Frontmatter::default(), text);
    }
    let rest = &text[first_end + 1..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim() == "---" {
            return (parse_yamlish(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (Frontmatter::default(), text)
}

pub fn parse_yamlish(text: &str) -> Frontmatter {
    let mut frontmatter = Frontmatter::default();
    let mut current_key: Option<String> = None;
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            current_key = None;
            continue;
        }
        if let Some(item) = stripped.strip_prefix("- ") {
            if let Some(key) = current_key.as_deref() {
                let item = strip_quotes(item.trim());
                if !item.is_empty() {
                    frontmatter.append_list_item(key, item.to_owned());
                }
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            current_key = None;
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len() - 1];
            let items = inner
                .split(',')
                .map(|item| strip_quotes(item.trim()).to_owned())
                .filter(|item| !item.is_empty())
                .collect();
            frontmatter.set(key, FrontmatterValue::List(items));
            current_key = None;
        } else if value.is_empty() {
            frontmatter.set(key, FrontmatterValue::List(Vec::new()));
            current_key = Some(key.to_owned());
        } else {
            frontmatter.set(
                key,
                FrontmatterValue::Scalar(strip_quotes(value).to_owned()),
            );
            current_key = None;
        }
    }
    frontmatter
}

pub fn render_document(frontmatter: &Frontmatter, body: &str) -> String {
    let mut rendered = String::from("---\n");
    for (key, value) in frontmatter.entries() {
        match value {
            FrontmatterValue::Scalar(value) => {
                rendered.push_str(&format!("{key}: {value}\n"));
            }
            FrontmatterValue::List(values) => {
                rendered.push_str(&format!("{key}: [{}]\n", values.join(", ")));
            }
        }
    }
    rendered.push_str("---\n\n");
    rendered.push_str(body.trim_start());
    rendered
}

pub fn atomic_write_text(path: &Path, text: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("write path has no parent directory")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("write path has no UTF-8 filename")?;
    let temporary = parent.join(format!(".{name}.tmp-{}", std::process::id()));
    fs::write(&temporary, text)
        .with_context(|| format!("could not write temporary file {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("could not replace {}", path.display()))?;
    Ok(())
}

pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn valid_iso8601(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
        || NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").is_ok()
}

pub fn bump_timestamp(path: &Path) -> Result<()> {
    let text =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let (mut frontmatter, body) = parse_frontmatter(&text);
    frontmatter.set("timestamp", FrontmatterValue::Scalar(now_iso()));
    let updated = render_document(&frontmatter, body);
    if updated != text {
        atomic_write_text(path, &updated)?;
    }
    Ok(())
}

pub fn slugify(value: &str) -> String {
    let filtered = value.chars().fold(String::new(), |mut output, character| {
        if character.is_alphanumeric()
            || character == '_'
            || character.is_whitespace()
            || character == '-'
        {
            output.extend(character.to_lowercase());
        }
        output
    });
    let mut slug = String::new();
    let mut previous_separator = true;
    for character in filtered.chars() {
        if character == '_' || character.is_whitespace() || character == '-' {
            if !previous_separator {
                slug.push('-');
            }
            previous_separator = true;
        } else {
            slug.push(character);
            previous_separator = false;
        }
    }
    let slug = slug.trim_end_matches('-');
    let limited: String = slug.chars().take(80).collect();
    if limited.is_empty() {
        "untitled".to_owned()
    } else {
        limited
    }
}

pub fn require_nonempty<'a>(value: &'a str, context: &str) -> Result<&'a str> {
    if value.trim().is_empty() {
        bail!("{context}")
    }
    Ok(value)
}

fn strip_quotes(value: &str) -> &str {
    value.trim_matches(['\'', '"'])
}
