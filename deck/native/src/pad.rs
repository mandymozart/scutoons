//! pad.rs (#228, #369): a gamepad on the native stage – the input adapter for deck-space's controls. The rules (a stick
//! goes down past its press point and up inside its release point, its deflection, A / Y cycling the selection) are
//! deck-space's (gamepad.rs, `Controls::pad`), the same the browser's Gamepad API hands its polls to (space-web
//! `DeckControls::pad`). This file keeps the pad's state as gilrs reports it, in the Gamepad API's standard mapping (the
//! left stick axes 0 / 1, the right 2 / 3 – gilrs says y up, the mapping y down –, south A / cross button 0, north Y /
//! triangle 3), hands it to the controls once a frame something changed and does what they answer: a stick's values on
//! the being's knobs, the space's ring on the selection. Built with `--features gamepad`; without a pad nothing happens.
use deck_space::controls::{Controls as Sticks, Selection};
use deck_space::gamepad::{PadEvent, PadEvents, AXES, BACK, NEXT};
use deck_space::graph::Graph;
use deck_space::space::values::Knobs;
use deck_space::space::Space;

/// One input in the standard mapping: gilrs's events map onto it (`from_gilrs`), the tests feed it directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// axis `index` (0..3) at −1..1, y down
    Axis { index: usize, value: f32 },
    /// button `index` (NEXT, BACK) pressed or let go
    Button { index: usize, pressed: bool },
    /// the pad went away: everything at rest
    Gone,
}

/// The pad's state on the native stage: gilrs (None = no pad system, the stage plays without one), deck-space's controls
/// over the being's knobs, the pad's axes and buttons, and whether they changed since the last frame.
pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
    sticks: Sticks,
    /// the being's knob paths, `node.port` – `paths[i]` is the being's button `i` (the engine's index order)
    paths: Vec<String>,
    axes: [f32; 4],
    buttons: [bool; 4],
    changed: bool,
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

    /// The pad's state without a reader: the being's knobs. Tests build it this way (no hardware).
    fn with_paths(graph: &Graph, space: &Space) -> Pad {
        let paths: Vec<String> = space.being.buttons.iter().map(|b| format!("{}.{}", graph.nodes[b.node].id, b.name)).collect();
        Pad { gilrs: None, sticks: Sticks::new(paths.as_slice()), paths, axes: [0.0; 4], buttons: [false; 4], changed: false }
    }

    /// This frame's pad events into the pad's state, then the frame; what the deck's menu gets (#386: Start, the d-pad,
    /// A, B) – while the menu is open (`menu`) its buttons are only the menu's. Without a pad system, nothing.
    pub fn poll(&mut self, space: &mut Space, ctl: &mut dyn Knobs, menu: bool) -> Vec<deck_ui::menu::Input> {
        let mut to_menu = Vec::new();
        let Some(mut reader) = self.gilrs.take() else { return to_menu };
        while let Some(event) = reader.next_event() {
            if let Some(i) = menu_input(&event.event) {
                to_menu.push(i);
                if menu {
                    continue;
                }
            }
            if let Some(input) = from_gilrs(&event.event) {
                self.input(input);
            }
        }
        self.gilrs = Some(reader);
        self.frame(space, ctl);
        to_menu
    }

    pub fn input(&mut self, input: Input) {
        match input {
            Input::Axis { index, value } => self.axes.get_mut(index).into_iter().for_each(|a| *a = value),
            Input::Button { index, pressed } => self.buttons.get_mut(index).into_iter().for_each(|b| *b = pressed),
            Input::Gone => (self.axes, self.buttons) = ([0.0; 4], [false; 4]),
        }
        self.changed = true;
    }

    /// One frame (once something changed): the controls take the space's selection (a tap since counts) and the pad's
    /// state; what they answer is done – a stick down takes its knobs' values, its deflection moves them, a cycle rings.
    pub fn frame(&mut self, space: &mut Space, ctl: &mut dyn Knobs) -> PadEvents {
        if !std::mem::take(&mut self.changed) {
            return PadEvents::default();
        }
        self.sticks.select(Selection::parse(&space.selection_name()));
        let events = self.sticks.pad(&self.axes, &self.buttons);
        for e in events.iter() {
            match e {
                PadEvent::Down(side) => self.stick_down(side, space),
                PadEvent::Move(side, dx, dy) => self.drive(side, dx, dy, space, ctl),
                PadEvent::Up(side) => self.sticks.stick_up(side),
                PadEvent::Cycle { .. } => space.select_named(&self.sticks.selection().name()),
            }
        }
        events
    }

    /// The thumb lands: the sticks take the values they start from.
    fn stick_down(&mut self, side: usize, space: &Space) {
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
}

/// gilrs's event as an input: a stick axis as it moves, a selection button pressed or let go, the pad gone.
pub fn from_gilrs(event: &gilrs::EventType) -> Option<Input> {
    match *event {
        gilrs::EventType::AxisChanged(axis, value, _) => axis_input(axis, value),
        gilrs::EventType::ButtonPressed(button, _) => button_input(button, true),
        gilrs::EventType::ButtonReleased(button, _) => button_input(button, false),
        gilrs::EventType::Disconnected => Some(Input::Gone),
        _ => None,
    }
}

/// #386: a pressed button as the deck's menu takes it – Start opens it, the d-pad chooses, A (south) takes, B (east)
/// goes back
pub fn menu_input(event: &gilrs::EventType) -> Option<deck_ui::menu::Input> {
    use deck_ui::menu::Input;
    let gilrs::EventType::ButtonPressed(button, _) = *event else { return None };
    Some(match button {
        gilrs::Button::Start => Input::Open,
        gilrs::Button::DPadUp => Input::Up,
        gilrs::Button::DPadDown => Input::Down,
        gilrs::Button::DPadLeft => Input::Left,
        gilrs::Button::DPadRight => Input::Right,
        gilrs::Button::South => Input::Enter,
        gilrs::Button::East => Input::Back,
        _ => return None,
    })
}

/// a stick axis (gilrs: y up) as the standard mapping's (y down)
pub fn axis_input(axis: gilrs::Axis, value: f32) -> Option<Input> {
    let (index, sign) = match axis {
        gilrs::Axis::LeftStickX => (AXES[0][0], 1.0),
        gilrs::Axis::LeftStickY => (AXES[0][1], -1.0),
        gilrs::Axis::RightStickX => (AXES[1][0], 1.0),
        gilrs::Axis::RightStickY => (AXES[1][1], -1.0),
        _ => return None,
    };
    Some(Input::Axis { index, value: sign * value })
}

/// the selection buttons: south (A / cross) forward, north (Y / triangle) back
pub fn button_input(button: gilrs::Button, pressed: bool) -> Option<Input> {
    let index = match button {
        gilrs::Button::South => NEXT,
        gilrs::Button::North => BACK,
        _ => return None,
    };
    Some(Input::Button { index, pressed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Controls as Knobset;
    use deck_space::controls::STICK_GAIN;
    use deck_space::graph::{Graph, Kind};
    use deck_space::scene::{Scene, CATHEDRAL_SCENE};
    use deck_space::space::portrait::Portraits;
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

    /// the gamepad's layout: the sticks' axes (y flipped to the mapping's), the buttons that cycle, and nothing else
    #[test]
    fn the_standard_layout_maps_sticks_and_buttons() {
        assert_eq!(axis_input(gilrs::Axis::LeftStickY, -1.0), Some(Input::Axis { index: 1, value: 1.0 }));
        assert_eq!(axis_input(gilrs::Axis::RightStickX, 0.5), Some(Input::Axis { index: 2, value: 0.5 }));
        assert_eq!(axis_input(gilrs::Axis::LeftZ, 1.0), None, "a trigger isn't a stick");
        assert_eq!(button_input(gilrs::Button::North, true), Some(Input::Button { index: 3, pressed: true }));
        assert_eq!(button_input(gilrs::Button::Start, true), None);
    }

    /// #369's acceptance: the browser's script (shared/live-deck/test/fixtures/controls.json, the core's answers) gives the
    /// same events here, and the cycle walks master → the being → a knob → back
    #[test]
    fn the_browsers_pad_script_gives_the_same_events() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/controls.json")).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(&text).unwrap();
        for (i, frame) in fixture["pad"].as_array().unwrap().iter().enumerate() {
            let num = |v: &serde_json::Value| v.as_f64().unwrap() as f32;
            frame["axes"].as_array().unwrap().iter().enumerate().for_each(|(index, v)| pad.input(Input::Axis { index, value: num(v) }));
            frame["buttons"].as_array().unwrap().iter().enumerate().for_each(|(index, v)| pad.input(Input::Button { index, pressed: v.as_bool().unwrap() }));
            let got: Vec<[f32; 4]> = pad.frame(&mut space, &mut ctl).iter().map(|e| e.code()).collect();
            let want: Vec<[f32; 4]> = frame["events"].as_array().unwrap().iter().map(|e| std::array::from_fn(|k| num(&e[k]))).collect();
            assert_eq!(got, want, "frame {i}");
            let sel = frame["selection"].as_str().unwrap();
            assert_eq!(space.selection_name().starts_with("knob:"), sel.starts_with("knob:"), "frame {i}: {sel}");
        }
    }

    /// a stick deflected past the press point is down and moves the being's pair from where it was; full deflection
    /// reaches the values the engine's STICK_GAIN gives; returning to rest lets it go; a small deflection does nothing
    #[test]
    fn a_stick_drives_the_being_from_where_it_was() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        let step = |pad: &mut Pad, space: &mut Space, ctl: &mut Knobset, input: Input| {
            pad.input(input);
            pad.frame(space, ctl)
        };
        step(&mut pad, &mut space, &mut ctl, Input::Button { index: NEXT, pressed: true });
        assert_eq!(space.selection_name(), "being");
        let button = |p: &str| space.being.buttons[pad.paths.iter().position(|q| q == p).unwrap()].clone();
        let (ratio, index) = (button("voice.ratio"), button("voice.index"));
        let (start_x, start_y) = (ratio.norm, index.norm);
        assert!(step(&mut pad, &mut space, &mut ctl, Input::Axis { index: 0, value: 0.02 }).is_empty(), "rest");
        step(&mut pad, &mut space, &mut ctl, Input::Axis { index: 0, value: 1.0 });
        assert!(pad.sticks.stick_is_down(0));
        let want = (start_x + STICK_GAIN).min(1.0);
        assert!((ctl.input(ratio.node, ratio.name, -9.0) - ratio.spec.value(want)).abs() < 1e-4, "x: the ratio");
        step(&mut pad, &mut space, &mut ctl, Input::Axis { index: 0, value: 0.0 });
        step(&mut pad, &mut space, &mut ctl, Input::Axis { index: 1, value: 1.0 });
        let want_y = (start_y - STICK_GAIN).max(0.0);
        assert!((ctl.input(index.node, index.name, -9.0) - index.spec.value(want_y)).abs() < 1e-4, "y (pulled down): the index");
        step(&mut pad, &mut space, &mut ctl, Input::Gone);
        assert!(!pad.sticks.stick_is_down(0), "the pad gone: up");
    }

    /// a pad that can't be opened, or none attached: the poll does nothing and the selection stays
    #[test]
    fn no_pad_no_input() {
        let (g, mut space, mut ctl) = cathedral();
        let mut pad = Pad::with_paths(&g, &space);
        assert!(pad.poll(&mut space, &mut ctl, false).is_empty(), "nothing for the menu");
        assert_eq!(space.selection_name(), "");
        assert!(pad.frame(&mut space, &mut ctl).is_empty());
    }
}
