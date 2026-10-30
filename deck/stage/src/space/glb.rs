//! A set's 3D from a binary glTF (.glb) – the subset tools/rooms' build writes (agents/research/LOCATION-SCENES.md §7):
//! triangle meshes with POSITION, COLOR_0 if any (Unreal's lights in the vertex colours – an atlas bake carries them
//! in its texture), TEXCOORD_0, unlit materials (KHR_materials_unlit: base colour texture × factor), WebP / PNG images
//! (EXT_texture_webp), quantized attributes (KHR_mesh_quantization), meshopt-compressed buffer views
//! (EXT_meshopt_compression, meshopt.rs) and empty nodes as named spots (`spot_*`, `view`). Everything is flattened
//! at load into one world-space vertex / index list with a draw range per material – the GPU side (gpu.rs) draws it
//! unlit with a depth buffer. Normals, tangents, skins, morph targets, animations and cameras are ignored; sparse
//! accessors are refused.
use anyhow::{bail, ensure, Context, Result};
use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// One vertex of the room, world space (24 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RoomVertex {
    pub pos: [f32; 3],
    /// the vertex colour × the material's base colour factor (sRGB-encoded bytes like the texture)
    pub color: [u8; 4],
    pub uv: [f32; 2],
}

/// A material's index range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draw {
    pub first: u32,
    pub count: u32,
    /// into `Model::images`; None = untextured
    pub image: Option<usize>,
}

/// an RGBA8 image (sRGB) with its mip chain: (width, height, pixels) per level, the full size first – made on the
/// loader's thread, so the frame that uploads it only copies
pub struct Image {
    pub levels: Vec<(u32, u32, Vec<u8>)>,
}

impl Image {
    pub fn size(&self) -> (u32, u32) {
        (self.levels[0].0, self.levels[0].1)
    }
}

pub struct Model {
    pub vertices: Vec<RoomVertex>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
    pub images: Vec<Image>,
    /// the nodes without a mesh, world position by name (`spot_altar`, `view` …)
    pub spots: Vec<(String, Vec3)>,
    pub min: Vec3,
    pub max: Vec3,
}

impl Model {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn spot(&self, name: &str) -> Option<Vec3> {
        self.spots.iter().find(|(n, _)| n == name).map(|(_, p)| *p)
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Doc {
    extensions_required: Vec<String>,
    scene: Option<usize>,
    scenes: Vec<SceneDef>,
    nodes: Vec<NodeDef>,
    meshes: Vec<MeshDef>,
    accessors: Vec<Accessor>,
    buffer_views: Vec<BufferView>,
    buffers: Vec<BufferDef>,
    materials: Vec<Material>,
    textures: Vec<Texture>,
    images: Vec<ImageDef>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SceneDef {
    nodes: Vec<usize>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NodeDef {
    name: Option<String>,
    children: Vec<usize>,
    mesh: Option<usize>,
    matrix: Option<[f32; 16]>,
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 4]>,
    scale: Option<[f32; 3]>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct MeshDef {
    primitives: Vec<Primitive>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Primitive {
    attributes: HashMap<String, usize>,
    indices: Option<usize>,
    material: Option<usize>,
    mode: Option<u32>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Accessor {
    buffer_view: Option<usize>,
    byte_offset: usize,
    component_type: u32,
    normalized: bool,
    count: usize,
    #[serde(rename = "type")]
    ty: String,
    min: Option<Vec<f64>>,
    max: Option<Vec<f64>>,
    sparse: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct BufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
    extensions: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Meshopt {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: usize,
    count: usize,
    mode: String,
    filter: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BufferDef {
    uri: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Material {
    pbr_metallic_roughness: Option<Pbr>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Pbr {
    base_color_factor: Option<[f32; 4]>,
    base_color_texture: Option<TexRef>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TexRef {
    index: usize,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Texture {
    source: Option<usize>,
    extensions: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ImageDef {
    buffer_view: Option<usize>,
    uri: Option<String>,
}

/// the extensions a room may require
const SUPPORTED: &[&str] = &["EXT_meshopt_compression", "KHR_mesh_quantization", "EXT_texture_webp", "KHR_materials_unlit"];

/// Load a room; `max_texture` caps the images' longer side (a bigger one is halved until it fits – the X1's memory);
/// `hide` = node names whose mesh is left out (the scene's `hide`, owner 2026-10-30: the cathedral without its incense
/// burner – the export stays as it is; the node's children and spots stay).
pub fn load(path: &Path, max_texture: u32, hide: &[String]) -> Result<Model> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&bytes, max_texture, hide).with_context(|| format!("in {}", path.display()))
}

pub fn parse(bytes: &[u8], max_texture: u32, hide: &[String]) -> Result<Model> {
    let (doc, bin) = split_glb(bytes)?;
    let doc: Doc = serde_json::from_slice(doc).context("the glTF JSON")?;
    for e in &doc.extensions_required {
        ensure!(SUPPORTED.contains(&e.as_str()), "the file requires the glTF extension {e}, which the stage does not read");
    }
    let mut views: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut room = Model { vertices: Vec::new(), indices: Vec::new(), draws: Vec::new(), images: Vec::new(), spots: Vec::new(), min: Vec3::INFINITY, max: Vec3::NEG_INFINITY };
    // materials → images (a texture's source, WebP's through its extension)
    let mut image_of: HashMap<usize, usize> = HashMap::new();
    let roots = doc.scenes.get(doc.scene.unwrap_or(0)).map(|s| s.nodes.clone()).unwrap_or_else(|| (0..doc.nodes.len()).collect());
    let mut stack: Vec<(usize, Mat4)> = roots.iter().map(|&n| (n, Mat4::IDENTITY)).collect();
    // per material: (vertices, indices) gathered, then concatenated in material order
    let mut parts: Vec<(Option<usize>, Vec<RoomVertex>, Vec<u32>)> = Vec::new();
    while let Some((n, parent)) = stack.pop() {
        let node = doc.nodes.get(n).with_context(|| format!("no node {n}"))?;
        let m = parent * local(node);
        stack.extend(node.children.iter().map(|&c| (c, m)));
        let Some(mesh) = node.mesh else {
            if let Some(name) = &node.name {
                room.spots.push((name.clone(), m.transform_point3(Vec3::ZERO)));
            }
            continue;
        };
        if node.name.as_ref().is_some_and(|n| hide.contains(n)) {
            continue;
        }
        let mesh = doc.meshes.get(mesh).with_context(|| format!("no mesh {mesh}"))?;
        for prim in &mesh.primitives {
            if prim.mode.unwrap_or(4) != 4 {
                continue; // points / lines: not a room's
            }
            let Some(&pos) = prim.attributes.get("POSITION") else { continue };
            let positions = read(&doc, bin, &mut views, pos)?;
            check_bounds(&doc.accessors[pos], &positions)?;
            let n_verts = positions.len();
            let colors = match prim.attributes.get("COLOR_0") {
                Some(&a) => read(&doc, bin, &mut views, a)?,
                None => vec![[1.0; 4]; n_verts],
            };
            let uvs = match prim.attributes.get("TEXCOORD_0") {
                Some(&a) => read(&doc, bin, &mut views, a)?,
                None => vec![[0.0; 4]; n_verts],
            };
            ensure!(colors.len() == n_verts && uvs.len() == n_verts, "a primitive's attributes differ in length");
            let indices: Vec<u32> = match prim.indices {
                Some(a) => read(&doc, bin, &mut views, a)?.iter().map(|v| v[0] as u32).collect(),
                None => (0..n_verts as u32).collect(),
            };
            ensure!(indices.iter().all(|&i| (i as usize) < n_verts), "an index past the vertices (a broken meshopt stream?)");
            let (factor, tex) = match prim.material.and_then(|i| doc.materials.get(i)).and_then(|m| m.pbr_metallic_roughness.as_ref()) {
                Some(p) => (p.base_color_factor.unwrap_or([1.0; 4]), p.base_color_texture.as_ref().map(|t| t.index)),
                None => ([1.0; 4], None),
            };
            let image = match tex {
                Some(t) => {
                    let tex = doc.textures.get(t).with_context(|| format!("no texture {t}"))?;
                    let src = tex
                        .extensions
                        .get("EXT_texture_webp")
                        .and_then(|e| e.get("source"))
                        .and_then(|s| s.as_u64())
                        .map(|s| s as usize)
                        .or(tex.source);
                    match src {
                        Some(s) => Some(match image_of.get(&s) {
                            Some(&i) => i,
                            None => {
                                room.images.push(read_image(&doc, bin, s, max_texture)?);
                                image_of.insert(s, room.images.len() - 1);
                                room.images.len() - 1
                            }
                        }),
                        None => None,
                    }
                }
                None => None,
            };
            let part = match parts.iter().position(|p| p.0 == image) {
                Some(i) => &mut parts[i],
                None => {
                    parts.push((image, Vec::new(), Vec::new()));
                    parts.last_mut().expect("pushed")
                }
            };
            let base = part.1.len() as u32;
            for k in 0..n_verts {
                let p = m.transform_point3(Vec3::from_slice(&positions[k][..3]));
                room.min = room.min.min(p);
                room.max = room.max.max(p);
                let c = colors[k];
                let byte = |x: f32, f: f32| ((x * f).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                part.1.push(RoomVertex {
                    pos: p.into(),
                    color: [byte(c[0], factor[0]), byte(c[1], factor[1]), byte(c[2], factor[2]), byte(c[3], factor[3])],
                    uv: [uvs[k][0], uvs[k][1]],
                });
            }
            part.2.extend(indices.iter().map(|i| i + base));
        }
    }
    for (image, verts, idx) in parts {
        let base = room.vertices.len() as u32;
        room.draws.push(Draw { first: room.indices.len() as u32, count: idx.len() as u32, image });
        room.vertices.extend(verts);
        room.indices.extend(idx.iter().map(|i| i + base));
    }
    ensure!(!room.indices.is_empty(), "the room has no triangles");
    Ok(room)
}

/// a node's local matrix (`matrix`, else T · R · S)
fn local(n: &NodeDef) -> Mat4 {
    if let Some(m) = n.matrix {
        return Mat4::from_cols_array(&m);
    }
    let t = n.translation.map(Vec3::from).unwrap_or(Vec3::ZERO);
    let r = n.rotation.map(Quat::from_array).unwrap_or(Quat::IDENTITY);
    let s = n.scale.map(Vec3::from).unwrap_or(Vec3::ONE);
    Mat4::from_scale_rotation_translation(s, r, t)
}

/// the GLB container: (the JSON chunk, the BIN chunk)
fn split_glb(b: &[u8]) -> Result<(&[u8], &[u8])> {
    ensure!(b.len() >= 20 && &b[0..4] == b"glTF", "not a binary glTF (.glb)");
    let u32_at = |o: usize| -> Result<usize> {
        let s = b.get(o..o + 4).context("a truncated .glb")?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
    };
    ensure!(u32_at(4)? == 2, "glTF version {} (only 2)", u32_at(4)?);
    let (json_len, json_ty) = (u32_at(12)?, u32_at(16)?);
    ensure!(json_ty == 0x4E4F534A, "the first chunk is not JSON");
    let json = b.get(20..20 + json_len).context("a truncated JSON chunk")?;
    let at = 20 + json_len;
    let bin = if b.len() >= at + 8 && u32_at(at + 4)? == 0x004E4942 {
        b.get(at + 8..at + 8 + u32_at(at)?).context("a truncated BIN chunk")?
    } else {
        &[]
    };
    Ok((json, bin))
}

/// a buffer's bytes: the GLB's BIN chunk (buffer 0 without a uri); external files are not read (a room is one file)
fn buffer<'a>(doc: &Doc, bin: &'a [u8], i: usize) -> Result<&'a [u8]> {
    let b = doc.buffers.get(i).with_context(|| format!("no buffer {i}"))?;
    ensure!(i == 0 && b.uri.is_none(), "buffer {i} is not the .glb's own (external buffers are not read)");
    Ok(bin)
}

/// a buffer view's bytes and stride – meshopt-compressed views are decoded once and kept
fn view<'a>(doc: &Doc, bin: &'a [u8], views: &'a mut HashMap<usize, Vec<u8>>, i: usize) -> Result<(&'a [u8], Option<usize>)> {
    let v = doc.buffer_views.get(i).with_context(|| format!("no buffer view {i}"))?;
    if let Some(ext) = v.extensions.get("EXT_meshopt_compression") {
        let m: Meshopt = serde_json::from_value(ext.clone()).context("EXT_meshopt_compression")?;
        if let Some(f) = m.filter.as_deref().filter(|f| *f != "NONE") {
            bail!("buffer view {i}: the meshopt filter {f} is not supported – export the room without filters");
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = views.entry(i) {
            let src = buffer(doc, bin, m.buffer)?.get(m.byte_offset..m.byte_offset + m.byte_length).context("a meshopt view past its buffer")?;
            let bytes = match m.mode.as_str() {
                "ATTRIBUTES" => crate::space::meshopt::decode_vertices(src, m.count, m.byte_stride)?,
                "TRIANGLES" => {
                    let idx = crate::space::meshopt::decode_triangles(src, m.count)?;
                    match m.byte_stride {
                        2 => idx.iter().flat_map(|&x| (x as u16).to_le_bytes()).collect(),
                        4 => idx.iter().flat_map(|&x| x.to_le_bytes()).collect(),
                        s => bail!("buffer view {i}: an index stride of {s}"),
                    }
                }
                mode => bail!("buffer view {i}: the meshopt mode {mode} is not supported"),
            };
            slot.insert(bytes);
        }
        return Ok((&views[&i], Some(m.byte_stride)));
    }
    let b = buffer(doc, bin, v.buffer)?;
    Ok((b.get(v.byte_offset..v.byte_offset + v.byte_length).context("a buffer view past its buffer")?, v.byte_stride))
}

/// an accessor's elements as f32 (normalized integers mapped like glTF says, up to 4 components)
fn read(doc: &Doc, bin: &[u8], views: &mut HashMap<usize, Vec<u8>>, a: usize) -> Result<Vec<[f32; 4]>> {
    let acc = doc.accessors.get(a).with_context(|| format!("no accessor {a}"))?;
    ensure!(acc.sparse.is_none(), "accessor {a} is sparse (not supported)");
    let comps = match acc.ty.as_str() {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        t => bail!("accessor {a}: type {t}"),
    };
    let size = match acc.component_type {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        t => bail!("accessor {a}: component type {t}"),
    };
    let Some(bv) = acc.buffer_view else { return Ok(vec![[0.0; 4]; acc.count]) };
    let (bytes, stride) = view(doc, bin, views, bv)?;
    let stride = stride.unwrap_or(comps * size);
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let at = acc.byte_offset + i * stride;
        let e = bytes.get(at..at + comps * size).with_context(|| format!("accessor {a}: element {i} past its view"))?;
        let mut v = [0.0f32; 4];
        for (c, x) in v.iter_mut().enumerate().take(comps) {
            let s = &e[c * size..(c + 1) * size];
            let n = acc.normalized;
            *x = match acc.component_type {
                5120 => norm(s[0] as i8 as f32, 127.0, n).max(-1.0),
                5121 => norm(s[0] as f32, 255.0, n),
                5122 => norm(i16::from_le_bytes([s[0], s[1]]) as f32, 32767.0, n).max(-1.0),
                5123 => norm(u16::from_le_bytes([s[0], s[1]]) as f32, 65535.0, n),
                5125 => u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32,
                _ => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            };
        }
        out.push(v);
    }
    Ok(out)
}

fn norm(v: f32, max: f32, normalized: bool) -> f32 {
    if normalized {
        v / max
    } else {
        v
    }
}

/// The accessor's min / max (in its stored units) against what was decoded – a decoder that went wrong shows here.
fn check_bounds(acc: &Accessor, v: &[[f32; 4]]) -> Result<()> {
    let (Some(min), Some(max)) = (&acc.min, &acc.max) else { return Ok(()) };
    let scale = match (acc.normalized, acc.component_type) {
        (true, 5120) => 127.0,
        (true, 5121) => 255.0,
        (true, 5122) => 32767.0,
        (true, 5123) => 65535.0,
        _ => 1.0,
    };
    for c in 0..min.len().min(max.len()).min(3) {
        let (lo, hi) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x[c] as f64), hi.max(x[c] as f64)));
        let tol = 1e-3 * (max[c] - min[c]).abs().max(1.0);
        ensure!(
            (lo * scale - min[c]).abs() <= tol && (hi * scale - max[c]).abs() <= tol,
            "decoded positions span {lo}..{hi}, the accessor says {}..{} (component {c})",
            min[c] / scale,
            max[c] / scale
        );
    }
    Ok(())
}

/// an image: its bytes (a buffer view, the only way in a .glb), decoded to RGBA8 (WebP, PNG), halved until it fits
fn read_image(doc: &Doc, bin: &[u8], i: usize, max_texture: u32) -> Result<Image> {
    let def = doc.images.get(i).with_context(|| format!("no image {i}"))?;
    let Some(bv) = def.buffer_view else { bail!("image {i} is not inside the .glb ({:?})", def.uri) };
    let mut none = HashMap::new();
    let (bytes, _) = view(doc, bin, &mut none, bv)?;
    let mut img = image::load_from_memory(bytes).with_context(|| format!("decoding image {i}"))?.to_rgba8();
    while img.width().max(img.height()) > max_texture.max(1) {
        img = image::imageops::resize(&img, (img.width() / 2).max(1), (img.height() / 2).max(1), image::imageops::FilterType::Triangle);
    }
    let mut levels = vec![(img.width(), img.height(), img.into_raw())];
    while let Some(&(w, h, ref px)) = levels.last().filter(|(w, h, _)| *w > 1 || *h > 1) {
        let next = half(w, h, px);
        levels.push(next);
    }
    Ok(Image { levels })
}

/// one mip level down: each texel the mean of its 2 × 2 (colour in linear light, alpha as is)
pub fn half(w: u32, h: u32, px: &[u8]) -> (u32, u32, Vec<u8>) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let lin = |b: u8| (b as f32 / 255.0).powf(2.2);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let mut sum = 0.0;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                    let b = px[((sy * w + sx) * 4 + c) as usize];
                    sum += if c == 3 { b as f32 / 255.0 } else { lin(b) };
                }
                let v = sum / 4.0;
                let v = if c == 3 { v } else { v.powf(1.0 / 2.2) };
                out[((y * nw + x) * 4 + c) as usize] = (v * 255.0 + 0.5) as u8;
            }
        }
    }
    (nw, nh, out)
}


#[cfg(test)]
mod tests {
    use super::*;

    /// the tiny room's glTF JSON: one triangle (float positions) on a node, a node without a mesh as a spot
    const TINY: &str = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0,1]}],
            "nodes":[{"mesh":0,"translation":[10,0,0],"scale":[2,2,2]},{"name":"spot_a","translation":[1,2,3]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0}]}],
            "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[1,0.5,0.25,1]}}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},
                         {"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],
            "buffers":[{"byteLength":44}]}"#;

    /// a .glb of the tiny room, built by hand
    fn tiny() -> Vec<u8> {
        pack(TINY)
    }

    fn pack(json: &str) -> Vec<u8> {
        let mut bin: Vec<u8> = [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
        bin.extend([0u16, 1, 2, 0].iter().flat_map(|i| i.to_le_bytes()));
        let mut j = json.as_bytes().to_vec();
        while j.len() % 4 != 0 {
            j.push(b' ');
        }
        let mut out = Vec::new();
        out.extend(b"glTF");
        out.extend(2u32.to_le_bytes());
        out.extend(((12 + 8 + j.len() + 8 + bin.len()) as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(0x4E4F534Au32.to_le_bytes());
        out.extend(&j);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(0x004E4942u32.to_le_bytes());
        out.extend(&bin);
        out
    }

    #[test]
    fn a_mip_level_halves_and_keeps_flat_colours() {
        let px: Vec<u8> = [10u8, 128, 250, 255].repeat(5 * 3);
        let (w, h, out) = half(5, 3, &px);
        assert_eq!((w, h), (2, 1));
        for c in out.chunks(4) {
            assert!(c.iter().zip([10u8, 128, 250, 255]).all(|(a, b)| a.abs_diff(b) <= 1), "{c:?}");
        }
    }

    #[test]
    fn a_tiny_room_flattens_into_world_space() {
        let r = parse(&tiny(), 2048, &[]).unwrap();
        assert_eq!(r.triangles(), 1);
        assert_eq!(r.vertices[1].pos, [12.0, 0.0, 0.0], "translation · scale");
        assert_eq!(r.vertices[0].color, [255, 128, 64, 255], "white × the base colour factor");
        assert_eq!(r.spot("spot_a"), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(r.draws, vec![Draw { first: 0, count: 3, image: None }]);
        assert_eq!((r.min, r.max), (Vec3::new(10.0, 0.0, 0.0), Vec3::new(12.0, 2.0, 0.0)));
    }

    #[test]
    fn a_hidden_node_drops_its_mesh_and_keeps_the_spots() {
        // a second node with the same mesh, named
        let json = TINY.replace(r#""nodes":[0,1]"#, r#""nodes":[0,1,2]"#).replace(r#"{"name":"spot_a","translation":[1,2,3]}]"#, r#"{"name":"spot_a","translation":[1,2,3]},{"name":"burner","mesh":0}]"#);
        let glb = pack(&json);
        assert_eq!(parse(&glb, 2048, &[]).unwrap().triangles(), 2);
        let r = parse(&glb, 2048, &["burner".to_string()]).unwrap();
        assert_eq!(r.triangles(), 1);
        assert_eq!(r.spot("spot_a"), Some(Vec3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn unknown_required_extensions_are_errors() {
        let json = r#"{"extensionsRequired":["KHR_draco_mesh_compression"],"asset":{"version":"2.0"}}"#;
        let mut glb = Vec::new();
        glb.extend(b"glTF");
        glb.extend(2u32.to_le_bytes());
        glb.extend(((20 + json.len()) as u32).to_le_bytes());
        glb.extend((json.len() as u32).to_le_bytes());
        glb.extend(0x4E4F534Au32.to_le_bytes());
        glb.extend(json.as_bytes());
        let e = parse(&glb, 2048, &[]).err().unwrap();
        assert!(format!("{e:#}").contains("KHR_draco_mesh_compression"), "{e:#}");
    }

    /// The owner's cathedral (a local dev copy, not in git): meshopt + quantization + WebP decode, the bounds check
    /// passes, the spots are there. Skipped when the file is missing.
    #[test]
    fn the_cathedral_loads_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../media/scutoons/rooms/cathedral.low.glb");
        if !path.exists() {
            eprintln!("skipped: {} is not here", path.display());
            return;
        }
        let r = load(&path, 1024, &[]).unwrap();
        // the atlas re-bake (#74, tools/rooms/blender/bake_atlas.py): without the incense burner (cathedral.json `drop`),
        // decimated to the low level's 35k, one 1024² atlas with the light in it – one draw call
        assert_eq!(r.triangles(), 34_935, "the report's count");
        assert_eq!(r.draws.len(), 1, "the report's draw calls: one atlas");
        assert!(r.images.len() == 1 && r.images.iter().all(|i| i.size().0 <= 1024 && i.levels.last().unwrap().0 == 1));
        assert!(r.spot("spot_aisle").is_some() && r.spot("view").is_some());
        let size = r.max - r.min;
        assert!(size.max_element() < 100.0 && size.min_element() > 2.0, "a room in metres: {size}");
    }
}
