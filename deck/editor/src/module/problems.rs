//! What a local build said, as the Problems list shows it: errors first, then warnings, each with its place in the working
//! copy (a click opens the external editor there); the same message at the same place once.
use scutoons_build::diag::Diag;

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub error: bool,
    /// `lib.rs:12:5`, `lib.rs`, or "module" for the whole module
    pub place: String,
    /// the message's first line
    pub text: String,
    /// the file in `src/`, "" for none
    pub file: String,
    pub line: u32,
    pub column: u32,
}

pub fn rows(diags: &[Diag]) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    for d in diags {
        let place = match (d.file.as_str(), d.line) {
            ("", _) => "module".to_string(),
            (f, 0) => f.to_string(),
            (f, l) => format!("{f}:{l}:{}", d.column.max(1)),
        };
        let row = Row { error: d.level == "error", place, text: d.message.lines().next().unwrap_or("").trim().to_string(), file: d.file.clone(), line: d.line, column: d.column };
        if !out.iter().any(|r| r.place == row.place && r.text == row.text) {
            out.push(row);
        }
    }
    out.sort_by_key(|r| !r.error);
    out
}

/// the count in words: "2 errors · 1 warning", "no problems"
pub fn summary(rows: &[Row]) -> String {
    let errors = rows.iter().filter(|r| r.error).count();
    let warnings = rows.len() - errors;
    let n = |k: usize, word: &str| format!("{k} {word}{}", if k == 1 { "" } else { "s" });
    match (errors, warnings) {
        (0, 0) => "no problems".into(),
        (e, 0) => n(e, "error"),
        (0, w) => n(w, "warning"),
        (e, w) => format!("{} · {}", n(e, "error"), n(w, "warning")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning(file: &str, line: u32, message: &str) -> Diag {
        Diag { file: file.into(), line, column: 3, level: "warning".into(), message: message.into() }
    }

    #[test]
    fn errors_come_first_with_their_places_and_twins_once() {
        let diags = vec![
            warning("lib.rs", 4, "use of a disallowed method `f32::sin`\nnote: …"),
            Diag::error("lib.rs", 12, 5, "expected one of `for`, found `fr`"),
            Diag::general("the module is 3000000 bytes, at most 2097152 are allowed"),
            warning("lib.rs", 4, "use of a disallowed method `f32::sin`"),
            Diag::error("osc.rs", 0, 0, "file \"osc.rs\" is larger than 262144 bytes"),
        ];
        let r = rows(&diags);
        let shown: Vec<(bool, &str, &str)> = r.iter().map(|r| (r.error, r.place.as_str(), r.text.as_str())).collect();
        assert_eq!(
            shown,
            [
                (true, "lib.rs:12:5", "expected one of `for`, found `fr`"),
                (true, "module", "the module is 3000000 bytes, at most 2097152 are allowed"),
                (true, "osc.rs", "file \"osc.rs\" is larger than 262144 bytes"),
                (false, "lib.rs:4:3", "use of a disallowed method `f32::sin`"),
            ]
        );
        assert_eq!((r[0].file.as_str(), r[0].line, r[0].column), ("lib.rs", 12, 5));
        assert_eq!(summary(&r), "3 errors · 1 warning");
    }

    #[test]
    fn the_summary_counts_in_words() {
        assert_eq!(summary(&[]), "no problems");
        assert_eq!(summary(&rows(&[warning("lib.rs", 1, "a")])), "1 warning");
        assert_eq!(summary(&rows(&[Diag::general("x")])), "1 error");
    }
}
