//! The resolver (#474): the order, the origin, the fallback, the timed rule, stable ids, and its two users.
use super::handler::{levels, Handlers};
use super::mapping::{MappingLayers, LOCAL, PRECODED};
use super::*;
use crate::rig::address::Target;
use crate::rig::preset::Mapping;

fn src(strength: Strength, id: &str) -> Source {
    Source::new(strength, id)
}

/// every strength has an opinion on one key; taking them back one by one walks down the order
#[test]
fn the_order_is_the_declaration_order() {
    let mut sorted = Strength::ALL;
    sorted.sort();
    assert_eq!(sorted, Strength::ALL, "ALL lists the strengths in their declared order");
    let mut r = Resolver::new();
    // given weakest first and strongest first: the order does not depend on when an opinion came
    for (i, s) in Strength::ALL.iter().rev().enumerate().filter(|(_, s)| **s != Strength::Timed) {
        r.set("mood", src(*s, s.word()), i);
    }
    r.call("mood", "conductor", 99, 0.0, 10.0);
    let walked: Vec<Strength> = r.chain("mood", 1.0).iter().map(|f| f.origin.strength).collect();
    assert_eq!(walked, Strength::ALL);
    for s in Strength::ALL {
        assert_eq!(r.resolve("mood", 1.0).unwrap().origin.strength, s);
        if s == Strength::Timed {
            r.clear("mood", &src(s, "conductor"));
        } else {
            r.clear("mood", &src(s, s.word()));
        }
    }
    assert!(r.resolve("mood", 1.0).is_none(), "nobody has an opinion: no value");
}

#[test]
fn a_value_says_where_it_came_from_and_what_it_falls_back_to() {
    let mut r = Resolver::new();
    r.set("glow", src(Strength::Character, "character:catpope"), 0.4);
    r.set("glow", src(Strength::Persona, "character:catpope.mountaintiger"), 0.8);
    r.set("glow", src(Strength::Shipped, "build"), 0.0);
    let got = r.resolve("glow", 0.0).unwrap();
    assert_eq!(*got.value, 0.8);
    assert_eq!(got.origin.to_string(), "persona character:catpope.mountaintiger");
    let back = got.fallback.unwrap();
    assert_eq!((*back.value, back.origin.strength), (0.4, Strength::Character));
    // only the floor: no fallback
    let mut floor = Resolver::new();
    floor.set("glow", src(Strength::Shipped, "build"), 0.0);
    assert!(floor.resolve("glow", 0.0).unwrap().fallback.is_none());
}

#[test]
fn per_key_and_source_one_opinion_the_newest_replaces() {
    let mut r = Resolver::new();
    let flash = src(Strength::Flashed, "show:a");
    r.set("k", flash.clone(), 1);
    r.set("k", flash.clone(), 2);
    assert_eq!(r.chain("k", 0.0).len(), 1);
    assert_eq!(*r.resolve("k", 0.0).unwrap().value, 2);
    // two sources of one strength: the newest given wins, the other is its fallback
    r.set("k", src(Strength::Flashed, "show:b"), 3);
    let got = r.resolve("k", 0.0).unwrap();
    assert_eq!((*got.value, got.origin.id.as_str()), (3, "show:b"));
    assert_eq!(got.fallback.unwrap().origin.id, "show:a");
}

/// overrides are keyed by stable ids: taking one away never moves another to its place
#[test]
fn stable_ids_never_positions() {
    let mut r = Resolver::new();
    for (i, id) in ["a", "b", "c"].iter().enumerate() {
        r.set(&format!("track.{id}.gain"), src(Strength::Draft, "member:7"), i);
    }
    r.clear("track.b.gain", &src(Strength::Draft, "member:7"));
    assert_eq!(*r.resolve("track.c.gain", 0.0).unwrap().value, 2, "c keeps its own opinion");
    assert!(r.resolve("track.b.gain", 0.0).is_none());
    assert_eq!(r.keys().collect::<Vec<_>>(), ["track.a.gain", "track.c.gain"]);
}

#[test]
fn a_timed_change_holds_for_its_length_then_the_next_source_shows() {
    let mut r = Resolver::new();
    r.set("mood", src(Strength::Character, "character:catpope"), "neutral");
    r.call("mood", "scene:party", "laughing", 4.0, 4.0);
    assert_eq!(*r.resolve("mood", 3.9).unwrap().value, "neutral", "not yet started");
    let at = r.resolve("mood", 4.0).unwrap();
    assert_eq!((*at.value, at.origin.strength), ("laughing", Strength::Timed));
    assert_eq!(*at.fallback.unwrap().value, "neutral");
    assert_eq!(*r.resolve("mood", 7.99).unwrap().value, "laughing");
    assert_eq!(*r.resolve("mood", 8.0).unwrap().value, "neutral", "ended: the base shows again");
}

/// a second call by the same caller replaces the first; its length runs from its own start
#[test]
fn a_second_call_replaces_and_runs_from_its_own_start() {
    let mut r = Resolver::new();
    r.set("mood", src(Strength::Character, "character:catpope"), "neutral");
    r.call("mood", "conductor", "laughing", 0.0, 4.0);
    r.call("mood", "conductor", "crying", 2.0, 4.0);
    assert_eq!(r.chain("mood", 2.0).len(), 2, "one timed opinion per caller, not two");
    assert_eq!(*r.resolve("mood", 3.0).unwrap().value, "crying");
    assert_eq!(*r.resolve("mood", 5.0).unwrap().value, "crying", "past the first call's end: the second's length counts");
    assert_eq!(*r.resolve("mood", 6.0).unwrap().value, "neutral");
    // the first one is gone, not waiting underneath: before the second started the base shows
    assert_eq!(*r.resolve("mood", 1.0).unwrap().value, "neutral");
}

/// two callers: the newest call shows; when it ends first, the other one still running shows again
#[test]
fn calls_of_two_callers_stack_by_newest() {
    let mut r = Resolver::new();
    r.set("light", src(Strength::Flashed, "show:a"), 0.5);
    r.call("light", "scene:storm", 0.1, 0.0, 16.0);
    r.call("light", "scene:flash", 1.0, 4.0, 1.0);
    assert_eq!(*r.resolve("light", 4.5).unwrap().value, 1.0);
    assert_eq!(*r.resolve("light", 5.0).unwrap().value, 0.1);
    assert_eq!(*r.resolve("light", 16.0).unwrap().value, 0.5);
    // the same rule for any key of any kind: an item's value, a mapping, a handler's key
    r.call("item:lamp/being:glow", "scene:storm", 0.9, 0.0, 2.0);
    assert_eq!(r.resolve("item:lamp/being:glow", 1.0).unwrap().origin.strength, Strength::Timed);
    assert!(r.resolve("item:lamp/being:glow", 2.0).is_none());
}

#[test]
fn a_local_change_beats_a_timed_one_and_a_timed_one_beats_the_flash() {
    let mut r = Resolver::new();
    r.set("k", src(Strength::Flashed, "show:a"), 1);
    r.call("k", "conductor", 2, 0.0, 8.0);
    assert_eq!(*r.resolve("k", 1.0).unwrap().value, 2);
    r.set("k", src(Strength::Local, "local"), 3);
    assert_eq!(*r.resolve("k", 1.0).unwrap().value, 3);
}

#[test]
fn prune_drops_what_ended() {
    let mut r = Resolver::new();
    r.call("a", "x", 1, 0.0, 1.0);
    r.call("b", "x", 1, 0.0, 5.0);
    r.prune(2.0);
    assert_eq!(r.keys().collect::<Vec<_>>(), ["b"]);
}

#[test]
fn strengths_have_one_word_each() {
    for s in Strength::ALL {
        assert_eq!(Strength::parse(s.word()), Some(s));
        assert_eq!(serde_json::to_value(s).unwrap(), serde_json::json!(s.word()));
    }
    assert_eq!(Strength::parse("inherit"), None);
}

fn t(s: &str) -> Target {
    Target::parse(s).unwrap()
}

/// SHOW.md §8: look → persona → character → the kind's default
#[test]
fn who_answers_goes_up_the_ladder() {
    let mut h = Handlers::new();
    h.hold(&t("character:*"), "dance:samba", "default samba");
    h.hold(&t("character:catpope"), "dance:samba", "catpope samba");
    h.hold(&t("character:catpope.mountaintiger"), "dance:samba", "tiger samba");
    h.hold(&t("character:catpope.mountaintiger.ghost"), "wave", "ghost wave");

    let look = h.answer(&t("character:catpope.mountaintiger.ghost"), "dance:samba").unwrap();
    assert_eq!(*look.value, "tiger samba");
    assert_eq!(look.origin.to_string(), "persona character:catpope.mountaintiger");
    assert_eq!(look.fallback.unwrap().origin.id, "character:catpope");
    // another persona of the same character: the character's
    let other = h.answer(&t("character:catpope.pope"), "dance:samba").unwrap();
    assert_eq!((*other.value, other.origin.strength), ("catpope samba", Strength::Character));
    // another character: the kind's default, and nothing below it
    let mouse = h.answer(&t("character:yellowmouse"), "dance:samba").unwrap();
    assert_eq!((*mouse.value, mouse.origin.strength), ("default samba", Strength::KindDefault));
    assert!(mouse.fallback.is_none());
    // the look's own handler answers for it only
    assert_eq!(*h.answer(&t("character:catpope.mountaintiger.ghost"), "wave").unwrap().value, "ghost wave");
    assert!(h.answer(&t("character:catpope.mountaintiger"), "wave").is_none(), "nobody knows: no answer, logged by the caller");
    // every one of a kind is who picks up, never who answers
    assert!(h.answer(&t("character:*"), "dance:samba").is_none());
}

#[test]
fn any_kind_answers_the_same_way() {
    let mut h = Handlers::new();
    h.hold(&t("location:*"), "close_doors", 1);
    h.hold(&t("location:hall"), "close_doors", 2);
    assert_eq!(*h.answer(&t("location:hall"), "close_doors").unwrap().value, 2);
    assert_eq!(h.answer(&t("location:yard"), "close_doors").unwrap().origin.id, "location:*");
    // a character's default does not answer for a location
    h.hold(&t("character:*"), "wave", 3);
    assert!(h.answer(&t("location:hall"), "wave").is_none());
    assert_eq!(h.release(&t("location:hall"), "close_doors"), Some(2));
    assert_eq!(*h.answer(&t("location:hall"), "close_doors").unwrap().value, 1);
}

#[test]
fn the_levels_of_a_look() {
    let ids: Vec<String> = levels(&t("character:catpope.mountaintiger.ghost")).unwrap().into_iter().map(|s| s.to_string()).collect();
    assert_eq!(ids, [
        "default character:*",
        "character character:catpope",
        "persona character:catpope.mountaintiger",
        "look character:catpope.mountaintiger.ghost",
    ]);
}

fn m(source: &str, target: &str) -> Mapping {
    Mapping { source: source.into(), target: target.into(), range: None }
}

/// research/INPUT.md §6b: precoded < flashed < local
#[test]
fn mapping_layers() {
    let mut deck = MappingLayers::new(&[m("pad:left.x", "being:x"), m("pad:left.y", "being:y")]);
    let pad = deck.of("pad:left.x").unwrap();
    assert_eq!((pad.value[0].target.as_str(), pad.origin.id.as_str()), ("being:x", PRECODED));

    // a flash wins over the precoded layer, control by control
    let kept = deck.flash("show:night", &[m("pad:left.x", "voice.cutoff"), m("midi:1:cc:74", "voice.cutoff")], false);
    assert!(kept.is_empty());
    let flashed = deck.of("pad:left.x").unwrap();
    assert_eq!((flashed.value[0].target.as_str(), flashed.origin.strength), ("voice.cutoff", Strength::Flashed));
    assert_eq!(flashed.fallback.unwrap().origin.id, PRECODED);
    assert_eq!(deck.of("pad:left.y").unwrap().origin.strength, Strength::Shipped, "what the flash leaves alone stays precoded");

    // local: a learned knob replaces what the flash bound to it; a control switched off drives nothing
    deck.set_local("midi:1:cc:74", vec![Mapping { range: Some([0.2, 0.6]), ..m("midi:1:cc:74", "voice.res") }]);
    deck.switch_off("pad:left.y");
    let knob = deck.of("midi:1:cc:74").unwrap();
    assert_eq!((knob.value[0].target.as_str(), knob.origin.id.as_str()), ("voice.res", LOCAL));
    assert!(deck.of("pad:left.y").unwrap().value.is_empty());
    assert_eq!(deck.local(), ["midi:1:cc:74", "pad:left.y"]);
    assert_eq!(deck.playing().len(), 2, "pad:left.x and the learned knob");

    // a flash replaces layer 2 and keeps the local changes, saying which
    let kept = deck.flash("show:day", &[m("midi:1:cc:74", "voice.gain")], false);
    assert_eq!(kept, ["midi:1:cc:74", "pad:left.y"]);
    assert_eq!(deck.of("pad:left.x").unwrap().origin.strength, Strength::Shipped, "the old flash is gone");
    assert_eq!(deck.of("midi:1:cc:74").unwrap().origin.strength, Strength::Local);

    // reset this mapping: the flash shows again
    assert!(deck.reset("midi:1:cc:74"));
    assert!(!deck.reset("midi:1:cc:74"));
    let back = deck.of("midi:1:cc:74").unwrap();
    assert_eq!((back.value[0].target.as_str(), back.origin.id.as_str()), ("voice.gain", "show:day"));

    // force defaults: the flash clears the local layer too
    let cleared = deck.flash("show:day", &[], true);
    assert_eq!(cleared, ["pad:left.y"]);
    assert!(deck.local().is_empty());
    assert_eq!(deck.of("pad:left.y").unwrap().value[0].target, "being:y");
    assert!(deck.of("midi:1:cc:74").is_none(), "nothing precoded for that knob");
}
