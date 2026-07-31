use anyhow::{Result, bail};

use crate::{
    bundle::{resolve_bundles, resolve_single_bundle},
    cli::{IndexArgs, LintArgs},
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
    model::LintReport,
};

pub(crate) fn index(args: IndexArgs) -> Result<()> {
    let report = rebuild_indexes(&args.bundle_dir, args.dry_run)?;
    for (path, pages) in &report.entries {
        let prefix = if args.dry_run { "[dry-run]" } else { "updated" };
        println!("{prefix} {} ({pages} pages)", path.display());
    }
    if !args.dry_run {
        println!("Rebuilt indexes. Changed: {}", report.changed);
    }
    Ok(())
}

pub(crate) fn lint(args: LintArgs) -> Result<()> {
    let bundles = resolve_lint_bundles(&args)?;
    let options = LintOptions {
        stale_days: args.stale_days,
        strict_frontmatter: args.strict_frontmatter,
    };
    let mut combined = LintReport::default();
    let paths = bundles
        .iter()
        .map(|(_, root)| root.display().to_string())
        .collect::<Vec<_>>();
    for (label, root) in bundles {
        let mut report = lint_bundle(&root, options)?;
        if args.location.tier.as_str() == "all" {
            for issue in report.errors.iter_mut().chain(report.warnings.iter_mut()) {
                issue.tier = Some(label.clone());
            }
        }
        combined.pages += report.pages;
        combined.errors.extend(report.errors);
        combined.warnings.extend(report.warnings);
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "bundle": paths,
                "pages": combined.pages,
                "errors": &combined.errors,
                "warnings": &combined.warnings,
                "ok": combined.errors.is_empty(),
            }))?
        );
    } else {
        println!("OKF lint: {}", paths.join(", "));
        println!("  concept pages: {}", combined.pages);
        println!("  errors:   {}", combined.errors.len());
        println!("  warnings: {}", combined.warnings.len());
        for issue in combined.errors.iter().chain(&combined.warnings) {
            let tag = if combined.errors.contains(issue) {
                "ERR"
            } else {
                "WARN"
            };
            let tier = issue
                .tier
                .as_deref()
                .map_or(String::new(), |tier| format!(" [{tier}]"));
            println!(
                "  [{tag}]{tier} {}  {}: {}",
                issue.file, issue.rule, issue.message
            );
        }
        println!(
            "  RESULT: {}",
            if combined.errors.is_empty() {
                "CONFORMANT"
            } else {
                "NON-CONFORMANT"
            }
        );
    }
    if combined.errors.is_empty() {
        Ok(())
    } else {
        bail!("lint found {} error(s)", combined.errors.len())
    }
}

fn resolve_lint_bundles(args: &LintArgs) -> Result<Vec<(String, std::path::PathBuf)>> {
    let explicit = args
        .location
        .bundle
        .as_deref()
        .or(args.bundle_dir.as_deref());
    match explicit {
        Some(path) => resolve_single_bundle(Some(path), "local")?.map_or_else(
            || bail!("bundle not found: {}", path.display()),
            |bundle| Ok(vec![bundle]),
        ),
        None => {
            let bundles = resolve_bundles(args.location.tier.as_str())?;
            if bundles.is_empty() {
                bail!("no bundle found for tier '{}'", args.location.tier.as_str());
            }
            Ok(bundles)
        }
    }
}
