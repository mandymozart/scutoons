//! The live cameras (agents/research/LIVE-CAMERAS.md): the conductor says which cameras its session has (`cameras`:
//! each with who may take it, the one condition control's filter); a reader's phone takes one (`camera.claim`) and steers
//! it with its pose. One holder per camera, the linked reader's rule (#527): the newest claim wins, the holder before
//! hears it and may take it back. A held camera persists: a phone that drops keeps its claim (shown `away`) and takes it
//! again by itself (`resume`) while the camera's holder is still its key – a reload too, the key lives in its browser. It
//! returns to its owner on Leave, a take-over, the conductor's free, or after `release_ms` without a pose when the
//! conductor set one. Pure state here; session.rs routes, checks the filter (roles.rs `admits`) and tells the clients.
use protocol::{LiveCamera, Loose};
use std::collections::HashMap;

/// Who holds a camera.
#[derive(Debug, Clone, PartialEq)]
struct Held {
    client: String,
    /// the phone's own secret (its browser keeps it): the same phone after a drop or a reload
    key: String,
    nick: Option<String>,
    /// the holder's link dropped at this ms (its claim is kept for its return)
    away: Option<i64>,
    /// its last pose (or the claim), ms
    pose_at: i64,
}

/// Why a claim is not granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// no camera of the session has that id
    Unknown,
    /// a resume, but another phone claimed the camera meanwhile: it stays with that one
    TakenOver,
}

/// The session's cameras and who holds each.
#[derive(Debug, Clone, Default)]
pub struct Cameras {
    defs: Vec<LiveCamera>,
    held: HashMap<String, Held>,
    /// the last flash per camera: each new holder gets it
    flash: HashMap<String, Loose>,
}

impl Cameras {
    /// the conductor's cameras; a camera that is gone lets its holder go
    pub fn set(&mut self, defs: Vec<LiveCamera>) {
        let defs: Vec<LiveCamera> = defs.into_iter().map(|c| LiveCamera { holder: None, nick: None, away: None, ..c }).collect();
        self.held.retain(|id, _| defs.iter().any(|c| &c.id == id));
        self.flash.retain(|id, _| defs.iter().any(|c| &c.id == id));
        self.defs = defs;
    }

    pub fn def(&self, id: &str) -> Option<&LiveCamera> {
        self.defs.iter().find(|c| c.id == id)
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// `client` (its `key`, its name) takes `camera` at `now`. A plain claim always wins (the person said yes); a
    /// `resume` only while the camera is free or still held by this key. The phone's other camera returns to its owner.
    pub fn claim(&mut self, client: &str, key: &str, nick: Option<String>, camera: &str, resume: bool, now: i64) -> Result<(), Refused> {
        if self.def(camera).is_none() {
            return Err(Refused::Unknown);
        }
        if resume && self.held.get(camera).is_some_and(|h| h.key != key) {
            return Err(Refused::TakenOver);
        }
        self.held.retain(|id, h| id == camera || (h.key != key && h.client != client));
        self.held.insert(camera.to_owned(), Held { client: client.to_owned(), key: key.to_owned(), nick, away: None, pose_at: now });
        Ok(())
    }

    /// `client` lets `camera` go; false: it did not hold it
    pub fn leave(&mut self, client: &str, camera: &str) -> bool {
        let holds = self.held.get(camera).is_some_and(|h| h.client == client);
        if holds {
            self.held.remove(camera);
        }
        holds
    }

    /// The conductor: `client` holds `camera` from now on (its key stays, so it can come back), whoever held it; `camera`
    /// empty: what it holds is freed. Err: the phone holds no camera (a force moves an active phone camera) or no such camera.
    pub fn force(&mut self, client: &str, camera: &str, now: i64) -> Result<(), &'static str> {
        let Some((from, h)) = self.held.iter().find(|(_, h)| h.client == client).map(|(id, h)| (id.clone(), h.clone())) else {
            return Err("that phone holds no camera");
        };
        if camera.is_empty() {
            self.held.remove(&from);
            return Ok(());
        }
        if self.def(camera).is_none() {
            return Err("no such camera");
        }
        self.held.remove(&from);
        self.held.insert(camera.to_owned(), Held { away: None, pose_at: now, ..h });
        Ok(())
    }

    /// the client's link is gone: its claims are kept, away
    pub fn gone(&mut self, client: &str, now: i64) -> bool {
        let mut any = false;
        for h in self.held.values_mut().filter(|h| h.client == client && h.away.is_none()) {
            h.away = Some(now);
            any = true;
        }
        any
    }

    /// the camera `client` holds (a pose of it counts as a sign of life)
    pub fn pose(&mut self, client: &str, now: i64) -> Option<&str> {
        let (id, h) = self.held.iter_mut().find(|(_, h)| h.client == client)?;
        h.pose_at = now;
        Some(id.as_str())
    }

    pub fn held_by(&self, client: &str) -> Option<&str> {
        self.held.iter().find(|(_, h)| h.client == client).map(|(id, _)| id.as_str())
    }

    pub fn holder(&self, camera: &str) -> Option<&str> {
        self.held.get(camera).map(|h| h.client.as_str())
    }

    /// Cameras whose conductor set a `release_ms` and that had no pose for that long return to their owner. True: one did.
    pub fn expire(&mut self, now: i64) -> bool {
        let defs = &self.defs;
        let before = self.held.len();
        self.held.retain(|id, h| defs.iter().find(|c| &c.id == id).and_then(|c| c.release_ms).is_none_or(|ms| now - h.pose_at < ms as i64));
        self.held.len() != before
    }

    /// The cameras a client sees, with who holds each: `all` (the conductor, a deck) every one, `with_filter` with its
    /// filter (the conductor only); else those `may` admits.
    pub fn view(&self, all: bool, with_filter: bool, may: impl Fn(&LiveCamera) -> bool) -> Vec<LiveCamera> {
        self.defs
            .iter()
            .filter(|c| all || may(c))
            .map(|c| {
                let h = self.held.get(&c.id);
                LiveCamera {
                    r#for: if with_filter { c.r#for.clone() } else { None },
                    holder: h.map(|h| h.client.clone()),
                    nick: h.and_then(|h| h.nick.clone()),
                    away: h.and_then(|h| h.away.map(|_| true)),
                    ..c.clone()
                }
            })
            .collect()
    }

    /// the conductor's flash for `camera`: kept for its next holders; the holder now, to send it to
    pub fn flash(&mut self, camera: &str, settings: Loose) -> Option<String> {
        self.def(camera)?;
        self.flash.insert(camera.to_owned(), settings);
        self.holder(camera).map(str::to_owned)
    }

    pub fn last_flash(&self, camera: &str) -> Option<&Loose> {
        self.flash.get(camera)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(id: &str, release: Option<u64>) -> LiveCamera {
        LiveCamera { id: id.into(), label: None, r#for: None, release_ms: release, holder: None, nick: None, away: None }
    }

    fn cams() -> Cameras {
        let mut c = Cameras::default();
        c.set(vec![cam("scene:duet > shot:close", None), cam("scene:duet", Some(5_000))]);
        c
    }

    const CLOSE: &str = "scene:duet > shot:close";

    #[test]
    fn the_newest_claim_wins_and_the_one_before_can_take_it_back() {
        let mut c = cams();
        assert_eq!(c.claim("c1", "k1", Some("Maya".into()), CLOSE, false, 0), Ok(()));
        assert_eq!(c.holder(CLOSE), Some("c1"));
        assert_eq!(c.claim("c2", "k2", None, CLOSE, false, 10), Ok(()), "a yes takes it over");
        assert_eq!(c.holder(CLOSE), Some("c2"));
        assert_eq!(c.claim("c1", "k1", None, CLOSE, false, 20), Ok(()), "Take it back is a claim like any other");
        assert_eq!(c.holder(CLOSE), Some("c1"));
        assert_eq!(c.claim("c1", "k1", None, "scene:nowhere", false, 30), Err(Refused::Unknown));
    }

    #[test]
    fn a_phone_holds_one_camera() {
        let mut c = cams();
        c.claim("c1", "k1", None, CLOSE, false, 0).unwrap();
        c.claim("c1", "k1", None, "scene:duet", false, 1).unwrap();
        assert_eq!((c.holder(CLOSE), c.holder("scene:duet")), (None, Some("c1")), "its other camera returns to its owner");
    }

    #[test]
    fn a_dropped_phone_keeps_its_camera_and_takes_it_again_by_itself_unless_someone_claimed_it() {
        let mut c = cams();
        c.claim("c1", "k1", None, CLOSE, false, 0).unwrap();
        assert!(c.gone("c1", 100));
        let v = c.view(true, false, |_| true);
        assert_eq!((v[0].holder.as_deref(), v[0].away), (Some("c1"), Some(true)), "held, away: the camera keeps its last view");
        // a reload: a new client id, the same key – no question asked
        assert_eq!(c.claim("c7", "k1", None, CLOSE, true, 200), Ok(()));
        assert_eq!(c.holder(CLOSE), Some("c7"));
        assert_eq!(c.view(true, false, |_| true)[0].away, None, "back");
        // it drops again and someone else claims meanwhile: the return is refused, the camera stays with the newcomer
        c.gone("c7", 300);
        c.claim("c2", "k2", None, CLOSE, false, 400).unwrap();
        assert_eq!(c.claim("c8", "k1", None, CLOSE, true, 500), Err(Refused::TakenOver));
        assert_eq!(c.holder(CLOSE), Some("c2"));
        // a free camera is taken by a returning phone too
        c.leave("c2", CLOSE);
        assert_eq!(c.claim("c8", "k1", None, CLOSE, true, 600), Ok(()));
    }

    #[test]
    fn leave_force_and_the_release_time() {
        let mut c = cams();
        c.claim("c1", "k1", None, "scene:duet", false, 0).unwrap();
        assert!(!c.leave("c2", "scene:duet"), "only the holder lets go");
        assert_eq!(c.pose("c1", 4_000), Some("scene:duet"));
        assert!(!c.expire(8_999), "a pose 4 s ago keeps it");
        assert!(c.expire(9_000), "5 s without a pose: back to its owner");
        assert_eq!(c.holder("scene:duet"), None);
        // the close camera has no release time: it waits for its phone however long
        c.claim("c1", "k1", None, CLOSE, false, 0).unwrap();
        c.gone("c1", 0);
        assert!(!c.expire(3_600_000));
        // the conductor forces the phone onto another camera, whoever held it; its key goes along (it can come back)
        c.claim("c2", "k2", None, "scene:duet", false, 10).unwrap();
        assert_eq!(c.force("c1", "scene:duet", 20), Ok(()));
        assert_eq!((c.holder(CLOSE), c.holder("scene:duet")), (None, Some("c1")));
        assert_eq!(c.claim("c9", "k1", None, "scene:duet", true, 30), Ok(()), "the forced phone comes back by its key");
        assert_eq!(c.force("c3", CLOSE, 40), Err("that phone holds no camera"));
        assert_eq!(c.force("c9", "", 50), Ok(()), "the conductor frees it");
        assert_eq!(c.holder("scene:duet"), None);
    }

    #[test]
    fn the_view_and_the_flash() {
        let mut c = cams();
        let only_close = |x: &LiveCamera| x.id == CLOSE;
        assert_eq!(c.view(false, false, only_close).len(), 1, "a reader sees what it may take");
        assert_eq!(c.flash("scene:duet", Loose::new()), None, "nobody holds it: kept for the next one");
        c.claim("c1", "k1", Some("Maya".into()), "scene:duet", false, 0).unwrap();
        assert!(c.last_flash("scene:duet").is_some());
        let v = c.view(true, false, |_| false);
        assert_eq!(v[1].nick.as_deref(), Some("Maya"));
        assert_eq!(c.flash("scene:duet", Loose::new()).as_deref(), Some("c1"));
        c.set(vec![cam(CLOSE, None)]);
        assert_eq!((c.holder("scene:duet"), c.last_flash("scene:duet")), (None, None), "a camera that is gone lets its holder go");
    }
}
