//! The routing scenes of P4b (#318): what the engine plays on several device channels. One case = a scene and the deck
//! configuration it plays on; the render is every device channel, interleaved, 2 s at 48 kHz. These are the P0 probe's
//! routing graph (plan 3.8) and the acceptance scenes: a 5.1 bed on a six-channel device and folded on two, a cue outlet on
//! channels 3-4, the feedback loop.
use deck_engine::DeckConfig;

pub struct RoutingCase {
    pub name: &'static str,
    pub toml: String,
    pub deck: DeckConfig,
}

fn osc(id: &str, freq: f32, gain: f32) -> String {
    format!("[[node]]\nid = \"{id}\"\nkind = \"osc\"\nfreq = {freq}\ngain = {gain}\n")
}

/// six oscillators at the pitches of a 5.1 bed (L R C LFE Ls Rs), one per channel of a six-channel bus `bed`, and `bed` into
/// an outlet of the `main` group
fn bed() -> String {
    let mut s = String::from("[[node]]\nid = \"bed\"\nkind = \"bus\"\nchannels = 6\n");
    for (k, f) in [220.0, 261.6, 329.6, 110.0, 392.0, 440.0].iter().enumerate() {
        s += &osc(&format!("s{k}"), *f, 0.08);
        s += &format!("[[link]]\nfrom = \"s{k}\"\nto = \"bed.in\"\nmap = [[1, {}]]\n", k + 1);
    }
    s + "[[node]]\nid = \"bedout\"\nkind = \"out\"\nlayout = \"5.1\"\n[[link]]\nfrom = \"bed.out\"\nto = \"bedout.in\"\n"
}

/// the P0 routing graph on eight device channels: a send 1/2 -> 3/4 at half level into the cue outlet on 3-4, a folder summing
/// two buses as the main outlet on 1-2, a mono source VBAP-panned over four speakers on 5-8, a 5.1 bed folded onto 1-2 by the
/// deck configuration's fold
fn probe() -> String {
    let mut s = osc("a", 220.0, 0.1) + &osc("b", 330.0, 0.08) + &osc("m", 440.0, 0.1);
    s += "[[node]]\nid = \"ta\"\nkind = \"bus\"\n[[node]]\nid = \"tb\"\nkind = \"bus\"\ngain = 0.5\n[[node]]\nid = \"folder\"\nkind = \"bus\"\n\
          [[node]]\nid = \"sendbus\"\nkind = \"bus\"\nchannels = 4\n\
          [[node]]\nid = \"main\"\nkind = \"out\"\n[[node]]\nid = \"cue\"\nkind = \"out\"\ngroup = \"cue\"\n\
          [[node]]\nid = \"pan\"\nkind = \"pan\"\nmode = \"vbap\"\naz = [45.0, 135.0, 225.0, 315.0]\nazimuth = 100.0\n\
          [[node]]\nid = \"array\"\nkind = \"out\"\ngroup = \"array\"\nfirst = 5\n\
          [[link]]\nfrom = \"a\"\nto = \"ta.in\"\n[[link]]\nfrom = \"b\"\nto = \"tb.in\"\n\
          [[link]]\nfrom = \"ta.out\"\nto = \"folder.in\"\n[[link]]\nfrom = \"tb.out\"\nto = \"folder.in\"\n\
          [[link]]\nfrom = \"ta.pre\"\nto = \"sendbus.in\"\nscale = 0.5\nmap = [[1, 3], [2, 4]]\n\
          [[link]]\nfrom = \"folder.out\"\nto = \"main.in\"\n\
          [[link]]\nfrom = \"sendbus.out\"\nto = \"cue.in\"\nmap = [[3, 1], [4, 2]]\n\
          [[link]]\nfrom = \"m\"\nto = \"pan.in\"\n[[link]]\nfrom = \"pan.out\"\nto = \"array.in\"\n";
    s + &bed()
}

pub fn cases() -> Vec<RoutingCase> {
    let probe_deck = DeckConfig::device(8).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]);
    vec![
        RoutingCase { name: "routing graph (8 ch: send, folder, cue, VBAP, 5.1 fold)", toml: probe(), deck: probe_deck },
        RoutingCase { name: "5.1 on a 6-channel device", toml: bed(), deck: DeckConfig::device(6) },
        RoutingCase { name: "5.1 folded on 2 channels", toml: bed(), deck: DeckConfig::stereo() },
        RoutingCase {
            name: "cue outlet on channels 3-4",
            toml: osc("a", 220.0, 0.2) + &osc("b", 330.0, 0.2) + "[[node]]\nid = \"m\"\nkind = \"out\"\n[[node]]\nid = \"c\"\nkind = \"out\"\ngroup = \"cue\"\n[[link]]\nfrom = \"a\"\nto = \"m.in\"\n[[link]]\nfrom = \"b\"\nto = \"c.in\"\n",
            deck: DeckConfig::device(4).with_outlet("cue", &[3, 4]),
        },
        RoutingCase {
            name: "feedback loop (stereo)",
            toml: osc("a", 220.0, 0.05) + "[[node]]\nid = \"b\"\nkind = \"bus\"\n[[node]]\nid = \"fb\"\nkind = \"feedback\"\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\nvolume = 1.0\n\
                [[link]]\nfrom = \"a\"\nto = \"b.in\"\n[[link]]\nfrom = \"fb.out\"\nto = \"b.in\"\nscale = 0.5\n[[link]]\nfrom = \"b.out\"\nto = \"fb.in\"\n[[link]]\nfrom = \"b.out\"\nto = \"out.in\"\n",
            deck: DeckConfig::stereo(),
        },
    ]
}
