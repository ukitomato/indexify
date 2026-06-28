// builder.rs — populate and incrementally maintain the index.
//
//   build_root  full (re)build over one root: parallel walk -> trigrams -> one writer adds docs
//   sync_all    catch-up: reindex files whose mtime changed, drop entries for deleted files
//   update_path single-path incremental update (used by the watcher)
//
// Files are decoded with their root's encoding before trigram extraction, and the encoding name is
// stored on the doc so search can re-decode the same way.

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tantivy::schema::Value;
use tantivy::tokenizer::{PreTokenizedString, Token};
use tantivy::{DocAddress, TantivyDocument, Term};

use super::{Fields, State, WRITER_HEAP_BYTES};
use crate::encoding::enc_name_of;

const MAX_FILE_BYTES: u64 = 2_000_000;

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

fn distinct_trigrams(lc: &str) -> Vec<String> {
    let chars: Vec<char> = lc.chars().collect();
    let mut set = HashSet::new();
    for w in chars.windows(3) {
        set.insert(w.iter().collect::<String>());
    }
    set.into_iter().collect()
}

fn make_doc(fields: &Fields, path: &str, enc_name: &str, mtime: u64, tris: Vec<String>) -> TantivyDocument {
    let mut d = TantivyDocument::new();
    d.add_text(fields.path, path);
    d.add_text(fields.enc, enc_name);
    d.add_u64(fields.mtime, mtime);
    let tokens: Vec<Token> = tris
        .into_iter()
        .enumerate()
        .map(|(i, t)| Token { position: i, text: t, ..Default::default() })
        .collect();
    d.add_pre_tokenized_text(fields.tri, PreTokenizedString { text: String::new(), tokens });
    d
}

fn file_mtime_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Read+decode a file and return (mtime_ms, distinct trigrams). None if skipped (too big / binary).
fn file_meta(path: &Path, enc: &'static encoding_rs::Encoding) -> Option<(u64, Vec<String>)> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let mt = file_mtime_ms(&meta);
    let bytes = std::fs::read(path).ok()?;
    if is_binary(&bytes) {
        return None;
    }
    let (text, _, _) = enc.decode(&bytes);
    Some((mt, distinct_trigrams(&text.to_ascii_lowercase())))
}

/// Full (re)build over a single root. Parallel walk produces trigrams; one consumer adds docs.
pub fn build_root<F: Fn(u64) + Sync>(state: &State, root: &str, enc_name: &str, progress: F) -> Result<u64> {
    let enc = crate::encoding::enc_by_name(enc_name);
    let fields = state.fields;
    let enc_owned = enc_name.to_string();
    let (tx, rx) = mpsc::channel::<(String, u64, Vec<String>)>();

    let writer_mutex = &state.writer;
    let total = std::thread::scope(|scope| -> Result<u64> {
        let consumer = scope.spawn(|| -> Result<u64> {
            let w = writer_mutex.lock().unwrap();
            let mut n = 0u64;
            for (path, mt, tris) in rx {
                // delete any existing doc for this path (safe for re-builds), then add
                w.delete_term(Term::from_field_text(fields.path, &path));
                w.add_document(make_doc(&fields, &path, &enc_owned, mt, tris))?;
                n += 1;
                if n % 20000 == 0 {
                    progress(n);
                }
            }
            Ok(n)
        });

        let walker = ignore::WalkBuilder::new(root).standard_filters(true).build_parallel();
        walker.run(|| {
            let tx = tx.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {
                    if entry.file_type().map_or(false, |t| t.is_file()) {
                        if let Some((mt, tris)) = file_meta(entry.path(), enc) {
                            let _ = tx.send((entry.path().to_string_lossy().into_owned(), mt, tris));
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });
        drop(tx);
        consumer.join().map_err(|_| anyhow!("consumer panicked"))?
    })?;

    state.writer.lock().unwrap().commit()?;
    state.reader.reload()?; // make committed docs visible to searches immediately
    Ok(total)
}

/// Load the current { path -> mtime_ms } map from the index (live docs only).
fn load_index_mtimes(state: &State) -> Result<HashMap<String, u64>> {
    let searcher = state.reader.searcher();
    let mut map = HashMap::new();
    for (ord, seg) in searcher.segment_readers().iter().enumerate() {
        let alive = seg.alive_bitset();
        for doc_id in 0..seg.max_doc() {
            if let Some(bs) = alive {
                if !bs.is_alive(doc_id) {
                    continue;
                }
            }
            let d: TantivyDocument = searcher.doc(DocAddress::new(ord as u32, doc_id))?;
            let path = d.get_first(state.fields.path).and_then(|v| v.as_str()).unwrap_or("").to_string();
            if path.is_empty() {
                continue;
            }
            let mt = d.get_first(state.fields.mtime).and_then(|v| v.as_u64()).unwrap_or(0);
            map.insert(path, mt);
        }
    }
    Ok(map)
}

enum SyncMsg {
    Seen(String),
    Doc(String, String, u64, Vec<String>), // path, enc_name, mtime, trigrams
}

/// Outcome of a sync pass.
pub struct SyncStats {
    pub updated: u64,
    pub removed: u64,
}

/// Catch-up sync against the configured roots: reindex new/changed files (by mtime),
/// delete index entries whose files are gone.
pub fn sync_all<F: Fn(u64) + Sync>(state: &State, progress: F) -> Result<SyncStats> {
    let roots_snapshot: Vec<(String, &'static encoding_rs::Encoding)> = state
        .roots
        .lock()
        .unwrap()
        .iter()
        .map(|(p, e)| (p.to_string_lossy().into_owned(), *e))
        .collect();
    let indexed = Arc::new(load_index_mtimes(state)?);
    let fields = state.fields;
    let writer_mutex = &state.writer;
    let indexed_consumer = indexed.clone();
    let (tx, rx) = mpsc::channel::<SyncMsg>();

    let res = std::thread::scope(|scope| -> Result<(u64, u64)> {
        let consumer = scope.spawn(|| -> Result<(u64, u64)> {
            let w = writer_mutex.lock().unwrap();
            let mut seen: HashSet<String> = HashSet::new();
            let mut updated = 0u64;
            for msg in rx {
                match msg {
                    SyncMsg::Seen(p) => {
                        seen.insert(p);
                    }
                    SyncMsg::Doc(p, enc_name, mt, tris) => {
                        w.delete_term(Term::from_field_text(fields.path, &p));
                        w.add_document(make_doc(&fields, &p, &enc_name, mt, tris))?;
                        seen.insert(p);
                        updated += 1;
                        if updated % 5000 == 0 {
                            progress(updated);
                        }
                    }
                }
            }
            let mut removed = 0u64;
            for p in indexed_consumer.keys() {
                if !seen.contains(p) {
                    w.delete_term(Term::from_field_text(fields.path, p));
                    removed += 1;
                }
            }
            Ok((updated, removed))
        });

        for (rp, enc) in &roots_snapshot {
            let enc: &'static encoding_rs::Encoding = *enc;
            let enc_name = enc_name_of(enc).to_string();
            let walker = ignore::WalkBuilder::new(rp).standard_filters(true).build_parallel();
            walker.run(|| {
                let tx = tx.clone();
                let indexed = indexed.clone();
                let enc_name = enc_name.clone();
                Box::new(move |result| {
                    if let Ok(entry) = result {
                        if entry.file_type().map_or(false, |t| t.is_file()) {
                            let path = entry.path();
                            let pathstr = path.to_string_lossy().into_owned();
                            if let Ok(meta) = std::fs::metadata(path) {
                                if meta.len() > MAX_FILE_BYTES {
                                    let _ = tx.send(SyncMsg::Seen(pathstr));
                                } else {
                                    let mt = file_mtime_ms(&meta);
                                    match indexed.get(&pathstr) {
                                        Some(&old) if old == mt => {
                                            let _ = tx.send(SyncMsg::Seen(pathstr));
                                        }
                                        _ => match file_meta(path, enc) {
                                            Some((mt2, tris)) => {
                                                let _ = tx.send(SyncMsg::Doc(pathstr, enc_name.clone(), mt2, tris));
                                            }
                                            None => {
                                                let _ = tx.send(SyncMsg::Seen(pathstr));
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    ignore::WalkState::Continue
                })
            });
        }
        drop(tx);
        consumer.join().map_err(|_| anyhow!("sync consumer panicked"))?
    })?;

    state.writer.lock().unwrap().commit()?;
    state.reader.reload()?;
    Ok(SyncStats { updated: res.0, removed: res.1 })
}

/// Incrementally update a single changed path (add/modify => reindex, missing => delete).
pub fn update_path(state: &State, path: &Path) {
    let enc = {
        let roots = state.roots.lock().unwrap();
        roots
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
            .map(|(_, e)| *e)
    };
    let enc = match enc {
        Some(e) => e,
        None => return, // not under a watched root
    };
    let path_str = path.to_string_lossy().into_owned();
    let w = state.writer.lock().unwrap();
    w.delete_term(Term::from_field_text(state.fields.path, &path_str));
    if path.is_file() {
        if let Some((mt, tris)) = file_meta(path, enc) {
            let _ = w.add_document(make_doc(&state.fields, &path_str, enc_name_of(enc), mt, tris));
        }
    }
}

/// Replace the (possibly poisoned) IndexWriter with a fresh one — used to recover from
/// transient io errors (e.g. antivirus touching the index files) and retry a build.
pub fn recreate_writer(state: &State) -> Result<()> {
    std::thread::sleep(Duration::from_millis(600)); // let AV finish touching the index files
    let w = state.index.writer_with_num_threads::<TantivyDocument>(1, WRITER_HEAP_BYTES)?;
    *state.writer.lock().unwrap() = w;
    Ok(())
}
