use sha2::{Digest, Sha256};

const SKILL: &str = include_str!("../skills/okf-wiki/SKILL.md");
const SPEC: &str = include_str!("../skills/okf-wiki/references/okf-spec.md");
const TEMPLATE_GITIGNORE: &str = include_str!("../skills/okf-wiki/templates/.gitignore");

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
fn skill_file_has_the_expected_byte_hash() {
    let bytes = include_bytes!("../skills/okf-wiki/SKILL.md");

    let digest = Sha256::digest(bytes);

    assert_eq!(
        format!("{digest:x}"),
        "7833d10729b9ff990557229932a9fbd5232c7815c73f00c7ae96c69f1555fc8d"
    );
}
