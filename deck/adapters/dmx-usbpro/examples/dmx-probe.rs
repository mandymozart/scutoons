//! The hardware check for the owner (#187): lists the USB-DMX widgets this machine sees, opens one, logs its device
//! check (firmware, serial), sets channel values, holds them and blacks out.
//!
//! `cargo run -p adapters-dmx-usbpro --example dmx-probe -- [--port NAME] [--fade MS] [--hold S] [CH=V[,V…]] …`
//! e.g. `-- 1=255,0,0 10=128` (channel 1–3 red on an RGB panel at address 1, channel 10 at half). Without values it
//! only lists, checks and sends a black frame. Mind a strobe's speed channel: start low.

use std::time::Duration;

use adapters_core::Adapter;
use adapters_dmx_usbpro::{detect, DmxUsbProAdapter};
use anyhow::{anyhow, Context};
use protocol::{DeviceMsg, DmxMessage};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let mut port = None;
    let mut fade = None;
    let mut hold = 5u64;
    let mut sets = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => port = Some(args.next().ok_or_else(|| anyhow!("--port NAME"))?),
            "--fade" => fade = Some(args.next().ok_or_else(|| anyhow!("--fade MS"))?.parse().context("--fade")?),
            "--hold" => hold = args.next().ok_or_else(|| anyhow!("--hold S"))?.parse().context("--hold")?,
            set => {
                let (ch, values) = set.split_once('=').ok_or_else(|| anyhow!("{set:?}: CH=V[,V…]"))?;
                let values = values.split(',').map(|v| v.trim().parse::<u8>()).collect::<Result<Vec<_>, _>>().with_context(|| set.to_owned())?;
                sets.push(DmxMessage { ch: ch.parse().with_context(|| set.to_owned())?, values, fade });
            }
        }
    }

    let found = detect::candidates();
    println!("serial ports ({}):", found.len());
    for c in &found {
        println!("  {} {}", if c.score() > 0 { "*" } else { " " }, c.describe());
    }

    let mut adapter = DmxUsbProAdapter::new("probe", port.as_deref());
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    adapter.start(tx)?;
    // the thread opens the widget and runs the device check (≤ 0.5 s) before the first frame
    std::thread::sleep(Duration::from_millis(800));
    for set in sets {
        adapter.send(&DeviceMsg::Dmx(set))?;
    }
    println!("holding {hold} s …");
    std::thread::sleep(Duration::from_secs(hold));
    adapter.panic()?;
    std::thread::sleep(Duration::from_millis(100));
    adapter.stop()?;
    println!("blacked out.");
    Ok(())
}
