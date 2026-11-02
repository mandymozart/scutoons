//! pad.rs (#228): a gamepad on the native stage – the input adapter for deck-space's controls. The engine's half is
//! deck-space's `controls` (the selection → the stick pairs, `stick_down` / `stick_move` / `stick_up`), the same the
//! browser's Gamepad API drives (shared/live-deck/src/controls.ts). This file only turns a pad's sticks and buttons into
//! those calls, with the same rules: a stick drives what the selection says, it is down while it is deflected past the
//! dead zone, and a button cycles the selection – master section → the being → each of its knobs → … – where a tap in
//! the space would select the same.
//!
//! The layout is the Gamepad API's standard one: the left stick drives the left stick, the right stick the right one;
//! the south button (A / cross) cycles forward, the north button (Y / triangle) back. gilrs reports a stick's up as
//! positive, which is the controls' y (up). Built with `--features gamepad`; without a pad attached nothing happens.
//! A stick's movement never allocates; a button press may (the selection's name).
use deck_space::controls::{Controls as Sticks, Selection, DEAD, LEFT, RIGHT};
use deck_space::graph::Graph;
use deck_space::space::values::Knobs;
use deck_space::space::Space;

/// One input, the adapter's own vocabulary: gilrs's events map onto it (`from_gilrs`), the tests feed it directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// a stick axis: its `side` (LEFT / RIGHT), `axis` (0 x, 1 y) and deflection (−1..1, y up)
    Axis { side: usize, axis: usize, value: f32 },
    /// the selection button: the next selection, or the previous one (`back`)
    Cycle { back: bool },
}

/// The pad's state on the native stage: gilrs (None = no pad system, the stage plays without one), deck-space's stick
/// engine over the being's knobs, the selections in cycle order, and each stick's deflection and whether it is down.
pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
    sticks: Sticks,
    /// the being's knob paths, `node.port` – `paths[i]` is the being's button `i` (the engine's index order)
    paths: Vec<String>,
    /// the selections in cycle order: the master section (""), the being, then each knob (`knob:<path>`)
    names: Vec<String>,
    deflect: [[f32; 2]; 2],
    down: [bool; 2],
}

impl Pad {
    /// The pad over `space`'s being, reading the gamepads the system offers. A pad system that can't start is logged and
    /// the stage plays without one.
    pub fn new(graph: &Graph, space: &Space) -> Pad {
        let mut pad = Pad::with_paths(graph, space);
        pad.gilrs = match gilrs::Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                tracing::warn!("gamepad off: {e}");
                None
            }
        };
        pad
    }

    /// The pad's state without a reader: the being's knobs, the selections. Tests build it this way (no hardware).
    fn with_paths(graph: &Graph, space: &Space) -> Pad {
        let paths: Vec<String> = space.being.buttons.iter().map(|b| format!("{}.{}", graph.nodes[b.node].id, b.name)).collect();
        let names: Vec<String> = [String::new(), "being".to_string()].into_iter().chain(paths.iter().map(|p| format!("knob:{p}"))).collect();
        Pad { gilrs: None, sticks: Sticks::new(paths.as_slice()), paths, names, deflect: [[0.0; 2]; 2], down: [false; 2] }
    }

    /// This frame's pad events, in order: a stick moves the knobs, a button cycles the selection. Without a pad system or
    /// a pad, nothing.
    pub fn poll(&mut self, space: &mut Space, ctl: &mut dyn Knobs) {
        let Some(mut reader) = self.gilrs.take() else { return };
        while let Some(event) = reader.next_event() {
            if let Some(input) = from_gilrs(&event.event) {
                self.handle(input, space, ctl);
            }
        }
        self.gilrs = Some(reader);
    }

    /// One input: the selection moves on, or a stick's deflection drives the knobs it drives (down when it leaves rest,
    /// up when it returns).
    pub fn handle(&mut self, input: Input, space: &mut Space, ctl: &mut dyn Knobs) {
        match input {
            Input::Cycle { back } => self.cycle(back, space),
            Input::Axis { side, axis, value } => {
                let (side, axis) = (side.min(1), axis.min(1));
                self.deflect[side][axis] = value.clamp(-1.0, 1.0);
                let [dx, dy] = self.deflect[side];
                let deflected = dx.abs() >= DEAD || dy.abs() >= DEAD;
                if !deflected {
                    if self.down[side] {
                        self.sticks.stick_up(side);
                        self.down[side] = false;
                    }
                    return;
                }
                if !self.down[side] {
                    self.stick_down(side, space);
                    self.down[side] = true;
                }
                self.drive(side, dx, dy, space, ctl);
            }
        }
    }

    /// The thumb lands: the sticks take the selection now (a tap in the space since the last touch counts) and the
    /// values they start from.
    fn stick_down(&mut self, side: usize, space: &Space) {
        self.sticks.select(Selection::parse(&space.selection_name()));
        let [px, py] = self.sticks.stick_paths(side);
        let norm = |p: &str| self.paths.iter().position(|q| q == p).and_then(|i| space.being.buttons.get(i)).map_or(0.0, |b| b.norm);
        let start = [norm(px), norm(py)];
        self.sticks.stick_down(side, start[0], start[1]);
    }

    /// The deflection moves the values the stick drives, from where they were when it went down. A turnable knob takes
    /// the value as the pointer's drag would set it; a linked one (its base unknown) doesn't.
    fn drive(&self, side: usize, dx: f32, dy: f32, space: &Space, ctl: &mut dyn Knobs) {
        let values = self.sticks.stick_move(side, dx, dy);
        for (path, n) in self.sticks.stick_driving(side).into_iter().zip(values) {
            let Some(b) = self.paths.iter().position(|q| q == path).and_then(|i| space.being.buttons.get(i)) else { continue };
            if !b.turnable {
                continue;
            }
            let v = b.spec.value(n);
            if b.shape {
                ctl.set_lfo_shape(b.node, v.round() as usize);
            } else {
                ctl.set(b.node, b.port, v);
            }
        }
    }

    /// The button: the next selection (or the previous one), the space's ring follows and the sticks take it.
    fn cycle(&mut self, back: bool, space: &mut Space) {
        let n = self.names.len();
        let now = space.selection_name();
        let at = self.names.iter().position(|q| *q == now).unwrap_or(0);
        let next = if back { (at + n - 1) % n } else { (at + 1) % n };
        space.select_named(&self.names[next]);
        self.sticks.select(Selection::parse(&self.names[next]));
    }
}

/// gilrs's event as an input: a stick axis as it moves, a button on its press.
pub fn from_gilrs(event: &gilrs::EventType) -> Option<Input> {
    match *event {
        gilrs::EventType::AxisChanged(axis, value, _) => axis_input(axis, value),
        gilrs::EventType::ButtonPressed(button, _) => button_input(button),
        _ => None,
    }
}

/// a stick axis (the left and right sticks' x and y) as an input
pub fn axis_input(axis: gilrs::Axis, value: f32) -> Option<Input> {
    let (side, axis) = match axis {
        gilrs::Axis::LeftStickX => (LEFT, 0),
        gilrs::Axis::LeftStickY => (LEFT, 1),
        gilrs::Axis::RightStickX => (RIGHT, 0),
        gilrs::Axis::RightStickY => (RIGHT, 1),
        _ => return None,
    };
    Some(Input::Axis { side, axis, value })
}

/// the selection buttons: south (A / cross) forward, north (Y / triangle) back
pub fn button_input(button: gilrs::Button) -> Option<Input> {
    match button {
        gilrs::Button::South => Some(Input::Cycle { back: false }),
        gilrs::Button::North => Some(Input::Cycle { back: true }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Controls as Knobset;
    use deck_space::graph::{Graph, Kind};
    use deck_space::scene::{Scene, CATHEDRAL_SCENE};
    use deck_space::space::portrait::Portraits;
    use deck_space::controls::STICK_GAIN;
    use deck_space::tempo::Now;
    use std::path::Path;

    /// the cathedral's space and the stage's knobs over it (as the show builds them), its view set and one frame run
    fn cathedral() -> (Graph, Space, Knobset) {
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        let mut ctl = Knobset::new(&g, None);
        space.set_view(1280.0, 720.0);
        space.update(0.016, 0.0, &mut ctl, &Now::default(), &|_, _| 0.0);
        (g, space, ctl)
    }

    /// the gamepad's layout: the sticks' axes, the buttons that cycle, and nothing else
    #[test]
    fn the_standard_layout_maps_sticks_and_buttons() {
        assert_eq!(axis_input(gilrs::Axis::LeftStickX, 0.5), Some(Input::Axis { side: LEFT, axis: 0, value: 0.5 }));
        assert_eq!(axis_input(gilrs::Axis::LeftStickY, -1.0), Some(Input::Axis { side: LEFT, axis: 1, value: -1.0 }));
        assert_eq!(axis_input(gilrs::Axis::RightStickX, 0.0), Some(Input::Axis { side: RIGHT, axis: 0, value: 0.0 }));
        assert_eq!(axis_input(gilrs::Axis::RightStickY, 1.0), Some(Input::Axis { side: RIGHT, axis: 1, value: 1.0 }));
        assert_eq!(axis_input(gilrs::Axis::LeftZ, 1.0), None, "a trigger isn't a stick");
        assert_eq!(button_input(gilrs::Button::South), Some(Input::Cycle { back: false }));
        assert_eq!(button_input(gilrs::Button::North), Some(Input::Cycle { back: true }));
        assert_eq!(button_input(gilrs::Button::Start), None);
    }

    /// the button walks master → the being → each knob → back to master; the space's ring and the sticks follow
    #[test]
    fn the_button_cycles_the_selection_like_a_tap() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        assert_eq!(space.selection_name(), "");
        pad.handle(Input::Cycle { back: false }, &mut space, &mut ctl);
        assert_eq!(space.selection_name(), "being");
        assert_eq!(pad.sticks.selection(), &Selection::Being, "the sticks take it");
        pad.handle(Input::Cycle { back: false }, &mut space, &mut ctl);
        let first = format!("knob:{}", pad.paths[0]);
        assert_eq!(space.selection_name(), first);
        pad.handle(Input::Cycle { back: true }, &mut space, &mut ctl);
        assert_eq!(space.selection_name(), "being");
        pad.handle(Input::Cycle { back: true }, &mut space, &mut ctl);
        assert_eq!(space.selection_name(), "", "back to the master section");
        pad.handle(Input::Cycle { back: true }, &mut space, &mut ctl);
        assert_eq!(space.selection_name(), pad.names.last().unwrap().as_str(), "from the master section back: the last knob");
    }

    /// a stick deflected past the dead zone is down and moves the being's pair from where it was; full deflection
    /// reaches the values the engine's STICK_GAIN gives; returning to rest lets it go; a deflection inside the dead zone
    /// does nothing
    #[test]
    fn a_stick_drives_the_being_from_where_it_was() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        pad.handle(Input::Cycle { back: false }, &mut space, &mut ctl);
        let ratio = space.being.buttons[pad.paths.iter().position(|p| p == "voice.ratio").unwrap()].clone();
        let index = space.being.buttons[pad.paths.iter().position(|p| p == "voice.index").unwrap()].clone();
        let (start_x, start_y) = (ratio.norm, index.norm);
        pad.handle(Input::Axis { side: LEFT, axis: 0, value: 0.02 }, &mut space, &mut ctl);
        assert!(!pad.down[LEFT], "the dead zone");
        pad.handle(Input::Axis { side: LEFT, axis: 0, value: 1.0 }, &mut space, &mut ctl);
        assert!(pad.down[LEFT] && pad.sticks.stick_is_down(LEFT));
        let want = (start_x + STICK_GAIN).min(1.0);
        assert!((ctl.input(ratio.node, ratio.name, -9.0) - ratio.spec.value(want)).abs() < 1e-4, "x: the ratio");
        pad.handle(Input::Axis { side: LEFT, axis: 1, value: -1.0 }, &mut space, &mut ctl);
        let want_y = (start_y - STICK_GAIN).max(0.0);
        assert!((ctl.input(index.node, index.name, -9.0) - index.spec.value(want_y)).abs() < 1e-4, "y: the index");
        pad.handle(Input::Axis { side: LEFT, axis: 0, value: 0.0 }, &mut space, &mut ctl);
        pad.handle(Input::Axis { side: LEFT, axis: 1, value: 0.0 }, &mut space, &mut ctl);
        assert!(!pad.down[LEFT] && !pad.sticks.stick_is_down(LEFT), "back at rest: up");
    }

    /// a pad that can't be opened, or none attached: the poll does nothing and the selection stays
    #[test]
    fn no_pad_no_input() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        pad.poll(&mut space, &mut ctl);
        assert_eq!(space.selection_name(), "");
        assert!(!pad.down.iter().any(|d| *d));
    }
}
