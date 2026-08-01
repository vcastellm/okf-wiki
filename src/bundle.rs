use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use walkdir::WalkDir;

use crate::{
    config::FolderName, config::WikiConfig, frontmatter::parse_frontmatter, model::Concept,
};

pub fn resolve_bundle(path: &Path) -> Result<PathBuf> {
    let root = path
        .canonicalize()
        .with_context(|| format!("bundle does not exist: {}", path.display()))?;
    if !root.is_dir() {
        bail!("bundle is not a directory: {}", path.display());
    }
    Ok(root)
}

pub fn load_bundle(root: &Path) -> Result<Vec<Concept>> {
    let root = resolve_bundle(root)?;
    let config = WikiConfig::load(&root)?;
    load_bundle_from_resolved_root(&root, &config)
}

pub(crate) fn load_bundle_with_config(root: &Path, config: &WikiConfig) -> Result<Vec<Concept>> {
    let root = resolve_bundle(root)?;
    load_bundle_from_resolved_root(&root, config)
}

fn load_bundle_from_resolved_root(root: &Path, config: &WikiConfig) -> Result<Vec<Concept>> {
    let mut paths = WalkDir::new(root)
        .into_iter()
        .map(|entry| entry.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some("md")
            && is_visible_page(path, root, config.folders().raw())
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| load_concept(root, path))
        .collect()
}

pub fn relative_target(link: &str, from_rel: &str) -> String {
    if is_external(link) || link.starts_with('/') {
        return link.to_owned();
    }
    let from_directory = from_rel
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    let combined = format!("{from_directory}/{link}");
    let combined = combined.trim_start_matches('/');
    let mut parts = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            segment => parts.push(segment),
        }
    }
    format!("/{}", parts.join("/"))
}

pub fn is_external(link: &str) -> bool {
    let Some((scheme, _)) = link.split_once("://") else {
        return false;
    };
    let mut characters = scheme.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

fn load_concept(root: &Path, path: PathBuf) -> Result<Concept> {
    let bytes = fs::read(&path).with_context(|| format!("could not read {}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let (frontmatter, body) = parse_frontmatter(&text);
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside bundle", path.display()))?;
    let rel = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
    let links = markdown_targets(body)?
        .into_iter()
        .filter(|target| !is_external(target))
        .map(|target| relative_target(&target, &rel))
        .collect();
    Ok(Concept {
        path,
        rel,
        frontmatter,
        body: body.to_owned(),
        links,
    })
}

fn markdown_targets(body: &str) -> Result<Vec<String>> {
    let pattern = Regex::new(r"\[[^\]]*\]\(([^)\s]+?\.md)(?:#[^)]*)?\)")?;
    Ok(pattern
        .captures_iter(body)
        .filter_map(|capture| capture.get(1).map(|target| target.as_str().to_owned()))
        .collect())
}

fn is_visible_page(path: &Path, root: &Path, raw_folders: &[FolderName]) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut components = relative.components();
    let first_is_raw = matches!(components.next(), Some(Component::Normal(name)) if raw_folders.iter().any(|raw| name == raw.as_str()));
    !first_is_raw
        && relative.components().all(|component| match component {
            Component::Normal(name) => !name.to_string_lossy().starts_with('.'),
            _ => true,
        })
}
