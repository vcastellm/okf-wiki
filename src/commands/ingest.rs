use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    cli::IngestArgs,
    config::WikiConfig,
    frontmatter::{atomic_write_text, now_iso, slugify},
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
};

use super::support::{add_and_commit, prepend_log, print_lint, require_bundle};

pub(crate) fn run(args: IngestArgs) -> Result<()> {
    let root = require_bundle(&args.location)?;
    let config = WikiConfig::load(&root)?;
    let source = args
        .source
        .canonicalize()
        .with_context(|| format!("source not found: {}", args.source.display()))?;
    if !source.is_file() {
        bail!("source not found: {}", args.source.display());
    }
    let title = args.title.unwrap_or_else(|| title_from_source(&source));
    let slug = args.slug.unwrap_or_else(|| slugify(&title));
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .context("source file has no UTF-8 name")?;
    let raw_folder = config
        .folders()
        .raw()
        .first()
        .context("config raw folders must contain at least one folder")?;
    let raw_relative = format!("{}/{source_name}", raw_folder.as_str());
    let raw_destination = root.join(&raw_relative);
    let page_relative = format!("{}/{}.md", config.folders().sources().as_str(), slug);
    let page = root.join(&page_relative);
    if args.dry_run {
        println!(
            "[dry-run] copy {} → {}",
            source.display(),
            raw_destination.display()
        );
        println!("[dry-run] create {}", page.display());
        println!("[dry-run] title: {title}");
        println!("[dry-run] slug: {slug}");
        return Ok(());
    }
    fs::create_dir_all(root.join(raw_folder.as_str()))?;
    fs::create_dir_all(root.join(config.folders().sources().as_str()))?;
    fs::copy(&source, &raw_destination)
        .with_context(|| format!("could not copy {}", source.display()))?;
    let timestamp = now_iso();
    atomic_write_text(
        &page,
        &format!(
            "---\ntype: Source\ntitle: {title}\nsources: [{raw_relative}]\ntimestamp: {timestamp}\n---\n\n# {title}\n\n> Source: `{raw_relative}`\n\n_Skeleton page — read the source and fill in a summary._\n\n## timeline\n\n_(no entries yet)_\n"
        ),
    )?;
    println!("Copied  {}  →  {raw_relative}", source.display());
    println!("Created {page_relative}");
    prepend_log(
        &root,
        &timestamp,
        &format!("ingest: {slug} ({raw_relative})"),
    )?;
    println!("Updated log.md");
    let index = rebuild_indexes(&root, false)?;
    println!("Rebuilt indexes. Changed: {}", index.changed);
    if !args.no_lint {
        let report = lint_bundle(&root, LintOptions::default())?;
        print_lint(&report);
        if !report.errors.is_empty() {
            eprintln!("Lint errors found — fix them before committing.");
        }
    }
    add_and_commit(&root, &format!("ingest: {slug}"), !args.no_commit)?;
    println!("\nDone. Source: {raw_relative}  Page: {page_relative}");
    Ok(())
}

fn title_from_source(source: &Path) -> String {
    source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            stem.replace(['-', '_'], " ")
                .split_whitespace()
                .map(capitalize)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "Source".to_owned())
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
