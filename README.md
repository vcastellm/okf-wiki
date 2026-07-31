# okf-wiki

`okf-wiki` is a Rust CLI and portable Agent Skill for maintaining OKF-format markdown knowledge bases.

## Install

Install the skill for a supported coding agent:

```bash
npx skills add vcastellm/okf-wiki
```

Run the CLI without a permanent installation:

```bash
npx okf-wiki init .llm-wiki
```

Until a crates.io release is published, install the CLI directly from the repository:

```bash
cargo install --git https://github.com/vcastellm/okf-wiki
```

Published GitHub releases are compatible with the crate's `cargo-binstall` metadata:

```bash
cargo binstall okf-wiki
```

Direct downloads use versioned archives from the repository's GitHub Releases page. Verify an archive against the accompanying `SHA256SUMS` file before extracting it. These stable archives and checksums are suitable inputs for a future Homebrew formula; this repository does not currently provide a tap.

## Usage

```bash
okf-wiki <subcommand> [flags]
```

Subcommands:

- `init <bundle>` scaffolds a new OKF wiki bundle.
- `ingest <source>` copies a raw source and creates a source page.
- `update <page>` updates timestamps, logs changes, and can append timeline entries.
- `truth <page>` rewrites a page body from standard input with provenance.
- `archive <page>` archives a page and can record a reversal.
- `diff [page]` shows git-backed bundle diffs.
- `lint [bundle]` validates OKF frontmatter, links, sources, timelines, and duplicates.
- `search <query>` performs ranked search or a table-of-contents listing.
- `status` reports bundle health.
- `index <bundle>` rebuilds directory indexes.
- `now` prints an ISO-8601 UTC timestamp.
- `dir` resolves local and global bundle directories.
- `wire --agent <name>` adds idempotent guidance for Claude, Codex, Cursor, Copilot, or Windsurf.

Run `okf-wiki --help` or `okf-wiki <subcommand> --help` for complete flags.

## Distribution

The npm package is a dependency-free JavaScript launcher. npm selects a native optional package for the current platform; installation does not run a postinstall downloader. Release automation builds:

- Linux x64 and arm64 with glibc
- Linux x64 and arm64 with musl
- macOS x64 and arm64
- Windows x64

GitHub Releases are the source of truth for native archives and checksums. npm packages are assembled from those CI-built binaries and published with provenance. macOS binaries are signed and notarized when maintainer credentials are configured; release dry-runs do not require them.

The canonical Agent Skill is [`skills/okf-wiki/`](skills/okf-wiki/). It follows the portable `SKILL.md`, `references/`, and `templates/` layout rather than maintaining agent-specific copies.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
node --test tests/npm/*.test.js
node scripts/check-version-sync.js
npm pack --dry-run
```

Local source execution remains available with `cargo run -- <subcommand> [flags]`.
