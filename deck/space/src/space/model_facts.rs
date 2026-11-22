//! What a model file is, as the creator's 3D place shows it beside the model: its triangles, its bones (each with the
//! bone above it), its clips, its spots and the plain sentences for what it lacks. Read once from the GLB through the
//! anatomy (anatomy.rs) and the file's spots (spots.rs); the page draws these and decides nothing.
use super::{glb, spots};
use deck_catalog::spot::Spots;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bone {
    pub name: String,
    /// the nearest joint above it (None: a root of the skeleton)
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClipFacts {
    pub name: String,
    pub seconds: f32,
}

/// A spot of the file, where it is in the rest pose (model space).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpotFacts {
    pub name: String,
    pub kind: &'static str,
    pub at: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Facts {
    pub triangles: usize,
    pub bones: Vec<Bone>,
    pub clips: Vec<ClipFacts>,
    pub spots: Vec<SpotFacts>,
    /// what the file lacks or gets wrong, one sentence each (none: nothing to say)
    pub missing: Vec<String>,
}

/// The facts of a model file (its images are not decoded).
pub fn facts(bytes: &[u8]) -> anyhow::Result<Facts> {
    let model = glb::parse_with(bytes, 1, &[], &mut |_, _| Ok(image::RgbaImage::new(1, 1)))?;
    let a = &model.anatomy;
    let joint_above = |n: usize| {
        let mut p = a.nodes[n].parent;
        while let Some(i) = p {
            if a.nodes[i].joint {
                return Some(a.nodes[i].name.clone());
            }
            p = a.nodes[i].parent;
        }
        None
    };
    let mut bones: Vec<Bone> = Vec::new();
    for s in &a.skins {
        for &j in &s.joints {
            if !bones.iter().any(|b| b.name == a.nodes[j].name) {
                bones.push(Bone { name: a.nodes[j].name.clone(), parent: joint_above(j) });
            }
        }
    }
    let clips = a.clips.iter().map(|c| ClipFacts { name: c.name.clone(), seconds: c.duration }).collect::<Vec<_>>();
    let file = spots::file(a);
    let found = Spots { file: Some(file), hand: Vec::new() };
    let mut missing = Vec::new();
    if a.skins.is_empty() {
        missing.push("not rigged: the file has no skin, so nothing can pose it".to_string());
    }
    if a.clips.is_empty() {
        missing.push("no clips: the file has no animations".to_string());
    }
    if bones.iter().any(|b| b.name.is_empty()) {
        missing.push("a bone has no name: a clip or a spot cannot find it".to_string());
    }
    missing.extend(found.problems(None));
    let spots = found
        .resolved()
        .iter()
        .filter_map(|s| Some(SpotFacts { name: s.name.clone(), kind: s.kind.word(), at: spots::world(a, s, None, 0.0)?.w_axis.truncate().to_array() }))
        .collect();
    Ok(Facts { triangles: model.triangles(), bones, clips, spots, missing })
}

/// `facts` as the page reads it: `{ ok: true, triangles, bones, clips, spots, missing }` or `{ ok: false, why }`
pub fn facts_json(bytes: &[u8]) -> String {
    #[derive(Serialize)]
    struct Ok<'a> {
        ok: bool,
        #[serde(flatten)]
        facts: &'a Facts,
    }
    match facts(bytes) {
        Ok(f) => serde_json::to_string(&Ok { ok: true, facts: &f }).unwrap_or_else(|e| failed(&e.to_string())),
        Err(e) => failed(&format!("this file is no model the deck can read: {e}")),
    }
}

fn failed(why: &str) -> String {
    serde_json::json!({ "ok": false, "why": why }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::anatomy::bones;

    #[test]
    fn the_two_bone_model_says_its_bones_clip_and_what_it_lacks() {
        let f = facts(&bones::glb()).unwrap();
        assert_eq!(f.triangles, 1);
        assert_eq!(f.bones, [Bone { name: "upper".into(), parent: None }, Bone { name: "lower".into(), parent: Some("upper".into()) }]);
        assert_eq!(f.clips, [ClipFacts { name: "wave".into(), seconds: 2.0 }]);
        assert!(f.spots.is_empty());
        assert!(f.missing.is_empty(), "{:?}", f.missing);
    }

    #[test]
    fn a_broken_file_answers_why() {
        let v: serde_json::Value = serde_json::from_str(&facts_json(b"not a glb")).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["why"].as_str().unwrap().starts_with("this file is no model the deck can read"));
        let v: serde_json::Value = serde_json::from_str(&facts_json(&bones::glb())).unwrap();
        assert_eq!((v["ok"].as_bool(), v["bones"][1]["parent"].as_str(), v["clips"][0]["seconds"].as_f64()), (Some(true), Some("upper"), Some(2.0)));
    }

    #[test]
    fn catpope_has_its_nineteen_bones_when_present() {
        let Some(file) = bones::media("characters/catpope/catpope-regular.glb") else { return };
        let f = facts(&file).unwrap();
        assert_eq!(f.bones.len(), 19);
        assert_eq!(f.bones.iter().find(|b| b.name == "head").and_then(|b| b.parent.as_deref()), Some("neck"));
    }
}
