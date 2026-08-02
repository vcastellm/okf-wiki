# okf-wiki

[![CI](https://github.com/vcastellm/okf-wiki/actions/workflows/ci.yml/badge.svg)](https://github.com/vcastellm/okf-wiki/actions/workflows/ci.yml)

<img src="https://www.rust-lang.org/logos/rust-logo-32x32.png" alt="Rust" width="20" height="20" align="absmiddle"> Rust &nbsp; **Kernel powered by Rust**

`okf-wiki` is a Rust CLI and portable Agent Skill for maintaining OKF-format markdown knowledge bases.

## Install

Install the skill for a supported coding agent:

```bash
npx skills add vcastellm/okf-wiki
```

Run the CLI without a permanent installation from the directory that should become the bundle root:

```bash
npx okf-wiki init .
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
- `ingest <source> --bundle <path>` copies a raw source and creates a source page.
- `update <page> --bundle <path>` updates timestamps, logs changes, and can append timeline entries.
- `truth <page> --bundle <path>` rewrites a page body from standard input with provenance.
- `archive <page> --bundle <path>` archives a page and can record a reversal.
- `diff [page] --bundle <path>` shows git-backed bundle diffs.
- `lint --bundle <path>` validates OKF frontmatter, links, sources, timelines, and duplicates.
- `search <query> --bundle <path>` performs ranked search or a table-of-contents listing.
- `status --bundle <path>` reports bundle health.
- `index <bundle>` rebuilds directory indexes.
- `now` prints an ISO-8601 UTC timestamp.
- `wire --agent <name>` adds idempotent guidance for Claude, Codex, Cursor, Copilot, or Windsurf.

Bundle-scoped commands require `--bundle <path>`. Run them from the intended bundle root and pass
`--bundle .`, for example `okf-wiki status --bundle .`. Relative bundle paths are resolved against
the command's current working directory.

An optional `okf-wiki.toml` at the bundle root configures folder names. Absent config uses the
default layout (`raw/`, `sources/`, `notes/`, `entities/`, `concepts/`). A partial config
overrides only the keys it declares. See
[`skills/okf-wiki/references/okf-spec.md`](skills/okf-wiki/references/okf-spec.md) for the full
`[folders]` schema.

Run `okf-wiki --help` or `okf-wiki <subcommand> --help` for complete flags.

## Distribution

The single `okf-wiki` npm package contains a dependency-free JavaScript launcher and every supported native binary. The launcher selects its local binary at `bin/native/<Rust target>/okf-wiki` (or `.exe` on Windows); installation does not run a postinstall downloader or fetch binaries from the network. This avoids per-platform npm packages, at the cost of every npm installation downloading all supported binaries. Release automation builds:

- Linux x64 and arm64 with glibc
- Linux x64 and arm64 with musl
- macOS x64 and arm64
- Windows x64

GitHub Releases are the source of truth for native archives and checksums. The bundled npm package is assembled from those CI-built binaries and published with provenance. macOS binaries are signed and notarized when maintainer credentials are configured; release dry-runs do not require them.

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
