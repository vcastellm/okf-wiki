use std::collections::BTreeMap;

use crate::{
    lint::issue,
    model::{Concept, FACTUAL_TYPES, LintIssue, LintReport},
};

pub(crate) fn lint_orphans(
    concepts: &[Concept],
    incoming: &BTreeMap<String, usize>,
    report: &mut LintReport,
) {
    for concept in concepts {
        if !concept.is_reserved_file()
            && concept.rel != "/index.md"
            && concept.frontmatter.text("status") != "archived"
            && incoming.get(&concept.rel).copied().unwrap_or_default() == 0
        {
            report
                .warnings
                .push(issue(concept, "orphan", "no incoming links"));
        }
    }
}

pub(crate) fn lint_duplicate_sources(concepts: &[Concept], report: &mut LintReport) {
    let mut claims: BTreeMap<String, Vec<(&Concept, String)>> = BTreeMap::new();
    for concept in concepts {
        if concept.is_reserved_file() || !FACTUAL_TYPES.contains(&concept.type_tag().as_str()) {
            continue;
        }
        let Some(sources) = concept.frontmatter.get("sources") else {
            continue;
        };
        for source in sources.as_list() {
            if !source.is_empty() && !crate::bundle::is_external(&source) {
                claims
                    .entry(source)
                    .or_default()
                    .push((concept, concept.title_or_stem()));
            }
        }
    }
    for (source, claimants) in claims {
        let titles = claimants
            .iter()
            .map(|(_, title)| title)
            .collect::<std::collections::BTreeSet<_>>();
        if claimants.len() > 1 && titles.len() > 1 {
            report.errors.push(multi_page_issue(
                "duplicate-source-claim",
                format!(
                    "{} pages cite `{source}` with different titles: {}",
                    claimants.len(),
                    claimants
                        .iter()
                        .map(|(concept, _)| concept.rel.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
}

pub(crate) fn lint_duplicate_titles(concepts: &[Concept], report: &mut LintReport) {
    let mut titles: BTreeMap<String, Vec<&Concept>> = BTreeMap::new();
    for concept in concepts
        .iter()
        .filter(|concept| !concept.is_reserved_file())
    {
        let title = concept.frontmatter.text("title").trim().to_lowercase();
        if !title.is_empty() {
            let directory = concept
                .path
                .parent()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            titles
                .entry(format!("{directory}/{title}"))
                .or_default()
                .push(concept);
        }
    }
    for (title, claimants) in titles {
        if claimants.len() > 1 {
            report.warnings.push(multi_page_issue(
                "near-duplicate-title",
                format!(
                    "pages with similar title '{title}': {}",
                    claimants
                        .iter()
                        .map(|concept| concept.rel.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
}

fn multi_page_issue(rule: &str, message: String) -> LintIssue {
    LintIssue {
        file: "N/A (multiple pages)".to_owned(),
        rule: rule.to_owned(),
        message,
        tier: None,
    }
}
