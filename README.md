# okf-wiki
LLM Wiki CLI in OKF format

`okf-wiki` is a Rust CLI for maintaining OKF-format markdown knowledge bases.

The skill bundle is available under `skills/okf-wiki/`.
`skills/okf-wiki/SKILL.md` is guarded by the test suite.

## Usage

```bash
cargo run -- <subcommand> [flags]
```

Subcommands:

- `init <bundle>` scaffolds a new OKF wiki bundle.
- `ingest <source>` copies a raw source and creates a source page.
- `update <page>` bumps timestamps, logs changes, and can append timeline entries.
- `truth <page>` rewrites a page body from stdin and appends a decision entry.
- `archive <page>` marks a page archived and can record a reversal.
- `diff [page]` shows git-backed bundle diffs.
- `lint [bundle]` validates OKF frontmatter, links, sources, timelines, and duplicates.
- `search <query>` performs ranked search or `--toc` listing.
- `status` reports bundle health.
- `index <bundle>` rebuilds directory indexes.
- `now` prints an ISO-8601 UTC timestamp.
- `dir` resolves local/global bundle directories.
- `wire --agent <name>` injects idempotent OKF guidance for supported agents.

Run `cargo run -- --help` or `cargo run -- <subcommand> --help` for flags.
