//! `relay` – the Live relay binary (agents/research/LIVE.md B.4). `relay [addr]`, default `0.0.0.0:7700`
//! (`RELAY_ADDR` env). Clients connect to `ws://<host>:7700/live/<code>?role=<role>&group=<group>` and send `hello`;
//! `GET /status` → `{ "sessions": n, "live": n }` (the deploy freeze check of B.4).

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

    let relay = Arc::new(relay::Relay::new());
    tokio::select! {
        r = relay::serve(listener, relay) => r,
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("relay stopping");
            Ok(())
        }
    }
}
