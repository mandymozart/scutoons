//! What modules draw on the stage: every instance table the engine publishes (crate::instances) drawn each frame with
//! the core's placeholder shapes (`deck_module::instances::shape`), one instanced draw per mesh for all tables. A table
//! stands at its module node's place: a node of a holder's rig (its id under the member's prefix, the rule `Cast::of`
//! uses) at that member's feet, turned as its motion module turns it; any other at the scene's spot on the floor.
//! Instance positions are metres from that place. A table whose node names models in a `model` slot draws with them
//! instead of the placeholder: the instances of the shape the slot is named after (the cloud's `puff`), and those whose
//! `look` names the slot (bamboo's `thin` internodes). Each draws one model of the set (`pick`: by its look's group – a
//! stalk's segments one model – or else by its place among the table's instances of its shape, so it keeps its model while
//! the table lives), fitted to the placeholder (`fit`), flat shaded like it; a part the model brings itself (`STANDS_IN`, a
//! node ring) is left out. A model that cannot be read leaves the set; an empty set draws the placeholders.
use super::Space;
use crate::instances::Table;
use deck_module::instances::{look, shape};
use glam::{Quat, Vec3};
use std::sync::Arc;

/// how many placeholder shapes there are
pub const SHAPES: usize = shape::NAMES.len();
/// the most triangles a shape's model may have (a low-poly model: each instance draws all of them)
pub const MAX_MODEL_TRIANGLES: usize = 20_000;

/// a placeholder shape's vertex (flat shaded: a triangle's three carry its normal)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

/// one instance for the GPU (64 bytes): its place in the world (w: its mesh), its turn (x y z w), its size (w unused), its
/// colour (linear)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawnInst {
    pub position: [f32; 4],
    pub rotation: [f32; 4],
    pub scale: [f32; 4],
    pub colour: [f32; 4],
}

/// One table on this stage: whose it is (a cast member's index; none = the scene's), its slots' sets and, per shape,
/// the set of the slot named after it.
#[derive(Debug)]
struct Placed {
    table: Arc<Table>,
    owner: Option<usize>,
    sets: Vec<Set>,
    by_shape: [Option<usize>; SHAPES],
}

/// A `model` slot's set as meshes: per model, per shape the mesh fitted for that shape (an index into `Drawn::models`;
/// none: not needed – a slot named after a shape fits its models for that shape only).
#[derive(Debug)]
struct Set {
    slot: u32,
    models: Vec<[Option<usize>; SHAPES]>,
}

/// The tables on this stage and the models their shapes draw with.
#[derive(Debug, Default)]
pub struct Drawn {
    tables: Vec<Placed>,
    /// the models' meshes: mesh `SHAPES + k` is `models[k]`
    models: Vec<Vec<ShapeVertex>>,
    /// counts every change of `models` (the GPU uploads them anew)
    pub version: u64,
}

impl Drawn {
    /// the instances all tables hold at most (what the GPU buffer is sized for)
    pub fn capacity(&self) -> usize {
        self.tables.iter().map(|t| t.table.capacity()).sum()
    }

    /// how many meshes there are: the placeholders, then the models
    pub fn meshes(&self) -> usize {
        SHAPES + self.models.len()
    }

    /// the models' meshes, in mesh order after the placeholders
    pub fn models(&self) -> &[Vec<ShapeVertex>] {
        &self.models
    }
}

impl Space {
    /// the instances the stage's tables hold at most
    pub fn drawn_capacity(&self) -> usize {
        self.drawn.capacity()
    }

    /// what the stage's tables draw with
    pub fn drawn(&self) -> &Drawn {
        &self.drawn
    }

    /// The tables of the modules that draw (the engine's `Handles::instances`), each placed by its node, its models read
    /// (not a frame's work: a scene's start).
    pub fn set_tables(&mut self, tables: Vec<Arc<Table>>) {
        let looks: Vec<&str> = self.graph.nodes.iter().filter(|n| n.kind == crate::graph::Kind::Look).map(|n| n.id.strip_suffix(crate::scene::look_id("").as_str()).unwrap_or_default()).collect();
        let owner = |id: &str| looks.iter().enumerate().filter(|(_, p)| !p.is_empty() && id.starts_with(*p)).max_by_key(|(_, p)| p.len()).map(|(k, _)| k).filter(|&k| k < self.cast.len());
        let mut models: Vec<Vec<ShapeVertex>> = Vec::new();
        let placed = tables
            .into_iter()
            .map(|table| {
                let mut sets: Vec<Set> = Vec::new();
                let mut by_shape = [None; SHAPES];
                for m in &table.models {
                    let named = m.shape.filter(|s| (*s as usize) < SHAPES);
                    let k = match sets.iter().position(|x| x.slot == m.slot) {
                        Some(k) => k,
                        None => {
                            sets.push(Set { slot: m.slot, models: Vec::new() });
                            sets.len() - 1
                        }
                    };
                    if let Some(s) = named {
                        by_shape[s as usize] = Some(k);
                    }
                    let fitted: anyhow::Result<Vec<(usize, Vec<ShapeVertex>)>> =
                        (0..SHAPES as u32).filter(|s| named.is_none_or(|x| x == *s)).map(|s| Ok((s as usize, model_mesh(&m.bytes, s)?))).collect();
                    match fitted {
                        Ok(fitted) => {
                            let mut by = [None; SHAPES];
                            for (s, mesh) in fitted {
                                by[s] = Some(models.len());
                                models.push(mesh);
                            }
                            sets[k].models.push(by);
                        }
                        Err(e) => tracing::warn!("node '{}': a model of its slot {} can't be drawn ({e:#}) – the set goes without it", table.node, m.slot),
                    }
                }
                Placed { owner: owner(&table.node), table, sets, by_shape }
            })
            .collect();
        self.drawn.tables = placed;
        if !(models.is_empty() && self.drawn.models.is_empty()) {
            self.drawn.models = models;
            self.drawn.version += 1;
        }
        self.generation += 1;
    }

    /// where a table stands and how it is turned
    fn drawn_place(&self, owner: Option<usize>) -> (Vec3, Quat) {
        match owner.and_then(|k| self.cast.get(k)) {
            Some(c) => c.moved.map_or((c.feet, Quat::IDENTITY), |m| (m.position, m.quat())),
            None => (self.being.anchor - Vec3::Y * self.lift, Quat::IDENTITY),
        }
    }

    /// This frame's instances of every table, grouped by mesh (`spans[m]`: mesh m's first instance and count; `spans`
    /// holds `Drawn::meshes()`) – no allocation once `out` holds the tables' capacity. An instance of a shape the core
    /// does not know is left out.
    pub fn write_drawn(&self, out: &mut Vec<DrawnInst>, spans: &mut [(u32, u32)]) {
        out.clear();
        for t in &self.drawn.tables {
            let (at, turn) = self.drawn_place(t.owner);
            t.table.with(|rows| {
                let mut nth = [0u32; SHAPES];
                for r in rows.iter().filter(|r| (r.shape as usize) < SHAPES) {
                    let s = r.shape as usize;
                    let at_shape = nth[s];
                    nth[s] += 1;
                    let p = Vec3::from_array(r.position);
                    let q = Quat::from_array(r.rotation);
                    if !p.is_finite() || !q.is_finite() || !r.scale.iter().chain(&r.colour).all(|v| v.is_finite()) {
                        continue;
                    }
                    let q = turn * if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY };
                    let w = at + turn * p;
                    let set = match look::slot(r.look) {
                        Some(slot) => t.sets.iter().find(|x| x.slot == u32::from(slot)),
                        None => t.by_shape[s].map(|k| &t.sets[k]),
                    };
                    let models = set.map_or(&[][..], |x| x.models.as_slice());
                    let mesh = if models.is_empty() {
                        s
                    } else if r.look & look::STANDS_IN != 0 {
                        continue;
                    } else {
                        let group = look::group(r.look);
                        let k = pick(if group > 0 { group } else { at_shape }, models.len());
                        models[k][s].map_or(s, |m| SHAPES + m)
                    };
                    out.push(DrawnInst { position: [w.x, w.y, w.z, mesh as f32], rotation: q.to_array(), scale: [r.scale[0], r.scale[1], r.scale[2], 0.0], colour: r.colour });
                }
            });
        }
        out.sort_unstable_by_key(|i| i.position[3] as u32);
        let mut first = 0;
        for (m, span) in spans.iter_mut().enumerate() {
            let n = out[first..].iter().take_while(|i| i.position[3] as usize == m).count();
            *span = (first as u32, n as u32);
            first += n;
        }
    }
}

/// Which model of a set of `n` the `nth` instance of a shape (or the group `nth`) draws with: steady for that place in
/// the table, the set's models mixed rather than in turn.
pub fn pick(nth: u32, n: usize) -> usize {
    let mut z = u64::from(nth).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z % n.max(1) as u64) as usize
}

/// The placeholder shapes in one vertex list, and each shape's vertex range in it (by its index).
pub fn meshes() -> (Vec<ShapeVertex>, [(u32, u32); SHAPES]) {
    let mut v = Vec::new();
    let mut spans = [(0, 0); SHAPES];
    for (s, make) in [puff as fn() -> Vec<[Vec3; 3]>, segment].iter().enumerate() {
        let first = v.len();
        v.extend(flat(make()));
        spans[s] = (first as u32, (v.len() - first) as u32);
    }
    (v, spans)
}

/// triangles as flat-shaded vertices (each corner carries its face's normal)
fn flat(tris: Vec<[Vec3; 3]>) -> Vec<ShapeVertex> {
    let mut v = Vec::with_capacity(tris.len() * 3);
    for tri in tris {
        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
        v.extend(tri.iter().map(|p| ShapeVertex { pos: p.to_array(), normal: n.to_array() }));
    }
    v
}

/// A model (.glb) as the mesh of `shape`: its rest pose's triangles fitted where the placeholder stands (`fit`), flat shaded.
pub fn model_mesh(bytes: &[u8], shape: u32) -> anyhow::Result<Vec<ShapeVertex>> {
    // its pictures are not drawn: the instance's colour lights it
    let figure = super::figure::read(bytes, None, &mut |_, _| Ok(image::RgbaImage::new(1, 1)))?;
    let tris = figure.indices.len() / 3;
    anyhow::ensure!(tris > 0, "no triangles");
    anyhow::ensure!(tris <= MAX_MODEL_TRIANGLES, "{tris} triangles – a shape's model has at most {MAX_MODEL_TRIANGLES}");
    let at = figure.rest_positions();
    let corner = |i: u32| at.get(i as usize).copied().unwrap_or(Vec3::ZERO);
    let tris: Vec<[Vec3; 3]> = figure.indices.chunks_exact(3).map(|t| [corner(t[0]), corner(t[1]), corner(t[2])]).collect();
    Ok(flat(fit(tris, shape)))
}

/// A model's triangles where the shape's placeholder stands: a puff centred on its place, its longest side 1 m; a
/// segment standing on its foot at 0, stretched to the placeholder's 1 m along its axis (y) and 1 m across (its widest
/// side, centred), so the instance's size is the segment's length and width.
fn fit(mut tris: Vec<[Vec3; 3]>, shape: u32) -> Vec<[Vec3; 3]> {
    let (lo, hi) = tris.iter().flatten().fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(l, h), p| (l.min(*p), h.max(*p)));
    let inv = |x: f32| if x > 1e-6 { 1.0 / x } else { 1.0 };
    let middle = (lo + hi) * 0.5;
    let size = hi - lo;
    let (origin, k) = if shape == self::shape::SEGMENT {
        let across = inv(size.x.max(size.z));
        (Vec3::new(middle.x, lo.y, middle.z), Vec3::new(across, inv(size.y), across))
    } else {
        (middle, Vec3::splat(inv(size.max_element())))
    };
    for p in tris.iter_mut().flatten() {
        *p = (*p - origin) * k;
    }
    tris
}
/// a triangle wound to face away from `inside`
fn outward(t: [Vec3; 3], inside: Vec3) -> [Vec3; 3] {
    let n = (t[1] - t[0]).cross(t[2] - t[0]);
    if n.dot(t[0] + t[1] + t[2] - 3.0 * inside) < 0.0 {
        [t[0], t[2], t[1]]
    } else {
        t
    }
}

/// `shape::PUFF`: an icosphere split once (80 faces), about 1 m across, its surface lumpy
fn puff() -> Vec<[Vec3; 3]> {
    let g = (1.0 + 5f32.sqrt()) / 2.0;
    let p: Vec<Vec3> = [[-1.0, g, 0.0], [1.0, g, 0.0], [-1.0, -g, 0.0], [1.0, -g, 0.0], [0.0, -1.0, g], [0.0, 1.0, g], [0.0, -1.0, -g], [0.0, 1.0, -g], [g, 0.0, -1.0], [g, 0.0, 1.0], [-g, 0.0, -1.0], [-g, 0.0, 1.0]]
        .iter()
        .map(|a| Vec3::from_array(*a).normalize())
        .collect();
    const FACES: [[usize; 3]; 20] =
        [[0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8], [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1]];
    let lump = |d: Vec3| {
        let d = d.normalize();
        0.5 * (1.0 + 0.13 * (d.x * 5.1 + d.y * 1.3).sin() + 0.1 * (d.z * 4.3 - d.y * 2.2).sin())
    };
    let mut out = Vec::with_capacity(80);
    for f in FACES {
        let [a, b, c] = f.map(|i| p[i]);
        let (ab, bc, ca) = ((a + b).normalize(), (b + c).normalize(), (c + a).normalize());
        for t in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
            out.push(outward(t.map(|d| d * lump(d)), Vec3::ZERO));
        }
    }
    out
}

/// sides of `shape::SEGMENT`
const SIDES: usize = 7;

/// `shape::SEGMENT`: an upright prism of seven sides, 1 m long from its foot at 0, 1 m across, closed at both ends
fn segment() -> Vec<[Vec3; 3]> {
    let ring = |y: f32, k: usize| {
        let a = std::f32::consts::TAU * k as f32 / SIDES as f32;
        Vec3::new(0.5 * a.cos(), y, 0.5 * a.sin())
    };
    let mut out = Vec::with_capacity(SIDES * 4);
    for k in 0..SIDES {
        let (a0, a1, b0, b1) = (ring(0.0, k), ring(0.0, k + 1), ring(1.0, k), ring(1.0, k + 1));
        let axis = Vec3::new(0.0, 0.5, 0.0);
        out.push(outward([a0, a1, b1], axis));
        out.push(outward([a0, b1, b0], axis));
        out.push(outward([Vec3::ZERO, a1, a0], Vec3::new(0.0, 1.0, 0.0)));
        out.push(outward([Vec3::Y, b0, b1], Vec3::ZERO));
    }
    out
}

#[cfg(feature = "gpu")]
pub use gpu::DrawnGpu;

#[cfg(feature = "gpu")]
mod gpu {
    use super::{meshes, DrawnInst, ShapeVertex};

    /// The placeholder shapes and the tables' models, instanced: one vertex buffer of the meshes, one instance buffer (a
    /// vertex buffer stepped per instance – WebGL2 has no storage buffers there) rewritten each frame, a draw per mesh,
    /// inside the space's pass.
    pub struct DrawnGpu {
        pipe: wgpu::RenderPipeline,
        vb: wgpu::Buffer,
        /// each mesh's vertex range in `vb`
        shapes: Vec<(u32, u32)>,
        /// the models' `Drawn::version` in `vb`
        version: u64,
        buf: wgpu::Buffer,
        capacity: usize,
        inst: Vec<DrawnInst>,
        spans: Vec<(u32, u32)>,
    }

    impl DrawnGpu {
        /// `module`: the space's shader (`vs_drawn` / `fs_drawn`); `layout`: the frame's bind group alone
        pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, module: &wgpu::ShaderModule, layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat, depth: wgpu::TextureFormat) -> DrawnGpu {
            let shape_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
            let inst_attrs = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];
            let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("space drawn"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_drawn"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<ShapeVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &shape_attrs }),
                        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<DrawnInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &inst_attrs }),
                    ],
                },
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_drawn"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::COLOR })],
                }),
                multiview_mask: None,
                cache: None,
            });
            let (vb, shapes) = vertices(device, queue, &[]);
            let mut gpu = DrawnGpu { pipe, vb, spans: vec![(0, 0); shapes.len()], shapes, version: 0, buf: instances(device, 0), capacity: 0, inst: Vec::new() };
            gpu.size(device, 0);
            gpu
        }

        /// the tables' models on the GPU, once they changed (a scene's start)
        pub fn models(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, drawn: &super::Drawn) {
            if drawn.version != self.version {
                (self.vb, self.shapes) = vertices(device, queue, drawn.models());
                self.spans = vec![(0, 0); self.shapes.len()];
                self.version = drawn.version;
            }
        }

        /// room for `capacity` instances (the stage's tables'), when they came or went
        pub fn size(&mut self, device: &wgpu::Device, capacity: usize) {
            if capacity != self.capacity || self.inst.capacity() < capacity {
                self.capacity = capacity;
                self.buf = instances(device, capacity);
                self.inst = Vec::with_capacity(capacity);
            }
        }

        /// this frame's instances into the buffer
        pub fn write(&mut self, queue: &wgpu::Queue, space: &super::Space) {
            if self.version != space.drawn().version {
                // the models came after this frame's upload: placeholders only until `models` ran
                self.inst.clear();
                return;
            }
            space.write_drawn(&mut self.inst, &mut self.spans);
            self.inst.truncate(self.capacity);
            if !self.inst.is_empty() {
                queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&self.inst));
            }
        }

        /// a draw per shape, inside the space's pass (the frame's bind group set)
        pub fn draw(&self, rp: &mut wgpu::RenderPass<'_>) {
            if self.inst.is_empty() {
                return;
            }
            rp.set_pipeline(&self.pipe);
            rp.set_vertex_buffer(0, self.vb.slice(..));
            rp.set_vertex_buffer(1, self.buf.slice(..));
            let n = self.inst.len() as u32;
            for (&(first, count), &(start, k)) in self.shapes.iter().zip(&self.spans) {
                let end = (start + k).min(n);
                if end > start {
                    rp.draw(first..first + count, start..end);
                }
            }
        }
    }

    /// the placeholders, then `models`, in one vertex buffer; each mesh's range
    fn vertices(device: &wgpu::Device, queue: &wgpu::Queue, models: &[Vec<ShapeVertex>]) -> (wgpu::Buffer, Vec<(u32, u32)>) {
        let (mut verts, placeholders) = meshes();
        let mut shapes = placeholders.to_vec();
        for m in models {
            shapes.push((verts.len() as u32, m.len() as u32));
            verts.extend_from_slice(m);
        }
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("space drawn shapes"),
            size: std::mem::size_of_val(verts.as_slice()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, bytemuck::cast_slice(&verts));
        (vb, shapes)
    }

    fn instances(device: &wgpu::Device, n: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("space drawn instances"),
            size: (n.max(1) * std::mem::size_of::<DrawnInst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::Instance;

    #[test]
    fn the_shapes_are_closed_low_poly_and_face_outward() {
        let (v, spans) = meshes();
        assert_eq!(spans[shape::PUFF as usize], (0, 240), "80 faces");
        assert_eq!(spans[shape::SEGMENT as usize].1 as usize, SIDES * 4 * 3);
        for (s, &(first, count)) in spans.iter().enumerate() {
            let tris = &v[first as usize..(first + count) as usize];
            let middle = tris.iter().fold(Vec3::ZERO, |a, x| a + Vec3::from(x.pos)) / tris.len() as f32;
            for t in tris.chunks(3) {
                let c = (Vec3::from(t[0].pos) + Vec3::from(t[1].pos) + Vec3::from(t[2].pos)) / 3.0;
                assert!(Vec3::from(t[0].normal).dot(c - middle) > 0.0, "shape {s}: a face turned in");
            }
            let lo = tris.iter().fold(Vec3::splat(9.0), |a, x| a.min(Vec3::from(x.pos)));
            let hi = tris.iter().fold(Vec3::splat(-9.0), |a, x| a.max(Vec3::from(x.pos)));
            assert!((hi - lo).max_element() < 1.3 && (hi - lo).min_element() > 0.8, "about a metre: {lo} … {hi}");
        }
    }

    #[test]
    fn a_table_stands_at_its_place_grouped_by_shape() {
        let scene = crate::scene::Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\n").unwrap();
        let graph = crate::graph::Graph::build(&scene).unwrap();
        let mut space = Space::new(&graph, 0, &scene.being, std::path::Path::new("/nowhere"), super::super::portrait::Portraits::manual());
        let t = Table::new("sky", 4);
        t.publish(&[Instance::new(shape::SEGMENT, [0.0, 1.0, 0.0]), Instance::new(shape::PUFF, [1.0, 2.0, 0.0]), Instance::new(7, [0.0; 3])]);
        space.set_tables(vec![t]);
        let (mut out, mut spans) = (Vec::new(), [(0, 0); SHAPES]);
        space.write_drawn(&mut out, &mut spans);
        assert_eq!(spans, [(0, 1), (1, 1)], "a shape the core does not know is left out");
        let place = space.being.anchor - Vec3::Y * space.lift;
        assert_eq!(Vec3::from_slice(&out[0].position[..3]), place + Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(space.drawn.capacity(), 4);
        assert_eq!((space.drawn.meshes(), space.drawn.version), (SHAPES, 0), "no models: the placeholders alone");
    }

    fn space() -> Space {
        let scene = crate::scene::Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\n").unwrap();
        let graph = crate::graph::Graph::build(&scene).unwrap();
        Space::new(&graph, 0, &scene.being, std::path::Path::new("/nowhere"), super::super::portrait::Portraits::manual())
    }

    fn named(slot: u32, shape: u32, bytes: Arc<[u8]>) -> crate::instances::Model {
        crate::instances::Model { slot, shape: Some(shape), bytes }
    }

    /// a table whose node names a puff model draws its puffs with it – fitted to a metre – and its other shapes as the
    /// placeholders; a model that cannot be read leaves the placeholder
    #[test]
    fn a_shapes_model_draws_it_instead_of_the_placeholder() {
        let mut space = space();
        let big = super::super::bubble::glb();
        let t = Table::with_models("sky", 4, vec![named(0, shape::PUFF, big.into()), named(1, shape::SEGMENT, Arc::from(&b"no glb"[..]))]);
        t.publish(&[Instance::new(shape::PUFF, [0.0; 3]), Instance::new(shape::SEGMENT, [0.0; 3]), Instance::new(shape::PUFF, [1.0, 0.0, 0.0])]);
        space.set_tables(vec![t]);
        assert_eq!((space.drawn.meshes(), space.drawn.version), (SHAPES + 1, 1), "the puff's model; the broken segment model left out");
        let model = &space.drawn.models()[0];
        assert_eq!(model.len(), 320 * 3);
        let (lo, hi) = model.iter().fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(l, h), v| (l.min(v.pos.into()), h.max(v.pos.into())));
        assert!(((hi - lo).max_element() - 1.0).abs() < 1e-4 && (lo + hi).length() < 1e-4, "a metre, centred: {lo} … {hi}");
        let mut spans = vec![(0, 0); space.drawn.meshes()];
        let mut out = Vec::new();
        space.write_drawn(&mut out, &mut spans);
        assert_eq!(spans, [(0, 0), (0, 1), (1, 2)], "no placeholder puff, one segment, two puffs of the model");
        space.set_tables(Vec::new());
        assert_eq!((space.drawn.meshes(), space.drawn.version), (SHAPES, 2), "the next scene without it");
    }

    /// a shape's set of models: each instance picks one, steadily by its place among the shape's instances (moving or
    /// a split keeps it), the set's models mixed; a broken model leaves the set, an empty set draws the placeholder
    #[test]
    fn each_instance_picks_one_model_of_the_set() {
        let mut space = space();
        let bubble: Arc<[u8]> = super::super::bubble::glb().into();
        let models = vec![named(0, shape::PUFF, bubble.clone()), named(0, shape::PUFF, Arc::from(&b"no glb"[..])), named(0, shape::PUFF, bubble)];
        let t = Table::with_models("sky", 16, models);
        let puffs = |dx: f32| -> Vec<Instance> { (0..12).map(|i| Instance::new(shape::PUFF, [i as f32 * dx, 0.0, 0.0])).collect() };
        let mut rows = vec![Instance::new(shape::SEGMENT, [0.0; 3])];
        rows.extend(puffs(1.0));
        t.publish(&rows);
        space.set_tables(vec![t.clone()]);
        assert_eq!(space.drawn.meshes(), SHAPES + 2, "two of the three models read");
        let meshes = |space: &Space| {
            let mut spans = vec![(0, 0); space.drawn.meshes()];
            let mut out = Vec::new();
            space.write_drawn(&mut out, &mut spans);
            let mut by_x: Vec<(i32, u32)> = out.iter().filter(|i| i.position[3] as usize >= SHAPES).map(|i| ((i.position[0] * 100.0) as i32, i.position[3] as u32)).collect();
            by_x.sort_unstable();
            (spans, by_x.into_iter().map(|(_, m)| m).collect::<Vec<_>>())
        };
        let (spans, first) = meshes(&space);
        assert_eq!(spans[shape::SEGMENT as usize].1, 1, "a shape without models: the placeholder");
        assert!(spans[SHAPES].1 > 0 && spans[SHAPES + 1].1 > 0 && spans[SHAPES].1 + spans[SHAPES + 1].1 == 12, "a mix of both: {spans:?}");
        t.publish(&{
            let mut moved = vec![Instance::new(shape::SEGMENT, [0.0; 3])];
            moved.extend(puffs(1.5));
            moved
        });
        assert_eq!(meshes(&space).1, first, "every puff keeps its model when the cloud moves or splits");
        assert_eq!((0..12).map(|i| pick(i, 2)).collect::<Vec<_>>(), (0..12).map(|i| pick(i, 2)).collect::<Vec<_>>());
        assert_eq!(pick(5, 1), 0);
    }

    /// the owner's Tripo models (one textured material, `KHR_materials_volume` and `FB_ngon_encoding` used, not
    /// required): their triangles read, fitted to a metre and centred; the extensions and the texture are not drawn
    #[test]
    fn the_owners_blob_clouds_read_as_puffs() {
        for (bytes, tris) in [(&include_bytes!("../../tests/fixtures/cloud-three-blobs.glb")[..], 4118), (&include_bytes!("../../tests/fixtures/cloud-four-blobs.glb")[..], 2178)] {
            let mesh = model_mesh(bytes, shape::PUFF).unwrap();
            assert_eq!(mesh.len(), tris * 3);
            let (lo, hi) = mesh.iter().fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(l, h), v| (l.min(v.pos.into()), h.max(v.pos.into())));
            assert!(((hi - lo).max_element() - 1.0).abs() < 1e-4 && (lo + hi).length() < 1e-4, "a metre, centred: {lo} … {hi}");
        }
    }

    /// segments whose look names a slot (bamboo's internodes) draw its models: a group's segments (a stalk) one model,
    /// the groups mixed; the parts that stand in for the model (the node rings) are left out while the set has models,
    /// drawn with everything else as placeholders while it is empty
    #[test]
    fn a_stalks_segments_pick_one_internode_and_its_rings_step_aside() {
        use deck_module::AssetSlot;
        let (thin, thick) = (AssetSlot(0), AssetSlot(1));
        let stalks = |groups: u32| -> Vec<Instance> {
            let mut rows = Vec::new();
            for g in 1..=groups {
                for k in 0..6 {
                    rows.push(Instance::new(shape::SEGMENT, [g as f32, k as f32, 0.0]).looks(thin, g));
                    rows.push(Instance::new(shape::SEGMENT, [g as f32, k as f32 + 0.9, 0.0]).standing_in(thin));
                }
                rows.push(Instance::new(shape::SEGMENT, [g as f32, 0.0, 5.0]).looks(thick, g));
                rows.push(Instance::new(shape::PUFF, [g as f32, 6.0, 0.0]));
            }
            rows
        };
        let bubble: Arc<[u8]> = super::super::bubble::glb().into();
        let model = |slot| crate::instances::Model { slot, shape: None, bytes: bubble.clone() };
        let mut space = space();
        let t = Table::with_models("grove", 256, vec![model(0), model(0)]);
        t.publish(&stalks(10));
        space.set_tables(vec![t]);
        assert_eq!(space.drawn.meshes(), SHAPES + 2 * SHAPES, "a slot named after no shape: each model fitted for every shape");
        let mut spans = vec![(0, 0); space.drawn.meshes()];
        let mut out = Vec::new();
        space.write_drawn(&mut out, &mut spans);
        assert_eq!(spans[shape::SEGMENT as usize].1, 10, "no rings; the thick stalks (an empty set) as placeholders");
        assert_eq!(spans[shape::PUFF as usize].1, 10, "the crowns");
        let mut by_stalk = std::collections::BTreeMap::<i32, std::collections::BTreeSet<u32>>::new();
        for i in out.iter().filter(|i| i.position[3] as usize >= SHAPES) {
            by_stalk.entry(i.position[0] as i32).or_default().insert(i.position[3] as u32);
        }
        assert_eq!(by_stalk.len(), 10);
        assert!(by_stalk.values().all(|m| m.len() == 1), "a stalk's six segments one model: {by_stalk:?}");
        let used: std::collections::BTreeSet<u32> = by_stalk.values().flatten().copied().collect();
        assert_eq!(used.len(), 2, "the grove a mix of both");
        assert_eq!(out.iter().filter(|i| i.position[3] as usize >= SHAPES).count(), 60);

        // an empty set: every segment and ring as today
        let mut bare = super::tests::space();
        let t = Table::new("grove", 256);
        t.publish(&stalks(10));
        bare.set_tables(vec![t]);
        let mut spans = vec![(0, 0); bare.drawn.meshes()];
        bare.write_drawn(&mut out, &mut spans);
        assert_eq!(spans, [(0, 10), (10, 130)], "the placeholders, rings included");
    }

    #[test]
    fn a_segments_model_stands_on_its_foot_stretched_to_it() {
        let mesh = model_mesh(&super::super::bubble::glb(), shape::SEGMENT).unwrap();
        let (lo, hi) = mesh.iter().fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(l, h), v| (l.min(v.pos.into()), h.max(v.pos.into())));
        assert!(lo.y.abs() < 1e-5 && (hi.y - 1.0).abs() < 1e-5, "its foot at 0, a metre long: {lo} … {hi}");
        let tall = fit(vec![[Vec3::ZERO, Vec3::new(0.2, 0.0, 0.0), Vec3::new(0.0, 3.0, 0.1)]], shape::SEGMENT);
        assert_eq!(tall[0], [Vec3::new(-0.5, 0.0, -0.25), Vec3::new(0.5, 0.0, -0.25), Vec3::new(-0.5, 1.0, 0.25)], "1 m along, 1 m across");
        let fit = fit(vec![[Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)]], shape::PUFF);
        assert_eq!(fit[0], [Vec3::new(-0.5, -0.25, 0.0), Vec3::new(0.5, -0.25, 0.0), Vec3::new(-0.5, 0.25, 0.0)]);
    }
}
