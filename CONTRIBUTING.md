# Contributing

## Layout

```
loupe/
├── core/                          ← Rust crate: the `loupe` binary (CLI + MCP + sidecar)
│   ├── Cargo.toml
│   ├── .cargo/config.toml         ← crt-static (no MinGW runtime DLL on Windows)
│   └── src/
│       ├── main.rs                ← clap subcommand routing
│       ├── store.rs               ← .loupe/ layout: settings.json, meta.json, .gitignore
│       ├── encoding.rs            ← UTF-8 / Shift_JIS / EUC-JP name resolution
│       ├── watcher.rs             ← notify-based incremental watcher (debounced)
│       ├── index/
│       │   ├── schema.rs          ← Tantivy schema (path, enc, tri, mtime)
│       │   ├── builder.rs         ← build / sync / single-path update
│       │   └── searcher.rs        ← trigram candidates → parallel verify
│       └── cmd/                   ← one module per subcommand
│           ├── init.rs            ← configure roots → settings.json
│           ├── build.rs           ← full (re)build from settings.json
│           ├── sync.rs            ← incremental catch-up
│           ├── search.rs          ← human/JSON search (auto-syncs first)
│           ├── status.rs          ← index statistics
│           ├── serve.rs           ← NDJSON daemon for the VS Code extension
│           └── mcp.rs             ← MCP (Model Context Protocol) stdio server
├── vscode/                        ← VS Code extension (the .vsix unit)
│   ├── package.json / tsconfig.json / esbuild.js
│   └── src/
│       ├── extension.ts           ← lifecycle: spawn sidecar, commands, build-with-progress
│       ├── search.ts              ← progressive QuickPick over streamed matches
│       ├── config.ts             ← settings (indexDir, binaryPath, maxResults), binary resolution
│       └── sidecarClient.ts       ← NDJSON-over-stdio client (search / build / sync / watch)
└── bin/<os>-<arch>/loupe       ← compiled binary per platform [shipped + bundled by the vsix]
```

## Architecture

- **One binary, several front-ends.** `loupe` is a single Rust executable; subcommands select the
  interface (`search`/`build`/… for humans, `mcp` for AI agents, `serve` for VS Code). They all share
  the `index::` core and the same on-disk index.
- **Single source of truth.** Roots and per-folder encodings live in `<index-dir>/settings.json` (JSON,
  so the TypeScript extension and the Rust binary read it with no extra deps). The CLI, MCP server, and
  extension all read it — none of them carry their own root list — so they can't drift apart and miss
  files. `init` writes it; `build`/`sync`/`serve`/`mcp` read it.
- **Index**: one Tantivy doc per file `{ path, enc, tri, mtime }` where `tri` is the file's *distinct*
  char trigrams (codesearch-style — far cheaper to build than indexing every position). Content is
  **not** stored; verification re-reads candidate files, decoding by `enc`.
- **Search**: AND the query's trigrams → candidate docs → parallel verify (memmem for substring, the
  `regex` crate for regex) → `file:line`. Search auto-runs an incremental sync first (skip with
  `--no-sync`).
- **Incremental**: `sync` compares filesystem mtimes against the index; the watcher (`notify`) debounces
  changed paths then applies `delete_term(path)` + `add_document` + a single `commit`.

## Build the binary

This repo is a Cargo workspace (`Cargo.toml` at the root, the crate in `core/`), so build from the
repo root — the target dir is `./target/`, not `core/target/`.

```bash
cargo build --release                          # → target/release/loupe
```

`bin/` is git-ignored (binaries are distributed via Releases, not committed). For local CLI use, put
`target/release/loupe` on your `$PATH` (e.g. symlink it into `~/.local/bin`). For VS Code dev, set
`loupe.binaryPath` to it, or copy into `bin/<os>-<arch>/` which `resolveBinary` also checks.

### Releases (cargo-dist)

Cross-platform binaries + `curl|sh` / `irm|iex` installers are produced by
[dist](https://github.com/axodotdev/cargo-dist). Config lives in `dist-workspace.toml`; CI is
`.github/workflows/release.yml`. Cut a release by pushing a tag:

```bash
# bump the version in core/Cargo.toml first, then:
git tag v0.1.0 && git push --tags        # CI builds every target and publishes a GitHub Release
dist plan                                # preview locally what a release would produce
```

Run `dist init` again after changing `dist-workspace.toml` to regenerate the workflow.

### Manual Windows build (without CI)

Requires the **GNU** Rust toolchain plus MinGW-w64 binutils (for `dlltool`):

```powershell
winget install -e --id Rustlang.Rust.GNU
winget install -e --id BrechtSanders.WinLibs.POSIX.MSVCRT   # provides gcc/dlltool/ar
# add both bin dirs to PATH, then from the repo root:
cargo build --release --target x86_64-pc-windows-gnu
copy target\x86_64-pc-windows-gnu\release\loupe.exe bin\win32-x64\
```

`core/.cargo/config.toml` sets `crt-static` for the windows-gnu target so the binary has no MinGW
runtime DLL dependency. (cargo-dist CI instead builds the `windows-msvc` target, which needs no such
config.)

## Build / package the extension

```bash
cd vscode
npm install
npm run typecheck     # tsc --noEmit
npm run build         # esbuild → out/extension.js
npm run package       # vsce package → .vsix
```

`.vscodeignore` excludes `src/**` and `node_modules/**`; `out/` and the platform `bin/` ship.

## Quick manual check

```bash
loupe init --root . && loupe build
loupe search "<some token in your tree>"
loupe status
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | loupe mcp   # MCP smoke test
```

## Notes

- Set a real `publisher` and `repository.url` in `vscode/package.json` before publishing.
- The binary retries a build once on transient I/O errors (antivirus). Excluding the index directory
  from AV avoids them.
- `regex` uses the index only when the pattern has a literal run of ≥3 characters.
