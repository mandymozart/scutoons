//! The space node on the GPU (#188): its own depth buffer at the render size; per frame the room (one vertex / index
//! buffer, a draw per texture, unlit, depth-written), the being's solid spheres (depth-written), then its tubes and
//! ghost spheres (additive, depth-tested only – the order among them doesn't matter). The sphere and tube buffers are
//! sized once from the being (`Being::capacity`) and rewritten per frame from preallocated vectors.
use super::being::{Sphere, TubeVertex};
use super::glb::{Image, Room, RoomVertex};
use super::Space;
use glam::Vec3;

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
}

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
            bind_group_layouts: &[&frame_layout, &tex_layout],
            push_constant_ranges: &[],
        });
        let being_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("space being"),
            bind_group_layouts: &[&frame_layout],
            push_constant_ranges: &[],
        });
        let room_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2];
        let unit_attrs = wgpu::vertex_attr_array![0 => Float32x3];
        let inst_attrs = wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32, 3 => Float32x4, 4 => Float32x4];
        let tube_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let pipe = |label: &str, layout: &wgpu::PipelineLayout, vs: &str, fs: &str, buffers: &[wgpu::VertexBufferLayout], blend: Option<wgpu::BlendState>, depth_write: bool, cull: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState { module: &module, entry_point: Some(vs), compilation_options: Default::default(), buffers },
                primitive: wgpu::PrimitiveState { cull_mode: cull, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::Less,
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
                multiview: None,
                cache: None,
            })
        };
        let room_vbl = [wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<RoomVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &room_attrs }];
        let sphere_vbl = [
            wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes: &unit_attrs },
            wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Sphere>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &inst_attrs },
        ];
        let tube_vbl = [wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<TubeVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tube_attrs }];
        // scans have no reliable winding: no culling in the room; spheres cull their back (a ghost shows its near rim)
        let room_pipe = pipe("space room", &room_layout, "vs_room", "fs_room", &room_vbl, None, true, None);
        let solid_pipe = pipe("space solid", &being_layout, "vs_sphere", "fs_solid", &sphere_vbl, None, true, Some(wgpu::Face::Back));
        let ghost_pipe = pipe("space ghost", &being_layout, "vs_sphere", "fs_ghost", &sphere_vbl, Some(additive), false, Some(wgpu::Face::Back));
        let tube_pipe = pipe("space tube", &being_layout, "vs_tube", "fs_tube", &tube_vbl, Some(additive), false, None);

        let (unit_v, unit_i) = unit_sphere(14, 9);
        let buffer = |label: &str, size: usize, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: size.max(16) as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
        };
        let unit_vb = buffer("space unit sphere", unit_v.len() * 12, wgpu::BufferUsages::VERTEX);
        let unit_ib = buffer("space unit sphere idx", unit_i.len() * 4, wgpu::BufferUsages::INDEX);
        let (ns, nv) = space.being.capacity();
        let inst_buf = buffer("space spheres", ns * std::mem::size_of::<Sphere>(), wgpu::BufferUsages::VERTEX);
        let tube_vb = buffer("space tubes", nv * std::mem::size_of::<TubeVertex>(), wgpu::BufferUsages::VERTEX);
        let tube_idx = space.being.tube_indices();
        let tube_ib = buffer("space tube idx", tube_idx.len() * 4, wgpu::BufferUsages::INDEX);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("space room"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
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
        }
    }

    /// the depth buffer follows the render size
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.depth = depth_view(device, size);
    }

    /// The room on the GPU: one vertex / index buffer, a texture per image (sRGB, mipmapped on the CPU), a white one
    /// for the untextured draws.
    pub fn upload_room(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, room: &Room) {
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
        let binds = views
            .iter()
            .map(|v| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("room texture"),
                    layout: &self.tex_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(v) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    ],
                })
            })
            .collect();
        let draws = room.draws.iter().map(|d| (d.first, d.count, d.image.unwrap_or(white))).collect();
        self.room = Some(RoomGpu { vb, ib, draws, binds });
    }

    /// One frame of the space into `target` (the node's texture, `size` = its size). `time` drives the shimmer.
    pub fn draw(&mut self, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, size: (u32, u32), space: &Space, time: f32) {
        let solid = space.being.write(&mut self.spheres, &mut self.tubes) as u32;
        let all = self.spheres.len() as u32;
        queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&self.spheres));
        queue.write_buffer(&self.tube_vb, 0, bytemuck::cast_slice(&self.tubes));
        let cam = &space.camera;
        let eye: Vec3 = cam.eye();
        let u = FrameUniform {
            view_proj: cam.view_proj(size.0 as f32 / size.1.max(1) as f32).to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            look: [time, space.light, space.being.glow, space.being.hue],
            fog: FOG,
        };
        queue.write_buffer(&self.frame_buf, 0, bytemuck::bytes_of(&u));
        let srgb = |c: f32| (c as f64).powf(1.0 / 2.2);
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("space"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
        rp.set_pipeline(&self.tube_pipe);
        rp.set_vertex_buffer(0, self.tube_vb.slice(..));
        rp.set_index_buffer(self.tube_ib.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.tube_count, 0, 0..1);
        rp.set_pipeline(&self.ghost_pipe);
        rp.set_vertex_buffer(0, self.unit_vb.slice(..));
        rp.set_index_buffer(self.unit_ib.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.unit_count, 0, solid..all);
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
