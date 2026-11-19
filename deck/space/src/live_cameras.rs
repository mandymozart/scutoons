//! The live cameras (agents/research/LIVE-CAMERAS.md): a phone holds a camera of the live session and steers it
//! with its motion. Here, once for every client of the core (RULES A6): the links a QR carries (`camera_link`,
//! `cameras_link`), a phone's hold on a camera as its page hears it (`Seat`, driven by relay.rs `Client::camera_claim`;
//! the relay's rule is relay cameras.rs) and the camera on the deck that follows the phone (`Follow`, an external pose
//! source over the camera the deck had – recentered on it, so nothing jumps).
use crate::relay::{encode, parse_relay, relay_param, HOSTED_RELAY};
#[cfg(feature = "being")]
use crate::space::camera::Over;
#[cfg(feature = "being")]
use glam::{Quat, Vec3};
use protocol::{CameraClaim, LiveCamera};
use serde::Serialize;

/// the relay part of a link a conductor offers: its base and QUIC door, only when it is not the hosted relay
/// (as linked.rs `reader_link`)
fn relay_part(relay: Option<&str>) -> String {
    let hosted = parse_relay(HOSTED_RELAY).map(|(b, _)| b).unwrap_or_default();
    let door = |url: &str| match relay_param(url, "quic") {
        Some(off) if off == "off" => String::new(),
        Some(door) => format!("&quic={}{}", encode(&door), relay_param(url, "quichash").filter(|h| !h.is_empty()).map(|h| format!("&quichash={}", encode(&h))).unwrap_or_default()),
        None => String::new(),
    };
    relay
        .and_then(|r| parse_relay(r).ok().map(|(b, _)| (b, r)))
        .filter(|(b, _)| !b.eq_ignore_ascii_case(&hosted))
        .map(|(b, r)| format!("&relay={}{}", encode(&b), door(r)))
        .unwrap_or_default()
}

/// The handheld page of one camera: `<base>/live/cameras/<id>?live=<code>&relay=<relay>` – the id (its address, e.g.
/// `scene:duet > shot:close`) escaped into one path segment.
pub fn camera_link(base: &str, camera: &str, code: &str, relay: Option<&str>) -> String {
    format!("{}/live/cameras/{}?live={}{}", base.trim().trim_end_matches('/'), encode(camera.trim()), encode(code.trim()), relay_part(relay))
}

/// The list of the session's cameras: `<base>/live/cameras?live=<code>&relay=<relay>`.
pub fn cameras_link(base: &str, code: &str, relay: Option<&str>) -> String {
    format!("{}/live/cameras?live={}{}", base.trim().trim_end_matches('/'), encode(code.trim()), relay_part(relay))
}

/// Where a phone stands with the camera it wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    /// it holds none
    #[default]
    None,
    /// the person said yes; the relay's word is on its way
    Asked,
    /// it holds the camera: its pose steers it
    Held,
    /// a newer claim took it: Take it back claims again
    TakenOver,
}

/// A phone's hold as its page shows it (relay.rs `Out::Holding`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Holding {
    pub hold: Hold,
    /// the camera it holds, asked for or lost (a forced switch names the new one)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    /// another phone holds that camera now: its name, else its client id (the take-over question, the warning)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
    /// why the relay refused the last claim (the camera's rule)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// A phone's claim on a camera and the relay's word (relay.rs keeps one per client). `key`: the phone's secret its
/// browser keeps – a drop or a reload is the same holder.
#[derive(Debug, Clone, Default)]
pub struct Seat {
    key: String,
    want: Option<String>,
    hold: Hold,
    /// its client id when it last held (a list sent before its return still names that one)
    was: Option<String>,
    /// a return's claim is out: the old id in the list is still its own
    returning: bool,
    refused: Option<String>,
    list: Vec<LiveCamera>,
    told: Option<Holding>,
}

impl Seat {
    pub fn new(key: &str) -> Seat {
        Seat { key: key.to_string(), ..Seat::default() }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// the person said yes to `camera` (or Take it back): the claim to send when joined
    pub fn claim(&mut self, camera: &str) -> CameraClaim {
        self.want = Some(camera.to_string());
        self.hold = Hold::Asked;
        self.refused = None;
        CameraClaim { camera: camera.to_string(), key: self.key.clone(), resume: None }
    }

    /// After a reload: the page held `camera` as the client `was` (its browser kept both) – it takes it again by itself at
    /// the welcome, as after a drop
    pub fn resume(&mut self, camera: &str, was: &str) {
        (self.want, self.hold, self.was, self.returning, self.refused) = (Some(camera.to_string()), Hold::Held, Some(was.to_string()), true, None);
    }

    /// Leave: the camera to let go, if it holds or asked for one
    pub fn leave(&mut self) -> Option<String> {
        let camera = self.want.take().filter(|_| matches!(self.hold, Hold::Held | Hold::Asked));
        (self.hold, self.returning) = (Hold::None, false);
        camera
    }

    /// A welcome as `me`: the claim to send – a yes still waiting goes again; a held camera is taken again by itself (a
    /// `resume`: the relay grants it while nobody else claimed it meanwhile).
    pub fn welcome(&mut self, me: &str) -> Option<CameraClaim> {
        let camera = self.want.clone()?;
        match self.hold {
            Hold::Asked => Some(CameraClaim { camera, key: self.key.clone(), resume: None }),
            Hold::Held => {
                self.returning = self.was.as_deref() != Some(me);
                Some(CameraClaim { camera, key: self.key.clone(), resume: Some(true) })
            }
            Hold::None | Hold::TakenOver => None,
        }
    }

    /// the relay's list as `me`: the camera it holds (a forced switch too), or who took it
    pub fn heard(&mut self, me: &str, list: Vec<LiveCamera>) {
        self.list = list;
        if let Some(c) = self.list.iter().find(|c| c.holder.as_deref() == Some(me)) {
            (self.want, self.hold, self.was, self.returning, self.refused) = (Some(c.id.clone()), Hold::Held, Some(me.to_string()), false, None);
            return;
        }
        let holder = self.want.as_ref().and_then(|w| self.list.iter().find(|c| &c.id == w)).and_then(|c| c.holder.clone());
        match (self.hold, holder) {
            // still its own old id: the return is on its way
            (Hold::Held, Some(h)) if self.returning && self.was.as_deref() == Some(h.as_str()) => {}
            (Hold::Held, None) if self.returning => {}
            (Hold::Held, Some(_)) => (self.hold, self.returning) = (Hold::TakenOver, false),
            // freed: Leave elsewhere, the conductor, the release time
            (Hold::Held, None) => (self.hold, self.want) = (Hold::None, None),
            _ => {}
        }
    }

    /// the relay refused the claim (the camera's rule, no such camera)
    pub fn refused(&mut self, why: &str) {
        if self.hold == Hold::Asked {
            (self.hold, self.want) = (Hold::None, None);
        }
        self.refused = Some(why.to_string());
    }

    pub fn holds(&self) -> bool {
        self.hold == Hold::Held
    }

    pub fn list(&self) -> &[LiveCamera] {
        &self.list
    }

    pub fn now(&self, me: &str) -> Holding {
        let other = self.want.as_ref().and_then(|w| self.list.iter().find(|c| &c.id == w)).filter(|c| c.holder.as_deref().is_some_and(|h| h != me && Some(h) != self.was.as_deref().filter(|_| self.returning))).map(|c| c.nick.clone().or(c.holder.clone()).unwrap_or_default());
        Holding { hold: self.hold, camera: self.want.clone(), other, refused: self.refused.clone() }
    }

    /// what changed since it was last told (relay.rs `Out::Holding`); a phone that never took part says nothing
    pub fn change(&mut self, me: &str) -> Option<Holding> {
        let now = self.now(me);
        let quiet = self.told.is_none() && now == Holding::default();
        (!quiet && self.told.as_ref() != Some(&now)).then(|| {
            self.told = Some(now.clone());
            now
        })
    }
}

#[cfg(feature = "being")]
/// how fast the held view follows the phone (s): a short ease against the sensor's jitter, no lag a hand feels
pub const FOLLOW_EASE_S: f32 = 0.06;

/// The deck's camera following a phone (INPUT.md's takeover rule): the deck's own view is put aside, not changed. The
/// first pose (and each Recenter) anchors the phone's pose on the view the deck had – only the heading is matched, up
/// stays up (both worlds are gravity-aligned), so walking forward moves the camera forward on the stage's ground. A
/// late or malformed pose never steers it; while the phone is away the camera keeps its last view.
#[cfg(feature = "being")]
#[derive(Debug, Clone, Default)]
pub struct Follow {
    /// the anchor: the phone's pose and the camera's (eye, rotation) at the recenter, and the heading turn between them
    origin: Option<Origin>,
    /// a Recenter waits for the next pose
    recenter: bool,
    /// the newest pose's time (the phone's clock, ms)
    last_t: Option<f64>,
    /// the view it shows now and where it goes
    eye: Option<Vec3>,
    rot: Option<Quat>,
    goal: Option<(Vec3, Quat)>,
}

#[cfg(feature = "being")]
#[derive(Debug, Clone, Copy)]
struct Origin {
    phone_p: Vec3,
    phone_q: Quat,
    cam_p: Vec3,
    cam_q: Quat,
    turn: Quat,
}

/// the heading of a camera's rotation about the vertical (rad): from its forward axis, its up axis when it looks
/// straight down or up
#[cfg(feature = "being")]
pub fn yaw_of(q: Quat) -> f32 {
    let f = q * Vec3::NEG_Z;
    if f.x.hypot(f.z) < 0.2 {
        let u = q * Vec3::Y * if f.y < 0.0 { 1.0 } else { -1.0 };
        return (-u.x).atan2(-u.z);
    }
    (-f.x).atan2(-f.z)
}

/// a camera's rotation looking from `eye` at `target` (Y up; -Z forward)
#[cfg(feature = "being")]
pub fn look_rotation(eye: Vec3, target: Vec3) -> Quat {
    let m = glam::Mat4::look_at_rh(eye, target, Vec3::Y);
    Quat::from_mat4(&m.inverse())
}

#[cfg(feature = "being")]
impl Follow {
    /// Recenter: the phone's next pose = the deck's camera now
    pub fn recenter(&mut self) {
        self.recenter = true;
    }

    /// A pose of the phone at `t` (its clock, ms): position (3), rotation (x y z w). `camera`: the deck's own view now
    /// (eye, rotation) – the anchor when there is none yet or a Recenter waits. False: late or malformed, dropped.
    pub fn pose(&mut self, t: f64, p: &[f64], q: &[f64], camera: (Vec3, Quat)) -> bool {
        let finite = |v: &[f64], n: usize| v.len() == n && v.iter().all(|x| x.is_finite());
        if !(t.is_finite() && finite(p, 3) && finite(q, 4)) || self.last_t.is_some_and(|l| t <= l) {
            return false;
        }
        self.last_t = Some(t);
        let pp = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
        let pq = Quat::from_xyzw(q[0] as f32, q[1] as f32, q[2] as f32, q[3] as f32).normalize();
        if self.origin.is_none() || self.recenter {
            // from where the view is now (a held view stays put on a recenter), else the deck's own camera
            let (cam_p, cam_q) = self.eye.zip(self.rot).unwrap_or(camera);
            let turn = Quat::from_rotation_y(yaw_of(cam_q) - yaw_of(pq));
            self.origin = Some(Origin { phone_p: pp, phone_q: pq, cam_p, cam_q, turn });
            self.recenter = false;
        }
        let o = self.origin.expect("anchored");
        let eye = o.cam_p + o.turn * (pp - o.phone_p);
        let rot = (o.turn * (pq * o.phone_q.inverse()) * o.turn.inverse() * o.cam_q).normalize();
        self.goal = Some((eye, rot));
        if self.eye.is_none() {
            (self.eye, self.rot) = (Some(eye), Some(rot));
        }
        true
    }

    /// The view eased toward the newest pose by `dt` s; none until the first pose.
    pub fn step(&mut self, dt: f32) -> Option<(Vec3, Quat)> {
        let (ge, gr) = self.goal?;
        let k = 1.0 - (-dt.max(0.0) / FOLLOW_EASE_S).exp();
        let eye = self.eye.unwrap_or(ge).lerp(ge, k);
        let rot = self.rot.unwrap_or(gr).slerp(gr, k);
        (self.eye, self.rot) = (Some(eye), Some(rot));
        Some((eye, rot))
    }

    /// the held view as the picture takes it (over the deck's camera): eye, a point ahead of it, the lens
    pub fn over(&self, fov_y: f32, ortho: Option<f32>) -> Option<Over> {
        let (eye, rot) = self.eye.zip(self.rot)?;
        Some(Over { eye, target: eye + rot * Vec3::NEG_Z, fov_y, ortho })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(id: &str, holder: Option<&str>) -> LiveCamera {
        LiveCamera { id: id.into(), label: None, r#for: None, release_ms: None, holder: holder.map(String::from), nick: None, away: None }
    }

    #[test]
    fn the_links_a_qr_carries() {
        assert_eq!(camera_link("https://scutoons.com/", "scene:duet > shot:close", "H7K2", None), "https://scutoons.com/live/cameras/scene%3Aduet%20%3E%20shot%3Aclose?live=H7K2");
        assert_eq!(cameras_link("http://192.168.1.4:5173", "H7K2", Some("ws://192.168.1.4:7071/live/H7K2")), "http://192.168.1.4:5173/live/cameras?live=H7K2&relay=ws%3A%2F%2F192.168.1.4%3A7071");
    }

    #[test]
    fn a_phone_holds_from_the_relay_s_word_and_is_taken_over() {
        let mut s = Seat::new("k1");
        assert_eq!(s.welcome("c1"), None, "nothing asked: nothing claimed");
        s.heard("c1", vec![cam("a", Some("c9"))]);
        let claim = s.claim("a");
        assert_eq!((claim.camera.as_str(), claim.key.as_str(), claim.resume), ("a", "k1", None));
        assert_eq!(s.now("c1"), Holding { hold: Hold::Asked, camera: Some("a".into()), other: Some("c9".into()), refused: None }, "the take-over question names the holder");
        s.heard("c1", vec![cam("a", Some("c1"))]);
        assert!(s.holds());
        s.heard("c1", vec![cam("a", Some("c2"))]);
        assert_eq!(s.now("c1").hold, Hold::TakenOver, "a newer claim took it");
        assert_eq!(s.now("c1").other.as_deref(), Some("c2"));
        s.claim("a");
        s.heard("c1", vec![cam("a", Some("c1"))]);
        assert!(s.holds(), "taken back");
        assert_eq!(s.leave().as_deref(), Some("a"));
        assert_eq!(s.now("c1").hold, Hold::None);
    }

    #[test]
    fn a_held_camera_is_taken_again_after_a_drop_or_a_reload() {
        let mut s = Seat::new("k1");
        s.claim("a");
        s.heard("c1", vec![cam("a", Some("c1"))]);
        // a reload: a new client id; the relay's first list still names the old one, away
        let back = s.welcome("c5").expect("it claims again by itself");
        assert_eq!(back.resume, Some(true));
        s.heard("c5", vec![LiveCamera { away: Some(true), ..cam("a", Some("c1")) }]);
        assert_eq!(s.now("c5").hold, Hold::Held, "the old id is its own: no take-over shown");
        assert_eq!(s.now("c5").other, None);
        s.heard("c5", vec![cam("a", Some("c5"))]);
        assert!(s.holds());
        // a drop while someone else claimed it: the return is refused, the list names the other
        s.welcome("c6");
        s.heard("c6", vec![cam("a", Some("c8"))]);
        assert_eq!(s.now("c6").hold, Hold::TakenOver);
        assert_eq!(s.welcome("c7"), None, "taken over: no claim by itself");
        // a reload: a fresh seat, told what the browser kept
        let mut r = Seat::new("k1");
        r.resume("a", "c5");
        assert_eq!(r.welcome("c11").and_then(|c| c.resume), Some(true));
        r.heard("c11", vec![LiveCamera { away: Some(true), ..cam("a", Some("c5")) }]);
        assert_eq!((r.now("c11").hold, r.now("c11").other), (Hold::Held, None));
        r.heard("c11", vec![cam("a", Some("c11"))]);
        assert!(r.holds());
    }

    #[test]
    fn a_forced_switch_a_free_and_a_refusal() {
        let mut s = Seat::new("k1");
        s.claim("a");
        s.heard("c1", vec![cam("a", Some("c1")), cam("b", None)]);
        s.heard("c1", vec![cam("a", None), cam("b", Some("c1"))]);
        assert_eq!((s.now("c1").hold, s.now("c1").camera.as_deref()), (Hold::Held, Some("b")), "the page names the new camera");
        s.heard("c1", vec![cam("a", None), cam("b", None)]);
        assert_eq!(s.now("c1").hold, Hold::None, "freed by the conductor or the release time");
        s.claim("a");
        s.refused("this camera is not open to you");
        assert_eq!((s.now("c1").hold, s.now("c1").refused.as_deref()), (Hold::None, Some("this camera is not open to you")));
        assert!(s.change("c1").is_some() && s.change("c1").is_none(), "each new state is told once");
    }

    #[cfg(feature = "being")]
    fn close(a: glam::Vec3, b: glam::Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    #[cfg(feature = "being")]
    #[test]
    fn the_deck_camera_follows_the_phone_from_where_it_was() {
        use glam::{Quat, Vec3};
        let mut f = Follow::default();
        let deck = (Vec3::new(0.0, 1.6, 5.0), look_rotation(Vec3::new(0.0, 1.6, 5.0), Vec3::new(0.0, 1.6, 0.0)));
        // the phone looks along +X (turned a quarter to the left of -Z): the anchor matches the heading, nothing jumps
        let phone_q = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        let q = [phone_q.x as f64, phone_q.y as f64, phone_q.z as f64, phone_q.w as f64];
        assert!(f.pose(10.0, &[1.0, 1.4, 0.0], &q, deck));
        let (eye, rot) = f.step(0.0).unwrap();
        assert!(close(eye, deck.0) && rot.dot(deck.1).abs() > 0.9999, "the first pose is the deck's view");
        // one step forward along the phone's heading = one metre toward the aim on the stage
        assert!(f.pose(20.0, &[2.0, 1.4, 0.0], &q, deck));
        let (eye, _) = f.step(10.0).unwrap();
        assert!(close(eye, Vec3::new(0.0, 1.6, 4.0)), "{eye}");
        assert!(!f.pose(15.0, &[9.0, 9.0, 9.0], &q, deck), "a late pose never steers");
        assert!(!f.pose(30.0, &[0.0, 1.0], &q, deck), "nor a malformed one");
        // Recenter: the next pose is the view it holds now, whatever the phone did meanwhile
        f.recenter();
        assert!(f.pose(40.0, &[7.0, 0.0, 7.0], &[0.0, 0.0, 0.0, 1.0], deck));
        let (eye, _) = f.step(10.0).unwrap();
        assert!(close(eye, Vec3::new(0.0, 1.6, 4.0)));
        let over = f.over(0.8, None).unwrap();
        assert!(close(over.target, over.eye + Vec3::NEG_Z), "aims ahead of the eye");
    }
}
