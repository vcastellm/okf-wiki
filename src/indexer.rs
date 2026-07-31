use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    bundle::load_bundle,
    frontmatter::now_iso,
    model::{AUTO_INDEX_CLOSE, AUTO_INDEX_MARKER, Concept, IndexReport},
};

pub fn rebuild_indexes(root: &Path, dry_run: bool) -> Result<IndexReport> {
    let root = root.canonicalize()?;
    let concepts = load_bundle(&root)?;
    let mut pages_by_directory: BTreeMap<PathBuf, Vec<&Concept>> = BTreeMap::new();
    for concept in &concepts {
        if !concept.is_reserved_file() {
            let directory = concept
                .path
                .parent()
                .context("concept path has no parent directory")?
                .to_owned();
            pages_by_directory
                .entry(directory)
                .or_default()
                .push(concept);
        }
    }
    let mut report = IndexReport::default();
    for (directory, pages) in pages_by_directory {
        let mut pages = pages;
        pages.sort_by_key(|concept| concept.title_or_stem().to_lowercase());
        let index_path = directory.join("index.md");
        let content = index_content(&root, &directory, &pages, &index_path)?;
        let current = fs::read_to_string(&index_path).ok();
        if current.as_deref() != Some(&content) {
            report.entries.push((index_path.clone(), pages.len()));
            if !dry_run {
                fs::write(&index_path, content)
                    .with_context(|| format!("could not write {}", index_path.display()))?;
                report.changed += 1;
            }
        }
    }
    Ok(report)
}

fn index_content(
    root: &Path,
    directory: &Path,
    pages: &[&Concept],
    index_path: &Path,
) -> Result<String> {
    let auto_block = generated_block(root, pages)?;
    match fs::read_to_string(index_path) {
        Ok(existing)
            if existing.contains(AUTO_INDEX_MARKER) && existing.contains(AUTO_INDEX_CLOSE) =>
        {
            let (before, rest) = existing
                .split_once(AUTO_INDEX_MARKER)
                .context("index marker disappeared while rebuilding")?;
            let (_, after) = rest
                .split_once(AUTO_INDEX_CLOSE)
                .context("index close marker disappeared while rebuilding")?;
            let head = format!("{}\n\n", before.trim_end());
            let tail = after.trim_start_matches('\n');
            Ok(join_index_content(&head, &auto_block, tail))
        }
        Ok(existing) if existing.contains(AUTO_INDEX_MARKER) => {
            let (before, _) = existing
                .split_once(AUTO_INDEX_MARKER)
                .context("index marker disappeared while migrating")?;
            Ok(join_index_content(
                &format!("{}\n\n", before.trim_end()),
                &auto_block,
                "",
            ))
        }
        Ok(existing) => Ok(join_index_content(
            &format!("{}\n\n", existing.trim_end()),
            &auto_block,
            "",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let directory_title = directory_title(root, directory)?;
            let head = format!(
                "---\ntype: Index\ntitle: {directory_title}\ntimestamp: {}\n---\n\n# {directory_title}\n\n",
                now_iso()
            );
            Ok(join_index_content(&head, &auto_block, ""))
        }
        Err(error) => {
            Err(error).with_context(|| format!("could not read {}", index_path.display()))
        }
    }
}

fn generated_block(root: &Path, pages: &[&Concept]) -> Result<String> {
    let mut lines = vec![AUTO_INDEX_MARKER.to_owned(), String::new()];
    for concept in pages {
        let relative = concept
            .path
            .strip_prefix(root)
            .context("concept path is outside bundle")?;
        let mut entry = format!(
            "- [{}](/{})",
            concept.title_or_stem(),
            relative.to_string_lossy().replace('\\', "/")
        );
        let description = concept.frontmatter.text("description");
        if !description.is_empty() {
            entry.push_str(&format!(" — {description}"));
        }
        lines.push(entry);
    }
    lines.extend([String::new(), AUTO_INDEX_CLOSE.to_owned(), String::new()]);
    Ok(lines.join("\n"))
}

fn join_index_content(head: &str, auto_block: &str, tail: &str) -> String {
    if tail.is_empty() {
        format!("{head}{auto_block}")
    } else {
        format!("{head}{auto_block}\n{tail}")
    }
}

fn directory_title(root: &Path, directory: &Path) -> Result<String> {
    let name = if directory == root {
        root.file_name()
    } else {
        directory.file_name()
    }
    .and_then(|name| name.to_str())
    .context("index directory has no UTF-8 name")?;
    Ok(name
        .replace('-', " ")
        .split_whitespace()
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" "))
}

fn capitalize(word: &str) -> String {
    let mut characters = word.chars();
    match characters.next() {
        Some(first) => format!(
            "{}{}",
            first.to_uppercase(),
            characters.as_str().to_lowercase()
        ),
        None => String::new(),
    }
}
