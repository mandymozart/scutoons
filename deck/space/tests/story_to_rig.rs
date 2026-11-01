//! "Test an event from a chapter" (#236, research/EVENTS.md §12): the real chapter fixture's slice event goes through
//! the story adapter, the resulting `story.showMessage` is routed to catpope's core rig by a small receiver, and the
//! cathedral voice switches to its bright preset for `happy`.
use deck_events::adapters::story::from_story;
use deck_events::{Event, Names};
use deck_space::rig::bundle::{Bundle, Played};
use serde_json::{json, Value};

/// A rig's receiver: a `story.*` event that names a mood (`data.mood`, and the `persona`) re-plays the rig in it.
struct RigReceiver {
    bundle: Bundle,
}

impl RigReceiver {
    fn on_event(&self, names: &Names, e: &Event) -> Option<Played> {
        if !names.in_domain(e.name, "story") {
            return None;
        }
        let data = e.data.as_ref()?;
        let mood = data.get("mood")?.as_str()?;
        let persona = data.get("persona").and_then(Value::as_str);
        Some(self.bundle.play(persona, Some(mood)).expect("the rig plays"))
    }
}

fn chapter_slice_event() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/content/test/fixtures/neutral/chapter.json")).unwrap();
    fn find(v: &Value) -> Option<Value> {
        match v {
            Value::Object(o) if o.get("type") == Some(&json!("showMessage")) && o.contains_key("trigger") => Some(v.clone()),
            Value::Object(o) => o.values().find_map(find),
            Value::Array(a) => a.iter().find_map(find),
            _ => None,
        }
    }
    find(&serde_json::from_str(&text).unwrap()).expect("the chapter has a slice showMessage")
}

fn ratio(p: &Played) -> f32 {
    p.knobs.iter().find(|k| k.path == "voice_voice.ratio").unwrap().value
}

#[test]
fn a_chapters_happy_message_makes_the_cathedral_voice_bright() {
    let mut names = Names::new();
    let rig = RigReceiver { bundle: Bundle::core("character.catpope").unwrap().unwrap() };

    // before: the rig's defaults – the base preset, ratio 1.5
    assert_eq!(ratio(&rig.bundle.play(None, None).unwrap()), 1.5);

    let story = chapter_slice_event();
    assert_eq!(story["params"]["mood"], "happy", "the fixture's event carries the mood");
    let event = from_story(&mut names, &story, Some("story:neutral/1")).unwrap().expect("a showMessage is an event");
    assert_eq!(names.name(event.name), "story.showMessage");

    let played = rig.on_event(&names, &event).expect("the rig takes a story event with a mood");
    assert_eq!(ratio(&played), 3.0, "happy → cathedral_voice.bright");
    let voice = played.tracks.iter().find(|t| t.track == "voice").unwrap();
    assert_eq!(voice.preset.as_deref(), Some("cathedral_voice.bright"));

    // another mood switches the other way; an event without a mood leaves the rig alone
    let sad = from_story(&mut names, &json!({"type": "showMessage", "params": {"text": "x", "mood": "sad"}}), None).unwrap().unwrap();
    assert_eq!(ratio(&rig.on_event(&names, &sad).unwrap()), 0.5, "sad → cathedral_voice.low");
    let plain = from_story(&mut names, &json!({"type": "hideMessage"}), None).unwrap().unwrap();
    assert!(rig.on_event(&names, &plain).is_none());
}
