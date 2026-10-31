//! `relay` – the Live relay binary (agents/research/LIVE.md B.4). `relay [addr] [--tls-cert FILE --tls-key FILE]`,
//! default `0.0.0.0:7700` (`RELAY_ADDR` env). Clients connect to `ws://<host>:7700/live/<code>?role=<role>&group=<group>`
//! and send `hello`; with a certificate (`RELAY_TLS_CERT` / `RELAY_TLS_KEY` too) the same port also takes
//! `wss://<name>:7700/…` (tls.rs, #201 – a venue's relay for phones on https pages; reloaded when the files change).
//! Env: `LIVE_TOKEN_SECRET` (the HMAC secret shared with the API), `RELAY_LEAD_MS` (50 LAN, 150 hosted),
//! `RELAY_BLOCKLIST` (nickname words, comma-separated). `GET /status` → `{ "sessions": n, "live": n }` (the deploy
//! freeze check of B.4), over http and https.

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

    let listen = relay::tls::Listen::parse(std::env::args().skip(1), |k| std::env::var(k).ok()).map_err(anyhow::Error::msg)?;
    let addr: SocketAddr = listen.addr.parse()?;
    let tls = match &listen.tls {
        Some(files) => Some(relay::tls::acceptor(files.clone()).map_err(anyhow::Error::msg)?),
        None => None,
    };
    let listener = TcpListener::bind(addr).await?;
    match &listen.tls {
        Some(files) => tracing::info!(%addr, cert = %files.cert.display(), "relay listening (ws + wss)"),
        None => tracing::info!(%addr, "relay listening (ws)"),
    }

    let config = relay::Config::from_env();
    if config.token_secret.is_none() {
        tracing::warn!("LIVE_TOKEN_SECRET unset: dev mode, any non-empty session token passes");
    }
    tracing::info!(lead_ms = config.lead_ms, "clock lead");
    let relay = Arc::new(relay::Relay::with_config(config));
    tokio::select! {
        r = relay::serve_with(listener, relay, tls) => r,
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
