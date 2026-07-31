use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    bundle::{load_bundle, resolve_bundles, resolve_single_bundle},
    cli::{DirArgs, StatusArgs},
    model::LIFECYCLE_STATUSES,
};

pub(crate) fn status(args: StatusArgs) -> Result<()> {
    let bundles =
        resolve_overview_bundles(args.location.bundle.as_deref(), args.location.tier.as_str())?;
    let mut results = BTreeMap::new();
    for (tier, root) in &bundles {
        let label = if tier.is_empty() {
            root.display().to_string()
        } else {
            tier.clone()
        };
        results.insert(label, bundle_status(root)?);
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for (label, result) in results {
            let prefix = if args.location.tier.as_str() == "all" {
                format!("[{label}] ")
            } else {
                String::new()
            };
            println!("OKF status: {prefix}{}", result.root);
            println!("  pages:       {}", result.page_count);
            println!("  by type:     {}", format_counts(&result.types));
            let lifecycle = LIFECYCLE_STATUSES
                .iter()
                .filter_map(|status| {
                    result
                        .statuses
                        .get(*status)
                        .map(|count| format!("{status}={count}"))
                })
                .collect::<Vec<_>>();
            if !lifecycle.is_empty() {
                println!("  lifecycle:  {}", lifecycle.join(", "));
            }
            if !result.last_log.is_empty() {
                println!(
                    "  last change: {}",
                    result.last_log.chars().take(80).collect::<String>()
                );
            }
            println!();
        }
    }
    Ok(())
}

pub(crate) fn directory(args: DirArgs) -> Result<()> {
    let bundles = resolve_overview_bundles(args.bundle.as_deref(), args.tier.as_str())?;
    let entries = bundles
        .iter()
        .map(|(tier, root)| directory_entry(tier, root))
        .collect::<Result<Vec<_>>>()?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for entry in entries {
            let tag = if entry.tier.is_empty() {
                String::new()
            } else {
                format!("[{}] ", entry.tier)
            };
            println!("{tag}{}", entry.path);
            println!("  exists:    {}", entry.exists);
            println!("  populated: {}", entry.populated);
            println!("  pages:     {}", entry.pages);
            println!("  raw files: {}", entry.raw_files);
            println!();
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct Status {
    root: String,
    page_count: usize,
    types: BTreeMap<String, usize>,
    statuses: BTreeMap<String, usize>,
    raw_files: usize,
    last_log: String,
}

#[derive(Serialize)]
struct DirectoryEntry {
    tier: String,
    path: String,
    exists: bool,
    populated: bool,
    pages: usize,
    raw_files: usize,
}

fn resolve_overview_bundles(path: Option<&Path>, tier: &str) -> Result<Vec<(String, PathBuf)>> {
    if let Some(path) = path {
        return resolve_single_bundle(Some(path), "local")?.map_or_else(
            || bail!("bundle not found: {}", path.display()),
            |bundle| Ok(vec![bundle]),
        );
    }
    let bundles = resolve_bundles(tier)?;
    if bundles.is_empty() {
        bail!("no bundle found for tier '{tier}'");
    }
    Ok(bundles)
}

fn bundle_status(root: &Path) -> Result<Status> {
    let concepts = load_bundle(root)?;
    let pages = concepts
        .iter()
        .filter(|concept| !concept.is_reserved_file())
        .collect::<Vec<_>>();
    let mut types = BTreeMap::new();
    let mut statuses = BTreeMap::new();
    for page in &pages {
        *types.entry(page.type_tag()).or_insert(0) += 1;
        let status = {
            let status = page.frontmatter.text("status");
            if status.is_empty() {
                "active".to_owned()
            } else {
                status
            }
        };
        *statuses.entry(status).or_insert(0) += 1;
    }
    Ok(Status {
        root: root.display().to_string(),
        page_count: pages.len(),
        types,
        statuses,
        raw_files: raw_file_count(root),
        last_log: last_log_entry(root),
    })
}

fn directory_entry(tier: &str, root: &Path) -> Result<DirectoryEntry> {
    let concepts = load_bundle(root)?;
    let pages = concepts
        .iter()
        .filter(|concept| !concept.is_reserved_file())
        .count();
    let raw_files = raw_file_count(root);
    Ok(DirectoryEntry {
        tier: tier.to_owned(),
        path: root.canonicalize()?.display().to_string(),
        exists: root.is_dir(),
        populated: pages > 0 || raw_files > 0,
        pages,
        raw_files,
    })
}

fn raw_file_count(root: &Path) -> usize {
    fs::read_dir(root.join("raw"))
        .map(|entries| {
            entries
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.path().is_file())
                .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
                .count()
        })
        .unwrap_or_default()
}

fn last_log_entry(root: &Path) -> String {
    fs::read_to_string(root.join("log.md"))
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find_map(|line| line.trim().strip_prefix("- ").map(ToOwned::to_owned))
        })
        .unwrap_or_default()
}

fn format_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}
