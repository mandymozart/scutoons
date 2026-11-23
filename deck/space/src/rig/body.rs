//! A rig's 3D body and the rigs attached to it: a body part is a rig with a body of its own, usable on a character, a
//! vehicle or anywhere a rig fits; rigs nest.
//!
//! - **Body:** the model (a `.glb` by its CDN path from the root, as an asset slot stores it) the stage draws where the
//!   rig stands – a placed rig, or a member whose rig names one; none = its being. `height` sizes it in metres (none =
//!   its kind's card height, cast.rs).
//! - **Attached:** another rig (`rig:<subject>`) carried at a spot of this rig's body (a model's spots by name,
//!   deck-catalog spot.rs) – where it stands is the spot's, so a body part rides the bone its spot is on.
use deck_catalog::spot::{self, Spot, Spots};
use serde::{Deserialize, Serialize};

/// What a rig looks like on a stage: its own model.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// the model: a `.glb` by its CDN path from the root (`characters/takeyari-drum/takeyari-drum-regular.glb`)
    pub model: String,
    /// how tall it stands, metres; none = its kind's card height
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

/// the tallest body a stage takes, metres (a tower is a set, not a rig)
pub const MAX_HEIGHT: f32 = 100.0;

impl Body {
    /// what keeps it from being drawn: a model that is no `.glb` path from the root, a height that is no length
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let m = self.model.as_str();
        let path_ok = !m.is_empty()
            && !m.starts_with('/')
            && !m.contains(['\\', '?', '#', ':'])
            && !m.chars().any(char::is_whitespace)
            && m.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
            && m.to_ascii_lowercase().ends_with(".glb");
        if !path_ok {
            out.push(format!("body: the model '{m}' is no .glb path from the CDN's root (characters/<id>/<file>.glb)"));
        }
        if let Some(h) = self.height.filter(|h| !(h.is_finite() && *h > 0.0 && *h <= MAX_HEIGHT)) {
            out.push(format!("body: the height {h} is no length (more than 0, at most {MAX_HEIGHT} metres)"));
        }
        out
    }
}

/// A rig carried at a spot of this rig's body.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attached {
    /// the rig: `rig:<subject>` (`rig:item.takeyari-exhaust`)
    pub rig: String,
    /// the spot of this rig's body it rides (`exhaust_left`: the model's `spot_exhaust_left`)
    pub spot: String,
}

impl Attached {
    /// the attached rig's subject (`item.takeyari-exhaust`)
    pub fn subject(&self) -> Option<&str> {
        self.rig.strip_prefix("rig:").filter(|s| crate::rig::address::Place::of_subject(s).is_some())
    }

    /// The spot it rides on a body whose spots are `spots` (the model's, with its hand edits); why not when the body has
    /// no spot of that name.
    pub fn find(&self, spots: &Spots) -> Result<Spot, String> {
        spots.spot(&self.spot).ok_or_else(|| format!("attach {}: the body has no spot '{}'", self.rig, self.spot))
    }
}

/// A rig's body and attachments checked (`own`: its subject): the body's model and height; each attachment a rig
/// reference, not the rig itself, at a spot name, once per spot – and only on a rig that has a body to carry it.
pub fn problems(own: &str, body: Option<&Body>, attached: &[Attached]) -> Vec<String> {
    let mut out = body.map(Body::problems).unwrap_or_default();
    if !attached.is_empty() && body.is_none() {
        out.push("attach: a rig is carried at a spot of a body – this rig names no body".to_string());
    }
    let mut spots = std::collections::BTreeSet::new();
    for a in attached {
        match a.subject() {
            None => out.push(format!("attach: '{}' is no `rig:<subject>` (rig:item.takeyari-exhaust)", a.rig)),
            Some(s) if s == own => out.push(format!("attach: {} cannot carry itself", a.rig)),
            Some(_) => {}
        }
        if let Err(e) = spot::name(&a.spot) {
            out.push(format!("attach {}: {e}", a.rig));
        } else if !spots.insert(a.spot.as_str()) {
            out.push(format!("attach {}: the spot '{}' carries a rig already", a.rig, a.spot));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_catalog::spot::{Attach, File};

    fn body(model: &str) -> Body {
        Body { model: model.into(), height: None }
    }

    #[test]
    fn a_body_is_a_glb_from_the_cdn_root() {
        assert!(body("characters/takeyari-drum/takeyari-drum-regular.glb").problems().is_empty());
        for bad in ["", "/characters/x.glb", "characters/x.png", "../x.glb", "characters//x.glb", "https://cdn/x.glb", "a b.glb", "x.glb?v=2", "a\\x.glb"] {
            assert_eq!(body(bad).problems().len(), 1, "{bad}");
        }
        for h in [0.0, -1.0, f32::NAN, 101.0] {
            assert_eq!(Body { height: Some(h), ..body("x.glb") }.problems().len(), 1, "{h}");
        }
        assert!(Body { height: Some(1.2), ..body("x.glb") }.problems().is_empty());
    }

    #[test]
    fn an_attachment_rides_a_spot_of_the_body() {
        let exhaust = Attached { rig: "rig:item.takeyari-exhaust".into(), spot: "exhaust_left".into() };
        let b = body("characters/yellowmouse/modules/bike.glb");
        assert!(problems("item.bike", Some(&b), std::slice::from_ref(&exhaust)).is_empty());
        assert_eq!(exhaust.subject(), Some("item.takeyari-exhaust"));
        // no body to carry it, itself, a reference that is none, a spot that is no name, one spot twice
        assert_eq!(problems("item.bike", None, std::slice::from_ref(&exhaust)), ["attach: a rig is carried at a spot of a body – this rig names no body"]);
        let own = Attached { rig: "rig:item.bike".into(), spot: "seat".into() };
        let bad_ref = Attached { rig: "item.exhaust".into(), spot: "tail".into() };
        let bad_spot = Attached { rig: "rig:item.light".into(), spot: "Tail Light".into() };
        let twice = Attached { rig: "rig:item.other".into(), spot: "exhaust_left".into() };
        let p = problems("item.bike", Some(&b), &[exhaust.clone(), own, bad_ref, bad_spot, twice]);
        assert_eq!(p.len(), 4, "{p:?}");
        assert!(p[0].contains("cannot carry itself") && p[1].contains("no `rig:<subject>`") && p[2].contains("no spot name") && p[3].contains("carries a rig already"), "{p:?}");
        // the spot is the body model's: found by name with what carries it (a bone) – or the body has none of that name
        let mut found = Spot::new("exhaust_left");
        found.attach = Attach::Bone("pelvis".into());
        found.at = [0.2, 0.4, -0.3];
        let spots = Spots { file: Some(File { spots: vec![found.clone()], ..File::default() }), hand: Vec::new() };
        assert_eq!(exhaust.find(&spots), Ok(found));
        let tail = Attached { spot: "tail".into(), ..exhaust };
        assert_eq!(tail.find(&spots), Err("attach rig:item.takeyari-exhaust: the body has no spot 'tail'".into()));
    }
}
