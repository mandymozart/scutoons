//! The `chapter` node's loader (S3, CYBERDECKS.md §8 "Library"): the chapter from the library API, then its slices one
//! after the other in reading order (looping) – each image from the CDN cache, decoded (webp / png / jpeg), its dialog
//! set into it (text.rs) and its caption (the big text the overlay node shows) rendered – handed to the renderer
//! through a channel of one: the next slice is always ready, at most two are in memory. A plain thread, off the frame
//! and audio threads; the HUD and the splash read `status` / `ready`.
use crate::graph::Node;
use crate::library::Library;
use crate::text::{self, Dialog};
use anyhow::{bail, Context, Result};
use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// the longest side a slice texture gets (a taller slice is scaled down)
const MAX_SIDE: u32 = 4096;
/// the caption texture's width
const CAPTION_W: u32 = 1536;

pub struct Prepared {
    pub index: usize,
    pub count: usize,
    pub slice: RgbaImage,
    pub caption: Option<RgbaImage>,
}

pub struct ChapterFeed {
    pub node: usize,
    pub rx: Receiver<Prepared>,
    pub status: Arc<Mutex<String>>,
    /// the first slice is decoded (the splash may go)
    pub ready: Arc<AtomicBool>,
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
        if best.as_ref().map_or(true, |(r, len, _)| (rank, n) < (*r, *len)) {
            best = Some((rank, n, t));
        }
    }
    best.map(|(_, _, t)| t)
}

pub fn spawn(lib: Library, node: &Node, index: usize) -> ChapterFeed {
    let toon = node.str("toon").unwrap_or("catpope").to_string();
    let n = node.f32("chapter", 1.0) as u32;
    let quality = node.str("quality").unwrap_or("medium").to_string();
    let bake = node.bool("dialog", true);
    let start = node.f32("start", 0.0).max(0.0) as usize;
    let (tx, rx) = sync_channel(1);
    let status = Arc::new(Mutex::new("waiting".to_string()));
    let ready = Arc::new(AtomicBool::new(false));
    let (st, rd) = (status.clone(), ready.clone());
    let label = format!("{toon} {n}");
    let set = move |s: String| {
        tracing::info!("slices {label}: {s}");
        *st.lock().unwrap() = s;
    };
    std::thread::Builder::new()
        .name(format!("chapter-{}", node.id))
        .spawn(move || {
            // the chapter: retried while there is neither a network nor a cached copy (a venue's Wi-Fi comes late)
            let ch = loop {
                set("loading the chapter".into());
                match lib.chapter(&toon, n) {
                    Ok((ch, _)) => break ch,
                    Err(e) => {
                        set(format!("no chapter yet ({e:#}) – again in 10 s"));
                        std::thread::sleep(Duration::from_secs(10));
                    }
                }
            };
            let slices: Vec<(String, Vec<Dialog>)> = ch
                .slices()
                .filter_map(|s| Some((s.image_path(&quality)?.to_string(), s.dialog.clone())))
                .collect();
            if slices.is_empty() {
                set(format!("'{}' has no slices", ch.title));
                return;
            }
            let count = slices.len();
            let mut failed = 0;
            for i in (start..).map(|k| k % count) {
                let (path, dialog) = &slices[i];
                match prepare(&lib, &toon, path, dialog, bake) {
                    Ok((slice, caption)) => {
                        failed = 0;
                        if !rd.swap(true, Ordering::Relaxed) {
                            set(format!("'{}': {count} slices", ch.title));
                        }
                        if tx.send(Prepared { index: i, count, slice, caption }).is_err() {
                            return; // the renderer is gone
                        }
                    }
                    Err(e) => {
                        tracing::warn!("slice {} ({path}): {e:#}", i + 1);
                        failed += 1;
                        if failed >= count {
                            set(format!("no slice loads ({e:#}) – again in 10 s"));
                            std::thread::sleep(Duration::from_secs(10));
                            failed = 0;
                        }
                    }
                }
            }
        })
        .expect("spawning a chapter loader");
    ChapterFeed { node: index, rx, status, ready }
}

fn prepare(lib: &Library, toon: &str, path: &str, dialog: &[Dialog], bake: bool) -> Result<(RgbaImage, Option<RgbaImage>)> {
    let file = lib.file(toon, path)?;
    let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
    let mut img = image::load_from_memory(&bytes).with_context(|| format!("decoding {path}"))?.to_rgba8();
    if img.width() == 0 || img.height() == 0 {
        bail!("{path}: an empty image");
    }
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
    Ok((img, caption))
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
