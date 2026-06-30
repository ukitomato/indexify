# Configuration

## `settings.json`

`settings.json` is the **single source of truth** for what gets indexed. It is read by all three front-ends — CLI, MCP server, and VS Code extension — so they always agree on which folders are indexed and with which encoding.

**Location:** `<index-dir>/settings.json` — default index dir is `<workspace>/.loupe/`.

### Schema

```jsonc
{
  "roots": [
    { "path": "src",    "encoding": "utf-8" },
    { "path": "legacy", "encoding": "shift_jis" }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `roots` | array | List of root folders to index. |
| `roots[].path` | string | Path to the root folder, relative to the workspace root (parent of the index dir). |
| `roots[].encoding` | string | Encoding label for all files under this root. |

### Supported encodings

| Label | Encoding |
|---|---|
| `utf-8` | UTF-8 (default) |
| `shift_jis` | Shift_JIS / Windows-31J |
| `euc-jp` | EUC-JP |

### Creating and editing

Generate it with `init`:

```bash
loupe init --root src --root legacy@shift_jis
```

Or write it by hand — the format is straightforward. After editing by hand, run `loupe build --force` to rebuild the index from the new configuration.

### Version control

`settings.json` is designed to be **committed**. Add it to your repository so that teammates and CI share the same indexing configuration.

The index body (`tantivy/`, `meta.json`) is large and regenerable, so `init` adds those paths to `.gitignore` automatically.

---

## Index directory

By default, the index lives at `<workspace>/.loupe/`. Override with:

- **CLI flag:** `--index-dir <PATH>` (available on all commands)
- **Environment variable:** `LOUPE_INDEX_DIR=<PATH>`
- **VS Code setting:** `loupe.indexDir`

When using a non-default index directory, pass the same path consistently to all front-ends. The easiest way is to set `LOUPE_INDEX_DIR` in your shell profile.

---

## `.loupe/` layout

```
.loupe/
├── settings.json   # Committable — roots + encodings
├── meta.json       # Index metadata (git-ignored)
└── tantivy/        # Tantivy segment files (git-ignored)
```

---

## VS Code settings

VS Code settings (`settings.json` in `.vscode/` or user settings) control only the editor side. They do **not** affect what is indexed.

| Setting | Default | Description |
|---|---|---|
| `loupe.indexDir` | `.loupe` | Path to the index directory. Relative to workspace root. |
| `loupe.binaryPath` | (auto) | Explicit path to the `loupe` binary. |
| `loupe.maxResults` | `100` | Default max results for the QuickPick search. |

---

## Environment variables

| Variable | Description |
|---|---|
| `LOUPE_INDEX_DIR` | Override the index directory for all commands. |
