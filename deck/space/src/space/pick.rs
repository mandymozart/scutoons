//! pick.rs (#496): a cast member picked by clicking it on the stage, like a unit in a strategy game (owner 2026-11-12
//! [scenes]: *"the select might be best via clicking the character like in a strategy game"*).
//!
//! - **The hit test** (`pick_at`, pure): the pointer's ray against each member as the stage draws it – a 3D look's
//!   bounds (its body's sphere and its head), a card's rectangle (`Space::card`, the very quad `write_cards` draws) –
//!   the nearest along the ray wins.
//! - **The pick** lives here, in the core: a tap on a member picks it (its whole self selected – the sticks drive its main
//!   pairs), a tap on none of them (the empty floor, the sky) drops it. **The hand** follows the pick: the picked
//!   member's knobs are the only ones the pointer, the sticks and the page's selection reach (`hand` – `button`,
//!   `select_named`, `selection_name`, `hand_paths`); nothing picked, the hand is the space's own being (on a stage of
//!   several a bodiless one: the knobs reach no one).
//! - **The mark**: a ring of beads on the floor at the picked member's feet, turning slowly, drawn in the space's own pass
//!   (`write`) – it reads on a look and on a card alike. A picked card shows its knobs (its being's buttons, no body).
//! - **Keys** (`key`, the DOM's names – the native deck says the same): Tab picks the next member (Shift: the one
//!   before), Escape drops the pick; any other key is not the pick's.
//!
//! The decks only pass the click (native: the pointer's press and release; the browser's scene: `tap`) and draw what the
//! core writes. The editor's stage (#244) takes the same API: `pick_at`, `pick`, `pick_named`, `picked`, `pick_holder`.
use super::being::{kind, Being, Hit, Sphere};
use super::camera::Ray;
use super::{Card, Cast, Drag, Space};
use crate::cast::Form;
use glam::Vec3;

/// the mark's beads
pub const MARK_BEADS: usize = 24;
/// a bead's radius (m)
const BEAD: f32 = 0.03;
/// the ring's radius around a look's feet, in its sizes
const LOOK_RING: f32 = 0.9;
/// around a card's feet, in its half widths – at least MIN_RING (m)
const CARD_RING: f32 = 1.3;
const MIN_RING: f32 = 0.3;
/// a look's bounds: its body's sphere around its centre, in its sizes
const LOOK_BOUNDS: f32 = 1.0;
/// how fast the ring turns (rad / s)
const SPIN: f32 = 0.5;

/// the spheres a picked card's knobs write at most: each knob, its ring (hover's, the selection's or the lock's), its value
/// ring
pub fn knob_spheres(b: &Being) -> usize {
    b.buttons.len() * 3
}

/// Where `ray` meets the card's rectangle (its t), if it does – from the front or the back.
pub fn ray_card(ray: &Ray, c: &Card) -> Option<f32> {
    let n = c.right.cross(c.up);
    let facing = ray.dir.dot(n);
    if facing.abs() < 1e-6 {
        return None;
    }
    let t = (c.feet - ray.origin).dot(n) / facing;
    let p = ray.origin + ray.dir * t - c.feet;
    let across = p.dot(c.right) / c.right.length_squared().max(1e-9);
    let up = p.dot(c.up) / c.up.length_squared().max(1e-9);
    (t >= 0.0 && across.abs() <= 1.0 && (0.0..=1.0).contains(&up)).then_some(t)
}

impl Space {
    /// The being the hands drive: the picked member's, else the space's own.
    pub fn hand(&self) -> &Being {
        self.picked.map_or(&self.being, |i| &self.cast[i].being)
    }

    pub(super) fn hand_mut(&mut self) -> &mut Being {
        match self.picked {
            Some(i) => &mut self.cast[i].being,
            None => &mut self.being,
        }
    }

    /// the picked member stands as its card: no body – only its knobs are the hand's to reach
    fn hand_is_card(&self) -> bool {
        self.picked.is_some_and(|i| self.cast[i].form == Form::Card)
    }

    /// what of the hand `ray` reaches: a knob, or – a body's – one of its points
    pub(super) fn hand_hit(&self, ray: &Ray, reach: f32) -> Option<Hit> {
        let hit = self.hand().pick_reach(ray, reach);
        if self.hand_is_card() {
            hit.filter(|h| matches!(h, Hit::Button(_)))
        } else {
            hit
        }
    }

    /// the hand's head under `ray` (the whole being), a body's only
    pub(super) fn hand_head(&self, ray: &Ray) -> Option<Hit> {
        let h = self.hand();
        h.core_point.filter(|_| !self.hand_is_card() && ray.sphere(h.head.pos, h.head.radius).is_some()).map(Hit::Point)
    }

    /// the picked member (an index into `cast`)
    pub fn picked(&self) -> Option<usize> {
        self.picked
    }

    /// the picked member's holder (`left`, a zone's id)
    pub fn pick_holder(&self) -> Option<&str> {
        self.picked.map(|i| self.cast[i].holder.as_str())
    }

    /// The member under window pixel (`x`, `y`) as the stage draws it this frame – the nearest along the pointer's ray –
    /// or None. Pure: nothing is picked.
    pub fn pick_at(&self, x: f32, y: f32) -> Option<usize> {
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        let eye = self.camera.eye();
        let hits = self.cast.iter().enumerate().filter_map(|(i, c)| self.member_hit(c, &ray, eye).map(|t| (t, i)));
        hits.min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, i)| i)
    }

    fn member_hit(&self, c: &Cast, ray: &Ray, eye: Vec3) -> Option<f32> {
        match c.form {
            Form::Card => ray_card(ray, &self.card(c, eye)),
            Form::Look => {
                let b = &c.being;
                let body = ray.sphere(b.center, b.look.size * LOOK_BOUNDS);
                body.into_iter().chain(ray.sphere(b.head.pos, b.head.radius)).min_by(f32::total_cmp)
            }
        }
    }

    /// Pick member `member` (None drops the pick): the old hand lets go of what it held, the new one is selected whole –
    /// the sticks drive its main pairs; with none picked the selection is the master section.
    pub fn pick(&mut self, member: Option<usize>) {
        let member = member.filter(|&i| i < self.cast.len());
        if member == self.picked {
            return;
        }
        let old = self.hand_mut();
        (old.hover, old.selected, old.dragging) = (None, None, false);
        (self.drag, self.last_press, self.pressed) = (Drag::None, None, None);
        self.picked = member;
        self.select(member.map(|_| Hit::Point(0)));
    }

    /// pick the member of `holder` ("" drops the pick); false = no member stands there
    pub fn pick_named(&mut self, holder: &str) -> bool {
        let member = self.cast.iter().position(|c| c.holder == holder);
        self.pick(member);
        member.is_some() || holder.is_empty()
    }

    /// a tap on the stage that met none of the hand's parts: the member under it is picked (tapped again: selected
    /// whole), none drops the pick
    pub(super) fn tap_stage(&mut self, x: f32, y: f32) {
        match self.pick_at(x, y) {
            Some(m) if Some(m) == self.picked => self.select(Some(Hit::Point(0))),
            m => self.pick(m),
        }
    }

    /// A whole tap at window pixel (`x`, `y`) – a host that passes only taps (the browser's scene): a press and its
    /// release there, as the pointer's (a knob of the hand, else the pick). false = the stage has no cast to pick from.
    pub fn tap(&mut self, x: f32, y: f32) -> bool {
        if self.cast.is_empty() {
            return false;
        }
        self.pointer = Some((x, y));
        self.press_reach(1.0);
        self.let_go();
        self.pointer_left();
        true
    }

    /// A key by the DOM's name (`KeyboardEvent.key`; `back` = Shift held): Tab picks the next member (the one before),
    /// Escape drops the pick. true = it was the pick's.
    pub fn key(&mut self, key: &str, back: bool) -> bool {
        let n = self.cast.len();
        match (key, self.picked) {
            ("Tab", _) if n > 0 => {
                let next = match self.picked {
                    None if back => n - 1,
                    None => 0,
                    Some(i) if back => (i + n - 1) % n,
                    Some(i) => (i + 1) % n,
                };
                self.pick(Some(next));
                true
            }
            ("Escape", Some(_)) => {
                self.pick(None);
                true
            }
            _ => false,
        }
    }

    /// where `holder`'s member is on the screen (window pixels): the middle of its card, its body's centre (a check
    /// run's click, a scenario's)
    pub fn member_pixel(&self, holder: &str) -> Option<(f32, f32)> {
        let c = self.cast.iter().find(|c| c.holder == holder)?;
        let at = match c.form {
            Form::Card => {
                let card = self.card(c, self.camera.eye());
                card.feet + card.up * 0.5
            }
            Form::Look => c.being.center,
        };
        self.to_pixel(at)
    }

    /// the sphere a member stands in as the stage draws it: its card's, its body's
    pub(super) fn member_sphere(&self, c: &Cast) -> (Vec3, f32) {
        match c.form {
            Form::Card => {
                let card = self.card(c, self.camera.eye());
                (card.feet + card.up * 0.5, (card.up.length() * 0.5).max(card.right.length()))
            }
            Form::Look => (c.being.center, c.being.look.size * LOOK_BOUNDS),
        }
    }

    /// the hand's knobs as paths (`node.port`, `node.shape`), in its buttons' order – what the sticks are set over
    pub fn hand_paths(&self) -> Vec<String> {
        self.hand().buttons.iter().map(|b| format!("{}.{}", self.graph_id(b.node), b.name)).collect()
    }

    /// the mark's ring this frame: its centre (the picked member's feet) and radius (m) – a look's by its size, a card's
    /// by its width; None = nothing picked
    pub fn mark(&self) -> Option<(Vec3, f32)> {
        let c = &self.cast[self.picked?];
        let r = match c.form {
            Form::Look => c.being.look.size * LOOK_RING,
            Form::Card => (self.card(c, self.camera.eye()).right.length() * CARD_RING).max(MIN_RING),
        };
        Some((c.feet, r))
    }

    /// The mark: a ring of beads on the floor round the picked member's feet, turning with the show's time – solid,
    /// white, lit as a hand on a knob is.
    pub(super) fn write_mark(&self, spheres: &mut Vec<Sphere>) {
        let Some((feet, r)) = self.mark() else { return };
        for k in 0..MARK_BEADS {
            let a = k as f32 / MARK_BEADS as f32 * std::f32::consts::TAU + self.clock * SPIN;
            let pos = feet + Vec3::new(a.cos() * r, BEAD, a.sin() * r);
            spheres.push(Sphere { pos: pos.into(), radius: BEAD, color: [1.0, 1.0, 1.0, 1.0], extra: [kind::NUCLEUS, 1.0, 1.5, 0.0] });
        }
    }

    /// the HUD's line while nothing is hovered: who is picked (`PICKED LEFT CATPOPE`)
    pub(super) fn pick_line(&self) -> Option<String> {
        let c = &self.cast[self.picked?];
        Some(format!("PICKED {} {}", c.holder, c.who.id()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::Scene;
    use crate::space::portrait::Portraits;
    use crate::space::values::{Knobs, Table};
    use crate::tempo;
    use std::path::Path;

    /// stage-duo: Cat Pope (her core rig's knobs) at `left`, Top Toy (no knobs) at `right` – both cards, no set
    fn duo() -> (Graph, Space, Table) {
        let scene = Scene::parse(crate::scene::sandbox("stage-duo").unwrap()).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        let mut ctl = Table::new(&g);
        space.set_view(1280.0, 720.0);
        for k in 0..180 {
            space.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        (g, space, ctl)
    }

    fn click(space: &mut Space, ctl: &mut Table, (x, y): (f32, f32)) {
        space.pointer_moved(x, y, ctl);
        space.press();
        space.release();
    }

    #[test]
    fn a_card_is_hit_inside_its_rectangle_from_either_side() {
        let c = Card { feet: Vec3::ZERO, right: Vec3::X * 0.4, up: Vec3::Y * 1.7, picture: None, glow: 0.0, hue: 0.0, pulse: 0.0 };
        let at = |x: f32, y: f32, z: f32| Ray { origin: Vec3::new(x, y, z), dir: Vec3::new(0.0, 0.0, -z.signum()) };
        assert_eq!(ray_card(&at(0.0, 1.0, 5.0), &c), Some(5.0));
        assert_eq!(ray_card(&at(0.3, 0.1, -2.0), &c), Some(2.0), "from behind");
        assert_eq!(ray_card(&at(0.5, 1.0, 5.0), &c), None, "beside it");
        assert_eq!(ray_card(&at(0.0, 1.8, 5.0), &c), None, "over its head");
        assert_eq!(ray_card(&Ray { origin: Vec3::new(0.0, 1.0, 5.0), dir: Vec3::Z }, &c), None, "behind the eye");
    }

    /// #496 (owner 2026-11-12: "clicking the character like in a strategy game"): a click on a card picks its member, the
    /// mark rings its feet, its knobs are the hand's – only its own; a click on the empty floor drops the pick
    #[test]
    fn a_click_picks_the_member_under_it_and_the_floor_drops_it() {
        let (g, mut space, mut ctl) = duo();
        assert_eq!(space.hand().buttons.len(), 0, "nothing picked: the knobs reach no one");
        let left = space.member_pixel("left").unwrap();
        let right = space.member_pixel("right").unwrap();
        assert!(left.0 < right.0, "left stands left: {left:?} {right:?}");
        assert_eq!(space.pick_at(left.0, left.1), Some(0));
        click(&mut space, &mut ctl, left);
        assert_eq!(space.pick_holder(), Some("left"));
        assert_eq!(space.selection_name(), "being", "picked whole: the sticks drive its pairs");
        assert!(space.hand_paths().iter().all(|p| p.starts_with("left_")) && !space.hand_paths().is_empty(), "{:?}", space.hand_paths());
        // the mark and the card's knobs are drawn (within the buffers' capacity)
        let (cap, tcap) = space.capacity();
        let (mut spheres, mut tubes) = (Vec::with_capacity(cap), Vec::with_capacity(tcap));
        let solid = space.write(&mut spheres, &mut tubes);
        assert!(spheres.len() <= cap && solid >= MARK_BEADS + space.hand().buttons.len(), "{} of {cap}, {solid} solid", spheres.len());
        // its knob turns its own node; the other member's knob is no button of the hand
        assert!(space.button(&g, "left_voice_voice.gain").is_some());
        click(&mut space, &mut ctl, right);
        assert_eq!(space.pick_holder(), Some("right"));
        assert_eq!(space.button(&g, "left_voice_voice.gain"), None, "Cat Pope's knob is not Top Toy's hand's");
        // the empty floor: dropped
        click(&mut space, &mut ctl, (40.0, 700.0));
        assert_eq!((space.picked(), space.selection_name().as_str()), (None, ""));
    }

    /// #496: dragging the picked card's knob moves its value, and nobody else's
    #[test]
    fn the_picked_member_s_knob_moves_only_it() {
        let (g, mut space, mut ctl) = duo();
        assert!(space.pick_named("left"));
        for k in 0..30 {
            space.update(1.0 / 60.0, 3.0 + k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        let i = space.button(&g, "left_voice_voice.gain").unwrap();
        let before: Vec<f32> = (0..g.nodes.len()).map(|n| ctl.input(n, "gain", -1.0)).collect();
        let at = space.to_pixel(space.hand().buttons[i].pos).unwrap();
        space.pointer_moved(at.0, at.1, &mut ctl);
        space.press();
        let (_, axis) = space.drag_axis(i);
        space.pointer_moved(at.0 + axis.0 * 0.3, at.1 + axis.1 * 0.3, &mut ctl);
        space.release();
        let after: Vec<f32> = (0..g.nodes.len()).map(|n| ctl.input(n, "gain", -1.0)).collect();
        let moved: Vec<&str> = (0..g.nodes.len()).filter(|&n| before[n] != after[n]).map(|n| g.nodes[n].id.as_str()).collect();
        assert_eq!(moved, ["left_voice_voice"]);
        assert_eq!(space.pick_holder(), Some("left"), "a drag is no tap: the pick stays");
    }

    #[test]
    fn tab_cycles_the_cast_and_escape_drops_the_pick() {
        let (_, mut space, _) = duo();
        assert!(!space.key("Escape", false), "nothing picked: Escape is not the pick's (the menu's)");
        assert!(space.key("Tab", false));
        assert_eq!(space.pick_holder(), Some("left"));
        space.key("Tab", false);
        assert_eq!(space.pick_holder(), Some("right"));
        space.key("Tab", false);
        assert_eq!(space.pick_holder(), Some("left"), "round again");
        space.key("Tab", true);
        assert_eq!(space.pick_holder(), Some("right"), "Shift: back");
        assert!(space.key("Escape", false));
        assert_eq!(space.picked(), None);
        assert!(!space.key("x", false));
    }

    /// stage-cards: Cat Pope as her 3D look (the set's 3D on its way – no loader here), the incense burner as its card –
    /// a click on her body picks her, on the burner's card the burner; her knobs are her body's, the burner's drawn alone
    #[test]
    fn a_look_and_a_card_are_picked_alike() {
        let scene = Scene::parse(crate::scene::sandbox("stage-cards").unwrap()).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        // the set stays on its way (no loader thread to fail it): 3D – she is her look
        (space.set_rx, space.set_failed) = (None, false);
        space.place_cast();
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Look, Form::Card]);
        let mut ctl = Table::new(&g);
        space.set_view(1280.0, 720.0);
        for k in 0..180 {
            space.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        let (pope, burner) = (space.member_pixel("pope").unwrap(), space.member_pixel("burner").unwrap());
        click(&mut space, &mut ctl, pope);
        assert_eq!(space.pick_holder(), Some("pope"));
        assert!(space.button(&g, "pope_voice_voice.ratio").is_some() && space.button(&g, "burner_air_hiss.gain").is_none());
        click(&mut space, &mut ctl, burner);
        assert_eq!(space.pick_holder(), Some("burner"), "{pope:?} {burner:?}");
        assert!(space.button(&g, "burner_air_hiss.gain").is_some() && space.button(&g, "pope_voice_voice.ratio").is_none());
        let (cap, tcap) = space.capacity();
        let (mut spheres, mut tubes) = (Vec::with_capacity(cap), Vec::with_capacity(tcap));
        space.write(&mut spheres, &mut tubes);
        assert!(spheres.len() <= cap, "{} of {cap}", spheres.len());
    }

    fn settle(space: &mut Space, ctl: &mut Table) {
        for k in 0..240 {
            space.update(1.0 / 60.0, 5.0 + k as f32 / 60.0, ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
    }

    /// an editor's free view, panned away, snaps back to the scene's camera – the view a deck opens with – and a pan leaves it
    #[test]
    fn the_editor_s_view_snaps_to_the_scene_camera_and_a_pan_leaves_it() {
        let (_, mut space, mut ctl) = duo();
        let deck = space.camera;
        space.camera.free = true;
        space.camera.drag(200.0, 80.0);
        space.camera.pan(300.0, -120.0, 720.0);
        settle(&mut space, &mut ctl);
        assert!(space.camera.eye().distance(deck.eye()) > 0.5, "moved off");
        space.look_through_scene();
        settle(&mut space, &mut ctl);
        assert!(space.camera.arrived());
        let primary = space.scene_camera();
        let mut want = deck;
        (want.target, want.yaw, want.pitch, want.distance) = (primary.target, primary.yaw, primary.pitch, primary.distance);
        assert!(space.camera.eye().distance(want.eye()) < 0.01, "{:?} {:?}", space.camera.eye(), want.eye());
        assert!(space.camera.target.distance(primary.target) < 0.01);
        assert!(primary.yaw.abs() < 1e-6, "head-on");
        space.camera.pan(40.0, 0.0, 720.0);
        assert_eq!(space.camera.through, None);
    }

    /// F frames the whole cast; a member framed alone comes nearer, at its middle
    #[test]
    fn frame_all_holds_the_cast_and_a_member_is_framed_alone() {
        let (_, mut space, mut ctl) = duo();
        space.camera.free = true;
        space.camera.zoom(-30.0);
        space.frame_all();
        settle(&mut space, &mut ctl);
        let all = space.camera.distance;
        let spheres: Vec<_> = space.cast.iter().map(|c| space.member_sphere(c)).collect();
        assert!(space.camera.fits(1280.0 / 720.0, spheres.iter().copied(), 0.0), "every member in view");
        space.look_through_scene();
        space.frame_member(1);
        assert_eq!(space.camera.through, None, "framing leaves the camera");
        settle(&mut space, &mut ctl);
        assert!(space.camera.distance < all, "{} {all}", space.camera.distance);
        let (middle, _) = space.member_sphere(&space.cast[1]);
        assert!(space.camera.target.distance(middle) < 0.05);
    }

    /// a host that passes only taps (the browser's scene): the same pick
    #[test]
    fn a_tap_picks_as_a_click_does() {
        let (_, mut space, _) = duo();
        let right = space.member_pixel("right").unwrap();
        assert!(space.tap(right.0, right.1));
        assert_eq!(space.pick_holder(), Some("right"));
        assert!(space.tap(5.0, 5.0));
        assert_eq!(space.picked(), None);
    }
}
