//! A cast member moved on the stage by hand (the editor's scene view) and the grid's lines on the screen: the snap math is
//! snap.rs; here the camera and the cast give it its plane, its lines and its reach. The member slides along the floor
//! at its feet's height and snaps to the other members' feet (objects), the set's spots (outlines), the grid's zero
//! lines (axes) and its steps.
use super::Space;
use crate::snap::{self, Guide, Line, PerAxis, Settings, Target};
use glam::{Vec3, Vec4};

/// a member in the hand: which, and where its feet sit from where the pointer met the floor
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grab {
    pub member: usize,
    offset: Vec3,
    height: f32,
}

/// a line on the screen (view pixels): its ends and whether it is one of the grid's zero lines
pub type ScreenLine = ((f32, f32), (f32, f32), bool);

impl Space {
    /// where the stage's floor is: the first member's spot, else under the scene's being
    pub fn floor_point(&self) -> Vec3 {
        self.cast.first().map_or(self.being.anchor - Vec3::Y * self.lift, |c| c.feet - c.at)
    }

    /// where the pointer's ray at view pixel (`x`, `y`) meets the level `height` (in front of the eye)
    fn on_level(&self, x: f32, y: f32, height: f32) -> Option<Vec3> {
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        let t = (height - ray.origin.y) / ray.dir.y;
        (ray.dir.y.abs() > 1e-4 && t > 0.0).then(|| ray.origin + ray.dir * t)
    }

    /// Where a holder that plays nothing (an empty slot, `k` of `of`) would stand on the screen as a card `height`
    /// metres tall: on its set's spot, else in the cast's row (after its members; centred when there are none), moved by
    /// `at`. (left, top, width, height) in view pixels; None while its feet are behind the eye.
    pub fn standee_pixels(&self, spot: Option<&str>, at: [f32; 3], (k, of): (usize, usize), height: f32) -> Option<[f32; 4]> {
        let n = self.cast.len() as f32;
        let place = if self.cast.is_empty() { k as f32 - (of as f32 - 1.0) * 0.5 } else { n + k as f32 - (n - 1.0) * 0.5 };
        let row = self.being.anchor - Vec3::Y * self.lift + Vec3::X * place * super::CAST_SPACING;
        let feet = spot.and_then(|s| self.spots.iter().find(|(name, _)| name == s)).map_or(row, |(_, p)| *p) + Vec3::from(at);
        let (bx, by) = self.to_pixel(feet)?;
        let (_, ty) = self.to_pixel(feet + Vec3::Y * height)?;
        let h = (by - ty).max(1.0);
        let w = h * super::CARD_ASPECT;
        Some([bx - w * 0.5, ty, w, h])
    }

    /// a press at view pixel (`x`, `y`): the member under it in the hand, None = no member there
    pub fn grab_member(&self, x: f32, y: f32) -> Option<Grab> {
        let member = self.pick_at(x, y)?;
        let feet = self.cast[member].feet;
        let hit = self.on_level(x, y, feet.y)?;
        Some(Grab { member, offset: feet - hit, height: feet.y })
    }

    /// the lines a moved member snaps to: the other members' feet, the set's spots
    fn member_lines(&self, member: usize) -> PerAxis<Vec<Line>> {
        let others = self.cast.iter().enumerate().filter(|(i, _)| *i != member).map(|(_, c)| snap::point_lines(c.feet.to_array(), Target::Objects));
        let spots = self.spots.iter().map(|(_, p)| snap::point_lines(p.to_array(), Target::Outlines));
        snap::join(others.chain(spots))
    }

    /// The member in the hand to view pixel (`x`, `y`) along the floor, snapped as `s` says while `active` (lines within
    /// `px` view pixels catch it): its new offset from its spot and the lines it snapped to. None: the pointer meets no
    /// floor (above the horizon), the member stays.
    pub fn drag_member(&mut self, g: Grab, x: f32, y: f32, s: &Settings, active: bool, px: f32) -> Option<([f32; 3], Vec<Guide>)> {
        let feet = self.on_level(x, y, g.height)? + g.offset;
        let reach = snap::threshold_m(px, self.camera.eye().distance(feet), self.camera.fov_y, self.view.1);
        let (p, guides) = snap::snap_place(s, active, feet.to_array(), [true, false, true], self.member_lines(g.member), reach);
        let lift = self.lift;
        let c = self.cast.get_mut(g.member)?;
        let spot = c.feet - c.at;
        c.feet = Vec3::from(p);
        c.at = c.feet - spot;
        c.being.anchor = c.feet + Vec3::Y * lift;
        Some((c.at.to_array(), guides))
    }

    /// a world segment on the screen (view pixels), cut where it passes behind the eye
    fn segment_pixels(&self, a: Vec3, b: Vec3) -> Option<((f32, f32), (f32, f32))> {
        const NEAR: f32 = 0.05;
        let m = self.camera.view_proj(self.view.0 / self.view.1);
        let (mut ca, mut cb) = (m * a.extend(1.0), m * b.extend(1.0));
        if ca.w < NEAR && cb.w < NEAR {
            return None;
        }
        let cut = |p: Vec4, q: Vec4| p + (q - p) * ((NEAR - p.w) / (q.w - p.w));
        if ca.w < NEAR {
            ca = cut(ca, cb);
        } else if cb.w < NEAR {
            cb = cut(cb, ca);
        }
        let px = |c: Vec4| ((c.x / c.w + 1.0) * 0.5 * self.view.0, (1.0 - c.y / c.w) * 0.5 * self.view.1);
        Some((px(ca), px(cb)))
    }

    /// The grid as `s` lays it, around where the camera looks (`half` metres each way), on the screen; empty while off.
    pub fn grid_pixels(&self, s: &Settings, half: f32) -> Vec<ScreenLine> {
        let centre = self.camera.target.to_array();
        snap::grid_lines(s, centre, half).into_iter().filter_map(|l| self.segment_pixels(Vec3::from(l.a), Vec3::from(l.b)).map(|(a, b)| (a, b, l.zero))).collect()
    }

    /// the lines a drag snapped to, through the member's feet across the floor (`half` metres each way), on the screen
    pub fn guide_pixels(&self, member: usize, guides: &[Guide], half: f32) -> Vec<ScreenLine> {
        let Some(c) = self.cast.get(member) else { return Vec::new() };
        guides
            .iter()
            .filter_map(|g| {
                let along = if g.axis == 0 { Vec3::Z } else { Vec3::X };
                let mut at = c.feet;
                at[g.axis] = g.line.at;
                self.segment_pixels(at - along * half, at + along * half).map(|(a, b)| (a, b, false))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Grab;
    use crate::graph::Graph;
    use crate::scene::Scene;
    use crate::snap::{Guide, Line, Plane, Settings, Target};
    use crate::space::portrait::Portraits;
    use crate::space::values::Table;
    use crate::space::Space;
    use crate::tempo;
    use glam::Vec3;
    use std::path::Path;

    /// stage-duo: Cat Pope at `left`, Top Toy at `right` – both cards, no set
    fn duo() -> Space {
        let scene = Scene::parse(crate::scene::sandbox("stage-duo").unwrap()).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        let mut ctl = Table::new(&g);
        space.set_view(1280.0, 720.0);
        for k in 0..60 {
            space.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        space
    }

    /// the left member in the hand, held at its feet
    fn grab(space: &Space) -> Grab {
        let (x, y) = space.member_pixel("left").expect("drawn");
        let g = space.grab_member(x, y).expect("the member under the pointer");
        assert_eq!(g.member, 0);
        Grab { offset: Vec3::ZERO, ..g }
    }

    fn pixel(space: &Space, p: Vec3) -> (f32, f32) {
        space.to_pixel(p).expect("in front")
    }

    #[test]
    fn a_dragged_member_snaps_to_the_floor_grid_and_to_another_member() {
        let mut space = duo();
        let (a, b) = (space.cast[0].feet, space.cast[1].feet);
        let g = grab(&space);
        let snap = Settings { snap: true, step: 0.5, origin: space.floor_point().to_array(), ..Settings::default() };

        // off the grid: its feet go to the floor grid's nearest crossing, on the floor
        let to = pixel(&space, a + Vec3::new(0.37, 0.0, 0.21));
        let (at, guides) = space.drag_member(g, to.0, to.1, &snap, true, 8.0).expect("on the floor");
        let feet = space.cast[0].feet;
        for k in [0, 2] {
            let steps = (feet[k] - snap.origin[k]) / 0.5;
            assert!((steps - steps.round()).abs() < 1e-3, "axis {k} on the grid: {feet}");
        }
        assert!((feet.y - a.y).abs() < 1e-4, "it stays on the floor");
        assert!(guides.is_empty());
        assert!((Vec3::from(at) - (feet - a)).length() < 1e-4, "its offset from its spot");

        // a hair beside the other member's x: it lines up with it
        let near = pixel(&space, Vec3::new(b.x + 0.01, a.y, a.z + 0.73));
        let (_, guides) = space.drag_member(g, near.0, near.1, &snap, true, 8.0).unwrap();
        assert!((space.cast[0].feet.x - b.x).abs() < 1e-3, "lined up with the other member");
        assert!(matches!(guides.first(), Some(Guide { axis: 0, line: Line { kind: Target::Objects, .. } })), "{guides:?}");
        assert!(!space.guide_pixels(0, &guides, 3.0).is_empty(), "its guide is drawn");

        // snapping off: it goes where the pointer is
        let free = a + Vec3::new(0.37, 0.0, 0.21);
        let p = pixel(&space, free);
        let (_, guides) = space.drag_member(g, p.0, p.1, &snap, false, 8.0).unwrap();
        assert!(guides.is_empty());
        assert!((space.cast[0].feet - free).length() < 1e-2, "{} vs {free}", space.cast[0].feet);
    }

    #[test]
    fn the_grid_is_drawn_on_the_floor_or_a_wall_and_not_while_off() {
        let space = duo();
        let floor = Settings { origin: space.floor_point().to_array(), ..Settings::default() };
        let lines = space.grid_pixels(&floor, 4.0);
        assert!(lines.len() > 10, "{}", lines.len());
        assert_eq!(lines.iter().filter(|l| l.2).count(), 2, "the origin's two zero lines");
        assert!(!space.grid_pixels(&Settings { plane: Plane::Wall, ..floor.clone() }, 4.0).is_empty());
        assert!(space.grid_pixels(&Settings { grid: false, ..floor }, 4.0).is_empty());
    }

    #[test]
    fn an_empty_slot_stands_after_the_cast_as_tall_as_its_card() {
        let space = duo();
        let right = space.cast[1].feet;
        let r = space.standee_pixels(None, [0.0; 3], (0, 1), 1.7).expect("in front");
        let (rx, _) = pixel(&space, right);
        assert!(r[0] > rx, "beside the row's last member: {r:?} vs {rx}");
        let small = space.standee_pixels(None, [0.0; 3], (0, 1), 0.6).unwrap();
        assert!(small[3] < r[3] && (small[1] + small[3] - (r[1] + r[3])).abs() < 1.0, "same feet, shorter");
        let moved = space.standee_pixels(None, [-1.0, 0.0, 0.0], (0, 1), 1.7).unwrap();
        assert!(moved[0] < r[0], "its offset moves it");
        assert_eq!(space.standee_pixels(Some("spot_nowhere"), [0.0; 3], (0, 1), 1.7), Some(r), "a spot the set lacks: the row");
    }

    #[test]
    fn above_the_horizon_the_member_stays() {
        let mut space = duo();
        let g = grab(&space);
        let before = space.cast[0].feet;
        assert!(space.drag_member(g, 640.0, -50000.0, &Settings::default(), true, 8.0).is_none());
        assert_eq!(space.cast[0].feet, before);
    }
}

