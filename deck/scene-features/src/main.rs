//! The cargo features a deck build needs, as one `--features` value, so the deck carries only those modules:
//!   `deck-scene-features <scene.toml | dir>…`  what the scenes need (a dir: its `*.toml`)
//!   `deck-scene-features --profile os/profiles.toml <class> [--host] <scene.toml | dir>…`  an image class's deck: its
//!     platform, its modules, what its scenes and the core rigs every deck carries need (`--host`: without the platform)
//!   `deck-scene-features --apps os/profiles.toml <class>`  the cargo packages the class builds beside the deck
//!   `cargo build -p deck-native --no-default-features --features "$(deck-scene-features --profile os/profiles.toml lean native/scenes)"`
//! An unknown kind, tempo source or class is an error (exit 1), named with its file.
use std::collections::BTreeSet;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => println!("{}", out.join(",")),
        Err(e) => {
            eprintln!("deck-scene-features: {e}");
            std::process::exit(1)
        }
    }
}

fn run(args: &[String]) -> Result<Vec<String>, String> {
    match args.first().map(String::as_str) {
        Some("--apps") => {
            let [_, file, class] = args else { return Err("usage: --apps <profiles.toml> <class>".into()) };
            Ok(Profile::read(file, class)?.apps)
        }
        Some("--profile") => {
            let [_, file, class, rest @ ..] = args else { return Err("usage: --profile <profiles.toml> <class> [--host] <scenes>…".into()) };
            let host = rest.first().is_some_and(|a| a == "--host");
            let scenes = if host { &rest[1..] } else { rest };
            let profile = Profile::read(file, class)?;
            let mut out: BTreeSet<String> = features(&scene_files(scenes)?)?.into_iter().map(String::from).collect();
            out.extend(core_rigs()?.into_iter().map(String::from));
            out.extend(profile.modules);
            if !host {
                out.extend(profile.platform);
            }
            Ok(out.into_iter().collect())
        }
        _ => Ok(features(&scene_files(args)?)?.into_iter().map(String::from).collect()),
    }
}

/// An image class of os/profiles.toml, its modules resolved against deck-core's stage table
#[derive(Debug)]
struct Profile {
    apps: Vec<String>,
    platform: Vec<String>,
    modules: Vec<String>,
}

impl Profile {
    fn read(file: &str, class: &str) -> Result<Profile, String> {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        Profile::parse(&text, class).map_err(|e| format!("{file}: {e}"))
    }

    fn parse(text: &str, class: &str) -> Result<Profile, String> {
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let Some(c) = table.get(class).and_then(|v| v.as_table()) else {
            let known: Vec<&str> = table.keys().map(String::as_str).collect();
            return Err(format!("no class '{class}' ({})", known.join(", ")));
        };
        let list = |key: &str| -> Result<Vec<String>, String> {
            match c.get(key) {
                None => Ok(Vec::new()),
                Some(toml::Value::Array(a)) => a.iter().map(|v| v.as_str().map(String::from).ok_or(format!("{class}.{key}: not text"))).collect(),
                Some(_) => Err(format!("{class}.{key}: not a list")),
            }
        };
        let without = list("without")?;
        let modules = match c.get("modules") {
            Some(toml::Value::String(s)) if s == "all" => deck_core::stage::STAGE_MODULES
                .iter()
                .map(|m| m.feature)
                .filter(|f| *f != deck_core::stage::CORE && !without.iter().any(|w| w == f))
                .map(String::from)
                .collect(),
            _ => list("modules")?,
        };
        for m in &modules {
            if !deck_core::stage::STAGE_MODULES.iter().any(|s| s.feature == m) {
                return Err(format!("{class}.modules: '{m}' is no stage module"));
            }
        }
        Ok(Profile { apps: list("apps")?, platform: list("platform")?, modules })
    }
}

/// the scene files of the arguments: a file as it is, a directory's `*.toml` (not its subfolders), sorted
fn scene_files(args: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for a in args {
        if Path::new(a).is_dir() {
            let mut files: Vec<String> = std::fs::read_dir(a)
                .map_err(|e| format!("{a}: {e}"))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "toml"))
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            files.sort();
            out.extend(files);
        } else {
            out.push(a.clone());
        }
    }
    Ok(out)
}

/// what the core rigs every deck carries need when they play (deck-space rig::bundle::CORE)
fn core_rigs() -> Result<Vec<&'static str>, String> {
    let mut out = BTreeSet::new();
    for (subject, _) in deck_space::rig::bundle::CORE {
        let turn = deck_space::deck::Deck::new().play_core(subject, "");
        if turn.failed || !turn.rig {
            return Err(format!("core rig {subject}: {}", turn.note));
        }
        let nodes = &turn.played.nodes;
        out.extend(deck_core::stage::features(nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str(), &n.params))).map_err(|e| format!("core rig {subject}: {e}"))?);
    }
    Ok(out.into_iter().collect())
}

/// the features the scene files need, sorted, without duplicates
fn features<'a>(files: impl IntoIterator<Item = &'a String>) -> Result<Vec<&'static str>, String> {
    let mut out = BTreeSet::new();
    for path in files {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        out.extend(deck_core::stage::scene_features(&text).map_err(|e| format!("{path}: {e}"))?);
        // the scene's [outputs] table (not a node) needs the module too
        let table: toml::Table = text.parse().map_err(|e| format!("{path}: {e}"))?;
        if table.contains_key("outputs") {
            out.insert("outputs");
        }
    }
    Ok(out.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

    fn scene(name: &str) -> String {
        let system = format!("{DECK}/native/scenes/{name}.toml");
        if Path::new(&system).is_file() { system } else { format!("{DECK}/native/scenes/sandbox/{name}.toml") }
    }

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.replace("{deck}", DECK)).collect()
    }

    /// deck-native's cargo features (its Cargo.toml)
    fn native_features() -> BTreeSet<String> {
        let text = std::fs::read_to_string(format!("{DECK}/native/Cargo.toml")).unwrap();
        let table: toml::Table = text.parse().unwrap();
        table["features"].as_table().unwrap().keys().cloned().collect()
    }

    #[test]
    fn the_bundled_scenes() {
        let one = |n: &str| features([&scene(n)]).unwrap();
        assert_eq!(one("madness"), ["chapter"]);
        assert!(one("idle").is_empty(), "the idle scene is the core alone");
        assert_eq!(one("cathedral"), ["chapter", "space"]);
        assert!(one("test-pattern").is_empty(), "the core alone");
        assert_eq!(one("strobe"), ["outputs"], "the [outputs] table");
        let all: Vec<String> = ["madness", "cathedral", "test-pattern"].map(scene).into();
        assert_eq!(features(&all).unwrap(), ["chapter", "space"]);
    }

    #[test]
    fn the_core_rigs() {
        assert_eq!(core_rigs().unwrap(), ["events", "space"], "catpope's melody is a seeded walk, the being stands in its space");
    }

    #[test]
    fn a_directory_is_its_toml_files() {
        let dir = run(&args(&["{deck}/native/scenes"])).unwrap();
        assert!(dir.contains(&"live".into()), "the outlet scene: {dir:?}");
        assert!(!dir.contains(&"modules".into()), "not the sandbox folder: {dir:?}");
    }

    #[test]
    fn the_classes() {
        let profiles = "{deck}/os/profiles.toml";
        let native = native_features();
        let lean = run(&args(&["--profile", profiles, "lean", "{deck}/native/scenes"])).unwrap();
        let full = run(&args(&["--profile", profiles, "full", "{deck}/native/scenes"])).unwrap();
        for f in lean.iter().chain(&full) {
            assert!(native.contains(f), "'{f}' is no deck-native feature");
        }
        for f in ["pipewire", "live", "space", "sync"] {
            assert!(lean.contains(&f.into()), "lean lacks {f}: {lean:?}");
        }
        for f in core_rigs().unwrap() {
            assert!(lean.contains(&f.into()), "the core rigs need {f}: {lean:?}");
        }
        for heavy in ["modules", "gamepad", "midi", "osc", "outputs", "link"] {
            assert!(!lean.contains(&heavy.into()), "lean carries {heavy}: {lean:?}");
        }
        for f in ["pipewire", "gamepad", "modules", "space", "sync", "chapter", "live", "midi", "osc", "outputs", "events"] {
            assert!(full.contains(&f.into()), "full lacks {f}: {full:?}");
        }
        assert!(!full.contains(&"link".into()));
        let host = run(&args(&["--profile", profiles, "lean", "--host", "{deck}/native/scenes"])).unwrap();
        assert!(!host.contains(&"pipewire".into()) && host.contains(&"live".into()), "{host:?}");
        let sandbox = run(&args(&["--profile", profiles, "lean", "{deck}/native/scenes", "{deck}/native/scenes/sandbox"])).unwrap();
        assert!(sandbox.contains(&"modules".into()), "a dev image's sandbox scenes add their modules: {sandbox:?}");
        assert_eq!(run(&args(&["--apps", profiles, "full"])).unwrap(), ["deck-editor"]);
        assert!(run(&args(&["--apps", profiles, "lean"])).unwrap().is_empty());
    }

    #[test]
    fn errors_name_the_file_and_the_class() {
        let e = features([&"no/such.toml".to_string()]).unwrap_err();
        assert!(e.starts_with("no/such.toml: "), "{e}");
        assert!(Profile::parse("[lean]\n", "tiny").unwrap_err().contains("no class 'tiny' (lean)"));
        assert!(Profile::parse("[x]\nmodules = [\"teleport\"]\n", "x").unwrap_err().contains("'teleport' is no stage module"));
    }
}
