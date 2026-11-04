//! relay-web (#366): deck-space's relay client behind wasm-bindgen – the page hands in the socket's events and its clock
//! (ms since the epoch) and does what `take` says (dial, send, close; the status and what the app hears as JSON). Every
//! decision is relay.rs's; this crate only converts JSON at the edge. Natively empty.
#![cfg(target_arch = "wasm32")]
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

    /// a conductor opens a vote (`vote.open`, JSON)
    pub fn vote_open(&mut self, json: &str) -> Result<bool, JsValue> {
        let p: protocol::VoteOpen = parse(json)?;
        Ok(self.c.vote_open(p))
    }

    pub fn vote_cast(&mut self, option: i32) -> bool {
        self.c.vote_cast(i64::from(option))
    }

    pub fn vote_close(&mut self, over: Option<i32>) -> bool {
        self.c.vote_close(over.map(i64::from))
    }

    pub fn vote_hold(&mut self, hold: bool) -> bool {
        self.c.vote_hold(hold)
    }

    /// the vote open now as a screen hears it (an envelope, JSON); '' when none is open
    pub fn current_vote(&self) -> String {
        self.c.current_vote().and_then(|e| serde_json::to_string(&e).ok()).unwrap_or_default()
    }
}
