//! What a video frame costs to reach the GPU, before and after the planes went up as they are:
//!   cargo run --release -p deck-space --example video-upload -- CLIP.mp4 [FRAMES]
//! before: the CPU's YUV → RGBA, the copy the old upload made of an RGBA picture, RGBA written into the texture;
//! after: the three planes written, the conversion pass on the GPU. Each frame waits for the GPU to finish.
use anyhow::{Context, Result};
use deck_media::{Clip, Format, Frames, Player, Software};
use deck_space::video::{upload, Converter};
use std::time::Instant;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().context("a clip")?;
    let count: usize = args.next().map_or(Ok(60), |n| n.parse())?;
    let clip = Clip::open(std::fs::read(&path)?.into()).map_err(|e| anyhow::anyhow!(e.0))?;
    let track = clip.video.clone().context("no video track")?;
    let mut player = Player::new(&clip, Box::new(Software::default())).map_err(|e| anyhow::anyhow!(e.0))?;
    let mut queue = Frames::new(4);
    let mut pictures = Vec::new();
    for s in track.samples.iter().take(count) {
        let f = player.frame_for(track.seconds(s.pts), &mut queue).map_err(|e| anyhow::anyhow!(e.0))?.context("a frame")?;
        pictures.push(f.picture.clone());
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).context("a GPU adapter")?;
    let info = adapter.get_info();
    let (device, gpu) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("video-upload"), ..Default::default() }))?;
    let mut conv = Converter::new(&device);
    let (w, h) = (pictures[0].width, pictures[0].height);
    println!("{path}: {} frames {w} × {h} {:?} on {} ({:?})", pictures.len(), pictures[0].format, info.name, info.backend);
    let wait = || device.poll(wgpu::PollType::wait_indefinitely()).map(|_| ());
    let mut run = |label: &str, planes: bool| -> Result<()> {
        let mut slot = None;
        let (mut cpu, mut total, mut bytes) = (0.0, 0.0, 0usize);
        // a first round makes the textures and pipelines; the second is measured
        for round in 0..2 {
            for pic in &pictures {
                let t = Instant::now();
                let rgba;
                let shown = if planes {
                    &**pic
                } else {
                    rgba = pic.to_rgba().to_rgba();
                    &rgba
                };
                upload(&device, &gpu, &mut conv, shown, false, None, &mut slot);
                let c = t.elapsed().as_secs_f64();
                wait()?;
                if round == 1 {
                    (cpu, total, bytes) = (cpu + c, total + t.elapsed().as_secs_f64(), bytes + shown.data.len());
                }
            }
        }
        let n = pictures.len() as f64;
        println!("{label}: {:.2} ms CPU, {:.2} ms until the GPU is done, {:.2} MB written per frame", cpu / n * 1e3, total / n * 1e3, bytes as f64 / n / 1e6);
        Ok(())
    };
    run("before (CPU YUV → RGBA, copied, RGBA up)", false)?;
    if matches!(pictures[0].format, Format::Yuv420 { .. }) {
        run("after (planes up, converted on the GPU)", true)?;
    }
    Ok(())
}
