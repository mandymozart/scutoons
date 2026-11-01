//! deck-space's lowering and live-core agree (#191, #236): every core rig, played for every mood, lowers to a scene
//! live-core loads – its batch nodes, its walks (their knobs as table rows) – and sounds.
use deck_space::rig::bundle::{Bundle, CORE};
use live_core::Core;

#[test]
fn every_core_rig_loads_and_plays_in_live_core() {
    for (subject, _) in CORE {
        let b = Bundle::core(subject).unwrap().unwrap();
        for mood in [None, Some("happy"), Some("sad")] {
            let p = b.play(None, mood).unwrap();
            let mut c = Core::load(&p.engine.scene, 48_000.0).unwrap_or_else(|e| panic!("{subject} {mood:?}: {e}\n{}", p.engine.scene));
            let table: serde_json::Value = serde_json::from_str(&c.table_json()).unwrap();
            let ids: Vec<&str> = table["nodes"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
            // every knob the engine plays is a table row (the page's own nodes aside)
            for k in &p.knobs {
                let node = k.path.rsplit_once('.').unwrap().0;
                if !p.engine.page.iter().any(|n| n.node.id == node) {
                    assert!(ids.contains(&node), "{subject}: knob {} has no row in {ids:?}", k.path);
                }
            }
            let mut loud = false;
            for _ in 0..200 {
                c.step(128.0 / 48_000.0);
                loud |= c.render(128).iter().any(|x| x.abs() > 1e-3);
            }
            // the incense burner's noise is the page's: its engine scene is silent
            assert_eq!(loud, subject.starts_with("character."), "{subject} {mood:?}");
        }
    }
}

/// #236: a rigged arpeggiator – an instrument whose `events` node is a patch (an arp on a walk) – lowers to an
/// `[[events]]` live-core plays, its knob `a` a table row
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
  { id = "arp", kind = "events", bpm = 110.0, a = 4.0, out = "arp.notes", knobs = ["arp.rate"], generator = [
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
    assert!(p.engine.scene.contains("[[events]]") && p.engine.scene.contains("target = \"lead_voice.freq\""), "{}", p.engine.scene);
    let mut c = Core::load(&p.engine.scene, 48_000.0).unwrap_or_else(|e| panic!("{e}\n{}", p.engine.scene));
    let table: serde_json::Value = serde_json::from_str(&c.table_json()).unwrap();
    let row = table["nodes"].as_array().unwrap().iter().find(|n| n["id"] == "lead_arp").expect("the arp's row");
    assert_eq!(row["ports"][0], "a");
    assert_eq!(p.knobs[0].path, "lead_arp.a");
    let mut loud = false;
    for _ in 0..400 {
        c.step(128.0 / 48_000.0);
        loud |= c.render(128).iter().any(|x| x.abs() > 1e-3);
    }
    assert!(loud);
}
