use std::fs;

use okf_wiki::{
    bundle::{load_bundle, relative_target},
    frontmatter::{parse_frontmatter, render_document},
    model::FrontmatterValue,
    search::search_bundle,
};
use tempfile::tempdir;

#[test]
fn parses_and_renders_yamlish_block_lists() {
    // Given: a document using the supported yamlish subset.
    let input = "---\ntype: Note\nsources:\n  - raw/a.md\n  - raw/b.md\n---\n\nBody.\n";

    // When: its frontmatter is parsed and rendered.
    let (frontmatter, body) = parse_frontmatter(input);
    let rendered = render_document(&frontmatter, body);

    // Then: list values survive as an inline list and the body is preserved.
    assert_eq!(
        frontmatter.get("sources"),
        Some(&FrontmatterValue::List(vec![
            "raw/a.md".to_owned(),
            "raw/b.md".to_owned(),
        ]))
    );
    assert_eq!(
        rendered,
        "---\ntype: Note\nsources: [raw/a.md, raw/b.md]\n---\n\nBody.\n"
    );
}

#[test]
fn loads_only_visible_concepts_and_normalizes_relative_links() -> anyhow::Result<()> {
    // Given: a bundle with a visible page, raw content, and hidden content.
    let bundle = tempdir()?;
    fs::create_dir_all(bundle.path().join("notes"))?;
    fs::create_dir_all(bundle.path().join("raw"))?;
    fs::create_dir_all(bundle.path().join(".private"))?;
    fs::write(
        bundle.path().join("notes/page.md"),
        "---\ntype: Note\ntitle: Page\n---\n\n[Other](../notes/other.md)\n",
    )?;
    fs::write(bundle.path().join("raw/evidence.md"), "raw")?;
    fs::write(bundle.path().join(".private/secret.md"), "secret")?;

    // When: the bundle is loaded.
    let concepts = load_bundle(bundle.path())?;

    // Then: only visible concept markdown is loaded and its link is root-relative.
    assert_eq!(concepts.len(), 1);
    assert_eq!(concepts[0].rel, "/notes/page.md");
    assert_eq!(concepts[0].links, vec!["/notes/other.md"]);
    assert_eq!(
        relative_target("../../overview.md", "/entities/user/page.md"),
        "/overview.md"
    );
    Ok(())
}

#[test]
fn excludes_configured_raw_folders_from_loading_and_search() -> anyhow::Result<()> {
    // Given: a bundle with two configured raw roots and a custom managed page folder.
    let bundle = tempdir()?;
    fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nraw = [\"incoming\", \"research\"]\nnotes = \"pages\"\n",
    )?;
    fs::create_dir_all(bundle.path().join("incoming"))?;
    fs::create_dir_all(bundle.path().join("research"))?;
    fs::create_dir_all(bundle.path().join("pages"))?;
    fs::write(
        bundle.path().join("incoming/raw-one.md"),
        "---\ntype: Note\ntitle: Raw One\n---\n\nUniqueRawOne\n",
    )?;
    fs::write(
        bundle.path().join("research/raw-two.md"),
        "---\ntype: Note\ntitle: Raw Two\n---\n\nUniqueRawTwo\n",
    )?;
    fs::write(
        bundle.path().join("pages/kept.md"),
        "---\ntype: Note\ntitle: Managed Page\n---\n\nUniqueManaged\n",
    )?;

    // When: the shared read-side bundle and search APIs inspect the bundle.
    let concepts = load_bundle(bundle.path())?;
    let raw_results = search_bundle(bundle.path(), "UniqueRawOne UniqueRawTwo", 10, false)?;
    let managed_results = search_bundle(bundle.path(), "UniqueManaged", 10, false)?;

    // Then: both raw roots are excluded while the configured managed folder is loaded.
    assert_eq!(concepts.len(), 1);
    assert_eq!(concepts[0].rel, "/pages/kept.md");
    assert!(raw_results.is_empty());
    assert_eq!(managed_results.len(), 1);
    assert_eq!(managed_results[0].rel, "/pages/kept.md");
    Ok(())
}

#[test]
fn excludes_configured_ignored_roots_from_loading_and_search() -> anyhow::Result<()> {
    // Given: configured ignored roots with nested markdown descendants and one managed page.
    let bundle = tempdir()?;
    fs::write(
        bundle.path().join("okf-wiki.toml"),
        "[folders]\nignored = [\"scratch\", \"vendor\"]\n",
    )?;
    fs::create_dir_all(bundle.path().join("scratch/nested/deeper"))?;
    fs::create_dir_all(bundle.path().join("vendor/package/docs"))?;
    fs::create_dir_all(bundle.path().join("notes"))?;
    fs::write(
        bundle.path().join("scratch/nested/deeper/draft.md"),
        "---\ntype: Note\ntitle: Scratch Draft\n---\n\nUniqueScratchIgnored\n",
    )?;
    fs::write(
        bundle.path().join("vendor/package/docs/readme.md"),
        "---\ntype: Note\ntitle: Vendor Readme\n---\n\nUniqueVendorIgnored\n",
    )?;
    fs::write(
        bundle.path().join("notes/kept.md"),
        "---\ntype: Note\ntitle: Kept\n---\n\nUniqueManagedKept\n",
    )?;

    // When: the shared read-side bundle and search APIs inspect the bundle.
    let concepts = load_bundle(bundle.path())?;
    let ignored_results = search_bundle(
        bundle.path(),
        "UniqueScratchIgnored UniqueVendorIgnored",
        10,
        false,
    )?;
    let managed_results = search_bundle(bundle.path(), "UniqueManagedKept", 10, false)?;

    // Then: ignored root subtrees are absent while managed markdown remains searchable.
    assert_eq!(concepts.len(), 1);
    assert_eq!(concepts[0].rel, "/notes/kept.md");
    assert!(ignored_results.is_empty());
    assert_eq!(managed_results.len(), 1);
    assert_eq!(managed_results[0].rel, "/notes/kept.md");
    Ok(())
}

#[test]
fn ranks_title_matches_above_body_only_matches() -> anyhow::Result<()> {
    // Given: a pair of searchable pages with different field matches.
    let bundle = tempdir()?;
    fs::create_dir_all(bundle.path().join("notes"))?;
    fs::write(
        bundle.path().join("notes/auth.md"),
        "---\ntype: Note\ntitle: Authentication\n---\n\nOverview.\n",
    )?;
    fs::write(
        bundle.path().join("notes/other.md"),
        "---\ntype: Note\ntitle: Other\n---\n\nAuthentication appears in this body.\n",
    )?;

    // When: the shared search ranks the query.
    let results = search_bundle(bundle.path(), "authentication", 10, false)?;

    // Then: identity fields receive the expected higher score.
    assert_eq!(results[0].title, "Authentication");
    assert!(results[0].score > results[1].score);
    Ok(())
}
