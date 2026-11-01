//! A patch (#236, research/EVENTS.md §3 "compound generators"): generators rigged together as data – what a rig's
//! `events` node holds (the stage's host node, live-core's `[[events]]`), so an arpeggiator on a random walk is a few
//! lines of TOML or JSON, not code. One builder for both sides: the same patch plays the same notes everywhere.
//!
//! ```toml
//! generator = [
//!   { id = "walk", kind = "random_walk", seed = 7, rate = 0.5, spread = 0.6 },
//!   { id = "arp", kind = "arp", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = "updown", rate = 4.0 },
//! ]
//! edge = [{ from = "walk.notes", to = "arp.notes" }]   # `delayed = true` for a loop
//! out = "arp.notes"                                     # the notes the host plays
//! knobs = ["walk.spread", "arp.rate"]                   # the host's inputs a, b, c, d, in order
//! listen = [{ to = "arp.notes", name = "midi.note" }]   # routed in (route.rs): a `name` or a whole `domain`
//! ```
//! Kinds: `random_walk` (seed, base, spread, octaves, rate, length, glide), `pulse` (every, offset), `arp` (ratios,
//! octaves, mode up · down · updown, rate, length). Notes are `music.note` with `pitch` (and `glide`).
use crate::generator::Generator;
use crate::generators::{Arp, ArpMode, Pulse, RandomWalk};
use crate::graph::EventGraph;
use crate::names::{Names, Sym};
use crate::route::Match;
use serde::Deserialize;

/// the host's knob inputs (a patch maps at most this many)
pub const KNOBS: usize = 4;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Patch {
    #[serde(default)]
    pub generator: Vec<GenDef>,
    #[serde(default)]
    pub edge: Vec<EdgeDef>,
    /// `generator.port` whose events the host plays
    pub out: String,
    /// `generator.param` per host input a, b, c, d
    #[serde(default)]
    pub knobs: Vec<String>,
    /// external events into generator inputs: the dispatcher routes them here (route.rs)
    #[serde(default)]
    pub listen: Vec<ListenDef>,
}

/// `to` = `generator.input`; exactly one of `name` (`midi.note`) or `domain` (`story`)
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListenDef {
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
}

/// a built `listen`: the generator, its input (an index for `EventGraph::inject_at`), what it hears
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listen {
    pub node: usize,
    pub port: usize,
    pub on: Match,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GenDef {
    pub id: String,
    pub kind: String,
    #[serde(flatten)]
    pub params: toml::Table,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeDef {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub delayed: bool,
}

/// the names every patch speaks
#[derive(Debug, Clone, Copy)]
pub struct NoteNames {
    pub note: Sym,
    pub pitch: Sym,
    pub glide: Sym,
}

impl NoteNames {
    pub fn new(names: &mut Names) -> NoteNames {
        NoteNames {
            note: names.event("music.note").expect("a valid name"),
            pitch: names.key("pitch").expect("a valid key"),
            glide: names.key("glide").expect("a valid key"),
        }
    }
}

/// a built patch: the graph (built), the generator and port it plays, and (generator, param) per knob
pub struct Built {
    pub graph: EventGraph,
    pub out: (usize, String),
    pub knobs: Vec<(usize, usize)>,
    pub listen: Vec<Listen>,
}

fn num(t: &toml::Table, k: &str) -> Option<f64> {
    t.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
}

fn gen(d: &GenDef, n: NoteNames, names: &mut Names) -> Result<Box<dyn Generator>, String> {
    let p = &d.params;
    let f = |k: &str, default: f64| num(p, k).unwrap_or(default);
    Ok(match d.kind.as_str() {
        "random_walk" => Box::new(RandomWalk {
            note: n.note,
            pitch: n.pitch,
            glide_key: n.glide,
            seed: f("seed", 1.0) as u64,
            base_hz: f("base", 147.0) as f32,
            spread: f("spread", 0.0) as f32,
            octaves: f("octaves", 2.0) as f32,
            per_beat: f("rate", 2.0),
            length: f("length", 1.0),
            glide: f("glide", 0.5) as f32,
        }),
        "pulse" => Box::new(Pulse {
            name: names.event(p.get("name").and_then(|v| v.as_str()).unwrap_or("music.pulse")).map_err(|e| format!("{}: {e}", d.id))?,
            index: names.key("index").expect("a valid key"),
            every: f("every", 1.0),
            offset: f("offset", 0.0),
        }),
        "arp" => {
            let ratios: Vec<f32> = match p.get("ratios") {
                Some(toml::Value::Array(a)) => a.iter().filter_map(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).map(|x| x as f32).collect(),
                _ => vec![1.0, 1.25, 1.5],
            };
            let mode = match p.get("mode").and_then(|v| v.as_str()).unwrap_or("up") {
                "up" => ArpMode::Up,
                "down" => ArpMode::Down,
                "updown" => ArpMode::UpDown,
                m => return Err(format!("{}: unknown mode '{m}' (up, down, updown)", d.id)),
            };
            Box::new(Arp {
                note: n.note,
                pitch: n.pitch,
                ratios,
                octaves: f("octaves", 1.0) as u32,
                mode,
                per_beat: f("rate", 4.0),
                length: f("length", 0.5),
                root: None,
            })
        }
        k => return Err(format!("generator '{}': unknown kind '{k}' (random_walk, pulse, arp)", d.id)),
    })
}

/// `"id.port"` → (generator index, port)
fn split<'a>(ids: &[String], r: &'a str) -> Result<(usize, &'a str), String> {
    let (id, port) = r.rsplit_once('.').ok_or_else(|| format!("'{r}' is not <generator>.<port>"))?;
    let i = ids.iter().position(|x| x == id).ok_or_else(|| format!("no generator '{id}' ({r})"))?;
    Ok((i, port))
}

impl Patch {
    /// read from a TOML table (a node's params – other keys are ignored)
    pub fn from_table(t: &toml::Table) -> Result<Patch, String> {
        toml::Value::Table(t.clone()).try_into().map_err(|e| format!("the patch: {e}"))
    }

    pub fn build(&self, n: NoteNames, names: &mut Names) -> Result<Built, String> {
        let mut graph = EventGraph::new();
        let ids: Vec<String> = self.generator.iter().map(|g| g.id.clone()).collect();
        for (i, id) in ids.iter().enumerate() {
            if ids[..i].contains(id) {
                return Err(format!("generator '{id}' twice"));
            }
        }
        for d in &self.generator {
            graph.add_boxed(gen(d, n, names)?);
        }
        for e in &self.edge {
            let (f, fp) = split(&ids, &e.from)?;
            let (t, tp) = split(&ids, &e.to)?;
            graph.connect((f, fp), (t, tp), e.delayed).map_err(|x| x.to_string())?;
        }
        graph.build().map_err(|e| e.to_string())?;
        let (o, op) = split(&ids, &self.out)?;
        graph.outputs(o, op).map_err(|e| e.to_string())?;
        if self.knobs.len() > KNOBS {
            return Err(format!("at most {KNOBS} knobs (a, b, c, d), not {}", self.knobs.len()));
        }
        let knobs = self
            .knobs
            .iter()
            .map(|k| {
                let (g, param) = split(&ids, k)?;
                Ok((g, graph.param(g, param).map_err(|e| e.to_string())?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let listen = self
            .listen
            .iter()
            .map(|l| {
                let (g, port) = split(&ids, &l.to)?;
                let port = graph.input(g, port).map_err(|e| e.to_string())?;
                let on = match (&l.name, &l.domain) {
                    (Some(n), None) => Match::Name(names.event(n).map_err(|e| format!("listen {}: {e}", l.to))?),
                    (None, Some(d)) => Match::Domain(names.key(d).map_err(|e| format!("listen {}: domain {e}", l.to))?),
                    _ => return Err(format!("listen {}: a `name` or a `domain`, one of them", l.to)),
                };
                Ok(Listen { node: g, port, on })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Built { graph, out: (o, op.to_string()), knobs, listen })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::Span;
    use deck_core::tempo::TempoMap;

    const ARP_ON_A_WALK: &str = r#"
generator = [
  { id = "walk", kind = "random_walk", seed = 7, rate = 0.5, spread = 0.6 },
  { id = "arp", kind = "arp", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = "updown", rate = 4.0 },
]
edge = [{ from = "walk.notes", to = "arp.notes" }]
out = "arp.notes"
knobs = ["walk.spread", "arp.rate"]
seed_of_the_node = 1
"#;

    fn build(text: &str) -> Result<Built, String> {
        let mut names = Names::new();
        let n = NoteNames::new(&mut names);
        Patch::from_table(&toml::from_str(text).unwrap())?.build(n, &mut names)
    }

    #[test]
    fn a_patch_builds_and_plays_an_arp_on_a_walk() {
        let mut b = build(ARP_ON_A_WALK).unwrap();
        assert_eq!(b.knobs.len(), 2);
        let t = TempoMap::new(120.0, 4, 48_000.0);
        b.graph.run(&Span::frames(&t, 0.0, 24_000.0)).unwrap();
        assert_eq!(b.graph.outputs(b.out.0, &b.out.1).unwrap().len(), 4, "the arp: four steps a beat");
        // knob b = the arp's rate: eight a beat
        let (g, p) = b.knobs[1];
        b.graph.set(g, p, 8.0);
        b.graph.run(&Span::frames(&t, 24_000.0, 48_000.0)).unwrap();
        assert_eq!(b.graph.outputs(b.out.0, &b.out.1).unwrap().len(), 8);
    }

    #[test]
    fn a_wrong_patch_says_what_is_wrong() {
        let err = |t: &str| build(t).err().unwrap();
        assert!(err("out = \"x.notes\"").contains("no generator 'x'"));
        assert!(err("generator = [{ id = \"g\", kind = \"nope\" }]\nout = \"g.notes\"").contains("unknown kind 'nope'"));
        assert!(err("generator = [{ id = \"g\", kind = \"arp\", mode = \"sideways\" }]\nout = \"g.notes\"").contains("unknown mode"));
        assert!(err("generator = [{ id = \"g\", kind = \"random_walk\" }]\nout = \"g.notes\"\nknobs = [\"g.loudness\"]").contains("loudness"));
        let five = "generator = [{ id = \"g\", kind = \"random_walk\" }]\nout = \"g.notes\"\nknobs = [\"g.spread\", \"g.spread\", \"g.spread\", \"g.spread\", \"g.spread\"]";
        assert!(err(five).contains("at most 4"));
        let loop_ = "generator = [{ id = \"a\", kind = \"arp\" }, { id = \"b\", kind = \"arp\" }]\n\
                     edge = [{ from = \"a.notes\", to = \"b.notes\" }, { from = \"b.notes\", to = \"a.notes\" }]\nout = \"b.notes\"";
        assert!(err(loop_).contains("delayed"), "a loop needs a delayed edge");
        assert!(build(&loop_.replace("to = \"a.notes\" }", "to = \"a.notes\", delayed = true }")).is_ok());
        let arp = "generator = [{ id = \"g\", kind = \"arp\" }]\nout = \"g.notes\"\n";
        assert!(err(&format!("{arp}listen = [{{ to = \"g.notes\" }}]")).contains("one of them"));
        assert!(err(&format!("{arp}listen = [{{ to = \"g.nope\", name = \"midi.note\" }}]")).contains("no input 'nope'"));
        assert!(err(&format!("{arp}listen = [{{ to = \"g.notes\", name = \"note\" }}]")).contains("no domain"));
    }

    #[test]
    fn listen_names_an_input_and_what_it_hears() {
        let mut names = Names::new();
        let n = NoteNames::new(&mut names);
        let text = "generator = [{ id = \"g\", kind = \"arp\" }]\nout = \"g.notes\"\n\
                    listen = [{ to = \"g.notes\", name = \"midi.note\" }, { to = \"g.notes\", domain = \"story\" }]";
        let b = Patch::from_table(&toml::from_str(text).unwrap()).unwrap().build(n, &mut names).unwrap();
        let (note, story) = (names.get("midi.note").unwrap(), names.get("story").unwrap());
        assert_eq!(b.listen, [Listen { node: 0, port: 0, on: Match::Name(note) }, Listen { node: 0, port: 0, on: Match::Domain(story) }]);
    }
}
