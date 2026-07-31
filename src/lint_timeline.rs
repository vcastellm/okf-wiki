use crate::{
    lint::issue,
    model::{Concept, LintReport, TIMELINE_KINDS},
    sections::extract_section,
};

pub(crate) fn lint_timeline(concept: &Concept, report: &mut LintReport) {
    let Some(timeline) = extract_section(&concept.body, "timeline") else {
        return;
    };
    if timeline.is_empty() || timeline == "_(no entries yet)_" {
        return;
    }
    let entries = parse_entries(&timeline);
    for (index, (time, kind, summary)) in entries.iter().enumerate() {
        if !kind.is_empty() && !TIMELINE_KINDS.contains(&kind.as_str()) {
            report.errors.push(issue(
                concept,
                "bad-timeline-kind",
                format!("unknown timeline kind `{kind}`"),
            ));
        }
        let missing = [("time", time), ("kind", kind), ("summary", summary)]
            .into_iter()
            .filter_map(|(name, value)| value.is_empty().then_some(name))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            report.warnings.push(issue(
                concept,
                "timeline-malformed",
                format!(
                    "entry {} missing field(s): {}",
                    index + 1,
                    missing.join(", ")
                ),
            ));
        }
    }
    for pair in entries.windows(2) {
        if !pair[0].0.is_empty() && !pair[1].0.is_empty() && pair[1].0 < pair[0].0 {
            report.warnings.push(issue(
                concept,
                "timeline-out-of-order",
                "timeline entries are not chronological",
            ));
            return;
        }
    }
}

fn parse_entries(timeline: &str) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    for line in timeline.lines() {
        if let Some(time) = line.strip_prefix("- time:") {
            entries.push((time.trim().to_owned(), String::new(), String::new()));
        } else if let Some(kind) = line.strip_prefix("  kind:")
            && let Some(entry) = entries.last_mut()
        {
            entry.1 = kind.trim().to_owned();
        } else if let Some(summary) = line.strip_prefix("  summary:")
            && let Some(entry) = entries.last_mut()
        {
            entry.2 = summary.trim().to_owned();
        }
    }
    entries
}
