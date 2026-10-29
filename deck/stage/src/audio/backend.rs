//! Where the engine plays: PipeWire (Linux, feature `pipewire` – the deck), cpal (every OS; on Linux through ALSA), or
//! the null backend (a timer thread, no device – the chosen fallback when there is no audio device, CYBERDECKS.md §2).
//! `auto` tries them in that order and logs why one was skipped. Each one stamps its blocks with the moment they are
//! heard (`Engine::mark`, the time base of clock.rs): PipeWire from its cycle timing (pw_time: now + the delay to the
//! device + what the resampler holds), cpal from the stream's playback instant, the null backend from its schedule.
use super::Engine;
use crate::clock::now_ns;
use anyhow::{bail, Result};
use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Auto,
    PipeWire,
    Cpal,
    Null,
}

impl Choice {
    pub fn parse(s: &str) -> Option<Choice> {
        Some(match s {
            "auto" => Choice::Auto,
            "pipewire" => Choice::PipeWire,
            "cpal" => Choice::Cpal,
            "null" | "none" => Choice::Null,
            _ => return None,
        })
    }
}

/// A playing backend; dropping it stops the audio.
pub struct Running {
    pub name: &'static str,
    pub rate: u32,
    _keep: Box<dyn Any>,
}

/// Builds an engine for a sample rate; called once per backend tried.
pub type Factory<'a> = dyn FnMut(f32) -> Result<Engine> + 'a;

pub fn start(choice: Choice, factory: &mut Factory) -> Result<Running> {
    let order: &[Choice] = match choice {
        Choice::Auto => &[Choice::PipeWire, Choice::Cpal, Choice::Null],
        Choice::PipeWire => &[Choice::PipeWire],
        Choice::Cpal => &[Choice::Cpal],
        Choice::Null => &[Choice::Null],
    };
    let mut last = None;
    for &c in order {
        let r = match c {
            Choice::PipeWire => pipewire(factory),
            Choice::Cpal => cpal(factory),
            _ => null(factory),
        };
        match r {
            Ok(run) => return Ok(run),
            Err(e) => {
                tracing::warn!("audio: {c:?} not available ({e:#}) – trying the next backend");
                last = Some(e);
            }
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("no audio backend")))
}

const NULL_RATE: u32 = 48_000;
const NULL_BLOCK: usize = 512;

/// no device: the engine runs on a timer thread so the clock, the music and the FFT still move
fn null(factory: &mut Factory) -> Result<Running> {
    let mut engine = factory(NULL_RATE as f32)?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new().name("audio-null".into()).spawn(move || {
        let mut buf = vec![0.0f32; NULL_BLOCK * 2];
        let period = Duration::from_secs_f64(NULL_BLOCK as f64 / NULL_RATE as f64);
        let mut next = std::time::Instant::now();
        // block k is "heard" on its schedule (no device, no latency)
        let (t0, mut k) = (now_ns(), 0u64);
        while !flag.load(Ordering::Relaxed) {
            engine.mark(t0 + k * NULL_BLOCK as u64 * 1_000_000_000 / NULL_RATE as u64);
            k += 1;
            engine.render(&mut buf, 2, |x| x);
            next += period;
            if let Some(wait) = next.checked_duration_since(std::time::Instant::now()) {
                std::thread::sleep(wait);
            }
        }
    })?;
    struct Stop(Arc<AtomicBool>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    Ok(Running { name: "null", rate: NULL_RATE, _keep: Box::new(Stop(stop)) })
}

fn cpal(factory: &mut Factory) -> Result<Running> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::SampleFormat;
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else { bail!("no default output device") };
    let cfg = device.default_output_config()?;
    let (rate, channels) = (cfg.sample_rate().0, cfg.channels() as usize);
    let mut engine = factory(rate as f32)?;
    let sc: cpal::StreamConfig = cfg.config();
    let err = |e| tracing::error!("audio stream: {e}");
    // the block is heard at its playback instant: now + (playback − callback), the output latency cpal reports
    let heard = |info: &cpal::OutputCallbackInfo| {
        let ts = info.timestamp();
        now_ns() + ts.playback.duration_since(&ts.callback).map(|d| d.as_nanos() as u64).unwrap_or(0)
    };
    let stream = match cfg.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &sc,
            move |d: &mut [f32], info| {
                engine.mark(heard(info));
                engine.render(d, channels, |x| x)
            },
            err,
            None,
        )?,
        SampleFormat::I16 => device.build_output_stream(
            &sc,
            move |d: &mut [i16], info| {
                engine.mark(heard(info));
                engine.render(d, channels, |x| (x * i16::MAX as f32) as i16)
            },
            err,
            None,
        )?,
        f => bail!("output sample format {f:?} (f32 and i16 are supported)"),
    };
    stream.play()?;
    tracing::info!("audio: cpal {} on '{}', {rate} Hz, {channels} ch", host.id().name(), device.name().unwrap_or_default());
    Ok(Running { name: "cpal", rate, _keep: Box::new(stream) })
}

#[cfg(not(all(target_os = "linux", feature = "pipewire")))]
fn pipewire(_: &mut Factory) -> Result<Running> {
    bail!("built without the pipewire feature")
}

/// PipeWire: an output stream at 48 kHz / f32 / stereo with a 128-frame latency request (the deck's quantum,
/// rootfs/etc/pipewire), processed on PipeWire's realtime data thread (RT_PROCESS).
#[cfg(all(target_os = "linux", feature = "pipewire"))]
fn pipewire(factory: &mut Factory) -> Result<Running> {
    use pipewire as pw;
    use pw::spa;
    const RATE: u32 = 48_000;
    const CHANNELS: usize = 2;
    let mut engine = factory(RATE as f32)?;
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::Builder::new().name("pipewire".into()).spawn(move || {
        let run = || -> Result<(), pw::Error> {
            pw::init();
            let mainloop = pw::main_loop::MainLoopRc::new(None)?;
            let context = pw::context::ContextRc::new(&mainloop, None)?;
            let core = context.connect_rc(None)?;
            let stream = pw::stream::StreamBox::new(
                &core,
                "deck-stage",
                pw::properties::properties! {
                    *pw::keys::MEDIA_TYPE => "Audio",
                    *pw::keys::MEDIA_ROLE => "Music",
                    *pw::keys::MEDIA_CATEGORY => "Playback",
                    *pw::keys::NODE_NAME => "deck-stage",
                    *pw::keys::APP_NAME => "SCUTOONS deck stage",
                    *pw::keys::NODE_LATENCY => "128/48000",
                    *pw::keys::AUDIO_CHANNELS => "2",
                },
            )?;
            let _listener = stream
                .add_local_listener_with_user_data(())
                .process(move |stream, _| {
                    let Some(mut buffer) = stream.dequeue_buffer() else { return };
                    let datas = buffer.datas_mut();
                    let Some(data) = datas.first_mut() else { return };
                    // pw_time (RT-safe): `now` = this cycle on CLOCK_MONOTONIC, `delay` = until the device plays the
                    // next sample (ticks of `rate`), `buffered` = frames in the resampler; no timing yet → the quantum
                    let heard = match stream.time() {
                        Ok(t) if t.rate().denom > 0 && t.now() > 0 => {
                            let r = t.rate();
                            let delay = t.delay().max(0) as f64 * r.num as f64 / r.denom as f64;
                            let buffered = t.buffered() as f64 / RATE as f64;
                            t.now() as u64 + ((delay + buffered) * 1e9) as u64
                        }
                        _ => now_ns() + 128 * 1_000_000_000 / RATE as u64,
                    };
                    engine.mark(heard);
                    let stride = std::mem::size_of::<f32>() * CHANNELS;
                    let frames = match data.data() {
                        Some(bytes) => match bytemuck::try_cast_slice_mut::<u8, f32>(bytes) {
                            Ok(samples) => {
                                let n = samples.len() / CHANNELS;
                                engine.render(&mut samples[..n * CHANNELS], CHANNELS, |x| x);
                                n
                            }
                            Err(_) => 0,
                        },
                        None => 0,
                    };
                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = stride as _;
                    *chunk.size_mut() = (stride * frames) as _;
                })
                .register()?;
            let mut info = spa::param::audio::AudioInfoRaw::new();
            info.set_format(spa::param::audio::AudioFormat::F32LE);
            info.set_rate(RATE);
            info.set_channels(CHANNELS as u32);
            let mut position = [0; spa::param::audio::MAX_CHANNELS];
            position[0] = spa::sys::SPA_AUDIO_CHANNEL_FL;
            position[1] = spa::sys::SPA_AUDIO_CHANNEL_FR;
            info.set_position(position);
            let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
                std::io::Cursor::new(Vec::new()),
                &spa::pod::Value::Object(spa::pod::Object {
                    type_: spa::sys::SPA_TYPE_OBJECT_Format,
                    id: spa::sys::SPA_PARAM_EnumFormat,
                    properties: info.into(),
                }),
            )
            .map_err(|_| pw::Error::CreationFailed)?
            .0
            .into_inner();
            let mut params = [spa::pod::Pod::from_bytes(&values).ok_or(pw::Error::CreationFailed)?];
            stream.connect(
                spa::utils::Direction::Output,
                None,
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS,
                &mut params,
            )?;
            let _ = ready_tx.send(Ok(()));
            mainloop.run();
            Ok(())
        };
        if let Err(e) = run() {
            let _ = ready_tx.send(Err(e.to_string()));
        }
    })?;
    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => {
            tracing::info!("audio: PipeWire stream 'deck-stage', {RATE} Hz, latency 128/48000");
            Ok(Running { name: "pipewire", rate: RATE, _keep: Box::new(()) })
        }
        Ok(Err(e)) => bail!("{e}"),
        Err(_) => bail!("no answer from PipeWire within 3 s"),
    }
}
