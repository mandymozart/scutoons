//! What modules draw on the stage: every instance table the engine publishes (crate::instances) drawn each frame with
//! the core's placeholder shapes (`deck_module::instances::shape`), one instanced draw per shape for all tables. A table
//! stands at its module node's place: a node of a holder's rig (its id under the member's prefix, the rule `Cast::of`
//! uses) at that member's feet, turned as its motion module turns it; any other at the scene's spot on the floor.
//! Instance positions are metres from that place.
use super::Space;
use crate::instances::Table;
use deck_module::instances::shape;
use glam::{Quat, Vec3};
use std::sync::Arc;

/// how many placeholder shapes there are
pub const SHAPES: usize = shape::NAMES.len();

/// a placeholder shape's vertex (flat shaded: a triangle's three carry its normal)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

/// one instance for the GPU (64 bytes): its place in the world (w unused), its turn (x y z w), its size (w unused), its
/// colour (linear)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawnInst {
    pub position: [f32; 4],
    pub rotation: [f32; 4],
    pub scale: [f32; 4],
    pub colour: [f32; 4],
}

/// The tables on this stage and whose they are (a cast member's index; none = the scene's).
#[derive(Debug, Default)]
pub struct Drawn {
    tables: Vec<(Arc<Table>, Option<usize>)>,
}

impl Drawn {
    /// the instances all tables hold at most (what the GPU buffer is sized for)
    pub fn capacity(&self) -> usize {
        self.tables.iter().map(|(t, _)| t.capacity()).sum()
    }
}

impl Space {
    /// the instances the stage's tables hold at most
    pub fn drawn_capacity(&self) -> usize {
        self.drawn.capacity()
    }

    /// The tables of the modules that draw (the engine's `Handles::instances`), each placed by its node.
    pub fn set_tables(&mut self, tables: Vec<Arc<Table>>) {
        let looks: Vec<&str> = self.graph.nodes.iter().filter(|n| n.kind == crate::graph::Kind::Look).map(|n| n.id.strip_suffix(crate::scene::look_id("").as_str()).unwrap_or_default()).collect();
        let owner = |id: &str| looks.iter().enumerate().filter(|(_, p)| !p.is_empty() && id.starts_with(*p)).max_by_key(|(_, p)| p.len()).map(|(k, _)| k).filter(|&k| k < self.cast.len());
        self.drawn.tables = tables.into_iter().map(|t| (owner(&t.node), t)).map(|(o, t)| (t, o)).collect();
        self.generation += 1;
    }

    /// where a table stands and how it is turned
    fn drawn_place(&self, owner: Option<usize>) -> (Vec3, Quat) {
        match owner.and_then(|k| self.cast.get(k)) {
            Some(c) => c.moved.map_or((c.feet, Quat::IDENTITY), |m| (m.position, m.quat())),
            None => (self.being.anchor - Vec3::Y * self.lift, Quat::IDENTITY),
        }
    }

    /// This frame's instances of every table, grouped by shape (`spans[s]`: shape s's first instance and count) – no
    /// allocation once `out` holds the tables' capacity. An instance of a shape the core does not know is left out.
    pub fn write_drawn(&self, out: &mut Vec<DrawnInst>, spans: &mut [(u32, u32); SHAPES]) {
        out.clear();
        for (table, owner) in &self.drawn.tables {
            let (at, turn) = self.drawn_place(*owner);
            table.with(|rows| {
                for r in rows.iter().filter(|r| (r.shape as usize) < SHAPES) {
                    let p = Vec3::from_array(r.position);
                    let q = Quat::from_array(r.rotation);
                    if !p.is_finite() || !q.is_finite() || !r.scale.iter().chain(&r.colour).all(|v| v.is_finite()) {
                        continue;
                    }
                    let q = turn * if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY };
                    let w = at + turn * p;
                    out.push(DrawnInst { position: [w.x, w.y, w.z, r.shape as f32], rotation: q.to_array(), scale: [r.scale[0], r.scale[1], r.scale[2], 0.0], colour: r.colour });
                }
            });
        }
        out.sort_unstable_by_key(|i| i.position[3] as u32);
        let mut first = 0;
        for (s, span) in spans.iter_mut().enumerate() {
            let n = out[first..].iter().take_while(|i| i.position[3] as usize == s).count();
            *span = (first as u32, n as u32);
            first += n;
        }
    }
}

/// The placeholder shapes in one vertex list, and each shape's vertex range in it (by its index).
pub fn meshes() -> (Vec<ShapeVertex>, [(u32, u32); SHAPES]) {
    let mut v = Vec::new();
    let mut spans = [(0, 0); SHAPES];
    for (s, make) in [puff as fn() -> Vec<[Vec3; 3]>, segment].iter().enumerate() {
        let first = v.len();
        for tri in make() {
            let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
            v.extend(tri.iter().map(|p| ShapeVertex { pos: p.to_array(), normal: n.to_array() }));
        }
        spans[s] = (first as u32, (v.len() - first) as u32);
    }
    (v, spans)
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
    use super::{meshes, DrawnInst, ShapeVertex, SHAPES};

    /// The placeholder shapes, instanced: one vertex buffer of the shapes, one instance buffer (a vertex buffer stepped
    /// per instance – WebGL2 has no storage buffers there) rewritten each frame, a draw per shape, inside the space's pass.
    pub struct DrawnGpu {
        pipe: wgpu::RenderPipeline,
        vb: wgpu::Buffer,
        shapes: [(u32, u32); SHAPES],
        buf: wgpu::Buffer,
        capacity: usize,
        inst: Vec<DrawnInst>,
        spans: [(u32, u32); SHAPES],
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
            let (verts, shapes) = meshes();
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("space drawn shapes"),
                size: std::mem::size_of_val(verts.as_slice()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, bytemuck::cast_slice(&verts));
            let mut gpu = DrawnGpu { pipe, vb, shapes, buf: instances(device, 0), capacity: 0, inst: Vec::new(), spans: [(0, 0); SHAPES] };
            gpu.size(device, 0);
            gpu
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
    }
}
