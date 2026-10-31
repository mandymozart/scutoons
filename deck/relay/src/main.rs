//! `relay` – the Live relay binary (agents/research/LIVE.md B.4). `relay [addr]`, default `0.0.0.0:7700`
//! (`RELAY_ADDR` env). Clients connect to `ws://<host>:7700/live/<code>?role=<role>&group=<group>` and send `hello`.
//! Env: `LIVE_TOKEN_SECRET` (the HMAC secret shared with the API), `RELAY_LEAD_MS` (50 LAN, 150 hosted),
//! `RELAY_BLOCKLIST` (nickname words, comma-separated). `GET /status` → `{ "sessions": n, "live": n }` (the deploy
//! freeze check of B.4).

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        // colours only in a terminal – a container's log (bunny) gets plain lines
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();

    let addr: SocketAddr = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("RELAY_ADDR").ok())
        .unwrap_or_else(|| "0.0.0.0:7700".to_owned())
        .parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "relay listening");

    let config = relay::Config::from_env();
    if config.token_secret.is_none() {
        tracing::warn!("LIVE_TOKEN_SECRET unset: dev mode, any non-empty session token passes");
    }
    tracing::info!(lead_ms = config.lead_ms, "clock lead");
    let relay = Arc::new(relay::Relay::with_config(config));
    tokio::select! {
        r = relay::serve(listener, relay) => r,
        _ = stop_signal() => {
            tracing::info!("relay stopping");
            Ok(())
        }
    }
}

/// Ctrl-C, or SIGTERM on unix – a container's stop (#199: the relay is PID 1 there, which ignores an unhandled SIGTERM,
/// so bunny / docker would wait out their grace period and kill it).
async fn stop_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
