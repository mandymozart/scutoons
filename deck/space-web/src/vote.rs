//! The phone's voting screen in the browser (#549, research/SHOW.md §15 point 3b): the audience start with the light
//! start. The decisions are the core's – deck-space `audience.rs` (the relay client as an audience phone of its group,
//! `device: phone`, its proof, the ballot and what shows between votes, the cast) drawn by deck-space's `stage::Light`
//! (deck-kit's 2D on a black canvas: no scene, no sound). This is only the browser's side: the canvas and its GPU, the
//! page's socket under the relay client (client.ts `RelaySocket` – the names are relay-web's), the proof the page fetches
//! with its reader's sign-in (`asks`, `proof`), the pointer and the keys passed on, and `awake` for the page's wake lock.
use crate::{err, open_canvas, CanvasGpu};
use deck_space::audience::Audience;
use deck_space::quality::MAX_SIDE;
use deck_space::stage::Light;
use deck_ui::menu::Input;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VoteView {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    canvas: web_sys::HtmlCanvasElement,
    light: Light,
    audience: Audience,
    backend: String,
    /// the page's clock (s), as of the last frame
    now: f64,
}

#[wasm_bindgen]
impl VoteView {
    /// The voting screen on `canvas` (its size in device pixels is the canvas' width / height), joining `relay` (ws:// or
    /// wss://…/live/<code>; "" = the hosted show) as an audience phone that came with `group` (the QR's `?group=`; "" =
    /// none), named after `reader` (the signed-in reader's name; "" = nobody), its words in `locale`, its proof from
    /// `api`. `prefer` as `SpaceView::create`.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, relay: String, group: String, reader: String, locale: String, api: String, prefer: String) -> Result<VoteView, JsValue> {
        let audience = Audience::new(&relay, &group, &reader, &locale, &api).map_err(err)?;
        let element = canvas.clone();
        let (w, h) = (canvas.width().clamp(1, MAX_SIDE), canvas.height().clamp(1, MAX_SIDE));
        let CanvasGpu { surface, device, queue, format, alpha, backend, .. } = open_canvas(canvas, &prefer).await?;
        element.set_width(w);
        element.set_height(h);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: alpha,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let light = Light::new(&device, format, (w, h));
        Ok(VoteView { surface, device, queue, config, canvas: element, light, audience, backend: backend.into(), now: 0.0 })
    }

    // the relay client, driven by the page's socket (client.ts `RelaySocket` – the names are relay-web's)
    pub fn start(&mut self, now: f64) {
        self.audience.relay.start(now);
    }

    pub fn opened(&mut self, now: f64) {
        self.audience.relay.opened(now);
    }

    pub fn closed(&mut self, now: f64, why: &str) {
        self.audience.relay.closed(now, why);
    }

    pub fn text(&mut self, now: f64, data: &str) {
        self.audience.relay.text(now, data);
    }

    pub fn quic_opened(&mut self, now: f64) {
        self.audience.relay.quic_opened(now);
    }

    pub fn quic_closed(&mut self, now: f64, why: &str) {
        self.audience.relay.quic_closed(now, why);
    }

    pub fn quic_stream(&mut self, now: f64, bytes: &[u8]) {
        self.audience.relay.quic_stream(now, bytes);
    }

    pub fn quic_datagram(&mut self, now: f64, bytes: &[u8]) {
        self.audience.relay.quic_datagram(now, bytes);
    }

    pub fn tick(&mut self, now: f64) {
        self.audience.relay.tick(now);
    }

    pub fn stop(&mut self) {
        self.audience.relay.stop();
    }

    pub fn next_wake(&self) -> Option<f64> {
        self.audience.relay.next_wake()
    }

    /// what the socket should do (JSON, as relay-web's `take`); a vote stays here (the ballot is drawn by the core)
    pub fn take(&mut self) -> String {
        serde_json::to_string(&self.audience.take(self.now)).unwrap_or_else(|_| "[]".into())
    }

    pub fn line(&self) -> String {
        self.audience.relay.line()
    }

    /// what the page fetches: key (`r:` the proof – with the reader's sign-in) and URL, in pairs
    pub fn asks(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        for (key, url) in self.audience.asks() {
            out.push(&JsValue::from_str(&key));
            out.push(&JsValue::from_str(&url));
        }
        out
    }

    /// the API's answer to the `r:` ask: its status (0 = no answer) and body
    pub fn proof(&mut self, status: u16, body: &str) {
        self.audience.proof(status, body, self.now);
    }

    /// the reader signed in or out: the proof is asked again at once
    pub fn proof_again(&mut self) {
        self.audience.proof_again();
    }

    /// The pointer at (x, y) of the canvas, in its pixels: `press` 0 = it moved, 1 = a press (a click, a tap). Whether
    /// the ballot took it.
    pub fn pointer(&mut self, x: f32, y: f32, press: u8) -> bool {
        let i = if press == 0 { Input::Point(x, y) } else { Input::Press(x, y) };
        self.light.ui_input(i);
        self.audience.input(i, js_sys::Date::now()) && press > 0
    }

    /// A key of the page (the DOM's `key`): the arrows and Enter are the ballot's. Whether it took it.
    pub fn key(&mut self, key: &str) -> bool {
        let i = match key {
            "ArrowUp" => Input::Up,
            "ArrowDown" => Input::Down,
            "Enter" => Input::Enter,
            _ => return false,
        };
        self.light.ui_input(i);
        self.audience.input(i, js_sys::Date::now())
    }

    /// a vote can come: the page keeps the screen on while this holds
    pub fn awake(&self) -> bool {
        self.audience.awake()
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// the display's new size in device pixels
    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.clamp(1, MAX_SIDE), h.clamp(1, MAX_SIDE));
        if (w, h) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.light.resize((w, h));
    }

    /// One frame at `time`, the page's clock (s): the ballot (or what shows between votes) when it changed, drawn.
    pub fn frame(&mut self, time: f64) -> Result<(), JsValue> {
        self.now = time.max(0.0);
        // the relay client's clock is the page's socket's: ms since the epoch (client.ts `now`), read here as `Date.now()`
        let size = (self.config.width as f32, self.config.height as f32);
        if let Some(layer) = self.audience.paint(js_sys::Date::now(), size, deck_outlet::text::mono_per_px()) {
            self.light.set_vote(layer);
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => return Err(err(format!("the canvas's surface: {other:?}"))),
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        let view = frame.texture.create_view(&Default::default());
        self.light.frame(&self.device, &self.queue, &mut enc, &view);
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        Ok(())
    }
}
