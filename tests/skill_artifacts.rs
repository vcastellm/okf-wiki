use std::{fs, process::Command};

use tempfile::tempdir;

const SKILL: &str = include_str!("../skills/okf-wiki/SKILL.md");
const SPEC: &str = include_str!("../skills/okf-wiki/references/okf-spec.md");
const TEMPLATE_GITIGNORE: &str = include_str!("../skills/okf-wiki/templates/.gitignore");

const BUNDLE_SCOPED_COMMANDS: &[&str] = &[
    "okf-wiki ingest",
    "okf-wiki update",
    "okf-wiki truth",
    "okf-wiki archive",
    "okf-wiki diff",
    "okf-wiki lint",
    "okf-wiki search",
    "okf-wiki status",
    "okf-wiki dir",
];

const WIRED_FILES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    ".cursor/rules/okf-wiki.md",
    ".github/copilot-instructions.md",
    ".windsurfrules",
];

#[test]
fn skill_uses_the_distributed_cli() {
    for command in [
        "okf-wiki init",
        "okf-wiki ingest",
        "okf-wiki update",
        "okf-wiki truth",
        "okf-wiki archive",
        "okf-wiki diff",
        "okf-wiki lint",
        "okf-wiki search",
        "okf-wiki status",
        "okf-wiki index",
        "okf-wiki now",
        "okf-wiki dir",
        "okf-wiki wire",
    ] {
        assert!(SKILL.contains(command), "missing command: {command}");
    }

    for (name, artifact) in [
        ("SKILL.md", SKILL),
        ("references/okf-spec.md", SPEC),
        ("templates/.gitignore", TEMPLATE_GITIGNORE),
    ] {
        for obsolete in ["python3", "scripts/okf_", ".py", "~/.local/bin", "`okf "] {
            assert!(
                !artifact.contains(obsolete),
                "obsolete command token in {name}: {obsolete}"
            );
        }
    }
}

#[test]
fn bundle_scoped_skill_examples_pin_the_current_bundle() {
    for command in BUNDLE_SCOPED_COMMANDS {
        assert!(
            SKILL
                .lines()
                .any(|line| line.contains(command) && line.contains("--bundle .")),
            "missing explicit bundle example for {command}"
        );
    }
}

#[test]
fn skill_artifacts_omit_obsolete_bundle_locations_and_tiers() {
    for (name, artifact) in [("SKILL.md", SKILL), ("references/okf-spec.md", SPEC)] {
        for obsolete in [".llm-wiki", "~/.llm-wiki", "--tier"] {
            assert!(
                !artifact.contains(obsolete),
                "obsolete bundle token in {name}: {obsolete}"
            );
        }
    }
}

#[test]
fn wire_all_writes_portable_bundle_guidance() -> anyhow::Result<()> {
    let workspace = tempdir()?;

    let output = Command::new(env!("CARGO_BIN_EXE_okf-wiki"))
        .args(["wire", "--agent", "all"])
        .current_dir(workspace.path())
        .output()?;

    assert!(
        output.status.success(),
        "wire failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for relative_path in WIRED_FILES {
        let artifact = fs::read_to_string(workspace.path().join(relative_path))?;

        assert!(
            !artifact.contains(".llm-wiki"),
            "obsolete bundle location in {relative_path}"
        );
        for required in ["okf-wiki search", "--bundle ."] {
            assert!(
                artifact.contains(required),
                "missing wire token in {relative_path}: {required}"
            );
        }
    }

    Ok(())
}
