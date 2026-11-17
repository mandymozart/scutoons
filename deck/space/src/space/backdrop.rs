//! The stage's background on the space (deck-catalog `Background`, the one that won): a colour is what the space clears
//! to and fades into; an image is laid over the drawn stage, cover-fit (deck-core `fit::cover`), its transparency kept.
//! Its picture comes through the portraits' loader (`Picture::media`). With no colour set the space clears to its ground
//! (black; the editor's `stage` token). A video does not play yet: the ground, said once.
use crate::cast::Picture;
use crate::Background;

use super::portrait::Portraits;

#[derive(Debug, Clone, Default)]
pub struct Backdrop {
    pub background: Option<Background>,
    /// the image's slot in the portraits
    pub slot: Option<usize>,
    /// what shows when no background sets a colour, display values
    pub ground: [f32; 3],
}

impl Backdrop {
    pub fn new(background: Option<Background>, portraits: &mut Portraits) -> Backdrop {
        let slot = background.as_ref().and_then(|b| b.image.as_deref()).map(|p| portraits.picture(Picture::media(p)));
        if let Some(v) = background.as_ref().and_then(|b| b.video.as_deref()) {
            // TODO(#326): play it on the core's own video playback
            tracing::warn!("background: the video '{v}' does not play on this deck yet – black");
        }
        Backdrop { background, slot, ground: [0.0; 3] }
    }

    /// what the stage clears to, display values: the colour, else the ground
    pub fn clear(&self) -> [f32; 3] {
        self.background.as_ref().and_then(Background::rgb).unwrap_or(self.ground)
    }

    /// the image's slot and its texture coordinates on a `size` stage, once its picture is here
    pub fn image(&self, portraits: &Portraits, size: (u32, u32)) -> Option<(usize, [f32; 4])> {
        let slot = self.slot.filter(|&s| portraits.ready(s))?;
        let aspect = portraits.slots[slot].aspect;
        Some((slot, deck_core::fit::cover((aspect, 1.0), (size.0 as f32, size.1.max(1) as f32))))
    }
}

#[cfg(feature = "gpu")]
pub use gpu::BackdropGpu;

#[cfg(feature = "gpu")]
mod gpu {
    const SHADER: &str = r"
struct U { rect: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: V;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, u.rect.xy + v.uv * u.rect.zw);
    return vec4<f32>(pow(c.rgb, vec3<f32>(1.0 / 2.2)), c.a);
}
";

    /// the image's pass over the drawn stage
    pub struct BackdropGpu {
        pipe: wgpu::RenderPipeline,
        buf: wgpu::Buffer,
        bind: wgpu::BindGroup,
    }

    impl BackdropGpu {
        /// `textures`: the layout of a sprite's bind group (texture, sampler)
        pub fn new(device: &wgpu::Device, textures: &wgpu::BindGroupLayout, format: wgpu::TextureFormat) -> BackdropGpu {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("backdrop"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("backdrop"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                }],
            });
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("backdrop"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("backdrop"), layout: &layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] });
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("backdrop"), bind_group_layouts: &[Some(&layout), Some(textures)], immediate_size: 0 });
            let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("backdrop"),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
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
            BackdropGpu { pipe, buf, bind }
        }

        /// the picture (`texture`, its sprite bind group) over `target`, shown in `rect` of it
        pub fn draw(&self, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, texture: &wgpu::BindGroup, rect: [f32; 4]) {
            queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&rect));
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("backdrop"),
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
            rp.set_pipeline(&self.pipe);
            rp.set_bind_group(0, &self.bind, &[]);
            rp.set_bind_group(1, texture, &[]);
            rp.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_clears_an_image_waits_for_its_picture() {
        let mut portraits = Portraits::manual();
        assert_eq!(Backdrop::new(None, &mut portraits).clear(), [0.0; 3], "none: black");
        assert_eq!(Backdrop::new(Some(Background::colour("#fff")), &mut portraits).clear(), [1.0; 3]);
        let b = Backdrop::new(Some(Background::image("backgrounds/wide.png")), &mut portraits);
        assert_eq!(b.clear(), [0.0; 3], "an image lies over black");
        assert_eq!(portraits.requests().len(), 1, "its picture is asked for");
        assert!(b.image(&portraits, (1600, 900)).is_none(), "not here yet");
        portraits.deliver(b.slot.unwrap(), Ok(image::RgbaImage::new(200, 100)));
        portraits.poll();
        let (_, rect) = b.image(&portraits, (1600, 900)).unwrap();
        assert!((rect[2] - 0.888_888_9).abs() < 1e-5 && rect[3] == 1.0, "too wide: the height fits, {rect:?}");
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
}
