// Shared helpers for integration tests.
// Each test that uses these creates its own tempdir so tests can run in parallel safely.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

use indexify::{build_root, open_state, State};

/// A self-contained workspace (tempdir) with an opened Tantivy State ready for use.
pub struct Workspace {
    /// Keep the tempdir alive for the duration of the test.
    pub _tmp: TempDir,
    pub index_dir: PathBuf,
    pub root: PathBuf,
    pub state: Arc<State>,
}

impl Workspace {
    /// Create a new workspace with one root directory whose encoding is `encoding`
    /// (e.g. `"utf-8"`, `"shift_jis"`, `"euc-jp"`).
    pub fn new(encoding: &str) -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let index_dir = tmp.path().join(".indexify");
        let root = tmp.path().join("src");
        std::fs::create_dir_all(&root).unwrap();
        let tantivy = index_dir.join("tantivy");
        let state = open_state(&tantivy).expect("open_state");
        state.set_roots(&[(root.clone(), encoding.to_string())]);
        Workspace {
            _tmp: tmp,
            index_dir,
            root,
            state,
        }
    }

    /// Write `content` bytes to `relative_path` inside the workspace root.
    pub fn write(&self, relative_path: &str, content: &[u8]) {
        let full = self.root.join(relative_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }

    /// Build the full index from the workspace root (with `encoding`).
    pub fn build(&self, encoding: &str) {
        build_root(&self.state, self.root.to_str().unwrap(), encoding, |_| {}).expect("build_root");
    }

    /// Convenience: write text content with UTF-8, then build.
    pub fn write_and_build(&self, relative_path: &str, content: &str) {
        self.write(relative_path, content.as_bytes());
        self.build("utf-8");
    }
}

/// Encode `text` (a UTF-8 Rust string) as Shift_JIS bytes.
pub fn to_shift_jis(text: &str) -> Vec<u8> {
    let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(text);
    encoded.into_owned()
}

/// Encode `text` (a UTF-8 Rust string) as EUC-JP bytes.
pub fn to_euc_jp(text: &str) -> Vec<u8> {
    let (encoded, _, _) = encoding_rs::EUC_JP.encode(text);
    encoded.into_owned()
}

/// Assert that at least one hit has the given file suffix and 1-based line number.
pub fn assert_hit(hits: &[indexify::Hit], file_suffix: &str, line: usize) {
    assert!(
        hits.iter()
            .any(|h| h.file.ends_with(file_suffix) && h.line == line),
        "expected a hit in '{file_suffix}' at line {line}, got: {hits:#?}",
        hits = hits.iter().map(|h| (&h.file, h.line)).collect::<Vec<_>>()
    );
}

/// Assert that no hit mentions the given file suffix.
pub fn assert_no_hit(hits: &[indexify::Hit], file_suffix: &str) {
    assert!(
        !hits.iter().any(|h| h.file.ends_with(file_suffix)),
        "expected NO hits in '{file_suffix}', got: {hits:#?}",
        hits = hits.iter().map(|h| (&h.file, h.line)).collect::<Vec<_>>()
    );
}

/// Convenience: add an encoding_rs dependency visible to test helpers.
pub fn workspace_with_encoding(encoding: &str) -> Workspace {
    Workspace::new(encoding)
}

/// Build a secondary workspace root under the same tmpdir (for mixed-encoding tests).
pub struct TwoRootWorkspace {
    pub _tmp: TempDir,
    pub index_dir: PathBuf,
    pub root_utf8: PathBuf,
    pub root_sjis: PathBuf,
    pub state: Arc<State>,
}

impl TwoRootWorkspace {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let index_dir = tmp.path().join(".indexify");
        let root_utf8 = tmp.path().join("utf8");
        let root_sjis = tmp.path().join("sjis");
        std::fs::create_dir_all(&root_utf8).unwrap();
        std::fs::create_dir_all(&root_sjis).unwrap();
        let tantivy = index_dir.join("tantivy");
        let state = open_state(&tantivy).expect("open_state");
        state.set_roots(&[
            (root_utf8.clone(), "utf-8".to_string()),
            (root_sjis.clone(), "shift_jis".to_string()),
        ]);
        TwoRootWorkspace {
            _tmp: tmp,
            index_dir,
            root_utf8,
            root_sjis,
            state,
        }
    }

    pub fn write_utf8(&self, name: &str, content: &str) {
        std::fs::write(self.root_utf8.join(name), content.as_bytes()).unwrap();
    }

    pub fn write_sjis(&self, name: &str, text: &str) {
        std::fs::write(self.root_sjis.join(name), to_shift_jis(text)).unwrap();
    }

    pub fn build(&self) {
        build_root(
            &self.state,
            self.root_utf8.to_str().unwrap(),
            "utf-8",
            |_| {},
        )
        .expect("build utf8 root");
        build_root(
            &self.state,
            self.root_sjis.to_str().unwrap(),
            "shift_jis",
            |_| {},
        )
        .expect("build sjis root");
    }
}

// Make encoding_rs available to helpers without needing a separate import in every test file.
pub use encoding_rs;

/// Path helper: make a `relative_path` absolute under `root`.
pub fn abs(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}
