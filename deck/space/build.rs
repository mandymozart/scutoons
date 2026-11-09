// #423: the build's version, `deck_space::BUILD` (every deck draws it top right, always). The string is
// `<workspace version>.r<commit count>.g<short sha>[.dirty]` – os/version.sh's formula (update.sh puts the same string into
// the update bundle). Where it comes from: the env DECK_BUILD_VERSION (update.sh / os/build.sh pass it in – the Docker build
// has no .git), else git here (a PC build), else `dev`.
use std::path::{Path, PathBuf};
use std::process::Command;

/// a git command's trimmed stdout in `dir`, None when git is missing or the command fails
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().ok().filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// the version from git – os/version.sh's formula
fn from_git(deck: &Path) -> Option<String> {
    let count = git(deck, &["rev-list", "--count", "HEAD"])?;
    let sha = git(deck, &["rev-parse", "--short", "HEAD"])?;
    let mut v = format!("{}.r{count}.g{sha}", std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    if git(deck, &["status", "--porcelain", "--", "."]).is_some_and(|s| !s.is_empty()) {
        v.push_str(".dirty");
    }
    Some(v)
}

/// a git path (`--git-path` is right in a worktree, where .git is a file) made absolute
fn git_path(deck: &Path, what: &str) -> Option<PathBuf> {
    let p = PathBuf::from(git(deck, &["rev-parse", "--git-path", what])?);
    Some(if p.is_absolute() { p } else { deck.join(p) })
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DECK_BUILD_VERSION");
    let deck = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("..");
    let env = std::env::var("DECK_BUILD_VERSION").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let version = match env {
        Some(v) => v,
        None => {
            // a rebuild when HEAD moves (the branch's ref, packed refs), when something is staged, and when a source folder of
            // deck/ changes (the `.dirty` mark) – not deck/target (every build writes it) and not deck/os (no code)
            let mut watch: Vec<PathBuf> = ["HEAD", "index", "packed-refs"].iter().filter_map(|w| git_path(&deck, w)).collect();
            if let Some(r) = git(&deck, &["symbolic-ref", "-q", "HEAD"]).and_then(|r| git_path(&deck, &r)) {
                watch.push(r);
            }
            if let Ok(rd) = std::fs::read_dir(&deck) {
                for e in rd.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() && !n.starts_with('.') && n != "target" && n != "os" {
                        watch.push(e.path());
                    }
                }
            }
            watch.extend(["Cargo.toml", "Cargo.lock"].iter().map(|f| deck.join(f)));
            for p in watch {
                println!("cargo:rerun-if-changed={}", p.display());
            }
            from_git(&deck).unwrap_or_else(|| "dev".into())
        }
    };
    println!("cargo:rustc-env=DECK_BUILD={version}");
}
