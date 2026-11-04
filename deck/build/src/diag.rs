//! What the service tells a creator about their code: one message with its place. rustc's own diagnostics
//! (`--error-format=json`, which cargo wraps as `{"reason":"compiler-message","message":{…}}` lines on stdout) and the
//! service's own refusals (a file name, a banned construct) come out in the same shape.
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diag {
    /// the member's file (`lib.rs`), or "" for a message about the whole module
    pub file: String,
    /// 1-based; 0 = no place
    pub line: u32,
    pub column: u32,
    /// `error` | `warning`
    pub level: String,
    pub message: String,
}

impl Diag {
    pub fn error(file: &str, line: u32, column: u32, message: impl Into<String>) -> Diag {
        Diag { file: file.to_string(), line, column, level: "error".into(), message: message.into() }
    }

    /// an error about the module as a whole
    pub fn general(message: impl Into<String>) -> Diag {
        Diag::error("", 0, 0, message)
    }
}

/// The member crate's file for a span's `file_name` (`src/lib.rs` → `lib.rs`); anything else as rustc wrote it
fn member_file(name: &str) -> String {
    let name = name.replace('\\', "/");
    name.strip_prefix("src/").unwrap_or(&name).to_string()
}

/// The errors and warnings in cargo's `--message-format=json` output (one JSON object a line; other lines and other
/// reasons are skipped), in the order rustc gave them. The summary line "aborting due to …" is not a diagnostic.
pub fn parse_cargo_json(stdout: &str) -> Vec<Diag> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        // a message of a dependency (the SDK) is not the member's to fix, but a failure inside it is still shown
        let Some(m) = v.get("message") else { continue };
        let level = m.get("level").and_then(Value::as_str).unwrap_or("");
        let text = m.get("message").and_then(Value::as_str).unwrap_or("");
        if !matches!(level, "error" | "warning") || text.starts_with("aborting due to") || text.contains("warning emitted") || text.contains("warnings emitted") {
            continue;
        }
        let spans = m.get("spans").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        let span = spans.iter().find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true)).or_else(|| spans.first());
        let (file, line, column) = match span {
            Some(s) => (
                member_file(s.get("file_name").and_then(Value::as_str).unwrap_or("")),
                s.get("line_start").and_then(Value::as_u64).unwrap_or(0) as u32,
                s.get("column_start").and_then(Value::as_u64).unwrap_or(0) as u32,
            ),
            None => (String::new(), 0, 0),
        };
        let code = m.get("code").and_then(|c| c.get("code")).and_then(Value::as_str);
        let message = match code {
            Some(c) => format!("{text} [{c}]"),
            None => text.to_string(),
        };
        out.push(Diag { file, line, column, level: level.to_string(), message });
    }
    out
}

/// The lint warnings of the determinism rules in `cargo clippy --message-format=json` output (`clippy::disallowed_methods`,
/// the only lint the service reports from clippy: rustc's own warnings come from the build). The message carries the
/// lint's note, which is clippy.toml's `reason` – what to call instead: "use of a disallowed method `f32::sin` – use
/// deck_module::math::sin …".
pub fn parse_lints(stdout: &str) -> Vec<Diag> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(m) = v.get("message") else { continue };
        if m.get("code").and_then(|c| c.get("code")).and_then(Value::as_str) != Some("clippy::disallowed_methods") {
            continue;
        }
        let spans = m.get("spans").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        let span = spans.iter().find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true)).or_else(|| spans.first());
        let (file, line, column) = match span {
            Some(s) => (
                member_file(s.get("file_name").and_then(Value::as_str).unwrap_or("")),
                s.get("line_start").and_then(Value::as_u64).unwrap_or(0) as u32,
                s.get("column_start").and_then(Value::as_u64).unwrap_or(0) as u32,
            ),
            None => (String::new(), 0, 0),
        };
        // the reason is the first note child (the others: the link to clippy's docs, "on by default")
        let reason = m.get("children").and_then(Value::as_array).and_then(|c| c.iter().find(|c| c.get("level").and_then(Value::as_str) == Some("note"))).and_then(|c| c.get("message")).and_then(Value::as_str);
        let text = m.get("message").and_then(Value::as_str).unwrap_or("a disallowed method");
        let message = match reason {
            Some(r) => format!("{text} – {r} [clippy::disallowed_methods]"),
            None => format!("{text} [clippy::disallowed_methods]"),
        };
        out.push(Diag { file, line, column, level: "warning".to_string(), message });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(level: &str, text: &str, spans: &str, code: &str) -> String {
        format!(r#"{{"reason":"compiler-message","package_id":"path+file:///w#member@0.0.0","message":{{"message":"{text}","code":{code},"level":"{level}","spans":{spans},"children":[],"rendered":"x"}}}}"#)
    }

    #[test]
    fn an_error_has_its_primary_span() {
        let spans = r#"[{"file_name":"src/osc.rs","line_start":7,"column_start":13,"is_primary":false},{"file_name":"src/lib.rs","line_start":3,"column_start":9,"is_primary":true}]"#;
        let out = parse_cargo_json(&message("error", "cannot find value `x` in this scope", spans, r#"{"code":"E0425","explanation":null}"#));
        assert_eq!(out, vec![Diag { file: "lib.rs".into(), line: 3, column: 9, level: "error".into(), message: "cannot find value `x` in this scope [E0425]".into() }]);
    }

    #[test]
    fn without_a_span_there_is_no_place_and_without_a_code_no_suffix() {
        let out = parse_cargo_json(&message("error", "linking failed", "[]", "null"));
        assert_eq!(out, vec![Diag::general("linking failed")]);
        let first = parse_cargo_json(&message("warning", "unused variable: `x`", r#"[{"file_name":"src/lib.rs","line_start":1,"column_start":5,"is_primary":false}]"#, "null"));
        assert_eq!((first[0].file.as_str(), first[0].line, first[0].level.as_str()), ("lib.rs", 1, "warning"), "no primary: the first span");
    }

    #[test]
    fn the_summary_notes_and_other_lines_are_skipped() {
        let stdout = [
            message("error", "aborting due to 1 previous error", "[]", "null"),
            message("warning", "1 warning emitted", "[]", "null"),
            message("note", "required by a bound", "[]", "null"),
            message("help", "consider borrowing", "[]", "null"),
            r#"{"reason":"compiler-artifact","package_id":"x"}"#.to_string(),
            r#"{"reason":"build-finished","success":false}"#.to_string(),
            "   Compiling member v0.0.0".to_string(),
            "not json {".to_string(),
            message("error", "mismatched types", r#"[{"file_name":"src\\lib.rs","line_start":2,"column_start":1,"is_primary":true}]"#, r#"{"code":"E0308"}"#),
        ]
        .join("\n");
        let out = parse_cargo_json(&stdout);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].file.as_str(), out[0].line, out[0].message.as_str()), ("lib.rs", 2, "mismatched types [E0308]"), "a Windows path becomes the member's file");
    }

    #[test]
    fn a_disallowed_method_warning_carries_its_reason() {
        let clippy = r#"{"reason":"compiler-message","package_id":"x","message":{"message":"use of a disallowed method `f32::sin`","code":{"code":"clippy::disallowed_methods","explanation":null},"level":"warning","spans":[{"file_name":"src\\lib.rs","line_start":4,"column_start":29,"is_primary":true}],"children":[{"level":"note","message":"use deck_module::math::sin (libm)","spans":[]},{"level":"help","message":"for further information visit https://x","spans":[]}],"rendered":"x"}}"#;
        let other = message("warning", "this could be a `let` binding", r#"[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"is_primary":true}]"#, r#"{"code":"clippy::needless_late_init"}"#);
        let out = parse_lints(&[other, clippy.to_string(), "not json".to_string()].join("\n"));
        assert_eq!(out, vec![Diag { file: "lib.rs".into(), line: 4, column: 29, level: "warning".into(), message: "use of a disallowed method `f32::sin` – use deck_module::math::sin (libm) [clippy::disallowed_methods]".into() }]);
        // rustc's own warnings come from the build, not again from the clippy run
        assert!(parse_lints(&message("warning", "unused variable", "[]", "null")).is_empty());
    }

    #[test]
    fn a_real_rustc_message_parses() {
        // `rustc --error-format=json` of `fn main() { x }` as cargo wraps it (trimmed to the fields that matter)
        let stdout = r#"{"reason":"compiler-message","package_id":"path+file:///w/member#0.0.0","manifest_path":"/w/member/Cargo.toml","target":{"kind":["cdylib"],"name":"member"},"message":{"$message_type":"diagnostic","message":"cannot find value `x` in this scope","code":{"code":"E0425","explanation":"…"},"level":"error","spans":[{"file_name":"src/lib.rs","byte_start":12,"byte_end":13,"line_start":1,"line_end":1,"column_start":13,"column_end":14,"is_primary":true,"text":[],"label":"not found in this scope","suggested_replacement":null,"expansion":null}],"children":[],"rendered":"error[E0425]: …"}}"#;
        assert_eq!(parse_cargo_json(stdout), vec![Diag { file: "lib.rs".into(), line: 1, column: 13, level: "error".into(), message: "cannot find value `x` in this scope [E0425]".into() }]);
    }
}
