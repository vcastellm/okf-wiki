use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Result, bail};

use crate::{
    bundle::{load_bundle, resolve_bundles, resolve_single_bundle},
    cli::SearchArgs,
    model::SearchResult,
    search::search_bundle,
};

pub(crate) fn run(args: SearchArgs) -> Result<()> {
    let query = args.query.join(" ");
    if query.trim().is_empty() && !args.toc {
        bail!("query required (or use --toc)");
    }
    let bundles = resolve_search_bundles(&args)?;
    if args.toc {
        return print_toc(
            &bundles,
            args.include_archived,
            args.location.tier.as_str() == "all",
        );
    }
    let mut results = Vec::new();
    for (tier, root) in &bundles {
        for result in search_bundle(root, &query, args.max_results, args.include_archived)? {
            results.push((tier.clone(), result));
        }
    }
    if args.location.tier.as_str() == "all" {
        results.sort_by(|left, right| {
            right
                .1
                .score
                .cmp(&left.1.score)
                .then_with(|| left.1.rel.cmp(&right.1.rel))
        });
        results.truncate(args.max_results);
    }
    if args.json {
        print_json(&query, args.location.tier.as_str(), &results)?;
    } else {
        print_text(&query, args.location.tier.as_str(), &results);
    }
    Ok(())
}

fn resolve_search_bundles(args: &SearchArgs) -> Result<Vec<(String, PathBuf)>> {
    if let Some(path) = args.location.bundle.as_deref() {
        return resolve_single_bundle(Some(path), "local")?.map_or_else(
            || bail!("bundle not found: {}", path.display()),
            |bundle| Ok(vec![bundle]),
        );
    }
    let bundles = resolve_bundles(args.location.tier.as_str())?;
    if bundles.is_empty() {
        bail!("no bundle found for tier '{}'", args.location.tier.as_str());
    }
    Ok(bundles)
}

fn print_text(query: &str, tier: &str, results: &[(String, SearchResult)]) {
    println!("Search: {query:?}");
    println!("  tier: {tier}  results: {}\n", results.len());
    for (index, (result_tier, result)) in results.iter().enumerate() {
        let tag = if tier == "all" {
            format!(" [{result_tier}]")
        } else {
            String::new()
        };
        println!(
            "{}. {}{}  ({})  score={}",
            index + 1,
            result.title,
            tag,
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

fn print_json(query: &str, tier: &str, results: &[(String, SearchResult)]) -> Result<()> {
    let rows = results
        .iter()
        .map(|(result_tier, result)| {
            let mut row = serde_json::to_value(result)?;
            if tier == "all" {
                row["tier"] = serde_json::Value::String(result_tier.clone());
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    let count = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"query": query, "tier": tier, "results": rows, "count": count})
        )?
    );
    Ok(())
}

fn print_toc(
    bundles: &[(String, PathBuf)],
    include_archived: bool,
    include_tier: bool,
) -> Result<()> {
    for (tier, root) in bundles {
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
                let prefix = if include_tier {
                    format!("[{tier}] ")
                } else {
                    String::new()
                };
                println!("{prefix}[{page_type}] {title}  ({rel})");
            }
        }
    }
    Ok(())
}
