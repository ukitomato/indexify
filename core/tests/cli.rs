// CLI black-box tests via assert_cmd.
// Each test builds a fresh tempdir so tests run in parallel safely.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────

struct Env {
    _tmp: TempDir,
    pub root: PathBuf,
    pub index_dir: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("src");
        let index_dir = tmp.path().join(".indexify");
        std::fs::create_dir_all(&root).unwrap();
        Env {
            _tmp: tmp,
            root,
            index_dir,
        }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("indexify").unwrap();
        c.env("INDEXIFY_INDEX_DIR", &self.index_dir);
        c
    }

    fn write(&self, name: &str, content: &[u8]) {
        std::fs::write(self.root.join(name), content).unwrap();
    }

    fn init(&self) {
        self.cmd()
            .args(["init", "--root", self.root.to_str().unwrap()])
            .assert()
            .success();
    }

    fn build(&self) {
        self.cmd().arg("build").assert().success();
    }
}

// ── init ─────────────────────────────────────────────────────────

#[test]
fn init_creates_settings_json() {
    let env = Env::new();
    env.init();
    assert!(env.index_dir.join("settings.json").exists());
}

#[test]
fn init_with_encoding_stores_canonical_name() {
    let env = Env::new();
    env.cmd()
        .args([
            "init",
            "--root",
            &format!("{}@sjis", env.root.to_str().unwrap()),
        ])
        .assert()
        .success();
    let cfg = std::fs::read_to_string(env.index_dir.join("settings.json")).unwrap();
    assert!(
        cfg.contains("shift_jis"),
        "expected 'shift_jis' in settings.json, got: {cfg}"
    );
}

// ── build ─────────────────────────────────────────────────────────

#[test]
fn build_indexes_files_and_reports_count() {
    let env = Env::new();
    env.write("a.rs", b"fn main() {}");
    env.write("b.rs", b"fn helper() {}");
    env.init();
    env.cmd()
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("2").or(predicate::str::contains("file")));
}

#[test]
fn build_without_init_fails_with_clear_message() {
    // CLI build requires prior `init`; resolved_roots_or_default is only used by sidecar/MCP.
    let env = Env::new();
    env.write("x.txt", b"hello");
    env.cmd()
        .arg("build")
        .assert()
        .failure()
        .stderr(predicate::str::contains("init"));
}

// ── search ────────────────────────────────────────────────────────

#[test]
fn search_finds_content_default_output() {
    let env = Env::new();
    env.write("hello.txt", b"fn calculate_total(x: u32) -> u32 { x }");
    env.init();
    env.build();
    env.cmd()
        .args(["search", "calculate_total", "--no-sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("calculate_total"));
}

#[test]
fn search_json_output_is_valid_json() {
    let env = Env::new();
    env.write("f.rs", b"struct FooBar {}");
    env.init();
    env.build();
    let out = env
        .cmd()
        .args(["search", "FooBar", "--json", "--no-sync"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("search --json output must be valid JSON");
    assert!(parsed.is_array(), "expected JSON array, got: {parsed}");
}

#[test]
fn search_json_contains_expected_fields() {
    let env = Env::new();
    env.write("f.rs", b"pub fn public_api() {}");
    env.init();
    env.build();
    let out = env
        .cmd()
        .args(["search", "public_api", "--json", "--no-sync"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.get("file").is_some(), "missing 'file' field");
    assert!(first.get("line").is_some(), "missing 'line' field");
    assert!(first.get("text").is_some(), "missing 'text' field");
}

#[test]
fn search_two_char_query_returns_hit_json() {
    // 2-char queries are searchable now (bigram-indexed) instead of returning empty.
    let env = Env::new();
    env.write("f.txt", b"ab");
    env.init();
    env.build();
    let out = env
        .cmd()
        .args(["search", "ab", "--json", "--no-sync"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    assert!(!arr.is_empty(), "expected a hit for 2-char query");
}

#[test]
fn search_no_match_returns_empty_json() {
    let env = Env::new();
    env.write("f.txt", b"hello world");
    env.init();
    env.build();
    let out = env
        .cmd()
        .args(["search", "zzz_not_in_file_zzz", "--json", "--no-sync"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&String::from_utf8(out).unwrap()).unwrap();
    assert!(arr.is_empty());
}

#[test]
fn search_invalid_regex_nonzero_exit() {
    let env = Env::new();
    env.write("f.txt", b"hello");
    env.init();
    env.build();
    // \d{3} has no literal >=2-char run → indexify returns an error
    env.cmd()
        .args(["search", r"\d{3}", "--regex", "--no-sync"])
        .assert()
        .failure();
}

#[test]
fn search_case_sensitive_flag() {
    let env = Env::new();
    env.write("f.txt", b"calculateTotal");
    env.init();
    env.build();
    // case-sensitive: uppercase won't match
    let out = env
        .cmd()
        .args([
            "search",
            "CALCULATETOTAL",
            "--case-sensitive",
            "--json",
            "--no-sync",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&String::from_utf8(out).unwrap()).unwrap();
    assert!(
        arr.is_empty(),
        "case-sensitive search should find nothing for wrong case"
    );
}

// ── status ────────────────────────────────────────────────────────

#[test]
fn status_reports_file_count_after_build() {
    let env = Env::new();
    env.write("a.txt", b"hello");
    env.write("b.txt", b"world");
    env.init();
    env.build();
    env.cmd()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("2").or(predicate::str::contains("file")));
}

#[test]
fn status_json_output_has_file_count() {
    let env = Env::new();
    env.write("a.txt", b"hello");
    env.init();
    env.build();
    let out = env
        .cmd()
        .args(["status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out).unwrap()).unwrap();
    assert!(
        v.get("file_count").is_some(),
        "status --json should include file_count"
    );
}
