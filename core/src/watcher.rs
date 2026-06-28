// watcher.rs — watch the configured roots and incrementally update the index.
//
// File events are debounced (collected for a quiet period) and then applied as single-path updates,
// followed by one commit + reader reload. Used by the long-running `serve` sidecar.

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::index::{builder, State};

const DEBOUNCE: Duration = Duration::from_millis(800);

pub fn start_watcher(state: Arc<State>) -> Result<()> {
    use notify::{RecursiveMode, Watcher};
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(raw_tx)?;
    {
        let roots = state.roots.lock().unwrap();
        for (prefix, _) in roots.iter() {
            let _ = watcher.watch(prefix, RecursiveMode::Recursive);
        }
    }
    // debounce thread: collect changed paths, flush after quiet period, then commit once.
    std::thread::spawn(move || {
        let _watcher = watcher; // keep alive
        let mut pending: HashSet<PathBuf> = HashSet::new();
        loop {
            let first = match raw_rx.recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };
            collect_event(&mut pending, first);
            // drain until quiet
            loop {
                match raw_rx.recv_timeout(DEBOUNCE) {
                    Ok(ev) => collect_event(&mut pending, ev),
                    Err(_) => break,
                }
            }
            if pending.is_empty() {
                continue;
            }
            for p in pending.drain() {
                builder::update_path(&state, &p);
            }
            if let Ok(mut w) = state.writer.lock() {
                let _ = w.commit();
            }
            let _ = state.reader.reload(); // reflect incremental updates in searches
        }
    });
    Ok(())
}

fn collect_event(pending: &mut HashSet<PathBuf>, ev: notify::Result<notify::Event>) {
    if let Ok(ev) = ev {
        use notify::EventKind;
        if matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
            for p in ev.paths {
                pending.insert(p);
            }
        }
    }
}
