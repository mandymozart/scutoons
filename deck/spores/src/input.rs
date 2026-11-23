//! What a person does to the widget, for every platform (winit natively, the DOM in the browser): the wheel zooms round
//! the pointer and a touchpad's two fingers pan, a grab (the middle button, or the left with space held) pans, a press on
//! the canvas starts a box, Tab (shift: back) moves the focus, Escape lets the selection go, F frames the selection (or
//! everything), ctrl+G groups it. What only the host can do to its documents comes back as a `Gesture`.
use crate::state::At;
use crate::{Hit, State};
use deck_kit::base::graph::{self, PILL};
use deck_kit::editor::controls::Rect;

/// a key the widget takes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Tab,
    Escape,
    /// F: frame
    Frame,
    /// ctrl+G: group the selection
    Group,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// a notch or a pinch zooms round `at`, a touchpad's two fingers pan
    Wheel { at: (f32, f32), delta: (f32, f32), fingers: bool, pinch: bool },
    /// a press that pans, at
    Grab { at: (f32, f32) },
    /// the pointer moved with a button down
    Drag { at: (f32, f32) },
    /// the button let go
    Release,
    /// space or shift held (true) or let go
    Space(bool),
    Shift(bool),
    /// a key, the pointer at
    Key { key: Key, at: (f32, f32) },
    /// a press on what the last paint returned, the pointer at
    Press { hit: Hit, at: (f32, f32) },
}

/// what the host does to its documents (the widget changed nothing for it)
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// fold or open a spore, or a group's pill (`group:<id>`)
    Fold(String),
    /// group these spores
    Group(Vec<String>),
    /// a note beside this spore (none: after the weave), to be typed
    Note(Option<String>),
    /// a group's label to be typed, its colour to change, it to fold – by its index in the canvas
    RenameGroup(usize),
    RecolourGroup(usize),
    FoldGroup(usize),
    /// a note to be typed, by its index in the canvas
    EditNote(usize),
    /// a port pressed: a thread starts or lifts there
    TakePin(At),
    /// a spore pressed at `from`: the host may carry it
    TakeSpore { id: String, from: (f32, f32) },
}

impl State {
    /// One event. True when the widget took it; the gesture the host carries out, if any.
    pub fn handle(&mut self, e: Event) -> (bool, Option<Gesture>) {
        match e {
            Event::Wheel { at: (x, y), delta: (dx, dy), fingers, pinch } => {
                if !self.over(x, y) {
                    return (false, None);
                }
                self.touched = true;
                match fingers && !pinch {
                    true => self.view.pan_by((dx * 40.0, dy * 40.0)),
                    false => {
                        let at = self.local(x, y);
                        self.view.zoom_at(at, dy)
                    }
                }
                (true, None)
            }
            Event::Grab { at: (x, y) } => {
                if !self.over(x, y) {
                    return (false, None);
                }
                self.grab = Some((x, y));
                self.touched = true;
                (true, None)
            }
            Event::Drag { at: (x, y) } => {
                if let Some((gx, gy)) = self.grab {
                    self.view.pan_by((x - gx, y - gy));
                    self.grab = Some((x, y));
                }
                self.box_move((x, y));
                (self.grab.is_some() || self.boxing.is_some(), None)
            }
            Event::Release => {
                let took = self.grab.is_some() || self.boxing.is_some();
                self.grab = None;
                self.box_end();
                (took, None)
            }
            Event::Space(down) => {
                self.space = down;
                (down, None)
            }
            Event::Shift(down) => {
                self.shift = down;
                (false, None)
            }
            Event::Key { key, at } => self.key(key, at),
            Event::Press { hit, at } => self.press(hit, at),
        }
    }

    fn key(&mut self, key: Key, (x, y): (f32, f32)) -> (bool, Option<Gesture>) {
        match key {
            Key::Tab if !self.order.is_empty() => {
                let len = self.order.len();
                let at = self.focus.as_ref().and_then(|f| self.order.iter().position(|&i| self.drawn.get(i).is_some_and(|(id, _)| id == f)));
                let next = match (at, self.shift) {
                    (None, false) => 0,
                    (None, true) => len - 1,
                    (Some(k), false) => (k + 1) % len,
                    (Some(k), true) => (k + len - 1) % len,
                };
                if let Some(id) = self.id(self.order[next]) {
                    self.focus_on(id);
                }
                (true, None)
            }
            Key::Escape if !self.selected.is_empty() => {
                self.selected.clear();
                self.focus = None;
                (true, None)
            }
            Key::Frame if self.over(x, y) => {
                self.fit();
                (true, None)
            }
            Key::Group if !self.selected.is_empty() && self.over(x, y) => self.press(Hit::Group, (x, y)),
            _ => (false, None),
        }
    }

    /// F or Fit: the selected spores framed, or everything when none is
    pub fn fit(&mut self) {
        let picked: Vec<Rect> = self.drawn.iter().filter(|(id, _)| self.selected.contains(id)).map(|(_, r)| *r).collect();
        match picked.is_empty() {
            true => self.frame(),
            false => self.frame_on(graph::bounds_of(picked)),
        }
        self.touched = true;
    }

    fn press(&mut self, hit: Hit, at: (f32, f32)) -> (bool, Option<Gesture>) {
        let gesture = match hit {
            Hit::Flow(d) => {
                self.direction = d;
                None
            }
            Hit::Fit => {
                self.fit();
                None
            }
            Hit::Canvas => {
                self.box_start(at);
                None
            }
            Hit::Node(i) => self.id(i).map(|id| {
                self.focus_on(id.clone());
                Gesture::TakeSpore { id, from: at }
            }),
            Hit::NodeAlso(i) => {
                if let Some(id) = self.id(i) {
                    match self.selected.iter().position(|s| *s == id) {
                        Some(k) => {
                            self.selected.remove(k);
                        }
                        None => {
                            self.selected.push(id.clone());
                            self.flash = Some((id.clone(), self.secs));
                            self.ask_tone(&id);
                            self.focus = Some(id);
                        }
                    }
                }
                None
            }
            Hit::Pin(i, out, k) => Some(Gesture::TakePin((i, out, k))),
            Hit::Fold(i) => self.id(i).map(Gesture::Fold),
            Hit::Group => Some(Gesture::Group(self.selected.iter().filter(|s| !s.starts_with(PILL)).cloned().collect())),
            Hit::Note => Some(Gesture::Note(self.focus.clone().or_else(|| self.selected.first().cloned()).filter(|s| !s.starts_with(PILL)))),
            Hit::GroupName(k) => Some(Gesture::RenameGroup(k)),
            Hit::GroupColour(k) => Some(Gesture::RecolourGroup(k)),
            Hit::GroupFold(k) => Some(Gesture::FoldGroup(k)),
            Hit::NoteText(k) => Some(Gesture::EditNote(k)),
        };
        (true, gesture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// three spores drawn in a row on a 400 × 300 canvas
    fn three() -> State {
        let r = |x: f32| Rect { x, y: 10.0, w: 40.0, h: 20.0 };
        State {
            canvas: Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 },
            drawn: vec![("a".into(), r(10.0)), ("b".into(), r(80.0)), ("c".into(), r(150.0))],
            order: vec![0, 1, 2],
            ..State::default()
        }
    }

    #[test]
    fn tab_moves_the_focus_round_and_shift_goes_back_escape_lets_go() {
        let mut s = three();
        let at = (5.0, 5.0);
        s.handle(Event::Key { key: Key::Tab, at });
        assert_eq!(s.focus.as_deref(), Some("a"));
        s.handle(Event::Key { key: Key::Tab, at });
        s.handle(Event::Key { key: Key::Tab, at });
        s.handle(Event::Key { key: Key::Tab, at });
        assert_eq!(s.focus.as_deref(), Some("a"), "round again");
        s.handle(Event::Shift(true));
        s.handle(Event::Key { key: Key::Tab, at });
        assert_eq!((s.focus.as_deref(), s.selected.clone()), (Some("c"), vec!["c".to_string()]));
        assert_eq!(s.handle(Event::Key { key: Key::Escape, at }), (true, None));
        assert!(s.selected.is_empty() && s.focus.is_none());
    }

    #[test]
    fn a_press_selects_shift_adds_and_what_the_host_does_comes_back() {
        let mut s = three();
        let at = (20.0, 20.0);
        assert_eq!(s.handle(Event::Press { hit: Hit::Node(1), at }).1, Some(Gesture::TakeSpore { id: "b".into(), from: at }));
        s.handle(Event::Press { hit: Hit::NodeAlso(2), at });
        assert_eq!(s.selected, ["b", "c"]);
        assert_eq!(s.handle(Event::Press { hit: Hit::Group, at }).1, Some(Gesture::Group(vec!["b".into(), "c".into()])));
        assert_eq!(s.handle(Event::Press { hit: Hit::Fold(0), at }).1, Some(Gesture::Fold("a".into())));
        assert_eq!(s.handle(Event::Wheel { at: (900.0, 900.0), delta: (0.0, 1.0), fingers: false, pinch: false }), (false, None), "off the canvas");
    }
}
