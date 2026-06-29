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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enc_by_name_shift_jis_variants() {
        for alias in &["shift_jis", "SHIFT_JIS", "sjis", "Sjis", "cp932", "CP932"] {
            assert_eq!(enc_by_name(alias), encoding_rs::SHIFT_JIS, "alias: {alias}");
        }
    }

    #[test]
    fn enc_by_name_euc_jp_variants() {
        for alias in &["euc-jp", "EUC-JP", "eucjp", "euc_jp", "EUC_JP"] {
            assert_eq!(enc_by_name(alias), encoding_rs::EUC_JP, "alias: {alias}");
        }
    }

    #[test]
    fn enc_by_name_unknown_falls_back_to_utf8() {
        for unknown in &["utf-8", "UTF-8", "utf8", "", "latin1", "unknown"] {
            assert_eq!(enc_by_name(unknown), encoding_rs::UTF_8, "input: {unknown}");
        }
    }

    #[test]
    fn enc_name_of_round_trips() {
        assert_eq!(enc_name_of(encoding_rs::SHIFT_JIS), "shift_jis");
        assert_eq!(enc_name_of(encoding_rs::EUC_JP), "euc-jp");
        assert_eq!(enc_name_of(encoding_rs::UTF_8), "utf-8");
    }

    #[test]
    fn enc_by_name_then_enc_name_of_round_trips() {
        for canonical in &["shift_jis", "euc-jp", "utf-8"] {
            assert_eq!(enc_name_of(enc_by_name(canonical)), *canonical);
        }
    }

    #[test]
    fn canonical_name_normalizes_aliases() {
        assert_eq!(canonical_name("sjis"), "shift_jis");
        assert_eq!(canonical_name("cp932"), "shift_jis");
        assert_eq!(canonical_name("eucjp"), "euc-jp");
        assert_eq!(canonical_name("euc_jp"), "euc-jp");
        assert_eq!(canonical_name("utf-8"), "utf-8");
        assert_eq!(canonical_name("anything_else"), "utf-8");
    }
}
