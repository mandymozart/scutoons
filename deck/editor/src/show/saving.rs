//! Show's documents save themselves (autosave.rs): each show document edited, by its id, through /api/shows (the cache
//! keeps it either way), and the scene sheet's scene once it names an id, through /api/scenes on its version.
use super::api::Refusal;
use super::Host;
use crate::autosave::{fingerprint, Autosave};
use deck_show::Show;
use deck_space::scene_document::SceneDocument;
use std::time::Instant;

impl Host {
    /// the document of show `id`: the top's or one entered
    fn document(&self, id: &str) -> Option<&Show> {
        let top = self.sub.conductor().show();
        if top.id == id {
            Some(top)
        } else {
            self.library.get(id)
        }
    }

    /// each frame: the answers, the shown document and the sheet's scene looked at, what is due sent
    pub fn autosave(&mut self, leave: bool, now: Instant) {
        for a in self.saves.values_mut() {
            if let Some((_, Err(e @ (Refusal::Conflict { .. } | Refusal::Other(_))))) = a.poll() {
                self.status = format!("not saved: {e}");
            }
        }
        let shown = self.shown();
        let (id, print, version) = (shown.id.clone(), fingerprint(shown), shown.version);
        self.saves.entry(id).or_insert_with(|| Autosave::new((version > 0).then_some(u64::from(version)))).look(print, now);
        let due: Vec<String> = self.saves.iter().filter(|(_, a)| a.due(now) || (leave && a.leave())).map(|(id, _)| id.clone()).collect();
        for id in due {
            self.send_show(&id, now);
        }
        self.compose_autosave(leave, now);
    }

    fn send_show(&mut self, id: &str, now: Instant) {
        let Some(doc) = self.document(id).and_then(|s| serde_json::to_value(s).ok()) else { return };
        let (api, key) = (self.api.clone(), id.to_string());
        if let Some(a) = self.saves.get_mut(id) {
            a.send(now, move |_| {
                let answer = api.save(&key, &doc);
                (key, answer)
            });
        }
    }

    fn compose_answer(&mut self, doc: SceneDocument, answer: Result<u64, Refusal>) {
        match answer {
            Ok(version) => {
                let key = format!("scene:{}", doc.id);
                if self.compose.id == doc.id {
                    (self.compose.base, self.compose.doc) = (Some(version), Some(doc.clone()));
                }
                if !self.compose.drafts.iter().any(|(a, _)| *a == key) {
                    self.compose.drafts.push((key, doc.stage.name.clone()));
                }
            }
            Err(Refusal::Offline(_)) => {}
            Err(e) => self.status = format!("not saved: {e}"),
        }
    }

    fn compose_autosave(&mut self, leave: bool, now: Instant) {
        if let Some((doc, answer)) = self.compose.auto.poll() {
            self.compose_answer(doc, answer);
        }
        if self.compose.id.is_empty() {
            return;
        }
        self.compose.auto.look(fingerprint(&self.compose_document()), now);
        if self.compose.auto.due(now) || (leave && self.compose.auto.leave()) {
            self.compose_send(now);
        }
    }

    pub(super) fn compose_send(&mut self, now: Instant) {
        let doc = self.compose_document();
        self.compose.auto.version = self.compose.base;
        match self.scene_job(&doc) {
            Ok(job) => self.compose.auto.send(now, job),
            Err(why) => self.compose.auto.refuse(why),
        }
    }

    /// ctrl+s in Show: the sheet's scene while it is open, else the shown document
    pub fn save_now(&mut self, now: Instant) {
        if self.sheet == Some(super::Sheet::Scene) {
            if self.compose.auto.now() {
                self.compose_send(now);
            }
            return;
        }
        let id = self.shown().id.clone();
        if self.saves.get(&id).is_some_and(Autosave::now) {
            self.send_show(&id, now);
        }
    }

    /// the pill: the sheet's scene while it is open, else the shown document
    pub fn save_pill(&self) -> String {
        if self.sheet == Some(super::Sheet::Scene) && !self.compose.id.is_empty() {
            return format!("scene:{} {}", self.compose.id, self.compose.auto.pill());
        }
        self.saves.get(&self.shown().id).map_or_else(|| "not saved yet".into(), Autosave::pill)
    }

    /// quitting: the answers on their way
    pub fn wait_saves(&mut self, until: Instant) {
        for a in self.saves.values_mut() {
            a.wait(until);
        }
        self.compose.auto.wait(until);
    }

    /// a test's save of the sheet's scene: sent at once, its answer waited for and said in the status
    #[cfg(test)]
    pub fn compose_save_waiting(&mut self) {
        let now = Instant::now();
        self.compose.auto.edit(now);
        self.compose_send(now);
        if let crate::autosave::State::Refused(why) = &self.compose.auto.state {
            self.status = format!("not saved: {why}");
            return;
        }
        let Some((doc, answer)) = crate::autosave::tests::settle(&mut self.compose.auto) else { return };
        let words = match &answer {
            Ok(v) => crate::rigs::saved_words(*v, &crate::rigs::creator_link(&self.api.base, &format!("scene:{}", doc.id))),
            Err(e) => format!("not saved: {e}"),
        };
        self.compose_answer(doc, answer);
        self.status = words;
    }

    /// a test's save: sent at once and its answer waited for
    #[cfg(test)]
    pub fn save_waiting(&mut self) {
        let now = Instant::now();
        self.autosave(false, now);
        let id = self.shown().id.clone();
        self.send_show(&id, now);
        if let Some(a) = self.saves.get_mut(&id) {
            crate::autosave::tests::settle(a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autosave::{tests::Fake, IDLE, RETRY};
    use std::time::Duration;

    fn host(base: &str) -> Host {
        let mut host = Host::new(None, None, None, base.to_string());
        host.api = super::super::api::Api::with_cache(base.to_string(), std::env::temp_dir().join(format!("deck-editor-show-saves-{}", std::process::id())));
        host
    }

    fn rename(host: &mut Host, to: &str) {
        let mut show = host.shown().clone();
        show.outlets[0].id = to.to_string();
        host.sub = deck_show::Subtree::new(show, deck_show::ClockChoice::Parent);
    }

    /// the sheet's scene answered, and the answer taken in by the frame
    fn answered(host: &mut Host) {
        for _ in 0..500 {
            host.autosave(false, Instant::now());
            if !host.compose.auto.busy() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("no answer");
    }

    fn wait(host: &mut Host, id: &str) {
        let a = host.saves.get_mut(id).expect("looked at");
        crate::autosave::tests::settle(a);
    }

    /// the panel model on a fake API: a burst of edits is one PUT once it is over; leaving the workspace sends at once
    #[test]
    fn a_show_saves_itself_once_per_burst_and_on_leaving() {
        let fake = Fake::start(&[]);
        let mut host = host(&fake.base);
        let id = host.shown().id.clone();
        let t0 = Instant::now();
        host.autosave(false, t0);
        assert_eq!(host.save_pill(), "saved · v1", "the empty show's own version");
        for (i, name) in ["a", "b", "c", "d"].into_iter().enumerate() {
            rename(&mut host, name);
            host.autosave(false, t0 + Duration::from_millis(500 * i as u64));
        }
        assert_eq!(host.save_pill(), "saving…");
        assert_eq!(fake.count(), 0, "not while the burst goes on");
        host.autosave(false, t0 + Duration::from_millis(1500) + IDLE);
        host.autosave(false, t0 + Duration::from_millis(1600) + IDLE);
        wait(&mut host, &id);
        assert_eq!(fake.count(), 1, "one PUT for the burst");
        assert_eq!(fake.puts.lock().unwrap()[0].1["content"]["outlets"][0]["id"], "d");
        host.autosave(false, t0 + IDLE * 3);
        assert_eq!(host.save_pill(), "saved · v101");
        rename(&mut host, "e");
        let t1 = t0 + IDLE * 4;
        host.autosave(false, t1);
        host.autosave(true, t1 + Duration::from_millis(10));
        wait(&mut host, &id);
        assert_eq!(fake.count(), 2, "leaving saved it at once");
    }

    /// no answer: kept (the cache), the pill says so, sent again once the API answers
    #[test]
    fn offline_the_show_is_kept_and_sent_later() {
        let mut host = host("http://127.0.0.1:9/api");
        let id = host.shown().id.clone();
        let t0 = Instant::now();
        host.autosave(false, t0);
        rename(&mut host, "x");
        host.autosave(false, t0);
        host.autosave(false, t0 + IDLE);
        wait(&mut host, &id);
        assert_eq!(host.save_pill(), "offline – kept locally");
        let fake = Fake::start(&[]);
        host.api = super::super::api::Api::with_cache(fake.base.clone(), std::env::temp_dir().join(format!("deck-editor-show-saves-{}", std::process::id())));
        host.autosave(false, t0 + IDLE + Duration::from_secs(1));
        assert_eq!(fake.count(), 0, "not before the retry");
        host.autosave(false, t0 + IDLE + RETRY);
        wait(&mut host, &id);
        assert_eq!((fake.count(), host.save_pill()), (1, "saved · v101".to_string()));
    }

    /// the scene sheet's scene saves itself on its version; someone else's newer one: a conflict naming both, kept
    #[test]
    fn the_sheets_scene_saves_itself_and_a_conflict_names_both_versions() {
        let fake = Fake::start(&[200, 409]);
        let mut host = host(&fake.base);
        host.toon = Some("catpope".into());
        host.sheet = Some(super::super::Sheet::Scene);
        let t0 = Instant::now();
        (host.compose.id, host.compose.name, host.compose.cast) = ("duet".into(), "Duet".into(), vec!["character:catpope".into()]);
        host.autosave(false, t0);
        host.compose.cast.push("item:incense_burner".into());
        host.autosave(false, t0);
        host.autosave(false, t0 + IDLE);
        answered(&mut host);
        host.autosave(false, t0 + IDLE);
        assert_eq!((host.compose.base, host.save_pill()), (Some(101), "scene:duet saved · v101".to_string()));
        assert!(host.compose.drafts.iter().any(|(a, _)| a == "scene:duet"));
        host.compose.name = "Duet two".into();
        host.autosave(false, t0 + IDLE * 2);
        host.autosave(false, t0 + IDLE * 3);
        answered(&mut host);
        host.autosave(false, t0 + IDLE * 3);
        let puts = fake.puts.lock().unwrap().clone();
        assert_eq!((puts.len(), puts[1].0.as_str(), &puts[1].1["base"]), (2, "/api/scenes/duet", &serde_json::json!(101)));
        assert_eq!(host.save_pill(), "scene:duet conflict – v902 on the server, yours (on v101) kept locally");
        assert!(host.status.contains("someone saved a newer version (v902)"), "{}", host.status);
        host.autosave(true, t0 + IDLE * 10);
        assert_eq!(fake.count(), 2, "nothing more is sent");
    }
}
