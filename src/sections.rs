use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug)]
pub struct SectionRange {
    pub heading_start: usize,
    pub heading_end: usize,
    pub content_start: usize,
    pub content_end: usize,
}

#[derive(Clone, Debug)]
pub struct TimelineEntry<'a> {
    pub time: &'a str,
    pub kind: &'a str,
    pub summary: &'a str,
    pub source: Option<&'a str>,
    pub affects: &'a [String],
}

pub fn section_range(body: &str, name: &str) -> Option<SectionRange> {
    if name == "body" {
        let content_end = find_heading(body, "timeline").map_or(body.len(), |(start, _)| start);
        return Some(SectionRange {
            heading_start: 0,
            heading_end: 0,
            content_start: 0,
            content_end,
        });
    }
    let (heading_start, heading_end) = find_heading(body, name)?;
    let content_end = if name == "timeline" {
        body.len()
    } else {
        find_next_heading(body, heading_end).unwrap_or(body.len())
    };
    Some(SectionRange {
        heading_start,
        heading_end,
        content_start: heading_end,
        content_end,
    })
}

pub fn extract_section(body: &str, name: &str) -> Option<String> {
    let range = section_range(body, name)?;
    Some(
        body[range.content_start..range.content_end]
            .trim()
            .to_owned(),
    )
}

pub fn replace_section(body: &str, name: &str, new_content: &str) -> Result<String> {
    let range = section_range(body, name)
        .ok_or_else(|| anyhow::anyhow!("section `## {name}` not found in body"))?;
    let before = &body[..range.heading_end];
    let after = body[range.content_end..].trim_end();
    if after.is_empty() {
        Ok(format!("{before}\n\n{}\n", new_content.trim()))
    } else {
        Ok(format!("{before}\n\n{}\n\n{after}\n", new_content.trim()))
    }
}

pub fn append_to_section(body: &str, name: &str, text: &str) -> Result<String> {
    let range = section_range(body, name)
        .ok_or_else(|| anyhow::anyhow!("section `## {name}` not found in body"))?;
    let before = body[..range.content_end].trim_end();
    let after = body[range.content_end..].trim_start();
    Ok(format!("{before}\n\n{}\n{after}", text.trim()))
}

pub fn ensure_timeline_section(text: &str) -> String {
    if section_range(text, "timeline").is_some() {
        text.to_owned()
    } else {
        format!("{}\n\n## timeline\n\n_(no entries yet)_\n", text.trim_end())
    }
}

pub fn append_timeline_entry(text: &str, entry: &str) -> String {
    let Some(range) = section_range(text, "timeline") else {
        return format!("{}\n\n## timeline\n\n{entry}\n", text.trim_end());
    };
    let content = text[range.content_start..range.content_end].trim();
    if content == "_(no entries yet)_" {
        let before = &text[..range.content_start];
        let after = text[range.content_end..].trim_start();
        return format!("{before}\n\n{entry}\n{after}");
    }
    let before = text[..range.content_end].trim_end();
    let after = text[range.content_end..].trim_start();
    format!("{before}\n\n{entry}\n{after}")
}

pub fn format_timeline_entry(entry: &TimelineEntry<'_>) -> Result<String> {
    if !["decision", "evidence", "reversal", "note"].contains(&entry.kind) {
        bail!("unknown timeline kind: {}", entry.kind);
    }
    let mut lines = vec![
        format!("- time: {}", entry.time),
        format!("  kind: {}", entry.kind),
        format!("  summary: {}", yaml_scalar(entry.summary)),
    ];
    if let Some(source) = entry.source {
        lines.push(format!("  source: {}", yaml_scalar(source)));
    }
    if !entry.affects.is_empty() {
        lines.push(format!("  affects: [{}]", entry.affects.join(", ")));
    }
    Ok(lines.join("\n"))
}

pub fn yaml_scalar(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    if !value.starts_with(' ')
        && !value.ends_with(' ')
        && value.chars().all(|character| {
            character.is_alphanumeric() || matches!(character, ' ' | '_' | '.' | '/' | '\\' | '-')
        })
    {
        return value.to_owned();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn find_heading(body: &str, expected: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        let end = offset + line.len();
        let content = line.trim_end_matches(['\r', '\n']);
        if heading_name(content).is_some_and(|name| name == expected) {
            return Some((offset, offset + content.len()));
        }
        offset = end;
    }
    None
}

fn find_next_heading(body: &str, start: usize) -> Option<usize> {
    let mut offset = start;
    for line in body[start..].split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if heading_name(content).is_some() {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn heading_name(line: &str) -> Option<&str> {
    let after_hashes = line.strip_prefix("##")?;
    if !after_hashes.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    Some(after_hashes.trim())
}
