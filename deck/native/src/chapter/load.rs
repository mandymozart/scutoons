//! The chapter loader (the `chapter` module, #185): see chapter.rs.
use super::{ChapterFeed, Prepared};
use crate::graph::Node;
use crate::library::Library;
use deck_space::chapter::{finish_slice, Dialog, Source};
use anyhow::{bail, Context, Result};
use deck_catalog::rendition::{Media, Quality};
use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn spawn(lib: Library, node: &Node, index: usize) -> ChapterFeed {
    let Source { toon, chapter: n, quality, bake, start } = Source::of(node);
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
            let q = Quality::or_medium(&quality);
            let slices: Vec<(Media, Vec<Dialog>)> = ch.slices().map(|s| (s.image_media(), s.dialog.clone())).collect();
            if slices.is_empty() {
                set(format!("'{}' has no slices", ch.title));
                return;
            }
            let count = slices.len();
            let mut failed = 0;
            for i in (start..).map(|k| k % count) {
                let (media, dialog) = &slices[i];
                let path = media.pick(q);
                match prepare(&lib, &toon, media, q, dialog, bake) {
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

/// the slice's image for `q` through the general-purpose loader (offline: the best rendition the cache holds), decoded
fn prepare(lib: &Library, toon: &str, media: &Media, q: Quality, dialog: &[Dialog], bake: bool) -> Result<(RgbaImage, Option<RgbaImage>)> {
    let got = lib.load(Some(toon), media, q)?;
    let path = got.path;
    let img = image::load_from_memory(&got.bytes).with_context(|| format!("decoding {path}"))?.to_rgba8();
    if img.width() == 0 || img.height() == 0 {
        bail!("{path}: an empty image");
    }
    Ok(finish_slice(img, dialog, bake))
}
