use super::DEFAULT_RIG;
use crate::rig::bundle::Bundle;
use crate::rig::Track;
use crate::scene::Scene;

fn default_rig() -> Bundle {
    Bundle::core(DEFAULT_RIG).expect("the default character rig is a core rig").expect("it reads")
}

fn flat(b: &Bundle) -> Scene {
    b.rig.flatten(&b.library(), None, None).unwrap_or_else(|e| panic!("{e:#}"))
}

fn has_link(s: &Scene, from: &str, to: &str) -> bool {
    s.links.iter().any(|l| l.from == from && l.to == to)
}

fn kind_of<'a>(s: &'a Scene, id: &str) -> Option<&'a str> {
    s.nodes.iter().find(|n| n.id == id).map(|n| n.kind.as_str())
}

/// a scutoon's rig extending the default character rig: catpope's instruments, one own track that takes the voice over
fn popstar() -> Bundle {
    let cat = Bundle::core("character.catpope").unwrap().unwrap();
    let mut b = Bundle::fallback("character.giantpopstar");
    b.instruments = cat.instruments;
    b.presets = cat.presets;
    b.rig.tracks.push(Track { id: "sing".into(), instrument: "cathedral_voice".into(), preset: Some("cathedral_voice.base".into()), gain: 1.0, pan: 0.0, mute: false, takes: Some("voice".into()) });
    b
}

#[test]
fn the_default_character_rig_sings_and_its_voice_opens_the_mouth() {
    let b = default_rig();
    assert!(b.rig.extends.is_none() && b.rig.tracks.len() == 1, "one track, extending nothing");
    let s = flat(&b);
    assert_eq!(kind_of(&s, "voice_voice"), Some("voice"));
    assert_eq!(kind_of(&s, "voice_melody"), Some("walk"), "a melody");
    assert!(has_link(&s, "voice_melody.cv", "voice_voice.cv") && has_link(&s, "voice_voice.out", "mix1.in1"));
    // #819: it sings only when triggered – the trigger (a node the stage sets, 0 until then) opens the walk's gates
    assert_eq!(kind_of(&s, "trigger_sing"), Some("add"));
    assert!(has_link(&s, "trigger_sing.out", "voice_melody.open"), "{:?}", s.links);
    assert!(has_link(&s, "voice_ear.level", "space.mouth"), "the voice's level opens the mouth");
    let instruments: Vec<_> = b.instruments.iter().map(|i| (i.id.as_str(), i.kind)).collect();
    assert_eq!(instruments, [("default_voice", crate::rig::instrument::InstrumentKind::Voice)], "no other instrument");
    let sad = b.play(None, Some("sad")).unwrap();
    assert_eq!(sad.knobs.iter().find(|k| k.path == "voice_voice.vowel").map(|k| k.value), Some(4.0), "a mood moves the vowel");
}

#[test]
fn the_voice_turned_off_is_not_compiled_and_the_rig_still_plays() {
    let mut b = Bundle::fallback("character.giantpopstar");
    b.rig.off = vec!["voice".into()];
    let s = flat(&b);
    assert!(!s.nodes.iter().any(|n| n.id.starts_with("voice_") || n.kind == "voice" || n.kind == "walk"), "nothing of the voice: {:?}", s.nodes);
    assert!(!s.links.iter().any(|l| l.to == "space.mouth"), "its route is off with it");
    assert!(!s.nodes.iter().any(|n| n.id == "trigger_sing"), "and the trigger with it: nothing to open");
    let p = b.play(None, Some("sad")).unwrap();
    assert!(p.tracks.is_empty() && p.engine.page.is_empty() && p.character == "giantpopstar", "silent, the being stands");
    // muting every own track is still refused
    let mut muted = popstar();
    muted.rig.tracks[0].mute = true;
    assert!(muted.rig.flatten(&muted.library(), None, None).unwrap_err().to_string().contains("muted"));
}

#[test]
fn a_track_of_its_own_takes_the_voice_over() {
    let b = popstar();
    let s = flat(&b);
    assert!(!s.nodes.iter().any(|n| n.id.starts_with("voice_")), "the extended voice is not compiled");
    assert_eq!(kind_of(&s, "sing_voice"), Some("fm"));
    assert!(has_link(&s, "sing_voice.out", "mix1.in1") && has_link(&s, "mix1.out", "out.in"));
    assert!(!s.links.iter().any(|l| l.to == "space.mouth"), "the extended mouth route drops with its track");
    // #819: the trigger opens the taker's gates – its hook plays only when triggered too
    assert!(has_link(&s, "trigger_sing.out", "sing_melody.open"), "{:?}", s.links);
    assert_eq!(s.links.iter().filter(|l| l.from == "trigger_sing.out").count(), 1);
    let p = b.play(None, Some("sad")).unwrap();
    assert_eq!(p.tracks.iter().map(|t| t.track.as_str()).collect::<Vec<_>>(), ["sing"]);
    assert!(p.knobs.iter().all(|k| !k.path.starts_with("voice_")), "and no knob of it");
}

#[test]
fn the_extends_link_is_checked() {
    let refuse = |b: &Bundle, says: &str| {
        let e = format!("{:#}", b.rig.check(&b.library()).unwrap_err());
        assert!(e.contains(says), "{says}: {e}");
    };
    let mut b = Bundle::fallback("character.x");
    b.rig.extends = Some("character.catpope".into());
    b.rig.off = vec!["voice".into()];
    assert!(b.rig.check(&b.library()).is_ok(), "any core rig may be extended");
    b.rig.extends = Some("character.nobody".into());
    refuse(&b, "not a core rig");
    b.rig.extends = Some(DEFAULT_RIG.into());
    b.rig.off = vec!["drone".into()];
    refuse(&b, "has no track 'drone'");
    b.rig.extends = None;
    refuse(&b, "extends none");
    let mut clash = popstar();
    clash.rig.tracks[0].id = "voice".into();
    clash.rig.tracks[0].takes = None;
    refuse(&clash, "is the extended rig's too");
    let mut self_extended = default_rig();
    self_extended.rig.extends = Some(DEFAULT_RIG.into());
    refuse(&self_extended, "stands on its own");
}

#[test]
fn a_character_without_a_rig_of_its_own_falls_back_to_the_default_character_rig() {
    let cat = Bundle::core_or_default("character.catpope").unwrap().unwrap();
    assert!(cat.rig.extends.is_none() && cat.rig.tracks.len() == 2, "a core rig of its own wins");
    let b = Bundle::core_or_default("character.nobody").unwrap().unwrap();
    assert_eq!((b.rig.subject.as_str(), b.rig.extends.as_deref()), ("character.nobody", Some(DEFAULT_RIG)));
    let p = b.play(None, None).unwrap();
    assert_eq!(p.character, "nobody", "the default character rig wears the character");
    assert_eq!(p.tracks.iter().map(|t| t.track.as_str()).collect::<Vec<_>>(), ["voice"]);
    assert!(Bundle::core_or_default("item.nothing").is_none(), "an item has no default rig");
    let json = serde_json::to_string(&b).unwrap();
    assert!(json.contains(r#""extends":"character.default""#) && !json.contains("default_voice"), "the link is stored, not a copy: {json}");
    assert_eq!(Bundle::parse_json(&json).unwrap(), b);
}

/// #819: a taker that wires the trigger itself is not wired twice; a route to a track's gates is a target only and names a
/// track of the rig
#[test]
fn the_trigger_is_carried_once_and_checked() {
    let mut b = popstar();
    b.rig.routes.push(crate::rig::Route { from: "trigger:sing".into(), to: "sing:@open".into(), scale: 1.0, offset: 0.0 });
    let s = flat(&b);
    assert_eq!(s.links.iter().filter(|l| l.from == "trigger_sing.out").count(), 1, "{:?}", s.links);
    let refuse = |from: &str, to: &str, says: &str| {
        let mut b = default_rig();
        b.rig.routes.push(crate::rig::Route { from: from.into(), to: to.into(), scale: 1.0, offset: 0.0 });
        let e = format!("{:#}", b.rig.check(&b.library()).unwrap_err());
        assert!(e.contains(says), "{says}: {e}");
    };
    refuse("voice:@open", "being:pulse", "target only");
    refuse("trigger:sing", "drone:@open", "drone");
    refuse("voice:voice.gain", "trigger:sing", "a source only");
    refuse("trigger:shout", "voice:@open", "trigger has no 'shout'");
}
