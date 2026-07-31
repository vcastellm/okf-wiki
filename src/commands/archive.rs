use std::fs;

use anyhow::{Context, Result};

use crate::{
    cli::ArchiveArgs,
    frontmatter::{atomic_write_text, now_iso, parse_frontmatter, render_document},
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
    model::FrontmatterValue,
    sections::{
        TimelineEntry, append_timeline_entry, ensure_timeline_section, format_timeline_entry,
    },
};

use super::support::{add_and_commit, prepend_log, print_lint, require_bundle, resolve_page};

pub(crate) fn run(args: ArchiveArgs) -> Result<()> {
    let root = require_bundle(&args.location)?;
    let (page, relative) = resolve_page(&root, &args.page)?;
    let timestamp = now_iso();
    archive_page(&page, args.reversal_summary.as_deref(), &timestamp)?;
    println!("Archived: {relative}");
    let message = args
        .message
        .unwrap_or_else(|| format!("archive: {relative}"));
    prepend_log(&root, &timestamp, &message)?;
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
    add_and_commit(&root, &message, !args.no_commit)
}

fn archive_page(
    page: &std::path::Path,
    reversal_summary: Option<&str>,
    timestamp: &str,
) -> Result<()> {
    let text =
        fs::read_to_string(page).with_context(|| format!("could not read {}", page.display()))?;
    let (mut frontmatter, body) = parse_frontmatter(&text);
    let mut body = ensure_timeline_section(body);
    if let Some(summary) = reversal_summary {
        let entry = format_timeline_entry(&TimelineEntry {
            time: timestamp,
            kind: "reversal",
            summary,
            source: None,
            affects: &[],
        })?;
        body = append_timeline_entry(&body, &entry);
    }
    frontmatter.set("status", FrontmatterValue::Scalar("archived".to_owned()));
    frontmatter.set("timestamp", FrontmatterValue::Scalar(timestamp.to_owned()));
    atomic_write_text(page, &render_document(&frontmatter, &body))
}
