//! The kit's adapter (#456): its draw list as instanced quads in one pass (shaders/ui.wgsl). Shapes are a distance per
//! pixel; glyphs, icons and the logo's frames come from one atlas (IBM Plex Mono, deck-space's font; Feather; the creator's
//! logo); the stage is one textured quad. Premultiplied alpha over the cleared frame, rebuilt per frame.
use ab_glyph::{point, Font, PxScale, ScaleFont};
use anyhow::Result;
use deck_kit::draw::{DrawList, Prim};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// the characters the atlas holds: ASCII, then what German and the panel's symbols need
pub const EXTRA: &str = "äöüÄÖÜß€–·…×→←↑↓°≤≥";

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad {
    rect: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    shape: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Screen {
    size: [f32; 2],
    _pad: [f32; 2],
}

/// the glyph atlas: a cell per character at `px`
pub struct Atlas {
    pub texture: wgpu::Texture,
    pub cell: (f32, f32),
    pub px: f32,
    cells: HashMap<char, u32>,
    cols: u32,
    size: (u32, u32),
    /// an icon's and a logo frame's uv
    icons: HashMap<&'static str, [f32; 4]>,
    frames: Vec<[f32; 4]>,
}

impl Atlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, px: f32) -> Atlas {
        let font = deck_space::space::label::mono();
        let sf = font.as_scaled(PxScale::from(px));
        let chars: Vec<char> = (b' '..=b'~').map(char::from).chain(EXTRA.chars()).collect();
        let cw = sf.h_advance(font.glyph_id('0')).ceil() as u32 + 2;
        let ch = (sf.ascent() - sf.descent()).ceil() as u32 + 2;
        let cols = 16u32;
        let rows = (chars.len() as u32).div_ceil(cols);
        const ICON: u32 = 36;
        let logo = crate::logo::frames();
        let w = (cw * cols).max(10 * crate::logo::SIDE);
        let icon_cols = w / ICON;
        let glyph_h = ch * rows;
        let icons_h = (deck_kit::icons::ICONS.len() as u32).div_ceil(icon_cols) * ICON;
        let logo_cols = w / crate::logo::SIDE;
        let h = glyph_h + icons_h + (crate::logo::FRAMES as u32).div_ceil(logo_cols) * crate::logo::SIDE;
        let mut pixels = vec![255u8; (w * h * 4) as usize];
        pixels.chunks_exact_mut(4).for_each(|p| p[3] = 0);
        let mut cells = HashMap::new();
        for (i, &c) in chars.iter().enumerate() {
            let i = i as u32;
            cells.insert(c, i);
            let (x0, y0) = (i % cols * cw, i / cols * ch);
            // the mono font carries no arrows: the sentences of the core have them
            let shown = match c {
                '→' if font.glyph_id(c).0 == 0 => '>',
                '←' if font.glyph_id(c).0 == 0 => '<',
                other => other,
            };
            let g = font.glyph_id(shown).with_scale_and_position(PxScale::from(px), point(x0 as f32 + 1.0, y0 as f32 + 1.0 + sf.ascent()));
            if let Some(o) = font.outline_glyph(g) {
                let b = o.px_bounds();
                o.draw(|x, y, cov| {
                    let (px, py) = (b.min.x as i32 + x as i32, b.min.y as i32 + y as i32);
                    if px >= x0 as i32 && py >= y0 as i32 && px < (x0 + cw) as i32 && py < (y0 + ch) as i32 {
                        let a = &mut pixels[((py as u32 * w + px as u32) * 4 + 3) as usize];
                        *a = (*a).max((cov.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                });
            }
        }
        let (wf, hf) = (w as f32, h as f32);
        let inset = |x: u32, y: u32, s: u32| [(x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf, ((x + s) as f32 - 0.5) / wf, ((y + s) as f32 - 0.5) / hf];
        // the icons: white with their coverage in alpha, Feather's stroke 2.5 and square caps
        let mut icons = HashMap::new();
        for (i, (name, body)) in deck_kit::icons::ICONS.iter().enumerate() {
            let (x0, y0) = (i as u32 % icon_cols * ICON, glyph_h + i as u32 / icon_cols * ICON);
            let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.5\" stroke-linecap=\"square\" stroke-linejoin=\"miter\">{body}</svg>");
            let scale = (ICON - 4) as f32 / 24.0;
            let mut map = resvg::tiny_skia::Pixmap::new(ICON, ICON).expect("an icon's pixmap");
            if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
                resvg::render(&tree, resvg::tiny_skia::Transform::from_translate(2.0, 2.0).pre_scale(scale, scale), &mut map.as_mut());
            }
            for (k, p) in map.pixels().iter().enumerate() {
                let (px, py) = (x0 + k as u32 % ICON, y0 + k as u32 / ICON);
                pixels[((py * w + px) * 4 + 3) as usize] = p.alpha();
            }
            icons.insert(*name, inset(x0, y0, ICON));
        }
        // the logo's frames, in their own colours
        let side = crate::logo::SIDE;
        let mut frames = Vec::with_capacity(logo.len());
        for (i, f) in logo.iter().enumerate() {
            let (x0, y0) = (i as u32 % logo_cols * side, glyph_h + icons_h + i as u32 / logo_cols * side);
            for (k, p) in f.chunks_exact(4).enumerate() {
                let (px, py) = (x0 + k as u32 % side, y0 + k as u32 / side);
                pixels[((py * w + px) * 4) as usize..((py * w + px) * 4 + 4) as usize].copy_from_slice(p);
            }
            frames.push(inset(x0, y0, side));
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui atlas"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        Atlas { texture, cell: (cw as f32, ch as f32), px, cells, cols, size: (w, h), icons, frames }
    }

    fn uv(&self, c: char, inset: f32) -> [f32; 4] {
        let i = *self.cells.get(&c).or_else(|| self.cells.get(&'?')).expect("'?' is in the atlas");
        let (x0, y0) = ((i % self.cols) as f32 * self.cell.0, (i / self.cols) as f32 * self.cell.1);
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        [(x0 + inset) / w, (y0 + inset) / h, (x0 + self.cell.0 - inset) / w, (y0 + self.cell.1 - inset) / h]
    }

    /// a character's advance on the screen at font size `px`
    pub fn advance(&self, px: f32) -> f32 {
        (self.cell.0 - 2.0) * px / self.px
    }
}

/// The pass: the pipeline, the atlas and the stage bind groups, the quads of this frame.
pub struct Ui {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    screen: wgpu::Buffer,
    pub atlas: Atlas,
    atlas_bind: wgpu::BindGroup,
    stage_bind: Option<wgpu::BindGroup>,
    instances: wgpu::Buffer,
    capacity: usize,
    /// what the kit drew this frame
    list: DrawList,
    /// the last frame's quads
    quads: Vec<Quad>,
    /// the stage's quad, drawn first with its own texture
    stage_quad: Option<Quad>,
}

const FIRST_CAPACITY: usize = 4096;

impl Ui {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, font_px: f32) -> Result<Ui> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("ui"), source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()) });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty, count: None };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui"),
            entries: &[
                entry(0, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
                entry(1, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
                entry(2, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("ui"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let screen = device.create_buffer(&wgpu::BufferDescriptor { label: Some("ui screen"), size: std::mem::size_of::<Screen>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let atlas = Atlas::new(device, queue, font_px * 2.0);
        let list = DrawList::new(atlas.advance(1.0));
        let atlas_bind = bind(device, &layout, &screen, &atlas.texture.create_view(&Default::default()), &sampler);
        let instances = device.create_buffer(&wgpu::BufferDescriptor { label: Some("ui quads"), size: (FIRST_CAPACITY * std::mem::size_of::<Quad>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Ok(Ui { pipeline, layout, sampler, screen, atlas, atlas_bind, stage_bind: None, instances, capacity: FIRST_CAPACITY, list, quads: Vec::with_capacity(FIRST_CAPACITY), stage_quad: None })
    }

    /// the stage's texture (made again when the stage's texture changes)
    pub fn set_stage(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        self.stage_bind = Some(bind(device, &self.layout, &self.screen, view, &self.sampler));
    }

    pub fn begin(&mut self) {
        self.list.clear();
        self.stage_quad = None;
    }

    /// the kit's list as quads
    fn build(&mut self) {
        self.quads.clear();
        const IMAGE: [f32; 4] = [0.0; 4];
        let (atlas, list, quads) = (&self.atlas, &self.list, &mut self.quads);
        let shape = |x: f32, y: f32, w: f32, h: f32, r: f32, kind: f32, color: [f32; 4]| {
            let (hw, hh) = (w * 0.5, h * 0.5);
            Quad { rect: [x, y, w, h], uv: [-hw, -hh, hw, hh], color, shape: [kind, hw, hh, r.min(hw).min(hh)] }
        };
        for p in list.prims() {
            match *p {
                Prim::Rect { x, y, w, h, r, color } => quads.push(shape(x, y, w, h, r, 1.0, color)),
                Prim::Outline { x, y, w, h, r, thick, color } => quads.push(shape(x, y, w, h, r, 1.0 + thick, color)),
                Prim::Icon { x, y, size, name, color } => {
                    if let Some(uv) = atlas.icons.get(name) {
                        quads.push(Quad { rect: [x, y, size, size], uv: *uv, color, shape: IMAGE });
                    }
                }
                Prim::Logo { x, y, size, frame } => {
                    if let Some(uv) = atlas.frames.get(frame) {
                        quads.push(Quad { rect: [x, y, size, size], uv: *uv, color: [1.0; 4], shape: IMAGE });
                    }
                }
                Prim::Text { x, y, px, start, end, color } => {
                    let adv = atlas.advance(px);
                    let (cw, ch) = (atlas.cell.0 * px / atlas.px, atlas.cell.1 * px / atlas.px);
                    let mut cx = x;
                    for c in list.text_of(start, end).chars() {
                        if c != ' ' {
                            quads.push(Quad { rect: [cx - px / atlas.px, y, cw, ch], uv: atlas.uv(c, 0.5), color, shape: IMAGE });
                        }
                        cx += adv;
                    }
                }
            }
        }
    }

    /// the stage into this rectangle
    pub fn stage(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.stage_quad = Some(Quad { rect: [x, y, w, h], uv: [0.0, 0.0, 1.0, 1.0], color: [1.0; 4], shape: [0.0; 4] });
    }

    pub fn quads(&self) -> usize {
        self.quads.len() + usize::from(self.stage_quad.is_some())
    }

    /// draw this frame's quads over `view` (cleared to `clear`); `logical` = the window's size in the pixels the quads
    /// were laid out in (the surface may be larger on a HiDPI screen – the pass scales)
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, logical: (f32, f32), clear: [f32; 4]) {
        self.build();
        queue.write_buffer(&self.screen, 0, bytemuck::bytes_of(&Screen { size: [logical.0, logical.1], _pad: [0.0; 2] }));
        let all: Vec<Quad> = self.stage_quad.iter().copied().chain(self.quads.iter().copied()).collect();
        if all.len() > self.capacity {
            self.capacity = all.len().next_power_of_two();
            self.instances = device.create_buffer(&wgpu::BufferDescriptor { label: Some("ui quads"), size: (self.capacity * std::mem::size_of::<Quad>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        }
        if !all.is_empty() {
            queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&all));
        }
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: f64::from(clear[0]), g: f64::from(clear[1]), b: f64::from(clear[2]), a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        rp.set_pipeline(&self.pipeline);
        rp.set_vertex_buffer(0, self.instances.slice(..));
        let mut at = 0u32;
        if let (Some(_), Some(b)) = (&self.stage_quad, &self.stage_bind) {
            rp.set_bind_group(0, b, &[]);
            rp.draw(0..6, 0..1);
            at = 1;
        } else if self.stage_quad.is_some() {
            at = 1;
        }
        let n = self.quads.len() as u32;
        if n > 0 {
            rp.set_bind_group(0, &self.atlas_bind, &[]);
            rp.draw(0..6, at..at + n);
        }
    }
}

impl Deref for Ui {
    type Target = DrawList;
    fn deref(&self) -> &DrawList {
        &self.list
    }
}

impl DerefMut for Ui {
    fn deref_mut(&mut self) -> &mut DrawList {
        &mut self.list
    }
}

fn bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, screen: &wgpu::Buffer, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ui"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: screen.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}
