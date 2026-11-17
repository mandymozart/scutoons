//! A scene's camera in an editor (the DOP panel): shots set from the view, the lens, the trolley track and its points,
//! the named moves. Every change goes through `SceneEdit::change`: the core's checks refuse it in their words.
use crate::scene_camera::{Lens, MoveDef, MoveKind, MoveSpec, SceneCamera, ShotDef, Source, Track, TrackPoint};
use crate::scene_edit::SceneEdit;

impl SceneEdit {
    /// the scene's camera as edited (none yet: the default)
    pub fn camera(&self) -> SceneCamera {
        self.doc.camera.clone().unwrap_or_default()
    }

    fn camera_change(&mut self, f: impl FnOnce(&mut SceneCamera) -> Result<(), String>) -> Result<(), String> {
        self.change(|d| {
            let mut c = d.camera.take().unwrap_or_default();
            let r = f(&mut c);
            d.camera = (c != SceneCamera::default()).then_some(c);
            r
        })
    }

    pub fn set_source(&mut self, source: Source) -> Result<(), String> {
        self.camera_change(|c| {
            c.source = source;
            Ok(())
        })
    }

    /// a shot taken from the view (its id is made here): its place among the shots
    pub fn add_shot(&mut self, shot: ShotDef) -> Result<usize, String> {
        let mut at = 0;
        self.camera_change(|c| {
            let id = c.free_id("shot");
            c.shots.push(ShotDef { id, ..shot });
            at = c.shots.len() - 1;
            Ok(())
        })?;
        Ok(at)
    }

    /// shot `i` set to `view` – its id and label stay
    pub fn set_shot(&mut self, i: usize, view: ShotDef) -> Result<(), String> {
        self.camera_change(|c| {
            let s = c.shots.get_mut(i).ok_or("no such shot")?;
            *s = ShotDef { id: s.id.clone(), label: s.label.clone(), ..view };
            Ok(())
        })
    }

    pub fn remove_shot(&mut self, i: usize) -> Result<(), String> {
        self.camera_change(|c| {
            if i >= c.shots.len() {
                return Err("no such shot".into());
            }
            c.shots.remove(i);
            Ok(())
        })
    }

    /// the lens of shot `i`, or of the camera itself (none)
    pub fn set_lens(&mut self, shot: Option<usize>, lens: Lens) -> Result<(), String> {
        self.camera_change(|c| {
            match shot {
                Some(i) => c.shots.get_mut(i).ok_or("no such shot")?.lens = lens,
                None => c.lens = lens,
            }
            Ok(())
        })
    }

    /// the eye height of shot `i` (level at its aim, the bird looking down), or of the primary camera (none), `height` m
    /// above a floor at `floor`
    pub fn set_height(&mut self, shot: Option<usize>, height: f32, floor: f32) -> Result<(), String> {
        self.camera_change(|c| {
            match shot {
                Some(i) => {
                    let s = c.shots.get_mut(i).ok_or("no such shot")?;
                    *s = s.at_height(height, floor);
                }
                None => c.height = height,
            }
            Ok(())
        })
    }

    /// a straight track between two points
    pub fn add_track(&mut self, start: [f32; 3], end: [f32; 3]) -> Result<(), String> {
        self.camera_change(|c| {
            c.track = Some(Track::between(start, end));
            Ok(())
        })
    }

    pub fn remove_track(&mut self) -> Result<(), String> {
        self.camera_change(|c| {
            c.track = None;
            Ok(())
        })
    }

    /// a point halfway between point `i` and the next: its place
    pub fn add_track_point(&mut self, i: usize) -> Result<usize, String> {
        self.camera_change(|c| {
            let t = c.track.as_mut().ok_or("the scene has no track")?;
            let (a, b) = (t.points.get(i).ok_or("no such point")?.at, t.points.get(i + 1).ok_or("no point after it")?.at);
            t.points.insert(i + 1, TrackPoint { at: [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5], handle: None });
            Ok(())
        })?;
        Ok(i + 1)
    }

    pub fn move_track_point(&mut self, i: usize, at: [f32; 3]) -> Result<(), String> {
        self.camera_change(|c| {
            c.track.as_mut().and_then(|t| t.points.get_mut(i)).ok_or("no such point")?.at = at;
            Ok(())
        })
    }

    /// point `i`'s Bézier handle (none = smooth through its neighbours)
    pub fn set_handle(&mut self, i: usize, handle: Option<[f32; 3]>) -> Result<(), String> {
        self.camera_change(|c| {
            c.track.as_mut().and_then(|t| t.points.get_mut(i)).ok_or("no such point")?.handle = handle;
            Ok(())
        })
    }

    /// a point between start and end taken out (the track keeps its two ends)
    pub fn remove_track_point(&mut self, i: usize) -> Result<(), String> {
        self.camera_change(|c| {
            let t = c.track.as_mut().ok_or("the scene has no track")?;
            if i == 0 || i + 1 >= t.points.len() {
                return Err("the track's start and end stay – remove the track instead".into());
            }
            t.points.remove(i);
            Ok(())
        })
    }

    /// a named move to shot `to`: its place among the moves
    pub fn add_move(&mut self, spec: MoveSpec, to: &str) -> Result<usize, String> {
        let mut at = 0;
        let stem = match spec.kind {
            MoveKind::CraneDown | MoveKind::CraneUp => "crane",
            MoveKind::DollyZoom => "vertigo",
            k => k.word(),
        }
        .replace(' ', "_");
        self.camera_change(|c| {
            let id = c.free_id(&stem);
            c.moves.push(MoveDef { id, to: to.to_string(), spec, then: None });
            at = c.moves.len() - 1;
            Ok(())
        })?;
        Ok(at)
    }

    pub fn set_move(&mut self, i: usize, m: MoveDef) -> Result<(), String> {
        self.camera_change(|c| {
            *c.moves.get_mut(i).ok_or("no such move")? = m;
            Ok(())
        })
    }

    pub fn remove_move(&mut self, i: usize) -> Result<(), String> {
        self.camera_change(|c| {
            if i >= c.moves.len() {
                return Err("no such move".into());
            }
            let id = c.moves.remove(i).id;
            for m in &mut c.moves {
                if m.then.as_deref() == Some(id.as_str()) {
                    m.then = None;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::scene_camera::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_edit::SceneEdit;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn view(yaw: f32) -> ShotDef {
        ShotDef { id: String::new(), label: Default::default(), target: [0.0, 1.0, 0.0], yaw, pitch: 10.0, distance: 3.0, lens: Lens::default() }
    }

    #[test]
    fn shots_lens_track_and_moves_are_edited_and_refused_in_the_core_s_words() {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        let (mut e, _) = SceneEdit::open(doc, None, &core_rigs, &no_sets).unwrap();
        assert_eq!(e.doc.camera, None);
        assert_eq!(e.add_shot(view(10.0)), Ok(0));
        assert_eq!(e.add_shot(view(-40.0)), Ok(1));
        assert_eq!(e.camera().shots.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["shot1", "shot2"]);
        e.set_shot(0, view(25.0)).unwrap();
        assert_eq!((e.camera().shots[0].id.as_str(), e.camera().shots[0].yaw), ("shot1", 25.0), "set from the view: the id stays");
        e.set_lens(Some(1), Lens::orthographic(6.0)).unwrap();
        assert!(e.set_lens(Some(1), Lens::focal(2.0)).unwrap_err().contains("focal length"), "the core's words");
        e.add_track([-2.0, 1.5, 3.0], [2.0, 1.5, 3.0]).unwrap();
        assert_eq!(e.add_track_point(0), Ok(1));
        e.move_track_point(1, [0.0, 2.5, 1.0]).unwrap();
        e.set_handle(1, Some([1.0, 0.0, 0.0])).unwrap();
        assert!(e.remove_track_point(0).is_err(), "the start stays");
        let fly = e.add_move(MoveSpec::of(MoveKind::Fly), "shot2").unwrap();
        let crane = e.add_move(MoveSpec::of(MoveKind::CraneDown), "shot1").unwrap();
        assert_eq!(e.camera().moves[crane].id, "crane1");
        let mut m = e.camera().moves[fly].clone();
        m.then = Some("crane1".into());
        e.set_move(fly, m).unwrap();
        assert!(e.remove_shot(1).unwrap_err().contains("no shot of the camera"), "a move ends on it");
        assert!(e.remove_track().unwrap_err().contains("has none"), "a fly-through runs on it");
        e.remove_move(crane).unwrap();
        assert_eq!(e.camera().moves[0].then, None, "a chain to a removed move ends");
        let played = e.resolved(&core_rigs, &no_sets).unwrap().scene;
        assert_eq!(played.camera, e.doc.camera, "the stage plays it");
        assert!(e.changed());
    }
}
