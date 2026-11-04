//! `deck-scene-features [--core] <scene.toml>…` – the cargo features the scenes need, as one `--features` value, so a
//! deck build carries only those modules (#185):
//!   `cargo build -p deck-native --no-default-features --features "pipewire,$(deck-scene-features scenes/*.toml)"`
//! (the platform's features – pipewire on a deck – come from the build script, not from scenes). An unknown kind or
//! tempo source is an error (exit 1), named with its file.
use std::collections::BTreeSet;

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    match features(&files) {
        Ok(f) => println!("{}", f.join(",")),
        Err(e) => {
            eprintln!("deck-scene-features: {e}");
            std::process::exit(1)
        }
    }
}

/// the features the scene files need, sorted, without duplicates
fn features<'a>(files: impl IntoIterator<Item = &'a String>) -> Result<Vec<&'static str>, String> {
    let mut out = BTreeSet::new();
    for path in files {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        out.extend(deck_core::stage::scene_features(&text).map_err(|e| format!("{path}: {e}"))?);
        // #211: the scene's [outputs] table (not a node) needs the module too
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

    fn scene(name: &str) -> String {
        format!("{}/../native/scenes/{name}.toml", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_bundled_scenes() {
        let one = |n: &str| features([&scene(n)]).unwrap();
        assert_eq!(one("madness"), ["chapter"]);
        assert!(one("idle").is_empty(), "#252: the idle scene is the core alone");
        assert_eq!(one("cathedral"), ["chapter", "space"]);
        assert!(one("test-pattern").is_empty(), "the core alone");
        assert_eq!(one("strobe"), ["outputs"], "#211: the [outputs] table");
        let all: Vec<String> = ["madness", "cathedral", "test-pattern"].map(scene).into();
        assert_eq!(features(&all).unwrap(), ["chapter", "space"]);
    }

    #[test]
    fn errors_name_the_file() {
        let e = features([&"no/such.toml".to_string()]).unwrap_err();
        assert!(e.starts_with("no/such.toml: "), "{e}");
    }
}
