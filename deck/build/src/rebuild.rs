//! `scutoons-build rebuild <dir>` – every module of a folder through the same compile as a request (the scan, the
//! determinism rules, the strict test run), the failures being the list (research/ABI-V2.md §7, #344): "the sources that
//! use a removed API". Nothing is stored and no database is touched; the sources come from a folder, e.g. an export of the
//! catalog's `module:<id>` sources from a *copy* of the data (the server's `modules:rebuild` is the same list from the
//! database side). A folder holds one entry per module: `<id>/src/*.rs` (a cargo layout, `deck/modules`), `<id>/*.rs`
//! or `<id>.rs` (a single file is its `lib.rs`).
use std::path::Path;

use crate::compile::{Built, CompileError, Compiler};
use crate::diag::Diag;
use crate::files::{valid_name, Files, MAIN_FILE};

/// the modules in `dir`, by id; an entry that is no module (no `.rs` file) is left out
pub fn read_modules(dir: &Path) -> std::io::Result<Vec<(String, Files)>> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let id = entry.file_name().to_string_lossy().trim_end_matches(".rs").to_string();
        let mut files = Files::new();
        if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
            files.insert(MAIN_FILE.to_string(), std::fs::read_to_string(&path)?);
        } else if path.is_dir() {
            let src = path.join("src");
            let from = if src.is_dir() { src } else { path };
            for file in std::fs::read_dir(&from)?.flatten() {
                let name = file.file_name().to_string_lossy().to_string();
                // a folder inside a module is not part of a flat module: rustc says what is missing
                if file.path().is_file() && valid_name(&name) {
                    files.insert(name, std::fs::read_to_string(file.path())?);
                }
            }
        }
        if !files.is_empty() {
            out.push((id, files));
        }
    }
    Ok(out)
}

/// What became of one module
pub struct Row {
    pub id: String,
    pub result: Result<Built, CompileError>,
}

/// Compiles `modules` one after another
pub async fn rebuild_all(compiler: &Compiler, modules: Vec<(String, Files)>) -> Vec<Row> {
    let mut rows = Vec::new();
    for (id, files) in modules {
        let result = compiler.compile(&files).await;
        rows.push(Row { id, result });
    }
    rows
}

fn diags(errors: &[Diag]) -> String {
    errors
        .iter()
        .map(|d| if d.file.is_empty() { format!("    {}: {}", d.level, d.message) } else { format!("    {}:{}:{} {}: {}", d.file, d.line, d.column, d.level, d.message) })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The report: one block per module, the failures last in the summary line. Returns the text and the number of failures.
pub fn report(rows: &[Row]) -> (String, usize) {
    let mut out = String::new();
    let mut failed = Vec::new();
    for row in rows {
        match &row.result {
            Ok(built) => {
                out.push_str(&format!("{}\tOK\tABI {}, {} bytes, {}\n", row.id, built.descriptor["abi"], built.wasm.len(), &built.hash[..12]));
                if !built.warnings.is_empty() {
                    out.push_str(&format!("{}\n", diags(&built.warnings)));
                }
            }
            Err(CompileError::Rejected(errors) | CompileError::Failed(errors)) => {
                out.push_str(&format!("{}\tFAILED\n{}\n", row.id, diags(errors)));
                failed.push(row.id.as_str());
            }
            Err(CompileError::LiveShow) => {
                out.push_str(&format!("{}\tFAILED\n    a live show is running\n", row.id));
                failed.push(row.id.as_str());
            }
            Err(CompileError::Internal(message)) => {
                out.push_str(&format!("{}\tFAILED\n    the service itself: {message}\n", row.id));
                failed.push(row.id.as_str());
            }
        }
    }
    out.push_str(&format!("rebuild: {} module(s), {} failed{}\n", rows.len(), failed.len(), if failed.is_empty() { String::new() } else { format!(": {}", failed.join(", ")) }));
    (out, failed.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn a_folder_of_modules_in_any_of_the_three_layouts() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "cargo/src/lib.rs", "// a");
        write(dir.path(), "cargo/src/osc.rs", "// b");
        write(dir.path(), "cargo/Cargo.toml", "[package]");
        write(dir.path(), "flat/lib.rs", "// c");
        write(dir.path(), "single.rs", "// d");
        write(dir.path(), "split/src/lib.rs", "mod dsp;");
        write(dir.path(), "split/src/dsp/mod.rs", "// not flat");
        write(dir.path(), "empty/readme.txt", "no module");
        let modules = read_modules(dir.path()).unwrap();
        let ids: Vec<_> = modules.iter().map(|(id, f)| (id.as_str(), f.keys().map(String::as_str).collect::<Vec<_>>())).collect();
        assert_eq!(ids, [("cargo", vec!["lib.rs", "osc.rs"]), ("flat", vec!["lib.rs"]), ("single", vec!["lib.rs"]), ("split", vec!["lib.rs"])]);
    }

    #[test]
    fn the_failures_are_the_list() {
        let built = Built { hash: "ab".repeat(32), descriptor: serde_json::json!({ "abi": 2 }), wasm: vec![0; 10], warnings: vec![Diag { level: "warning".into(), ..Diag::error("lib.rs", 4, 2, "use of a disallowed method") }] };
        let rows = vec![
            Row { id: "good".into(), result: Ok(built) },
            Row { id: "old".into(), result: Err(CompileError::Failed(vec![Diag::error("lib.rs", 3, 9, "cannot find type `Ctx` [E0412]")])) },
            Row { id: "bad".into(), result: Err(CompileError::Rejected(vec![Diag::error("lib.rs", 1, 1, "include_str! reaches outside")])) },
        ];
        let (text, failed) = report(&rows);
        assert_eq!(failed, 2);
        assert!(text.contains("good\tOK\tABI 2, 10 bytes, abababababab\n    lib.rs:4:2 warning: use of a disallowed method"), "{text}");
        assert!(text.contains("old\tFAILED\n    lib.rs:3:9 error: cannot find type `Ctx` [E0412]"), "{text}");
        assert!(text.ends_with("rebuild: 3 module(s), 2 failed: old, bad\n"), "{text}");
        assert_eq!(report(&rows[..1]).1, 0);
    }
}
