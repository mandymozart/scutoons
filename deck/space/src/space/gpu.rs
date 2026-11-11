//! The space node on the GPU (#188): its own depth buffer at the render size; per frame the room (one vertex / index
//! buffer, a draw per texture, unlit, depth-written), the being's solid spheres (depth-written), then its tubes and
//! ghost spheres (additive, depth-tested only – the order among them doesn't matter). The sphere and tube buffers are
//! sized from the beings (`Space::capacity`) – again only when one comes or goes (`Space::generation`) – and rewritten
//! per frame from preallocated vectors.
//!
//! The heads (#188 phase 5) come between the solids and the ghosts: per head, far to near, a curved face plate (a grid
//! bulged toward the eye, turned to it around the vertical) drawn twice – the dark machine layer behind, then the
//! portrait (the previous mood fading out under the new one) – alpha-blended, depth-tested and written, so the skin's
//! ghost sphere and the bubbles behind the face sit right. A sprite is a texture per portrait slot, uploaded when its
//! loader has it.
//!
//! Last, the buttons' labels (label.rs): one quad per character from a glyph atlas made once, a fixed size on the
//! screen beside its button, at its button's depth (premultiplied alpha, depth-tested, not written) – their instances rewritten per frame from
//! preallocated vectors.
use super::being::{Sphere, TubeVertex};
use super::label::{self, Atlas, Label, LABELS};
use super::glb::{Image, Model, RoomVertex};
use super::post::Effect;
use super::post_gpu::{Pipes, PostGpu};
use crate::quality::Rung;
use super::{Space, MAX_LIGHTS};
use glam::Vec3;

/// the face plate's grid (quads per side)
const PLATE: u32 = 16;
/// how far the face plate bulges toward the eye (a fraction of its half width)
const BULGE: f32 = 0.32;

/// one face-plate layer for the GPU (96 bytes): centre + half width, right + bulge, up + alpha, forward + layer
/// (0 = the machine behind, 1 = the portrait), extra = (glow, hue, energy, has a face), more = (strain – the red
/// flush of an overloaded output, #188 –, 0, 0, 0)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct HeadLayer {
    center: [f32; 3],
    size: f32,
    right: [f32; 3],
    bulge: f32,
    up: [f32; 3],
    alpha: f32,
    fwd: [f32; 3],
    layer: f32,
    extra: [f32; 4],
    more: [f32; 4],
}

const SHADER: &str = include_str!("../shaders/space.wgsl");
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// the void / the far nave: a deep blue-black, and how fast the fog closes in (per metre)
const FOG: [f32; 4] = [0.012, 0.014, 0.03, 0.045];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    look: [f32; 4],
    fog: [f32; 4],
    /// the target's size in pixels (the labels' fixed screen size), z = how many lights
    screen: [f32; 4],
    /// the beings' cores as point lights in the room (#204): per light (x, y, z, range), (r, g, b, 0)
    lights: [[f32; 8]; MAX_LIGHTS],
}

/// one label quad for the GPU (64 bytes): where its label hangs (world), its box in pixels around that point (x0, y0,
/// x1, y1; y up), the atlas cell (u0, v0, u1, v1), its colour (premultiplied in the shader)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Glyph {
    anchor: [f32; 3],
    /// 1 = on top of everything (the shader puts it at the near plane), 0 = at its anchor's depth
    top: f32,
    rect: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
}

/// the atlas' font size (px) – drawn a little smaller on the screen
const ATLAS_PX: f32 = 30.0;
/// glyph quads per frame at most: every label's characters + its backing
const GLYPHS: usize = LABELS * (label::CHARS + 1);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct UnitVertex {
    pos: [f32; 3],
}

struct RoomGpu {
    vb: wgpu::Buffer,
    ib: wgpu::Buffer,
    /// (first index, count, its texture's bind group)
    draws: Vec<(u32, u32, usize)>,
    binds: Vec<wgpu::BindGroup>,
}

pub struct SpaceGpu {
    room_pipe: wgpu::RenderPipeline,
    solid_pipe: wgpu::RenderPipeline,
    ghost_pipe: wgpu::RenderPipeline,
    tube_pipe: wgpu::RenderPipeline,
    frame_buf: wgpu::Buffer,
    frame_bind: wgpu::BindGroup,
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    room: Option<RoomGpu>,
    unit_vb: wgpu::Buffer,
    unit_ib: wgpu::Buffer,
    unit_count: u32,
    inst_buf: wgpu::Buffer,
    tube_vb: wgpu::Buffer,
    tube_ib: wgpu::Buffer,
    tube_count: u32,
    depth: wgpu::TextureView,
    spheres: Vec<Sphere>,
    tubes: Vec<TubeVertex>,
    /// the Space generation the buffers are sized for
    generation: u64,
    head_pipe: wgpu::RenderPipeline,
    plate_vb: wgpu::Buffer,
    plate_ib: wgpu::Buffer,
    plate_count: u32,
    head_buf: wgpu::Buffer,
    head_sampler: wgpu::Sampler,
    /// a texture per portrait slot (None until its sprite came), and the blank one for a head without a face
    sprites: Vec<Option<wgpu::BindGroup>>,
    blank: wgpu::BindGroup,
    /// this frame's head layers and their textures (slot, or None = blank); the heads far to near
    layers: Vec<HeadLayer>,
    layer_tex: Vec<Option<usize>>,
    order: Vec<(f32, usize)>,
    label_pipe: wgpu::RenderPipeline,
    label_buf: wgpu::Buffer,
    atlas_bind: wgpu::BindGroup,
    atlas: Atlas,
    labels: Vec<Label>,
    glyphs: Vec<Glyph>,
    /// the post stack (#204, post_gpu.rs) – None when the scene lists no pass
    post: Option<PostGpu>,
    /// #220: the quality rung's knobs this deck renders with (quality.rs – the governor sets them)
    quality: Rung,
}

impl SpaceGpu {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, space: &Space, color: wgpu::TextureFormat, size: (u32, u32)) -> SpaceGpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("space"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("space frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("space room texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let frame_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("space frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("space frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame_buf.as_entire_binding() }],
        });
        let room_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("space room"),
            bind_group_layouts: &[Some(&frame_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let being_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("space being"),
            bind_group_layouts: &[Some(&frame_layout)],
            immediate_size: 0,
        });
        let room_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2];
        let unit_attrs = wgpu::vertex_attr_array![0 => Float32x3];
        let inst_attrs = wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32, 3 => Float32x4, 4 => Float32x4];
        let tube_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let pipe = |label: &str, layout: &wgpu::PipelineLayout, vs: &str, fs: &str, buffers: &[Option<wgpu::VertexBufferLayout>], blend: Option<wgpu::BlendState>, depth_write: bool, cull: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState { module: &module, entry_point: Some(vs), compilation_options: Default::default(), buffers },
                primitive: wgpu::PrimitiveState { cull_mode: cull, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format: color, blend, write_mask: wgpu::ColorWrites::COLOR })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let room_vbl = [Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<RoomVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &room_attrs })];
        let sphere_vbl = [
            Some(wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes: &unit_attrs }),
            Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Sphere>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &inst_attrs }),
        ];
        let tube_vbl = [Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<TubeVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tube_attrs })];
        // scans have no reliable winding: no culling in the room; spheres cull their back (a ghost shows its near rim)
        let room_pipe = pipe("space room", &room_layout, "vs_room", "fs_room", &room_vbl, None, true, None);
        let solid_pipe = pipe("space solid", &being_layout, "vs_sphere", "fs_solid", &sphere_vbl, None, true, Some(wgpu::Face::Back));
        let ghost_pipe = pipe("space ghost", &being_layout, "vs_sphere", "fs_ghost", &sphere_vbl, Some(additive), false, Some(wgpu::Face::Back));
        let tube_pipe = pipe("space tube", &being_layout, "vs_tube", "fs_tube", &tube_vbl, Some(additive), false, None);
        // the heads: premultiplied alpha over, depth written; LessEqual – the mood fading in lies on the old one
        let plate_attrs = wgpu::vertex_attr_array![0 => Float32x2];
        let layer_attrs = wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
        let head_vbl = [
            Some(wgpu::VertexBufferLayout { array_stride: 8, step_mode: wgpu::VertexStepMode::Vertex, attributes: &plate_attrs }),
            Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<HeadLayer>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &layer_attrs }),
        ];
        let head_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("space head"),
            layout: Some(&room_layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_head"), compilation_options: Default::default(), buffers: &head_vbl },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_head"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: color, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::COLOR })],
            }),
            multiview_mask: None,
            cache: None,
        });

        // the labels: a quad per character (six vertices from the vertex index) at its button's depth – a pillar in
        // front hides it, the being's own tubes and ghosts don't (they write no depth)
        let glyph_attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
        let label_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("space label"),
            layout: Some(&room_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_label"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Glyph>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &glyph_attrs })],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_label"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: color, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::COLOR })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let (unit_v, unit_i) = unit_sphere(14, 9);
        let buffer = |label: &str, size: usize, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: size.max(16) as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
        };
        let unit_vb = buffer("space unit sphere", unit_v.len() * 12, wgpu::BufferUsages::VERTEX);
        let unit_ib = buffer("space unit sphere idx", unit_i.len() * 4, wgpu::BufferUsages::INDEX);
        let (plate_v, plate_i) = plate(PLATE);
        let plate_vb = buffer("space face plate", plate_v.len() * 8, wgpu::BufferUsages::VERTEX);
        let plate_ib = buffer("space face plate idx", plate_i.len() * 4, wgpu::BufferUsages::INDEX);
        queue.write_buffer(&plate_vb, 0, bytemuck::cast_slice(&plate_v));
        queue.write_buffer(&plate_ib, 0, bytemuck::cast_slice(&plate_i));
        let (ns, nv) = space.capacity();
        let inst_buf = buffer("space spheres", ns * std::mem::size_of::<Sphere>(), wgpu::BufferUsages::VERTEX);
        let tube_vb = buffer("space tubes", nv * std::mem::size_of::<TubeVertex>(), wgpu::BufferUsages::VERTEX);
        let tube_idx = super::being::tube_indices(space.tubes());
        let tube_ib = buffer("space tube idx", tube_idx.len() * 4, wgpu::BufferUsages::INDEX);
        let heads = space.beings().count();
        let head_buf = buffer("space heads", heads * 3 * std::mem::size_of::<HeadLayer>(), wgpu::BufferUsages::VERTEX);
        let head_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("space head"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let atlas = Atlas::new(ATLAS_PX);
        let atlas_view = texture(device, queue, &Image { levels: vec![(atlas.w, atlas.h, atlas.pixels.clone())] });
        let atlas_bind = tex_bind(device, &tex_layout, &atlas_view, &head_sampler);
        let label_buf = buffer("space labels", GLYPHS * std::mem::size_of::<Glyph>(), wgpu::BufferUsages::VERTEX);
        let blank_view = texture(device, queue, &Image { levels: vec![(1, 1, vec![255; 4])] });
        let blank = tex_bind(device, &tex_layout, &blank_view, &head_sampler);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("space room"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        queue.write_buffer(&unit_vb, 0, bytemuck::cast_slice(&unit_v));
        queue.write_buffer(&unit_ib, 0, bytemuck::cast_slice(&unit_i));
        queue.write_buffer(&tube_ib, 0, bytemuck::cast_slice(&tube_idx));
        SpaceGpu {
            room_pipe,
            solid_pipe,
            ghost_pipe,
            tube_pipe,
            frame_buf,
            frame_bind,
            tex_layout,
            sampler,
            room: None,
            unit_vb,
            unit_ib,
            unit_count: unit_i.len() as u32,
            inst_buf,
            tube_vb,
            tube_ib,
            tube_count: tube_idx.len() as u32,
            depth: depth_view(device, size),
            spheres: Vec::with_capacity(ns),
            tubes: Vec::with_capacity(nv),
            generation: space.generation,
            head_pipe,
            plate_vb,
            plate_ib,
            plate_count: plate_i.len() as u32,
            head_buf,
            head_sampler,
            sprites: Vec::new(),
            blank,
            layers: Vec::with_capacity(heads * 3),
            layer_tex: Vec::with_capacity(heads * 3),
            order: Vec::with_capacity(heads),
            label_pipe,
            label_buf,
            atlas_bind,
            atlas,
            labels: Vec::with_capacity(LABELS),
            glyphs: Vec::with_capacity(GLYPHS),
            post: (!space.post.is_empty()).then(|| {
                let mut p = PostGpu::new(device, &space.post, color);
                p.resize(device, size);
                p
            }),
            quality: Rung::default(),
        }
    }

    /// The post pass `effect` from new WGSL – its prelude `common` and its `source` (the dev mode's hot reload, #204):
    /// its pipelines, or None when the scene doesn't list it. Validation errors go to the device's error handler – the
    /// caller scopes them and installs only what compiled (`install_post`).
    pub fn compile_post(&self, device: &wgpu::Device, effect: Effect, common: &str, source: &str) -> Option<Pipes> {
        self.post.as_ref().map(|p| p.compile(device, effect, common, source))
    }

    pub fn install_post(&mut self, effect: Effect, pipes: Pipes) {
        if let Some(p) = self.post.as_mut() {
            p.install(effect, pipes);
        }
    }

    /// The labels' quads this frame (into the preallocated vectors – no allocation): each label beside its button
    /// toward the camera's right, a backing behind its text, the characters a fixed size on the screen (2.6 % of its
    /// height, at least 14 px).
    fn label_glyphs(&mut self, space: &Space, eye: Vec3, height: f32) {
        space.write_labels(&mut self.labels);
        self.glyphs.clear();
        let (cw, ch) = (self.atlas.cell.0 as f32, self.atlas.cell.1 as f32);
        let solid = self.atlas.uv(label::SOLID);
        for l in &self.labels {
            let pos = Vec3::from(l.pos);
            let right = (pos - eye).cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
            // beside the button, a little toward the eye: its own chain's beads don't cut into it
            let toward = (eye - pos).try_normalize().unwrap_or(Vec3::Z);
            let anchor: [f32; 3] = (pos + right * l.offset + toward * l.offset * 1.5).into();
            let h = (height * 0.026).max(14.0) * l.scale;
            let w = h * cw / ch;
            let y = -l.line * h * 1.05;
            let n = l.text.len as f32;
            let a = l.color[3];
            if self.glyphs.len() + l.text.len + 1 > GLYPHS {
                break;
            }
            let top = if l.top { 1.0 } else { 0.0 };
            let back = if l.top { 0.85 } else { 0.6 };
            self.glyphs.push(Glyph { anchor, top, rect: [-0.3 * w, y - 0.5 * h, (n + 0.3) * w, y + 0.5 * h], uv: solid, color: [0.01, 0.012, 0.03, back * a] });
            for (k, &b) in l.text.bytes[..l.text.len].iter().enumerate() {
                if b == b' ' {
                    continue;
                }
                let x = k as f32 * w;
                self.glyphs.push(Glyph { anchor, top, rect: [x, y - 0.5 * h, x + w, y + 0.5 * h], uv: self.atlas.uv(label::glyph(b)), color: l.color });
            }
        }
    }

    /// Before a frame: the room once it arrived, the sprites the loader finished, and buffers sized anew when beings
    /// came or went (a join – not a frame's work).
    pub fn sync(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, space: &mut Space) {
        if let Some(room) = space.take_set() {
            #[cfg(not(target_arch = "wasm32"))]
            let t = std::time::Instant::now();
            self.upload_room(device, queue, &room);
            #[cfg(not(target_arch = "wasm32"))]
            tracing::info!("room on the GPU in {:.0} ms", t.elapsed().as_secs_f32() * 1000.0);
        }
        for (slot, img) in space.portraits.take() {
            let view = texture(device, queue, &img);
            if self.sprites.len() <= slot {
                self.sprites.resize_with(slot + 1, || None);
            }
            self.sprites[slot] = Some(tex_bind(device, &self.tex_layout, &view, &self.head_sampler));
        }
        if space.generation != self.generation {
            self.generation = space.generation;
            let buffer = |label: &str, size: usize, usage: wgpu::BufferUsages| {
                device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: size.max(16) as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
            };
            let (ns, nv) = space.capacity();
            self.inst_buf = buffer("space spheres", ns * std::mem::size_of::<Sphere>(), wgpu::BufferUsages::VERTEX);
            self.tube_vb = buffer("space tubes", nv * std::mem::size_of::<TubeVertex>(), wgpu::BufferUsages::VERTEX);
            let idx = super::being::tube_indices(space.tubes());
            self.tube_ib = buffer("space tube idx", idx.len() * 4, wgpu::BufferUsages::INDEX);
            queue.write_buffer(&self.tube_ib, 0, bytemuck::cast_slice(&idx));
            self.tube_count = idx.len() as u32;
            let heads = space.beings().count();
            self.head_buf = buffer("space heads", heads * 3 * std::mem::size_of::<HeadLayer>(), wgpu::BufferUsages::VERTEX);
            self.spheres = Vec::with_capacity(ns);
            self.tubes = Vec::with_capacity(nv);
            self.layers = Vec::with_capacity(heads * 3);
            self.layer_tex = Vec::with_capacity(heads * 3);
            self.order = Vec::with_capacity(heads);
        }
    }

    /// the heads' layers this frame, far to near (into the preallocated vectors – no allocation)
    fn heads(&mut self, space: &Space, eye: Vec3) {
        self.layers.clear();
        self.layer_tex.clear();
        self.order.clear();
        for (i, b) in space.beings().enumerate() {
            self.order.push(((b.head.pos - eye).length_squared(), i));
        }
        self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for k in 0..self.order.len() {
            let Some(b) = space.beings().nth(self.order[k].1) else { continue };
            let h = &b.head;
            let flat = Vec3::new(eye.x - h.pos.x, 0.0, eye.z - h.pos.z);
            let fwd = flat.try_normalize().unwrap_or(Vec3::Z);
            let right = Vec3::Y.cross(fwd);
            let have = |s: Option<usize>| s.filter(|&s| self.sprites.get(s).is_some_and(Option::is_some));
            let now = have(space.portraits.pick_near(&h.sprites, h.near.get(h.mood), h.mood));
            let before = have(space.portraits.pick_near(&h.sprites, h.near.get(h.prev), h.prev));
            let layer = |alpha: f32, layer: f32, face: bool| HeadLayer {
                center: h.pos.into(),
                size: h.radius * 1.35,
                right: right.into(),
                bulge: BULGE,
                up: [0.0, 1.0, 0.0],
                alpha,
                fwd: fwd.into(),
                layer,
                extra: [b.glow, b.hue, h.walk.energy, if face { 1.0 } else { 0.0 }],
                more: [b.strain, 0.0, 0.0, 0.0],
            };
            // the machine behind (the silhouette of the face it carries), then the portrait: the old mood under the new
            self.layers.push(layer(1.0, 0.0, now.is_some()));
            self.layer_tex.push(now);
            if h.fade < 1.0 && before.is_some() && before != now {
                self.layers.push(layer(1.0, 1.0, true));
                self.layer_tex.push(before);
            }
            self.layers.push(layer(if before.is_some() && before != now { h.fade } else { 1.0 }, 1.0, now.is_some()));
            self.layer_tex.push(now);
        }
    }

    /// #220: the knobs of a quality rung from the next frame on (the render size is the caller's: `resize`)
    pub fn set_quality(&mut self, rung: Rung) {
        self.quality = rung;
    }

    /// the depth buffer follows the render size
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.depth = depth_view(device, size);
        if let Some(p) = self.post.as_mut() {
            p.resize(device, size);
        }
    }

    /// The room on the GPU: one vertex / index buffer, a texture per image (sRGB, mipmapped on the CPU), a white one
    /// for the untextured draws.
    pub fn upload_room(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, room: &Model) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("room vertices"),
            size: (room.vertices.len() * std::mem::size_of::<RoomVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, bytemuck::cast_slice(&room.vertices));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("room indices"),
            size: (room.indices.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, bytemuck::cast_slice(&room.indices));
        let mut views: Vec<wgpu::TextureView> = room.images.iter().map(|i| texture(device, queue, i)).collect();
        views.push(texture(device, queue, &Image { levels: vec![(1, 1, vec![255; 4])] }));
        let white = views.len() - 1;
        let binds = views.iter().map(|v| tex_bind(device, &self.tex_layout, v, &self.sampler)).collect();
        let draws = room.draws.iter().map(|d| (d.first, d.count, d.image.unwrap_or(white))).collect();
        self.room = Some(RoomGpu { vb, ib, draws, binds });
    }

    /// One frame of the space into `target` (the node's texture, `size` = its size). `time` drives the shimmer.
    pub fn draw(&mut self, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, size: (u32, u32), space: &Space, time: f32) {
        let solid = space.write(&mut self.spheres, &mut self.tubes) as u32;
        let all = self.spheres.len() as u32;
        queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&self.spheres));
        queue.write_buffer(&self.tube_vb, 0, bytemuck::cast_slice(&self.tubes));
        let cam = &space.camera;
        let eye: Vec3 = cam.eye();
        self.heads(space, eye);
        queue.write_buffer(&self.head_buf, 0, bytemuck::cast_slice(&self.layers));
        self.label_glyphs(space, eye, size.1 as f32);
        queue.write_buffer(&self.label_buf, 0, bytemuck::cast_slice(&self.glyphs));
        let mut lights = [[0.0; 8]; MAX_LIGHTS];
        let lit = space.write_lights(eye, &mut lights[..self.quality.lights.min(MAX_LIGHTS)]);
        let u = FrameUniform {
            view_proj: cam.view_proj(size.0 as f32 / size.1.max(1) as f32).to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            look: [time, space.light, space.being.glow, space.being.hue],
            fog: FOG,
            screen: [size.0 as f32, size.1 as f32, lit as f32, 0.0],
            lights,
        };
        queue.write_buffer(&self.frame_buf, 0, bytemuck::bytes_of(&u));
        // the post stack (#204): the space into its first texture when a pass runs this frame, else into the target
        let plan = space.post.plan_quality(&space.post_knobs, self.quality.bloom, self.quality.fxaa);
        let post = self.post.as_ref().filter(|_| !plan.is_empty()).and_then(|p| p.scene().map(|s| (p, s)));
        let final_target = target;
        let target = post.map_or(target, |(_, s)| s);
        let srgb = |c: f32| (c as f64).powf(1.0 / 2.2);
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("space"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target, depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: srgb(FOG[0]), g: srgb(FOG[1]), b: srgb(FOG[2]), a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_bind_group(0, &self.frame_bind, &[]);
        if let Some(room) = &self.room {
            rp.set_pipeline(&self.room_pipe);
            rp.set_vertex_buffer(0, room.vb.slice(..));
            rp.set_index_buffer(room.ib.slice(..), wgpu::IndexFormat::Uint32);
            for &(first, count, bind) in &room.draws {
                rp.set_bind_group(1, &room.binds[bind], &[]);
                rp.draw_indexed(first..first + count, 0, 0..1);
            }
        }
        rp.set_pipeline(&self.solid_pipe);
        rp.set_vertex_buffer(0, self.unit_vb.slice(..));
        rp.set_vertex_buffer(1, self.inst_buf.slice(..));
        rp.set_index_buffer(self.unit_ib.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.unit_count, 0, 0..solid);
        rp.set_pipeline(&self.head_pipe);
        rp.set_vertex_buffer(0, self.plate_vb.slice(..));
        rp.set_vertex_buffer(1, self.head_buf.slice(..));
        rp.set_index_buffer(self.plate_ib.slice(..), wgpu::IndexFormat::Uint32);
        for (i, tex) in self.layer_tex.iter().enumerate() {
            let bind = tex.and_then(|s| self.sprites[s].as_ref()).unwrap_or(&self.blank);
            rp.set_bind_group(1, bind, &[]);
            rp.draw_indexed(0..self.plate_count, 0, i as u32..i as u32 + 1);
        }
        rp.set_pipeline(&self.tube_pipe);
        rp.set_vertex_buffer(0, self.tube_vb.slice(..));
        rp.set_index_buffer(self.tube_ib.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.tube_count, 0, 0..1);
        rp.set_pipeline(&self.ghost_pipe);
        rp.set_vertex_buffer(0, self.unit_vb.slice(..));
        rp.set_vertex_buffer(1, self.inst_buf.slice(..));
        rp.set_index_buffer(self.unit_ib.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.unit_count, 0, solid..all);
        if !self.glyphs.is_empty() {
            rp.set_pipeline(&self.label_pipe);
            rp.set_bind_group(1, &self.atlas_bind, &[]);
            rp.set_vertex_buffer(0, self.label_buf.slice(..));
            rp.draw(0..6, 0..self.glyphs.len() as u32);
        }
        drop(rp);
        if let Some((p, _)) = post {
            p.run(queue, enc, &plan, &space.post_knobs, &space.post, final_target);
        }
    }
}

fn depth_view(device: &wgpu::Device, (w, h): (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("space depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// an sRGB texture with the mip chain the loader made
fn texture(device: &wgpu::Device, queue: &wgpu::Queue, img: &Image) -> wgpu::TextureView {
    let (w, h, _) = &img.levels[0];
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("room image"),
        size: wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        mip_level_count: img.levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (mip, (lw, lh, px)) in img.levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: mip as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(lw * 4), rows_per_image: None },
            wgpu::Extent3d { width: *lw, height: *lh, depth_or_array_layers: 1 },
        );
    }
    tex.create_view(&Default::default())
}

fn tex_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("space texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// the face plate: an `n` × `n` grid over [-1, 1]² (the shader bulges it and turns it to the eye)
fn plate(n: u32) -> (Vec<[f32; 2]>, Vec<u32>) {
    let mut v = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            v.push([i as f32 / n as f32 * 2.0 - 1.0, j as f32 / n as f32 * 2.0 - 1.0]);
        }
    }
    let mut idx = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            let b = a + n + 1;
            idx.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    (v, idx)
}

/// a UV sphere of radius 1: `lon` × `lat` quads (its normals = its positions)
fn unit_sphere(lon: u32, lat: u32) -> (Vec<UnitVertex>, Vec<u32>) {
    let mut v = Vec::new();
    for j in 0..=lat {
        let theta = std::f32::consts::PI * j as f32 / lat as f32;
        for i in 0..=lon {
            let phi = std::f32::consts::TAU * i as f32 / lon as f32;
            v.push(UnitVertex { pos: [theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin()] });
        }
    }
    let mut idx = Vec::new();
    for j in 0..lat {
        for i in 0..lon {
            let a = j * (lon + 1) + i;
            let b = a + lon + 1;
            idx.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    (v, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unit_sphere_faces_outward() {
        let (v, idx) = unit_sphere(14, 9);
        assert!(v.iter().all(|x| (Vec3::from(x.pos).length() - 1.0).abs() < 1e-5));
        let mut outward = 0;
        for t in idx.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from(v[i as usize].pos));
            let n = (b - a).cross(c - a);
            if n.length() > 1e-6 {
                assert!(n.dot(a + b + c) > 0.0, "a triangle facing in");
                outward += 1;
            }
        }
        assert!(outward > 200);
    }
}
