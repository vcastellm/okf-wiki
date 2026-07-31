use anyhow::{Result, bail};

use crate::{
    cli::{IndexArgs, LintArgs},
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
};

use super::support::require_bundle;

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
    let root = require_bundle(&args.location)?;
    let options = LintOptions {
        stale_days: args.stale_days,
        strict_frontmatter: args.strict_frontmatter,
    };
    let combined = lint_bundle(&root, options)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "bundle": root.display().to_string(),
                "pages": combined.pages,
                "errors": &combined.errors,
                "warnings": &combined.warnings,
                "ok": combined.errors.is_empty(),
            }))?
        );
    } else {
        println!("OKF lint: {}", root.display());
        println!("  concept pages: {}", combined.pages);
        println!("  errors:   {}", combined.errors.len());
        println!("  warnings: {}", combined.warnings.len());
        for issue in &combined.errors {
            println!("  [ERR] {}  {}: {}", issue.file, issue.rule, issue.message);
        }
        for issue in &combined.warnings {
            println!("  [WARN] {}  {}: {}", issue.file, issue.rule, issue.message);
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
