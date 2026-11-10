//! The first use case, headless (owner 2026-11-09, SHOW.md §7): one simulated phone reader connected to the conductor. A
//! chapter event fires its row on auto, is held as a suggestion on suggest, is ignored on hand. No window, device or socket.
use deck_show::conductor::Playing;
use deck_show::{parse, Clock, Conductor, Logged, Now, Origin, Out, Says, Word};
use protocol::{CellMode, DeviceMsg, Envelope, LayerSrc, Message};
use serde_json::json;

const TOP_TOY: &str = include_str!("../fixtures/top-toy.show.json");
const FOYER: &str = include_str!("../fixtures/foyer-loop.show.json");

fn conductor() -> Conductor {
    Conductor::new(parse(TOP_TOY).unwrap())
}

fn child() -> Conductor {
    Conductor::new(parse(FOYER).unwrap())
}

/// A phone reader: all it ever sends is a story event with the id the chapter editor gave it.
struct Reader;

impl Reader {
    fn event(&self, id: &str) -> Message {
        Message::Story(json!({ "type": "showMessage", "id": id, "params": { "text": "…" } }).as_object().unwrap().clone())
    }

    fn scrolled_back(&self, id: &str) -> Message {
        Message::Story(json!({ "type": "showMessage", "id": id, "left": true }).as_object().unwrap().clone())
    }
}

/// the outlet's full cell in `out`: the piece and its values
fn piece<'a>(out: &'a [Out], outlet: &str) -> Option<(&'a str, serde_json::Value)> {
    out.iter().filter_map(Out::envelope).find_map(|e| match &e.message {
        Message::Cell(c) if c.outlet == outlet => match c.cell.layers.as_deref()?.first().map(|l| &l.src) {
            Some(LayerSrc::Piece(p)) => Some((p.piece.as_str(), serde_json::to_value(p.values.clone().unwrap_or_default()).unwrap())),
            _ => None,
        },
        _ => None,
    })
}

fn cells(out: &[Out]) -> usize {
    out.iter().filter_map(Out::envelope).filter(|e| matches!(e.message, Message::Cell(_))).count()
}

fn logged(c: &Conductor) -> Vec<Logged> {
    c.log().map(|e| e.what.clone()).collect()
}

fn opened() -> Conductor {
    let mut c = conductor();
    assert!(!c.fire("opening", Now::at(0)).is_empty());
    c
}

#[test]
fn a_chapter_event_fires_its_row_on_auto() {
    let mut c = opened();
    let out = c.on_message(&Reader.event("ch3.dialog-start"), Now::at(1_000));
    // the row is a `values` cell; what goes out is the full cell: the piece with every value it now has
    assert_eq!(piece(&out, "left"), Some(("scene:nave", json!({ "light": 0.2, "dim": 0.5 }))));
    assert_eq!(c.current(), Some("dialog"));
    assert!(matches!(logged(&c).last(), Some(Logged::Fired { row, origin: Origin::Auto }) if row == "dialog"));
}

#[test]
fn on_suggest_it_is_held_after_its_hold_time_and_go_fires_it() {
    let mut c = opened();
    assert!(c.on_message(&Reader.event("ch3.yellowmouse-enters"), Now::at(1_000)).is_empty());
    assert!(c.suggestion().is_none(), "the event must stand for its hold time first");
    assert!(c.tick(Now::at(2_000)).is_empty());
    assert!(c.suggestion().is_none());
    assert!(c.tick(Now::at(2_500)).is_empty());
    assert_eq!(c.suggestion().map(|s| s.row.as_str()), Some("yellowmouse-enters"));
    assert_eq!(c.current(), Some("opening"), "a suggestion fires nothing");

    let out = c.go(Now::at(3_000));
    assert_eq!(piece(&out, "left"), Some(("character:yellowmouse", json!({ "mood": "shy" }))));
    assert!(c.suggestion().is_none());
    assert!(matches!(logged(&c).last(), Some(Logged::Fired { origin: Origin::Go, .. })));
}

#[test]
fn on_hand_the_event_is_ignored_and_the_hand_still_fires() {
    let mut c = opened();
    assert!(c.on_message(&Reader.event("ch3.end"), Now::at(1_000)).is_empty());
    assert_eq!(c.current(), Some("opening"));
    assert!(matches!(logged(&c).last(), Some(Logged::Ignored { row, .. }) if row == "finale"));
    assert_eq!(cells(&c.fire("finale", Now::at(2_000))), 2);
    assert_eq!(c.current(), Some("finale"));
}

#[test]
fn an_accidental_scroll_never_fires() {
    let mut c = opened();
    // scrolled past before the hold time, then a newer event takes its place
    c.on_message(&Reader.event("ch3.yellowmouse-enters"), Now::at(1_000));
    c.on_message(&Reader.scrolled_back("ch3.yellowmouse-enters"), Now::at(1_400));
    assert!(c.tick(Now::at(9_000)).is_empty());
    assert!(c.suggestion().is_none());

    // held, then the reader scrolls back: the suggestion goes
    c.on_message(&Reader.event("ch3.yellowmouse-enters"), Now::at(10_000));
    c.tick(Now::at(12_000));
    assert!(c.suggestion().is_some());
    c.on_message(&Reader.scrolled_back("ch3.yellowmouse-enters"), Now::at(12_500));
    assert!(c.suggestion().is_none());
    assert_eq!(c.current(), Some("opening"));
}

#[test]
fn an_unknown_event_is_logged() {
    let mut c = conductor();
    assert!(c.on_event("nobody.listens", Now::at(0)).is_empty());
    assert_eq!(logged(&c), vec![Logged::NoRow { event: "nobody.listens".into() }]);
}

#[test]
fn a_full_cell_goes_out_as_the_protocols_message() {
    let mut c = conductor();
    let out = c.fire("opening", Now::at(0));
    // the mirror gets the source's cell itself (a deck takes a mirror as black), and only stage outlets get a `cell`
    assert_eq!(cells(&out), 2);
    assert_eq!(piece(&out, "right"), piece(&out, "left"));
    for e in out.iter().filter_map(Out::envelope) {
        let back: Envelope = serde_json::from_str(&serde_json::to_string(e).unwrap()).unwrap();
        assert_eq!(&back, e);
    }
    assert_eq!(c.on_stage("left").map(|s| s.row.as_str()), Some("opening"));
    assert!(matches!(c.on_stage("right").map(|s| &s.playing), Some(Playing::Mirror { .. })));
}

#[test]
fn a_mirror_follows_its_source_when_only_the_source_changed() {
    let mut c = opened();
    // the dialog row only writes `left`
    let out = c.on_event("ch3.dialog-start", Now::at(1_000));
    assert_eq!(piece(&out, "right"), Some(("scene:nave", json!({ "light": 0.2, "dim": 0.5 }))));
}

#[test]
fn a_device_outlet_gets_a_device_message_the_bridge_reads() {
    let mut c = conductor();
    let out = c.fire("opening", Now::at(0));
    let device = out.iter().filter_map(Out::envelope).find_map(|e| match &e.message {
        Message::Device(d) => Some(d),
        _ => None,
    });
    let device = device.expect("a device message");
    assert_eq!(device.outlet, "lamps");
    assert!(matches!(&device.msgs[0], DeviceMsg::Dmx(d) if d.ch == 1 && d.values == [255, 128]));
}

#[test]
fn black_and_keep_along_the_path() {
    let mut c = opened();
    let out = c.fire("finale", Now::at(1_000));
    let black = out.iter().filter_map(Out::envelope).find_map(|e| match &e.message {
        Message::Cell(c) if c.outlet == "left" => Some(c.cell.mode),
        _ => None,
    });
    assert_eq!(black, Some(CellMode::Black));
    // `lamps` is kept: nothing goes out for it, and it counts as the row that kept it
    assert!(!out.iter().filter_map(Out::envelope).any(|e| matches!(e.message, Message::Device(_))));
    assert_eq!(c.on_stage("lamps").map(|s| s.row.as_str()), Some("finale"));
}

#[test]
fn go_walks_the_rows_and_stops_at_the_end() {
    let mut c = conductor();
    for row in ["opening", "dialog", "yellowmouse-enters", "finale"] {
        c.go(Now::at(0));
        assert_eq!(c.current(), Some(row));
    }
    assert!(c.go(Now::at(0)).is_empty());
    assert_eq!(logged(&c).last(), Some(&Logged::NoNextRow));
}

#[test]
fn a_single_cell_by_hand_is_marked_off_row() {
    let mut c = opened();
    let out = c.launch_cell("yellowmouse-enters", "left", Now::at(1_000));
    assert_eq!(piece(&out, "left").map(|p| p.0), Some("character:yellowmouse"));
    assert!(c.on_stage("left").unwrap().off_row);
    assert!(logged(&c).contains(&Logged::CellLaunched { row: "yellowmouse-enters".into(), column: "left".into(), off_row: true }));
    assert_eq!(c.current(), Some("opening"), "a cell is no row");
}

#[test]
fn a_values_cell_on_the_wrong_piece_is_skipped_and_logged() {
    let mut c = opened();
    c.launch_cell("yellowmouse-enters", "left", Now::at(1_000));
    let out = c.on_event("ch3.dialog-start", Now::at(2_000));
    assert_eq!(piece(&out, "left"), None);
    assert!(logged(&c).contains(&Logged::ValuesSkipped {
        row: "dialog".into(),
        outlet: "left".into(),
        expected: "scene:nave".into(),
        found: Some("character:yellowmouse".into()),
    }));
    assert!(matches!(&c.on_stage("left").unwrap().playing, Playing::Piece { piece, .. } if piece == "character:yellowmouse"));
}

#[test]
fn an_address_finds_what_plays_it_or_stays_silent() {
    let mut c = opened();
    assert!(c.address("character:yellowmouse", "dance:samba", Now::at(1)).is_empty());
    assert_eq!(logged(&c).last(), Some(&Logged::Addressed { address: "character:yellowmouse".into(), action: "dance:samba".into(), reached: 0 }));

    c.launch_cell("yellowmouse-enters", "left", Now::at(2));
    let out = c.address("character:yellowmouse", "dance:samba", Now::at(3));
    assert_eq!(out.len(), 1);
    assert!(matches!(out[0].envelope().map(|e| &e.message), Some(Message::Story(_))));
    assert_eq!(logged(&c).last(), Some(&Logged::Addressed { address: "character:yellowmouse".into(), action: "dance:samba".into(), reached: 1 }));
}

#[test]
fn a_moment_waits_for_its_grid() {
    let mut c = opened();
    // beat 5.5 of a 4/4 bar at 120 bpm: the next bar line is beat 8, 2.5 beats = 1250 ms away
    let now = Now { ms: 10_000, clock: Some(Clock { bpm: 120.0, beat: 5.5, per_bar: 4 }) };
    let out = c.fire("yellowmouse-enters", now);
    let at = out.iter().filter_map(Out::envelope).find(|e| matches!(e.message, Message::Cell(_))).and_then(|e| e.at);
    assert_eq!(at, Some(11_250));
}

#[test]
fn the_tree_is_one_mechanism_a_child_takes_its_parent_like_a_reader() {
    let mut parent = conductor();
    let out = parent.fire("opening", Now::at(0));
    let Some((says, word)) = out.iter().find_map(|o| match o {
        Out::Seat { seat, word, says, .. } if seat == "foyer" => Some((says.clone(), *word)),
        _ => None,
    }) else {
        panic!("no word for the seat");
    };
    assert_eq!((says.clone(), word), (Says::Go("idle".into()), Word::Follow));

    let mut foyer = child();
    let out = foyer.on_parent(word, &says, Now::at(0));
    assert_eq!(piece(&out, "wall"), Some(("scene:fireflies", json!({}))));

    // suggest: its operator decides, the local GO takes it
    let mut foyer = child();
    assert!(foyer.on_parent(Word::Suggest, &Says::Go("storm".into()), Now::at(0)).is_empty());
    assert_eq!(foyer.suggestion().map(|s| s.row.as_str()), Some("storm"));
    assert_eq!(piece(&foyer.go(Now::at(1)), "wall").map(|p| p.0), Some("scene:storm"));

    // free: ignored, the local GO still works
    let mut foyer = child();
    assert!(foyer.on_parent(Word::Free, &Says::Go("storm".into()), Now::at(0)).is_empty());
    assert_eq!(foyer.current(), None);
    assert_eq!(foyer.go(Now::at(1)).len(), 1);
}
