//! The relay as a tree (#90, SHOW.md §4): a child conductor's relay dials its parent's relay, sits there in a seat (role `seat`,
//! its group the seat's id), and holds the other end as one client of its own session (`up`, session.rs). The child dials
//! because it is the child that moves: a foyer's deck behind a venue's NAT, the top one on the internet or the LAN.
//!
//! This only moves bytes: what comes down goes to the child's conductor, the rules of the tree are deck-show's (RULES A1).
//! If the parent goes away nothing happens to what plays; the link keeps trying and the snapshot of the next link says where
//! the parent stands.
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Hello, Message, Ping, Role, Snapshot, PARENT};
use tokio_tungstenite::tungstenite::Message as Ws;

use crate::outbox::Outbox;
use crate::session::Client;
use crate::{now_ms, Relay};

/// the client id of the `up` end in the child's session
const UP: &str = PARENT;
/// waits between two attempts: from the first to the last, then the last again
const RETRY: [Duration; 4] = [Duration::from_millis(100), Duration::from_millis(500), Duration::from_secs(2), Duration::from_secs(5)];
/// silence on the link past this: the parent is gone (the same as the relay's own IDLE_TIMEOUT)
const SILENCE: Duration = crate::IDLE_TIMEOUT;
/// B.6 clock: samples taken at join and how often one more is taken
const SAMPLES: usize = 5;
const PING_EVERY: Duration = Duration::from_secs(5);

/// Which relay a child conductor's relay hangs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parent {
    /// `ws://host:7700`
    pub url: String,
    /// the parent's session
    pub code: String,
    /// the seat's id in the parent's show
    pub seat: String,
    pub token: String,
    /// the child's own session
    pub session: String,
}

impl Parent {
    /// From the environment (main.rs); all of it or none.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Option<Parent> {
        Some(Parent {
            url: get("RELAY_PARENT_URL")?,
            code: get("RELAY_PARENT_CODE")?,
            seat: get("RELAY_PARENT_SEAT")?,
            token: get("RELAY_PARENT_TOKEN")?,
            session: get("RELAY_SESSION")?,
        })
    }

    fn link_url(&self) -> String {
        format!("{}/live/{}?role=seat&group={}", self.url.trim_end_matches('/'), self.code, self.seat)
    }
}

/// B.6's clock over the link: the parent's relay time minus ours, the median of the last few samples.
#[derive(Debug, Default)]
struct Offset {
    samples: VecDeque<i64>,
}

impl Offset {
    fn sample(&mut self, t0: i64, t1: i64, t2: i64) {
        self.samples.push_back(t1 - (t0 + t2) / 2);
        if self.samples.len() > SAMPLES {
            self.samples.pop_front();
        }
    }

    fn get(&self) -> i64 {
        let mut s: Vec<i64> = self.samples.iter().copied().collect();
        s.sort_unstable();
        s.get(s.len() / 2).copied().unwrap_or(0)
    }

    /// a moment in the parent's time, in ours
    fn map(&self, at: i64) -> i64 {
        at - self.get()
    }
}

/// Down the link, on the child's time. Times are relay times: the two relays' clocks differ.
fn on_our_time(mut e: Envelope, offset: &Offset) -> Envelope {
    e.at = e.at.map(|at| offset.map(at));
    if let Message::Tempo(t) = &mut e.message {
        t.at = t.at.map(|at| offset.map(at));
    }
    e
}

fn ping() -> String {
    let p = Envelope::new("", Message::Ping(Ping { t0: now_ms() }));
    serde_json::to_string(&p).expect("a ping serializes")
}

/// What the welcome's snapshot says that a child's conductor has to hear.
fn from_snapshot(snapshot: Snapshot) -> Vec<Message> {
    let mut out = vec![];
    if let Some(tempo) = snapshot.tempo {
        out.push(Message::Tempo(tempo));
    }
    if let Some(flash) = snapshot.flash {
        out.push(Message::Flash(flash));
    }
    if let Some(word) = snapshot.seat {
        out.push(Message::Seat(word));
    }
    out
}

/// Link the session to its parent, for as long as this runs. Every break: try again, the child plays on meanwhile.
pub async fn keep_linked(relay: Arc<Relay>, parent: Parent) {
    let mut failures = 0;
    loop {
        match link(&relay, &parent).await {
            Ok(()) => failures = 0,
            Err(e) => tracing::debug!(parent = parent.url, "parent link: {e}"),
        }
        relay.with_or_new_session(&parent.session, |s| {
            s.leave(UP, now_ms());
            s.parent_online(false);
        });
        tokio::time::sleep(RETRY[failures.min(RETRY.len() - 1)]).await;
        failures += 1;
    }
}

async fn link(relay: &Arc<Relay>, parent: &Parent) -> anyhow::Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(parent.link_url()).await?;
    let (mut sink, mut source) = ws.split();
    let hello = Hello { code: parent.code.clone(), role: Role::Seat, group: Some(parent.seat.clone()), nick: None, token: Some(parent.token.clone()), resume: None };
    sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello)))?.into())).await?;

    let mut offset = Offset::default();
    let outbox = Outbox::new();
    relay.with_or_new_session(&parent.session, |s| {
        let mut up = Client::new(Role::Seat, Some(PARENT.to_owned()), None, outbox.clone(), now_ms());
        up.up = true;
        s.join(UP.to_owned(), up);
        s.parent_online(true);
    });
    sink.send(Ws::Text(ping().into())).await?;
    let writer = tokio::spawn(crate::write(sink, outbox.clone(), relay.clone(), parent.session.clone()));
    let result = read(relay, parent, &mut source, &outbox, &mut offset).await;
    outbox.close();
    writer.abort();
    result
}

async fn read<S>(relay: &Arc<Relay>, parent: &Parent, source: &mut S, outbox: &Arc<Outbox>, offset: &mut Offset) -> anyhow::Result<()>
where
    S: futures_util::Stream<Item = Result<Ws, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let mut clock = tokio::time::interval(PING_EVERY);
    clock.tick().await;
    loop {
        let frame = tokio::select! {
            frame = tokio::time::timeout(SILENCE, source.next()) => frame,
            _ = clock.tick() => {
                relay.with_or_new_session(&parent.session, |s| s.send_to(UP, Message::Ping(Ping { t0: now_ms() })));
                continue;
            }
            _ = outbox.dropped() => return Ok(()),
        };
        let text = match frame {
            Ok(Some(Ok(Ws::Text(t)))) => t,
            Ok(Some(Ok(Ws::Close(_)))) | Ok(None) => return Ok(()),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => return Err(e.into()),
            Err(_) => anyhow::bail!("the parent went silent"),
        };
        let Ok(e) = serde_json::from_str::<Envelope>(&text) else { continue };
        let now = now_ms();
        match e.message {
            Message::Pong(p) => offset.sample(p.t0, p.t1, now),
            Message::Welcome(w) => {
                offset.sample(now, w.now, now); // a first guess until the first pong is back
                relay.with_or_new_session(&parent.session, |s| {
                    for message in from_snapshot(w.snapshot) {
                        // what the child already heard is not news: a reconnect must not fire the row over what the operator did since
                        if !matches!(&message, Message::Seat(w) if s.parent_word() == Some(w)) {
                            s.parent_says(on_our_time(Envelope::new(PARENT, message), offset), now);
                        }
                    }
                });
            }
            Message::Presence(_) | Message::Error(_) | Message::Snapshot(_) => {}
            _ => {
                let e = on_our_time(e, offset);
                relay.with_or_new_session(&parent.session, |s| s.parent_says(e, now));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_median_of_the_samples_takes_out_a_late_one() {
        let mut o = Offset::default();
        for (t0, t1, t2) in [(100, 1100, 102), (200, 1200, 202), (300, 1800, 900), (400, 1400, 402), (500, 1500, 502)] {
            o.sample(t0, t1, t2);
        }
        // a parent 999 ms ahead, one sample with a long way back
        assert_eq!(o.get(), 999);
        assert_eq!(o.map(2_000), 1_001);
    }

    #[test]
    fn all_of_it_or_no_parent() {
        let get = |all: bool| {
            move |k: &str| match k {
                "RELAY_PARENT_URL" => Some("ws://top:7700".to_string()),
                "RELAY_PARENT_CODE" => Some("TOP".to_string()),
                "RELAY_PARENT_SEAT" => Some("foyer".to_string()),
                "RELAY_PARENT_TOKEN" => Some("t".to_string()),
                "RELAY_SESSION" if all => Some("FOYER".to_string()),
                _ => None,
            }
        };
        let parent = Parent::from_env(get(true)).unwrap();
        assert_eq!(parent.link_url(), "ws://top:7700/live/TOP?role=seat&group=foyer");
        assert_eq!(Parent::from_env(get(false)), None);
    }
}
