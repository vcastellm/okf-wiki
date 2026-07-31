use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{
    bundle::{is_external, load_bundle},
    frontmatter::valid_iso8601,
    lint_relations::{lint_duplicate_sources, lint_duplicate_titles, lint_orphans},
    lint_timeline::lint_timeline,
    model::{Concept, FACTUAL_TYPES, LIFECYCLE_STATUSES, LintIssue, LintReport},
};

const MISUSED_FIELDS: &[(&str, &str)] = &[
    ("name", "title"),
    ("url", "resource"),
    ("updated", "timestamp"),
    ("date", "timestamp"),
    ("modified", "timestamp"),
    ("label", "title"),
];

#[derive(Clone, Copy, Debug)]
pub struct LintOptions {
    pub stale_days: i64,
    pub strict_frontmatter: bool,
}

impl Default for LintOptions {
    fn default() -> Self {
        Self {
            stale_days: 90,
            strict_frontmatter: false,
        }
    }
}

pub fn lint_bundle(root: &Path, options: LintOptions) -> Result<LintReport> {
    let concepts = load_bundle(root)?;
    let root = root.canonicalize()?;
    let known_pages = concepts
        .iter()
        .map(|concept| concept.rel.as_str())
        .collect::<BTreeSet<_>>();
    let incoming = incoming_links(&concepts);
    let mut validator = Validator {
        root: &root,
        known_pages,
        options,
        report: LintReport::default(),
    };
    for concept in &concepts {
        validator.check(concept);
    }
    lint_orphans(&concepts, &incoming, &mut validator.report);
    lint_duplicate_sources(&concepts, &mut validator.report);
    lint_duplicate_titles(&concepts, &mut validator.report);
    validator.report.pages = concepts
        .iter()
        .filter(|concept| !concept.is_reserved_file())
        .count();
    Ok(validator.report)
}

struct Validator<'a> {
    root: &'a Path,
    known_pages: BTreeSet<&'a str>,
    options: LintOptions,
    report: LintReport,
}

impl Validator<'_> {
    fn check(&mut self, concept: &Concept) {
        if !concept.is_reserved_file() {
            self.check_page_fields(concept);
            self.check_links(concept);
        }
        self.check_timestamp(concept);
        self.check_status(concept);
        if !concept.is_reserved_file() && FACTUAL_TYPES.contains(&concept.type_tag().as_str()) {
            self.check_sources(concept);
            self.check_staleness(concept);
            lint_timeline(concept, &mut self.report);
        }
    }

    fn check_page_fields(&mut self, concept: &Concept) {
        if concept.frontmatter.text("type").trim().is_empty() {
            self.error(concept, "missing-type", "no `type` field");
        }
        for (misused, intended) in MISUSED_FIELDS {
            if concept.frontmatter.get(misused).is_some() {
                self.error(
                    concept,
                    "reserved-field-misuse",
                    format!("uses `{misused}`; use `{intended}`"),
                );
            }
        }
        if self.options.strict_frontmatter {
            for (key, value) in concept.frontmatter.entries() {
                if matches!(value, crate::model::FrontmatterValue::List(values) if values.is_empty())
                    && !matches!(key.as_str(), "tags" | "sources")
                {
                    self.warning(
                        concept,
                        "suspicious-empty-list",
                        format!("field `{key}` is an empty list — may indicate yamlish parser couldn't parse the value"),
                    );
                }
            }
        }
    }

    fn check_links(&mut self, concept: &Concept) {
        for target in &concept.links {
            let exists = self.known_pages.contains(target.as_str())
                || self.root.join(target.trim_start_matches('/')).exists();
            if !exists {
                self.error(concept, "broken-link", format!("missing {target}"));
            }
        }
    }

    fn check_timestamp(&mut self, concept: &Concept) {
        let timestamp = concept.frontmatter.text("timestamp");
        if !timestamp.is_empty() && !valid_iso8601(&timestamp) {
            self.error(
                concept,
                "bad-timestamp",
                format!("not ISO 8601: {timestamp:?}"),
            );
        }
    }

    fn check_status(&mut self, concept: &Concept) {
        let status = concept.frontmatter.text("status");
        if !status.is_empty() && !LIFECYCLE_STATUSES.contains(&status.as_str()) {
            self.error(
                concept,
                "bad-status",
                format!(
                    "invalid status `{status}` (one of {})",
                    LIFECYCLE_STATUSES.join(", ")
                ),
            );
        }
    }

    fn check_sources(&mut self, concept: &Concept) {
        let Some(source_value) = concept.frontmatter.get("sources") else {
            self.warning(concept, "missing-sources", "factual page has no `sources`");
            return;
        };
        for source in source_value.as_list() {
            let source = source.trim();
            if !source.is_empty()
                && !is_external(source)
                && !self.root.join(source.trim_start_matches('/')).exists()
            {
                self.error(
                    concept,
                    "unresolvable-source",
                    format!("sources target does not exist: {source}"),
                );
            }
        }
    }

    fn check_staleness(&mut self, concept: &Concept) {
        if concept.frontmatter.text("confidence").to_lowercase() != "high" {
            return;
        }
        let timestamp = concept.frontmatter.text("timestamp");
        let Ok(timestamp) = DateTime::parse_from_rfc3339(&timestamp) else {
            return;
        };
        if (Utc::now() - timestamp.with_timezone(&Utc)).num_days() > self.options.stale_days {
            self.warning(
                concept,
                "stale-high-confidence",
                format!(
                    "high confidence but older than {}d",
                    self.options.stale_days
                ),
            );
        }
    }

    fn error(&mut self, concept: &Concept, rule: &str, message: impl Into<String>) {
        self.report.errors.push(issue(concept, rule, message));
    }

    fn warning(&mut self, concept: &Concept, rule: &str, message: impl Into<String>) {
        self.report.warnings.push(issue(concept, rule, message));
    }
}

fn incoming_links(concepts: &[Concept]) -> BTreeMap<String, usize> {
    let mut incoming = concepts
        .iter()
        .map(|concept| (concept.rel.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for concept in concepts
        .iter()
        .filter(|concept| !concept.is_reserved_file())
    {
        for target in &concept.links {
            if let Some(count) = incoming.get_mut(target) {
                *count += 1;
            }
        }
    }
    incoming
}

pub(crate) fn issue(concept: &Concept, rule: &str, message: impl Into<String>) -> LintIssue {
    LintIssue {
        file: concept.rel.clone(),
        rule: rule.to_owned(),
        message: message.into(),
        tier: None,
    }
}
