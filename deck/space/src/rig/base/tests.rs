use super::BASE;
use crate::rig::bundle::Bundle;
use crate::rig::Track;
use crate::scene::Scene;

fn base() -> Bundle {
    Bundle::core(BASE).expect("the base rig is a core rig").expect("it reads")
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

/// a scutoon's rig on the base: catpope's instruments, one own track that takes the voice over
fn popstar() -> Bundle {
    let cat = Bundle::core("character.catpope").unwrap().unwrap();
    let mut b = Bundle::fallback("character.giantpopstar");
    b.instruments = cat.instruments;
    b.presets = cat.presets;
    b.rig.tracks.push(Track { id: "sing".into(), instrument: "cathedral_voice".into(), preset: Some("cathedral_voice.base".into()), gain: 1.0, pan: 0.0, mute: false, takes: Some("voice".into()) });
    b
}

#[test]
fn the_base_rig_sings_and_its_voice_opens_the_mouth() {
    let b = base();
    assert!(b.rig.base.is_none() && b.rig.tracks.len() == 1, "one track, built on nothing");
    let s = flat(&b);
    assert_eq!(kind_of(&s, "voice_voice"), Some("voice"));
    assert_eq!(kind_of(&s, "voice_melody"), Some("walk"), "a melody, so it sings by default");
    assert!(has_link(&s, "voice_melody.cv", "voice_voice.cv") && has_link(&s, "voice_voice.out", "mix1.in1"));
    assert!(has_link(&s, "voice_ear.level", "space.mouth"), "the voice's level opens the mouth");
    let instruments: Vec<_> = b.instruments.iter().map(|i| (i.id.as_str(), i.kind)).collect();
    assert_eq!(instruments, [("base_voice", crate::rig::instrument::InstrumentKind::Voice)], "no other instrument");
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
    assert!(!s.nodes.iter().any(|n| n.id.starts_with("voice_")), "the base voice is not compiled");
    assert_eq!(kind_of(&s, "sing_voice"), Some("fm"));
    assert!(has_link(&s, "sing_voice.out", "mix1.in1") && has_link(&s, "mix1.out", "out.in"));
    assert!(!s.links.iter().any(|l| l.to == "space.mouth"), "the base's mouth route drops with its track");
    let p = b.play(None, Some("sad")).unwrap();
    assert_eq!(p.tracks.iter().map(|t| t.track.as_str()).collect::<Vec<_>>(), ["sing"]);
    assert!(p.knobs.iter().all(|k| !k.path.starts_with("voice_")), "and no knob of it");
}

#[test]
fn the_base_link_is_checked() {
    let refuse = |b: &Bundle, says: &str| {
        let e = format!("{:#}", b.rig.check(&b.library()).unwrap_err());
        assert!(e.contains(says), "{says}: {e}");
    };
    let mut b = Bundle::fallback("character.x");
    b.rig.base = Some("character.catpope".into());
    b.rig.off = vec!["voice".into()];
    assert!(b.rig.check(&b.library()).is_ok(), "any core rig may be a base");
    b.rig.base = Some("character.nobody".into());
    refuse(&b, "is not a core rig");
    b.rig.base = Some(BASE.into());
    b.rig.off = vec!["drone".into()];
    refuse(&b, "has no track 'drone'");
    b.rig.base = None;
    refuse(&b, "names no base");
    let mut clash = popstar();
    clash.rig.tracks[0].id = "voice".into();
    clash.rig.tracks[0].takes = None;
    refuse(&clash, "is the base rig's too");
    let mut self_based = base();
    self_based.rig.base = Some(BASE.into());
    refuse(&self_based, "stands on its own");
}

#[test]
fn a_character_without_a_rig_of_its_own_falls_back_to_the_base_rig() {
    let cat = Bundle::core_or_base("character.catpope").unwrap().unwrap();
    assert!(cat.rig.base.is_none() && cat.rig.tracks.len() == 2, "a core rig of its own wins");
    let b = Bundle::core_or_base("character.nobody").unwrap().unwrap();
    assert_eq!((b.rig.subject.as_str(), b.rig.base.as_deref()), ("character.nobody", Some(BASE)));
    let p = b.play(None, None).unwrap();
    assert_eq!(p.character, "nobody", "the base rig wears the character");
    assert_eq!(p.tracks.iter().map(|t| t.track.as_str()).collect::<Vec<_>>(), ["voice"]);
    assert!(Bundle::core_or_base("item.nothing").is_none(), "an item has no base rig");
    let json = serde_json::to_string(&b).unwrap();
    assert!(json.contains(r#""base":"character.base""#) && !json.contains("base_voice"), "the link is stored, not a copy: {json}");
    assert_eq!(Bundle::parse_json(&json).unwrap(), b);
}
