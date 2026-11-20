//! A scene's camera in an editor (the DOP panel): shots set from the view, the lens, the trolley track and its points,
//! the named moves, a phone's take kept as a move (with its end as a shot), how long a phone may go without a pose. Every change goes through `SceneEdit::change`: the core's checks refuse it in their words.
use crate::scene_camera::{Ease, Length, Lens, MoveDef, MoveKind, MoveSpec, Phones, SceneCamera, ShotDef, Source, Take, Track, TrackPoint};
use deck_ui::filter::{open_admits, Fact};
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

    /// shot `i` set to `view` – its id, label, aim, focus and phones stay
    pub fn set_shot(&mut self, i: usize, view: ShotDef) -> Result<(), String> {
        self.camera_change(|c| {
            let s = c.shots.get_mut(i).ok_or("no such shot")?;
            let lens = Lens { focus: s.lens.focus, ..view.lens };
            *s = ShotDef { id: s.id.clone(), label: s.label.clone(), aim: s.aim.clone(), focus_on: s.focus_on.clone(), phones: s.phones.clone(), lens, ..view };
            Ok(())
        })
    }

    /// what shot `i` aims at by address (none: its own `target`), checked by the core
    pub fn set_aim(&mut self, i: usize, aim: Option<&str>) -> Result<(), String> {
        self.camera_change(|c| {
            let s = c.shots.get_mut(i).ok_or("no such shot")?;
            s.aim = aim.map(str::trim).filter(|a| !a.is_empty()).map(String::from);
            Ok(())
        })
    }

    /// what shot `i` keeps in focus by address (none: its lens's focus distance), checked by the core
    pub fn set_focus_on(&mut self, i: usize, on: Option<&str>) -> Result<(), String> {
        self.camera_change(|c| {
            let s = c.shots.get_mut(i).ok_or("no such shot")?;
            s.focus_on = on.map(str::trim).filter(|a| !a.is_empty()).map(String::from);
            Ok(())
        })
    }

    /// phones may take shot `i`, or the primary camera (none), or no longer: opened to anyone with the code, or closed
    pub fn set_phones(&mut self, shot: Option<usize>, open: bool) -> Result<(), String> {
        self.camera_change(|c| {
            *phones_of(c, shot)? = open.then(Phones::default);
            Ok(())
        })
    }

    /// one fact of who may take shot `i`'s camera (none: the primary camera) on or off – the one condition control, any one
    /// fact; only the guest left is no filter, anyone with the code
    pub fn phones_fact(&mut self, shot: Option<usize>, fact: &Fact) -> Result<(), String> {
        self.camera_change(|c| {
            let p = phones_of(c, shot)?.as_mut().ok_or("no phone takes this camera")?;
            let mut a = p.admits.clone().unwrap_or_else(open_admits);
            fact.flip_admits(&mut a);
            p.admits = (a != open_admits()).then_some(a);
            Ok(())
        })
    }

    /// how long a phone holding shot `i`'s camera (none: the primary camera's) may go without a pose before it returns
    /// (ms); none = only Leave, a take-over or the conductor frees it
    pub fn set_release(&mut self, shot: Option<usize>, ms: Option<u64>) -> Result<(), String> {
        self.camera_change(|c| {
            phones_of(c, shot)?.as_mut().ok_or("no phone takes this camera")?.release_ms = ms;
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
            t.points.insert(i + 1, TrackPoint::at([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]));
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

    /// when the fly-through reaches point `i` (none: by the track's length), checked by the core
    pub fn set_point_time(&mut self, i: usize, time: Option<Length>) -> Result<(), String> {
        self.camera_change(|c| {
            c.track.as_mut().and_then(|t| t.points.get_mut(i)).ok_or("no such point")?.time = time;
            Ok(())
        })
    }

    /// the ease of the way to point `i` (none: an even speed)
    pub fn set_point_ease(&mut self, i: usize, ease: Option<Ease>) -> Result<(), String> {
        self.camera_change(|c| {
            c.track.as_mut().and_then(|t| t.points.get_mut(i)).ok_or("no such point")?.ease = ease;
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
            MoveKind::Rack => "rack",
            MoveKind::Fly => "fly",
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

    /// A phone's take kept as a move (LIVE-CAMERAS.md §7): `take<n>`, ending on its own shot `take<n>_end` – `end`, where
    /// the take stopped (its id is made here). Its place among the moves.
    pub fn add_take(&mut self, take: Take, end: ShotDef) -> Result<usize, String> {
        let mut at = 0;
        let length = take.length().ok_or("a take has two keys at least")?;
        self.camera_change(|c| {
            let id = (1..).map(|i| format!("take{i}")).find(|id| [id.clone(), format!("{id}_end")].iter().all(|x| c.shot(x).is_none() && c.find_move(x).is_none())).unwrap_or_default();
            let to = format!("{id}_end");
            c.shots.push(ShotDef { id: to.clone(), ..end });
            let spec = MoveSpec { length, take: Some(take), ..MoveSpec::of(MoveKind::Take) };
            c.moves.push(MoveDef { id, to, spec, then: None });
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

/// who may take shot `i`'s camera, or the primary camera's (none)
fn phones_of(c: &mut SceneCamera, shot: Option<usize>) -> Result<&mut Option<Phones>, String> {
    match shot {
        Some(i) => Ok(&mut c.shots.get_mut(i).ok_or("no such shot")?.phones),
        None => Ok(&mut c.phones),
    }
}

#[cfg(test)]
mod tests {
    use crate::scene_camera::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_edit::SceneEdit;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn view(yaw: f32) -> ShotDef {
        ShotDef { id: String::new(), label: Default::default(), target: [0.0, 1.0, 0.0], yaw, pitch: 10.0, distance: 3.0, lens: Lens::default(), aim: None, focus_on: None, phones: None }
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
        e.set_lens(Some(0), Lens { focus: Some(5.0), ..Lens::default() }).unwrap();
        e.set_focus_on(0, Some("character:catpope")).unwrap();
        e.set_shot(0, view(30.0)).unwrap();
        assert_eq!((e.camera().shots[0].lens.focus, e.camera().shots[0].focus_on.as_deref()), (Some(5.0), Some("character:catpope")), "set from the view: the focus stays");
        assert!(e.set_lens(Some(0), Lens { focus: Some(-1.0), ..Lens::default() }).unwrap_err().contains("the focus is"), "the core's words");
        assert!(e.set_focus_on(0, Some("ghost")).unwrap_err().contains("no holder"));
        let rack = e.add_move(MoveSpec::of(MoveKind::Rack), "shot1").unwrap();
        assert_eq!(e.camera().moves[rack].id, "rack1");
        e.remove_move(rack).unwrap();
        e.add_track([-2.0, 1.5, 3.0], [2.0, 1.5, 3.0]).unwrap();
        assert_eq!(e.add_track_point(0), Ok(1));
        e.move_track_point(1, [0.0, 2.5, 1.0]).unwrap();
        e.set_handle(1, Some([1.0, 0.0, 0.0])).unwrap();
        assert!(e.remove_track_point(0).is_err(), "the start stays");
        e.set_point_time(1, Some(Length::Bars(2.0))).unwrap();
        e.set_point_ease(1, Some(Ease::Out)).unwrap();
        assert!(e.set_point_time(0, Some(Length::Bars(1.0))).unwrap_err().contains("the start has no time"), "the core's words");
        assert!(e.set_point_time(2, Some(Length::Bars(1.0))).unwrap_err().contains("arrives before"));
        assert_eq!(e.camera().track.unwrap().points[1].time, Some(Length::Bars(2.0)));
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

    /// the DOP panel's access row: a phone may take the primary camera or a shot, who by the one condition control
    #[test]
    fn phones_are_let_take_a_camera_and_who_is_set_by_its_facts() {
        use deck_ui::filter::Fact;
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        let (mut e, _) = SceneEdit::open(doc, None, &core_rigs, &no_sets).unwrap();
        e.add_shot(view(10.0)).unwrap();
        assert!(e.phones_fact(None, &Fact::Account(protocol::Account::Member)).is_err(), "closed: nobody to admit");
        e.set_phones(None, true).unwrap();
        e.set_phones(Some(0), true).unwrap();
        assert_eq!(e.camera().phones, Some(Phones::default()), "open to anyone with the code");
        e.phones_fact(Some(0), &Fact::Account(protocol::Account::Member)).unwrap();
        assert_eq!(e.camera().shots[0].phones.as_ref().and_then(|p| p.admits.as_ref()).and_then(|a| a.account.clone()), Some(vec![protocol::Account::Guest, protocol::Account::Member]));
        e.phones_fact(Some(0), &Fact::Account(protocol::Account::Guest)).unwrap();
        assert_eq!(e.camera().shots[0].phones.as_ref().and_then(|p| p.admits.as_ref()).and_then(|a| a.account.clone()), Some(vec![protocol::Account::Member]), "members only");
        e.set_shot(0, view(30.0)).unwrap();
        assert!(e.camera().shots[0].phones.is_some(), "set from the view: who may take it stays");
        e.set_phones(None, false).unwrap();
        assert_eq!(e.camera().phones, None);
        let played = e.resolved(&core_rigs, &no_sets).unwrap().scene;
        assert_eq!(played.camera, e.doc.camera, "the stage plays it");
    }

    /// a phone's release time in the access row; a take kept as a move ending on its own shot, refused when malformed
    #[test]
    fn a_release_time_is_set_and_a_take_is_kept_as_a_move_on_its_own_end_shot() {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        let (mut e, _) = SceneEdit::open(doc, None, &core_rigs, &no_sets).unwrap();
        assert!(e.set_release(None, Some(5000)).is_err(), "no phone takes it: nothing to release");
        e.set_phones(None, true).unwrap();
        e.set_release(None, Some(5000)).unwrap();
        assert_eq!(e.camera().phones.and_then(|p| p.release_ms), Some(5000));
        e.set_release(None, None).unwrap();
        assert_eq!(e.camera().phones, Some(Phones::default()));
        let key = |x: f32, t: f32| TakeKey { at: [x, 1.6, 4.0], yaw: 0.0, pitch: 0.0, time: Length::Beats(t) };
        let take = Take { keys: vec![key(0.0, 0.0), key(1.0, 2.0), key(2.0, 4.0)] };
        e.add_shot(view(0.0)).unwrap();
        let i = e.add_take(take.clone(), view(0.0)).unwrap();
        let cam = e.camera();
        let m = &cam.moves[i];
        assert_eq!((m.id.as_str(), m.to.as_str(), m.spec.kind, m.spec.length), ("take1", "take1_end", MoveKind::Take, Length::Beats(4.0)));
        assert!(cam.shot("take1_end").is_some(), "it ends on its own shot");
        assert_eq!(e.add_take(take.clone(), view(0.0)).map(|i| e.camera().moves[i].id.clone()), Ok("take2".into()));
        let bad = Take { keys: vec![key(0.0, 0.0), key(1.0, 0.0)] };
        assert!(e.add_take(bad, view(0.0)).unwrap_err().contains("follow each other in time"));
        let played = e.resolved(&core_rigs, &no_sets).unwrap().scene;
        assert_eq!(played.camera, e.doc.camera, "the stage plays it");
    }
}
