// status — show what's in the index: file count, configured roots, last build/sync, on-disk size.

use anyhow::Result;

use crate::index::open_state;
use crate::store;

pub fn run(index_dir: Option<&str>, json: bool) -> Result<()> {
    let dir = store::resolve_index_dir(index_dir);
    let built = store::index_built(&dir);
    let cfg = store::load_config(&dir).unwrap_or_default();
    let meta = store::load_meta(&dir);
    let size = store::index_size_bytes(&dir);

    // Prefer the live doc count; fall back to the recorded one if the index can't be opened.
    let file_count = if built {
        open_state(&store::tantivy_dir(&dir))
            .map(|s| s.num_docs())
            .unwrap_or(meta.file_count)
    } else {
        meta.file_count
    };

    if json {
        let roots: Vec<_> = cfg
            .roots
            .iter()
            .map(|r| serde_json::json!({ "path": r.path, "encoding": r.encoding }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "built": built,
                "index_dir": dir.display().to_string(),
                "file_count": file_count,
                "size_bytes": size,
                "last_build": meta.last_build,
                "last_sync": meta.last_sync,
                "roots": roots,
            }))?
        );
        return Ok(());
    }

    println!("index dir : {}", dir.display());
    println!("built     : {}", if built { "yes" } else { "no" });
    println!("files     : {file_count}");
    println!("size      : {:.1} MB", size as f64 / 1_048_576.0);
    println!("last build: {}", meta.last_build.as_deref().unwrap_or("-"));
    println!("last sync : {}", meta.last_sync.as_deref().unwrap_or("-"));
    if cfg.roots.is_empty() {
        println!("roots     : (none configured)");
    } else {
        println!("roots     :");
        for r in &cfg.roots {
            println!("  - {} [{}]", r.path, r.encoding);
        }
    }
    Ok(())
}
