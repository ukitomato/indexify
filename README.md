<div align="center">

# 🔍 indexify

**Fast indexed full-text code search — one trigram index, three front-ends: CLI, MCP server, and VS Code.**

A small Rust ([Tantivy](https://github.com/quickwit-oss/tantivy)) binary keeps a compact trigram index,
decoding **UTF-8, Shift_JIS, and EUC-JP** per folder, and updates it incrementally as files change.

![CLI](https://img.shields.io/badge/CLI-indexify-DEA584?logo=rust&logoColor=white)
![MCP](https://img.shields.io/badge/MCP-stdio%20server-blue)
![VS Code](https://img.shields.io/badge/VS%20Code-extension-007ACC?logo=visualstudiocode&logoColor=white)
![Engine](https://img.shields.io/badge/engine-Rust%20%2F%20Tantivy-DEA584?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green)

</div>

---

Plain recursive grep re-scans the whole tree on every query, editor search crawls on big projects, and
most code-search tools assume everything is UTF-8. **indexify** trades a one-time index build for
near-instant searches afterward, and decodes each folder by its own encoding so legacy non-UTF-8 sources
are searchable too — **Docker-free, no runtime deps**.

It works on any project, and it shines where search usually hurts: **large or multi-encoding
codebases** — for example a monorepo holding many repositories, or a tree mixing modern UTF-8 code with
legacy Shift_JIS assets.

- ⚡ **Compact trigram index** — a small fraction of your code size, not a copy of it.
- 🈶 **Per-folder encoding** — each folder is decoded (UTF-8 / Shift_JIS / EUC-JP …) at index time, so a
  single index serves mixed-encoding trees and non-UTF-8 text is searchable without mojibake.
- 🔁 **Incremental** — search auto-syncs changed files first; the daemon/extension also watch the tree and
  reindex only what changed, so the index stays fresh without re-scanning everything.
- 🔎 **Substring and regex** — trigram candidates → exact verify (Zoekt/codesearch style).
- 🧩 **One index, three front-ends** — the **CLI**, an **MCP server** (for AI agents), and the **VS Code**
  extension all read the same index and the same `settings.json`, so they can never disagree about what's
  indexed.
- 🪶 **Self-contained native binary** — one `indexify` executable per platform, no Docker, no runtime deps.

## 🧠 Model

Three steps, separated on purpose:

1. **`init`** — choose which folders to index and each folder's encoding. Writes
   `<index-dir>/settings.json` — the single source of truth shared by all front-ends.
2. **`build`** — create the index from `settings.json`.
3. **`search`** — the everyday operation; it auto-syncs changed files first.

The index lives in `<workspace>/.indexify/` by default (override with `--index-dir` or
`INDEXIFY_INDEX_DIR`). `settings.json` is safe to commit; the index body (`tantivy/`, `meta.json`) is
git-ignored automatically.

## 📦 Install

Prebuilt binaries for Linux, macOS, and Windows are published to GitHub Releases (built by
[cargo-dist](https://github.com/axodotdev/cargo-dist)). One-line installers:

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ukitomato/indexify/releases/latest/download/indexify-installer.sh | sh
```

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ukitomato/indexify/releases/latest/download/indexify-installer.ps1 | iex"
```

Or grab a tarball from the [Releases page](https://github.com/ukitomato/indexify/releases), or build
from source: `cargo install --git https://github.com/ukitomato/indexify indexify`.

## 🚀 CLI

```bash
# 1. configure roots (interactive in a terminal, or via flags); add @enc for non-UTF-8 folders
indexify init --root src --root lib
indexify init --root src --root legacy@shift_jis      # mixed encodings in one index

# 2. build the index from settings.json
indexify build

# 3. search (auto-syncs first)
indexify search "calcTotal"
indexify search "parse[A-Za-z]+Request" --regex
indexify search "calcTotal" --json                    # JSON array of { file, line, text }

indexify status                                        # built? file count, roots, last build/sync
```

| Command | Purpose |
| --- | --- |
| `init [--root PATH[@ENC]]… [--force]` | Configure roots/encodings → `settings.json` |
| `build [--force]` | (Re)build the index from `settings.json` |
| `sync` | Incremental catch-up (search does this automatically) |
| `search <q> [--regex] [--max N] [--json] [--no-sync]` | Search the index |
| `status [--json]` | Index statistics |
| `serve` | NDJSON daemon used by the VS Code extension |
| `mcp` | MCP (Model Context Protocol) stdio server |

## 🤖 MCP server (AI agents)

`indexify mcp` speaks the Model Context Protocol over stdio. Register it with your MCP client:

```jsonc
{
  "mcpServers": {
    "indexify": {
      "command": "/path/to/indexify",
      "args": ["mcp", "--index-dir", "/path/to/workspace/.indexify"]
    }
  }
}
```

Tools exposed: `search_code`, `search_regex`, `build_index`, `sync_index`, `index_status`. The server
opens the shared index and keeps it fresh via a file watcher for the lifetime of the session.

## 🧩 VS Code extension

1. Configure roots once — run `indexify init …` (or edit `.indexify/settings.json`), then accept the
   **"Build it now?"** prompt (or run **indexify: Build / rebuild index**). If no `settings.json` exists,
   the first build indexes the whole workspace as UTF-8.
2. Hit **`Ctrl+Alt+F`** to search; use **Search (regex)** for patterns.

| Command | Keybinding |
| --- | --- |
| **indexify: Search (substring)** | `Ctrl+Alt+F` |
| **indexify: Search (regex)** | — |
| **indexify: Build / rebuild index** | — |

VS Code settings cover only the editor side — `indexify.indexDir`, `indexify.binaryPath`,
`indexify.maxResults`. **Roots and encodings are not VS Code settings**; they live in
`settings.json` so the CLI, MCP server, and extension stay in agreement.

## ⚙️ Configuration — `settings.json`

`<index-dir>/settings.json` is the one place that defines what gets indexed:

```jsonc
{
  "roots": [
    { "path": "src",    "encoding": "utf-8" },
    { "path": "assets", "encoding": "shift_jis" }
  ]
}
```

Write it with `indexify init`, or edit it by hand. Relative paths resolve against the workspace root
(the parent of the index dir).

## 🔧 How it works

```
   CLI / MCP server / VS Code extension
     │  (all read settings.json + the same index)
     ▼
   indexify  (Rust / Tantivy)
     ├─ build:   parallel walk → per-file decode (UTF-8/Shift_JIS/EUC-JP) → DISTINCT trigrams → index
     ├─ sync:    compare mtimes → reindex only changed files, drop deleted ones
     ├─ watch:   notify FS events → incremental update (delete+add, debounced)
     └─ search:  trigram-AND candidates → parallel verify (substring/regex) → file:line
```

## 📊 Measured (≈290k files: ~260k UTF-8 + ~29k Shift_JIS)

| | |
| --- | --- |
| Index size | **≈237 MB** |
| First build (cold, one-time) | ~28 min · then incremental is instant |
| Search — specific identifier | ~180 ms |
| Search — Japanese in Shift_JIS | ~156 ms |
| Search — very common term | <1 s |

## 📋 Notes

- **Binaries** are distributed via GitHub Releases (built by cargo-dist), not committed to the repo.
  The VS Code extension's CI downloads the matching one and bundles it under `bin/<os>-<arch>/` at
  package time; for local development, `cargo build` and point `indexify.binaryPath` (or `$PATH`) at it.
- **regex** uses the index only when the pattern contains a literal run of ≥3 characters (e.g. `func\s+\w+`).
- If antivirus scans the index directory, builds can occasionally hit a transient I/O error; indexify
  retries automatically. Excluding the index folder from AV avoids it entirely.

## 📄 License

MIT — see the `LICENSE` file. Built on [Tantivy](https://github.com/quickwit-oss/tantivy) (MIT).
