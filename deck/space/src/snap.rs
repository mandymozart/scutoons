//! The stage's grid and snapping – OSCT's grid & snap (its editor's format surface) taken over for the whole stage.
//! Metres in the stage's frame: x across, y up, z deep. The grid lies on the floor (x, z) or on a wall (x, y) through its
//! origin; a move snaps per axis to the nearest line in reach – an **axis** (the origin's zero lines), another **object**
//! (a member's feet), an **outline** (the set's spots) – else to the grid's steps (the move step when the grid is no
//! target). Rotate snaps to whole steps of degrees, scale to whole steps. Pure: the editor draws the grid and drags
//! through it (space/place.rs); a deck never draws it.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plane {
    #[default]
    Floor,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Grid,
    Axes,
    Objects,
    Outlines,
}

pub const TARGETS: [Target; 4] = [Target::Grid, Target::Axes, Target::Objects, Target::Outlines];

/// a line one axis can snap to: its place on that axis (m) and what it belongs to (never `Grid`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub at: f32,
    pub kind: Target,
}

/// steps along an axis: every `step` from `zero`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Increment {
    pub zero: f32,
    pub step: f32,
}

pub type PerAxis<T> = [T; 3];

/// how near a line must be drawn to catch an anchor (logical pixels); turned into metres by the camera's distance
pub const SNAP_PX: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Grid,
    Move,
    Rotate,
    Scale,
}

impl Precision {
    /// its bounds: the grid's and the move's step (m), the rotate step (degrees), the scale step
    pub fn limits(self) -> (f32, f32) {
        match self {
            Precision::Grid => (0.05, 10.0),
            Precision::Move => (0.01, 10.0),
            Precision::Rotate => (1.0, 90.0),
            Precision::Scale => (0.01, 1.0),
        }
    }

    pub fn clamp(self, value: f32) -> f32 {
        let (min, max) = self.limits();
        if value.is_finite() {
            value.clamp(min, max)
        } else {
            min
        }
    }
}

/// what an editor's view keeps of grid & snap (the editor's cache, per view)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub grid: bool,
    pub plane: Plane,
    /// the grid's step (m)
    pub step: f32,
    /// where the grid's zero lines cross (m)
    pub origin: [f32; 3],
    pub snap: bool,
    /// what a move snaps to, in `TARGETS`' order
    pub to: Vec<Target>,
    /// a move's step while the grid is no target (m)
    pub move_step: f32,
    /// degrees
    pub rotate: f32,
    pub scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { grid: true, plane: Plane::Floor, step: 0.5, origin: [0.0; 3], snap: false, to: TARGETS.to_vec(), move_step: 0.1, rotate: 15.0, scale: 0.1 }
    }
}

impl Settings {
    /// snapping right now: the setting, turned around while Shift or Ctrl is held
    pub fn active(&self, modifier: bool) -> bool {
        self.snap != modifier
    }

    /// `target` switched on or off, the targets kept in their fixed order
    pub fn toggle(&mut self, target: Target) {
        let on = !self.to.contains(&target);
        self.to = TARGETS.into_iter().filter(|t| if *t == target { on } else { self.to.contains(t) }).collect();
    }

    /// the grid's two directions: x and z on the floor, x and y on a wall
    pub fn plane_axes(&self) -> [usize; 2] {
        match self.plane {
            Plane::Floor => [0, 2],
            Plane::Wall => [0, 1],
        }
    }

    /// the steps of a move per axis: the grid's in its plane when the grid is a target, else the move step
    pub fn increments(&self) -> PerAxis<Increment> {
        let grid = self.to.contains(&Target::Grid);
        let plane = self.plane_axes();
        std::array::from_fn(|k| Increment { zero: self.origin[k], step: if grid && plane.contains(&k) { self.step } else { self.move_step } })
    }

    /// the origin's zero lines, as axes
    pub fn axis_lines(&self) -> PerAxis<Vec<Line>> {
        std::array::from_fn(|k| vec![Line { at: self.origin[k], kind: Target::Axes }])
    }

    /// only the lines of the chosen targets
    pub fn chosen(&self, lines: PerAxis<Vec<Line>>) -> PerAxis<Vec<Line>> {
        lines.map(|l| l.into_iter().filter(|line| self.to.contains(&line.kind)).collect())
    }
}

fn tidy(v: f32) -> f32 {
    (v * 1e4).round() / 1e4
}

/// `value` at the nearest of the steps of `step` from `zero` (a step of 0 or less leaves it)
pub fn snap_step(value: f32, step: f32, zero: f32) -> f32 {
    if step > 0.0 && value.is_finite() {
        tidy(zero + ((value - zero) / step).round() * step)
    } else {
        value
    }
}

/// an angle (degrees) at whole steps of `step`
pub fn snap_angle(degrees: f32, step: f32) -> f32 {
    snap_step(degrees, step, 0.0)
}

/// a scale factor at whole steps of `step`, its sign kept, never below one step
pub fn snap_scale(value: f32, step: f32) -> f32 {
    if !(step > 0.0) || !value.is_finite() {
        return value;
    }
    value.signum() * tidy(step).max(snap_step(value.abs(), step, 0.0))
}

/// the snap threshold in metres: `px` on the screen at `distance` from a camera with a vertical field of view of
/// `fov_y` (radians) over `viewport_px`
pub fn threshold_m(px: f32, distance: f32, fov_y: f32, viewport_px: f32) -> f32 {
    if viewport_px > 0.0 && distance > 0.0 {
        px * 2.0 * distance * (fov_y * 0.5).tan() / viewport_px
    } else {
        0.0
    }
}

/// one axis's result: the value and what it snapped to – a line, the steps, or nothing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisSnap {
    pub value: f32,
    pub line: Option<Line>,
    pub stepped: bool,
}

/// One axis of a move. `value` is the pivot, `offsets` the other anchors relative to it (a box's edges and centre); the
/// nearest line within `threshold` of any anchor wins and the value moves so that anchor lies on it. With none near,
/// it rounds to `increment` (None: it stays).
pub fn snap_axis(value: f32, offsets: &[f32], lines: &[Line], threshold: f32, increment: Option<Increment>) -> AxisSnap {
    let mut best: Option<(f32, f32, Line)> = None;
    let pivot = [0.0];
    let anchors = pivot.iter().chain(offsets.iter().filter(|o| **o != 0.0));
    for offset in anchors {
        for line in lines {
            let distance = (value + offset - line.at).abs();
            if distance <= threshold && best.is_none_or(|b| distance < b.0 - 1e-6) {
                best = Some((distance, tidy(line.at - offset), *line));
            }
        }
    }
    match (best, increment) {
        (Some((_, v, line)), _) => AxisSnap { value: v, line: Some(line), stepped: false },
        (None, Some(i)) if i.step > 0.0 => AxisSnap { value: snap_step(value, i.step, i.zero), line: None, stepped: true },
        _ => AxisSnap { value, line: None, stepped: false },
    }
}

/// which stage axes a constrained drag may change: those lying in the constraint – a line (one direction) or a plane
/// (two), each a unit vector; three directions (or none) leave every axis free. An axis only nearly in it (a turned
/// node) stays put: snapping it would pull the thing off its handle's line.
pub fn free_axes(directions: &[[f32; 3]], tolerance: f32) -> PerAxis<bool> {
    match directions {
        [d] => d.map(|c| c.abs() >= 1.0 - tolerance),
        [a, b] => {
            let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let length = n.iter().map(|c| c * c).sum::<f32>().sqrt().max(f32::MIN_POSITIVE);
            n.map(|c| (c / length).abs() <= tolerance)
        }
        _ => [true; 3],
    }
}

/// a line something snapped to, drawn while the drag lasts
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub axis: usize,
    pub line: Line,
}

/// a move to snap: the pivot, the other anchors per axis, the lines per axis, which axes may change
pub struct Move<'a> {
    pub point: [f32; 3],
    pub offsets: PerAxis<&'a [f32]>,
    pub lines: &'a PerAxis<Vec<Line>>,
    pub free: PerAxis<bool>,
    pub threshold: f32,
    pub increments: PerAxis<Option<Increment>>,
}

/// a move snapped on every free axis; the guides name the lines it snapped to
pub fn snap_move(m: &Move) -> ([f32; 3], Vec<Guide>) {
    let mut guides = Vec::new();
    let point = std::array::from_fn(|k| {
        if !m.free[k] {
            return m.point[k];
        }
        let r = snap_axis(m.point[k], m.offsets[k], &m.lines[k], m.threshold, m.increments[k]);
        if let Some(line) = r.line {
            guides.push(Guide { axis: k, line });
        }
        r.value
    });
    (point, guides)
}

/// a box's lines per axis: both edges and the centre
pub fn box_lines(min: [f32; 3], max: [f32; 3], kind: Target) -> PerAxis<Vec<Line>> {
    std::array::from_fn(|k| [min[k], (min[k] + max[k]) * 0.5, max[k]].map(|at| Line { at: tidy(at), kind }).to_vec())
}

/// a point's lines per axis (an object's pivot, a spot)
pub fn point_lines(p: [f32; 3], kind: Target) -> PerAxis<Vec<Line>> {
    std::array::from_fn(|k| vec![Line { at: tidy(p[k]), kind }])
}

/// lines of several sources joined per axis
pub fn join(sources: impl IntoIterator<Item = PerAxis<Vec<Line>>>) -> PerAxis<Vec<Line>> {
    let mut out: PerAxis<Vec<Line>> = Default::default();
    for s in sources {
        for (k, l) in s.into_iter().enumerate() {
            out[k].extend(l);
        }
    }
    out
}

/// A dragged thing's place snapped as the settings say: on the free axes, to the chosen lines within `threshold`, else
/// to the steps. Snapping off (`active` false): the place as it is.
pub fn snap_place(s: &Settings, active: bool, point: [f32; 3], free: PerAxis<bool>, lines: PerAxis<Vec<Line>>, threshold: f32) -> ([f32; 3], Vec<Guide>) {
    if !active {
        return (point, Vec::new());
    }
    let lines = s.chosen(join([s.axis_lines(), lines]));
    let increments = s.increments().map(Some);
    snap_move(&Move { point, offsets: [&[], &[], &[]], lines: &lines, free, threshold, increments })
}

/// one line of the grid to draw: its two ends, and whether it is one of the origin's zero lines
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLine {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub zero: bool,
}

/// at most this many lines each way: a finer grid shows every n-th line
pub const MAX_LINES: usize = 81;

/// The grid's lines around `centre` (reaching `half` metres each way along its plane), on its plane through the origin;
/// empty while the grid is off.
pub fn grid_lines(s: &Settings, centre: [f32; 3], half: f32) -> Vec<GridLine> {
    if !s.grid || !(s.step > 0.0) || !(half > 0.0) {
        return Vec::new();
    }
    let [u, v] = s.plane_axes();
    let normal = 3 - u - v;
    let mut step = s.step;
    while (2.0 * half / step) as usize + 1 > MAX_LINES {
        step *= 2.0;
    }
    let range = |k: usize| {
        let lo = snap_step(centre[k] - half, step, s.origin[k]);
        let n = ((2.0 * half) / step).round() as i32;
        (0..=n).map(move |i| tidy(lo + i as f32 * step))
    };
    let span = |k: usize| (snap_step(centre[k] - half, step, s.origin[k]), snap_step(centre[k] + half, step, s.origin[k]));
    let mut out = Vec::new();
    for (along, across) in [(u, v), (v, u)] {
        let (from, to) = span(along);
        for at in range(across) {
            let mut a = [0.0; 3];
            a[normal] = s.origin[normal];
            a[across] = at;
            let mut b = a;
            (a[along], b[along]) = (from, to);
            out.push(GridLine { a, b, zero: (at - s.origin[across]).abs() < 1e-4 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<Line> {
        vec![Line { at: 0.0, kind: Target::Axes }, Line { at: 6.0, kind: Target::Objects }, Line { at: 11.5, kind: Target::Outlines }]
    }

    #[test]
    fn on_off_and_the_modifier() {
        let mut s = Settings::default();
        s.snap = true;
        assert!(s.active(false));
        assert!(!s.active(true));
        s.snap = false;
        assert!(!s.active(false));
        assert!(s.active(true));
    }

    #[test]
    fn the_precision_stays_in_bounds() {
        assert_eq!(Precision::Move.clamp(0.0), 0.01);
        assert_eq!(Precision::Rotate.clamp(400.0), 90.0);
        assert_eq!(Precision::Scale.clamp(0.25), 0.25);
        assert_eq!(Precision::Grid.clamp(f32::NAN), 0.05);
    }

    #[test]
    fn a_target_switches_on_and_off_in_its_fixed_order() {
        let mut s = Settings { to: vec![Target::Grid, Target::Objects], ..Settings::default() };
        s.toggle(Target::Axes);
        assert_eq!(s.to, [Target::Grid, Target::Axes, Target::Objects]);
        s.toggle(Target::Grid);
        assert_eq!(s.to, [Target::Axes, Target::Objects]);
    }

    #[test]
    fn the_grid_rounds_to_the_nearest_step_from_its_zero() {
        assert_eq!(snap_step(1.4, 1.0, 0.0), 1.0);
        assert_eq!(snap_step(1.6, 1.0, 0.0), 2.0);
        assert_eq!(snap_step(-1.4, 1.0, 0.0), -1.0);
        assert_eq!(snap_step(1.6, 1.0, 0.3), 1.3);
        assert_eq!(snap_step(0.3 + 0.6, 0.3, 0.0), 0.9);
        assert_eq!(snap_step(1.6, 0.0, 0.0), 1.6);
    }

    #[test]
    fn the_floor_grid_steps_x_and_z_the_wall_grid_x_and_y() {
        let floor = Settings { step: 0.5, move_step: 0.1, origin: [1.0, 0.0, 2.0], ..Settings::default() };
        assert_eq!(floor.increments().map(|i| i.step), [0.5, 0.1, 0.5]);
        assert_eq!(floor.increments().map(|i| i.zero), [1.0, 0.0, 2.0]);
        let wall = Settings { plane: Plane::Wall, ..floor.clone() };
        assert_eq!(wall.increments().map(|i| i.step), [0.5, 0.5, 0.1]);
        // the grid no target: the move step everywhere
        let free = Settings { to: vec![Target::Axes], ..floor };
        assert_eq!(free.increments().map(|i| i.step), [0.1; 3]);
    }

    #[test]
    fn rotation_and_scale_in_whole_steps() {
        assert_eq!(snap_angle(22.0, 15.0), 15.0);
        assert_eq!(snap_angle(23.0, 15.0), 30.0);
        assert_eq!(snap_angle(-50.0, 15.0), -45.0);
        assert_eq!(snap_angle(44.0, 45.0), 45.0);
        assert_eq!(snap_angle(7.0, 15.0), 0.0);
        assert_eq!(snap_scale(1.04, 0.1), 1.0);
        assert_eq!(snap_scale(1.06, 0.1), 1.1);
        assert_eq!(snap_scale(0.02, 0.1), 0.1);
        assert_eq!(snap_scale(-0.74, 0.25), -0.75);
        assert_eq!(snap_scale(1.3, 0.0), 1.3);
    }

    #[test]
    fn the_pivot_goes_onto_a_line_near_it() {
        let r = snap_axis(5.8, &[], &lines(), 0.3, None);
        assert_eq!(r, AxisSnap { value: 6.0, line: Some(Line { at: 6.0, kind: Target::Objects }), stepped: false });
    }

    #[test]
    fn any_anchor_goes_onto_its_nearest_line() {
        // a box 2 m wide around the pivot: its right edge (+1) is 0.1 from the outline at 11.5
        assert_eq!(snap_axis(10.4, &[-1.0, 0.0, 1.0], &lines(), 0.3, None).value, 10.5);
        // its left edge (−1) nearest to the object at 6
        assert_eq!(snap_axis(7.1, &[-1.0, 0.0, 1.0], &lines(), 0.3, None).value, 7.0);
    }

    #[test]
    fn the_nearest_of_several_lines_in_reach_wins() {
        let l = [Line { at: 0.0, kind: Target::Axes }, Line { at: 0.3, kind: Target::Outlines }];
        assert_eq!(snap_axis(0.2, &[], &l, 0.4, None).line.map(|l| l.kind), Some(Target::Outlines));
    }

    #[test]
    fn with_no_line_near_it_steps_and_without_steps_it_stays() {
        let step = Some(Increment { zero: 0.0, step: 1.0 });
        assert_eq!(snap_axis(3.3, &[], &lines(), 0.3, step), AxisSnap { value: 3.0, line: None, stepped: true });
        assert_eq!(snap_axis(3.3, &[], &lines(), 0.3, None), AxisSnap { value: 3.3, line: None, stepped: false });
        // a line in reach wins over the grid
        assert_eq!(snap_axis(5.75, &[], &lines(), 0.3, step).value, 6.0);
    }

    #[test]
    fn the_threshold_in_metres_by_the_cameras_distance() {
        // 50° over 800 px at 5 m: 8 px ≈ 4.66 cm
        let t = threshold_m(8.0, 5.0, 50f32.to_radians(), 800.0);
        assert!((t - 0.04663).abs() < 1e-4, "{t}");
        assert_eq!(threshold_m(8.0, 5.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn only_the_free_axes_snap_and_the_guides_name_their_lines() {
        let s = Settings { origin: [0.0; 3], ..Settings::default() };
        let lines = join([s.axis_lines(), point_lines([2.0, 0.0, 2.0], Target::Outlines)]);
        let (point, guides) = snap_move(&Move {
            point: [0.05, 1.37, 1.3],
            offsets: [&[], &[], &[]],
            lines: &lines,
            free: [true, false, true],
            threshold: 0.1,
            increments: [None, None, Some(Increment { zero: 0.0, step: 1.0 })],
        });
        // x onto the axis, y not free, z steps to 1
        assert_eq!(point, [0.0, 1.37, 1.0]);
        assert_eq!(guides, [Guide { axis: 0, line: Line { at: 0.0, kind: Target::Axes } }]);
    }

    #[test]
    fn the_free_axes_of_a_handle_a_line_a_plane_a_turned_node() {
        assert_eq!(free_axes(&[[1.0, 0.0, 0.0]], 1e-3), [true, false, false]);
        assert_eq!(free_axes(&[[0.0, 0.0, -1.0]], 1e-3), [false, false, true]);
        assert_eq!(free_axes(&[[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], 1e-3), [true, false, true]);
        let (c, s) = (30f32.to_radians().cos(), 30f32.to_radians().sin());
        assert_eq!(free_axes(&[[c, 0.0, s]], 1e-3), [false, false, false]);
        assert_eq!(free_axes(&[[c, 0.0, s], [-s, 0.0, c]], 1e-3), [true, false, true]);
        assert_eq!(free_axes(&[], 1e-3), [true; 3]);
    }

    #[test]
    fn a_box_gives_its_edges_and_centre() {
        let l = box_lines([1.0, 0.0, -1.0], [2.0, 1.8, 1.0], Target::Objects);
        assert_eq!(l[0].iter().map(|l| l.at).collect::<Vec<_>>(), [1.0, 1.5, 2.0]);
        assert_eq!(l[2].iter().map(|l| l.at).collect::<Vec<_>>(), [-1.0, 0.0, 1.0]);
    }

    #[test]
    fn only_the_chosen_targets_lines() {
        let s = Settings { to: vec![Target::Objects], ..Settings::default() };
        let lines = s.chosen(join([s.axis_lines(), point_lines([1.0, 2.0, 3.0], Target::Objects), point_lines([4.0; 3], Target::Outlines)]));
        assert_eq!(lines, [vec![Line { at: 1.0, kind: Target::Objects }], vec![Line { at: 2.0, kind: Target::Objects }], vec![Line { at: 3.0, kind: Target::Objects }]]);
    }

    #[test]
    fn a_place_snaps_to_the_floor_grid_and_to_another_member() {
        let s = Settings { snap: true, step: 0.5, origin: [0.0; 3], ..Settings::default() };
        let floor = [true, false, true];
        // nothing near: the floor grid's steps on x and z, the height kept
        let (p, g) = snap_place(&s, true, [0.74, 0.0, -1.2], floor, Default::default(), 0.05);
        assert_eq!((p, g.len()), ([0.5, 0.0, -1.0], 0));
        // another member's feet at x 1.33 within reach: x goes onto it, z still steps
        let (p, g) = snap_place(&s, true, [1.31, 0.0, 0.6], floor, point_lines([1.33, 0.0, 4.0], Target::Objects), 0.05);
        assert_eq!(p, [1.33, 0.0, 0.5]);
        assert_eq!(g, [Guide { axis: 0, line: Line { at: 1.33, kind: Target::Objects } }]);
        // snapping off, or the objects no target: as it is, or the grid
        assert_eq!(snap_place(&s, false, [1.31, 0.0, 0.6], floor, Default::default(), 0.05).0, [1.31, 0.0, 0.6]);
        let grid_only = Settings { to: vec![Target::Grid], ..s };
        assert_eq!(snap_place(&grid_only, true, [1.31, 0.0, 0.6], floor, point_lines([1.33, 0.0, 4.0], Target::Objects), 0.05).0, [1.5, 0.0, 0.5]);
    }

    #[test]
    fn the_grid_lies_on_the_floor_or_on_a_wall_through_its_origin() {
        let floor = Settings { step: 1.0, origin: [0.0, 0.2, 0.0], ..Settings::default() };
        let lines = grid_lines(&floor, [0.0; 3], 2.0);
        // five lines each way, all at the origin's height, two of them its zero lines
        assert_eq!(lines.len(), 10);
        assert!(lines.iter().all(|l| l.a[1] == 0.2 && l.b[1] == 0.2));
        assert_eq!(lines.iter().filter(|l| l.zero).count(), 2);
        let wall = Settings { plane: Plane::Wall, origin: [0.0, 0.0, -3.0], ..floor.clone() };
        assert!(grid_lines(&wall, [0.0; 3], 2.0).iter().all(|l| l.a[2] == -3.0 && l.b[2] == -3.0));
        // off: nothing; too fine: thinned to at most MAX_LINES each way
        assert!(grid_lines(&Settings { grid: false, ..floor.clone() }, [0.0; 3], 2.0).is_empty());
        assert!(grid_lines(&Settings { step: 0.05, ..floor }, [0.0; 3], 50.0).len() <= 2 * MAX_LINES);
    }

    #[test]
    fn the_settings_keep_in_the_cache_and_a_missing_field_takes_its_default() {
        let s = Settings { plane: Plane::Wall, snap: true, ..Settings::default() };
        let text = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&text).unwrap(), s);
        assert_eq!(serde_json::from_str::<Settings>(r#"{"plane":"wall"}"#).unwrap().step, 0.5);
    }
}
