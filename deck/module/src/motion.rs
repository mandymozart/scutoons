//! The motion region (a slot module's transform, research/MODULE-EXTENSIONS.md §3): a module whose descriptor names a
//! `*.motion` slot gets one more region each way. `TransformIn` (host → module): what its entity is and where the shot,
//! the move or the placement put it this call, the row's time, the entity's marks (a camera's shots, an item's spot).
//! `TransformOut` (module → host): the transform it writes – each part offset on top of the base, or set (`absolute`),
//! only the parts its `written` bits name. Both `repr(C)`, frozen with the region: a field added is a new region.

/// a camera's motion slot
pub const KIND_CAMERA: u32 = 2;
/// an entity's on the stage (an item, a character)
pub const KIND_ENTITY: u32 = 1;
/// marks a call carries at most (a camera's first shots)
pub const MARKS: usize = 8;

/// host → module (256 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformIn {
    /// `KIND_ENTITY` or `KIND_CAMERA`
    pub kind: u32,
    /// ≠ 0: a show row plays and `row_secs` / `row_beats` count from its start
    pub row_valid: u32,
    pub row_secs: f32,
    pub row_beats: f32,
    /// the entity where its placement puts it (m), its rotation (rad, x y z), its scale
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    /// a camera: where the shot or the move aims (m), its focal length (mm), an orthographic view's height (m)
    pub target: [f32; 3],
    pub focal: f32,
    pub size: f32,
    /// a camera: 0 perspective, 1 orthographic
    pub projection: u32,
    /// a camera: the shot it went to last (1-based in its list), 0 = none; a move's progress 0 … 1, `moving` ≠ 0 while
    /// one plays
    pub shot: u32,
    pub shots: u32,
    pub progress: f32,
    pub moving: u32,
    /// how many `marks` hold: a camera's shots' eyes (xyz, w = the focal length), an entity's spot (xyz, w = 0)
    pub marks: u32,
    pub _pad: [u32; 2],
    pub mark: [[f32; 4]; MARKS],
    pub _reserved: [u8; 24],
}

impl TransformIn {
    pub const ZERO: TransformIn = TransformIn {
        kind: 0,
        row_valid: 0,
        row_secs: 0.0,
        row_beats: 0.0,
        position: [0.0; 3],
        rotation: [0.0; 3],
        scale: [1.0; 3],
        target: [0.0; 3],
        focal: 0.0,
        size: 0.0,
        projection: 0,
        shot: 0,
        shots: 0,
        progress: 0.0,
        moving: 0,
        marks: 0,
        _pad: [0; 2],
        mark: [[0.0; 4]; MARKS],
        _reserved: [0; 24],
    };

    pub fn camera(&self) -> bool {
        self.kind == KIND_CAMERA
    }

    /// the marks this call carries
    pub fn marks(&self) -> &[[f32; 4]] {
        &self.mark[..(self.marks as usize).min(MARKS)]
    }

    /// seconds and beats since the row started (`None` without a row)
    pub fn row(&self) -> Option<(f32, f32)> {
        (self.row_valid != 0).then_some((self.row_secs, self.row_beats))
    }
}

/// a part of the transform: a bit of `written` / `absolute`
pub mod part {
    pub const POSITION: u32 = 1;
    pub const ROTATION: u32 = 2;
    pub const SCALE: u32 = 4;
    pub const TARGET: u32 = 8;
    pub const FOCAL: u32 = 16;
    pub const PROJECTION: u32 = 32;
    /// every part, by the slot table's words (deck-catalog `slot`)
    pub const WORDS: [(&str, u32); 6] = [("position", POSITION), ("rotation", ROTATION), ("scale", SCALE), ("target", TARGET), ("focal", FOCAL), ("projection", PROJECTION)];
}

/// module → host (80 bytes). Reset at the start of each call: nothing written.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformOut {
    /// the parts written this call (`part::*`)
    pub written: u32,
    /// of those, the parts set instead of offset on top of the base
    pub absolute: u32,
    /// 0 perspective, 1 orthographic (always absolute)
    pub projection: u32,
    pub _pad: u32,
    /// m, added (or set)
    pub position: [f32; 3],
    /// rad, added (or set)
    pub rotation: [f32; 3],
    /// a factor (or the scale)
    pub scale: [f32; 3],
    /// m, added (or set)
    pub target: [f32; 3],
    /// mm, added (or set)
    pub focal: f32,
    /// an orthographic view's height (m) with `projection` = 1
    pub size: f32,
    pub _reserved: [u8; 8],
}

impl TransformOut {
    pub const ZERO: TransformOut = TransformOut {
        written: 0,
        absolute: 0,
        projection: 0,
        _pad: 0,
        position: [0.0; 3],
        rotation: [0.0; 3],
        scale: [1.0; 3],
        target: [0.0; 3],
        focal: 0.0,
        size: 0.0,
        _reserved: [0; 8],
    };

    fn put(&mut self, part: u32, absolute: bool) {
        self.written |= part;
        if absolute {
            self.absolute |= part;
        } else {
            self.absolute &= !part;
        }
    }

    /// moved by `by` (m) from where the base puts it
    pub fn offset_position(&mut self, by: [f32; 3]) {
        self.position = by;
        self.put(part::POSITION, false);
    }

    /// at `at` (m), whatever the base says – the module drives it
    pub fn set_position(&mut self, at: [f32; 3]) {
        self.position = at;
        self.put(part::POSITION, true);
    }

    /// turned by `by` (rad, x y z) on top of the base
    pub fn offset_rotation(&mut self, by: [f32; 3]) {
        self.rotation = by;
        self.put(part::ROTATION, false);
    }

    pub fn set_rotation(&mut self, to: [f32; 3]) {
        self.rotation = to;
        self.put(part::ROTATION, true);
    }

    /// the base's scale times `by`
    pub fn scale_by(&mut self, by: [f32; 3]) {
        self.scale = by;
        self.put(part::SCALE, false);
    }

    pub fn set_scale(&mut self, to: [f32; 3]) {
        self.scale = to;
        self.put(part::SCALE, true);
    }

    /// a camera's aim moved by `by` (m)
    pub fn offset_target(&mut self, by: [f32; 3]) {
        self.target = by;
        self.put(part::TARGET, false);
    }

    pub fn set_target(&mut self, at: [f32; 3]) {
        self.target = at;
        self.put(part::TARGET, true);
    }

    /// a camera's focal length plus `mm`
    pub fn offset_focal(&mut self, mm: f32) {
        self.focal = mm;
        self.put(part::FOCAL, false);
    }

    pub fn set_focal(&mut self, mm: f32) {
        self.focal = mm;
        self.put(part::FOCAL, true);
    }

    /// a camera's projection: perspective, or orthographic `size` m high
    pub fn set_projection(&mut self, ortho: Option<f32>) {
        (self.projection, self.size) = match ortho {
            Some(h) => (1, h),
            None => (0, 0.0),
        };
        self.put(part::PROJECTION, true);
    }

    pub fn wrote(&self, part: u32) -> bool {
        self.written & part != 0
    }

    pub fn sets(&self, part: u32) -> bool {
        self.absolute & part != 0
    }
}

const _: () = assert!(core::mem::size_of::<TransformIn>() == 256);
const _: () = assert!(core::mem::size_of::<TransformOut>() == 80);
