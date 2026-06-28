# Changelog

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
