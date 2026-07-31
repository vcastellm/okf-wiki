---
name: okf-wiki
description: >-
  Persistent markdown knowledge base (OKF format) for LLM project memory.
  Use whenever the user or project has documented facts — architecture,
  design decisions, domain entities, datasets — that must be consulted
  before answering, even if the user doesn't mention the wiki explicitly.
  Also use when ingesting raw documents into a project or personal wiki,
  updating pages after a source changes, or linting a bundle. Trigger on
  phrases like "remember this", "what do we know about X", "add to the
  wiki", "update the wiki", "what did we decide about...", or any question
  about prior project decisions or recorded knowledge.
---

# LLM Wiki (OKF-light)

Persistent markdown knowledge base with two layers:
- `raw/` — immutable source documents. Read only.
- `*.md` — LLM-generated concept pages, one idea per file.

Full format reference: [references/okf-spec.md](references/okf-spec.md).

## When to use

- The user asks about recorded facts: the project, its architecture, domain entities, tools, or prior decisions.
- You are about to ingest a raw source into a wiki bundle.
- You need to update wiki pages after a raw source changes.
- You need to lint a wiki for format or link errors.

## Query-first (non-negotiable)

**`index.md` is the mandatory entry point for every query — no exceptions.** Never answer a
memory question from raw files, project files, or external sources before the wiki has been
consulted through its index.

1. Determine the bundle root. The current working directory is the bundle for every invocation.
   Run commands from that directory and pass `--bundle .` explicitly. If the intended bundle
   directory is unknown, determine it before invoking `okf-wiki`.
2. **Read `index.md` first. Always.** Even if it looks short, or its `<!-- okf:auto-index -->`
   section is empty.
3. **Follow the section links listed in `index.md`** — `/notes/index.md`, `/sources/index.md`,
   `/entities/index.md`, `/concepts/index.md`. Subsection indexes list the actual pages.
4. If a topic isn't surfaced by the indexes, use **`okf-wiki search <query> --bundle .`** for ranked
   results across concept pages before falling back to raw grep.
5. Cite by bundle-relative path: `per /notes/project-overview.md`.
6. Only after the wiki is silent or confidence is low, fall back to `raw/` then external sources.

Common failure modes to avoid:
- Skipping the wiki and reading project / raw files directly.
- Reading only the top-level `index.md`, seeing an empty auto-index, and concluding the wiki is empty.
- Answering from memory of a prior turn instead of re-reading the cited page.
- Grepping the bundle without running `okf-wiki search <query> --bundle .` first — ranked search uses weighted scoring,
  not linear scan.

## CLI

Use the distributed `okf-wiki` command for all operations:

```bash
okf-wiki init .
okf-wiki ingest <source> --bundle .
okf-wiki update <page> --bundle .
okf-wiki truth <page> --bundle .
okf-wiki archive <page> --bundle .
okf-wiki diff <page> --bundle .
okf-wiki lint --bundle .
okf-wiki search <query> --bundle .
okf-wiki status --bundle .
okf-wiki index .
okf-wiki dir --bundle .
okf-wiki now
okf-wiki wire --agent <name>
```

## Operations

### SEARCH
Find relevant pages for a query using ranked token scoring:

```bash
okf-wiki search <query> --bundle . [--max-results N] [--json]
```

Use this whenever the index path doesn't surface a topic. Searches frontmatter (title, tags,
description) and page body, ranking by relevance.

For a table-of-contents overview of all pages:

```bash
okf-wiki search --toc --bundle .
```

### STATUS
Quick health overview of the current bundle:

```bash
okf-wiki status --bundle . [--json]
```

Shows page counts by type, number of raw source files, and the last logged change.

### INIT
Scaffold a new wiki bundle:

```bash
okf-wiki init . [--title "My Wiki"]
```

Use `--from-readme` to infer the title and description from a `README.md` in the current directory:

```bash
okf-wiki init . --from-readme [--readme-path path/to/README.md]
```

This creates the `raw/`, `sources/`, `notes/`, `entities/`, and `concepts/` directories plus initial index files.

### INGEST
Add a new raw source to the wiki. The script copies the source into `raw/`, creates a skeleton
`source` page with correct frontmatter, updates `log.md`, rebuilds indexes, lints, and commits:

```bash
okf-wiki ingest <source-file> --bundle . [--title "Title"] [--slug my-slug] [--no-commit] [--dry-run]
```

After the script runs:

1. Read the source and fill in the skeleton `sources/<slug>.md` page.
2. Grep affected pages; re-read the raw source; make surgical edits to existing pages.
3. Create new `Note` pages for new concepts, linking each to at least one existing page.
4. Run `okf-wiki update <page> --bundle .` on each page you edited to bump timestamps and log.

If a new source contradicts an existing page, use `okf-wiki diff <page> --bundle .` to show the current state,
then flag the contradiction and ask before resolving.

### UPDATE
After editing a page, bump its timestamp, log, re-index, lint, and commit:

```bash
okf-wiki update <page> --bundle . [--message "custom log message"] [--no-commit]
```

### DIFF
Show a git diff for a page — use this before meaning changes to review what's there:

```bash
okf-wiki diff <page> --bundle . [--previous N] [--since <commit>]
```

### LINT
```bash
okf-wiki lint --bundle . [--json] [--strict-frontmatter]
```

Fix errors immediately; treat warnings as real problems.

`--strict-frontmatter` adds warnings when the yamlish parser may have silently dropped
content — useful for catching nested YAML, multiline scalars, or tab-indented lists
that the parser doesn't support.


### UPDATE (timeline)

Since v1.3.0, pages can carry a per-page `## timeline` section for provenance.
The timeline records *why* content changed, not just that it changed.

```bash
okf-wiki update <page> --bundle . --kind decision --summary "Switched to session cookies"
```

If the page has no `## timeline` section, `okf-wiki update <page> --bundle . --kind <kind>` creates one.
For timeline entry kinds, see `okf-spec.md`.


### TRUTH (atomic rewrite with provenance)

For wholesale rewrites of a page's meaning, pipe the new body to stdin:

```bash
cat new-body.md | okf-wiki truth <page> --bundle . --summary "Rewrote after security review"
```

This does **one atomic write**: replaces the body section, appends a `kind: decision`
timeline entry, bumps timestamp, reindexes, lints, and commits. Changing the
understanding and recording why happen together — they cannot come apart.


### ARCHIVE

When a conclusion is overturned, archive the old page instead of deleting it:

```bash
okf-wiki archive <page> --bundle . --reversal-summary "Superseded by session-cookies.md"
```

Sets `status: archived`, appends a `kind: reversal` timeline entry (if summary
given), and preserves the full page history. Archived pages are excluded from
`okf-wiki search <query> --bundle .` by default and exempt from orphan-link lint checks.


### DIR

Show the resolved current bundle directory and whether it is populated:

```bash
okf-wiki dir --bundle . [--json]
```


### WIRE

Idempotently inject the wiki discipline into agent config files:

```bash
okf-wiki wire --agent claude cursor copilot
okf-wiki wire --agent all
```

Uses `<!-- BEGIN okf -->` / `<!-- END okf -->` markers so re-running upgrades
the block in place without touching the rest of the file.


## Format essentials

- `type` is required on every concept page: `Source`, `Note`, `Index`.
- `sources` is required on factual pages (`Source`, `Note`). Must point to `raw/<file>` that exists.
- Cross-links are root-relative: `[label](/path/to/file.md)`. Targets must exist. External URLs (`https://...`) are fine and are not checked.
- `index.md` is the directory entry point; `log.md` is the change history.
- `status` is optional on concept pages: `active` (default), `draft`, `archived`.
- Pages can carry an optional `## timeline` section after the body (since v1.3.0).
- Get a fresh timestamp via `okf-wiki now` rather than hand-writing one (avoids bad-timestamp errors).

## Rules

- Compile from `raw/`, not from wiki text.
- Every factual page has `sources`.
- Surgical edits only; do not rewrite whole pages.
- Flag contradictions; do not overwrite silently.
- No orphan pages; every concept page must be linked from at least one non-index page (links from `index.md` do not count).
- Show diff and confirm before meaning changes.
- Commit after every INGEST / UPDATE.
- **Provenance discipline**: When you change a page's meaning, append a timeline entry explaining why. Use `okf-wiki update <page> --bundle . --kind decision --summary "..."` for surgical edits, or `okf-wiki truth <page> --bundle .` for atomic rewrites. Never change the body without recording the reason.
- **Archive, don't delete**: When a conclusion is overturned, use `okf-wiki archive <page> --bundle . --reversal-summary "..."` so the history survives.
