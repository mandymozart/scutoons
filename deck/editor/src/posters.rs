//! The live frame on a video node's card in Build: each `video` node's file played by the deck's own playback
//! (deck-space `video`, deck-media) on the editor's clock, its frame on the GPU (planes → RGB like the decks), drawn under
//! the card's lines (ui.rs `card`). A file that does not open says why on its card.
use deck_space::video::{upload, Converter, Movie, Texture};
use std::collections::HashMap;
use std::time::Instant;

struct Card {
    src: String,
    movie: Result<Movie, String>,
    tex: Option<Texture>,
    since: Instant,
}

#[derive(Default)]
pub struct Posters {
    conv: Option<Converter>,
    /// by the node's id
    cards: HashMap<String, Card>,
}

/// what a card shows this frame
pub enum Frame<'a> {
    /// its frame's texture; true = a new texture (an egui id over the old one is stale)
    Live(&'a wgpu::TextureView, bool),
    /// none (yet): why
    Not(&'a str),
}

impl Posters {
    /// Each frame: the scene's `video` nodes (id, file, the use's trim) – a card opens its file once and keeps it while
    /// the file stays; its frame for the time since it opened, inside the trim, goes into its texture. The cards of nodes
    /// gone are dropped.
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, nodes: &[(String, String, deck_media::Trim)]) -> Vec<(String, Frame<'_>)> {
        if self.conv.is_none() {
            deck_media_hw::install();
        }
        self.cards.retain(|id, c| nodes.iter().any(|(i, s, _)| i == id && *s == c.src));
        let mut fresh = Vec::new();
        for (id, src, trim) in nodes {
            let card = self.cards.entry(id.clone()).or_insert_with(|| Card {
                src: src.clone(),
                movie: std::fs::read(src).map_err(|e| format!("{src}: {e}")).and_then(|b| Movie::spawn(b, id)),
                tex: None,
                since: Instant::now(),
            });
            let Ok(m) = card.movie.as_mut() else { continue };
            m.set_trim(*trim);
            if let Some(p) = m.next_picture(card.since.elapsed().as_secs_f64()) {
                let conv = self.conv.get_or_insert_with(|| Converter::new(device));
                if upload(device, queue, conv, &p, false, None, &mut card.tex) {
                    fresh.push(id.clone());
                }
            }
        }
        nodes
            .iter()
            .filter_map(|(id, _, _)| {
                let c = self.cards.get(id)?;
                let f = match (&c.movie, &c.tex) {
                    (Err(e), _) => Frame::Not(e),
                    (Ok(_), Some(t)) => Frame::Live(&t.view, fresh.contains(id)),
                    (Ok(_), None) => Frame::Not("decoding the first frame"),
                };
                Some((id.clone(), f))
            })
            .collect()
    }
}
