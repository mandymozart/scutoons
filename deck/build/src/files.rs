//! The member's source files: the same limits the API holds them to (server Domain\Modules\ModuleSource) – checked
//! again here because the service is reachable on its own, behind a shared secret only.
use std::collections::BTreeMap;

use crate::diag::Diag;

pub const MAX_FILES: usize = 32;
pub const MAX_FILE_BYTES: usize = 256 * 1024;
pub const MAX_TOTAL_BYTES: usize = 512 * 1024;
pub const MAIN_FILE: &str = "lib.rs";

/// `{ "lib.rs": "…", "osc.rs": "…" }`, in name order
pub type Files = BTreeMap<String, String>;

/// `[a-z0-9_]+\.rs`
pub fn valid_name(name: &str) -> bool {
    match name.strip_suffix(".rs") {
        Some(stem) => !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        None => false,
    }
}

/// The files of a request body `{ "files": { name: source } }`, or what is wrong with it
pub fn from_json(body: &[u8]) -> Result<Files, Vec<Diag>> {
    let v: serde_json::Value = serde_json::from_slice(body).map_err(|e| vec![Diag::general(format!("the body is not JSON: {e}"))])?;
    let Some(object) = v.get("files").and_then(|f| f.as_object()) else {
        return Err(vec![Diag::general("the body must be { \"files\": { \"lib.rs\": \"source\", … } }")]);
    };
    let mut files = Files::new();
    let mut problems = Vec::new();
    for (name, source) in object {
        match source.as_str() {
            Some(s) => {
                files.insert(name.clone(), s.to_string());
            }
            None => problems.push(Diag::error(name, 0, 0, format!("file \"{name}\" must be a text"))),
        }
    }
    problems.extend(check(&files));
    if problems.is_empty() { Ok(files) } else { Err(problems) }
}

/// The limits: 1–32 files named `[a-z0-9_]+.rs`, a `lib.rs`, each ≤ 256 KB, all ≤ 512 KB
pub fn check(files: &Files) -> Vec<Diag> {
    let mut p = Vec::new();
    if files.is_empty() || files.len() > MAX_FILES {
        p.push(Diag::general(format!("a module has 1–{MAX_FILES} files, this has {}", files.len())));
    }
    let mut total = 0;
    for (name, source) in files {
        if !valid_name(name) {
            p.push(Diag::error(name, 0, 0, format!("file \"{name}\": a name is [a-z0-9_]+.rs")));
        }
        if source.len() > MAX_FILE_BYTES {
            p.push(Diag::error(name, 0, 0, format!("file \"{name}\" is larger than {MAX_FILE_BYTES} bytes")));
        }
        total += source.len();
    }
    if total > MAX_TOTAL_BYTES {
        p.push(Diag::general(format!("the files together are larger than {MAX_TOTAL_BYTES} bytes")));
    }
    if !files.contains_key(MAIN_FILE) {
        p.push(Diag::general(format!("a module needs a {MAIN_FILE}")));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(files: &[(&str, &str)]) -> Vec<u8> {
        let map: serde_json::Map<String, serde_json::Value> = files.iter().map(|(n, s)| (n.to_string(), (*s).into())).collect();
        serde_json::to_vec(&serde_json::json!({ "files": map })).unwrap()
    }

    fn messages(r: Result<Files, Vec<Diag>>) -> String {
        r.unwrap_err().into_iter().map(|d| d.message).collect::<Vec<_>>().join(" | ")
    }

    #[test]
    fn names_are_lowercase_rs_files() {
        for ok in ["lib.rs", "osc_2.rs", "a.rs", "9.rs"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["Lib.rs", "lib.rs.rs ", "lib", ".rs", "a-b.rs", "a/b.rs", "../lib.rs", "build.RS", "lib.txt", "", "ä.rs", "a b.rs"] {
            assert!(!valid_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_good_body_is_taken() {
        let files = from_json(&body(&[("lib.rs", "// a"), ("osc.rs", "// b")])).unwrap();
        assert_eq!(files.keys().collect::<Vec<_>>(), ["lib.rs", "osc.rs"]);
    }

    #[test]
    fn the_limits_hold() {
        assert!(messages(from_json(&body(&[("osc.rs", "//")]))).contains("needs a lib.rs"));
        assert!(messages(from_json(&body(&[]))).contains("1–32 files"));
        assert!(messages(from_json(&body(&[("lib.rs", "//"), ("build.rs", "//"), ("Osc.rs", "//")]))).contains("\"Osc.rs\": a name is"));
        let big = "x".repeat(MAX_FILE_BYTES + 1);
        assert!(messages(from_json(&body(&[("lib.rs", &big)]))).contains("larger than 262144"));
        let half = "x".repeat(MAX_FILE_BYTES);
        assert!(messages(from_json(&body(&[("lib.rs", &half), ("a.rs", &half), ("b.rs", "//")]))).contains("together are larger than 524288"));
        let many: Vec<(String, &str)> = (0..33).map(|i| (if i == 0 { "lib.rs".to_string() } else { format!("f{i}.rs") }, "//")).collect();
        let refs: Vec<(&str, &str)> = many.iter().map(|(n, s)| (n.as_str(), *s)).collect();
        assert!(messages(from_json(&body(&refs))).contains("1–32 files"));
        // the edges pass
        let edge = "x".repeat(MAX_FILE_BYTES);
        assert!(from_json(&body(&[("lib.rs", &edge)])).is_ok());
    }

    #[test]
    fn a_malformed_body_is_a_message_not_a_panic() {
        assert!(messages(from_json(b"nope")).contains("not JSON"));
        assert!(messages(from_json(b"{}")).contains("must be { \"files\""));
        assert!(messages(from_json(br#"{"files":["lib.rs"]}"#)).contains("must be { \"files\""));
        assert!(messages(from_json(br#"{"files":{"lib.rs":5}}"#)).contains("must be a text"));
    }
}
