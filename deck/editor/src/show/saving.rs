//! Show's documents save themselves (autosave.rs): each show document edited, by its id, through /api/shows on its
//! version, and the scene sheet's scene once it names an id, through /api/scenes on its version. What the outbox kept
//! from before a restart opens as the local copy and goes up first; a scene open nowhere goes up from `unsent`.
use super::api::{Api, Refusal};
use super::Host;
use crate::autosave::{fingerprint, Autosave};
use deck_show::Show;
use deck_space::scene_document::SceneDocument;
use deck_show::{ClockChoice, Subtree};
use std::time::Instant;

/// a scene's save as a new draft on the autosave's version, through the outbox; the core's problems refuse it
pub fn send_scene(api: &Api, auto: &mut Autosave<SceneDocument>, doc: &SceneDocument, now: Instant) {
    let problems = doc.check_new();
    if !problems.is_empty() {
        return auto.refuse(problems.join("; "));
    }
    let Ok(content) = serde_json::to_value(doc) else { return };
    let (put, sent) = (api.clone(), doc.clone());
    auto.send_kept(now, &api.outbox, "scene", &doc.id, content, move |body, base| {
        let answer = put.save_scene(&sent.id, body, base, "deck editor");
        (sent, answer)
    });
}

impl Host {
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// the outbox's shows open as their local copies, its scenes wait in `unsent`; each goes up on its base
    pub fn adopt_kept(&mut self) {
        for e in self.api.outbox.all() {
            match e.kind.as_str() {
                "show" => {
                    let Ok(show) = deck_show::document::open(&e.body.to_string()) else { continue };
                    if self.sub.conductor().show().id == e.id {
                        self.sub = Subtree::new(show, ClockChoice::Parent);
                        self.status = format!("{}: the local copy, not on the server yet", e.id);
                    } else {
                        self.library.insert(e.id.clone(), show);
                    }
                    self.saves.insert(e.id.clone(), Autosave::kept(&e));
                }
                "scene" => {
                    if let Ok(doc) = SceneDocument::from_value(e.body.clone()) {
                        self.unsent.insert(e.id.clone(), (doc, Autosave::kept(&e)));
                    }
                }
                _ => {}
            }
        }
    }

    /// a scene left with something unsent waits in `unsent`
    pub fn keep_unsent(&mut self, doc: SceneDocument, auto: Autosave<SceneDocument>) {
        if !auto.settled() && !doc.id.is_empty() {
            self.unsent.insert(doc.id.clone(), (doc, auto));
        }
    }

    /// scene `id` opened: its unsent copy and its save, when one waits
    pub fn take_unsent(&mut self, id: &str) -> Option<(SceneDocument, Autosave<SceneDocument>)> {
        self.unsent.remove(id)
    }

    fn unsent_autosave(&mut self, leave: bool, now: Instant) {
        for (doc, a) in self.unsent.values_mut() {
            if let Some((_, Err(e @ (Refusal::Conflict { .. } | Refusal::Other(_))))) = a.poll() {
                self.status = format!("not saved: {e}");
            }
            if a.due(now) || (leave && a.leave()) {
                send_scene(&self.api, a, doc, now);
            }
        }
        self.unsent.retain(|_, (_, a)| !a.settled());
    }
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
        let (id, print) = (shown.id.clone(), fingerprint(shown));
        self.saves.entry(id).or_default().look(print, now);
        let due: Vec<String> = self.saves.iter().filter(|(_, a)| a.due(now) || (leave && a.leave())).map(|(id, _)| id.clone()).collect();
        for id in due {
            self.send_show(&id, now);
        }
        self.compose_autosave(leave, now);
        self.unsent_autosave(leave, now);
    }

    fn send_show(&mut self, id: &str, now: Instant) {
        let Some(doc) = self.document(id).and_then(|s| serde_json::to_value(s).ok()) else { return };
        let (api, key) = (self.api.clone(), id.to_string());
        if let Some(a) = self.saves.get_mut(id) {
            a.send_kept(now, &self.api.outbox, "show", id, doc, move |body, base| {
                let answer = api.save(&key, body, base);
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
        send_scene(&self.api, &mut self.compose.auto, &doc, now);
    }

    /// the sheet leaves its scene: what it has not sent waits in `unsent`
    pub(super) fn compose_leave(&mut self) {
        if self.compose.id.is_empty() {
            return;
        }
        let (doc, auto) = (self.compose_document(), std::mem::take(&mut self.compose.auto));
        self.keep_unsent(doc, auto);
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
        for (_, a) in self.unsent.values_mut() {
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
    use crate::autosave::{tests::Fake, Outbox, IDLE, RETRY};
    use std::time::Duration;

    fn host(base: &str) -> Host {
        let mut host = Host::new(None, None, None, base.to_string());
        host.api = super::super::api::Api::with_cache(base.to_string(), std::env::temp_dir().join(format!("deck-editor-show-saves-{}", std::process::id())));
        host
    }

    /// an editor process on `outbox`: what the outbox kept is taken up as it starts
    fn started(base: &str, outbox: &Outbox) -> Host {
        let mut host = host(base);
        host.api.outbox = outbox.clone();
        host.adopt_kept();
        host
    }

    fn outbox(name: &str) -> Outbox {
        let dir = std::env::temp_dir().join(format!("deck-editor-outbox-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Outbox::at(&dir)
    }

    const OFFLINE: &str = "http://127.0.0.1:9/api";

    /// offline, the show's change goes into the outbox; the process ends; the next one shows the local copy and, the
    /// API back, sends exactly one PUT with it, and the outbox is empty
    #[test]
    fn an_unsent_show_survives_a_restart() {
        let ob = outbox("show");
        let t0 = Instant::now();
        {
            let mut host = started(OFFLINE, &ob);
            let id = host.shown().id.clone();
            host.autosave(false, t0);
            rename(&mut host, "kept");
            host.autosave(false, t0);
            host.autosave(true, t0 + Duration::from_millis(10));
            wait(&mut host, &id);
            assert_eq!(host.save_pill(), "offline – kept locally");
        }
        assert_eq!(ob.all().len(), 1, "on disk when the process ends");
        let fake = Fake::start(&[]);
        let mut host = started(&fake.base, &ob);
        assert_eq!((host.shown().outlets[0].id.as_str(), host.save_pill()), ("kept", "unsent – kept locally".to_string()), "the local copy opens");
        let id = host.shown().id.clone();
        host.autosave(false, Instant::now());
        wait(&mut host, &id);
        for _ in 0..3 {
            host.autosave(false, Instant::now() + RETRY * 2);
        }
        let puts = fake.puts.lock().unwrap().clone();
        assert_eq!((puts.len(), puts[0].0.as_str(), &puts[0].1["content"]["outlets"][0]["id"]), (1, "/api/shows/new-show", &serde_json::json!("kept")));
        assert!(ob.all().is_empty() && host.save_pill() == "saved · v101");
    }

    /// saved on v101, then a change offline; after the restart the server has v901: kept and named, not overwritten,
    /// and the start after that sends nothing
    #[test]
    fn a_conflict_after_a_restart_is_kept_and_named() {
        let ob = outbox("conflict");
        let first = Fake::start(&[]);
        {
            let mut host = started(&first.base, &ob);
            let id = host.shown().id.clone();
            let t0 = Instant::now();
            host.autosave(false, t0);
            rename(&mut host, "one");
            host.autosave(false, t0);
            host.autosave(true, t0);
            wait(&mut host, &id);
            host.autosave(false, t0);
            assert_eq!((host.save_pill(), ob.all().len()), ("saved · v101".to_string(), 0));
            host.api = host.api.with_base(OFFLINE);
            rename(&mut host, "two");
            host.autosave(false, t0);
            host.autosave(true, t0);
            wait(&mut host, &id);
        }
        let fake = Fake::start(&[409]);
        {
            let mut host = started(&fake.base, &ob);
            let id = host.shown().id.clone();
            for _ in 0..500 {
                host.autosave(false, Instant::now());
                if fake.count() == 1 && !host.saves[&id].busy() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(fake.puts.lock().unwrap()[0].1["base"], 101, "sent on the version it was made on");
            assert_eq!(host.save_pill(), "conflict – v901 on the server, yours (on v101) kept locally");
            assert!(host.status.contains("someone saved a newer version (v901)"), "{}", host.status);
        }
        let kept = ob.all();
        assert_eq!((kept.len(), kept[0].conflict, kept[0].theirs, &kept[0].body["outlets"][0]["id"]), (1, true, Some(901), &serde_json::json!("two")));
        let mut host = started(&fake.base, &ob);
        assert_eq!(host.shown().outlets[0].id, "two", "yours stays open");
        host.autosave(true, Instant::now() + RETRY * 3);
        assert_eq!((fake.count(), host.save_pill()), (1, "conflict – v901 on the server, yours (on v101) kept locally".to_string()), "nothing sent over theirs");
    }

    /// the sheet's scene offline: kept; the next process sends it once from `unsent` as the API answers
    #[test]
    fn an_unsent_sheet_scene_survives_a_restart() {
        let ob = outbox("sheet");
        {
            let mut host = started(OFFLINE, &ob);
            host.toon = Some("catpope".into());
            (host.compose.id, host.compose.name, host.compose.cast) = ("duet".into(), "Duet".into(), vec!["character:catpope".into()]);
            host.compose_save_waiting();
            assert_eq!(host.compose.auto.pill(), "offline – kept locally", "{}", host.status);
            assert!(host.status.contains("127.0.0.1:9"), "{}", host.status);
        }
        assert_eq!(ob.get("scene", "duet").map(|e| e.base), Some(None));
        let fake = Fake::start(&[]);
        let mut host = started(&fake.base, &ob);
        assert_eq!(host.unsent.get("duet").map(|(_, a)| a.pill()), Some("unsent – kept locally".to_string()));
        for _ in 0..500 {
            host.autosave(false, Instant::now());
            if host.unsent.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        host.autosave(true, Instant::now() + RETRY * 2);
        let puts = fake.puts.lock().unwrap().clone();
        assert_eq!((puts.len(), puts[0].0.as_str(), &puts[0].1["content"]["stage"]["name"]), (1, "/api/scenes/duet", &serde_json::json!("Duet")));
        assert!(ob.all().is_empty() && host.unsent.is_empty());
    }

    /// Build's scene saved offline on v3: kept; opened again in the next process it is the local copy with its save,
    /// and that save sends it once on v3
    #[test]
    fn an_unsent_build_scene_survives_a_restart_and_opens_as_the_local_copy() {
        let ob = outbox("build-scene");
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        {
            let host = started(OFFLINE, &ob);
            let mut auto: Autosave<SceneDocument> = Autosave::new(Some(3));
            auto.edit(Instant::now());
            send_scene(host.api(), &mut auto, &doc, Instant::now());
            crate::autosave::tests::settle(&mut auto);
            assert_eq!(auto.pill(), "offline – kept locally");
        }
        let mut host = started(OFFLINE, &ob);
        let open = host.scene_for_build("scene:duet").expect("the local copy");
        assert_eq!((open.doc.id.as_str(), open.base, open.note.as_deref()), ("duet", Some(3), Some("unsent – kept locally")));
        let mut auto = open.save.expect("its save comes along");
        assert!(host.unsent.is_empty(), "Build sends it now, not `unsent`");
        let fake = Fake::start(&[]);
        host.api = host.api.with_base(&fake.base);
        assert!(auto.due(Instant::now()));
        send_scene(host.api(), &mut auto, &open.doc, Instant::now());
        crate::autosave::tests::settle(&mut auto);
        let puts = fake.puts.lock().unwrap().clone();
        assert_eq!((puts.len(), &puts[0].1["base"]), (1, &serde_json::json!(3)));
        assert!(ob.all().is_empty());
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
        assert_eq!(host.save_pill(), "not saved yet", "the empty show is on no version of the server");
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
