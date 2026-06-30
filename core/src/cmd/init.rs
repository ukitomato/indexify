// init — configure which folders the index covers, writing settings.json.
//
// settings.json is the single source of truth shared by the CLI, the MCP server, and the VSCode
// extension. Separating configuration (init) from indexing (build) means there's exactly one place
// that decides what gets indexed, so the front-ends can't drift apart and silently miss files.
//
// Roots can come from --root flags (scriptable) or, when run in a terminal with no flags, from an
// interactive prompt. With neither, we fall back to the whole workspace as a single UTF-8 root.

use anyhow::{bail, Result};
use std::io::{BufRead, IsTerminal, Write};

use crate::store::{self, Config, RootCfg};

pub fn run(index_dir: Option<&str>, root_args: &[String], force: bool) -> Result<()> {
    let dir = store::resolve_index_dir(index_dir);
    let settings = store::settings_path(&dir);

    if settings.exists() && !force {
        bail!(
            "{} already exists. Edit it directly, or re-run with --force to overwrite.",
            settings.display()
        );
    }

    let roots: Vec<RootCfg> = if !root_args.is_empty() {
        root_args.iter().map(|a| store::parse_root_arg(a)).collect()
    } else if std::io::stdin().is_terminal() {
        prompt_roots()?
    } else {
        // Non-interactive with no flags: index everything under the workspace as UTF-8.
        eprintln!("no --root given and not a terminal; defaulting to the whole workspace (utf-8).");
        vec![RootCfg {
            path: ".".into(),
            encoding: "utf-8".into(),
        }]
    };

    let cfg = Config { roots };
    store::save_config(&dir, &cfg)?;
    store::ensure_gitignore(&dir)?;

    println!("wrote {}", settings.display());
    for r in &cfg.roots {
        println!("  {} ({})", r.path, r.encoding);
    }
    println!("next: run `loupe build` to create the index.");
    Ok(())
}

/// Read roots from the terminal: one folder per line, optional @enc suffix, blank line to finish.
fn prompt_roots() -> Result<Vec<RootCfg>> {
    println!("Configure loupe — which folders should be indexed?");
    println!("  • one folder per line, relative to the workspace root");
    println!("  • append @shift_jis or @euc-jp for non-UTF-8 folders (default is UTF-8)");
    println!("  • press Enter on an empty line to finish (no entries = the whole workspace)");

    let mut roots = Vec::new();
    let stdin = std::io::stdin();
    loop {
        print!("> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }
        let arg = line.trim();
        if arg.is_empty() {
            break;
        }
        roots.push(store::parse_root_arg(arg));
    }
    if roots.is_empty() {
        roots.push(RootCfg {
            path: ".".into(),
            encoding: "utf-8".into(),
        });
    }
    Ok(roots)
}
