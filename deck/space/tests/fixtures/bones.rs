//! A test model with a skeleton, built by hand (no file): "rig" at (0, 1, 0) carries the bone "upper", that the bone
//! "lower" two metres up, and "body", a triangle skinned to both. Its clip "wave" (2 s) turns "upper" a quarter turn
//! about z over the first second (linear), jumps "lower" from 2 m to 4 m at 1 s (step) and grows it from 1 to 2 over
//! the two seconds (cubic, flat tangents). With it: a reference export of a real model and where to find that model.
//! Shared by deck-space's and deck-headless' tests (`#[path]`).
#![allow(dead_code)]

/// (bone, clip, seconds) → the bone's world matrix, column by column – worked out by hand from the description above
pub const REFERENCE: &[(&str, Option<&str>, f32, [f32; 16])] = &[
    ("upper", None, 0.0, [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0]),
    ("lower", None, 0.0, [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 3.0, 0.0, 1.0]),
    ("lower", Some("wave"), -1.0, [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 3.0, 0.0, 1.0]),
    ("lower", Some("wave"), 0.0, [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 3.0, 0.0, 1.0]),
    // an eighth turn: cos = sin = 0.70710678
    ("upper", Some("wave"), 0.5, [R, R, 0.0, 0.0, -R, R, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0]),
    // still 2 m up the turned bone, grown to 1.15625 (the cubic at a quarter of its span)
    ("lower", Some("wave"), 0.5, [S, S, 0.0, 0.0, -S, S, 0.0, 0.0, 0.0, 0.0, 1.15625, 0.0, -std::f32::consts::SQRT_2, 2.4142137, 0.0, 1.0]),
    ("upper", Some("wave"), 1.0, [0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0]),
    // 4 m along the bone, which now points along −x; grown to 1.5
    ("lower", Some("wave"), 1.0, [0.0, 1.5, 0.0, 0.0, -1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 1.5, 0.0, -4.0, 1.0, 0.0, 1.0]),
    ("lower", Some("wave"), 2.0, [0.0, 2.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, -4.0, 1.0, 0.0, 1.0]),
    ("lower", Some("wave"), 9.0, [0.0, 2.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, -4.0, 1.0, 0.0, 1.0]),
];
const R: f32 = std::f32::consts::FRAC_1_SQRT_2;
const S: f32 = 1.15625 * R;

/// two matrices the same within a float's rounding through the chain (relative to the matrix's largest number)
pub fn close(a: &[f32; 16], b: &[f32; 16]) -> bool {
    let size = a.iter().chain(b).fold(1.0f32, |m, x| m.max(x.abs()));
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 2e-5 * size)
}

/// A file of the owner's media folder (a local dev copy, not in the repository; `SCUTOONS_MEDIA` names the folder
/// for a worktree, which has none); None when it is not here.
pub fn media(file: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("SCUTOONS_MEDIA").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../media"));
    let found = std::fs::read(dir.join(file)).ok();
    if found.is_none() {
        eprintln!("skipped: {} is not here", dir.join(file).display());
    }
    found
}

/// The mannequin's joints as three.js poses them (media/scutoons/3d/mannequin-more-legs.glb: 24 joints, the clips
/// Survey, Walk, Run): the rest pose and each clip at three times.
pub const MANNEQUIN: &str = include_str!("mannequin-more-legs.bones.tsv");

/// a reference export's lines: (bone, clip – None is the rest pose, seconds, the world matrix)
pub fn export(tsv: &str) -> Vec<(&str, Option<&str>, f32, [f32; 16])> {
    tsv.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert_eq!(f.len(), 19, "{l}");
            (f[2], Some(f[0]).filter(|c| *c != "-"), f[1].parse().unwrap(), std::array::from_fn(|i| f[3 + i].parse().unwrap()))
        })
        .collect()
}

/// the two-bone model as a .glb
pub fn glb() -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    // an accessor over a view of its own; 5126 = f32, 5121 = u8, 5123 = u16
    let mut add = |bytes: Vec<u8>, component: u32, count: usize, ty: &str| {
        views.push(format!(r#"{{"buffer":0,"byteOffset":{},"byteLength":{}}}"#, bin.len(), bytes.len()));
        bin.extend(&bytes);
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        accessors.push(format!(r#"{{"bufferView":{},"componentType":{component},"count":{count},"type":"{ty}"}}"#, views.len() - 1));
    };
    let f = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let quarter = [0.0, 0.0, R, R];
    #[rustfmt::skip]
    let inverse_bind = [
        1.0, 0.0, 0.0, 0.0,  0.0, 1.0, 0.0, 0.0,  0.0, 0.0, 1.0, 0.0,  0.0, -1.0, 0.0, 1.0,
        1.0, 0.0, 0.0, 0.0,  0.0, 1.0, 0.0, 0.0,  0.0, 0.0, 1.0, 0.0,  0.0, -3.0, 0.0, 1.0,
    ];
    add(f(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0]), 5126, 3, "VEC3"); // 0 POSITION
    add(vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0], 5121, 3, "VEC4"); // 1 JOINTS_0
    add(f(&[1.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.25, 0.75, 0.0, 0.0]), 5126, 3, "VEC4"); // 2 WEIGHTS_0
    add([0u16, 1, 2].iter().flat_map(|i| i.to_le_bytes()).collect(), 5123, 3, "SCALAR"); // 3 indices
    add(f(&inverse_bind), 5126, 2, "MAT4"); // 4
    add(f(&[0.0, 1.0, 2.0]), 5126, 3, "SCALAR"); // 5 the turn's times
    add(f(&[[0.0, 0.0, 0.0, 1.0], quarter, quarter].concat()), 5126, 3, "VEC4"); // 6
    add(f(&[0.0, 1.0]), 5126, 2, "SCALAR"); // 7 the jump's times
    add(f(&[0.0, 2.0, 0.0, 0.0, 4.0, 0.0]), 5126, 2, "VEC3"); // 8
    add(f(&[0.0, 2.0]), 5126, 2, "SCALAR"); // 9 the growth's times
    add(f(&[[0.0; 3], [1.0; 3], [0.0; 3], [0.0; 3], [2.0; 3], [0.0; 3]].concat()), 5126, 6, "VEC3"); // 10 in-tangent, value, out-tangent
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
        "nodes":[{{"name":"rig","translation":[0,1,0],"children":[1,3]}},{{"name":"upper","children":[2]}},
                 {{"name":"lower","translation":[0,2,0]}},{{"name":"body","mesh":0,"skin":0}}],
        "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"JOINTS_0":1,"WEIGHTS_0":2}},"indices":3}}]}}],
        "skins":[{{"name":"skeleton","joints":[1,2],"inverseBindMatrices":4}}],
        "animations":[{{"name":"wave","channels":[{{"sampler":0,"target":{{"node":1,"path":"rotation"}}}},
                {{"sampler":1,"target":{{"node":2,"path":"translation"}}}},{{"sampler":2,"target":{{"node":2,"path":"scale"}}}}],
            "samplers":[{{"input":5,"output":6}},{{"input":7,"output":8,"interpolation":"STEP"}},{{"input":9,"output":10,"interpolation":"CUBICSPLINE"}}]}}],
        "accessors":[{}],"bufferViews":[{}],"buffers":[{{"byteLength":{}}}]}}"#,
        accessors.join(","),
        views.join(","),
        bin.len()
    );
    let mut json = json.into_bytes();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(0x4E4F534Au32.to_le_bytes());
    out.extend(&json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004E4942u32.to_le_bytes());
    out.extend(&bin);
    out
}
