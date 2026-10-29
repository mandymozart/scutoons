//! `deck-scene-features <scene.toml>…` – the cargo features (modules) the scenes need, as one `--features` value, so a
//! build carries only those: `cargo build -p deck-core --no-default-features --features "$(deck-scene-features a.toml)"`.
//! Kinds of deck/stage (`stage`) and planned modules are listed too; a build tool maps them to the stage's features.
fn main() {
    let mut kinds = vec![];
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let scene = deck_core::SceneDef::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        kinds.extend(scene.nodes.into_iter().map(|n| n.kind));
    }
    println!("{}", deck_core::features_for(kinds.iter().map(String::as_str)).join(","));
}
