//! A model's anatomy, kept beside the flattened draw list (glb.rs): the node tree with names, the parts (a mesh
//! primitive on a node, as a range of the model's vertices and indices), the skins (joints, inverse bind matrices)
//! and the animation clips (channels of keys per node). A node's world matrix at a clip's time is answered from
//! these without allocating; a time outside a channel's keys holds its first / last key (looping is the player's).
use anyhow::{bail, ensure, Result};
use glam::{Mat4, Quat, Vec3};
use std::ops::Range;

#[derive(Debug, Clone, Default)]
pub struct Anatomy {
    /// in the file's order (a glTF node index is an index here)
    pub nodes: Vec<Node>,
    /// the scene's root nodes
    pub roots: Vec<usize>,
    pub parts: Vec<Part>,
    pub skins: Vec<Skin>,
    pub clips: Vec<Clip>,
    /// every node, a parent before its children
    order: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// "" when the file gives none
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// the rest pose, relative to the parent
    pub local: Mat4,
    /// `local` as translation, rotation, scale – what a clip's channels replace
    pub rest: (Vec3, Quat, Vec3),
    /// the rest pose in model space (the space of `Model::vertices`)
    pub world: Mat4,
    /// the skin its mesh is bound to
    pub skin: Option<usize>,
    /// a joint of some skin: a bone
    pub joint: bool,
}

impl Node {
    pub fn new(name: String, children: Vec<usize>, local: Mat4, skin: Option<usize>) -> Self {
        let (s, r, t) = local.to_scale_rotation_translation();
        Node { name, parent: None, children, local, rest: (t, r, s), world: local, skin, joint: false }
    }
}

/// One mesh primitive on a node: where its vertices and triangles lie in the model's flattened lists.
#[derive(Debug, Clone)]
pub struct Part {
    pub node: usize,
    /// the glTF mesh and the primitive in it
    pub mesh: usize,
    pub primitive: usize,
    /// into `Model::images`
    pub image: Option<usize>,
    /// into `Model::vertices`
    pub vertices: Range<u32>,
    /// into `Model::indices`
    pub indices: Range<u32>,
    pub skin: Option<usize>,
    /// per vertex of a skinned part (empty otherwise)
    pub influences: Vec<Influence>,
}

/// A vertex's four joints (indices into its skin's `joints`) and their weights.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Influence {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub name: String,
    /// node indices
    pub joints: Vec<usize>,
    /// per joint: mesh space → the joint's space in the bind pose
    pub inverse_bind: Vec<Mat4>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    Translation,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Step,
    Linear,
    /// three values per key: the in-tangent, the value, the out-tangent
    Cubic,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub node: usize,
    pub path: Path,
    pub interpolation: Interpolation,
    /// seconds, rising
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>,
}

impl Channel {
    pub fn new(node: usize, path: Path, interpolation: Interpolation, times: Vec<f32>, values: Vec<[f32; 4]>) -> Result<Self> {
        let per_key = if interpolation == Interpolation::Cubic { 3 } else { 1 };
        ensure!(!times.is_empty(), "an animation channel without keys");
        ensure!(values.len() == times.len() * per_key, "an animation channel with {} keys and {} values", times.len(), values.len());
        ensure!(times.windows(2).all(|w| w[0] < w[1]), "an animation channel's times do not rise");
        Ok(Channel { node, path, interpolation, times, values })
    }

    fn key(&self, k: usize) -> [f32; 4] {
        self.values[if self.interpolation == Interpolation::Cubic { 3 * k + 1 } else { k }]
    }

    /// the channel's value at `t` seconds
    pub fn at(&self, t: f32) -> [f32; 4] {
        let last = self.times.len() - 1;
        if t.is_nan() || t <= self.times[0] {
            return self.key(0);
        }
        if t >= self.times[last] {
            return self.key(last);
        }
        let k = self.times.partition_point(|&x| x <= t) - 1;
        let span = self.times[k + 1] - self.times[k];
        let s = (t - self.times[k]) / span;
        let (a, b) = (self.key(k), self.key(k + 1));
        match (self.interpolation, self.path) {
            (Interpolation::Step, _) => a,
            (Interpolation::Linear, Path::Rotation) => Quat::from_array(a).normalize().slerp(Quat::from_array(b).normalize(), s).to_array(),
            (Interpolation::Linear, _) => std::array::from_fn(|i| a[i] + (b[i] - a[i]) * s),
            (Interpolation::Cubic, path) => {
                let (out, into) = (self.values[3 * k + 2], self.values[3 * k + 3]);
                let (s2, s3) = (s * s, s * s * s);
                let v: [f32; 4] = std::array::from_fn(|i| {
                    (2.0 * s3 - 3.0 * s2 + 1.0) * a[i] + (s3 - 2.0 * s2 + s) * span * out[i] + (-2.0 * s3 + 3.0 * s2) * b[i] + (s3 - s2) * span * into[i]
                });
                if path == Path::Rotation {
                    Quat::from_array(v).normalize().to_array()
                } else {
                    v
                }
            }
        }
    }
}

const NONE: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub struct Clip {
    /// "" when the file gives none
    pub name: String,
    /// seconds: the last key of any channel
    pub duration: f32,
    pub channels: Vec<Channel>,
    /// per node: its translation, rotation and scale channel (NONE: the rest pose's)
    by_node: Vec<[u32; 3]>,
}

impl Clip {
    fn new(name: String, channels: Vec<Channel>, nodes: usize) -> Result<Self> {
        let mut by_node = vec![[NONE; 3]; nodes];
        for (i, c) in channels.iter().enumerate() {
            ensure!(c.node < nodes, "the animation {name:?} moves node {}, which the file does not have", c.node);
            by_node[c.node][c.path as usize] = i as u32;
        }
        let duration = channels.iter().map(|c| c.times[c.times.len() - 1]).fold(0.0, f32::max);
        Ok(Clip { name, duration, channels, by_node })
    }
}

impl Anatomy {
    /// The tree checked and linked: parents from the children lists, the rest pose's world matrices, the joints marked.
    pub fn new(mut nodes: Vec<Node>, roots: Vec<usize>, parts: Vec<Part>, skins: Vec<Skin>, clips: Vec<(String, Vec<Channel>)>) -> Result<Self> {
        let count = nodes.len();
        for n in 0..count {
            for c in std::mem::take(&mut nodes[n].children) {
                ensure!(c < count, "node {n} names the child {c}, which the file does not have");
                // a node under two parents is not a tree: the first one keeps it
                if nodes[c].parent.is_none() && c != n {
                    nodes[c].parent = Some(n);
                    nodes[n].children.push(c);
                }
            }
        }
        let mut order: Vec<usize> = (0..count).filter(|&n| nodes[n].parent.is_none()).collect();
        let mut at = 0;
        while at < order.len() {
            let n = order[at];
            if let Some(p) = nodes[n].parent {
                nodes[n].world = nodes[p].world * nodes[n].local;
            }
            order.extend_from_slice(&nodes[n].children);
            at += 1;
        }
        if order.len() != count {
            bail!("the node tree runs in a circle");
        }
        for s in &skins {
            ensure!(s.inverse_bind.len() == s.joints.len(), "the skin {:?} has {} joints and {} inverse bind matrices", s.name, s.joints.len(), s.inverse_bind.len());
            for &j in &s.joints {
                ensure!(j < count, "the skin {:?} names the joint {j}, which the file does not have", s.name);
                nodes[j].joint = true;
            }
        }
        let clips = clips.into_iter().map(|(name, channels)| Clip::new(name, channels, count)).collect::<Result<_>>()?;
        Ok(Anatomy { nodes, roots, parts, skins, clips, order })
    }

    /// the first node of that name
    pub fn node(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    /// the first joint of that name
    pub fn bone(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.joint && n.name == name)
    }

    pub fn clip(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    /// the joints' names, skin by skin
    pub fn bones(&self) -> impl Iterator<Item = &str> {
        self.skins.iter().flat_map(|s| &s.joints).map(|&j| self.nodes[j].name.as_str())
    }

    /// A node's matrix relative to its parent at `t` seconds of a clip (None: the rest pose).
    pub fn local(&self, node: usize, clip: Option<usize>, t: f32) -> Mat4 {
        let n = &self.nodes[node];
        let Some(clip) = clip.and_then(|c| self.clips.get(c)) else { return n.local };
        let [translation, rotation, scale] = clip.by_node[node];
        if translation == NONE && rotation == NONE && scale == NONE {
            return n.local;
        }
        let at = |channel: u32| clip.channels[channel as usize].at(t);
        let (t0, r0, s0) = n.rest;
        let v3 = |channel: u32, rest: Vec3| if channel == NONE { rest } else { Vec3::from_slice(&at(channel)[..3]) };
        let r = if rotation == NONE { r0 } else { Quat::from_array(at(rotation)) };
        Mat4::from_scale_rotation_translation(v3(scale, s0), r, v3(translation, t0))
    }

    /// A node's matrix in model space at `t` seconds of a clip (None: the rest pose). Walks up the parents; no allocation.
    pub fn node_world(&self, node: usize, clip: Option<usize>, t: f32) -> Mat4 {
        if clip.is_none_or(|c| c >= self.clips.len()) {
            return self.nodes[node].world;
        }
        let mut m = self.local(node, clip, t);
        let mut up = self.nodes[node].parent;
        while let Some(p) = up {
            m = self.local(p, clip, t) * m;
            up = self.nodes[p].parent;
        }
        m
    }

    /// A bone's matrix in model space at `t` seconds of a clip (None: the rest pose); None when the model has no such
    /// bone or clip.
    pub fn bone_world(&self, bone: &str, clip: Option<&str>, t: f32) -> Option<Mat4> {
        let clip = match clip {
            Some(name) => Some(self.clip(name)?),
            None => None,
        };
        Some(self.node_world(self.bone(bone)?, clip, t))
    }

    /// Every node's matrix in model space at `t` seconds of a clip, into `out` (one per node, the caller's buffer).
    pub fn pose(&self, clip: Option<usize>, t: f32, out: &mut [Mat4]) {
        assert_eq!(out.len(), self.nodes.len(), "one matrix per node");
        for &n in &self.order {
            let local = self.local(n, clip, t);
            out[n] = match self.nodes[n].parent {
                Some(p) => out[p] * local,
                None => local,
            };
        }
    }
}

#[cfg(test)]
#[path = "../../tests/fixtures/bones.rs"]
mod bones;

#[cfg(test)]
mod tests {
    use super::bones::{self, close, REFERENCE};
    use super::*;
    use crate::space::glb;

    #[test]
    fn the_two_bone_model_keeps_its_tree_parts_skin_and_clip() {
        let model = glb::parse_with(&bones::glb(), 2048, &[], &mut |_, _| bail!("no images")).unwrap();
        let a = &model.anatomy;
        assert_eq!(a.nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["rig", "upper", "lower", "body"]);
        assert_eq!(a.roots, [0]);
        assert_eq!(a.nodes[2].parent, Some(1));
        assert_eq!(a.nodes[0].children, [1, 3]);
        assert_eq!(a.bones().collect::<Vec<_>>(), ["upper", "lower"]);
        assert_eq!((a.bone("rig"), a.node("rig"), a.bone("lower")), (None, Some(0), Some(2)), "a bone is a skin's joint");

        let [part] = &a.parts[..] else { panic!("one part: {:?}", a.parts) };
        assert_eq!((part.node, part.mesh, part.primitive, part.skin), (3, 0, 0, Some(0)));
        assert_eq!((part.vertices.clone(), part.indices.clone()), (0..3, 0..3));
        assert_eq!(part.influences[2], Influence { joints: [0, 1, 0, 0], weights: [0.25, 0.75, 0.0, 0.0] });
        assert_eq!(model.vertices[2].pos, [0.0, 3.0, 0.0], "the flattened list is unchanged: the rest pose");

        let skin = &a.skins[0];
        assert_eq!(skin.joints, [1, 2]);
        for (&j, ibm) in skin.joints.iter().zip(&skin.inverse_bind) {
            assert!(close(&(a.nodes[j].world * *ibm).to_cols_array(), &Mat4::IDENTITY.to_cols_array()), "rest world × inverse bind");
        }

        let clip = &a.clips[a.clip("wave").unwrap()];
        assert_eq!((clip.channels.len(), clip.duration), (3, 2.0));
        assert_eq!(clip.channels.iter().map(|c| (c.node, c.path, c.interpolation)).collect::<Vec<_>>(), [
            (1, Path::Rotation, Interpolation::Linear),
            (2, Path::Translation, Interpolation::Step),
            (2, Path::Scale, Interpolation::Cubic),
        ]);
    }

    #[test]
    fn a_bone_s_world_matrix_at_a_time_matches_the_reference() {
        let a = glb::anatomy(&bones::glb()).unwrap();
        for (bone, clip, t, want) in REFERENCE {
            let got = a.bone_world(bone, *clip, *t).unwrap().to_cols_array();
            assert!(close(&got, want), "{bone} in {clip:?} at {t}: {got:?}, the reference says {want:?}");
        }
        assert!(a.bone_world("rig", None, 0.0).is_none() && a.bone_world("upper", Some("jump"), 0.0).is_none());
        assert_eq!(a.bone_world("lower", Some("wave"), f32::NAN), a.bone_world("lower", Some("wave"), 0.0), "no time: the first key");
    }

    #[test]
    fn the_mannequin_s_bones_match_the_reference_export_when_present() {
        let Some(file) = bones::media("scutoons/3d/mannequin-more-legs.glb") else { return };
        let a = glb::anatomy(&file).unwrap();
        assert_eq!((a.skins[0].joints.len(), a.clips.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()), (24, vec!["Survey", "Walk", "Run"]));
        let export = bones::export(bones::MANNEQUIN);
        assert_eq!(export.len(), 24 * 10);
        for (bone, clip, t, want) in export {
            let got = a.bone_world(bone, clip, t).unwrap().to_cols_array();
            assert!(close(&got, &want), "{bone} in {clip:?} at {t}: {got:?}, the reference says {want:?}");
        }
    }

    #[test]
    fn catpope_loads_with_its_joint_names_when_present() {
        let Some(file) = bones::media("characters/catpope/catpope-regular.glb") else { return };
        let model = glb::parse_with(&file, 1, &[], &mut |_, _| Ok(image::RgbaImage::new(1, 1))).unwrap();
        let a = &model.anatomy;
        let names = ["root", "pelvis", "spine", "chest", "neck", "head", "robe", "shL", "elL", "wrL", "hipL", "knL", "anL", "shR", "elR", "wrR", "hipR", "knR", "anR"];
        assert_eq!(a.bones().collect::<Vec<_>>(), names);
        let skinned: Vec<&Part> = a.parts.iter().filter(|p| p.skin == Some(0)).collect();
        assert!(!skinned.is_empty() && skinned.iter().all(|p| p.influences.len() == p.vertices.len()), "every vertex of the body is weighted");
        assert_eq!(a.parts.iter().map(|p| p.indices.len()).sum::<usize>(), model.indices.len(), "the parts cover the draw list");
        let head = a.bone_world("head", None, 0.0).unwrap().w_axis.truncate();
        assert!(head.y > a.bone_world("pelvis", None, 0.0).unwrap().w_axis.y && model.min.y <= head.y && head.y <= model.max.y, "the head is over the pelvis, inside the model: {head}");
    }

    #[test]
    fn the_whole_pose_agrees_with_the_single_bone() {
        let a = glb::anatomy(&bones::glb()).unwrap();
        let mut pose = vec![Mat4::ZERO; a.nodes.len()];
        for t in [0.0, 0.5, 1.3, 2.0] {
            a.pose(a.clip("wave"), t, &mut pose);
            for (n, m) in pose.iter().enumerate() {
                assert!(close(&m.to_cols_array(), &a.node_world(n, a.clip("wave"), t).to_cols_array()), "node {n} at {t}");
            }
        }
        a.pose(None, 0.0, &mut pose);
        assert!(pose.iter().zip(&a.nodes).all(|(m, n)| *m == n.world));
    }

    #[test]
    fn a_circle_of_nodes_and_a_channel_out_of_step_are_errors() {
        let node = |children: Vec<usize>| Node::new(String::new(), children, Mat4::IDENTITY, None);
        let e = Anatomy::new(vec![node(vec![1]), node(vec![0])], vec![0], vec![], vec![], vec![]).err().unwrap();
        assert!(format!("{e:#}").contains("circle"), "{e:#}");
        assert!(Channel::new(0, Path::Scale, Interpolation::Cubic, vec![0.0, 1.0], vec![[1.0; 4]; 2]).is_err());
        assert!(Channel::new(0, Path::Scale, Interpolation::Linear, vec![1.0, 1.0], vec![[1.0; 4]; 2]).is_err());
    }
}
