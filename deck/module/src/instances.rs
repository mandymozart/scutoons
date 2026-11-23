//! The instance table (a control module that draws, `#[port(output, instances, capacity = 256)]`): one region of `out`,
//! a `TableHead` and then `capacity` `Instance` records. The module fills it in its control step; the table holds what
//! it last wrote (a step that writes nothing keeps it), the host reads it after every call and the space draws it each
//! frame, one block late. Both records are `repr(C)` and frozen with the region: a field added is a new region.

/// What an instance draws: one of the core's placeholder shapes – or, when the module has a `model` slot of the shape's word
/// (`puff`) and its node names a .glb there, that model fitted to the placeholder's size.
pub mod shape {
    /// a low-poly lumpy ball, about 1 m across (a cloud's puff)
    pub const PUFF: u32 = 0;
    /// a low-poly upright segment, 1 m long from its foot, 1 m across (a bamboo stalk's segment)
    pub const SEGMENT: u32 = 1;
    /// every shape by its word, in index order
    pub const NAMES: [&str; 2] = ["puff", "segment"];
}

/// One drawn shape (64 bytes): where it stands (m, from the module's place), its turn (a unit quaternion x y z w), its
/// size per axis, its colour (linear RGB, alpha 0 … 1).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    pub position: [f32; 3],
    /// `shape::*`
    pub shape: u32,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub _pad: u32,
    pub colour: [f32; 4],
}

impl Instance {
    pub const ZERO: Instance = Instance { position: [0.0; 3], shape: 0, rotation: [0.0, 0.0, 0.0, 1.0], scale: [1.0; 3], _pad: 0, colour: [1.0; 4] };

    /// `shape` at `position`, unturned, 1 m, white
    pub const fn new(shape: u32, position: [f32; 3]) -> Instance {
        Instance { shape, position, ..Instance::ZERO }
    }

    pub const fn turned(mut self, quaternion: [f32; 4]) -> Instance {
        self.rotation = quaternion;
        self
    }

    pub const fn scaled(mut self, scale: [f32; 3]) -> Instance {
        self.scale = scale;
        self
    }

    pub const fn coloured(mut self, colour: [f32; 4]) -> Instance {
        self.colour = colour;
        self
    }
}

/// The table's head (16 bytes): how many records hold.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableHead {
    pub len: u32,
    pub _reserved: [u32; 3],
}

/// A module's view on its table in a call (`Outputs::instances`); empty for a module without one.
pub struct Table<'a> {
    head: Option<&'a mut TableHead>,
    rows: &'a mut [Instance],
}

impl<'a> Table<'a> {
    pub(crate) fn new(head: Option<&'a mut TableHead>, rows: &'a mut [Instance]) -> Table<'a> {
        Table { head, rows }
    }

    pub fn capacity(&self) -> usize {
        self.rows.len()
    }

    pub fn len(&self) -> usize {
        self.head.as_ref().map_or(0, |h| (h.len as usize).min(self.rows.len()))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// nothing drawn
    pub fn clear(&mut self) {
        self.set_len(0);
    }

    /// the first `n` records hold (at most the capacity)
    pub fn set_len(&mut self, n: usize) {
        let n = n.min(self.rows.len()) as u32;
        if let Some(h) = self.head.as_mut() {
            h.len = n;
        }
    }

    /// `i` appended; false when the table is full
    pub fn push(&mut self, i: Instance) -> bool {
        let n = self.len();
        let Some(slot) = self.rows.get_mut(n) else { return false };
        *slot = i;
        self.set_len(n + 1);
        true
    }

    /// the records that hold
    pub fn rows(&self) -> &[Instance] {
        &self.rows[..self.len()]
    }

    /// the records that hold, to change in place
    pub fn rows_mut(&mut self) -> &mut [Instance] {
        let n = self.len();
        &mut self.rows[..n]
    }
}

pub const INSTANCE_SIZE: u32 = core::mem::size_of::<Instance>() as u32;
pub const TABLE_HEAD_SIZE: u32 = core::mem::size_of::<TableHead>() as u32;
const _: () = assert!(INSTANCE_SIZE == 64 && TABLE_HEAD_SIZE == 16);
