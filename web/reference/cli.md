# CLI Reference

## Synopsis

```
loupe <COMMAND> [OPTIONS]
```

Global option available on all commands:

| Flag | Default | Description |
|---|---|---|
| `--index-dir <PATH>` | `<workspace>/.loupe` | Override the index directory. Also reads from `$LOUPE_INDEX_DIR`. |

---

## `init`

Configure which folders to index and their encodings. Writes (or updates) `settings.json` inside the index directory.

```bash
loupe init [--root PATH[@ENC]]... [--force]
```

| Flag | Description |
|---|---|
| `--root <PATH[@ENC]>` | Add a root folder. Append `@enc` to set a non-UTF-8 encoding (e.g., `legacy@shift_jis`). Repeatable. |
| `--force` | Overwrite an existing `settings.json` without prompting. |

**Supported encodings:**

| Label | Encoding |
|---|---|
| `utf-8` | UTF-8 (default when no `@enc` is given) |
| `shift_jis` | Shift_JIS / Windows-31J |
| `euc-jp` | EUC-JP |

**Examples:**

```bash
# Single UTF-8 root
loupe init --root src

# Mixed encodings
loupe init --root src --root assets@shift_jis --root legacy@euc-jp

# Overwrite existing settings
loupe init --root src --force
```

::: tip Commit settings.json
`settings.json` is designed to be committed. The index body (Tantivy files, `meta.json`) is added to `.gitignore` automatically.
:::

---

## `build`

Build the index from `settings.json`. This is a one-time full scan; subsequent updates are incremental via `sync` or automatic before `search`.

```bash
loupe build [--force]
```

| Flag | Description |
|---|---|
| `--force` | Delete any existing index and rebuild from scratch. Required after a format-breaking upgrade (e.g., v0.2.x → v0.3.0). |

---

## `sync`

Incrementally update the index: reindex changed and new files, drop deleted files. Much faster than a full rebuild.

```bash
loupe sync
```

`search` calls this automatically before querying, so you rarely need to run `sync` explicitly.

---

## `search`

Search the index. Auto-syncs before querying.

```bash
loupe search <QUERY> [OPTIONS]
```

| Argument / Flag | Description |
|---|---|
| `<QUERY>` | The search query string (substring by default). |
| `--regex` | Treat `<QUERY>` as a regular expression. Requires a literal run of ≥ 2 characters in the pattern. |
| `--case-sensitive` | Match exact case. Default is case-insensitive. |
| `--max <N>` | Maximum number of results to return. Default: 100. |
| `--json` | Output results as a JSON array of `{ "file": "…", "line": N, "text": "…" }`. |
| `--no-sync` | Skip the pre-search sync step. |

**Examples:**

```bash
loupe search "calcTotal"
loupe search "calcTotal" --case-sensitive
loupe search "parse[A-Za-z]+Request" --regex
loupe search "parseRequest" --regex --case-sensitive
loupe search "calcTotal" --max 50 --json
loupe search "契約" --case-sensitive
```

::: warning Incomplete results
If the n-gram candidate set hits the internal cap, a notice is printed on stderr. This typically happens with very short or very common queries. Try a more specific term.
:::

---

## `status`

Show index statistics.

```bash
loupe status [--json]
```

Output includes:
- Whether the index has been built
- Number of indexed files
- Configured roots and their encodings
- Timestamp of last build and last sync

| Flag | Description |
|---|---|
| `--json` | Output as JSON. |

---

## `serve`

Start the NDJSON daemon used by the VS Code extension. Reads requests from stdin, writes NDJSON responses to stdout. Maintains a filesystem watcher to keep the index fresh automatically.

```bash
loupe serve [--index-dir <PATH>]
```

This command is managed by the VS Code extension — you do not normally need to run it manually.

---

## `mcp`

Start the [Model Context Protocol](https://modelcontextprotocol.io/) stdio server for AI agents.

```bash
loupe mcp [--index-dir <PATH>]
```

See [MCP Server](/reference/mcp-server) for configuration and exposed tools.
