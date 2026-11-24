//! What the deck tells the member's other apps (DECK-LINK.md): its device row – the workspace in front and each one's
//! last item (deck-ui `link::deck_state`, PUT /api/me/session) whenever that changes – and its cards' stills: the front
//! workspace's picture, taken once it held still and again now and then, sent up as the member's own
//! `workspace:deck-<workspace>` (PUT /api/thumbs) when the deck leaves it (another workspace, the switcher, Home).
//! Signed out, nothing is sent; a check run sends nothing. One sender thread, in order; a failure is a log line.
use crate::library::{fitted, STILL};
use deck_ui::link;
use deck_ui::workspaces::Workspace;
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

/// frames the front workspace plays before its picture is taken
const SETTLE: u64 = 90;
/// and not more often than this while it stays in front
const EVERY: Duration = Duration::from_secs(20);

enum Job {
    Row { url: String, token: String, body: String },
    Still { url: String, token: String, webp: Vec<u8> },
}

/// the workspace in front and its item
type Front = (Option<Workspace>, Option<String>);

#[derive(Default)]
pub struct DeviceRow {
    /// the front (workspace, item) and the state last sent for it (none: not yet); the row's last items change only
    /// with the front
    sent: Option<(Front, serde_json::Value)>,
    /// the front workspace and item, since which frame, when its picture was last taken
    front: Front,
    since: u64,
    taken: Option<Instant>,
    /// each workspace's newest picture not sent yet
    stills: BTreeMap<Workspace, Vec<u8>>,
    tx: Option<Sender<Job>>,
}

/// the still's key for a deck workspace's card
pub fn still_key(w: Workspace) -> String {
    format!("workspace:deck-{}", link::id_of(w))
}

/// the device row's name, as the creator's switcher says where the deck is
fn device_name() -> String {
    let machine = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    if machine.is_empty() { "Deck".to_string() } else { format!("Deck · {machine}") }
}

/// a picture of the window (RGBA) as the card's still: the middle cut, the still's size, WebP
pub fn webp_of(px: Vec<u8>, (w, h): (u32, u32)) -> Option<Vec<u8>> {
    let img = image::RgbaImage::from_raw(w, h, px)?;
    let still = image::DynamicImage::ImageRgba8(fitted(&img)).to_rgb8();
    let mut out = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut out).encode(still.as_raw(), STILL.0, STILL.1, image::ExtendedColorType::Rgb8).ok()?;
    Some(out)
}

impl DeviceRow {
    fn send(&mut self, job: Job) {
        let tx = self.tx.get_or_insert_with(|| {
            let (tx, rx) = channel::<Job>();
            std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
                for job in rx {
                    let (what, r) = match job {
                        Job::Row { url, token, body } => ("the deck's row", agent.put(&url).set("Authorization", &format!("Bearer {token}")).set("Content-Type", "application/json").send_string(&body)),
                        Job::Still { url, token, webp } => ("a card's still", agent.put(&url).set("Authorization", &format!("Bearer {token}")).set("Content-Type", "image/webp").send_bytes(&webp)),
                    };
                    if let Err(e) = r {
                        tracing::info!("{what} is not sent: {e}");
                    }
                }
            });
            tx
        });
        let _ = tx.send(job);
    }

    /// Each frame (not in a check run): the front workspace and item, the row's last items, whether the switcher or
    /// Home covers the workspace. A changed row goes up; a workspace left sends its picture.
    pub fn follow(&mut self, api: &str, token: Option<String>, front: Front, last: &BTreeMap<Workspace, String>, covered: bool, frame: u64) {
        let shown = if covered { (None, None) } else { front.clone() };
        if shown != self.front {
            if let Some(left) = self.front.0.filter(|w| shown.0 != Some(*w)) {
                if let (Some(webp), Some(t)) = (self.stills.remove(&left), token.clone()) {
                    self.send(Job::Still { url: format!("{api}/thumbs?key={}", still_key(left)), token: t, webp });
                }
            }
            (self.front, self.since, self.taken) = (shown, frame, None);
        }
        let Some(token) = token else { return };
        if self.sent.as_ref().is_some_and(|(f, _)| *f == front) {
            return;
        }
        let state = link::deck_state(front.0, last);
        if self.sent.as_ref().is_none_or(|(_, s)| *s != state) {
            let body = serde_json::json!({ "device": crate::creator_cards::device(), "name": device_name(), "state": state }).to_string();
            self.send(Job::Row { url: format!("{api}/me/session"), token, body });
        }
        self.sent = Some((front, state));
    }

    /// the front workspace's picture is due this frame
    pub fn wants_still(&self, frame: u64) -> bool {
        self.front.0.is_some() && frame >= self.since + SETTLE && self.taken.is_none_or(|t| t.elapsed() >= EVERY)
    }

    /// the frame's pixels (RGBA, `size`) taken for the front workspace's card
    pub fn took(&mut self, px: Vec<u8>, size: (u32, u32)) {
        self.taken = Some(Instant::now());
        if let (Some(w), Some(webp)) = (self.front.0, webp_of(px, size)) {
            self.stills.insert(w, webp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cards_still_is_a_webp_of_the_stills_size_under_its_deck_key() {
        let webp = webp_of(vec![40; 1600 * 900 * 4], (1600, 900)).unwrap();
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
        let back = image::load_from_memory_with_format(&webp, image::ImageFormat::WebP).unwrap();
        assert_eq!((back.width(), back.height()), STILL);
        assert!(webp.len() < 256 * 1024, "under the server's limit");
        assert_eq!(still_key(Workspace::Scene), "workspace:deck-stage");
        assert_eq!(still_key(Workspace::Rigging), "workspace:deck-rigging");
    }

    #[test]
    fn signed_out_nothing_is_kept_to_send_and_a_picture_waits_for_the_workspace_to_settle() {
        let mut r = DeviceRow::default();
        let last = BTreeMap::from([(Workspace::Scene, "scene:nave".to_string())]);
        r.follow("http://127.0.0.1:9", None, (Some(Workspace::Scene), Some("scene:nave".into())), &last, false, 10);
        assert!(r.sent.is_none() && r.tx.is_none(), "signed out: no row, no sender");
        assert!(!r.wants_still(10 + SETTLE - 1));
        assert!(r.wants_still(10 + SETTLE));
        r.took(vec![0; 64 * 64 * 4], (64, 64));
        assert!(!r.wants_still(10 + SETTLE + 1), "taken; the next one later");
        assert!(r.stills.contains_key(&Workspace::Scene));
        r.follow("http://127.0.0.1:9", None, (Some(Workspace::Scene), Some("scene:nave".into())), &last, true, 200);
        assert!(!r.wants_still(1000), "the switcher covers it: no picture");
        assert!(r.stills.is_empty(), "left: its picture is let go (signed out: not sent)");
    }
}
