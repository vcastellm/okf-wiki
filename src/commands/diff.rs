use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::cli::DiffArgs;

use super::support::{require_bundle, resolve_page};

pub(crate) fn run(args: DiffArgs) -> Result<()> {
    let root = require_bundle(&args.location)?;
    if !root.join(".git").is_dir() {
        bail!("no git repo at {} — diff requires git", root.display());
    }
    let page = args
        .page
        .as_deref()
        .map(|page| resolve_page(&root, page).map(|(_, relative)| relative))
        .transpose()?;
    let output = if let Some(since) = args.since.as_deref() {
        git_output(&root, ["diff", since], page.as_deref())?.into()
    } else if let Some(page) = page.as_deref() {
        page_history_diff(&root, page, args.previous)?
    } else {
        git_output(&root, ["log", "-1", "-p"], None)?.into()
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        println!("(no diff)");
    } else {
        print!("{stdout}");
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        eprint!("{stderr}");
    }
    Ok(())
}

fn page_history_diff(root: &std::path::Path, page: &str, previous: usize) -> Result<DiffOutput> {
    let log = git_output(
        root,
        ["log", &format!("-{previous}"), "--format=%H"],
        Some(page),
    )?;
    let log_stdout = String::from_utf8_lossy(&log.stdout);
    let hashes = log_stdout
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let Some(commit) = hashes.last() else {
        return Ok(DiffOutput::empty());
    };
    let parent = git_output(root, ["rev-parse", &format!("{commit}^")], None)?;
    if parent.status.success() {
        let parent = String::from_utf8_lossy(&parent.stdout).trim().to_owned();
        git_output(root, ["diff", &parent, commit], Some(page)).map(Into::into)
    } else {
        git_output(root, ["show", commit], Some(page)).map(Into::into)
    }
}

fn git_output<const N: usize>(
    root: &std::path::Path,
    arguments: [&str; N],
    page: Option<&str>,
) -> Result<std::process::Output> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(arguments);
    if let Some(page) = page {
        command.arg("--").arg(page);
    }
    command.output().context("could not run git")
}

#[derive(Debug)]
struct DiffOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl DiffOutput {
    const fn empty() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

impl From<std::process::Output> for DiffOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}
