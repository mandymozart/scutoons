//! The post stack on the GPU (#204; the data side: post.rs). Fragment passes only (WebGL2 has no compute): every pass
//! is a fullscreen triangle reading the last one's picture. Two textures at the render size take turns – the space
//! draws into the first, pass 1 reads it and writes the second, pass 2 writes the first again … – and the last pass
//! writes the space node's target itself, so the stack costs no copy. Bloom adds three small textures (half, quarter,
//! eighth size) and five draws at those sizes. One uniform buffer for all passes, written once a frame.
//!
//! Nothing is made for a pass the scene doesn't list, nothing is made at all for an empty `post`, and a frame whose
//! listed passes all stand at neutral draws the space straight into its target (post.rs `Chain::plan`). The textures
//! are made at `resize` (the render size changes), never per frame.
//!
//! `compile` + `install` rebuild one pass from new WGSL (the stage's `--dev` hot reload wraps them in an error scope:
//! a pass that doesn't compile keeps the running one).
use super::post::{Chain, Effect, Knobs, Plan, COMMON, MAX, PALETTE};

/// the format of the textures between passes (display values, like the stage's offscreen targets)
const INTER: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// bloom's levels below the picture: half, quarter, eighth
const LEVELS: usize = 3;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    bloom: [f32; 4],
    curves: [f32; 4],
    pixel: [f32; 4],
    palette: [[f32; 4]; PALETTE],
}

/// one pass's pipelines: into the next texture (`mid`), into the space node's target (`last` – its format may differ:
/// the browser's canvas), and bloom's own chain
pub struct Pipes {
    mid: wgpu::RenderPipeline,
    last: wgpu::RenderPipeline,
    bloom: Option<BloomPipes>,
}

struct BloomPipes {
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    /// additive onto the level above
    up: wgpu::RenderPipeline,
}

/// the textures at the render size and the bind groups that read them
struct Targets {
    size: (u32, u32),
    /// the space draws into `views[0]`; the passes take turns between the two
    views: [wgpu::TextureView; 2],
    /// bloom's levels (half, quarter, eighth)
    levels: [wgpu::TextureView; LEVELS],
    /// per picture texture: read alone, and read with bloom's half level (its composite)
    plain: [wgpu::BindGroup; 2],
    composite: [wgpu::BindGroup; 2],
    /// per bloom level: read alone
    level_binds: [wgpu::BindGroup; LEVELS],
}

pub struct PostGpu {
    layout: wgpu::BindGroupLayout,
    pipe_layout: wgpu::PipelineLayout,
    ubuf: wgpu::Buffer,
    sampler: wgpu::Sampler,
    dummy: wgpu::TextureView,
    target: wgpu::TextureFormat,
    /// per Effect (its index in Effect::ALL): None = not listed
    pipes: [Option<Pipes>; MAX],
    targets: Option<Targets>,
    bloom_used: bool,
}

fn index(e: Effect) -> usize {
    Effect::ALL.iter().position(|&x| x == e).unwrap_or(0)
}

impl PostGpu {
    /// The scene's stack for a target of format `target` (the space node's texture, the browser's canvas): the listed
    /// passes compiled from their built-in WGSL. The textures come with the first `resize`.
    pub fn new(device: &wgpu::Device, chain: &Chain, target: wgpu::TextureFormat) -> PostGpu {
        let tex = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty, count: None };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                entry(0, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
                entry(1, tex),
                entry(2, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                entry(3, tex),
            ],
        });
        let pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("post"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dummy = make_texture(device, (1, 1), wgpu::TextureUsages::TEXTURE_BINDING);
        let mut post = PostGpu { layout, pipe_layout, ubuf, sampler, dummy, target, pipes: [None, None, None, None], targets: None, bloom_used: chain.contains(Effect::Bloom) };
        for e in chain.effects() {
            let p = post.compile(device, e, COMMON, e.source());
            post.install(e, p);
        }
        post
    }

    /// The pipelines of `effect` from `source` (its WGSL) after the prelude `common` (post.rs `COMMON`, or the file
    /// when it reloads). Validation errors go to the device's error handler – the stage's hot reload pushes an error
    /// scope around this and installs only what compiled.
    pub fn compile(&self, device: &wgpu::Device, effect: Effect, common: &str, source: &str) -> Pipes {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(effect.name()),
            source: wgpu::ShaderSource::Wgsl(format!("{common}\n{source}").into()),
        });
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent::REPLACE,
        };
        let pipe = |fs: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(effect.name()),
                layout: Some(&self.pipe_layout),
                vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview: None,
                cache: None,
            })
        };
        let fs = if effect == Effect::Bloom { "fs_composite" } else { "fs" };
        let mid = pipe(fs, INTER, None);
        let last = if self.target == INTER { mid.clone() } else { pipe(fs, self.target, None) };
        let bloom = (effect == Effect::Bloom).then(|| BloomPipes { prefilter: pipe("fs_prefilter", INTER, None), down: pipe("fs_down", INTER, None), up: pipe("fs_up", INTER, Some(additive)) });
        Pipes { mid, last, bloom }
    }

    /// `effect`'s pipelines from now on (only for a listed pass – the textures were made for the listed ones)
    pub fn install(&mut self, effect: Effect, pipes: Pipes) {
        self.pipes[index(effect)] = Some(pipes);
    }

    /// The textures at the render size (only when it changed – a resize, the first frame).
    pub fn resize(&mut self, device: &wgpu::Device, (w, h): (u32, u32)) {
        let size = (w.max(1), h.max(1));
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let views = [make_texture(device, size, usage), make_texture(device, size, usage)];
        // bloom's levels: tiny placeholders when the stack has no bloom
        let level = |k: u32| if self.bloom_used { ((size.0 >> k).max(1), (size.1 >> k).max(1)) } else { (1, 1) };
        let levels = [make_texture(device, level(1), usage), make_texture(device, level(2), usage), make_texture(device, level(3), usage)];
        let bind = |a: &wgpu::TextureView, b: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.ubuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(a) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(b) },
                ],
            })
        };
        let plain = [bind(&views[0], &self.dummy), bind(&views[1], &self.dummy)];
        let composite = [bind(&views[0], &levels[0]), bind(&views[1], &levels[0])];
        let level_binds = [bind(&levels[0], &self.dummy), bind(&levels[1], &self.dummy), bind(&levels[2], &self.dummy)];
        self.targets = Some(Targets { size, views, levels, plain, composite, level_binds });
    }

    /// where the space draws when the stack runs this frame (None before the first `resize`)
    pub fn scene(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.views[0])
    }

    /// This frame's passes over what the space drew into `scene()`, the last into `target`. Returns false when there
    /// was nothing to do (an empty plan, no textures yet) – the caller then draws straight into `target`.
    pub fn run(&self, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, plan: &Plan, k: &Knobs, chain: &Chain, target: &wgpu::TextureView) -> bool {
        let Some(t) = &self.targets else { return false };
        let passes = plan.passes();
        if passes.is_empty() || passes.iter().any(|&e| self.pipes[index(e)].is_none()) {
            return false;
        }
        let u = Uniform {
            bloom: [k.bloom, k.threshold, (k.threshold * 0.5).max(0.05), 0.0],
            curves: [k.lift, k.gamma, k.gain, 0.0],
            pixel: [k.pixel.max(1.0), k.levels, chain.palette_len as f32, 0.0],
            palette: chain.palette,
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let mut cur = 0;
        for (i, &e) in passes.iter().enumerate() {
            let last = i + 1 == passes.len();
            let Some(p) = &self.pipes[index(e)] else { continue };
            let (dst, pipe) = if last { (target, &p.last) } else { (&t.views[1 - cur], &p.mid) };
            if let Some(b) = &p.bloom {
                // the bright part at half size, down to an eighth, back up – each level added onto the one above
                draw(enc, &b.prefilter, &t.plain[cur], &t.levels[0], true);
                draw(enc, &b.down, &t.level_binds[0], &t.levels[1], true);
                draw(enc, &b.down, &t.level_binds[1], &t.levels[2], true);
                draw(enc, &b.up, &t.level_binds[2], &t.levels[1], false);
                draw(enc, &b.up, &t.level_binds[1], &t.levels[0], false);
                draw(enc, pipe, &t.composite[cur], dst, true);
            } else {
                draw(enc, pipe, &t.plain[cur], dst, true);
            }
            cur = 1 - cur;
        }
        true
    }
}

/// one fullscreen triangle into `dst` (`clear`: its old content isn't needed; else drawn onto it)
fn draw(enc: &mut wgpu::CommandEncoder, pipe: &wgpu::RenderPipeline, bind: &wgpu::BindGroup, dst: &wgpu::TextureView, clear: bool) {
    let load = if clear { wgpu::LoadOp::Clear(wgpu::Color::BLACK) } else { wgpu::LoadOp::Load };
    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("post"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: dst, resolve_target: None, ops: wgpu::Operations { load, store: wgpu::StoreOp::Store } })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    rp.set_pipeline(pipe);
    rp.set_bind_group(0, bind, &[]);
    rp.draw(0..3, 0..1);
}

fn make_texture(device: &wgpu::Device, (w, h): (u32, u32), usage: wgpu::TextureUsages) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("post"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: INTER,
            usage,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
