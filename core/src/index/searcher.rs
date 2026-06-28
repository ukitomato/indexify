// searcher.rs — trigram-AND candidate retrieval, then parallel line-level verification.
//
//   substring: lowercased needle (>=3 chars) -> distinct trigrams -> candidate docs
//              -> memmem on an ascii-lowercased copy of each candidate file
//   regex:     literal runs (>=3 chars) of the pattern give the trigrams; the full regex
//              (case-insensitive) verifies each line

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::collections::HashSet;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{TantivyDocument, Term};

use super::State;
use crate::encoding::enc_by_name;

const CANDIDATE_LIMIT: usize = 2000;
const PER_FILE_MATCH_CAP: usize = 50;

#[derive(Clone)]
pub struct Hit {
    pub file: String,
    pub line: usize,
    pub text: String,
}

/// Extract lowercased literal runs ([A-Za-z0-9_]) of length >= 3 from a regex pattern.
fn extract_literals(pattern: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut cur = String::new();
    for c in pattern.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            cur.push(c.to_ascii_lowercase());
        } else if cur.len() >= 3 {
            runs.push(std::mem::take(&mut cur));
        } else {
            cur.clear();
        }
    }
    if cur.len() >= 3 {
        runs.push(cur);
    }
    runs
}

/// Collect distinct char-trigrams from a set of literal strings.
fn trigrams_of<'a>(strs: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = HashSet::new();
    for s in strs {
        let chars: Vec<char> = s.chars().collect();
        for w in chars.windows(3) {
            seen.insert(w.iter().collect::<String>());
        }
    }
    seen.into_iter().collect()
}

fn line_starts(bytes: &[u8]) -> Vec<usize> {
    let mut v = vec![0usize];
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

enum Verifier {
    Substr { finder: memchr::memmem::Finder<'static>, case_sensitive: bool },
    Regex(regex::Regex),
}

pub fn search(state: &State, query: &str, regex_mode: bool, max: usize, case_sensitive: bool) -> Result<Vec<Hit>> {
    let searcher = state.reader.searcher();

    // Build the trigram candidate set + the line verifier.
    let (trigrams, verifier): (Vec<String>, Verifier) = if regex_mode {
        let runs = extract_literals(query);
        if runs.is_empty() {
            return Err(anyhow!("regex needs a literal substring of >=3 chars to use the index"));
        }
        let re = if case_sensitive {
            regex::Regex::new(query)?
        } else {
            regex::Regex::new(&format!("(?i){query}"))?
        };
        (trigrams_of(runs.iter().map(|s| s.as_str())), Verifier::Regex(re))
    } else {
        let needle = query.to_ascii_lowercase();
        if needle.chars().count() < 3 {
            return Ok(Vec::new());
        }
        let tris = trigrams_of(std::iter::once(needle.as_str()));
        // For case-sensitive verify, the finder contains original-case bytes; for
        // case-insensitive it contains the lowercased needle (matching the lowercased haystack).
        let finder_needle: &[u8] = if case_sensitive { query.as_bytes() } else { needle.as_bytes() };
        let finder = memchr::memmem::Finder::new(finder_needle).into_owned();
        (tris, Verifier::Substr { finder, case_sensitive })
    };

    let mut subs: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for tg in &trigrams {
        subs.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(state.fields.tri, tg),
                IndexRecordOption::Basic,
            )),
        ));
    }
    let top = searcher.search(&BooleanQuery::new(subs), &TopDocs::with_limit(CANDIDATE_LIMIT))?;

    let mut targets: Vec<(String, &'static encoding_rs::Encoding)> = Vec::with_capacity(top.len());
    for (_s, addr) in top {
        let d: TantivyDocument = searcher.doc(addr)?;
        let path = d.get_first(state.fields.path).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let enc_name = d.get_first(state.fields.enc).and_then(|v| v.as_str()).unwrap_or("utf-8");
        targets.push((path, enc_by_name(enc_name)));
    }

    let mut hits: Vec<Hit> = targets
        .par_iter()
        .flat_map_iter(|(path, enc)| {
            let mut out = Vec::new();
            if let Ok(bytes) = std::fs::read(path) {
                let (text, _, _) = enc.decode(&bytes);
                match &verifier {
                    Verifier::Substr { finder, case_sensitive } => {
                        let orig = text.as_bytes();
                        let haystack_buf;
                        let haystack: &[u8] = if *case_sensitive {
                            orig
                        } else {
                            haystack_buf = text.to_ascii_lowercase().into_bytes();
                            &haystack_buf
                        };
                        let starts = line_starts(orig);
                        let mut last = usize::MAX;
                        for off in finder.find_iter(haystack) {
                            let li = match starts.binary_search(&off) {
                                Ok(i) => i,
                                Err(i) => i - 1,
                            };
                            if li == last {
                                continue;
                            }
                            last = li;
                            let s = starts[li];
                            let e = starts.get(li + 1).copied().unwrap_or(orig.len());
                            let line = String::from_utf8_lossy(&orig[s..e]).trim_end().to_string();
                            out.push(Hit { file: path.clone(), line: li + 1, text: line });
                            if out.len() >= PER_FILE_MATCH_CAP {
                                break;
                            }
                        }
                    }
                    Verifier::Regex(re) => {
                        for (i, line) in text.lines().enumerate() {
                            if re.is_match(line) {
                                out.push(Hit { file: path.clone(), line: i + 1, text: line.trim_end().to_string() });
                                if out.len() >= PER_FILE_MATCH_CAP {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            out
        })
        .collect();
    hits.truncate(max);
    Ok(hits)
}
