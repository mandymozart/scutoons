//! #318: the multichannel check scenes (scenes/channel-walk, surround-51, cue-34) build and play where they say – on the
//! X1's RME in its Pro Audio profile (20 channels) and on a stereo device. The walk's LFO steps in the engine (#319) and
//! starts at the front: in its first tenth of a second (< 3°) the two speakers either side of it (channels 4 and 5) play.
use deck_engine::{Build, DeckConfig, Engine, Seams, Sequencing};
use deck_space::graph::Graph;
use deck_space::scene::Scene;

/// a tenth of a second of every device channel's peak
fn peaks(file: &str, deck: DeckConfig) -> Vec<f32> {
    let text = std::fs::read_to_string(format!("{}/scenes/sandbox/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let graph = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
    let d = deck.channels;
    let (mut e, _) = Engine::build_seams(&graph, 48_000.0, 1.0, Build::detect(), Sequencing::InBlock, &Seams::with_deck(deck)).unwrap();
    let mut buf = vec![0.0f32; 4_800 * d];
    e.render(&mut buf, d, |x| x);
    (0..d).map(|c| buf.chunks_exact(d).fold(0.0f32, |m, f| m.max(f[c].abs()))).collect()
}

fn loud(p: &[f32]) -> Vec<usize> {
    (0..p.len()).filter(|&c| p[c] > 0.01).map(|c| c + 1).collect()
}

#[test]
fn the_channel_check_scenes_play_where_they_say() {
    let rme = || DeckConfig::device(20);
    assert_eq!(loud(&peaks("channel-walk.toml", rme())), [4, 5], "the walk at the front: channels 4 and 5");
    assert_eq!(loud(&peaks("surround-51.toml", rme())), [1, 2, 3, 4, 5, 6], "the 5.1 bed on outs 1–6");
    assert_eq!(loud(&peaks("cue-34.toml", rme())), [1, 2, 3, 4], "main 1/2, cue 3/4");
    // a stereo device: the bed folded, the cue silent, the walk's first two channels (silent at the front)
    assert_eq!(loud(&peaks("surround-51.toml", DeckConfig::stereo())), [1, 2]);
    let cue = peaks("cue-34.toml", DeckConfig::stereo());
    assert_eq!(loud(&cue), [1, 2]);
    assert!((cue[0] - 0.12).abs() < 0.01, "only the main: {}", cue[0]);
    assert!(loud(&peaks("channel-walk.toml", DeckConfig::stereo())).is_empty());
}
