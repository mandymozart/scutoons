//! A scene in a browser canvas (#463): the renderer every deck draws with (deck-space `stage`) on the page's GPU, the
//! scene's picture nodes as the native deck plays them, the splash and the build label over them; the HUD is off. No scene given = the deck's idle scene, as on the native deck.
//! Not yet: the nodes that need a loader of their own here (a chapter's slices, an outlet's planes, a space's set) are
//! refused by name, and a knob reads the node's own value (no control-domain links in the browser's picture yet).
use crate::{err, open_canvas, CanvasGpu};
use deck_space::graph::{Graph, Kind};
use deck_space::quality::MAX_SIDE;
use deck_space::scene::{Scene, IDLE_SCENE};
use deck_space::splash::Fade;
use deck_space::stage::{Over, Stage, Tick};
use deck_space::tempo;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SceneView {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    canvas: web_sys::HtmlCanvasElement,
    graph: Graph,
    stage: Stage,
    beat: tempo::Now,
    fade: Fade,
    backend: String,
}

#[wasm_bindgen]
impl SceneView {
    /// `scene` (a scene document, "" = the idle scene) on `canvas` (its size in device pixels is the canvas' width /
    /// height). `prefer` as `SpaceView::create`.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, scene: String, prefer: String) -> Result<SceneView, JsValue> {
        let scene = Scene::parse(if scene.is_empty() { IDLE_SCENE } else { &scene }).map_err(err)?;
        let graph = Graph::build(&scene).map_err(err)?;
        if let Some(n) = graph.nodes.iter().find(|n| matches!(n.kind, Kind::Chapter | Kind::Outlet | Kind::Space)) {
            return Err(err(format!("node '{}': the browser does not load this kind of node in a scene yet", n.id)));
        }
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
        };
        surface.configure(&device, &config);
        let scale = scene.stage.render_scale;
        let mut stage = Stage::new(&device, &queue, format, (w, h), scale, scale, false).await.map_err(err)?;
        stage.load(&device, format, &graph, Vec::new()).await.map_err(err)?;
        Ok(SceneView { surface, device, queue, config, canvas: element, graph, stage, beat: tempo::Now::default(), fade: Fade { alpha: 1.0 }, backend: backend.into() })
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
        self.stage.resize(&self.device, (w, h));
    }

    /// One frame: `dt` since the last (s), `time` the page's clock (s).
    pub fn frame(&mut self, dt: f32, time: f32) -> Result<(), JsValue> {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(err(e)),
        };
        let graph = &self.graph;
        let input = |n: usize, p: &str, d: f32| graph.nodes[n].f32(p, d);
        let mut enc = self.device.create_command_encoder(&Default::default());
        let view = frame.texture.create_view(&Default::default());
        self.fade.step(false, dt);
        self.stage.frame(&self.device, &self.queue, &mut enc, &view, &Tick { graph, input: &input, tempo: &self.beat, time, dt }, &Over { hud: None, splash: (self.fade.alpha, false), label_centred: true }, None);
        self.queue.submit([enc.finish()]);
        frame.present();
        Ok(())
    }
}
