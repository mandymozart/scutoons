//! What a video frame costs from packet to the GPU, per decode backend – decoding (and the copy back from a hardware
//! decoder), then the planes up and the conversion pass – and whether decoding keeps the clip's own frame rate:
//!   cargo run --release -p deck-space --example video-decode -- CLIP [FRAMES]
//! Backends: deck-media's software decoder (AV1, HAP); on Windows Media Foundation on the GPU (DXVA) and, to compare,
//! the same decoder on the CPU; on Linux VA-API. Each frame waits for the GPU to finish, so the upload is measured apart from decoding.
use anyhow::{Context, Result};
use deck_media::{Clip, DecodeBackend, Frames, Player, Software};
use deck_space::video::{upload, Converter};
use std::time::Instant;

/// makes a fresh backend of one kind
type Make = fn() -> Box<dyn DecodeBackend>;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().context("a clip")?;
    let count: usize = args.next().map_or(Ok(300), |n| n.parse())?;
    let clip = Clip::open(std::fs::read(&path)?.into()).map_err(|e| anyhow::anyhow!(e.0))?;
    let track = clip.video.clone().context("no video track")?;
    let n = count.min(track.samples.len());
    let fps = (n - 1).max(1) as f64 / track.seconds(track.samples[n - 1].pts).max(1e-3);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).context("a GPU adapter")?;
    let info = adapter.get_info();
    let (device, gpu) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("video-decode"), ..Default::default() }))?;
    let mut conv = Converter::new(&device);
    println!("{path}: {:?} {} × {}, {n} frames at {fps:.1} fps, on {} ({:?})", track.codec, track.width, track.height, info.name, info.backend);
    let mut backends: Vec<(&str, Make)> = vec![("software (deck-media)", || Box::new(Software::default()))];
    #[cfg(windows)]
    {
        use deck_media_hw::mf::MediaFoundation;
        println!("Media Foundation decodes on this GPU: {:?}", deck_media_hw::mf::codecs());
        backends.push(("hardware (Media Foundation, DXVA)", || Box::new(MediaFoundation::new())));
        backends.push(("Media Foundation on the CPU", || Box::new(MediaFoundation::software())));
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        println!("VA-API decodes here: {:?}", deck_media_hw::vaapi::codecs());
        backends.push(("hardware (VA-API)", || Box::new(deck_media_hw::vaapi::Vaapi::new())));
    }
    for (label, make) in backends {
        let mut player = match Player::new(&clip, make()) {
            Ok(p) => p,
            Err(e) => {
                println!("{label}: {}", e.0);
                continue;
            }
        };
        player.looping = false;
        let mut queue = Frames::new(8);
        let mut slot = None;
        #[cfg(windows)]
        let copied = deck_media_hw::mf::copied();
        let (mut decode, mut up, mut gpu_done, mut worst) = (0.0, 0.0, 0.0, 0.0f64);
        let mut shown = 0;
        let start = Instant::now();
        for s in track.samples.iter().take(n) {
            let t = Instant::now();
            let Some(f) = player.frame_for(track.seconds(s.pts), &mut queue).map_err(|e| anyhow::anyhow!(e.0))? else { continue };
            let pic = f.picture.clone();
            let d = t.elapsed().as_secs_f64();
            let t = Instant::now();
            upload(&device, &gpu, &mut conv, &pic, false, None, &mut slot);
            let u = t.elapsed().as_secs_f64();
            device.poll(wgpu::PollType::wait_indefinitely())?;
            let g = t.elapsed().as_secs_f64();
            // the first frame makes the decoder and the textures
            if shown > 0 {
                (decode, up, gpu_done, worst) = (decode + d, up + u, gpu_done + g, worst.max(d + g));
            }
            shown += 1;
        }
        let wall = start.elapsed().as_secs_f64();
        let m = (shown - 1).max(1) as f64;
        // a deck decodes on its own thread and uploads on the render thread: decoding alone must keep the frame rate
        let per = decode / m;
        println!("{label}: {shown} frames in {wall:.2} s");
        println!("  decode {:.2} ms, upload {:.2} ms CPU / {:.2} ms until the GPU is done, worst frame {:.2} ms", decode / m * 1e3, up / m * 1e3, gpu_done / m * 1e3, worst * 1e3);
        #[cfg(windows)]
        {
            let (c, t) = deck_media_hw::mf::copied();
            if c > copied.0 {
                println!("  of the decode: copying the NV12 picture back from the GPU {:.2} ms", (t - copied.1).as_secs_f64() / (c - copied.0) as f64 * 1e3);
            }
        }
        println!("  decoding {:.2} ms per frame against {:.2} ms at {fps:.0} fps: {:.1}× real time – {}", per * 1e3, 1e3 / fps, 1.0 / (per * fps), if per * fps < 1.0 { "keeps up" } else { "falls behind" });
    }
    Ok(())
}
