//! Every subject's two outlets of the scene camera's focus: how sharp it is (`in focus`, 0…1, scene_camera.rs `in_focus`)
//! and how far it is from the eye (`distance`, m). A rig reads them as `camera:focus` / `camera:distance` (rig/address.rs
//! `SOURCES`); flatten writes a node for each it reads, and the stage sets that node's value every frame. What they do –
//! a reverb, a light – is the wiring's, nothing here.
use super::values::Knobs;
use crate::graph::{Graph, Kind};
use crate::rig::address::{source_slot, CAMERA};
use crate::scene_camera::in_focus;
use glam::Vec3;

/// a subject's outlet nodes: whose (a cast member by its place in the cast; none = the space's own being) and the nodes
/// its `in focus` and `distance` go to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outlet {
    pub member: Option<usize>,
    pub focus: Option<usize>,
    pub distance: Option<usize>,
}

/// the node a rig's `camera:<name>` lands on (`camera_focus`), under a prefix
fn node_id(prefix: &str, name: &str) -> Option<String> {
    source_slot(CAMERA, name).and_then(|s| s.split_once('.')).map(|(n, _)| format!("{prefix}{n}"))
}

/// The outlets of `graph`: each `look` node's (in the cast's order, its nodes under its prefix) and, with no looks, the
/// space's own being's (a rig played alone, unprefixed). Only subjects whose rig reads one.
pub fn outlets(graph: &Graph) -> Vec<Outlet> {
    let find = |prefix: &str, name: &str| {
        let id = node_id(prefix, name)?;
        graph.nodes.iter().position(|n| n.id == id && n.kind == Kind::Add)
    };
    let of = |member: Option<usize>, prefix: &str| Some(Outlet { member, focus: find(prefix, "focus"), distance: find(prefix, "distance") }).filter(|o| o.focus.is_some() || o.distance.is_some());
    let looks: Vec<&str> = graph.nodes.iter().filter(|n| n.kind == Kind::Look).map(|n| n.id.strip_suffix(crate::scene::look_id("").as_str()).unwrap_or_default()).collect();
    if looks.is_empty() {
        return of(None, "").into_iter().collect();
    }
    looks.iter().enumerate().filter_map(|(i, p)| of(Some(i), p)).collect()
}

/// a subject at `at` seen from `eye` with the focus `focus` m away: its `in focus` and its distance (m)
pub fn values(eye: Vec3, focus: f32, at: Vec3) -> (f32, f32) {
    let d = eye.distance(at);
    (in_focus(d, focus), d)
}

/// The outlets' values into the control domain: each subject where `at` says it stands (none: left as it was).
pub fn push(outlets: &[Outlet], eye: Vec3, focus: f32, at: &dyn Fn(Option<usize>) -> Option<Vec3>, ctl: &mut dyn Knobs) {
    let Some(port) = Kind::Add.input("a") else { return };
    for o in outlets {
        let Some(p) = at(o.member) else { continue };
        let (sharp, d) = values(eye, focus, p);
        for (node, v) in [(o.focus, sharp), (o.distance, d)] {
            if let Some(n) = node {
                ctl.set(n, port, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;
    use crate::space::values::Table;

    fn graph(text: &str) -> Graph {
        Graph::build(&Scene::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn each_subject_s_outlets_carry_how_sharp_and_how_far_it_is() {
        let g = graph("[[node]]\nid=\"space\"\nkind=\"space\"\n[[node]]\nid=\"camera_focus\"\nkind=\"add\"\n[[node]]\nid=\"camera_distance\"\nkind=\"add\"\n");
        let o = outlets(&g);
        assert_eq!(o.len(), 1, "a rig played alone: the space's own being");
        assert_eq!(o[0].member, None);
        let mut t = Table::new(&g);
        let (f, d) = (o[0].focus.unwrap(), o[0].distance.unwrap());
        push(&o, Vec3::ZERO, 4.0, &|_| Some(Vec3::new(0.0, 0.0, -4.0)), &mut t);
        assert_eq!((t.input(f, "a", -1.0), t.input(d, "a", -1.0)), (1.0, 4.0), "at the focus: sharp, 4 m");
        push(&o, Vec3::ZERO, 4.0, &|_| Some(Vec3::new(0.0, 0.0, -8.0)), &mut t);
        assert_eq!((t.input(f, "a", -1.0), t.input(d, "a", -1.0)), (0.0, 8.0), "twice as far: out of focus");
        push(&o, Vec3::ZERO, 4.0, &|_| None, &mut t);
        assert_eq!(t.input(d, "a", -1.0), 8.0, "a subject not on the stage: left as it was");
        assert!(outlets(&graph("[[node]]\nid=\"space\"\nkind=\"space\"\n")).is_empty(), "no rig reads them: nothing to set");
    }
}
