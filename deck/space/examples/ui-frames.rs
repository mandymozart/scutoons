//! #485: frames of the deck's own UI drawn by the real path – deck-ui's menu and deck-kit's panel as layers of the
//! kit's egui screen, egui-wgpu in the stage's pass over the idle scene – offscreen through wgpu (no window), one PNG
//! a frame. A check of the path, not a design loop (RULES H8).
//!   cargo run -p deck-space --example ui-frames -- <out dir>
use anyhow::{Context, Result};
use deck_core::tempo::Now;
use deck_kit::base::draw::DrawList;
use deck_kit::base::screen::Layer;
use deck_kit::live::panel::{self, Part};
use deck_space::graph::Graph;
use deck_space::scene::{Scene, IDLE_SCENE};
use deck_space::stage::{Over, Stage, Tick};
use deck_ui::menu::{Facts, Input, Menu, Page};
use std::path::Path;

/// the deck's surface format: not sRGB (the shaders write display values)
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DECK: (u32, u32) = (1920, 1080);
const PHONE: (u32, u32) = (780, 1688);

fn main() -> Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    std::fs::create_dir_all(&out)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).context("a GPU adapter")?;
    let info = adapter.get_info();
    println!("{} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ui-frames"),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))?;
    let gpu = (&device, &queue);
    let deck = Facts {
        native: true,
        live_page: true,
        scenes_page: true,
        settings_page: true,
        density: 1.0,
        session: Some("output C7 · tenor: anna @ CHOIR".into()),
        who: "tenor: anna".into(),
        scene: "fireflies".into(),
        ..Facts::default()
    };
    // 1 · the main menu over the scene
    let mut m = Menu::new(true);
    m.input(Input::Open, &deck);
    frame(gpu, DECK, menu(&mut m, &deck, DECK), None, &Path::new(&out).join("1-main-menu.png"))?;
    // 2 · pair this deck, waiting for a member: the code and its QR
    let waiting = Facts { pair: deck_space::pairing::waiting("Q5HRT2", "https://scutoons.com"), ..deck.clone() };
    m.direct(Page::Pair);
    frame(gpu, DECK, menu(&mut m, &waiting, DECK), None, &Path::new(&out).join("2-pair.png"))?;
    // 3 · a phone: a finger's rows, the value under the label
    let phone = Facts { native: false, scenes_page: false, settings_page: false, density: 2.0, top: 48.0, ..deck.clone() };
    let mut m = Menu::new(true);
    m.input(Input::Open, &phone);
    frame(gpu, PHONE, menu(&mut m, &phone, PHONE), None, &Path::new(&out).join("3-phone-menu.png"))?;
    // 4 · the panel of a flash's suggestion (#481) along the bottom, its countdown running
    let parts = [
        Part::Caption("Flash · Vorschlag", "von ana"),
        Part::Title("top · Version 3"),
        Part::Line("Du spielst deine eigene Version (top · 2). Sie bleibt behalten.", true),
        Part::Buttons(&["Annehmen", "Ablehnen"], Some(0), Some(0.4)),
    ];
    frame(gpu, DECK, None, Some(panel_layer(DECK, &parts)), &Path::new(&out).join("4-panel.png"))?;
    frame(gpu, PHONE, None, Some(panel_layer(PHONE, &parts)), &Path::new(&out).join("5-phone-panel.png"))?;
    Ok(())
}

fn menu(m: &mut Menu, f: &Facts, (w, h): (u32, u32)) -> Option<Layer> {
    m.paint(f, (w as f32, h as f32)).flatten()
}

/// the panel as deck-show's face lays it out: the reader's scale, the mono face's advance
fn panel_layer((w, h): (u32, u32), parts: &[Part]) -> Layer {
    let scale = panel::scale(w as f32, h as f32);
    let mut d = DrawList::new(deck_outlet::text::mono_per_px());
    panel::draw(&mut d, &deck_kit::live::tokens::panel(), (w as f32 / scale, h as f32 / scale), parts, false);
    panel::layer(d, scale)
}

/// One frame of the stage (the idle scene, the layers over it) into a texture, read back, saved.
fn frame((device, queue): (&wgpu::Device, &wgpu::Queue), (w, h): (u32, u32), menu: Option<Layer>, panel: Option<Layer>, path: &Path) -> Result<()> {
    let graph = Graph::build(&Scene::parse(IDLE_SCENE)?)?;
    let mut stage = pollster::block_on(Stage::new(device, queue, FORMAT, (w, h), 1.0, 1.0, false))?;
    pollster::block_on(stage.load(device, FORMAT, &graph, Vec::new()))?;
    stage.set_menu(menu);
    stage.set_panel(panel);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let input = |_: usize, _: &str, d: f32| d;
    let now = Now::default();
    let mut enc = device.create_command_encoder(&Default::default());
    let tick = Tick { graph: &graph, input: &input, tempo: &now, time: 4.0, dt: 1.0 / 60.0 };
    stage.frame(device, queue, &mut enc, &view, &tick, &Over { hud: None, splash: (0.0, false), label_centred: false, hud_at: None }, None);
    let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("read back"), size: (row * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = slice.get_mapped_range()?;
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for p in data[(y * row) as usize..(y * row + w * 4) as usize].chunks_exact(4) {
            px.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
    }
    image::save_buffer(path, &px, w, h, image::ExtendedColorType::Rgba8)?;
    println!("{}", path.display());
    Ok(())
}
