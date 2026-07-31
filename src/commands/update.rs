use std::{
    fs,
    io::{self, Read},
};

use anyhow::{Context, Result, bail};

use crate::{
    cli::UpdateArgs,
    frontmatter::{
        atomic_write_text, bump_timestamp, now_iso, parse_frontmatter, render_document,
        require_nonempty,
    },
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
    model::FrontmatterValue,
    sections::{
        TimelineEntry, append_timeline_entry, ensure_timeline_section, format_timeline_entry,
        replace_section,
    },
};

use super::support::{add_and_commit, prepend_log, print_lint, require_bundle, resolve_page};

pub(crate) fn run(args: UpdateArgs, force_truth: bool) -> Result<()> {
    let truth = force_truth || args.truth;
    if truth && args.kind.is_some() {
        bail!("--truth and --kind are mutually exclusive; --truth implies kind=decision");
    }
    if args.kind.is_some() && args.summary.is_none() {
        bail!("--summary is required with --kind");
    }
    let root = require_bundle(&args.location)?;
    let (page, relative) = resolve_page(&root, &args.page)?;
    let timestamp = now_iso();
    let content_changed = truth || args.kind.is_some();
    if content_changed {
        update_page_content(&page, &args, truth, &timestamp)?;
        println!("Updated page content: {relative}");
    }
    if !content_changed && !args.no_timestamp {
        bump_timestamp(&page)?;
        println!("Bumped timestamp: {relative}");
    } else if !content_changed {
        println!("Timestamp unchanged: {relative}");
    }
    let message = args
        .message
        .unwrap_or_else(|| format!("update: {relative}"));
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

fn update_page_content(
    page: &std::path::Path,
    args: &UpdateArgs,
    truth: bool,
    timestamp: &str,
) -> Result<()> {
    let text =
        fs::read_to_string(page).with_context(|| format!("could not read {}", page.display()))?;
    let (mut frontmatter, body) = parse_frontmatter(&text);
    let body = ensure_timeline_section(body);
    let (body, kind, summary) = if truth {
        let new_body = stdin_body()?;
        let updated = replace_section(&body, "body", &new_body)?;
        (
            updated,
            "decision",
            args.summary.as_deref().unwrap_or("Rewrote page body"),
        )
    } else {
        let kind = args.kind.context("timeline kind is required")?.as_str();
        (
            body,
            kind,
            args.summary
                .as_deref()
                .context("timeline summary is required")?,
        )
    };
    let entry = format_timeline_entry(&TimelineEntry {
        time: timestamp,
        kind,
        summary,
        source: None,
        affects: &[],
    })?;
    frontmatter.set("timestamp", FrontmatterValue::Scalar(timestamp.to_owned()));
    atomic_write_text(
        page,
        &render_document(&frontmatter, &append_timeline_entry(&body, &entry)),
    )
}

fn stdin_body() -> Result<String> {
    let mut body = String::new();
    io::stdin().read_to_string(&mut body)?;
    let body = body.trim().to_owned();
    require_nonempty(
        &body,
        "--truth reads new body from stdin, but stdin was empty",
    )?;
    Ok(body)
}
