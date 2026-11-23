//! stage.rs (#496): the stage's pick on the headless deck – deck-space's `Space` over the last loaded scene document (its
//! cast, the camera, the pointer), driven as a deck's window drives it: a click (on a member where the core draws it, or
//! at a pixel), a key, a drag on a knob of the hand. What a knob's drag turned goes to the engine (`send`), so a `heard`
//! step hears it. No GPU, no loader: a stage whose set is named stays 3D (its 3D on its way), as on a deck before it lands.
use crate::deck::{round, Deck};
use deck_space::graph::{Graph, Kind};
use deck_space::scene::Scene;
use deck_space::space::portrait::Portraits;
use deck_space::space::values::{path_name, Knobs, Path, Table};
use deck_space::space::Space;
use deck_space::tempo;
use live_core::wire::Mode;
use serde_json::{json, Map, Value};

/// the window the clicks are in (pixels)
const VIEW: (f32, f32) = (1280.0, 720.0);
/// frames before the first input: the camera has settled on the stage
const SETTLE: usize = 120;
const DT: f32 = 1.0 / 60.0;

/// The control domain the space turns: a `Table` of the graph, and what a hand set on it since the last ask.
struct Hands {
    table: Table,
    turned: Vec<(usize, usize, f32)>,
}

impl Knobs for Hands {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        self.table.input(node, port, default)
    }
    fn outputs(&self, node: usize) -> &[f32] {
        self.table.outputs(node)
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        self.table.lfo_shape(node)
    }
    fn set(&mut self, node: usize, port: usize, value: f32) -> bool {
        self.turned.retain(|t| (t.0, t.1) != (node, port));
        self.turned.push((node, port, value));
        self.table.set(node, port, value)
    }
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.table.base(node, port)
    }
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        self.table.set_lfo_shape(node, shape)
    }
}

pub struct Stage {
    graph: Graph,
    space: Space,
    hands: Hands,
    time: f32,
    /// the scene's camera, taken on by the stage at its first camera word
    camera: Option<Option<deck_space::scene_camera::SceneCamera>>,
}

impl Stage {
    /// the stage of a resolved scene (its `space` node, its `look` nodes), the camera settled
    pub fn new(scene: &Scene) -> Result<Stage, String> {
        let graph = Graph::build(scene).map_err(|e| format!("{e:#}"))?;
        let node = graph.first(Kind::Space).ok_or("the scene has no space node – no stage to pick on")?;
        let mut space = Space::new(&graph, node, &scene.being, std::path::Path::new(""), Portraits::manual());
        space.set_view(VIEW.0, VIEW.1);
        let hands = Hands { table: Table::new(&graph), turned: Vec::new() };
        let mut stage = Stage { graph, space, hands, time: 0.0, camera: Some(scene.camera.clone()) };
        stage.frames(SETTLE);
        Ok(stage)
    }

    /// A camera word (`scene:<id> > shot:<id>` / `move:<id>`) to the stage's camera, as a deck's Space takes the relay's
    /// `address`; its glide later: did (camera, not_mine), what the stage looks through, its lens.
    pub fn camera(&mut self, address: &str, action: &str) -> Value {
        if let Some(camera) = self.camera.take() {
            self.space.set_camera(None, camera);
        }
        let took = self.space.camera_word(address, action);
        self.frames(75);
        let viewing = match self.space.viewing() {
            deck_space::space::dop::Viewing::Free => "free".to_string(),
            deck_space::space::dop::Viewing::Primary => "primary".to_string(),
            deck_space::space::dop::Viewing::Shot(id) => format!("shot:{id}"),
        };
        let c = &self.space.camera;
        json!({ "did": if took { "camera" } else { "not_mine" }, "viewing": viewing, "ortho": c.ortho.map(|h| round(f64::from(h), 2)), "fov": round(f64::from(c.fov_y.to_degrees()), 1) })
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.time += DT;
            self.space.update(DT, self.time, &mut self.hands, &tempo::Now::default(), &|_, _| 0.0);
        }
    }

    fn click(&mut self, (x, y): (f32, f32)) {
        self.space.pointer_moved(x, y, &mut self.hands);
        self.space.press();
        self.space.release();
        self.frames(1);
    }

    /// One `pick` step: `click` (a holder – its member where the core draws it – or `[x, y]`), `key` (+ `shift`), `drag`
    /// (a knob `node.port` of the hand, pulled `by` of its tether) → who is picked, the selection, the hand's knobs, the
    /// mark; a drag's `reach` and what it `turned` (sent to `deck`).
    pub fn step(&mut self, step: &toml::Table, deck: &mut Deck) -> Result<Value, String> {
        let mut out = Map::new();
        match step.get("click") {
            Some(toml::Value::String(holder)) => {
                let at = self.space.member_pixel(holder).ok_or_else(|| format!("no cast member at '{holder}' on the screen"))?;
                self.click(at);
            }
            Some(toml::Value::Array(xy)) => {
                let n = |i: usize| xy.get(i).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(0.0) as f32;
                self.click((n(0), n(1)));
            }
            Some(v) => return Err(format!("click = {v}: a holder or [x, y]")),
            None => {}
        }
        if let Some(key) = step.get("key").and_then(|v| v.as_str()) {
            let shift = step.get("shift").and_then(|v| v.as_bool()).unwrap_or(false);
            out.insert("key".into(), json!(self.space.key(key, shift)));
            self.frames(1);
        }
        if let Some(name) = step.get("drag").and_then(|v| v.as_str()) {
            let by = step.get("by").and_then(|v| v.as_float()).unwrap_or(0.3) as f32;
            let reach = self.drag(name, by);
            out.insert("reach".into(), json!(reach));
            let mut turned = Map::new();
            for (node, port, v) in std::mem::take(&mut self.hands.turned) {
                let path = path_name(&self.graph, Path::Input(node, port));
                deck.send(&path, v, Mode::Force, 1.0)?;
                turned.insert(path, json!(round(f64::from(v), 2)));
            }
            out.insert("turned".into(), Value::Object(turned));
        }
        out.insert("picked".into(), json!(self.space.pick_holder().unwrap_or_default()));
        // #819: the sing button held (true) or let go, then the stage's time running on (`wait`, ms): the triggers go to
        // the engine
        if let Some(on) = step.get("sing").and_then(|v| v.as_bool()) {
            out.insert("sang".into(), json!(self.space.sing(on)));
            self.frames(1);
        }
        if let Some(ms) = step.get("wait").and_then(|v| v.as_integer().or_else(|| v.as_float().map(|f| f as i64))) {
            self.frames((ms.max(0) as f32 / 1000.0 / DT).ceil() as usize);
        }
        out.insert("triggers".into(), self.send_triggers(deck)?);
        out.insert("picked".into(), json!(self.space.pick_holder().unwrap_or_default()));
        out.insert("selection".into(), json!(self.space.selection_name()));
        out.insert("knobs".into(), json!(self.space.hand_paths()));
        out.insert("mark".into(), json!(self.space.mark().is_some()));
        Ok(Value::Object(out))
    }

    /// #819: a story event to the stage (Space `sing_line`: a text line opens its character's trigger), the triggers sent
    /// → whether one opened, the triggers
    pub fn story(&mut self, event: &Value, deck: &mut Deck) -> Result<Value, String> {
        let opened = self.space.sing_line(event);
        self.frames(1);
        Ok(json!({ "opened": opened, "triggers": self.send_triggers(deck)? }))
    }

    /// each trigger's value as the stage set it, to the engine → path → value
    fn send_triggers(&mut self, deck: &mut Deck) -> Result<Value, String> {
        let mut out = Map::new();
        for node in self.space.sing_nodes().collect::<Vec<_>>() {
            let v = self.hands.table.input(node, "a", 0.0);
            let path = path_name(&self.graph, Path::Input(node, 0));
            deck.send(&path, v, Mode::Force, 1.0)?;
            out.insert(path, json!(round(f64::from(v), 2)));
        }
        Ok(Value::Object(out))
    }

    /// a deck's drag on the hand's knob `name`: the pointer on it, pressed, pulled `by` along its tether, let go (native's
    /// `--drag`); false = no such knob in the hand
    fn drag(&mut self, name: &str, by: f32) -> bool {
        self.hands.turned.clear();
        let Some(i) = self.space.button(&self.graph, name) else { return false };
        let Some(at) = self.space.to_pixel(self.space.hand().buttons[i].pos) else { return false };
        self.space.pointer_moved(at.0, at.1, &mut self.hands);
        self.space.press();
        let (_, axis) = self.space.drag_axis(i);
        self.space.pointer_moved(at.0 + axis.0 * by, at.1 + axis.1 * by, &mut self.hands);
        self.space.release();
        self.frames(1);
        true
    }
}
