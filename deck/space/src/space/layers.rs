//! The scene's parallax layers in the 3D (deck-catalog `Layer`): each a flat textured quad standing `depth` metres
//! behind the scene camera's pivot, facing that camera, sized to its part of the camera's view at that distance – the
//! whole view, or a strip at an edge – so a picture dropped in covers exactly its part, and a layer moved in depth is
//! fitted anew. The parallax is the camera's own perspective as it moves. A picture is cover-fit into its quad (cut off
//! centred, never distorted). The quads are drawn after the opaque 3D, far to near, depth tested, no depth written.
use glam::Vec3;

use crate::cast::Picture;
use crate::Layer;
use deck_catalog::layer::{Anchor, STRIP};

use super::camera::Shot;
use super::portrait::Portraits;

/// the nearest a layer is drawn to the scene camera's eye, metres
const NEAR: f32 = 0.3;

/// the scene's layers in the document's order, each with its picture's slot
#[derive(Debug, Clone, Default)]
pub struct Layers {
    pub list: Vec<(Layer, Option<usize>)>,
}

/// A layer's quad this frame: its middle, its half extents across and up, the part of its picture shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    /// its place in `stage.layers`
    pub layer: usize,
    /// its picture's slot once the file is here
    pub picture: Option<usize>,
    pub middle: Vec3,
    pub across: Vec3,
    pub up: Vec3,
    /// `[u, v, width, height]` of the picture
    pub uv: [f32; 4],
}

impl Plane {
    /// its corners: top left, top right, bottom right, bottom left
    pub fn corners(&self) -> [Vec3; 4] {
        let (m, a, u) = (self.middle, self.across, self.up);
        [m - a + u, m + a + u, m + a - u, m - a - u]
    }
}

/// The quad of a layer `depth` metres behind the pivot of `shot`, covering `anchor`'s part of its view on a stage
/// `aspect` (w / h) wide: (middle, half across, half up).
pub fn place(shot: &Shot, aspect: f32, depth: f32, anchor: Anchor) -> (Vec3, Vec3, Vec3) {
    let (sy, cy) = shot.yaw.sin_cos();
    let (sp, cp) = shot.pitch.sin_cos();
    let back = Vec3::new(cp * sy, sp, cp * cy);
    let (right, up) = (Vec3::new(cy, 0.0, -sy), Vec3::new(-sp * sy, cp, -sp * cy));
    let eye = shot.target + back * shot.distance;
    let far = (shot.distance + depth).max(NEAR);
    let hh = far * (shot.fov_y * 0.5).tan();
    let hw = hh * aspect.max(1e-3);
    let (x, y, w, h) = match anchor {
        Anchor::Full => (0.0, 0.0, hw, hh),
        Anchor::Left => (-hw * (1.0 - STRIP), 0.0, hw * STRIP, hh),
        Anchor::Right => (hw * (1.0 - STRIP), 0.0, hw * STRIP, hh),
        Anchor::Top => (0.0, hh * (1.0 - STRIP), hw, hh * STRIP),
        Anchor::Bottom => (0.0, -hh * (1.0 - STRIP), hw, hh * STRIP),
    };
    (eye - back * far + right * x + up * y, right * w, up * h)
}

impl Layers {
    pub fn new(layers: &[Layer], portraits: &mut Portraits) -> Layers {
        Layers { list: layers.iter().map(|l| (l.clone(), l.image.as_deref().map(|p| portraits.picture(Picture::media(p))))).collect() }
    }

    /// This frame's quads for `shot` on a stage `aspect` wide, far to near from `eye` (into `out`, cleared first): the
    /// ones whose picture is here, or with `all` every layer (an editor outlines them).
    pub fn planes(&self, shot: &Shot, aspect: f32, portraits: &Portraits, eye: Vec3, all: bool, out: &mut Vec<Plane>) {
        out.clear();
        for (i, (l, slot)) in self.list.iter().enumerate() {
            let picture = slot.filter(|&s| portraits.ready(s));
            if picture.is_none() && !all {
                continue;
            }
            let (middle, across, up) = place(shot, aspect, l.depth, l.anchor);
            let uv = picture.map_or([0.0, 0.0, 1.0, 1.0], |s| deck_core::fit::cover((portraits.slots[s].aspect, 1.0), (across.length(), up.length().max(1e-6))));
            out.push(Plane { layer: i, picture, middle, across, up, uv });
        }
        out.sort_by(|a, b| b.middle.distance_squared(eye).total_cmp(&a.middle.distance_squared(eye)));
    }
}

#[cfg(feature = "gpu")]
pub use gpu::PlanesGpu;

#[cfg(feature = "gpu")]
mod gpu {
    use super::Plane;

    const SHADER: &str = r"
struct F { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> f: F;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32, @location(0) m: vec4<f32>, @location(1) a: vec4<f32>, @location(2) u: vec4<f32>, @location(3) r: vec4<f32>) -> V {
    let c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let k = c[i];
    let p = m.xyz + a.xyz * (k.x * 2.0 - 1.0) + u.xyz * (k.y * 2.0 - 1.0);
    var o: V;
    o.pos = f.view_proj * vec4<f32>(p, 1.0);
    o.uv = r.xy + vec2<f32>(k.x, 1.0 - k.y) * r.zw;
    return o;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, v.uv);
    if c.a < 0.004 { discard; }
    return vec4<f32>(pow(c.rgb, vec3<f32>(1.0 / 2.2)), c.a);
}
";

    /// the most quads one frame draws
    pub const MOST: usize = deck_catalog::layer::MOST;

    /// The layers' quads: one instance each from one buffer, inside the space's pass.
    pub struct PlanesGpu {
        pipe: wgpu::RenderPipeline,
        buf: wgpu::Buffer,
        inst: Vec<[[f32; 4]; 4]>,
    }

    impl PlanesGpu {
        /// `layout`: the frame's bind group, then a sprite's (texture, sampler); `depth`: the space pass's depth format
        pub fn new(device: &wgpu::Device, layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat, depth: wgpu::TextureFormat) -> PlanesGpu {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("layers"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
            let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
            let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("layers"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::COLOR })],
                }),
                multiview_mask: None,
                cache: None,
            });
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("layers"), size: (MOST * 64) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            PlanesGpu { pipe, buf, inst: Vec::with_capacity(MOST) }
        }

        /// this frame's quads into the buffer, far to near
        pub fn write(&mut self, queue: &wgpu::Queue, planes: &[Plane]) {
            let v = |p: glam::Vec3| [p.x, p.y, p.z, 0.0];
            self.inst.clear();
            self.inst.extend(planes.iter().take(MOST).map(|p| [v(p.middle), v(p.across), v(p.up), p.uv]));
            if !self.inst.is_empty() {
                queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&self.inst));
            }
        }

        /// the quads written, each with its picture's bind group (group 1), inside the space's pass
        pub fn draw<'a>(&self, rp: &mut wgpu::RenderPass<'_>, pictures: impl Iterator<Item = Option<&'a wgpu::BindGroup>>) {
            if self.inst.is_empty() {
                return;
            }
            rp.set_pipeline(&self.pipe);
            rp.set_vertex_buffer(0, self.buf.slice(..));
            for (i, bind) in pictures.take(self.inst.len()).enumerate() {
                if let Some(bind) = bind {
                    rp.set_bind_group(1, bind, &[]);
                    rp.draw(0..6, i as u32..i as u32 + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot() -> Shot {
        Shot { target: Vec3::new(0.0, 1.0, 0.0), yaw: 0.0, pitch: 0.0, distance: 4.0, fov_y: 55f32.to_radians(), ortho: None }
    }

    fn near(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    #[test]
    fn a_full_layer_fills_the_cameras_view_at_its_depth() {
        let s = shot();
        let (m, a, u) = place(&s, 16.0 / 9.0, 6.0, Anchor::Full);
        assert!(near(m, Vec3::new(0.0, 1.0, -6.0)), "behind the pivot on the camera's axis: {m}");
        let hh = 10.0 * (s.fov_y * 0.5).tan();
        assert!((u.length() - hh).abs() < 1e-4 && (a.length() - hh * 16.0 / 9.0).abs() < 1e-3);
        let (_, _, u2) = place(&s, 16.0 / 9.0, 16.0, Anchor::Full);
        assert!((u2.length() / u.length() - 2.0).abs() < 1e-4, "twice as far from the eye: twice as big – it still covers the view");
    }

    #[test]
    fn edge_strips_cover_their_edge_only() {
        let s = shot();
        let (full, fa, fu) = place(&s, 2.0, -1.0, Anchor::Full);
        let (left, la, lu) = place(&s, 2.0, -1.0, Anchor::Left);
        assert!(near(left - la, full - fa), "its outer edge is the view's");
        assert!((la.length() - fa.length() * STRIP).abs() < 1e-4 && near(lu, fu));
        let (bottom, _, bu) = place(&s, 2.0, -1.0, Anchor::Bottom);
        assert!(near(bottom - bu, full - fu) && (bu.length() - fu.length() * STRIP).abs() < 1e-4);
        let (top, ..) = place(&s, 2.0, -1.0, Anchor::Top);
        let (right, ..) = place(&s, 2.0, -1.0, Anchor::Right);
        assert!(top.y > full.y && right.x > full.x);
    }

    #[test]
    fn far_to_near_from_the_eye_and_only_the_ready_ones() {
        let mut portraits = Portraits::manual();
        let layers = [Layer { image: Some("layers/near.png".into()), ..Layer::empty(1.0, Anchor::Full) }, Layer::empty(30.0, Anchor::Full), Layer { image: Some("layers/far.png".into()), ..Layer::empty(12.0, Anchor::Full) }];
        let l = Layers::new(&layers, &mut portraits);
        let (s, mut out) = (shot(), Vec::new());
        let eye = s.target + Vec3::Z * s.distance;
        l.planes(&s, 16.0 / 9.0, &portraits, eye, false, &mut out);
        assert!(out.is_empty(), "no picture here yet");
        l.planes(&s, 16.0 / 9.0, &portraits, eye, true, &mut out);
        assert_eq!(out.iter().map(|p| p.layer).collect::<Vec<_>>(), [1, 2, 0], "every place, far to near");
        for (slot, w) in [(l.list[0].1.unwrap(), 400), (l.list[2].1.unwrap(), 1600)] {
            portraits.deliver(slot, Ok(image::RgbaImage::new(w, 900)));
        }
        portraits.poll();
        l.planes(&s, 16.0 / 9.0, &portraits, eye, false, &mut out);
        assert_eq!(out.iter().map(|p| p.layer).collect::<Vec<_>>(), [2, 0]);
        let narrow = out.iter().find(|p| p.layer == 0).unwrap();
        assert!(narrow.uv[2] == 1.0 && narrow.uv[3] < 1.0, "too narrow for the view: the width fits, top and bottom cut, {:?}", narrow.uv);
    }
}
