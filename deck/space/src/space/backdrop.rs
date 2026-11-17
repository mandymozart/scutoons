//! The stage's background and its 2D layers on the space (deck-catalog `Background`, the one that won; `Layer`). A colour
//! is what the space clears to and fades into; an image is laid over the drawn stage, cover-fit, its transparency kept,
//! and with a horizon or a parallax it follows the camera. The layers lie around the 3D, far to near: behind it before
//! it is drawn, the front ones over it. Each picture is one textured quad placed by deck-core `parallax`; across it
//! repeats mirrored. Pictures come through the portraits' loader (`Picture::media`). With no colour set the space clears
//! to its ground (black; the editor's `stage` token). A video does not play yet: the ground, said once.
use deck_core::parallax::{self, Quad, View};

use crate::cast::Picture;
use crate::{Background, Layer};

use super::camera::Orbit;
use super::portrait::Portraits;

#[derive(Debug, Clone, Default)]
pub struct Backdrop {
    pub background: Option<Background>,
    /// the image's slot in the portraits
    pub slot: Option<usize>,
    /// what shows when no background sets a colour, display values
    pub ground: [f32; 3],
    /// the layers far to near, each with its picture's slot
    pub layers: Vec<(Layer, usize)>,
}

/// the pictures of one frame: the layers behind the 3D, the ones in front, the background over all
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shown {
    pub behind: Vec<(usize, Quad)>,
    pub front: Vec<(usize, Quad)>,
    pub over: Option<(usize, Quad)>,
}

impl Backdrop {
    pub fn new(background: Option<Background>, portraits: &mut Portraits) -> Backdrop {
        let slot = background.as_ref().and_then(|b| b.image.as_deref()).map(|p| portraits.picture(Picture::media(p)));
        if let Some(v) = background.as_ref().and_then(|b| b.video.as_deref()) {
            // TODO(#326): play it on the core's own video playback
            tracing::warn!("background: the video '{v}' does not play on this deck yet – black");
        }
        Backdrop { background, slot, ground: [0.0; 3], layers: Vec::new() }
    }

    /// the scene's layers (`stage.layers`), their pictures asked for
    pub fn set_layers(&mut self, layers: &[Layer], portraits: &mut Portraits) {
        let far_to_near = deck_catalog::layer::order(layers).into_iter().map(|i| &layers[i]);
        self.layers = far_to_near.map(|l| (l.clone(), portraits.picture(Picture::media(&l.image)))).collect();
    }

    /// what the stage clears to, display values: the colour, else the ground
    pub fn clear(&self) -> [f32; 3] {
        self.background.as_ref().and_then(Background::rgb).unwrap_or(self.ground)
    }

    /// This frame's pictures whose files are here, placed for `view` (into `out`, cleared first).
    pub fn show(&self, portraits: &Portraits, view: &View, out: &mut Shown) {
        out.behind.clear();
        out.front.clear();
        let ready = |s: usize| portraits.ready(s).then(|| portraits.slots[s].aspect);
        for (l, s) in &self.layers {
            let Some(aspect) = ready(*s) else { continue };
            let q = parallax::layer(aspect, l.depth, l.horizon, view);
            if l.front() { &mut out.front } else { &mut out.behind }.push((*s, q));
        }
        out.over = self.slot.zip(self.background.as_ref()).and_then(|(s, b)| Some((s, parallax::background(ready(s)?, b.horizon, b.follows(), view))));
    }
}

/// the camera as the pictures see it, on a stage `size` pixels
pub fn view(cam: &Orbit, size: (u32, u32)) -> View {
    let (home_yaw, home_pitch, _) = cam.home();
    let (sy, cy) = cam.yaw.sin_cos();
    let (sp, cp) = cam.pitch.sin_cos();
    let t = cam.target;
    let pan = (t.x * cy - t.z * sy, -t.x * sp * sy + t.y * cp - t.z * sp * cy);
    View { yaw: cam.yaw, pitch: cam.pitch, home_yaw, home_pitch, distance: cam.distance, pan, fov_y: cam.fov_y, aspect: size.0 as f32 / size.1.max(1) as f32 }
}

#[cfg(feature = "gpu")]
pub use gpu::LayersGpu;

#[cfg(feature = "gpu")]
mod gpu {
    use deck_core::parallax::Quad;

    const SHADER: &str = r"
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
struct V { @builtin(position) pos: vec4<f32>, @location(0) ndc: vec2<f32>, @location(1) q: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32, @location(0) q: vec4<f32>) -> V {
    let c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let k = c[i];
    let p = vec2<f32>(k.x * 2.0 - 1.0, q.z - 2.0 * q.w * k.y);
    var o: V;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.ndc = p;
    o.q = q;
    return o;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    let t = (0.5 + (v.ndc.x - v.q.x) / (2.0 * v.q.y)) - 2.0 * floor((0.5 + (v.ndc.x - v.q.x) / (2.0 * v.q.y)) * 0.5);
    let uv = vec2<f32>(1.0 - abs(1.0 - t), clamp((v.q.z - v.ndc.y) / (2.0 * v.q.w), 0.0, 1.0));
    let c = textureSample(tex, smp, uv);
    return vec4<f32>(pow(c.rgb, vec3<f32>(1.0 / 2.2)), c.a);
}
";

    /// the most pictures one frame places
    pub const MOST: usize = deck_catalog::layer::MOST + 1;

    /// Every picture of the backdrop: a quad per picture from one instance buffer – inside the space's pass (no depth
    /// test, no depth written) and over the finished stage.
    pub struct LayersGpu {
        inside: wgpu::RenderPipeline,
        over: wgpu::RenderPipeline,
        buf: wgpu::Buffer,
        quads: Vec<[f32; 4]>,
    }

    impl LayersGpu {
        /// `layout`: the frame's bind group, then a sprite's (texture, sampler); `depth`: the space pass's depth format
        pub fn new(device: &wgpu::Device, layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat, depth: wgpu::TextureFormat) -> LayersGpu {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("layers"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
            let attrs = wgpu::vertex_attr_array![0 => Float32x4];
            let pipe = |label: &str, depth_stencil: Option<wgpu::DepthStencilState>| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 16, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs })],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::COLOR })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            };
            let flat = wgpu::DepthStencilState { format: depth, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always), stencil: Default::default(), bias: Default::default() };
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("layers"), size: (MOST * 16) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            LayersGpu { inside: pipe("layers", Some(flat)), over: pipe("background", None), buf, quads: Vec::with_capacity(MOST) }
        }

        /// this frame's quads into the buffer, in the order they are drawn: behind, front, the background
        pub fn write(&mut self, queue: &wgpu::Queue, shown: &super::Shown) {
            self.quads.clear();
            let all = shown.behind.iter().chain(&shown.front).chain(&shown.over).take(MOST);
            self.quads.extend(all.map(|(_, q): &(usize, Quad)| [q.x, q.half_w, q.top, q.half_h]));
            if !self.quads.is_empty() {
                queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&self.quads));
            }
        }

        /// the quads `first..` of `pictures` inside the space's pass (group 1 is a sprite's)
        pub fn draw<'a>(&self, rp: &mut wgpu::RenderPass<'_>, first: usize, pictures: impl Iterator<Item = Option<&'a wgpu::BindGroup>>) {
            rp.set_pipeline(&self.inside);
            rp.set_vertex_buffer(0, self.buf.slice(..));
            for (i, bind) in pictures.enumerate().filter(|(i, _)| first + i < MOST) {
                if let Some(bind) = bind {
                    let n = (first + i) as u32;
                    rp.set_bind_group(1, bind, &[]);
                    rp.draw(0..6, n..n + 1);
                }
            }
        }

        /// the background's quad (`at` in the buffer) over the finished stage in `target`
        pub fn over(&self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, frame: &wgpu::BindGroup, picture: &wgpu::BindGroup, at: usize) {
            if at >= MOST {
                return;
            }
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("background"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.over);
            rp.set_bind_group(0, frame, &[]);
            rp.set_bind_group(1, picture, &[]);
            rp.set_vertex_buffer(0, self.buf.slice(..));
            rp.draw(0..6, at as u32..at as u32 + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn deliver(portraits: &mut Portraits, slot: usize, w: u32, h: u32) {
        portraits.deliver(slot, Ok(image::RgbaImage::new(w, h)));
        portraits.poll();
    }

    #[test]
    fn a_colour_clears_an_image_waits_for_its_picture() {
        let mut portraits = Portraits::manual();
        assert_eq!(Backdrop::new(None, &mut portraits).clear(), [0.0; 3], "none: black");
        assert_eq!(Backdrop::new(Some(Background::colour("#fff")), &mut portraits).clear(), [1.0; 3]);
        let b = Backdrop::new(Some(Background::image("backgrounds/wide.png")), &mut portraits);
        assert_eq!(b.clear(), [0.0; 3], "an image lies over black");
        assert_eq!(portraits.requests().len(), 1, "its picture is asked for");
        let v = view(&Orbit::new(Vec3::ZERO, 0.0, 0.2, 4.0), (1600, 900));
        let mut shown = Shown::default();
        b.show(&portraits, &v, &mut shown);
        assert!(shown.over.is_none(), "not here yet");
        deliver(&mut portraits, b.slot.unwrap(), 200, 100);
        b.show(&portraits, &v, &mut shown);
        let (_, q) = shown.over.unwrap();
        assert!((q.half_w - 2.0 / (16.0 / 9.0)).abs() < 1e-5 && q.half_h == 1.0 && q.top == 1.0 && q.x == 0.0, "too wide: the height fits, centred, {q:?}");
    }

    #[test]
    fn with_no_colour_set_the_ground_shows() {
        let mut portraits = Portraits::manual();
        let grey = [0.2, 0.2, 0.2];
        let mut b = Backdrop::new(Some(Background::image("backgrounds/wide.png")), &mut portraits);
        b.ground = grey;
        assert_eq!(b.clear(), grey);
        b = Backdrop { ground: grey, ..Backdrop::new(Some(Background::colour("#fff")), &mut portraits) };
        assert_eq!(b.clear(), [1.0; 3], "a set colour wins over the ground");
    }

    #[test]
    fn layers_far_to_near_behind_and_in_front_moving_by_depth() {
        let mut portraits = Portraits::manual();
        let mut b = Backdrop::default();
        let layer = |image: &str, depth| Layer { image: image.into(), depth, horizon: None };
        b.set_layers(&[layer("fog.png", Some(-1.0)), layer("sky.png", None), layer("hills.png", Some(10.0)), layer("trees.png", Some(2.0))], &mut portraits);
        assert_eq!(b.layers.iter().map(|(l, _)| l.image.as_str()).collect::<Vec<_>>(), ["sky.png", "hills.png", "trees.png", "fog.png"]);
        for s in 0..4 {
            deliver(&mut portraits, s, 1600, 900);
        }
        let mut cam = Orbit::new(Vec3::ZERO, 0.0, 0.2, 4.0);
        let mut shown = Shown::default();
        b.show(&portraits, &view(&cam, (1600, 900)), &mut shown);
        assert_eq!((shown.behind.len(), shown.front.len()), (3, 1));
        assert!(shown.behind.iter().chain(&shown.front).all(|(_, q)| q.x == 0.0), "at home all centred");
        cam.drag(-20.0, 0.0);
        b.show(&portraits, &view(&cam, (1600, 900)), &mut shown);
        let x: Vec<f32> = shown.behind.iter().chain(&shown.front).map(|(_, q)| q.x).collect();
        assert!(x[0] > x[1] && x[1] > x[2] && x[2] > 0.0 && x[3] < 0.0, "the camera turned: the far follow more, the front one goes against, {x:?}");
    }

    #[test]
    fn the_pan_is_the_pivot_on_the_cameras_right_and_up() {
        let mut cam = Orbit::new(Vec3::ZERO, 0.7, 0.3, 4.0);
        cam.pan(-100.0, 0.0, 900.0);
        let v = view(&cam, (1600, 900));
        assert!(v.pan.0 > 0.0 && v.pan.1.abs() < 1e-5, "a drag to the left moved the pivot right: {:?}", v.pan);
        let mut cam = Orbit::new(Vec3::ZERO, -1.1, 0.4, 4.0);
        cam.pan(0.0, 100.0, 900.0);
        let v = view(&cam, (1600, 900));
        assert!(v.pan.1 > 0.0 && v.pan.0.abs() < 1e-5, "up: {:?}", v.pan);
    }
}
