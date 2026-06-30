# Getting Started

## Install

Prebuilt native binaries for Linux, macOS, and Windows are published to [GitHub Releases](https://github.com/ukitomato/loupe/releases) (built by [cargo-dist](https://github.com/axodotdev/cargo-dist)).

### One-line installer

::: code-group

```bash [Linux / macOS]
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/ukitomato/loupe/releases/latest/download/loupe-installer.sh | sh
```

```powershell [Windows]
powershell -ExecutionPolicy Bypass -c `
  "irm https://github.com/ukitomato/loupe/releases/latest/download/loupe-installer.ps1 | iex"
```

:::

### Download a tarball

Grab a platform tarball directly from the [Releases page](https://github.com/ukitomato/loupe/releases), extract it, and put the `loupe` binary somewhere on your `$PATH`.

### Build from source

```bash
cargo install --git https://github.com/ukitomato/loupe loupe
```

---

## Step 1 — Configure: `init`

`init` writes `settings.json` inside the index directory (default: `<workspace>/.loupe/settings.json`). This is the single source of truth shared by the CLI, MCP server, and VS Code extension.

```bash
# Index the src/ folder (UTF-8, the default)
loupe init --root src

# Multiple roots — mix UTF-8 and legacy Shift_JIS in one index
loupe init --root src --root legacy@shift_jis

# Supported encodings: utf-8  shift_jis  euc-jp
```

The generated `settings.json` looks like:

```jsonc
{
  "roots": [
    { "path": "src",    "encoding": "utf-8" },
    { "path": "legacy", "encoding": "shift_jis" }
  ]
}
```

You can edit this file by hand at any time — just run `loupe build --force` afterward to rebuild.

`settings.json` is safe to **commit** to version control. The index body (`tantivy/`, `meta.json`) is added to `.gitignore` automatically.

---

## Step 2 — Build: `build`

```bash
loupe build
```

This walks every configured root, decodes each file by its folder's encoding, extracts bigrams and trigrams, and writes the Tantivy index. The build is one-time; subsequent updates are incremental.

```bash
# Force a full rebuild (needed after upgrading to a new index format)
loupe build --force
```

---

## Step 3 — Search: `search`

```bash
# Substring search (case-insensitive by default)
loupe search "calcTotal"

# Exact case
loupe search "calcTotal" --case-sensitive

# Regular expression
loupe search "parse[A-Za-z]+Request" --regex

# Regex + case-sensitive
loupe search "parseRequest" --regex --case-sensitive

# Limit results and output as JSON
loupe search "calcTotal" --max 50 --json
```

`search` auto-syncs changed files before querying, so results are always up to date.

::: tip 2-character queries
Queries as short as 2 characters are supported, including 2-char Japanese words — e.g. `loupe search "契約"`. Single-character queries are not supported (no meaningful pre-filter candidate).
:::

::: tip Regex minimum
Regex patterns need a literal run of **≥ 2 characters** (ASCII or CJK) for the index to pre-filter candidates. Patterns with no such run (e.g. `^.*$`) fall back to full verify.
:::

---

## Checking status

```bash
loupe status          # human-readable summary
loupe status --json   # JSON output
```

Shows whether the index is built, the number of indexed files, the configured roots, and the timestamps of the last build and sync.

---

## Keeping the index up to date

`search` automatically syncs changed files before each query, so you don't need to think about it for interactive use.

For background freshness (e.g., in CI or a long-running session) you can also:

```bash
loupe sync    # incremental catch-up — reindexes only changed/new files
```

The VS Code extension and MCP server use a filesystem watcher (`notify`) to keep the index current without polling.
