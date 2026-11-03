//! the bundled scenarios (deck/headless/scenarios) with the deck/ files they name
use deck_headless::Bundle;
use std::path::PathBuf;

pub fn deck_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub fn bundles() -> Vec<Bundle> {
    let deck = deck_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(deck.join("headless/scenarios"))
        .expect("the scenarios folder")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|f| {
            let scenario = std::fs::read_to_string(f).expect("a scenario");
            let name = f.file_stem().unwrap().to_string_lossy().into_owned();
            deck_headless::bundle(&name, scenario, |p| std::fs::read_to_string(deck.join(p)).map_err(|e| e.to_string())).expect("its files")
        })
        .collect()
}
