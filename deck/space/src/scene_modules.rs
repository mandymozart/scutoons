//! The modules on a scene's slots in an editor (#590, research/MODULE-EXTENSIONS.md §3): what slots the camera and a member
//! have, what each holds, a module from the library attached to one or taken off. The fit is the core's
//! (deck-catalog `slot::fits`); every change goes through `SceneEdit::change` and its checks.
use crate::scene_edit::SceneEdit;
use deck_catalog::slot::{self, Attach, Entity};
use deck_catalog::Descriptor;

/// What carries the slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    Camera,
    /// a holder, by its place in the document
    Holder(usize),
}

/// One slot as an inspector lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotRow {
    /// the slot's domain word (`motion`)
    pub slot: &'static str,
    /// `camera.motion`
    pub name: String,
    /// the deck runs a module here
    pub built: bool,
    /// what it may write, in words
    pub writes: &'static [&'static str],
    /// the module attached (its id)
    pub module: Option<String>,
}

impl SceneEdit {
    fn entity(&self, on: On) -> Option<Entity> {
        match on {
            On::Camera => Some(Entity::Camera),
            On::Holder(h) => self.doc.holders.get(h).map(|h| h.entity()),
        }
    }

    fn attached(&self, on: On) -> &[Attach] {
        match on {
            On::Camera => self.doc.camera.as_ref().map_or(&[], |c| c.modules.as_slice()),
            On::Holder(h) => self.doc.holders.get(h).map_or(&[], |h| h.modules.as_slice()),
        }
    }

    /// the slots of what `on` names, each with what it holds
    pub fn slot_rows(&self, on: On) -> Vec<SlotRow> {
        let Some(e) = self.entity(on) else { return Vec::new() };
        let held = self.attached(on);
        slot::of(e)
            .map(|s| SlotRow {
                slot: s.domain.word(),
                name: s.name(),
                built: s.built,
                writes: s.writes,
                module: held.iter().find(|a| a.slot == s.domain.word()).map(|a| a.module.clone()),
            })
            .collect()
    }

    fn modules_change(&mut self, on: On, f: impl FnOnce(&mut Vec<Attach>)) -> Result<(), String> {
        self.change(|d| {
            match on {
                On::Camera => {
                    let mut c = d.camera.take().unwrap_or_default();
                    f(&mut c.modules);
                    d.camera = (c != crate::scene_camera::SceneCamera::default()).then_some(c);
                }
                On::Holder(h) => f(&mut d.holders.get_mut(h).ok_or("no such member")?.modules),
            }
            Ok(())
        })
    }

    /// Module `d` (the catalog's `module:<id>`) on the slot it is made for, in place of what that slot held. Err: why it
    /// does not fit, in the core's words.
    pub fn attach(&mut self, on: On, id: &str, d: &Descriptor) -> Result<&'static str, String> {
        let e = self.entity(on).ok_or("no such member")?;
        let name = d.slot.as_deref().ok_or_else(|| format!("'{id}' is no slot module (it plays on a rig's track)"))?;
        let s = slot::find(name).ok_or_else(|| format!("'{id}' names the slot '{name}', which no entity has"))?;
        if s.entity != e {
            return Err(format!("'{id}' is made for {name}: it goes on {} {}, not here", if s.entity.word().starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }, s.entity.word()));
        }
        let a = Attach::new(s.domain.word(), id);
        slot::fits(e, &a.slot, d, &a.params)?;
        let word = s.domain.word();
        self.modules_change(on, |list| {
            list.retain(|x| x.slot != word);
            list.push(a);
        })?;
        Ok(word)
    }

    /// the slot `slot` of `on` emptied
    pub fn detach(&mut self, on: On, slot: &str) -> Result<(), String> {
        self.modules_change(on, |list| list.retain(|x| x.slot != slot))
    }

    /// what the stage runs: the camera's modules and each holder's, by holder id
    #[cfg(feature = "being")]
    pub fn motions(&self) -> Vec<(crate::space::motion::Target, Attach)> {
        use crate::space::motion::Target;
        let camera = self.attached(On::Camera).iter().map(|a| (Target::Camera, a.clone()));
        let members = self.doc.holders.iter().flat_map(|h| h.modules.iter().map(|a| (Target::Holder(h.id.clone()), a.clone())));
        camera.chain(members).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_edit::SceneEdit;
    use crate::scene_resolve::{core_rigs, no_sets};

    /// a character at holder 0, an item at holder 1
    fn edit() -> SceneEdit {
        let mut doc = SceneDocument::new("duo", "Duo", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into(), "item:incense".into()]);
        let e = SceneEdit::open(doc, None, &core_rigs, &no_sets).expect("it plays").0;
        assert_eq!(e.doc.holders[1].filler.as_deref(), Some("item:incense"));
        e
    }

    fn module(name: &str, slot: Option<&str>) -> Descriptor {
        let mut d: Descriptor = serde_json::from_str(&format!(r#"{{"name":"{name}","abi":2,"domain":"control"}}"#)).unwrap();
        d.slot = slot.map(|s| s.to_string().into());
        d
    }

    #[test]
    fn a_module_lands_on_the_slot_it_is_made_for_and_comes_off_again() {
        let mut e = edit();
        let rows = e.slot_rows(On::Camera);
        assert_eq!(rows.iter().map(|r| (r.name.as_str(), r.built)).collect::<Vec<_>>(), [("camera.motion", true), ("camera.control", false)]);
        assert_eq!(e.attach(On::Camera, "dolly", &module("dolly", Some("camera.motion"))), Ok("motion"));
        assert_eq!(e.slot_rows(On::Camera)[0].module.as_deref(), Some("dolly"));
        assert_eq!(e.attach(On::Holder(1), "bob", &module("bob", Some("item.motion"))), Ok("motion"));
        assert_eq!(e.doc.holders[1].modules, [Attach::new("motion", "bob")]);
        assert_eq!(e.motions().len(), 2);

        assert!(e.attach(On::Holder(1), "dolly", &module("dolly", Some("camera.motion"))).unwrap_err().contains("goes on a camera"));
        assert!(e.attach(On::Camera, "walk", &module("walk", None)).unwrap_err().contains("no slot module"));
        assert!(e.attach(On::Holder(0), "bob", &module("bob", Some("item.motion"))).unwrap_err().contains("goes on an item"), "a character is no item");
        assert_eq!(e.attach(On::Camera, "shake", &module("shake", Some("camera.motion"))), Ok("motion"));
        assert_eq!(e.slot_rows(On::Camera)[0].module.as_deref(), Some("shake"), "one module a slot: the new one in its place");

        e.detach(On::Camera, "motion").unwrap();
        assert_eq!(e.doc.camera, None, "an empty camera is no camera");
        let text = serde_json::to_string(&e.doc).unwrap();
        assert!(text.contains(r#""module":[{"slot":"motion","module":"bob"}]"#), "{text}");
        assert_eq!(SceneDocument::parse_json(&text).unwrap(), e.doc);
    }

    #[test]
    fn a_slot_not_built_is_refused_by_the_documents_check() {
        let mut e = edit();
        let r = e.change(|d| {
            d.holders[0].modules.push(Attach::new("motion", "walker"));
            Ok(())
        });
        assert!(r.unwrap_err().contains("character.motion is not built yet"));
    }
}
