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
