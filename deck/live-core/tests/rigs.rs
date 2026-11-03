//! deck-space's lowering and live-core agree (#191, #236, P6 #320): every core rig, played for every mood, lowers to a
//! scene live-core loads – every node, its walks and events hosts included – and sounds.
use deck_space::rig::bundle::{Bundle, CORE};
use live_core::Core;

fn loud(c: &mut Core, blocks: usize) -> bool {
    (0..blocks).fold(false, |l, _| l | c.render(128).iter().any(|x| x.abs() > 1e-3))
}

#[test]
fn every_core_rig_loads_and_plays_in_live_core() {
    for (subject, _) in CORE {
        let b = Bundle::core(subject).unwrap().unwrap();
        for mood in [None, Some("happy"), Some("sad")] {
            let p = b.play(None, mood).unwrap();
            assert!(p.engine.page.is_empty(), "{subject}: the browser carries every kind of a core rig");
            let mut c = Core::load(&p.engine.scene, 48_000.0, 2).unwrap_or_else(|e| panic!("{subject} {mood:?}: {e}\n{}", p.engine.scene));
            let table: serde_json::Value = serde_json::from_str(&c.table_json()).unwrap();
            let rows = table["nodes"].as_array().unwrap();
            // every knob is an input of a row
            for k in &p.knobs {
                let (node, port) = k.path.rsplit_once('.').unwrap();
                let row = rows.iter().find(|r| r["id"] == node).unwrap_or_else(|| panic!("{subject}: knob {} has no row", k.path));
                assert!(row["ports"].as_array().unwrap().iter().any(|p| p == port), "{subject}: {}", k.path);
            }
            assert!(c.set_tempo(84.0), "a rig has no clock of its own: the page's");
            // the incense burner's noise is silent until its gain is raised
            assert_eq!(loud(&mut c, 200), subject.starts_with("character."), "{subject} {mood:?}");
        }
    }
}

/// #236: a rigged arpeggiator – an instrument whose `events` node is a patch (an arp on a walk) – plays in live-core, its
/// knob `a` an input of its row
#[test]
fn a_rigged_arp_plays_in_live_core() {
    let bundle: Bundle = toml::from_str(
        r#"
[rig]
subject = "character.test"
track = [{ id = "lead", instrument = "arp_lead" }]

[[instrument]]
id = "arp_lead"
kind = "lead"
out = "voice"
node = [
  { id = "voice", kind = "fm", freq = 220.0, ratio = 2.0, index = 1.0, gain = 0.2 },
  { id = "arp", kind = "events", a = 4.0, out = "arp.notes", knobs = ["arp.rate"], generator = [
    { id = "walk", kind = "random_walk", seed = 9, rate = 0.5, spread = 0.5 },
    { id = "arp", kind = "arp", ratios = [1.0, 1.25, 1.5], octaves = 2, mode = "up", rate = 4.0 },
  ], edge = [{ from = "walk.notes", to = "arp.notes" }] },
]
link = [{ from = "arp.cv", to = "voice.cv" }]
expose = [{ path = "arp.a", label = { en = "arp rate" }, range = [1.0, 16.0] }]
"#,
    )
    .unwrap();
    let p = bundle.play(None, None).unwrap();
    let mut c = Core::load(&p.engine.scene, 48_000.0, 2).unwrap_or_else(|e| panic!("{e}\n{}", p.engine.scene));
    let table: serde_json::Value = serde_json::from_str(&c.table_json()).unwrap();
    let row = table["nodes"].as_array().unwrap().iter().find(|n| n["id"] == "lead_arp").expect("the arp's row");
    assert_eq!((row["kind"].as_str(), row["ports"][0].as_str()), (Some("events"), Some("a")));
    assert_eq!(p.knobs[0].path, "lead_arp.a");
    assert!(loud(&mut c, 400));
}
