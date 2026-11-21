//! A model's spots read from its anatomy (deck-catalog spot.rs holds the type and the rules): the empties named
//! `spot_*` / `port_*` become the file's spots, each attached to what carries it in the file – the bone or the named
//! node above it, else the model. `world` answers where a spot is at a clip's time.
use super::anatomy::Anatomy;
use deck_catalog::spot::{self, Attach, File, Spot};
use glam::{Mat4, Quat, Vec3};

/// What a model's file says about spots: the names a spot can attach to, and its own spots.
pub fn file(a: &Anatomy) -> File {
    let mut out = File::default();
    for (n, node) in a.nodes.iter().enumerate() {
        let empty = !node.joint && !a.parts.iter().any(|p| p.node == n);
        let Some(mut found) = empty.then(|| spot::of_file_name(&node.name)).flatten() else {
            if !node.name.is_empty() && !node.joint && !out.nodes.contains(&node.name) {
                out.nodes.push(node.name.clone());
            }
            continue;
        };
        // the first node of a name is the one an attachment finds
        let carrier = node.parent.filter(|&p| !a.nodes[p].name.is_empty() && a.node(&a.nodes[p].name) == Some(p));
        let (attach, m) = match carrier {
            Some(p) if a.nodes[p].joint => (Attach::Bone(a.nodes[p].name.clone()), node.local),
            Some(p) => (Attach::Node(a.nodes[p].name.clone()), node.local),
            None => (Attach::Root, node.world),
        };
        let (_, turn, at) = m.to_scale_rotation_translation();
        found.attach = attach;
        found.at = at.to_array();
        found.turn = turn.normalize().to_array();
        out.spots.push(found);
    }
    for bone in a.bones() {
        if !out.bones.iter().any(|b| b == bone) {
            out.bones.push(bone.to_string());
        }
    }
    out
}

/// A spot's matrix in model space at `t` seconds of a clip (None: the rest pose); None when the model has no such
/// node or bone. No allocation.
pub fn world(a: &Anatomy, s: &Spot, clip: Option<usize>, t: f32) -> Option<Mat4> {
    let carrier = match &s.attach {
        Attach::Root => Mat4::IDENTITY,
        Attach::Node(n) => a.node_world(a.node(n)?, clip, t),
        Attach::Bone(b) => a.node_world(a.bone(b)?, clip, t),
    };
    Some(carrier * Mat4::from_rotation_translation(Quat::from_array(s.turn).normalize(), Vec3::from_array(s.at)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::anatomy::{Channel, Interpolation, Node, Part, Path, Skin};
    use deck_catalog::spot::{Hand, Kind, Spots};

    /// "rig" one metre up carries the bone "arm" (one metre along x; the clip "lift" raises it two metres in a
    /// second), the empty `spot_base.light` and the mesh `port_mesh`; the arm carries the empty `port_out`, turned a
    /// quarter about y; beside the rig the empty `spot_floor`, the empty `view` and the unnamed node's `spot_lost`.
    fn model() -> Anatomy {
        let at = |x: f32, y: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z));
        let node = |name: &str, children: Vec<usize>, local: Mat4| Node::new(name.to_string(), children, local, None);
        let nodes = vec![
            node("rig", vec![1, 2, 3], at(0.0, 1.0, 0.0)),
            node("arm", vec![4], at(1.0, 0.0, 0.0)),
            node("spot_base.light", vec![], at(0.0, 0.0, 2.0)),
            node("port_mesh", vec![], Mat4::IDENTITY),
            node("port_out", vec![], Mat4::from_rotation_translation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), Vec3::new(0.0, 0.5, 0.0))),
            node("spot_floor", vec![], at(3.0, 0.0, 0.0)),
            node("view", vec![], Mat4::IDENTITY),
            node("", vec![8], at(0.0, 0.0, -1.0)),
            node("spot_lost", vec![], at(0.0, 2.0, 0.0)),
        ];
        let part = Part { node: 3, mesh: 0, primitive: 0, image: None, vertices: 0..0, indices: 0..0, skin: None, influences: vec![] };
        let skin = Skin { name: "s".into(), joints: vec![1], inverse_bind: vec![Mat4::IDENTITY] };
        let lift = Channel::new(1, Path::Translation, Interpolation::Linear, vec![0.0, 1.0], vec![[1.0, 0.0, 0.0, 0.0], [1.0, 2.0, 0.0, 0.0]]).unwrap();
        Anatomy::new(nodes, vec![0, 5, 6, 7], vec![part], vec![skin], vec![("lift".into(), vec![lift])]).unwrap()
    }

    #[test]
    fn the_empties_named_spot_and_port_are_the_file_s_spots() {
        let f = file(&model());
        assert_eq!(f.nodes, ["rig", "port_mesh", "view"], "what a spot can attach to: no bone, no spot, no nameless node");
        assert_eq!(f.bones, ["arm"]);
        let got: Vec<_> = f.spots.iter().map(|s| (s.name.as_str(), s.kind, s.attach.to_string(), s.at)).collect();
        assert_eq!(
            got,
            [
                ("base", Kind::Light, "node:rig".to_string(), [0.0, 0.0, 2.0]),
                ("out", Kind::Port, "bone:arm".to_string(), [0.0, 0.5, 0.0]),
                ("floor", Kind::Mark, "root".to_string(), [3.0, 0.0, 0.0]),
                ("lost", Kind::Mark, "root".to_string(), [0.0, 2.0, -1.0]),
            ],
            "a mesh named port_ is no spot; under a nameless node a spot stands in model space"
        );
        let quarter = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).to_array();
        assert!(f.spots[1].turn.iter().zip(quarter).all(|(a, b)| (a - b).abs() < 1e-6), "{:?}", f.spots[1].turn);
        assert_eq!(Spots { file: Some(f), hand: vec![] }.problems(None), Vec::<String>::new());
    }

    #[test]
    fn a_spot_follows_what_it_is_attached_to() {
        let a = model();
        let spots = Spots { file: Some(file(&a)), hand: vec![Hand { name: "tip".into(), attach: Some(Attach::Bone("arm".into())), at: Some([0.0, 0.0, 1.0]), ..Hand::default() }] };
        let lift = a.clip("lift");
        let place = |name: &str, clip, t| world(&a, &spots.spot(name).unwrap(), clip, t).unwrap().w_axis.truncate();
        assert_eq!(place("base", None, 0.0), Vec3::new(0.0, 1.0, 2.0));
        assert_eq!(place("out", None, 0.0), Vec3::new(1.0, 1.5, 0.0));
        assert_eq!(place("out", lift, 0.5), Vec3::new(1.0, 2.5, 0.0), "half way up with the bone");
        assert_eq!(place("tip", lift, 1.0), Vec3::new(1.0, 3.0, 1.0), "a hand spot on the bone too");
        assert_eq!(place("floor", lift, 1.0), Vec3::new(3.0, 0.0, 0.0), "on the model: it stays");
        let out = world(&a, &spots.spot("out").unwrap(), None, 0.0).unwrap();
        assert!((out.transform_vector3(Vec3::NEG_Z) - Vec3::NEG_X).length() < 1e-6, "it points along its own −z, turned a quarter about y");
        assert!(world(&a, &Spot { attach: Attach::Bone("rig".into()), ..Spot::new("x") }, None, 0.0).is_none(), "a node is no bone");
    }
}
