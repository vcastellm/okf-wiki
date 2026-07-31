use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, bail};

use crate::{bundle::load_bundle, cli::SearchArgs, model::SearchResult, search::search_bundle};

use super::support::require_bundle;

pub(crate) fn run(args: SearchArgs) -> Result<()> {
    let query = args.query.join(" ");
    if query.trim().is_empty() && !args.toc {
        bail!("query required (or use --toc)");
    }
    let root = require_bundle(&args.location)?;
    if args.toc {
        return print_toc(&root, args.include_archived);
    }
    let results = search_bundle(&root, &query, args.max_results, args.include_archived)?;
    if args.json {
        print_json(&query, &root, &results)?;
    } else {
        print_text(&query, &root, &results);
    }
    Ok(())
}

fn print_text(query: &str, root: &Path, results: &[SearchResult]) {
    println!("Search: {query:?}");
    println!("  bundle: {}  results: {}\n", root.display(), results.len());
    for (index, result) in results.iter().enumerate() {
        println!(
            "{}. {}  ({})  score={}",
            index + 1,
            result.title,
            result.page_type,
            result.score
        );
        println!("   {}", result.rel);
        if !result.description.is_empty() {
            println!("   {}", result.description);
        }
        if !result.preview.is_empty() {
            println!(
                "   {}",
                result.preview.chars().take(120).collect::<String>()
            );
        }
        println!();
    }
}

fn print_json(query: &str, root: &Path, results: &[SearchResult]) -> Result<()> {
    let rows = results
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let count = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "query": query,
            "bundle": root.display().to_string(),
            "results": rows,
            "count": count,
        }))?
    );
    Ok(())
}

fn print_toc(root: &Path, include_archived: bool) -> Result<()> {
    let mut pages: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for concept in load_bundle(root)? {
        if concept.is_reserved_file()
            || (!include_archived && concept.frontmatter.text("status") == "archived")
        {
            continue;
        }
        pages
            .entry(concept.type_tag())
            .or_default()
            .push((concept.title_or_stem(), concept.rel));
    }
    for (page_type, pages) in &mut pages {
        pages.sort_by_key(|(title, _)| title.to_lowercase());
        for (title, rel) in pages {
            println!("[{page_type}] {title}  ({rel})");
        }
    }
    Ok(())
}
