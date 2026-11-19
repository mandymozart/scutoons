//! A video's frames on the GPU: each frame written into the textures that are there (made again only when the size or
//! the format changes); AV1's planes and HAP Q's YCoCg blocks turned into RGB by a pass (shaders/video.wgsl), HAP's
//! BC1 / BC3 blocks sampled as they are where the device samples BC; a frame smaller than its place upscaled by FSR 1
//! (EASU, then RCAS) when the caller asks for a size.
use deck_media::{Format, Picture};
use std::collections::HashMap;
use wgpu::TextureFormat as T;

/// RCAS's sharpening in stops (0 = the most; AMD's default 0.2)
const SHARPNESS_STOPS: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Stage {
    Yuv,
    YCoCg,
    Easu,
    Rcas,
}

impl Stage {
    fn entry(self) -> &'static str {
        match self {
            Stage::Yuv => "fs_yuv",
            Stage::YCoCg => "fs_ycocg",
            Stage::Easu => "fs_easu",
            Stage::Rcas => "fs_rcas",
        }
    }
}

/// The passes' shader, layout and sampler, once per device; their pipelines made on first use per target format.
pub struct Converter {
    module: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<(Stage, T), wgpu::RenderPipeline>,
}

impl Converter {
    pub fn new(device: &wgpu::Device) -> Converter {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("video"), source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/video.wgsl").into()) });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty, count: None };
        let tex = wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("video"),
            entries: &[
                entry(0, tex),
                entry(1, tex),
                entry(2, tex),
                entry(3, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                entry(4, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("video"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("video"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Converter { module, layout, pipeline_layout, sampler, pipelines: HashMap::new() }
    }

    fn pipeline(&mut self, device: &wgpu::Device, stage: Stage, format: T) -> wgpu::RenderPipeline {
        let (module, layout) = (&self.module, &self.pipeline_layout);
        self.pipelines
            .entry((stage, format))
            .or_insert_with(|| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(stage.entry()),
                    layout: Some(layout),
                    vertex: wgpu::VertexState { module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some(stage.entry()),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .clone()
    }
}

/// What a texture set is made for: a change makes it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    width: u32,
    height: u32,
    format: Format,
    bc: bool,
    srgb: bool,
    /// the upscaled size (FSR), none: the frame's own
    up: Option<(u32, u32)>,
}

struct Pass {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    target: wgpu::TextureView,
}

/// A video's textures: what its frames are written into (the planes, the blocks or RGBA), the passes after that, and
/// `view`, the picture the stage samples.
pub struct Texture {
    pub view: wgpu::TextureView,
    key: Key,
    sources: Vec<wgpu::Texture>,
    passes: Vec<Pass>,
    _targets: Vec<wgpu::Texture>,
}

impl Texture {
    /// the picture's size on the GPU (upscaled or not)
    pub fn size(&self) -> (u32, u32) {
        self.key.up.unwrap_or((self.key.width, self.key.height))
    }
}

/// The size FSR upscales a `w` × `h` frame to so it covers `slot` (both sides at least the slot's) – none when the frame
/// is as large already, or `fsr` is off (the quality rung: off on low).
pub fn upscale_to(w: u32, h: u32, slot: (u32, u32), fsr: bool) -> Option<(u32, u32)> {
    let s = (slot.0 as f32 / w.max(1) as f32).max(slot.1 as f32 / h.max(1) as f32);
    let side = |v: u32| ((v as f32 * s).round() as u32).min(crate::quality::MAX_SIDE);
    (fsr && s > 1.05).then(|| (side(w), side(h)))
}

fn make(device: &wgpu::Device, label: &str, (width, height): (u32, u32), format: T, target: bool) -> wgpu::Texture {
    let usage = if target { wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC } else { wgpu::TextureUsages::COPY_DST };
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: usage | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// a texture's format and size
type Made = (T, (u32, u32));

/// how a frame of this kind goes up: (the source textures, the pass that turns them into RGB)
fn sources(key: &Key) -> (Vec<Made>, Option<Stage>) {
    let (w, h) = (key.width, key.height);
    // straight to the stage (no pass at all): encoded as the stage samples it
    let direct = key.up.is_none();
    let rgba = if direct && key.srgb { T::Rgba8UnormSrgb } else { T::Rgba8Unorm };
    match (key.format, key.bc) {
        (Format::Yuv420 { .. }, _) => {
            let c = (w.div_ceil(2), h.div_ceil(2));
            (vec![(T::R8Unorm, (w, h)), (T::R8Unorm, c), (T::R8Unorm, c)], Some(Stage::Yuv))
        }
        (Format::Bc3YCoCg, true) => (vec![(T::Bc3RgbaUnorm, (w, h))], Some(Stage::YCoCg)),
        (Format::Bc1, true) => (vec![(if direct && key.srgb { T::Bc1RgbaUnormSrgb } else { T::Bc1RgbaUnorm }, (w, h))], None),
        (Format::Bc3, true) => (vec![(if direct && key.srgb { T::Bc3RgbaUnormSrgb } else { T::Bc3RgbaUnorm }, (w, h))], None),
        _ => (vec![(rgba, (w, h))], None),
    }
}

impl Texture {
    fn new(device: &wgpu::Device, conv: &mut Converter, key: Key) -> Texture {
        let (formats, convert) = sources(&key);
        let sources: Vec<wgpu::Texture> = formats.iter().map(|&(f, size)| make(device, "video frame", size, f, false)).collect();
        let last = if key.srgb { T::Rgba8UnormSrgb } else { T::Rgba8Unorm };
        let own = (key.width, key.height);
        // the chain: [convert] → [EASU → RCAS]; the last pass writes `last`, the others gamma-encoded RGBA
        let mut stages: Vec<(Stage, (u32, u32))> = convert.map(|s| (s, own)).into_iter().collect();
        if let Some(up) = key.up {
            stages.extend([(Stage::Easu, up), (Stage::Rcas, up)]);
        }
        let mut input: Vec<wgpu::TextureView> = sources.iter().map(|t| t.create_view(&Default::default())).collect();
        let mut passes = Vec::new();
        let mut keep = Vec::new();
        for (i, &(stage, size)) in stages.iter().enumerate() {
            let end = i + 1 == stages.len();
            let format = if end { last } else { T::Rgba8Unorm };
            let target = make(device, stage.entry(), size, format, true);
            let mut u = [0f32; 12];
            if let Format::Yuv420 { matrix, full } = key.format {
                let ([a, rv, gu, gv, bu], black) = matrix.coefficients(full);
                u[..6].copy_from_slice(&[a as f32 / 256.0, rv as f32 / 256.0, gu as f32 / 256.0, gv as f32 / 256.0, bu as f32 / 256.0, black as f32]);
            }
            u[8..].copy_from_slice(&[if end && key.srgb { 1.0 } else { 0.0 }, (-SHARPNESS_STOPS).exp2(), size.0 as f32, size.1 as f32]);
            let ubuf = wgpu::util::DeviceExt::create_buffer_init(device, &wgpu::util::BufferInitDescriptor { label: Some("video"), contents: bytemuck::cast_slice(&u), usage: wgpu::BufferUsages::UNIFORM });
            let t = |k: usize| &input[k.min(input.len() - 1)];
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("video"),
                layout: &conv.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(t(0)) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(t(1)) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(t(2)) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&conv.sampler) },
                    wgpu::BindGroupEntry { binding: 4, resource: ubuf.as_entire_binding() },
                ],
            });
            let view = target.create_view(&Default::default());
            passes.push(Pass { pipeline: conv.pipeline(device, stage, format), bind, target: view.clone() });
            input = vec![view];
            keep.push(target);
        }
        let view = input.pop().expect("a source or a pass");
        Texture { view, key, sources, passes, _targets: keep }
    }
}

/// Frame `pic` into `slot`'s textures – the ones there when the frame's size and format match, else new ones (then
/// true: a bind group over the old view is stale). `srgb`: the picture is sampled as sRGB (the space's pictures) or as
/// it is (the stage's passes); `up`: FSR's size (`upscale_to`), none = the frame's own. The passes go to the queue
/// with the frame.
pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, conv: &mut Converter, pic: &Picture, srgb: bool, up: Option<(u32, u32)>, slot: &mut Option<Texture>) -> bool {
    let bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC) && pic.width % 4 == 0 && pic.height % 4 == 0;
    let key = Key { width: pic.width, height: pic.height, format: pic.format, bc, srgb, up };
    let fresh = slot.as_ref().is_none_or(|t| t.key != key);
    if fresh {
        *slot = Some(Texture::new(device, conv, key));
    }
    let t = slot.as_ref().expect("made above");
    let write = |tex: &wgpu::Texture, data: &[u8], row: u32, rows: u32| {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(rows) },
            tex.size(),
        );
    };
    match (pic.planes(), pic.compressed() && bc) {
        (Some(planes), _) => {
            for (tex, (data, w, h)) in t.sources.iter().zip(planes) {
                write(tex, data, w, h);
            }
        }
        (None, true) => write(&t.sources[0], &pic.data, pic.width / 4 * if pic.format == Format::Bc1 { 8 } else { 16 }, pic.height / 4),
        (None, false) if pic.format == Format::Rgba8 => write(&t.sources[0], &pic.data, pic.width * 4, pic.height),
        (None, false) => write(&t.sources[0], &pic.to_rgba().data, pic.width * 4, pic.height),
    }
    if !t.passes.is_empty() {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("video") });
        for p in &t.passes {
            let mut rp = crate::stage::begin(&mut enc, &p.target);
            rp.set_pipeline(&p.pipeline);
            rp.set_bind_group(0, &p.bind, &[]);
            rp.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
    }
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_media::yuv::Matrix;

    /// a device (with BC where the adapter has it); none on a machine without an adapter – the test then says so
    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let required_features = adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("video test"), required_features, ..Default::default() })).ok()
    }

    /// the RGBA8 of `t`'s picture (`Rgba8Unorm`), read back
    fn read(device: &wgpu::Device, queue: &wgpu::Queue, t: &Texture) -> Vec<u8> {
        let tex = t.view.texture();
        let (w, h) = (tex.width(), tex.height());
        let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: u64::from(row * h), usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(tex.as_image_copy(), wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } }, tex.size());
        queue.submit([enc.finish()]);
        buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = buf.slice(..).get_mapped_range().unwrap();
        (0..h).flat_map(|y| data[(y * row) as usize..][..(w * 4) as usize].to_vec()).collect()
    }

    fn worst(a: &[u8], b: &[u8]) -> u8 {
        a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
    }

    /// the first frame of a fixture clip
    fn first(name: &str) -> Picture {
        let bytes = std::fs::read(format!("{}/../media/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let mut m = crate::video::Movie::open(bytes).unwrap();
        (*m.next_picture(0.0).unwrap()).clone()
    }

    #[test]
    fn the_planes_become_the_cpu_s_rgb_on_the_gpu() {
        let Some((device, queue)) = device() else { return eprintln!("no GPU adapter: skipped") };
        let mut conv = Converter::new(&device);
        // chroma flat (the CPU picks its texel, the GPU samples between them): the same RGB within 1
        let (w, h) = (8u32, 6u32);
        let y: Vec<u8> = (0..w * h).map(|i| (16 + i * 5 % 220) as u8).collect();
        let (u, v) = (vec![90u8; 12], vec![170u8; 12]);
        for (matrix, full) in [(Matrix::Bt709, false), (Matrix::Bt601, true)] {
            let pic = Picture { width: w, height: h, format: Format::Yuv420 { matrix, full }, data: [&y[..], &u, &v].concat() };
            let mut slot = None;
            assert!(upload(&device, &queue, &mut conv, &pic, false, None, &mut slot));
            let got = read(&device, &queue, slot.as_ref().unwrap());
            assert!(worst(&got, &pic.to_rgba().data) <= 1, "{matrix:?} full {full}");
            assert!(!upload(&device, &queue, &mut conv, &pic, false, None, &mut slot), "the next frame into the same textures");
        }
        // the AV1 fixture's frame: close to the CPU's (chroma interpolated, not picked)
        let pic = first("av1.mp4");
        assert!(matches!(pic.format, Format::Yuv420 { .. }));
        let mut slot = None;
        upload(&device, &queue, &mut conv, &pic, false, None, &mut slot);
        let (got, want) = (read(&device, &queue, slot.as_ref().unwrap()), pic.to_rgba().data);
        let mean = got.iter().zip(&want).map(|(a, b)| f64::from(a.abs_diff(*b))).sum::<f64>() / got.len() as f64;
        assert!(mean < 6.0, "mean difference {mean:.2}");
    }

    #[test]
    fn hap_q_s_ycocg_becomes_the_cpu_s_rgb_on_the_gpu() {
        let Some((device, queue)) = device() else { return eprintln!("no GPU adapter: skipped") };
        if !device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC) {
            return eprintln!("no BC sampling: HAP Q goes up as RGBA, skipped");
        }
        let pic = first("hap_q.mov");
        assert_eq!(pic.format, Format::Bc3YCoCg);
        let (mut conv, mut slot) = (Converter::new(&device), None);
        upload(&device, &queue, &mut conv, &pic, false, None, &mut slot);
        let got = read(&device, &queue, slot.as_ref().unwrap());
        let want = pic.to_rgba().data;
        let mean = got.iter().zip(&want).map(|(a, b)| f64::from(a.abs_diff(*b))).sum::<f64>() / got.len() as f64;
        // a GPU interpolates the blocks' colours its own way (the scale magnifies it): a few steps at most
        assert!(worst(&got, &want) <= 6 && mean < 0.5, "worst {} mean {mean:.3}", worst(&got, &want));
    }

    #[test]
    fn fsr_upscales_to_the_place_and_never_rings() {
        assert_eq!(upscale_to(64, 48, (1280, 720), true), Some((1280, 960)), "covers the place");
        assert_eq!(upscale_to(64, 48, (1280, 720), false), None, "off on the low rung");
        assert_eq!(upscale_to(1920, 1080, (1280, 720), true), None, "large enough already");
        let Some((device, queue)) = device() else { return eprintln!("no GPU adapter: skipped") };
        // a hard edge: black | white; upscaled 4× it stays within black..white and the edge stays where it was
        let (w, h) = (16u32, 8u32);
        let data: Vec<u8> = (0..w * h).flat_map(|i| if i % w < w / 2 { [0, 0, 0, 255] } else { [255, 255, 255, 255] }).collect();
        let pic = Picture { width: w, height: h, format: Format::Rgba8, data };
        let (mut conv, mut slot) = (Converter::new(&device), None);
        upload(&device, &queue, &mut conv, &pic, false, Some((64, 32)), &mut slot);
        let t = slot.as_ref().unwrap();
        assert_eq!(t.size(), (64, 32));
        let got = read(&device, &queue, t);
        let at = |x: usize, y: usize| got[(y * 64 + x) * 4];
        for y in 0..32 {
            assert!(at(4, y) < 8 && at(27, y) < 8, "black side, row {y}");
            assert!(at(36, y) > 247 && at(60, y) > 247, "white side, row {y}");
        }
    }
}
