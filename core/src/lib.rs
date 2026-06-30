// loupe — incremental trigram code-search index (library crate).
//
// The binary (`src/main.rs`) is a thin CLI wrapper over this library: it parses arguments and
// dispatches to `cmd::*`. The library is split out so the index/build/search logic can be exercised
// directly from integration tests (`core/tests/`) without spawning a process.
//
// Public surface is intentionally small — a façade over the index engine for tests and embedders:
//   open_state / State            open (or create) a Tantivy index
//   build_root / sync_all / SyncStats   populate and incrementally maintain it
//   search / Hit                  query it (substring or regex)
//
// Everything else stays crate-private; `cmd` is public only because the binary front-end calls into
// it. Internal helpers (trigram extraction, line-offset math, binary detection, …) are unit-tested in
// place via `#[cfg(test)] mod tests` and need no widened visibility.

pub mod cmd;

pub(crate) mod encoding;
pub(crate) mod index;
pub(crate) mod store;
pub(crate) mod watcher;

pub use index::builder::{build_root, sync_all, SyncStats};
pub use index::searcher::{search, Hit};
pub use index::{open_state, State};
