//! deck-engine behind raw wasm exports (no wasm-bindgen): the scene goes in through the inbox, `ew_run` renders it, the
//! samples come out through `ew_out`. The same function runs natively (`run`) – the conformance suite compares both.
use deck_core::tempo::Now;
use deck_engine::control::Controls;
use deck_engine::{Build, Engine};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use std::sync::Mutex;

/// The scene (TOML) rendered at `rate` for `frames` frames in blocks of `block`, the control domain stepped before each
/// block (the frame thread's `Controls::update`); the left channel after master and limiter. `Err` = the scene is refused.
pub fn run(toml: &str, rate: f32, block: usize, frames: usize, build: Build) -> Result<Vec<f32>, String> {
    let graph = Graph::build(&Scene::parse(toml).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let (mut engine, mut handles) = Engine::build_with(&graph, rate, 1.0, build).map_err(|e| format!("{e:#}"))?;
    let mut controls = Controls::new(&graph, Some(&mut handles));
    let now = Now::default();
    let mut out = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        controls.update(n as f32 / rate, 0, &now);
        engine.render(&mut buf[..n * 2], 2, |x| x);
        out.extend(buf[..n * 2].chunks_exact(2).map(|f| f[0]));
        done += n;
    }
    Ok(out)
}

static INBOX: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static OUT: Mutex<Vec<f32>> = Mutex::new(Vec::new());

/// Reserve `len` bytes for the scene and return where to write them.
#[no_mangle]
pub extern "C" fn ew_inbox(len: u32) -> *mut u8 {
    let mut b = INBOX.lock().unwrap();
    b.clear();
    b.resize(len as usize, 0);
    b.as_mut_ptr()
}

/// Render the scene in the inbox: returns the number of samples (0 = refused). `batch` 0 = the patcher build.
#[no_mangle]
pub extern "C" fn ew_run(rate: f32, block: u32, frames: u32, batch: u32) -> u32 {
    let text = String::from_utf8_lossy(&INBOX.lock().unwrap()).into_owned();
    let build = if batch == 0 { Build::patcher() } else { Build::detect() };
    match run(&text, rate, block as usize, frames as usize, build) {
        Ok(v) => {
            let n = v.len() as u32;
            *OUT.lock().unwrap() = v;
            n
        }
        Err(_) => 0,
    }
}

/// Where `ew_run`'s samples are.
#[no_mangle]
pub extern "C" fn ew_out() -> *const f32 {
    OUT.lock().unwrap().as_ptr()
}
