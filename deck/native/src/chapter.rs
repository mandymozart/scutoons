//! The `chapter` node's slices on their way to the renderer (S3, CYBERDECKS.md §8 "Library"): what a loader hands
//! over (`Prepared`, through `ChapterFeed`). The loader itself is the `chapter` module's (#185, load.rs): the chapter
//! from the library API, then its slices one after the other in reading order (looping) – each image from the CDN
//! cache, decoded (webp / png / jpeg), its dialog set into it (text.rs) and its caption (the big text the overlay node
//! shows) rendered – through a channel of one: the next slice is always ready, at most two are in memory. A plain
//! thread, off the frame and audio threads; the HUD and the splash read `status` / `ready`.
#[cfg(feature = "chapter")]
mod load;
#[cfg(feature = "chapter")]
pub use load::spawn;

pub use deck_space::chapter::pick_caption;
pub use deck_space::stage::{ChapterFeed, Prepared};
