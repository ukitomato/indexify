// store.rs — the on-disk layout of an index directory and its config/metadata.
//
//   <workspace>/.indexify/
//   ├── settings.json  roots + per-root encoding — the single source of truth shared by the CLI,
//   │                  the MCP server, and the VSCode extension (so they can't disagree on what
//   │                  to index and silently miss files)
//   ├── meta.json      statistics: last_build, last_sync, file_count
//   ├── .gitignore     ignores tantivy/ and meta.json (settings.json is safe to commit)
//   └── tantivy/       the Tantivy index itself
//
// Roots in settings.json may be relative; they are resolved against the *workspace root*, i.e. the
// parent of the index directory. So `.indexify` next to `src/` resolves "src" correctly no matter
// what the current working directory is (important for the MCP server and the sidecar). JSON is used
// (not TOML) so the TypeScript extension and the Rust binary can both read/write it without extra
// dependencies.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::encoding::canonical_name;

pub const DEFAULT_DIR_NAME: &str = ".indexify";
pub const ENV_INDEX_DIR: &str = "INDEXIFY_INDEX_DIR";

/// One indexed folder and the encoding its files are decoded with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCfg {
    pub path: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

/// Persisted index configuration (`settings.json`).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub roots: Vec<RootCfg>,
}

/// Persisted index statistics (`meta.json`).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_build: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,
    #[serde(default)]
    pub file_count: u64,
}

/// `"assets/x@sjis"` -> `RootCfg { path: "assets/x", encoding: "shift_jis" }`.
/// Without an `@enc` suffix the encoding defaults to UTF-8.
pub fn parse_root_arg(arg: &str) -> RootCfg {
    match arg.rsplit_once('@') {
        Some((path, enc)) if !enc.is_empty() => RootCfg {
            path: path.to_string(),
            encoding: canonical_name(enc).to_string(),
        },
        _ => RootCfg {
            path: arg.to_string(),
            encoding: "utf-8".to_string(),
        },
    }
}

/// Resolve the index directory: `--index-dir` flag > `$INDEXIFY_INDEX_DIR` > `./.indexify`.
pub fn resolve_index_dir(flag: Option<&str>) -> PathBuf {
    if let Some(f) = flag {
        return PathBuf::from(f);
    }
    if let Ok(env) = std::env::var(ENV_INDEX_DIR) {
        if !env.is_empty() {
            return PathBuf::from(env);
        }
    }
    PathBuf::from(DEFAULT_DIR_NAME)
}

/// The workspace root that relative roots are resolved against (the index dir's parent).
pub fn workspace_root(index_dir: &Path) -> PathBuf {
    match index_dir.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Resolve a (possibly relative) root path against the workspace root, canonicalized to an absolute
/// path. Canonicalizing matters because the stored document keys are derived from this: if one
/// front-end passes a relative `--index-dir .indexify` (roots resolve under `./`) and another passes
/// an absolute one (roots resolve under `/abs/…`), the *same* file would get two different keys and a
/// `sync` would re-index everything. Canonicalizing makes the keys identical no matter how the index
/// dir was specified. Falls back to the joined path if the root doesn't exist yet.
pub fn resolve_root(index_dir: &Path, root: &str) -> PathBuf {
    let p = Path::new(root);
    let joined = if p.is_absolute() { p.to_path_buf() } else { workspace_root(index_dir).join(p) };
    std::fs::canonicalize(&joined).unwrap_or(joined)
}

pub fn tantivy_dir(index_dir: &Path) -> PathBuf {
    index_dir.join("tantivy")
}

pub fn settings_path(index_dir: &Path) -> PathBuf {
    index_dir.join("settings.json")
}

pub fn meta_path(index_dir: &Path) -> PathBuf {
    index_dir.join("meta.json")
}

/// True once a Tantivy index has been committed at least once (it writes a `meta.json`).
pub fn index_built(index_dir: &Path) -> bool {
    tantivy_dir(index_dir).join("meta.json").exists()
}

pub fn load_config(index_dir: &Path) -> Result<Config> {
    let p = settings_path(index_dir);
    if !p.exists() {
        return Ok(Config::default());
    }
    let text = std::fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn save_config(index_dir: &Path, cfg: &Config) -> Result<()> {
    std::fs::create_dir_all(index_dir)?;
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(settings_path(index_dir), text)?;
    Ok(())
}

pub fn load_meta(index_dir: &Path) -> Meta {
    let p = meta_path(index_dir);
    std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_meta(index_dir: &Path, meta: &Meta) -> Result<()> {
    std::fs::create_dir_all(index_dir)?;
    std::fs::write(meta_path(index_dir), serde_json::to_string_pretty(meta)?)?;
    Ok(())
}

/// Write a `.gitignore` inside the index dir so the heavy/volatile parts aren't committed,
/// while `settings.json` (the roots definition) can be checked in if desired.
pub fn ensure_gitignore(index_dir: &Path) -> Result<()> {
    let p = index_dir.join(".gitignore");
    if p.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(index_dir)?;
    std::fs::write(&p, "tantivy/\nmeta.json\n")?;
    Ok(())
}

fn resolve_cfg(index_dir: &Path, cfg: &Config) -> Vec<(PathBuf, String)> {
    cfg.roots
        .iter()
        .map(|r| (resolve_root(index_dir, &r.path), canonical_name(&r.encoding).to_string()))
        .collect()
}

/// Roots from settings.json, resolved to absolute paths paired with their encoding.
/// Errors if the index has no configured roots (the caller should tell the user to run `init`).
pub fn resolved_roots(index_dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let cfg = load_config(index_dir)?;
    if cfg.roots.is_empty() {
        return Err(anyhow!(
            "no roots configured in {}. Run `indexify init` first.",
            settings_path(index_dir).display()
        ));
    }
    Ok(resolve_cfg(index_dir, &cfg))
}

/// Like `resolved_roots`, but when nothing is configured it falls back to the whole workspace
/// (the index dir's parent) as a single UTF-8 root and persists that to settings.json — so the
/// programmatic front-ends (VSCode sidecar, MCP) can bootstrap without a human running `init`.
pub fn resolved_roots_or_default(index_dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let cfg = load_config(index_dir)?;
    if !cfg.roots.is_empty() {
        return Ok(resolve_cfg(index_dir, &cfg));
    }
    let def = Config { roots: vec![RootCfg { path: ".".into(), encoding: "utf-8".into() }] };
    save_config(index_dir, &def)?;
    Ok(resolve_cfg(index_dir, &def))
}

/// Total size of the Tantivy index directory in bytes.
pub fn index_size_bytes(index_dir: &Path) -> u64 {
    dir_size(&tantivy_dir(index_dir))
}

fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
