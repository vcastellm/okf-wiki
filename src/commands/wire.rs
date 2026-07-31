use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    cli::{Agent, WireArgs},
    frontmatter::atomic_write_text,
};

const BEGIN: &str = "<!-- BEGIN okf -->";
const END: &str = "<!-- END okf -->";

pub(crate) fn run(args: WireArgs) -> Result<()> {
    let agents = expanded_agents(&args.agent);
    for agent in agents {
        let target = target(agent);
        let action = apply_block(Path::new(target), block(agent))?;
        println!("[{}] {action}: {target}", agent_name(agent));
    }
    Ok(())
}

fn expanded_agents(requested: &[Agent]) -> BTreeSet<Agent> {
    let mut agents = BTreeSet::new();
    for agent in requested {
        match agent {
            Agent::All => agents.extend([
                Agent::Claude,
                Agent::Codex,
                Agent::Cursor,
                Agent::Copilot,
                Agent::Windsurf,
            ]),
            agent => {
                agents.insert(*agent);
            }
        }
    }
    agents
}

fn target(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "CLAUDE.md",
        Agent::Codex => "AGENTS.md",
        Agent::Cursor => ".cursor/rules/okf-wiki.md",
        Agent::Copilot => ".github/copilot-instructions.md",
        Agent::Windsurf => ".windsurfrules",
        Agent::All => "",
    }
}

fn agent_name(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
        Agent::Cursor => "cursor",
        Agent::Copilot => "copilot",
        Agent::Windsurf => "windsurf",
        Agent::All => "all",
    }
}

fn block(agent: Agent) -> &'static str {
    match agent {
        Agent::Cursor => {
            "---\ndescription: Persistent markdown knowledge base (OKF) for project memory\nglobs: **/*\nalwaysApply: false\n---\n\n<!-- BEGIN okf -->\nAlways consult `.llm-wiki/index.md` before answering questions about project architecture, design decisions, entities, or domain concepts.\n\nScripts: `okf-wiki search`, `okf-wiki ingest`, `okf-wiki update`, `okf-wiki diff`, `okf-wiki status`, `okf-wiki init`, `okf-wiki lint`, `okf-wiki index`\n<!-- END okf -->"
        }
        Agent::Copilot => {
            "<!-- BEGIN okf -->\n## Project Memory (okf-wiki)\n\nThis project uses `okf-wiki` for persistent knowledge. Always check `.llm-wiki/index.md` first before answering questions about architecture, design decisions, entities, or domain concepts.\n<!-- END okf -->"
        }
        Agent::Windsurf => {
            "<!-- BEGIN okf -->\nBefore answering questions about project architecture, design decisions, entities, or domain concepts, consult `.llm-wiki/index.md` first. Follow index to subsection index to concept page. Cite paths. Never skip the wiki.\n<!-- END okf -->"
        }
        Agent::Claude | Agent::Codex => {
            "<!-- BEGIN okf -->\n## Project Memory (okf-wiki)\n\nThis project uses `okf-wiki` for persistent knowledge. Consult `.llm-wiki/index.md` before answering questions about project architecture, design decisions, entities, or domain concepts. See skills/okf-wiki/SKILL.md for the full protocol.\n<!-- END okf -->"
        }
        Agent::All => "",
    }
}

fn apply_block(path: &Path, block: &str) -> Result<&'static str> {
    if !path.exists() {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        atomic_write_text(path, &format!("{block}\n"))?;
        return Ok("created");
    }
    let current =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let updated = replace_existing(&current, block);
    if updated == current {
        return Ok("no change (identical)");
    }
    let action = if current.contains(BEGIN) && current.contains(END) {
        "replaced existing block"
    } else {
        "appended"
    };
    atomic_write_text(path, &updated)?;
    Ok(action)
}

fn replace_existing(current: &str, block: &str) -> String {
    let Some(start) = current.find(BEGIN) else {
        let separator = if current.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return format!("{current}{separator}{block}\n");
    };
    let after_start = &current[start..];
    let Some(end_offset) = after_start.find(END) else {
        let separator = if current.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return format!("{current}{separator}{block}\n");
    };
    let end = start + end_offset + END.len();
    format!("{}{block}{}", &current[..start], &current[end..])
}
