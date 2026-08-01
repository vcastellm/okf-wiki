use std::fs;

use okf_wiki::{
    indexer::rebuild_indexes,
    lint::{LintOptions, lint_bundle},
    model::{AUTO_INDEX_CLOSE, AUTO_INDEX_MARKER},
};
use tempfile::tempdir;

#[test]
fn index_replaces_marker_block_and_lint_reports_broken_links() -> anyhow::Result<()> {
    // Given: a concept page whose existing directory index contains legacy generated content.
    let bundle = tempdir()?;
    fs::create_dir_all(bundle.path().join("notes"))?;
    fs::write(
        bundle.path().join("notes/index.md"),
        format!("---\ntype: Index\ntitle: Notes\n---\n\n# Notes\n\n{AUTO_INDEX_MARKER}\nold\n"),
    )?;
    fs::write(
        bundle.path().join("notes/architecture.md"),
        "---\ntype: Note\ntitle: Architecture\ntimestamp: 2026-07-03T00:00:00Z\n---\n\n[Missing](/notes/missing.md)\n",
    )?;

    // When: indexes are rebuilt and the bundle is linted.
    let report = rebuild_indexes(bundle.path(), false)?;
    let lint = lint_bundle(bundle.path(), LintOptions::default())?;
    let index = fs::read_to_string(bundle.path().join("notes/index.md"))?;

    // Then: the index is migrated and the unresolved cross-link remains an error.
    assert_eq!(report.changed, 1);
    assert!(index.contains(AUTO_INDEX_MARKER));
    assert!(index.contains(AUTO_INDEX_CLOSE));
    assert!(index.contains("[Architecture](/notes/architecture.md)"));
    assert!(lint.errors.iter().any(|issue| issue.rule == "broken-link"));
    Ok(())
}

#[test]
fn index_and_lint_ignore_configured_raw_roots() -> anyhow::Result<()> {
    // Given: configured raw roots containing invalid markdown and one managed page.
    let bundle = tempdir()?;
    fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nraw = [\"incoming\", \"research\"]\n",
    )?;
    fs::create_dir_all(bundle.path().join("incoming"))?;
    fs::create_dir_all(bundle.path().join("research"))?;
    fs::create_dir_all(bundle.path().join("notes"))?;
    fs::write(
        bundle.path().join("incoming/bad.md"),
        "raw without frontmatter",
    )?;
    fs::write(
        bundle.path().join("research/bad.md"),
        "---\ntype: Note\n---\n\n[Missing](/missing.md)\n",
    )?;
    fs::write(
        bundle.path().join("notes/page.md"),
        "---\ntype: Note\ntitle: Page\ntimestamp: 2026-07-03T00:00:00Z\n---\n\nBody.\n",
    )?;

    // When: indexes are rebuilt and lint runs through their shared bundle reader.
    let report = rebuild_indexes(bundle.path(), false)?;
    let lint = lint_bundle(bundle.path(), LintOptions::default())?;

    // Then: raw markdown is absent from generated indexes and lint accounting.
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].0, bundle.path().join("notes/index.md"));
    assert_eq!(lint.pages, 1);
    assert!(lint.errors.iter().all(
        |issue| !issue.file.starts_with("/incoming/") && !issue.file.starts_with("/research/")
    ));
    Ok(())
}
