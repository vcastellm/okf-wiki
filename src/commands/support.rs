use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::{
    bundle::resolve_single_bundle, cli::BundleArgs, frontmatter::atomic_write_text,
    model::LintReport,
};

pub(crate) fn require_bundle(location: &BundleArgs) -> Result<PathBuf> {
    let resolved = resolve_single_bundle(location.bundle.as_deref(), location.tier.as_str())?;
    match resolved {
        Some((_, root)) => Ok(root),
        None => bail!("bundle not found (tier: {})", location.tier.as_str()),
    }
}

pub(crate) fn resolve_page(root: &Path, page: &Path) -> Result<(PathBuf, String)> {
    let path = if page.is_absolute() {
        page.to_owned()
    } else {
        root.join(page)
    };
    if !path.is_file() {
        bail!("page not found: {}", page.display());
    }
    let path = path.canonicalize()?;
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("page is outside bundle: {}", path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    Ok((path, relative))
}

pub(crate) fn prepend_log(root: &Path, timestamp: &str, message: &str) -> Result<()> {
    let log_path = root.join("log.md");
    let content = fs::read_to_string(&log_path).unwrap_or_else(|_| "# Log\n\n".to_owned());
    let entry = format!("- {timestamp} — {message}\n");
    let updated = if let Some((header, rest)) = content.split_once('\n') {
        if header == "# Log" {
            format!("{header}\n\n{entry}{rest}")
        } else {
            format!("# Log\n\n{entry}{content}")
        }
    } else {
        format!("# Log\n\n{entry}{content}")
    };
    atomic_write_text(&log_path, &updated)
}

pub(crate) fn print_lint(report: &LintReport) {
    println!("  errors:   {}", report.errors.len());
    println!("  warnings: {}", report.warnings.len());
    for issue in report.errors.iter().chain(&report.warnings) {
        println!("  {}  {}: {}", issue.file, issue.rule, issue.message);
    }
}

pub(crate) fn add_and_commit(root: &Path, message: &str, commit_enabled: bool) -> Result<()> {
    if !commit_enabled || !root.join(".git").is_dir() {
        return Ok(());
    }
    let add = Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref(), "add", "-A"])
        .status()
        .context("could not run git add")?;
    if !add.success() {
        eprintln!("(git add skipped: exit {})", add);
        return Ok(());
    }
    let commit = Command::new("git")
        .args([
            "-C",
            root.to_string_lossy().as_ref(),
            "commit",
            "-m",
            message,
        ])
        .status()
        .context("could not run git commit")?;
    if commit.success() {
        println!("Committed: {message}");
    }
    Ok(())
}
