//! #427: the deck's scenes apart – the system scenes always, the sandboxes (deck/native/scenes/sandbox) only in dev mode:
//! F5's list (switcher.rs) on a release and a dev deck, a sandbox's name by `--scene`, "set as start" apart from loading.
use deck_native::switcher::{entries, read, sandboxes, Key, Switcher, BUILTIN, SANDBOX_DIR};
use std::path::{Path, PathBuf};

fn folder(name: &str, files: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deck-switcher-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::write(dir.join(f), "").unwrap();
    }
    dir
}

/// #427: a release deck lists the four system scenes; dev mode adds the sandbox folder's files, apart
#[test]
fn release_lists_the_system_scenes_dev_mode_adds_the_sandboxes() {
    let release = entries(None);
    assert_eq!(release.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(), ["idle", "silent", "outlet", "test-pattern"]);
    assert!(release.iter().all(|e| !e.sandbox));
    let dir = folder("list", &["zeta.toml", "alpha.toml", "idle.toml", "notes.txt"]);
    let l = entries(Some(&dir));
    let labels: Vec<&str> = l.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(&labels[..BUILTIN.len()], BUILTIN);
    assert_eq!(&labels[BUILTIN.len()..], ["alpha", "zeta"], "a file named like a system scene is skipped");
    assert!(l[BUILTIN.len()].value.ends_with("alpha.toml") && l[BUILTIN.len()..].iter().all(|e| e.sandbox));
    // no folder: the system scenes alone
    assert_eq!(entries(Some(&dir.join("none"))).len(), BUILTIN.len());
    let _ = std::fs::remove_dir_all(&dir);
    // the repo's sandbox folder is a developer's with --dev; --scenes wins
    let repo = sandboxes(None, true).unwrap();
    assert!(Path::new(SANDBOX_DIR).is_dir() || repo.join("cathedral.toml").is_file(), "{}", repo.display());
    assert_eq!(sandboxes(Some("x".into()), false), Some(PathBuf::from("x")));
    if !Path::new(SANDBOX_DIR).is_dir() {
        assert_eq!(sandboxes(None, false), None, "no dev image, no --dev: no sandboxes");
    }
}

/// #427: a sandbox's name plays only where the sandbox folder is (dev mode); a system name always; a file always
#[test]
fn a_sandbox_name_needs_dev_mode() {
    let repo = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/scenes/sandbox"));
    assert_eq!(read(Path::new("cathedral"), Some(&repo)).unwrap().stage.name, "Cathedral of decks");
    let refused = read(Path::new("cathedral"), None).unwrap_err().to_string();
    assert!(refused.contains("sandbox") && refused.contains("dev mode"), "{refused}");
    assert_eq!(read(Path::new("idle"), None).unwrap().stage.name, "idle");
    assert!(read(&repo.join("strobe.toml"), None).is_ok(), "a file plays as a file");
    assert!(read(Path::new("nothing-here"), None).unwrap_err().to_string().contains("reading the scene"));
}

#[test]
fn the_cursor_moves_and_enter_picks() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/scenes/sandbox"));
    let cathedral = dir.join("cathedral.toml").to_string_lossy().into_owned();
    let mut s = Switcher::default();
    s.open(Some(&dir), Some(&cathedral));
    assert_eq!(s.key(Key::Enter).unwrap().label, "cathedral");
    (0..20).for_each(|_| drop(s.key(Key::Up)));
    assert_eq!(s.key(Key::Enter).unwrap().value, "idle");
    for _ in 0..30 {
        s.key(Key::Down);
    }
    assert_eq!(s.key(Key::Enter).unwrap().label, "surround-51");
    let (lines, cursor) = s.lines(Some(&cathedral));
    assert!(lines[cursor].starts_with('>') && lines[cursor].contains("SURROUND-51"));
    assert!(lines.len() <= 13 && lines.iter().all(|l| l.chars().count() <= 39));
    // the sandboxes under their heading, after the system scenes
    (0..30).for_each(|_| drop(s.key(Key::Up)));
    let (top, _) = s.lines(None);
    assert_eq!(top[1..6], [">*IDLE", "  SILENT", "  OUTLET", "  TEST-PATTERN", " -- SANDBOX (DEV) --"]);
    // a release deck: no heading
    s.open(None, None);
    assert!(!s.lines(None).0.iter().any(|l| l.contains("SANDBOX")));
}

/// #427: Enter only loads; S writes the start scene – the file is untouched until then
#[test]
fn set_as_start_is_apart_from_loading() {
    let dir = folder("start", &[]);
    let file = dir.join("deck.toml");
    std::fs::write(&file, "audio_host = \"asio\"\n").unwrap();
    let mut s = Switcher::default();
    s.open(None, None);
    s.key(Key::Down);
    assert_eq!(s.key(Key::Enter).unwrap().value, "silent");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "audio_host = \"asio\"\n", "loading writes nothing");
    s.set_start(&file);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "audio_host = \"asio\"\nscene = \"silent\"\n");
    assert_eq!(s.status, "START SCENE: silent");
    let _ = std::fs::remove_dir_all(&dir);
}
