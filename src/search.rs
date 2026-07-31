use std::path::Path;

use anyhow::Result;

use crate::{bundle::load_bundle, model::SearchResult};

const STOPWORDS: &[&str] = &[
    "and", "the", "this", "that", "with", "from", "have", "been", "were", "they", "their", "them",
    "will", "would", "could", "should", "about", "there", "which", "what", "when", "where", "than",
    "then", "also", "just", "more", "some", "such", "only", "other", "into", "over", "very",
    "after", "before", "because", "between", "through", "during", "without", "within", "along",
    "these", "those", "does", "being", "its",
];

pub fn tokenize_query(query: &str) -> Vec<String> {
    let normalized = query.chars().fold(String::new(), |mut text, character| {
        if character.is_alphanumeric() {
            text.extend(character.to_lowercase());
        } else {
            text.push(' ');
        }
        text
    });
    let mut tokens = Vec::new();
    for term in normalized.split_whitespace() {
        if term.chars().count() >= 2
            && !STOPWORDS.contains(&term)
            && !tokens.iter().any(|seen| seen == term)
        {
            tokens.push(term.to_owned());
        }
    }
    tokens
}

pub fn search_bundle(
    root: &Path,
    query: &str,
    max_results: usize,
    include_archived: bool,
) -> Result<Vec<SearchResult>> {
    let terms = tokenize_query(query);
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let mut results = load_bundle(root)?
        .into_iter()
        .filter(|concept| !concept.is_reserved_file())
        .filter(|concept| include_archived || concept.frontmatter.text("status") != "archived")
        .filter_map(|concept| scored_result(concept, &terms))
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.rel.cmp(&right.rel))
    });
    results.truncate(max_results);
    Ok(results)
}

pub fn term_hits(text: &str, terms: &[String]) -> i32 {
    let normalized = text.to_lowercase();
    terms
        .iter()
        .filter(|term| normalized.contains(term.as_str()))
        .count() as i32
}

fn scored_result(concept: crate::model::Concept, terms: &[String]) -> Option<SearchResult> {
    let metadata_weights = [
        ("aliases", 5),
        ("description", 4),
        ("tags", 3),
        ("type", 2),
        ("category", 2),
        ("domain", 2),
    ];
    let mut score = term_hits(&concept.frontmatter.text("title"), terms) * 6;
    score += term_hits(&concept.rel, terms) * 4;
    for (field, weight) in metadata_weights {
        score += term_hits(&concept.frontmatter.text(field), terms) * weight;
    }
    score += term_hits(&concept.body, terms);
    if score == 0 {
        return None;
    }
    let trimmed_body = concept.body.trim();
    let mut preview: String = trimmed_body.chars().take(200).collect();
    if trimmed_body.chars().count() > 200 {
        preview.push('…');
    }
    let title = concept.title_or_stem();
    let page_type = concept.type_tag();
    let description = concept.frontmatter.text("description");
    let path = concept.path.display().to_string();
    Some(SearchResult {
        rel: concept.rel,
        title,
        page_type,
        description,
        preview: preview.replace('\n', " "),
        score,
        path,
    })
}
