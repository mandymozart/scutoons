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
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("relay stopping");
            Ok(())
        }
    }
}
