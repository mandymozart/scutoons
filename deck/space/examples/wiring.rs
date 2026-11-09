//! The wiring model, shown (#433 W5, #434 W6): catpope's core rig at the rig, track and node level – each cable with its
//! home and whether a gesture may change it, the edge as stubs – then one edit (the voice sent into the drone's mixer) and
//! the rig's text view. `cargo run -p deck-space --example wiring [<track> | <track>:<node>]`
use deck_space::rig::edit::{Doc, Op};
use deck_space::rig::wiring::View;
use deck_space::rig::Bundle;

fn show(v: &View) {
    println!("\n== level {:?}  (chain: {})", v.context, v.levels.join(" › "));
    for (i, n) in v.nodes.iter().enumerate() {
        let ins: Vec<&str> = n.inputs.iter().map(|p| p.name.as_str()).collect();
        let outs: Vec<&str> = n.outputs.iter().map(|p| p.name.as_str()).collect();
        println!("  [{i}] {} ({}){}  in: {}  out: {}", n.id, n.kind, n.world.as_ref().map(|w| format!(" world {}{}", w.source, w.address.as_deref().map(|a| format!(" {a}")).unwrap_or_default())).unwrap_or_default(), ins.join(", "), outs.join(", "));
    }
    for l in &v.links {
        println!("  cable #{}: {}.{} → {}.{}  {}  home {:?}{}", l.id, v.nodes[l.from].id, l.out, v.nodes[l.to].id, l.input, l.ty, l.home, l.readonly.map(|r| format!("  read-only: {r}")).unwrap_or_default());
    }
    for s in &v.stubs {
        let arrow = if s.input { "←" } else { "→" };
        println!("  stub  #{}: {}.{} {arrow} {}  {}  home {:?}{}", s.id, v.nodes[s.node].id, s.port, s.other, s.ty, s.home, s.readonly.map(|r| format!("  read-only: {r}")).unwrap_or_default());
    }
}

fn main() -> anyhow::Result<()> {
    let b = Bundle::core("character.catpope").expect("the core rig")?;
    let played = b.play(None, None)?;
    let asked: Vec<String> = std::env::args().skip(1).collect();
    let contexts: Vec<&str> = if asked.is_empty() { vec!["", "voice", "voice:voice", "fft"] } else { asked.iter().map(String::as_str).collect() };
    for c in contexts {
        show(&played.wire_view(c)?);
    }
    // W6: one gesture on the rig document – the voice sent into the drone's mixer at half level –, then the rig as played again
    let mut doc = Doc::Rigs(b.rig.clone());
    let op = Op::Connect { from: "voice:voice.out".into(), to: "drone:mix.in3".into(), scale: 0.5, offset: 0.0 };
    println!("\n== edit {op:?}");
    match doc.apply(&op, &b.library()) {
        Ok(()) => {
            let Doc::Rigs(rig) = &doc else { unreachable!() };
            let edited = Bundle { rig: rig.clone(), ..b.clone() }.play(None, None)?;
            show(&edited.wire_view("")?);
            println!("\n== the rig document as text (the view's ids, the form the core sets ship in)\n{}", doc.to_toml()?);
            println!("problems after the edit: {:?}", deck_space::rig::validate::Request { kind: "rigs".into(), id: rig.subject.clone(), document: serde_json::to_value(rig)?, instruments: b.instruments.iter().map(serde_json::to_value).collect::<Result<_, _>>()?, presets: b.presets.iter().map(serde_json::to_value).collect::<Result<_, _>>()?, modules: vec![] }.answer().map(|a| a.problems));
        }
        Err(e) => println!("refused: {e}"),
    }
    Ok(())
}
