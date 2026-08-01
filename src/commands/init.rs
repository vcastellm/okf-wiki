use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result};

use crate::{
    cli::InitArgs,
    config::{FolderName, WikiConfig},
    frontmatter::now_iso,
    model::{AUTO_INDEX_CLOSE, AUTO_INDEX_MARKER},
};

pub(crate) fn run(mut args: InitArgs) -> Result<()> {
    let root = absolute_path(&args.bundle_dir)?;
    let config = WikiConfig::load(&root)?;
    let description = if args.from_readme {
        let (title, description) = infer_readme(&absolute_path(&args.readme_path)?);
        if let Some(title) = title {
            args.title = title;
        }
        description
    } else {
        None
    };
    let timestamp = now_iso();
    fs::create_dir_all(&root)?;
    for directory in config.folders().raw() {
        fs::create_dir_all(root.join(directory.as_str()))?;
    }
    let managed_folders = managed_folders(&config);
    for (_, directory) in managed_folders {
        fs::create_dir_all(root.join(directory.as_str()))?;
    }
    for (_, directory) in managed_folders {
        let title = title_case(directory.as_str());
        write_if_absent(
            &root.join(directory.as_str()).join("index.md"),
            &format!(
                "---\ntype: Index\ntitle: {title}\ntimestamp: {timestamp}\n---\n\n# {title}\n\n{AUTO_INDEX_MARKER}\n{AUTO_INDEX_CLOSE}\n"
            ),
        )?;
    }
    let description_line = description
        .as_ref()
        .map_or(String::new(), |value| format!("\n> {value}\n"));
    write_if_absent(
        &root.join("index.md"),
        &format!(
            "---\ntype: Index\ntitle: {}\ntimestamp: {timestamp}\n---\n\n# {}\n{description_line}\nSections:\n\n{}\n\nDrop source files into `{}/`, then run INGEST.\n",
            args.title,
            args.title,
            root_section_links(managed_folders),
            config
                .folders()
                .raw()
                .first()
                .context("config raw folders must contain at least one folder")?
                .as_str()
        ),
    )?;
    write_if_absent(
        &root.join("log.md"),
        &format!("# Log\n\n- {timestamp} — bundle initialized.\n"),
    )?;
    for directory in config.folders().raw() {
        fs::write(root.join(directory.as_str()).join(".gitkeep"), "")?;
    }
    println!("Initialized OKF wiki at {}", root.display());
    println!("  title: {}", args.title);
    if let Some(description) = description {
        println!(
            "  description: {}",
            description.chars().take(80).collect::<String>()
        );
    }
    initialize_git(&root, args.no_git)?;
    Ok(())
}

fn managed_folders(config: &WikiConfig) -> [(&'static str, &FolderName); 4] {
    [
        ("Notes", config.folders().notes()),
        ("Sources", config.folders().sources()),
        ("Entities", config.folders().entities()),
        ("Concepts", config.folders().concepts()),
    ]
}

fn root_section_links(folders: [(&str, &FolderName); 4]) -> String {
    folders
        .into_iter()
        .map(|(label, folder)| format!("- [{label}](/{}/index.md)", folder.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn initialize_git(root: &Path, disabled: bool) -> Result<()> {
    if disabled || root.join(".git").is_dir() {
        return Ok(());
    }
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status();
    let Ok(init) = init else {
        return Ok(());
    };
    if !init.success() {
        return Ok(());
    }
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(root)
        .status()?;
    if add.success() {
        let _status = Command::new("git")
            .args(["commit", "-q", "-m", "init OKF wiki"])
            .current_dir(root)
            .status()?;
    }
    Ok(())
}

fn absolute_path(path: &Path) -> Result<std::path::PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn write_if_absent(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        fs::write(path, content).with_context(|| format!("could not write {}", path.display()))?;
    }
    Ok(())
}

fn infer_readme(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (None, None);
    };
    let mut title = None;
    for line in text.lines() {
        let line = line.trim();
        if title.is_none() && line.starts_with("# ") && !line.starts_with("## ") {
            title = Some(line[2..].trim().to_owned());
            continue;
        }
        if title.is_some()
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with("[!")
            && !line.starts_with("![")
            && !line.starts_with('<')
        {
            let description: String = line.trim_end_matches('.').chars().take(200).collect();
            if description.len() > 20 {
                return (title, Some(description));
            }
            return (title, None);
        }
    }
    (title, None)
}

fn title_case(value: &str) -> String {
    value
        .split(['-', '_'])
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), characters.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
