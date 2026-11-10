//! A `chapter` node's slices as every deck reads them (S3, #465: out of deck-native): the part of a chapter the stage
//! needs, what the node asks for, and a decoded slice made ready to show (its dialog set in, its caption rendered). Where
//! the files come from is the host's: deck-native's library and disk cache, the page's fetch.
use crate::graph::Node;
pub use deck_outlet::text::Dialog;
use deck_outlet::text;
use image::RgbaImage;
use serde::Deserialize;
use std::collections::HashMap;

/// the longest side a slice texture gets (a taller slice is scaled down)
const MAX_SIDE: u32 = 4096;
/// the caption texture's width
const CAPTION_W: u32 = 1536;

/// The part of a chapter (shared/content/src/content.ts `Chapter`, audio.ts `AudioConfiguration`) the stage reads.
#[derive(Debug, Deserialize)]
pub struct Chapter {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub audio: Option<Audio>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub slices: Vec<Slice>,
}

/// One slice of the strip (content.ts `Slice`): the image (CDN path, per quality), its size, the dialog over it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub image_variants: HashMap<String, String>,
    #[serde(default)]
    pub dialog: Vec<Dialog>,
}

impl Slice {
    pub fn image_path(&self, quality: &str) -> Option<&str> {
        self.image_variants.get(quality).or(self.image.as_ref()).map(String::as_str)
    }
}

#[derive(Debug, Deserialize)]
pub struct Audio {
    #[serde(default)]
    pub music: Option<Music>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Music {
    #[serde(default)]
    pub default_track: Option<String>,
    #[serde(default)]
    pub tracks: Vec<MusicTrack>,
}

#[derive(Debug, Deserialize)]
pub struct MusicTrack {
    pub id: String,
    pub path: String,
    /// media quality → path (low / medium / high)
    #[serde(default)]
    pub variants: HashMap<String, String>,
}

impl Chapter {
    /// the slices with an image, in reading order
    pub fn slices(&self) -> impl Iterator<Item = &Slice> {
        self.groups.iter().flat_map(|g| &g.slices).filter(|s| s.image.is_some() || !s.image_variants.is_empty())
    }

    /// the slices' image paths at `quality` and their dialog
    pub fn strip(&self, quality: &str) -> Vec<(String, Vec<Dialog>)> {
        self.slices().filter_map(|s| Some((s.image_path(quality)?.to_string(), s.dialog.clone()))).collect()
    }

    /// the track `id` (else the chapter's default track, else the first) at `quality` (else its plain path)
    pub fn music_path(&self, id: Option<&str>, quality: &str) -> Option<(&str, &str)> {
        let m = self.audio.as_ref()?.music.as_ref()?;
        let want = id.or(m.default_track.as_deref());
        let t = want.and_then(|w| m.tracks.iter().find(|t| t.id == w)).or(m.tracks.first())?;
        Some((t.id.as_str(), t.variants.get(quality).unwrap_or(&t.path).as_str()))
    }
}

/// What a `chapter` node asks for.
pub struct Source {
    pub toon: String,
    pub chapter: u32,
    pub quality: String,
    /// set the dialog into the slice
    pub bake: bool,
    pub start: usize,
}

impl Source {
    pub fn of(node: &Node) -> Source {
        Source {
            toon: node.str("toon").unwrap_or("catpope").to_string(),
            chapter: node.f32("chapter", 1.0) as u32,
            quality: node.str("quality").unwrap_or("medium").to_string(),
            bake: node.bool("dialog", true),
            start: node.f32("start", 0.0).max(0.0) as usize,
        }
    }

    /// the chapter's path under the library API
    pub fn api_path(&self) -> String {
        format!("/library/toons/{}/chapters/{}", self.toon, self.chapter)
    }
}

/// the caption of a slice: its shortest painted lettering (a sound, a title), else its shortest line of dialog –
/// short enough to be shouted (≤ 48 characters)
pub fn pick_caption(dialog: &[Dialog]) -> Option<String> {
    let mut best: Option<(u8, usize, String)> = None;
    for d in dialog {
        let t = d.text().split_whitespace().collect::<Vec<_>>().join(" ");
        let n = t.chars().count();
        if n == 0 || n > 48 {
            continue;
        }
        let rank = match (d.font.as_deref(), d.variant.as_deref()) {
            (Some("lettering"), _) => 0,
            (_, Some("bubble")) => 1,
            _ => 2,
        };
        if best.as_ref().is_none_or(|(r, len, _)| (rank, n) < (*r, *len)) {
            best = Some((rank, n, t));
        }
    }
    best.map(|(_, _, t)| t)
}

/// A decoded slice made ready to show: its dialog set in (`bake`), scaled to fit a texture, its caption rendered.
pub fn finish_slice(mut img: RgbaImage, dialog: &[Dialog], bake: bool) -> (RgbaImage, Option<RgbaImage>) {
    if bake {
        text::bake_dialog(&mut img, dialog);
    }
    let side = img.width().max(img.height());
    if side > MAX_SIDE {
        let k = MAX_SIDE as f32 / side as f32;
        let (w, h) = (((img.width() as f32 * k) as u32).max(1), ((img.height() as f32 * k) as u32).max(1));
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    }
    let caption = pick_caption(dialog).and_then(|t| text::caption(&t, CAPTION_W, 2));
    (img, caption)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caption_prefers_short_lettering() {
        let d: Vec<Dialog> = serde_json::from_str(
            r#"[{"text":"Puh! Es war nur ein Traum.","variant":"bubble","font":"mono"},
                {"text":"An jedem Morgen ging die Sonne auf, so war es geschrieben. Es verschob sich.","variant":"bubble"},
                {"text":"ZZZZZ","variant":"bubble","font":"lettering"}]"#,
        )
        .unwrap();
        assert_eq!(pick_caption(&d).as_deref(), Some("ZZZZZ"));
        assert_eq!(pick_caption(&d[..2]).as_deref(), Some("Puh! Es war nur ein Traum."));
        assert_eq!(pick_caption(&d[1..2]), None, "too long to shout");
        let upper: Vec<Dialog> = serde_json::from_str(r#"[{"text":"Nacht\nInnen","uppercase":true}]"#).unwrap();
        assert_eq!(pick_caption(&upper).as_deref(), Some("NACHT INNEN"));
    }
}
