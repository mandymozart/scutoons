//! The `chapter` node's slices on their way to the renderer (S3, CYBERDECKS.md §8 "Library"): what a loader hands
//! over (`Prepared`, through `ChapterFeed`). The loader itself is the `chapter` module's (#185, load.rs): the chapter
//! from the library API, then its slices one after the other in reading order (looping) – each image from the CDN
//! cache, decoded (webp / png / jpeg), its dialog set into it (text.rs) and its caption (the big text the overlay node
//! shows) rendered – through a channel of one: the next slice is always ready, at most two are in memory. A plain
//! thread, off the frame and audio threads; the HUD and the splash read `status` / `ready`.
use image::RgbaImage;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

#[cfg(feature = "chapter")]
mod load;
#[cfg(feature = "chapter")]
pub use load::{pick_caption, spawn};

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
