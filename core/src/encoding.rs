// encoding.rs — map encoding names <-> encoding_rs::Encoding.
//
// Files are decoded to UTF-8 at index time (trigrams are extracted from the decoded text), and the
// encoding *name* is stored in the index so a candidate file can be re-decoded the same way at
// search time. This is why UTF-8 and Shift_JIS (and EUC-JP) folders coexist in one index.

/// Resolve an encoding by (case-insensitive) name. Unknown names fall back to UTF-8.
pub fn enc_by_name(name: &str) -> &'static encoding_rs::Encoding {
    match name.to_lowercase().as_str() {
        "shift_jis" | "sjis" | "cp932" => encoding_rs::SHIFT_JIS,
        "euc-jp" | "eucjp" | "euc_jp" => encoding_rs::EUC_JP,
        _ => encoding_rs::UTF_8,
    }
}

/// The canonical name we persist for an encoding (round-trips through `enc_by_name`).
pub fn enc_name_of(e: &'static encoding_rs::Encoding) -> &'static str {
    if e == encoding_rs::SHIFT_JIS {
        "shift_jis"
    } else if e == encoding_rs::EUC_JP {
        "euc-jp"
    } else {
        "utf-8"
    }
}

/// Normalize a user-supplied name to its canonical form ("sjis" -> "shift_jis").
pub fn canonical_name(name: &str) -> &'static str {
    enc_name_of(enc_by_name(name))
}
