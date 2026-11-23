//! Shared animations (#817): an animation asset is a .glb that holds a skeleton and its clips, nothing drawn. `extract`
//! makes one from a character's model – its nodes, skins and animations kept, its meshes, materials and images left
//! out; the stage plays it on any character whose skeleton has the joints it turns (anatomy.rs `adopt`, the rule
//! deck-catalog skeleton.rs `fit`).
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;

/// the JSON and the binary chunk of a .glb
fn chunks(b: &[u8]) -> Result<(Value, &[u8])> {
    ensure!(
        b.len() >= 20 && &b[0..4] == b"glTF",
        "not a binary glTF (.glb)"
    );
    let u32_at = |at: usize| -> Result<u32> {
        Ok(u32::from_le_bytes(
            b.get(at..at + 4)
                .context("the file ends early")?
                .try_into()?,
        ))
    };
    let (mut at, mut doc, mut bin): (usize, Option<Value>, &[u8]) = (12, None, &[]);
    while at + 8 <= b.len() {
        let (len, ty) = (u32_at(at)? as usize, u32_at(at + 4)?);
        let chunk = b
            .get(at + 8..at + 8 + len)
            .context("a chunk runs past the file")?;
        match ty {
            JSON_CHUNK => doc = Some(serde_json::from_slice(chunk).context("the glTF JSON")?),
            BIN_CHUNK => bin = chunk,
            _ => {}
        }
        at += 8 + len;
    }
    Ok((doc.context("no JSON chunk")?, bin))
}

fn component_size(c: u64) -> Result<usize> {
    Ok(match c {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        c => bail!("component type {c}"),
    })
}

fn components(ty: &str) -> Result<usize> {
    Ok(match ty {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT4" => 16,
        t => bail!("accessor type {t}"),
    })
}

/// an accessor's elements, tightly packed
fn accessor_bytes(doc: &Value, bin: &[u8], a: usize) -> Result<Vec<u8>> {
    let acc = &doc["accessors"][a];
    ensure!(
        acc.get("sparse").is_none(),
        "accessor {a} is sparse (not supported)"
    );
    let size = component_size(acc["componentType"].as_u64().unwrap_or(0))?
        * components(acc["type"].as_str().unwrap_or(""))?;
    let count = acc["count"].as_u64().context("an accessor without count")? as usize;
    let bv = &doc["bufferViews"][acc["bufferView"]
        .as_u64()
        .with_context(|| format!("accessor {a} has no buffer view"))?
        as usize];
    ensure!(
        bv.get("extensions").is_none(),
        "accessor {a}'s data is compressed – export the model without compression"
    );
    let stride = bv["byteStride"].as_u64().map_or(size, |s| s as usize);
    let start = bv["byteOffset"].as_u64().unwrap_or(0) as usize
        + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(size * count);
    for k in 0..count {
        out.extend_from_slice(
            bin.get(start + k * stride..start + k * stride + size)
                .with_context(|| format!("accessor {a} runs past the buffer"))?,
        );
    }
    Ok(out)
}

/// The animation asset of a model file: its node tree (without meshes), its skins and its animations, the data they
/// read copied into a new buffer; meshes, materials, textures, images and cameras left out. Fails when the file has no
/// skin or no animation – there would be nothing to share.
pub fn extract(bytes: &[u8]) -> Result<Vec<u8>> {
    let (doc, bin) = chunks(bytes)?;
    let skins = doc["skins"].as_array().cloned().unwrap_or_default();
    let animations = doc["animations"].as_array().cloned().unwrap_or_default();
    ensure!(
        !skins.is_empty(),
        "the file has no skin: no skeleton to share"
    );
    ensure!(
        !animations.is_empty(),
        "the file has no animation: no clip to share"
    );
    let mut out_bin: Vec<u8> = Vec::new();
    let (mut views, mut accessors) = (Vec::new(), Vec::new());
    let mut copied: BTreeMap<u64, usize> = BTreeMap::new();
    let mut copy = |a: &Value| -> Result<Value> {
        let a = a.as_u64().context("an accessor index")?;
        if let Some(&n) = copied.get(&a) {
            return Ok(json!(n));
        }
        let data = accessor_bytes(&doc, bin, a as usize)?;
        while out_bin.len() % 4 != 0 {
            out_bin.push(0);
        }
        views.push(json!({ "buffer": 0, "byteOffset": out_bin.len(), "byteLength": data.len() }));
        out_bin.extend_from_slice(&data);
        let mut acc = Map::new();
        for (k, v) in doc["accessors"][a as usize]
            .as_object()
            .context("an accessor")?
        {
            if [
                "componentType",
                "count",
                "type",
                "min",
                "max",
                "normalized",
                "name",
            ]
            .contains(&k.as_str())
            {
                acc.insert(k.clone(), v.clone());
            }
        }
        acc.insert("bufferView".into(), json!(views.len() - 1));
        accessors.push(Value::Object(acc));
        copied.insert(a, accessors.len() - 1);
        Ok(json!(accessors.len() - 1))
    };
    let mut new_skins = Vec::new();
    for s in &skins {
        let mut s = s.clone();
        if let Some(ib) = s.get("inverseBindMatrices").cloned() {
            s["inverseBindMatrices"] = copy(&ib)?;
        }
        new_skins.push(s);
    }
    let mut new_animations = Vec::new();
    for a in &animations {
        let mut a = a.clone();
        for s in a["samplers"].as_array_mut().into_iter().flatten() {
            s["input"] = copy(&s["input"].clone())?;
            s["output"] = copy(&s["output"].clone())?;
        }
        if let Some(channels) = a["channels"].as_array_mut() {
            channels.retain(|c| {
                ["translation", "rotation", "scale"]
                    .contains(&c["target"]["path"].as_str().unwrap_or(""))
            });
        }
        new_animations.push(a);
    }
    let nodes: Vec<Value> = doc["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| {
            let mut n = n.clone();
            if let Some(o) = n.as_object_mut() {
                // a node's skin needs its mesh (glTF); the skins stay listed on their own
                for k in ["mesh", "skin", "camera", "weights", "extensions"] {
                    o.remove(k);
                }
            }
            n
        })
        .collect();
    while out_bin.len() % 4 != 0 {
        out_bin.push(0);
    }
    let mut out = json!({
        "asset": { "version": "2.0", "generator": "scutoons deck-space animation::extract" },
        "nodes": nodes,
        "skins": new_skins,
        "animations": new_animations,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{ "byteLength": out_bin.len() }],
    });
    if let Some(scenes) = doc.get("scenes") {
        out["scenes"] = scenes.clone();
        out["scene"] = doc.get("scene").cloned().unwrap_or(json!(0));
    }
    Ok(write(&out, &out_bin))
}

/// a .glb from its JSON and its binary chunk
fn write(doc: &Value, bin: &[u8]) -> Vec<u8> {
    let mut json = serde_json::to_vec(doc).expect("JSON");
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut out = Vec::with_capacity(28 + json.len() + bin.len());
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(JSON_CHUNK.to_le_bytes());
    out.extend(&json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(BIN_CHUNK.to_le_bytes());
    out.extend(bin);
    out
}

#[cfg(test)]
mod tests {
    use super::super::anatomy::bones;
    use super::super::{figure, glb};
    use super::*;

    /// the two-bone model's animation, extracted: its skeleton and its clip, nothing drawn
    #[test]
    fn extract_keeps_the_skeleton_and_the_clips_and_drops_the_meshes() {
        let anim = extract(&bones::glb()).unwrap();
        let (doc, _) = chunks(&anim).unwrap();
        assert!(
            doc.get("meshes").is_none()
                && doc["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|n| n.get("mesh").is_none() && n.get("skin").is_none())
        );
        assert!(
            glb::parse_with(&anim, 1, &[], &mut |_, _| Ok(image::RgbaImage::new(1, 1))).is_err(),
            "nothing to draw"
        );
        let a = glb::skeleton(&anim).unwrap();
        let s = a.skeleton();
        assert_eq!(
            (
                s.joints.as_slice(),
                s.animated.as_slice(),
                a.clips[0].name.as_str(),
                a.clips[0].duration
            ),
            (
                ["upper", "lower"].map(String::from).as_slice(),
                ["upper".to_string()].as_slice(),
                "wave",
                2.0
            )
        );
        let source = glb::anatomy(&bones::glb()).unwrap();
        for t in [0.0, 0.7, 1.5] {
            assert_eq!(
                a.bone_world("upper", Some("wave"), t),
                source.bone_world("upper", Some("wave"), t),
                "the same clip at {t}"
            );
        }
        assert!(
            extract(&anim).is_ok(),
            "an animation asset extracts to itself"
        );
    }

    /// the fixture pair: the two-bone animation played on a model whose bones are longer and whose joints are named
    /// partly otherwise – rotations by name, its own lengths kept, the missing joint named
    #[test]
    fn an_adopted_clip_turns_the_joints_of_the_same_name_and_keeps_the_figure_s_lengths() {
        let source = glb::skeleton(&extract(&bones::glb()).unwrap()).unwrap();
        let mut target = glb::anatomy(&bones::glb()).unwrap();
        target.clips.clear();
        let lower = target.bone("lower").unwrap();
        target.nodes[lower].local = glam::Mat4::from_translation(glam::Vec3::new(0.0, 5.0, 0.0));
        let (clip, fit) = target.adopt(&source).unwrap();
        assert!(fit.fits(), "{fit:?}");
        assert_eq!(
            (
                clip,
                target.clips[clip].name.as_str(),
                target.clips[clip].channels.len()
            ),
            (0, "wave", 1),
            "only the rotation travels"
        );
        let upper = target.bone("upper").unwrap();
        target.nodes[upper].rest.0 = glam::Vec3::new(0.0, 3.0, 0.0);
        let (_, turn, at) = target
            .local(upper, Some(clip), 1.0)
            .to_scale_rotation_translation();
        let (_, want, _) = source
            .local(source.bone("upper").unwrap(), Some(0), 1.0)
            .to_scale_rotation_translation();
        assert!(
            turn.abs_diff_eq(want, 1e-6) && at == glam::Vec3::new(0.0, 3.0, 0.0),
            "the source's turn at the figure's own offset: {turn} {at}"
        );
        assert_eq!(
            target.local(lower, Some(clip), 1.0),
            target.nodes[lower].local,
            "the lower bone keeps its own length, untouched by the source's jump"
        );

        let mut other = glb::anatomy(&bones::glb()).unwrap();
        let upper = other.bone("upper").unwrap();
        other.nodes[upper].name = "arm".into();
        let (clip, fit) = other.adopt(&source).unwrap();
        assert_eq!(
            (
                fit.matched,
                fit.of,
                fit.missing.as_slice(),
                other.clips[clip].channels.len()
            ),
            (0, 1, ["upper".to_string()].as_slice(), 0),
            "nothing matches: the rest pose"
        );
    }

    /// The Cat Pope idle in the library (tests/fixtures/catpope-idle.glb, made by `extract-animation` from catpope.glb):
    /// the Mixamo skeleton, 65 turned joints, 15.38 s; on the two-bone model nothing fits, on itself everything does.
    #[test]
    fn the_catpope_idle_is_a_mixamo_animation_of_65_joints() {
        let file = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/catpope-idle.glb"
        ))
        .unwrap();
        let a = glb::skeleton(&file).unwrap();
        let s = a.skeleton();
        assert_eq!(
            (s.family.as_deref(), s.joints.len(), s.animated.len()),
            (Some("mixamo"), 65, 65)
        );
        assert!(
            (a.clips[0].duration - 15.38).abs() < 0.01,
            "{}",
            a.clips[0].duration
        );
        assert!(s.problems().is_empty(), "{:?}", s.problems());
        let mut two = glb::anatomy(&bones::glb()).unwrap();
        let (_, fit) = two.adopt(&a).unwrap();
        assert_eq!((fit.matched, fit.of), (0, 65));
        let mut same = a.clone();
        let (clip, fit) = same.adopt(&a).unwrap();
        assert!(fit.fits() && same.clips[clip].channels.len() == 65);
        assert!(
            figure::read(&file, None, &mut |_, _| Ok(image::RgbaImage::new(1, 1))).is_err(),
            "an animation is no figure"
        );
    }
}
