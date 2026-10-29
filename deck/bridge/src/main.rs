//! `bridge <venue.toml>` – start the adapters of the venue patch and join the relay as the bridge role
//! (LIVE.md B.7). Reconnects with a 2 s pause when the relay goes away; Ctrl-C stops the adapters cleanly.

use std::time::Duration;

use adapters_core::Inbound;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let path = std::env::args().nth(1).unwrap_or_else(|| "venue.toml".to_owned());
    let patch = bridge::Patch::parse(&std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("{path}: {e}"))?)?;
    let mut adapters = bridge::build_adapters(&patch)?;

    // Bounded: a device callback never blocks on a slow relay (adapters-core).
    let (tx, mut inbound) = mpsc::channel::<Inbound>(256);
    for (outlet, adapter) in adapters.iter_mut() {
        adapter.start(tx.clone()).map_err(|e| anyhow::anyhow!("{outlet}: {e}"))?;
        tracing::info!(outlet, adapter = adapter.name(), "started");
    }

    let run = async {
        loop {
            match bridge::session(&patch, &mut adapters, &mut inbound).await {
                Ok(()) => break,
                Err(e) => tracing::warn!("relay session ended: {e} – retrying in 2 s"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };
    tokio::select! {
        _ = run => {}
        _ = tokio::signal::ctrl_c() => tracing::info!("bridge stopping"),
    }
    for (outlet, adapter) in adapters.iter_mut() {
        if let Err(e) = adapter.stop() {
            tracing::warn!(outlet, "stop: {e}");
        }
    }
    Ok(())
}
