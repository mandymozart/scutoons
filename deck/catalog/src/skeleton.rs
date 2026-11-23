//! Skeletons (#817): which joints a model or an animation has, which of them an animation turns, and whether an
//! animation fits a character's model. An asset version's `skeleton` (the API stores it as the core wrote it, `/validate`
//! kind `skeleton`) is `Skeleton`. `fit` is the one compatibility rule – the creator's picker (through the wasm), the
//! API's check when a character is saved (`/validate` kind `animation-fit`) and the stage at load (deck-space
//! anatomy.rs `adopt`) all ask it: an animation fits when the model has every joint it turns, by name.
//!
//! A family label (`mixamo` …) is a hint for people, never what fits is decided by: a skeleton is recognised by the
//! first row of `FAMILIES` whose anchors it has. A new family is one more row.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A skeleton family the library names: its joints are `<prefix>[digits]<: or _><local name>` and it has every anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub name: &'static str,
    pub prefix: &'static str,
    /// local names (without the prefix) a skeleton of this family always has
    pub anchors: &'static [&'static str],
}

/// The families recognised, first match wins.
pub const FAMILIES: &[Family] = &[
    // Adobe Mixamo's standard rig: 65 joints with the fingers, 52 / 41 / 25 without (the anchors are in all of them)
    Family {
        name: "mixamo",
        prefix: "mixamorig",
        anchors: &[
            "Hips",
            "Spine",
            "Neck",
            "Head",
            "LeftArm",
            "RightArm",
            "LeftUpLeg",
            "RightUpLeg",
        ],
    },
];

/// at most this many joints in a stored skeleton
pub const MAX_JOINTS: usize = 1024;

impl Family {
    /// the local name of `joint` in this family (`mixamorig:Hips`, `mixamorig1_Hips` → `Hips`), None when it is not one
    pub fn local<'a>(&self, joint: &'a str) -> Option<&'a str> {
        let rest = joint
            .strip_prefix(self.prefix)?
            .trim_start_matches(|c: char| c.is_ascii_digit());
        rest.strip_prefix(':')
            .or_else(|| rest.strip_prefix('_'))
            .filter(|l| !l.is_empty())
    }

    fn has(&self, joints: &[String]) -> bool {
        let locals: BTreeSet<&str> = joints.iter().filter_map(|j| self.local(j)).collect();
        self.anchors.iter().all(|a| locals.contains(a))
    }
}

/// The family a skeleton with these joints belongs to, None when no row of `FAMILIES` recognises it.
pub fn family_of(joints: &[String]) -> Option<&'static str> {
    FAMILIES.iter().find(|f| f.has(joints)).map(|f| f.name)
}

/// The joints of a model or an animation file, as the core read them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skeleton {
    /// the family `FAMILIES` recognises in `joints` (none: no family known)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// every joint of the file's skins, by name, in the file's order
    #[serde(default)]
    pub joints: Vec<String>,
    /// the joints the file's first clip turns (a rotation channel): what an animation brings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animated: Vec<String>,
    /// the file's clips by name, in its order (a character's idle or a holder's `clip` names one of its model's)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<String>,
}

impl Skeleton {
    /// joints and the turned joints as read; the family recognised
    pub fn new(joints: Vec<String>, animated: Vec<String>, clips: Vec<String>) -> Skeleton {
        Skeleton {
            family: family_of(&joints).map(str::to_string),
            joints,
            animated,
            clips,
        }
    }

    /// whether a clip of this name is in the file – the one rule for a named clip (a character's idle, a holder's
    /// `clip`): None = it is, else the sentence
    pub fn clip_problem(&self, name: &str) -> Option<String> {
        if self.clips.iter().any(|c| c == name) {
            return None;
        }
        Some(if self.clips.is_empty() {
            format!("no clip '{name}': the model has no clips")
        } else {
            format!(
                "no clip '{name}': the model has {}",
                list(self.clips.iter().map(String::as_str))
            )
        })
    }

    /// What is wrong with a stored skeleton, one sentence each (none: it can be stored).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.joints.len() > MAX_JOINTS {
            out.push(format!(
                "{} joints – at most {MAX_JOINTS}",
                self.joints.len()
            ));
        }
        if self.joints.iter().any(String::is_empty) {
            out.push("a joint has no name: nothing can find it by name".to_string());
        }
        let mut seen = BTreeSet::new();
        let doubles: BTreeSet<&str> = self
            .joints
            .iter()
            .filter(|j| !j.is_empty() && !seen.insert(j.as_str()))
            .map(String::as_str)
            .collect();
        if !doubles.is_empty() {
            out.push(format!(
                "joints named twice: {}",
                list(doubles.iter().copied())
            ));
        }
        let strays: Vec<&str> = self
            .animated
            .iter()
            .filter(|a| !seen.contains(a.as_str()))
            .map(String::as_str)
            .collect();
        if !strays.is_empty() {
            out.push(format!(
                "turns joints the skeleton does not have: {}",
                list(strays.iter().copied())
            ));
        }
        let family = family_of(&self.joints);
        if self.family.as_deref() != family {
            out.push(match (&self.family, family) {
                (Some(said), Some(is)) => format!("the family is '{is}', not '{said}'"),
                (Some(said), None) => format!("the joints are not the family '{said}'"),
                (None, Some(is)) => {
                    format!("the joints are the family '{is}' – the skeleton should say so")
                }
                (None, None) => unreachable!(),
            });
        }
        out
    }
}

/// How well an animation fits a model: of the `of` joints the animation turns the model has `matched`; `missing` are
/// the others, by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fit {
    pub matched: usize,
    pub of: usize,
    pub missing: Vec<String>,
}

impl Fit {
    /// the model has every joint the animation turns (and it turns at least one)
    pub fn fits(&self) -> bool {
        self.of > 0 && self.missing.is_empty()
    }

    /// what does not fit, one sentence (None: it fits)
    pub fn problem(&self) -> Option<String> {
        if self.of == 0 {
            return Some("the animation turns no joint".to_string());
        }
        (!self.missing.is_empty()).then(|| {
            format!(
                "the model lacks {} of the {} joints the animation turns: {}",
                self.missing.len(),
                self.of,
                list(self.missing.iter().map(String::as_str))
            )
        })
    }
}

/// The one compatibility rule: which of the joints an animation turns (`animated`) the model's skeleton (`joints`) has,
/// matched by name.
pub fn fit(animated: &[String], joints: &[String]) -> Fit {
    let have: BTreeSet<&str> = joints.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    let turned: Vec<&String> = animated
        .iter()
        .filter(|a| seen.insert(a.as_str()))
        .collect();
    let missing: Vec<String> = turned
        .iter()
        .filter(|a| !have.contains(a.as_str()))
        .map(|a| a.to_string())
        .collect();
    Fit {
        matched: turned.len() - missing.len(),
        of: turned.len(),
        missing,
    }
}

/// The `/validate` kind `animation-fit` document: what a character's idle names – a library animation's skeleton or a
/// clip of its own model by name – and the model's skeleton it plays on.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<Skeleton>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    pub model: Skeleton,
}

impl FitCheck {
    pub fn problems(&self) -> Vec<String> {
        match (&self.animation, &self.clip) {
            (Some(a), None) => fit(&a.animated, &self.model.joints)
                .problem()
                .into_iter()
                .collect(),
            (None, Some(name)) => self.model.clip_problem(name).into_iter().collect(),
            (Some(_), Some(_)) => vec![
                "an idle names a library animation or a clip of the model, not both".to_string(),
            ],
            (None, None) => {
                vec!["an idle names a library animation or a clip of the model".to_string()]
            }
        }
    }
}

/// at most twelve names, then how many more
fn list<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    let shown = names
        .iter()
        .take(12)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > 12 {
        format!("{shown} … (+{} more)", names.len() - 12)
    } else {
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const MIXAMO: &[&str] = &[
        "mixamorig:Hips",
        "mixamorig:Spine",
        "mixamorig:Neck",
        "mixamorig:Head",
        "mixamorig:LeftArm",
        "mixamorig:RightArm",
        "mixamorig:LeftUpLeg",
        "mixamorig:RightUpLeg",
    ];

    #[test]
    fn a_family_is_recognised_by_its_anchors_with_any_prefix_variant() {
        assert_eq!(family_of(&names(MIXAMO)), Some("mixamo"));
        let numbered: Vec<String> = MIXAMO
            .iter()
            .map(|j| j.replace("mixamorig:", "mixamorig1_"))
            .collect();
        assert_eq!(
            family_of(&numbered),
            Some("mixamo"),
            "mixamorig1_ is the same family"
        );
        assert_eq!(
            family_of(&names(&MIXAMO[1..])),
            None,
            "without the hips: no family"
        );
        assert_eq!(family_of(&names(&["upper", "lower"])), None);
        assert_eq!(
            FAMILIES[0].local("mixamorig:"),
            None,
            "a bare prefix is no joint"
        );
    }

    #[test]
    fn fit_counts_the_turned_joints_the_model_has_by_name() {
        let f = fit(&names(&["a", "b", "c", "b"]), &names(&["a", "c", "d"]));
        assert_eq!(
            f,
            Fit {
                matched: 2,
                of: 3,
                missing: names(&["b"])
            }
        );
        assert!(!f.fits());
        assert_eq!(
            f.problem().unwrap(),
            "the model lacks 1 of the 3 joints the animation turns: b"
        );
        assert!(
            fit(&names(&["a"]), &names(&["a", "z"])).fits(),
            "extra joints on the model are fine"
        );
        assert!(
            !fit(&[], &names(&["a"])).fits(),
            "an animation that turns nothing fits nothing"
        );
    }

    #[test]
    fn a_stored_skeleton_must_say_its_family_and_turn_only_its_joints() {
        let s = Skeleton::new(names(MIXAMO), names(&["mixamorig:Hips"]), names(&["idle"]));
        assert_eq!(
            (s.family.as_deref(), s.problems()),
            (Some("mixamo"), Vec::<String>::new())
        );
        let wrong = Skeleton {
            family: Some("other".into()),
            joints: names(&["a", "a", ""]),
            animated: names(&["q"]),
            clips: vec![],
        };
        let p = wrong.problems().join(" | ");
        for want in [
            "a joint has no name",
            "joints named twice: a",
            "turns joints the skeleton does not have: q",
            "not the family 'other'",
        ] {
            assert!(p.contains(want), "{p}");
        }
        assert!(Skeleton { family: None, ..s }.problems()[0].contains("should say so"));
    }

    #[test]
    fn a_named_clip_must_be_in_the_model() {
        let model = Skeleton::new(names(&["a"]), vec![], names(&["walk", "stand"]));
        assert_eq!(model.clip_problem("walk"), None);
        assert_eq!(
            model.clip_problem("run").unwrap(),
            "no clip 'run': the model has walk, stand"
        );
        let check = |clip: Option<&str>, animation: Option<Skeleton>| {
            FitCheck {
                animation,
                clip: clip.map(String::from),
                model: model.clone(),
            }
            .problems()
        };
        assert!(check(Some("walk"), None).is_empty());
        assert_eq!(check(None, None).len(), 1);
        assert!(check(Some("walk"), Some(Skeleton::default()))[0].contains("not both"));
        assert!(
            check(
                None,
                Some(Skeleton::new(names(&["a"]), names(&["a"]), vec![]))
            )
            .is_empty(),
            "an animation that fits"
        );
    }

    #[test]
    fn the_list_of_missing_joints_is_cut_after_twelve() {
        let many: Vec<String> = (0..20).map(|i| format!("j{i}")).collect();
        assert!(fit(&many, &[])
            .problem()
            .unwrap()
            .ends_with("j11 … (+8 more)"));
    }
}
