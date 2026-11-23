//! The demos the editor lists (deck/demos): every show opens and validates, its first row says what to try, the rigs and
//! the story events beside them read.
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../demos")
}

fn shows() -> Vec<(String, deck_show::Show)> {
    let mut out: Vec<(String, deck_show::Show)> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            let name = p.file_name()?.to_str()?.strip_suffix(".show.json")?.to_string();
            let text = std::fs::read_to_string(&p).unwrap();
            Some((name.clone(), deck_show::document::open(&text).unwrap_or_else(|e| panic!("{name}: {e}"))))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn every_demo_validates_and_says_what_to_try() {
    let shows = shows();
    assert!(shows.len() >= 4, "{:?}", shows.iter().map(|s| &s.0).collect::<Vec<_>>());
    for (name, show) in &shows {
        assert_eq!(&show.id, name, "a demo's id is its file name");
        assert_eq!(deck_show::validate(show), vec![], "{name}");
        assert!(show.rows.first().is_some_and(|r| r.id.starts_with("try: ")), "{name}: the first row says what to try");
        // a check run names a cell as ROW,COLUMN
        assert!(show.rows.iter().all(|r| !r.id.contains(',')), "{name}: a row id without a comma");
    }
}

#[test]
fn the_rigs_a_demo_plays_are_beside_it() {
    for (name, show) in shows() {
        for row in &show.rows {
            for cell in row.cells.values() {
                let piece = match cell {
                    deck_show::document::Cell::Play { piece: Some(p), .. } | deck_show::document::Cell::Values { piece: p, .. } => p,
                    _ => continue,
                };
                let Some(id) = piece.strip_prefix("character:") else { continue };
                let rig = dir().join("rigs").join(format!("{id}.toml"));
                let text = std::fs::read_to_string(&rig).unwrap_or_else(|e| panic!("{name}: {}: {e}", rig.display()));
                assert!(text.contains(&format!("subject = \"character.{id}\"")), "{name}: {}", rig.display());
            }
        }
    }
}

#[test]
fn the_story_events_hold_the_vote_demos_event() {
    let text = std::fs::read_to_string(dir().join("story-events.json")).unwrap();
    let story = deck_show::story::read(&text, "catpope").unwrap();
    let show = shows().into_iter().find(|(n, _)| n == "vote-and-question").unwrap().1;
    let fired: Vec<&str> = show.rows.iter().flat_map(|r| r.fire.event.iter()).collect();
    assert!(fired.contains(&"ch3.door"));
    assert!(format!("{story:?}").contains("ch3.door"), "the story's question the right click offers");
}
