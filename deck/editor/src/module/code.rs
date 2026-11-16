//! The external code editor (decisions/deck.md 2026-11-09 [platform]: the Rust tab stays outside for now): a command line
//! with `{dir}`, `{file}`, `{line}`, `{column}` filled in. `DECK_CODE_EDITOR` sets it; VS Code is the default.
use std::path::Path;

pub const DEFAULT: &str = "code {dir} --goto {file}:{line}:{column}";

#[derive(Debug, Clone, PartialEq)]
pub struct Editor {
    pub template: String,
}

impl Editor {
    pub fn from_env() -> Editor {
        Editor { template: std::env::var("DECK_CODE_EDITOR").ok().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| DEFAULT.into()) }
    }

    /// the program and its arguments for `file` at `line`:`column` in the working copy `dir`
    pub fn command(&self, dir: &Path, file: &Path, line: u32, column: u32) -> Vec<String> {
        let fill = |word: &str| word.replace("{dir}", &dir.display().to_string()).replace("{file}", &file.display().to_string()).replace("{line}", &line.max(1).to_string()).replace("{column}", &column.max(1).to_string());
        let words: Vec<String> = self.template.split_whitespace().map(fill).collect();
        // `code` is a .cmd on Windows: only the shell finds it
        if cfg!(windows) && words.first().is_some_and(|p| Path::new(p).extension().is_none()) {
            return ["cmd".to_string(), "/C".to_string()].into_iter().chain(words).collect();
        }
        words
    }

    pub fn open(&self, dir: &Path, file: &Path, line: u32, column: u32) -> Result<(), String> {
        let words = self.command(dir, file, line, column);
        let (program, args) = words.split_first().ok_or("DECK_CODE_EDITOR is empty")?;
        std::process::Command::new(program).args(args).spawn().map(drop).map_err(|e| format!("{program}: {e} – set DECK_CODE_EDITOR to your editor's command"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_are_filled_in() {
        let e = Editor { template: "zed {dir} {file}:{line}:{column}".into() };
        let words = e.command(Path::new("/w/module_walk"), Path::new("/w/module_walk/src/lib.rs"), 12, 0);
        let tail: Vec<&str> = words.iter().map(String::as_str).skip_while(|w| *w != "zed").collect();
        assert_eq!(tail, ["zed", "/w/module_walk", "/w/module_walk/src/lib.rs:12:1"]);
    }

    #[test]
    fn vs_code_is_the_default_and_goes_through_the_shell_on_windows() {
        let e = Editor { template: DEFAULT.into() };
        let words = e.command(Path::new("d"), Path::new("d/src/lib.rs"), 3, 4);
        if cfg!(windows) {
            assert_eq!(&words[..3], ["cmd", "/C", "code"]);
        } else {
            assert_eq!(words[0], "code");
        }
        assert!(words.ends_with(&["d".to_string(), "--goto".to_string(), "d/src/lib.rs:3:4".to_string()]));
        let exe = Editor { template: "C:/Tools/editor.exe {file}".into() };
        assert_eq!(exe.command(Path::new("d"), Path::new("f.rs"), 1, 1), ["C:/Tools/editor.exe", "f.rs"]);
    }
}
