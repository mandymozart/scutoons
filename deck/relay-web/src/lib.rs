//! relay-web (#366): deck-space's relay client behind wasm-bindgen – the page hands in the socket's events and its clock
//! (ms since the epoch) and does what `take` says (dial, send, close; the status and what the app hears as JSON). Every
//! decision is relay.rs's; this crate only converts JSON at the edge. Natively empty.
#![cfg(target_arch = "wasm32")]
mod take;

use deck_space::relay::{Client, Options};
use deck_space::rig::Stream;
use wasm_bindgen::prelude::*;

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(err)
}

/// One relay client (deck-space `relay::Client`).
#[wasm_bindgen]
pub struct RelayClient {
    c: Client,
}

#[wasm_bindgen]
impl RelayClient {
    /// `options` (JSON): `{ relay, role, group?, nick?, token?, resume?, messages? }`
    #[wasm_bindgen(constructor)]
    pub fn new(options: &str) -> Result<RelayClient, JsValue> {
        let o: Options = parse(options)?;
        Ok(RelayClient { c: Client::new(o).map_err(err)? })
    }

    pub fn start(&mut self, now: f64) {
        self.c.start(now);
    }

    pub fn opened(&mut self, now: f64) {
        self.c.opened(now);
    }

    pub fn closed(&mut self, now: f64, why: &str) {
        self.c.closed(now, why);
    }

    pub fn text(&mut self, now: f64, data: &str) {
        self.c.text(now, data);
    }

    /// #438: the WebTransport session and its stream are up
    pub fn quic_opened(&mut self, now: f64) {
        self.c.quic_opened(now);
    }

    pub fn quic_closed(&mut self, now: f64, why: &str) {
        self.c.quic_closed(now, why);
    }

    pub fn quic_stream(&mut self, now: f64, bytes: &[u8]) {
        self.c.quic_stream(now, bytes);
    }

    pub fn quic_datagram(&mut self, now: f64, bytes: &[u8]) {
        self.c.quic_datagram(now, bytes);
    }

    pub fn tick(&mut self, now: f64) {
        self.c.tick(now);
    }

    /// the ms `tick` is due next; undefined: nothing to wait for
    pub fn next_wake(&self) -> Option<f64> {
        self.c.next_wake()
    }

    /// what to do and say since the last call (JSON: `[{ dial } | { send } | { close } | { status } | { joined } | …]`)
    pub fn take(&mut self) -> String {
        serde_json::to_string(&self.c.take()).unwrap_or_else(|_| "[]".into())
    }

    /// the status as the HUD's line
    pub fn line(&self) -> String {
        self.c.line()
    }

    pub fn client(&self) -> String {
        self.c.client().to_string()
    }

    pub fn joined(&self) -> bool {
        self.c.joined()
    }

    /// messages sent per kind (JSON)
    pub fn counts(&self) -> String {
        serde_json::to_string(self.c.counts()).unwrap_or_default()
    }

    pub fn stop(&mut self) {
        self.c.stop();
    }

    /// the deck's being (`deck.graph`, JSON)
    pub fn set_graph(&mut self, json: &str) -> Result<(), JsValue> {
        self.c.set_graph(parse(json)?);
        Ok(())
    }

    /// the paths the deck streams with how each moves (JSON: `[[path, kind]]`, the played scene's `streamed`)
    pub fn set_stream(&mut self, json: &str) -> Result<(), JsValue> {
        let streamed: Vec<(String, Stream)> = parse(json)?;
        self.c.set_stream(&streamed);
        Ok(())
    }

    /// the streamed paths, in the order `sample` reads them
    pub fn paths(&self) -> Vec<String> {
        self.c.stream_paths().to_vec()
    }

    /// one frame at `time` s: each path's value now (`paths`' order) and the mood ('' = none)
    pub fn sample(&mut self, time: f64, values: &[f64], mood: &str) {
        self.c.sample(time, values, (!mood.is_empty()).then_some(mood));
    }

    /// the render quality (`deck.quality`, JSON)
    pub fn quality(&mut self, json: &str) -> Result<(), JsValue> {
        self.c.quality(parse(json)?);
        Ok(())
    }

    /// the answer to the conductor's push (`deck.synced`, JSON)
    pub fn synced(&mut self, now: f64, json: &str) -> Result<(), JsValue> {
        self.c.synced(now, parse(json)?);
        Ok(())
    }

    /// a leading deck's anchor (`tempo`, JSON); false: not joined
    pub fn tempo(&mut self, json: &str) -> Result<bool, JsValue> {
        Ok(self.c.tempo(parse(json)?))
    }

    /// a ping now (the show clock asks); false: no socket
    pub fn ping(&mut self, now: f64) -> bool {
        self.c.ping(now)
    }

    pub fn vote_cast(&mut self, option: i32) -> bool {
        self.c.vote_cast(i64::from(option))
    }

    /// #469: the rung the joined link is on (`QUIC`, `WebSocket`); '' while not joined
    pub fn rung(&self) -> String {
        self.c.rung().unwrap_or_default().to_string()
    }

    /// #469: a phone's pose at `t` (its clock, ms): `p` 3 numbers, `q` 4; false: not joined
    pub fn hand_pose(&mut self, t: f64, p: &[f64], q: &[f64]) -> bool {
        self.c.hand_pose(t, p, q)
    }

    /// #469: what the phone tracks (`hand.state`, JSON)
    pub fn hand_state(&mut self, json: &str) -> Result<(), JsValue> {
        self.c.hand_state(parse(json)?);
        Ok(())
    }

    pub fn hand_record(&mut self, on: bool) -> bool {
        self.c.hand_record(on)
    }

    pub fn hand_recenter(&mut self) -> bool {
        self.c.hand_recenter()
    }

    /// #469: the editor's view for its phones (`hand.view`, JSON)
    pub fn hand_view(&mut self, json: &str) -> Result<(), JsValue> {
        self.c.hand_view(parse(json)?);
        Ok(())
    }

    /// #518: a story event the reader fired (the VNEvent, JSON) as a `story` message; false: dropped (#527: it does not
    /// drive the session's story now)
    pub fn story(&mut self, json: &str) -> Result<bool, JsValue> {
        Ok(self.c.story(parse(json)?))
    }

    /// #552: the reader's choices closed (picked, or none was shown) – a vote's result finds it at no choice now; a result
    /// it stands at comes out as `{ choice: { event, option, pick?, ignored? } }` (deck-space linked.rs `Choice`)
    pub fn choices_closed(&mut self) {
        self.c.choices_closed();
    }

    /// #527: this reader drives the session's story from the relay's answer on (the person said yes); false: no reader
    pub fn drive(&mut self) -> bool {
        self.c.drive()
    }

    /// #527: a reader's place in the story (JSON: `{ claim: none | asked | drives | taken_over, other }`)
    pub fn driving(&self) -> String {
        serde_json::to_string(&self.c.driving()).unwrap_or_default()
    }

    /// #527: a conductor's view of the linked readers (JSON: `{ linked: { id: name | null }, driver, last: { type,
    /// characters, from } }`)
    pub fn readers(&self) -> String {
        serde_json::to_string(self.c.readers()).unwrap_or_default()
    }

    /// #478: the page asks the API at `api` for this client's proof with the person's sign-in (`{ ask_proof: { url } }`
    /// says when; the answer to `proof`); the first hello waits for it. Call before `start`.
    pub fn ask_proof(&mut self, api: &str) {
        self.c.ask_proof(api);
    }

    /// #478: the API's answer to `ask_proof` (`status` 0: no answer)
    pub fn proof(&mut self, now: f64, status: u16, body: &str) {
        self.c.proof(now, status, body);
    }

    /// #478: the person signed in or out: the proof is asked again
    pub fn proof_again(&mut self, now: f64) {
        self.c.proof_again(now);
    }

    /// the person said yes to the live camera `camera` (or Take it back): this phone holds it from the relay's
    /// answer on, and takes it again by itself after a drop or a reload; false: no reader, or no `key` in its options
    pub fn camera_claim(&mut self, camera: &str) -> bool {
        self.c.camera_claim(camera)
    }

    /// after a reload: the page held `camera` as the client `was` (its browser kept both) – taken again by itself
    pub fn camera_resume(&mut self, camera: &str, was: &str) -> bool {
        self.c.camera_resume(camera, was)
    }

    /// Leave – the phone lets its camera go
    pub fn camera_leave(&mut self) -> bool {
        self.c.camera_leave()
    }

    /// a phone's hold (JSON: `{ hold: none | asked | holds | taken_over, camera?, other?, refused? }`)
    pub fn holding(&self) -> String {
        serde_json::to_string(&self.c.holding()).unwrap_or_default()
    }

    /// the session's cameras as the relay last showed them (JSON: protocol `LiveCamera`s, with who holds each)
    pub fn cameras(&self) -> String {
        serde_json::to_string(self.c.cameras()).unwrap_or_default()
    }

    /// a conductor's cameras (JSON: protocol `LiveCamera`s), sent again at every welcome
    pub fn set_cameras(&mut self, json: &str) -> Result<(), JsValue> {
        self.c.set_cameras(parse(json)?);
        Ok(())
    }

    /// a conductor forces the phone `client` onto `camera` ('' frees what it holds); false: not joined
    pub fn camera_force(&mut self, client: &str, camera: &str) -> bool {
        self.c.camera_force(client, camera)
    }

    /// a conductor flashes a camera's settings (JSON object) to the phone holding it; false: not joined
    pub fn camera_flash(&mut self, camera: &str, json: &str) -> Result<bool, JsValue> {
        Ok(self.c.camera_flash(camera, parse(json)?))
    }

    /// the vote open now as a screen hears it (an envelope, JSON); '' when none is open
    pub fn current_vote(&self) -> String {
        self.c.current_vote().and_then(|e| serde_json::to_string(&e).ok()).unwrap_or_default()
    }
}

/// #527: the address a linked reader opens – `<base>/<toon>/chapter/<n>?label=<label>&live=<code>&relay=<relay>`
/// (deck-space linked.rs `reader_link`; `label` '' = the chapter's start; `relay` the session's relay URL, named in the
/// link only when it is not the hosted one, #530), the same text the deck editor offers
/// the handheld page of one live camera – `<base>/live/cameras/<id>?live=<code>&relay=<relay>` (deck-space
/// live_cameras.rs `camera_link`; `relay` named only when it is not the hosted one); `camera` '' = the list of cameras
#[wasm_bindgen]
pub fn camera_link(base: &str, camera: &str, code: &str, relay: &str) -> String {
    if camera.is_empty() {
        deck_space::live_cameras::cameras_link(base, code, Some(relay))
    } else {
        deck_space::live_cameras::camera_link(base, camera, code, Some(relay))
    }
}

#[wasm_bindgen]
pub fn reader_link(base: &str, toon: &str, chapter: u32, label: &str, code: &str, relay: &str) -> String {
    deck_space::linked::reader_link(base, toon, chapter, Some(label), code, Some(relay))
}
