//! A character's own 3D model on the stage: a cast member whose form is its look and whose character has a model
//! (scene-content's `model`) is drawn as that model – skinned on the GPU by a joint palette per member, posed by its
//! idle (an animation asset by joint name, #817), else its first clip (looped), else its rest pose.
//!
//! - **The file** (glb.rs `figure_with`): every part kept in its own mesh space – the bind space its skin poses –
//!   with normals, UVs and the base colour texture (WebP, PNG, JPEG).
//! - **The palette:** each skin's joints (the joint's world matrix × its inverse bind), then each unskinned part's node
//!   (its vertices ride that node whole). At most `MAX_JOINTS`; a model with more is refused and its member keeps its being.
//! - **The fit:** the rest pose's bottom middle is the feet, its height scaled to the member's card height (cast.rs) –
//!   the same figure whether it stands as its card or its model; the Space turns it toward the scene camera's eye.
//! - **Loading:** `Figures` holds the models by character – and a rig's body by its model's path (rig/body.rs) – asked
//!   once: on a deck a thread reads them (`Source`); in the browser the page fetches what `requests` names and hands the
//!   parsed file in (`deliver`).
use super::anatomy::Anatomy;
use super::glb::{Draw, Image, Model};
use anyhow::{ensure, Result};
use glam::{Mat4, Vec3};
#[cfg(feature = "native")]
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

/// The most joints a figure poses: a member's palette is one uniform buffer of 4 × 4 matrices – 128 = 8 KiB, half of
/// WebGL2's guaranteed 16 KiB per uniform binding (the browser has no storage buffers in the vertex stage); a Mixamo
/// rig has 65.
pub const MAX_JOINTS: usize = 128;
/// a figure's texture's longer side at most
pub const MAX_TEXTURE: u32 = 1024;

/// One vertex of a figure, bind space (44 bytes): up to four palette entries and their weights (0..255, the shader
/// divides by their sum).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FigureVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joints: [u8; 4],
    pub weights: [u8; 4],
    /// the vertex colour × the material's base colour factor (sRGB-encoded bytes like the texture)
    pub color: [u8; 4],
}

/// what one palette entry follows
#[derive(Debug, Clone, Copy)]
enum Bone {
    Joint { node: usize, inverse_bind: Mat4 },
    Node(usize),
}

pub struct Figure {
    pub vertices: Vec<FigureVertex>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
    pub images: Vec<Image>,
    pub anatomy: Anatomy,
    bones: Vec<Bone>,
    /// the rest pose's bottom middle and height, model space
    pub feet: Vec3,
    pub height: f32,
    /// the clip it plays: its first; none = the rest pose
    pub clip: Option<usize>,
}

impl Figure {
    /// A model read in bind space (glb.rs `figure_with`) as a figure: its palette, its vertices weighted into it, its fit.
    pub fn of(model: Model) -> Result<Figure> {
        let a = &model.anatomy;
        let mut bones: Vec<Bone> = Vec::new();
        // a joint several skins share (one skeleton, a mesh per material) takes one palette slot
        let mut slots: Vec<Vec<usize>> = Vec::with_capacity(a.skins.len());
        for s in &a.skins {
            let skin = s.joints.iter().zip(&s.inverse_bind).map(|(&node, &inverse_bind)| {
                match bones.iter().position(|b| matches!(b, Bone::Joint { node: n, inverse_bind: m } if *n == node && m.abs_diff_eq(inverse_bind, 1e-5))) {
                    Some(i) => i,
                    None => {
                        bones.push(Bone::Joint { node, inverse_bind });
                        bones.len() - 1
                    }
                }
            });
            slots.push(skin.collect());
        }
        let node_slot = |bones: &mut Vec<Bone>, n: usize| match bones.iter().position(|b| matches!(b, Bone::Node(m) if *m == n)) {
            Some(i) => i,
            None => {
                bones.push(Bone::Node(n));
                bones.len() - 1
            }
        };
        let mut vertices: Vec<FigureVertex> = model
            .vertices
            .iter()
            .zip(model.normals.iter().chain(std::iter::repeat(&[0.0; 3])))
            .map(|(v, n)| FigureVertex { pos: v.pos, normal: *n, uv: v.uv, joints: [0; 4], weights: [255, 0, 0, 0], color: v.color })
            .collect();
        for p in &a.parts {
            let range = p.vertices.start as usize..p.vertices.end as usize;
            match p.skin.and_then(|s| slots.get(s)) {
                Some(skin) if p.influences.len() == range.len() => {
                    for (v, i) in vertices[range].iter_mut().zip(&p.influences) {
                        v.joints = i.joints.map(|j| skin.get(j as usize).copied().unwrap_or(0).min(u8::MAX as usize) as u8);
                        v.weights = quantized(i.weights);
                    }
                }
                _ => {
                    let slot = node_slot(&mut bones, p.node);
                    for v in &mut vertices[range] {
                        v.joints = [slot.min(u8::MAX as usize) as u8, 0, 0, 0];
                        v.weights = [255, 0, 0, 0];
                    }
                }
            }
        }
        ensure!(bones.len() <= MAX_JOINTS, "{} joints – a figure poses at most {MAX_JOINTS}", bones.len());
        ensure!(!bones.is_empty(), "a model without parts");
        smooth_normals(&mut vertices, &model.indices);
        let mut figure = Figure {
            vertices,
            indices: model.indices,
            draws: model.draws,
            images: model.images,
            anatomy: model.anatomy,
            bones,
            feet: Vec3::ZERO,
            height: 0.0,
            clip: None,
        };
        figure.clip = (!figure.anatomy.clips.is_empty()).then_some(0);
        let (min, max) = figure.rest_bounds();
        figure.height = max.y - min.y;
        ensure!(figure.height > 1e-6, "a model without height");
        figure.feet = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
        Ok(figure)
    }

    /// how many palette entries it poses
    pub fn joints(&self) -> usize {
        self.bones.len()
    }

    /// its clip's length (s); 0 = it stands in its rest pose
    pub fn duration(&self) -> f32 {
        self.clip.and_then(|c| self.anatomy.clips.get(c)).map_or(0.0, |c| c.duration)
    }

    /// The clip a member plays, chosen by name (#817: a holder's `clip`, else its persona's idle clip): none = its own
    /// (the idle it adopted, else its first clip); a name the file lacks is the sentence why – deck-catalog
    /// `Skeleton::clip_problem`, the rule the creator and the API ask too.
    pub fn choose(&self, name: Option<&str>) -> Result<Option<usize>, String> {
        let Some(name) = name else { return Ok(self.clip) };
        match self.anatomy.clip(name) {
            Some(c) => Ok(Some(c)),
            None => Err(self.anatomy.skeleton().clip_problem(name).unwrap_or_default()),
        }
    }

    /// The palette at `t` seconds (its clip looped) under `world`: `joints()` matrices into `out`, `nodes` the caller's
    /// buffer (one per node of its anatomy). No allocation.
    pub fn pose(&self, t: f32, world: Mat4, nodes: &mut [Mat4], out: &mut [[f32; 16]]) -> usize {
        self.pose_clip(self.clip, t, world, nodes, out)
    }

    /// `pose` with the clip chosen (`choose`; none = the rest pose)
    pub fn pose_clip(&self, clip: Option<usize>, t: f32, world: Mat4, nodes: &mut [Mat4], out: &mut [[f32; 16]]) -> usize {
        let d = clip.and_then(|c| self.anatomy.clips.get(c)).map_or(0.0, |c| c.duration);
        let t = if d > 0.0 && t.is_finite() { t.rem_euclid(d) } else { 0.0 };
        self.anatomy.pose(clip, t, nodes);
        let n = self.bones.len().min(out.len());
        for (o, b) in out.iter_mut().zip(&self.bones).take(n) {
            let m = match *b {
                Bone::Joint { node, inverse_bind } => nodes[node] * inverse_bind,
                Bone::Node(node) => nodes[node],
            };
            *o = (world * m).to_cols_array();
        }
        n
    }

    /// every vertex in the rest pose, through its weighted palette (model space)
    pub fn rest_positions(&self) -> Vec<Vec3> {
        let mut nodes = vec![Mat4::IDENTITY; self.anatomy.nodes.len()];
        let mut palette = vec![[0.0; 16]; self.bones.len()];
        self.anatomy.pose(None, 0.0, &mut nodes);
        let n = self.pose(0.0, Mat4::IDENTITY, &mut nodes, &mut palette);
        let palette: Vec<Mat4> = palette[..n].iter().map(Mat4::from_cols_array).collect();
        self.vertices
            .iter()
            .map(|v| {
                let sum = v.weights.iter().map(|&w| w as f32).sum::<f32>().max(1.0);
                let m = v.joints.iter().zip(v.weights).fold(Mat4::ZERO, |m, (&j, w)| m + palette.get(j as usize).copied().unwrap_or(Mat4::IDENTITY) * (w as f32 / sum));
                m.transform_point3(Vec3::from(v.pos))
            })
            .collect()
    }

    /// the rest pose's bounds
    fn rest_bounds(&self) -> (Vec3, Vec3) {
        self.rest_positions().iter().fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| (lo.min(*p), hi.max(*p)))
    }
}

/// four weights as bytes summing to 255 (the rounding's rest on the largest)
fn quantized(w: [f32; 4]) -> [u8; 4] {
    let sum: f32 = w.iter().map(|x| x.max(0.0)).sum();
    if sum <= 0.0 {
        return [255, 0, 0, 0];
    }
    let mut q = w.map(|x| (x.max(0.0) / sum * 255.0).round() as i32);
    let rest = 255 - q.iter().sum::<i32>();
    let top = (0..4).max_by(|&a, &b| q[a].cmp(&q[b])).unwrap_or(0);
    q[top] += rest;
    q.map(|x| x.clamp(0, 255) as u8)
}

/// the vertices the file gave no normal: the area-weighted mean of their triangles' faces
fn smooth_normals(v: &mut [FigureVertex], indices: &[u32]) {
    let missing: Vec<bool> = v.iter().map(|x| x.normal == [0.0; 3]).collect();
    if !missing.contains(&true) {
        return;
    }
    let mut sum = vec![Vec3::ZERO; v.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
        let n = (Vec3::from(v[b].pos) - Vec3::from(v[a].pos)).cross(Vec3::from(v[c].pos) - Vec3::from(v[a].pos));
        for i in [a, b, c] {
            sum[i] += n;
        }
    }
    for (i, x) in v.iter_mut().enumerate().filter(|(i, _)| missing[*i]) {
        x.normal = sum[i].normalize_or(Vec3::Y).into();
    }
}

/// What a figure's file is: a character's model, or a rig's body by its model's CDN path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// the character whose model it is (push.rs `drawn_as`) – the scene-content entry a page looks up
    Character(String),
    /// a rig's body: its model's path from the CDN's root
    Body(String),
}

impl Ask {
    /// where a page finds it: scene-content's entry kind (`characters`) and id, or "" and the file's path
    pub fn kind(&self) -> &'static str {
        match self {
            Ask::Character(_) => "characters",
            Ask::Body(_) => "",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Ask::Character(id) | Ask::Body(id) => id,
        }
    }
}

/// where a deck's figures come from: a character's model file as it is, a rig's body by its path
#[cfg(feature = "native")]
pub trait Source: Send + 'static {
    fn load(&self, character: &str) -> Result<Vec<u8>, String>;
    /// the character's idle (#817): a shared animation its model plays by joint name; None = its own first clip
    fn idle(&self, _character: &str) -> Option<Vec<u8>> {
        None
    }
    /// a rig's body: its model by its CDN path
    fn body(&self, path: &str) -> Result<Vec<u8>, String> {
        Err(format!("no body {path} here"))
    }
}

/// a local folder: `<dir>/<char>/<char>_model.glb` (the sync's layout), else `<dir>/<char>/<char>-regular.glb` (media/)
#[cfg(feature = "native")]
pub struct LocalDir(pub PathBuf);

#[cfg(feature = "native")]
impl Source for LocalDir {
    fn load(&self, character: &str) -> Result<Vec<u8>, String> {
        let dir = self.0.join(character);
        let path = [format!("{character}_model.glb"), format!("{character}-regular.glb")].into_iter().map(|f| dir.join(f)).find(|p| p.is_file());
        let path = path.ok_or_else(|| format!("no model of {character} in {}", self.0.display()))?;
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// `<dir>/<char>/<char>_idle.glb` (the sync's layout)
    fn idle(&self, character: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(character).join(format!("{character}_idle.glb"))).ok()
    }

    /// the path below the folder (the sync's media layout), else below its parent (the media copy, `media/characters/…`)
    fn body(&self, path: &str) -> Result<Vec<u8>, String> {
        let dirs = std::iter::once(self.0.as_path()).chain(self.0.parent());
        let file = dirs.filter_map(|d| crate::push::media_file(d, path)).find(|f| f.is_file());
        let file = file.ok_or_else(|| format!("no body {path} in {}", self.0.display()))?;
        std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))
    }
}

/// a folder whose files arrive from elsewhere (a deck's sync): `wait` for a character's model or a body, then read it
#[cfg(feature = "native")]
pub struct Arriving<W> {
    pub dir: LocalDir,
    pub wait: W,
}

#[cfg(feature = "native")]
impl<W: Fn(&Ask) + Send + 'static> Source for Arriving<W> {
    fn load(&self, character: &str) -> Result<Vec<u8>, String> {
        (self.wait)(&Ask::Character(character.to_string()));
        self.dir.load(character)
    }

    /// asked after `load`: the wait covers the idle too
    fn idle(&self, character: &str) -> Option<Vec<u8>> {
        self.dir.idle(character)
    }

    fn body(&self, path: &str) -> Result<Vec<u8>, String> {
        (self.wait)(&Ask::Body(path.to_string()));
        self.dir.body(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Loading,
    Ready,
    Missing(String),
}

pub struct Slot {
    /// the member's character, or the body's path
    pub key: String,
    /// its file
    pub ask: Ask,
    pub state: State,
    figure: Option<Figure>,
}

/// The figures the cast asked for, by slot (the renderer keeps a mesh per ready slot).
pub struct Figures {
    pub slots: Vec<Slot>,
    /// the loader thread's inbox; None = manual (the host asks: `requests`)
    ask: Option<Sender<(usize, Ask)>>,
    asked: Vec<usize>,
    got: Receiver<(usize, Result<Figure, String>)>,
    tell: Sender<(usize, Result<Figure, String>)>,
    /// ready, not yet on the GPU
    fresh: Vec<usize>,
}

/// A model's file → a figure, or why not. With an `idle` (an animation asset, #817) the figure plays it instead of its
/// own first clip: its rotations on the model's joints of the same name (anatomy.rs `adopt`); the joints it turns that
/// the model lacks stay at rest and are logged. An idle that cannot be read leaves the model's own clip.
pub fn read(bytes: &[u8], idle: Option<&[u8]>, decode: &mut dyn FnMut(usize, &[u8]) -> Result<image::RgbaImage>) -> Result<Figure> {
    let mut figure = Figure::of(super::glb::figure_with(bytes, MAX_TEXTURE, decode)?)?;
    let Some(idle) = idle else { return Ok(figure) };
    match super::glb::skeleton(idle).map(|source| figure.anatomy.adopt(&source)) {
        Ok(Some((clip, fit))) => {
            figure.clip = Some(clip);
            if let Some(why) = fit.problem() {
                tracing::warn!("figure: its idle plays on {} of {} joints – {why}", fit.matched, fit.of);
            }
        }
        Ok(None) => tracing::warn!("figure: its idle has no clip – the model's own plays"),
        Err(e) => tracing::warn!("figure: its idle can't be read ({e:#}) – the model's own clip plays"),
    }
    Ok(figure)
}

impl Figures {
    /// the loader thread over `source` (it ends with the Figures)
    #[cfg(feature = "native")]
    pub fn new(source: Box<dyn Source>) -> Figures {
        let (ask, asked) = mpsc::channel::<(usize, Ask)>();
        let (tell, got) = mpsc::channel();
        let teller = tell.clone();
        std::thread::Builder::new()
            .name("figures".into())
            .spawn(move || {
                while let Ok((slot, file)) = asked.recv() {
                    let character = file.id().to_string();
                    let decode = &mut |_: usize, b: &[u8]| Ok(image::load_from_memory(b)?.to_rgba8());
                    let r = match &file {
                        Ask::Character(c) => source.load(c).and_then(|b| read(&b, source.idle(c).as_deref(), decode).map_err(|e| format!("{c}: {e:#}"))),
                        Ask::Body(path) => source.body(path).and_then(|b| read(&b, None, decode).map_err(|e| format!("{path}: {e:#}"))),
                    };
                    match &r {
                        Ok(f) => tracing::info!("figure {character}: {} triangles, {} joints, {}", f.indices.len() / 3, f.joints(), f.clip.and_then(|c| f.anatomy.clips.get(c)).map_or("its rest pose".to_string(), |c| format!("the clip {:?}", c.name))),
                        Err(e) => tracing::warn!("figure: {e} – the member keeps its being"),
                    }
                    if teller.send((slot, r)).is_err() {
                        break;
                    }
                }
            })
            .expect("a thread");
        Figures::with(Some(ask), got, tell)
    }

    fn with(ask: Option<Sender<(usize, Ask)>>, got: Receiver<(usize, Result<Figure, String>)>, tell: Sender<(usize, Result<Figure, String>)>) -> Figures {
        Figures { slots: Vec::new(), ask, asked: Vec::new(), got, tell, fresh: Vec::new() }
    }

    /// no loader: the host takes the requests and delivers the files (the browser)
    pub fn manual() -> Figures {
        let (tell, got) = mpsc::channel();
        Figures::with(None, got, tell)
    }

    /// The slot of `character`'s figure, `file_of` the character whose model it is (asked for once).
    pub fn request(&mut self, character: &str, file_of: &str) -> usize {
        self.ask_for(character, Ask::Character(file_of.to_string()))
    }

    /// The slot of a rig's body, its model by CDN path (asked for once: members with the same body share it).
    pub fn request_body(&mut self, path: &str) -> usize {
        self.ask_for(path, Ask::Body(path.to_string()))
    }

    fn ask_for(&mut self, key: &str, file: Ask) -> usize {
        if let Some(i) = self.slots.iter().position(|s| s.key == key && s.ask.kind() == file.kind()) {
            return i;
        }
        let i = self.slots.len();
        match &self.ask {
            Some(ask) => {
                let _ = ask.send((i, file.clone()));
            }
            None => self.asked.push(i),
        }
        self.slots.push(Slot { key: key.to_string(), ask: file, state: State::Loading, figure: None });
        i
    }

    /// the slots asked for since the last call, with the file to fetch – manual Figures only
    pub fn requests(&mut self) -> Vec<(usize, Ask)> {
        std::mem::take(&mut self.asked).into_iter().map(|i| (i, self.slots[i].ask.clone())).collect()
    }

    /// a requested figure, read by the host (`read`), or why there is none – it draws from the next `poll`
    pub fn deliver(&mut self, slot: usize, figure: Result<Figure, String>) {
        let _ = self.tell.send((slot, figure));
    }

    /// what the loader finished (each frame); true when a figure became ready (the renderer sizes its buffers anew)
    pub fn poll(&mut self) -> bool {
        let mut ready = false;
        while let Ok((slot, r)) = self.got.try_recv() {
            let Some(s) = self.slots.get_mut(slot) else { continue };
            match r {
                Ok(f) => {
                    s.figure = Some(f);
                    s.state = State::Ready;
                    self.fresh.push(slot);
                    ready = true;
                }
                Err(e) => s.state = State::Missing(e),
            }
        }
        ready
    }

    /// the figures for the renderer to upload, once
    pub fn take_fresh(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.fresh)
    }

    pub fn ready(&self, slot: usize) -> bool {
        self.slots.get(slot).is_some_and(|s| s.state == State::Ready)
    }

    pub fn get(&self, slot: usize) -> Option<&Figure> {
        self.slots.get(slot).and_then(|s| s.figure.as_ref())
    }
}

/// One member drawn as its figure this frame: its slot, where it stands (model → world), its look's light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FigureDraw {
    pub slot: usize,
    /// the clip it plays (`Figure::choose`)
    pub clip: Option<usize>,
    pub world: Mat4,
    pub glow: f32,
    pub hue: f32,
    pub pulse: f32,
}

/// a figure's model → world: its feet on `feet`, turned `yaw` about the vertical (its +z toward the eye at 0), its rest
/// height scaled to `height` (× a module's `scale`)
pub fn placed(f: &Figure, feet: Vec3, yaw: f32, height: f32, scale: Vec3) -> Mat4 {
    Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(scale * (height / f.height)) * Mat4::from_translation(-f.feet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::anatomy::bones;
    use anyhow::bail;
    use crate::space::anatomy::{Node, Skin};
    use crate::space::glb;

    fn no_images() -> impl FnMut(usize, &[u8]) -> Result<image::RgbaImage> {
        |_, _| bail!("no images")
    }

    #[test]
    fn the_two_bone_model_keeps_its_bind_space_and_skin() {
        let f = read(&bones::glb(), None, &mut no_images()).unwrap();
        // the body's node sits under "rig" at (0, 1, 0): a room flattens it there, a figure keeps its mesh space
        let room = glb::parse_with(&bones::glb(), 2048, &[], &mut no_images()).unwrap();
        assert_eq!(room.vertices[2].pos, [0.0, 3.0, 0.0]);
        assert_eq!(f.vertices[2].pos, [0.0, 2.0, 0.0], "bind space: the mesh's own");
        assert_eq!(f.joints(), 2);
        assert_eq!((f.vertices[2].joints, f.vertices[2].weights), ([0, 1, 0, 0], [64, 191, 0, 0]), "the weights 0.25 / 0.75 as bytes");
        assert_eq!(f.clip, Some(0), "its first clip plays");
        assert!((f.duration() - 2.0).abs() < 1e-6);
        assert!(f.vertices.iter().all(|v| (Vec3::from(v.normal).length() - 1.0).abs() < 1e-4), "normals made where the file has none");
    }

    #[test]
    fn an_idle_plays_instead_of_the_model_s_own_clip() {
        let idle = super::super::animation::extract(&bones::glb()).unwrap();
        let f = read(&bones::glb(), Some(&idle), &mut no_images()).unwrap();
        assert_eq!((f.clip, f.anatomy.clips.len(), f.anatomy.clips[1].channels.len()), (Some(1), 2, 1), "the idle, rotations only");
        let broken = read(&bones::glb(), Some(b"not a glb"), &mut no_images()).unwrap();
        assert_eq!(broken.clip, Some(0), "an idle that can't be read leaves the model's own clip");
    }

    #[test]
    fn a_clip_is_chosen_by_name() {
        let f = read(&bones::glb(), None, &mut no_images()).unwrap();
        assert_eq!((f.choose(None), f.choose(Some("wave"))), (Ok(Some(0)), Ok(Some(0))));
        assert_eq!(f.choose(Some("walk")), Err("no clip 'walk': the model has wave".to_string()));
    }

    #[test]
    fn the_rest_palette_is_the_identity_and_the_fit_stands_on_the_feet() {
        let f = read(&bones::glb(), None, &mut no_images()).unwrap();
        let mut nodes = vec![Mat4::IDENTITY; f.anatomy.nodes.len()];
        let mut out = [[0.0; 16]; MAX_JOINTS];
        let mut rest = f.clone_rest();
        rest.clip = None;
        let n = rest.pose(0.0, Mat4::IDENTITY, &mut nodes, &mut out);
        assert_eq!(n, 2, "within the cap: one entry per joint");
        for m in &out[..n] {
            assert!(bones::close(m, &Mat4::IDENTITY.to_cols_array()), "rest world × inverse bind: {m:?}");
        }
        let world = placed(&f, Vec3::new(5.0, 0.0, 0.0), 0.0, 1.7, Vec3::ONE);
        let feet = world.transform_point3(f.feet);
        assert!((feet - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5, "{feet}");
        let top = world.transform_point3(f.feet + Vec3::Y * f.height);
        assert!((top.y - 1.7).abs() < 1e-5, "the card's height: {top}");
    }

    #[test]
    fn a_palette_over_the_cap_is_refused() {
        let count = MAX_JOINTS + 2;
        let nodes = (0..count).map(|_| Node::new(String::new(), vec![], Mat4::IDENTITY, None)).collect();
        let skin = Skin { name: "big".into(), joints: (0..count).collect(), inverse_bind: vec![Mat4::IDENTITY; count] };
        let anatomy = Anatomy::new(nodes, (0..count).collect(), vec![], vec![skin], vec![]).unwrap();
        let mut model = glb::parse_with(&bones::glb(), 2048, &[], &mut no_images()).unwrap();
        model.anatomy = anatomy;
        let e = Figure::of(model).err().unwrap();
        assert!(format!("{e:#}").contains("at most 128"), "{e:#}");
    }

    /// one skeleton under ten skinned meshes (a mesh per material, as three.js exports): its joints count once
    #[test]
    fn skins_sharing_a_skeleton_share_its_palette() {
        let count = 17;
        let nodes = (0..count).map(|_| Node::new(String::new(), vec![], Mat4::IDENTITY, None)).collect();
        let skins = (0..10).map(|i| Skin { name: format!("mesh{i}"), joints: (0..count).collect(), inverse_bind: vec![Mat4::IDENTITY; count] }).collect();
        let anatomy = Anatomy::new(nodes, (0..count).collect(), vec![], skins, vec![]).unwrap();
        let mut model = glb::parse_with(&bones::glb(), 2048, &[], &mut no_images()).unwrap();
        model.anatomy = anatomy;
        let f = Figure::of(model).unwrap();
        assert_eq!(f.joints(), count);
    }

    #[test]
    fn weights_quantize_to_a_full_byte() {
        for w in [[0.25, 0.75, 0.0, 0.0], [0.333, 0.333, 0.334, 0.0], [0.1, 0.2, 0.3, 0.4], [0.0; 4]] {
            assert_eq!(quantized(w).iter().map(|&x| x as u32).sum::<u32>(), 255, "{w:?}");
        }
    }

    #[test]
    fn manual_figures_ask_once_and_take_what_the_host_delivers() {
        let mut figs = Figures::manual();
        let a = figs.request("boy", "mellowmantis");
        assert_eq!(figs.request("boy", "mellowmantis"), a);
        assert_eq!(figs.requests(), [(a, Ask::Character("mellowmantis".into()))], "the page fetches the default character's model");
        assert!(figs.requests().is_empty() && !figs.ready(a));
        figs.deliver(a, read(&bones::glb(), None, &mut no_images()).map_err(|e| e.to_string()));
        assert!(figs.poll() && figs.ready(a));
        assert_eq!(figs.take_fresh(), [a]);
        let b = figs.request("ghost", "ghost");
        figs.deliver(b, Err("404".into()));
        assert!(!figs.poll());
        assert_eq!(figs.slots[b].state, State::Missing("404".into()));
    }

    /// The default character's model (`SCUTOONS_MEDIA` or the media folder; skipped when it is not here): one skinned
    /// mesh with 65 joints, a JPEG texture, no clips – it stands in its rest pose, about a metre tall.
    #[test]
    fn mellowmantis_loads_when_present() {
        let Some(file) = bones::media("characters/mellowmantis/mellowmantis-regular.glb") else { return };
        let f = read(&file, None, &mut |_, b| Ok(image::load_from_memory(b)?.to_rgba8())).unwrap();
        assert_eq!((f.joints(), f.clip, f.images.len()), (65, None, 1));
        assert!(f.images[0].size().0 <= MAX_TEXTURE);
        assert!((0.5..2.0).contains(&f.height), "{}", f.height);
        assert!(f.vertices.iter().all(|v| (v.joints.iter().map(|&j| j as usize).max().unwrap_or(0)) < f.joints()));
        let mut nodes = vec![Mat4::IDENTITY; f.anatomy.nodes.len()];
        let mut out = [[0.0; 16]; MAX_JOINTS];
        assert_eq!(f.pose(3.0, Mat4::IDENTITY, &mut nodes, &mut out), 65);
    }

    impl Figure {
        /// a copy without its vertices (a test poses it differently)
        fn clone_rest(&self) -> Figure {
            Figure { vertices: Vec::new(), indices: Vec::new(), draws: Vec::new(), images: Vec::new(), anatomy: self.anatomy.clone(), bones: self.bones.clone(), feet: self.feet, height: self.height, clip: self.clip }
        }
    }
}
