//! The stage's background: one setting on a module, a rig, a scene and a show. A colour fills the stage behind the 3D;
//! an image or a video is laid over the stage, covering it without distortion, its transparency kept. Unset everywhere:
//! black. Its picture's fit is deck-core's `fit::cover`.
use serde::{Deserialize, Serialize};

/// One of a colour, an image or a video.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Background {
    /// `#rgb` or `#rrggbb`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// a picture's media path, laid over the stage
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// a video's media path, laid over the stage
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<String>,
}

impl Background {
    pub fn colour(hex: &str) -> Background {
        Background { colour: Some(hex.into()), ..Background::default() }
    }

    pub fn image(path: &str) -> Background {
        Background { image: Some(path.into()), ..Background::default() }
    }

    /// What a person typed: nothing = none, `#…` = a colour, a path = an image, or a video by its extension.
    pub fn typed(text: &str) -> Result<Option<Background>, String> {
        let t = text.trim();
        if t.is_empty() {
            return Ok(None);
        }
        let b = if t.starts_with('#') {
            Background::colour(t)
        } else if is_video(t) {
            Background { video: Some(t.into()), ..Background::default() }
        } else {
            Background::image(t)
        };
        match b.check() {
            Some(e) => Err(e),
            None => Ok(Some(b)),
        }
    }

    /// as a person reads it (and types it back)
    pub fn text(&self) -> &str {
        self.colour.as_deref().or(self.image.as_deref()).or(self.video.as_deref()).unwrap_or_default()
    }

    /// what is wrong with it; None = well formed
    pub fn check(&self) -> Option<String> {
        let set = [&self.colour, &self.image, &self.video].iter().filter(|v| v.is_some()).count();
        if set != 1 {
            return Some("a background is one of `colour`, `image` or `video`".into());
        }
        if let Some(c) = &self.colour {
            if rgb(c).is_none() {
                return Some(format!("'{c}' is no colour: #rgb or #rrggbb"));
            }
        }
        match self.image.as_deref().or(self.video.as_deref()) {
            Some(p) if p.trim().is_empty() => Some("the media path is empty".into()),
            _ => None,
        }
    }

    /// the colour as display values 0–1; None for an image or a video
    pub fn rgb(&self) -> Option<[f32; 3]> {
        self.colour.as_deref().and_then(rgb)
    }
}

/// the extensions a media path is a video by
const VIDEO: [&str; 4] = [".mp4", ".webm", ".mov", ".mkv"];

/// whether a media path is a video (a background's, a layer's picture)
pub fn is_video(path: &str) -> bool {
    let p = path.trim().to_ascii_lowercase();
    VIDEO.iter().any(|e| p.ends_with(e))
}

/// `#rgb` / `#rrggbb` → display values 0–1
pub fn rgb(hex: &str) -> Option<[f32; 3]> {
    let h = hex.strip_prefix('#')?;
    if !h.is_ascii() {
        return None;
    }
    let digit = |s: &str| u8::from_str_radix(s, 16).ok().map(|v| v as f32 / 255.0);
    match h.len() {
        3 => {
            let one = |i: usize| digit(&h[i..=i].repeat(2));
            Some([one(0)?, one(1)?, one(2)?])
        }
        6 => Some([digit(&h[0..2])?, digit(&h[2..4])?, digit(&h[4..6])?]),
        _ => None,
    }
}

/// The background that plays: the nearest one set from the top – a show over its scene, a scene over its rigs, a rig
/// over its modules. None = black.
pub fn resolve<'a>(show: Option<&'a Background>, scene: Option<&'a Background>, rig: Option<&'a Background>, module: Option<&'a Background>) -> Option<&'a Background> {
    show.or(scene).or(rig).or(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parent_wins_and_none_is_black() {
        let (show, scene, rig, module) = (Background::colour("#f00"), Background::image("a.png"), Background::colour("#0f0"), Background::colour("#00f"));
        assert_eq!(resolve(Some(&show), Some(&scene), Some(&rig), Some(&module)), Some(&show));
        assert_eq!(resolve(None, Some(&scene), Some(&rig), Some(&module)), Some(&scene));
        assert_eq!(resolve(None, None, Some(&rig), Some(&module)), Some(&rig));
        assert_eq!(resolve(None, None, None, Some(&module)), Some(&module));
        assert_eq!(resolve(Some(&show), None, None, Some(&module)), Some(&show), "a gap between does not matter");
        assert_eq!(resolve(None, None, None, None), None, "none set: black");
    }

    #[test]
    fn checked_and_parsed() {
        assert_eq!(Background::colour("#ff8000").rgb(), Some([1.0, 128.0 / 255.0, 0.0]));
        assert_eq!(Background::colour("#fff").rgb(), Some([1.0; 3]));
        assert_eq!(Background::colour("#000").check(), None);
        assert!(Background::colour("red").check().unwrap().contains("no colour"));
        assert!(Background::colour("#12345").check().is_some());
        assert!(Background::image(" ").check().unwrap().contains("empty"));
        assert!(Background::default().check().is_some(), "nothing set");
        let two = Background { colour: Some("#000".into()), image: Some("a.png".into()), video: None };
        assert!(two.check().is_some(), "two set");
        let b: Background = serde_json::from_str(r##"{"image":"backgrounds/sky.png"}"##).unwrap();
        assert_eq!(b, Background::image("backgrounds/sky.png"));
        assert!(serde_json::from_str::<Background>(r#"{"picture":"x"}"#).is_err(), "unknown fields are refused");
    }

    #[test]
    fn typed() {
        assert_eq!(Background::typed("  "), Ok(None));
        assert_eq!(Background::typed("#102030"), Ok(Some(Background::colour("#102030"))));
        assert_eq!(Background::typed("backgrounds/sky.png"), Ok(Some(Background::image("backgrounds/sky.png"))));
        assert_eq!(Background::typed("live/x/loop.MP4").unwrap().unwrap().video.as_deref(), Some("live/x/loop.MP4"));
        assert!(Background::typed("#zz").is_err());
        assert_eq!(Background::image("a.png").text(), "a.png");
    }
}
