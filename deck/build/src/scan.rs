//! The reject scan of a member's sources (RIG-EDITING.md §4 "Hostile code"): what a module may not even try. The real
//! protection is structural – the service writes the `Cargo.toml` (no dependency, no `build.rs`, no proc-macro of the
//! member's own, offline) and never runs the code – so this scan is the second line: it refuses, with the line, the few
//! constructs that would reach beyond the member's own files or the SDK at compile time.
//!
//! It reads rustc-style tokens (`proc_macro2`'s lexer), not text: a string or a comment never trips it, and a macro
//! body is scanned like code (`macro_rules! m { () => { include_str!("/etc/passwd") } }` is refused).
//!
//! Refused: `extern crate` · attributes `#[path]`, `#[macro_use]`, `#[link]`, `#[link_name]`, `#[link_section]`,
//! `#[export_name]`, `#[no_link]`, `#[global_allocator]`, `#[crate_type]`, `#[crate_name]`, `#[proc_macro*]` (also inside
//! `cfg_attr`) · the macros `include!`, `include_str!`, `include_bytes!`, `env!`, `option_env!`, `asm!`, `global_asm!`,
//! `naked_asm!` · `std::process`.
use proc_macro2::{Delimiter, TokenStream, TokenTree};

use crate::diag::Diag;
use crate::files::Files;

const BANNED_ATTRIBUTES: &[&str] = &[
    "path", "macro_use", "link", "link_name", "link_section", "export_name", "no_link", "global_allocator", "crate_type", "crate_name",
    "proc_macro", "proc_macro_derive", "proc_macro_attribute",
];
const BANNED_MACROS: &[&str] = &["include", "include_str", "include_bytes", "env", "option_env", "asm", "global_asm", "naked_asm"];
const STD_ROOTS: &[&str] = &["std", "core", "alloc"];

/// What the scan found
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub rejected: Vec<Diag>,
    /// files the lexer could not read; rustc is left to say what is wrong (and if it accepts one anyway, the compile is refused)
    pub unlexable: Vec<String>,
}

pub fn scan(files: &Files) -> Scan {
    let mut out = Scan::default();
    for (name, source) in files {
        match source.parse::<TokenStream>() {
            Ok(stream) => walk(&stream.into_iter().collect::<Vec<_>>(), name, false, &mut out.rejected),
            Err(_) => out.unlexable.push(name.clone()),
        }
    }
    out
}

fn ident_name(t: &TokenTree) -> Option<String> {
    match t {
        TokenTree::Ident(i) => {
            let s = i.to_string();
            Some(s.strip_prefix("r#").map(str::to_string).unwrap_or(s))
        }
        _ => None,
    }
}

fn is_punct(t: Option<&TokenTree>, c: char) -> bool {
    matches!(t, Some(TokenTree::Punct(p)) if p.as_char() == c)
}

fn reject(out: &mut Vec<Diag>, file: &str, at: &TokenTree, message: String) {
    let start = at.span().start();
    out.push(Diag::error(file, start.line as u32, start.column as u32 + 1, message));
}

/// does `group` hold the identifier `name` at its top level (`use std::{fmt, process}`)
fn holds_ident(t: Option<&TokenTree>, name: &str) -> bool {
    match t {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => g.stream().into_iter().any(|t| ident_name(&t).as_deref() == Some(name)),
        Some(t) => ident_name(t).as_deref() == Some(name),
        None => false,
    }
}

fn walk(tokens: &[TokenTree], file: &str, in_attr: bool, out: &mut Vec<Diag>) {
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        match t {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                // `#[…]` and `#![…]`: the bracket group is an attribute; its idents are checked against the attribute list
                let bracket = if is_punct(tokens.get(i + 1), '!') { i + 2 } else { i + 1 };
                if let Some(TokenTree::Group(g)) = tokens.get(bracket) {
                    if g.delimiter() == Delimiter::Bracket {
                        walk(&g.stream().into_iter().collect::<Vec<_>>(), file, true, out);
                        i = bracket + 1;
                        continue;
                    }
                }
            }
            TokenTree::Group(g) => walk(&g.stream().into_iter().collect::<Vec<_>>(), file, in_attr, out),
            TokenTree::Ident(_) => {
                let name = ident_name(t).unwrap_or_default();
                let next = tokens.get(i + 1);
                if in_attr && BANNED_ATTRIBUTES.contains(&name.as_str()) {
                    reject(out, file, t, format!("the attribute `{name}` is not available in a module: it works with the SDK and its own files only"));
                }
                if name == "extern" && next.and_then(ident_name).as_deref() == Some("crate") {
                    reject(out, file, t, "`extern crate` is not available: a module links the SDK and nothing else".into());
                }
                // `include_str!(…)` – but not `include != x`
                if BANNED_MACROS.contains(&name.as_str()) && is_punct(next, '!') && !is_punct(tokens.get(i + 2), '=') {
                    reject(out, file, t, format!("`{name}!` is not available in a module: it cannot read files, the environment or write assembly"));
                }
                // std::process…, core::process… (the roots) and a bare `process::` path (`use std::process; process::exit(1)`)
                let path_after = is_punct(next, ':') && is_punct(tokens.get(i + 2), ':');
                let method_call = i > 0 && is_punct(tokens.get(i - 1), '.');
                if (STD_ROOTS.contains(&name.as_str()) && path_after && holds_ident(tokens.get(i + 3), "process")) || (name == "process" && path_after && !method_call) {
                    reject(out, file, t, "`std::process` is not available in a module".into());
                }
            }
            _ => {}
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_one(source: &str) -> Scan {
        scan(&Files::from([("lib.rs".to_string(), source.to_string())]))
    }

    fn refused(source: &str) -> Vec<(u32, u32, String)> {
        scan_one(source).rejected.into_iter().map(|d| (d.line, d.column, d.message)).collect()
    }

    #[test]
    fn the_walk_module_passes() {
        let walk = include_str!("../../modules/walk/src/lib.rs");
        assert_eq!(scan_one(walk), Scan::default());
    }

    #[test]
    fn the_constructs_that_reach_out_are_refused_with_their_place() {
        // (source, the token the message is about – its first occurrence is the place, the text of the message)
        let cases = [
            ("extern crate std;", "extern", "extern crate"),
            ("fn a() {}
  extern crate alloc as a;", "extern", "extern crate"),
            ("const X: &str = include_str!(\"/etc/passwd\");", "include_str", "include_str!"),
            ("const X: &[u8] = include_bytes!(\"x\");", "include_bytes", "include_bytes!"),
            ("include!(\"x.rs\");", "include", "include!"),
            ("const X: &str = env!(\"HOME\");", "env", "env!"),
            ("const X: Option<&str> = option_env!(\"HOME\");", "option_env", "option_env!"),
            ("fn a() { unsafe { core::arch::asm!(\"nop\"); } }", "asm", "asm!"),
            ("#[path = \"/etc/passwd\"]
mod a;", "path", "path"),
            ("#[macro_use]
mod a {}", "macro_use", "macro_use"),
            ("#![crate_type = \"proc-macro\"]", "crate_type", "crate_type"),
            ("#[proc_macro]
fn m() {}", "proc_macro", "proc_macro"),
            ("#[no_mangle]
#[link_section = \".x\"]
static X: u8 = 0;", "link_section", "link_section"),
            ("#[export_name = \"dm_new\"]
fn x() {}", "export_name", "export_name"),
            ("#[cfg_attr(all(), path = \"x\")]
mod a;", "path", "path"),
            ("#[doc = include_str!(\"x.md\")]
fn a() {}", "include_str", "include_str!"),
            ("fn a() { std::process::exit(1); }", "std", "std::process"),
            ("use std::{fmt, process};", "std", "std::process"),
            ("use std::process as p;", "std", "std::process"),
            ("use std::process;
fn a() { process::exit(1) }", "std", "std::process"),
            ("fn a() { 1; }
use std::process;
fn b() { process::exit(1) }", "process::exit", "std::process"),
            ("macro_rules! m { () => { include_str!(\"/etc/passwd\") }; }", "include_str", "include_str!"),
        ];
        for (source, needle, part) in cases {
            let at = source.find(needle).unwrap();
            let line = source[..at].matches('\n').count() as u32 + 1;
            let column = (source[..at].rsplit('\n').next().unwrap().chars().count() + 1) as u32;
            let found = refused(source);
            assert!(found.iter().any(|(l, c, m)| (*l, *c) == (line, column) && m.contains(part)), "{source:?}: wanted {line}:{column} {part}, got {found:?}");
        }
    }

    #[test]
    fn ordinary_code_is_left_alone() {
        let ok = r#"
            // extern crate x; include_str!("a"); std::process::exit(1)
            const S: &str = "extern crate include_str!(x) #[path]";
            struct B; impl B { fn process(&mut self, x: u8) -> u8 { x } }
            fn call(b: &mut B) -> u8 { let y = Self::process; b.process(1) + b.process::<u8>(2) + env::<u8>() }
            fn path() {} fn link() {} fn env<T>() -> u8 { 0 } fn include() {} fn include_not(a: bool) -> bool { a != true }
            #[derive(Clone)] struct P { path: u8, link: u8, env: u8 }
            #[allow(clippy::duplicated_attributes)] fn f() { let include = 3; let _ = include != 4; }
            mod osc; use osc::Osc; pub mod process_not {}
        "#;
        assert_eq!(refused(ok), Vec::<(u32, u32, String)>::new());
    }

    #[test]
    fn a_file_the_lexer_cannot_read_is_noted_for_rustc() {
        let s = scan_one("fn a() { \"unterminated }");
        assert_eq!((s.rejected.len(), s.unlexable.as_slice()), (0, ["lib.rs".to_string()].as_slice()));
    }

    #[test]
    fn every_file_is_scanned() {
        let files = Files::from([("lib.rs".to_string(), "mod osc;".to_string()), ("osc.rs".to_string(), "\n\nextern crate x;".to_string())]);
        let s = scan(&files);
        assert_eq!(s.rejected.iter().map(|d| (d.file.as_str(), d.line)).collect::<Vec<_>>(), [("osc.rs", 3)]);
    }
}
