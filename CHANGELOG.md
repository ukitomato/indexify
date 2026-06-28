# Changelog

## 0.2.0

### VS Code extension

- **Sidebar search view** (Activity Bar → indexify icon, `Ctrl+Alt+Shift+F`) — persistent panel with
  streaming results grouped by file, match highlighting, and file-at-line navigation.
  - File group headers show the filename on its own line with the directory path below; hover the
    header to see the full path as a tooltip.
  - **`Aa`** case-sensitive toggle.
  - **`.*`** regular-expression toggle.
  - **Max results** dropdown — 50 / 100 / 300 / 1000 / ∞.
  - **`···`** reveals path filter fields:
    - **Files to include** — glob patterns to restrict results (e.g. `src/`, `*.java`).
    - **Files to exclude** — glob patterns to hide results (e.g. `*.min.js`, `test/`).
    - Both support `*` (within segment), `**` (across segments), `?` (single char), or plain
      substring. Filters are applied client-side without re-searching.

### CLI / core

- **`--case-sensitive`** flag for `indexify search` — exact-case substring and regex matching.
  The trigram phase still uses lowercase for fast candidate selection; the verify step re-checks
  original bytes when `--case-sensitive` is set.

### CI

- `vscode.yml`: changed trigger from `release: published` to `workflow_run` to work around the
  GitHub Actions restriction that `GITHUB_TOKEN`-created releases do not cascade to other workflows.

---

## 0.1.0

Initial release.

- Fast indexed full-text code search over a Tantivy trigram index.
- One binary, three front-ends sharing a single index: a **CLI**
  (`init` / `build` / `sync` / `search` / `status`), an **MCP stdio server** (`mcp`), and an NDJSON
  **sidecar** (`serve`) for the VS Code extension.
- Roots and per-folder encodings are configured in `settings.json` — the single source of truth read
  by all three front-ends.
- Per-folder encoding decoded at index time: **UTF-8, Shift_JIS, and EUC-JP** coexist in one index.
- **Substring and regex** search: trigram candidates → exact verify (parallel).
- Search **auto-syncs** changed files first; a filesystem watcher keeps the index fresh incrementally.
- Index stored in `<workspace>/.indexify/` (`settings.json` is committable; the index body is
  git-ignored).
- Native binaries per platform under `bin/<os>-<arch>/`; no Docker, no runtime dependencies.
