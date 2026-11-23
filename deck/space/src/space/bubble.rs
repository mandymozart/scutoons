//! The cloud's default puff model: a low-poly bubble – an icosphere split twice (320 faces, 162 corners), 1 m across – as a
//! binary glTF. It is the file the library holds as `scutoons/3d/cloud-puff.glb` until the owner replaces it there with a
//! model of his own (`cargo run -p deck-space --bin cloud-puff -- cloud-puff.glb` writes it; the same bytes every run).
use glam::Vec3;

/// how often the icosahedron's faces are split in four
const SPLITS: usize = 2;

/// the bubble's corners (on the unit sphere × 0.5) and its triangles, wound counter-clockwise seen from outside
pub fn mesh() -> (Vec<Vec3>, Vec<[u16; 3]>) {
    let g = (1.0 + 5f32.sqrt()) / 2.0;
    let mut v: Vec<Vec3> = [[-1.0, g, 0.0], [1.0, g, 0.0], [-1.0, -g, 0.0], [1.0, -g, 0.0], [0.0, -1.0, g], [0.0, 1.0, g], [0.0, -1.0, -g], [0.0, 1.0, -g], [g, 0.0, -1.0], [g, 0.0, 1.0], [-g, 0.0, -1.0], [-g, 0.0, 1.0]]
        .iter()
        .map(|a| Vec3::from_array(*a).normalize())
        .collect();
    let mut faces: Vec<[u16; 3]> = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    for _ in 0..SPLITS {
        let mut halves: std::collections::BTreeMap<(u16, u16), u16> = std::collections::BTreeMap::new();
        let mut half = |a: u16, b: u16, v: &mut Vec<Vec3>| {
            *halves.entry((a.min(b), a.max(b))).or_insert_with(|| {
                v.push((v[a as usize] + v[b as usize]).normalize());
                (v.len() - 1) as u16
            })
        };
        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let (ab, bc, ca) = (half(a, b, &mut v), half(b, c, &mut v), half(c, a, &mut v));
            next.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        }
        faces = next;
    }
    // glTF's front is counter-clockwise seen from outside
    let outward = |[a, b, c]: [u16; 3]| {
        let [p, q, r] = [a, b, c].map(|i| v[i as usize]);
        if (q - p).cross(r - p).dot(p + q + r) < 0.0 {
            [a, c, b]
        } else {
            [a, b, c]
        }
    };
    let faces = faces.into_iter().map(outward).collect();
    (v.into_iter().map(|p| p * 0.5).collect(), faces)
}

/// The bubble as a .glb: one node, one mesh with POSITION, NORMAL and 16-bit indices, a plain white material (the cloud
/// gives each puff its colour).
pub fn glb() -> Vec<u8> {
    let (corners, faces) = mesh();
    let mut bin: Vec<u8> = Vec::new();
    for p in &corners {
        bin.extend(p.to_array().iter().flat_map(|x| x.to_le_bytes()));
    }
    let normals_at = bin.len();
    for p in &corners {
        bin.extend(p.normalize().to_array().iter().flat_map(|x| x.to_le_bytes()));
    }
    let indices_at = bin.len();
    for i in faces.iter().flatten() {
        bin.extend(i.to_le_bytes());
    }
    let indices_len = bin.len() - indices_at;
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let (lo, hi) = corners.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(l, h), p| (l.min(*p), h.max(*p)));
    let json = serde_json::json!({
        "asset": { "version": "2.0", "generator": "SCUTOONS deck-space bubble" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "name": "cloud-puff", "mesh": 0 }],
        "meshes": [{ "name": "puff", "primitives": [{ "attributes": { "POSITION": 0, "NORMAL": 1 }, "indices": 2, "material": 0 }] }],
        "materials": [{ "name": "bubble", "pbrMetallicRoughness": { "baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.6 } }],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": corners.len(), "type": "VEC3", "min": lo.to_array(), "max": hi.to_array() },
            { "bufferView": 1, "componentType": 5126, "count": corners.len(), "type": "VEC3" },
            { "bufferView": 2, "componentType": 5123, "count": faces.len() * 3, "type": "SCALAR" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": normals_at, "target": 34962 },
            { "buffer": 0, "byteOffset": normals_at, "byteLength": indices_at - normals_at, "target": 34962 },
            { "buffer": 0, "byteOffset": indices_at, "byteLength": indices_len, "target": 34963 }
        ],
        "buffers": [{ "byteLength": bin.len() }]
    });
    let mut text = serde_json::to_vec(&json).expect("plain JSON");
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(text);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bubble_is_a_closed_low_poly_sphere_a_metre_across_facing_out() {
        let (v, f) = mesh();
        assert_eq!((v.len(), f.len()), (162, 320));
        for [a, b, c] in &f {
            let [a, b, c] = [*a, *b, *c].map(|i| v[i as usize]);
            assert!((b - a).cross(c - a).dot(a + b + c) > 0.0, "counter-clockwise from outside");
        }
        assert!(v.iter().all(|p| (p.length() - 0.5).abs() < 1e-5));
        // every edge is shared by two faces: closed
        let mut edges = std::collections::BTreeMap::new();
        for t in &f {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|n| *n == 2));
    }

    #[test]
    fn the_glb_reads_back_as_a_model_and_is_the_same_every_run() {
        let bytes = glb();
        assert_eq!(bytes, glb());
        assert_eq!(&bytes[..4], b"glTF");
        let figure = crate::space::figure::read(&bytes, None, &mut |_, _| Ok(image::RgbaImage::new(1, 1))).unwrap();
        assert_eq!(figure.indices.len(), 960);
        assert!((figure.height - 1.0).abs() < 0.01, "{}", figure.height);
    }
}
