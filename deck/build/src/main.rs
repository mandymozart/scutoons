//! `scutoons-build [serve]` – the HTTP service (env: `MODULE_BUILD_TOKEN` required, see config.rs).
//! `scutoons-build stub <dir>` – writes the stub member crate the image vendors the SDK's dependencies from.
//! `scutoons-build prime` – compiles that stub once, so the SDK and its dependencies are built into the build folder.
//! `scutoons-build rebuild <dir>` – compiles every module source in a folder like a request would (#344); the failures are
//! the list, exit 1 when there are any. No token, no database: `docker run --rm -v "$PWD/modules:/modules" scutoons-build rebuild /modules`.
use std::sync::Arc;

use scutoons_build::compile::{manifest, Compiler, STUB_SOURCE};
use scutoons_build::config::Config;
use scutoons_build::files::Files;
use scutoons_build::service::{serve, Service};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("serve") => run().await,
        Some("stub") => stub(args.get(1).ok_or_else(|| anyhow::anyhow!("usage: scutoons-build stub <dir>"))?),
        Some("prime") => prime().await,
        Some("rebuild") => rebuild(args.get(1).ok_or_else(|| anyhow::anyhow!("usage: scutoons-build rebuild <dir of module sources>"))?).await,
        Some(other) => anyhow::bail!("unknown command {other:?} (serve | stub <dir> | prime | rebuild <dir>)"),
    }
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// the settings of `stub` and `prime`, which need no secret
fn config_without_secret() -> anyhow::Result<Config> {
    Config::from_env(|k| env(k).or_else(|| (k == "MODULE_BUILD_TOKEN").then(|| "-".to_string()))).map_err(anyhow::Error::msg)
}

async fn run() -> anyhow::Result<()> {
    let config = Arc::new(Config::from_env(env).map_err(anyhow::Error::msg)?);
    // folders a crashed run left behind
    if let Ok(entries) = std::fs::read_dir(&config.work_dir) {
        for entry in entries.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("member-")) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    let compiler = Arc::new(Compiler::new(config.clone()));
    tracing::info!(listen = %config.listen, toolchain = compiler.toolchain().unwrap_or("?"), live_check = config.status_url.as_deref().unwrap_or("off"), "module build service");
    let listener = TcpListener::bind(&config.listen).await?;
    let service = Arc::new(Service::new(config, compiler));
    tokio::select! {
        r = serve(listener, service) => Ok(r?),
        _ = stop_signal() => {
            tracing::info!("build service stopping");
            Ok(())
        }
    }
}

/// the stub member crate: ours, with a one-port module, so `cargo vendor` sees exactly what every member depends on
fn stub(dir: &str) -> anyhow::Result<()> {
    let config = config_without_secret()?;
    let dir = std::path::Path::new(dir);
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), manifest(&config.sdk_dir))?;
    std::fs::write(dir.join("src").join("lib.rs"), STUB_SOURCE)?;
    println!("stub crate in {}", dir.display());
    Ok(())
}

async fn prime() -> anyhow::Result<()> {
    let mut config = config_without_secret()?;
    config.status_url = None;
    config.wall = config.wall.max(std::time::Duration::from_secs(900));
    config.memory_mb = 0;
    let compiler = Compiler::new(Arc::new(config));
    let files = Files::from([("lib.rs".to_string(), STUB_SOURCE.to_string())]);
    match compiler.compile(&files).await {
        Ok(built) => {
            println!("primed: {} bytes, module {}, {}", built.wasm.len(), built.descriptor["name"], built.hash);
            Ok(())
        }
        Err(e) => anyhow::bail!("the stub module does not compile: {e:?}"),
    }
}

/// every module in `dir` through a compile; prints the report, fails when a module does (the failures are the recompile list)
async fn rebuild(dir: &str) -> anyhow::Result<()> {
    let mut config = config_without_secret()?;
    config.status_url = None;
    let modules = scutoons_build::rebuild::read_modules(std::path::Path::new(dir))?;
    anyhow::ensure!(!modules.is_empty(), "no module sources in {dir}: a folder per module with src/*.rs or *.rs, or <id>.rs");
    let compiler = Compiler::new(Arc::new(config));
    let rows = scutoons_build::rebuild::rebuild_all(&compiler, modules).await;
    let (text, failed) = scutoons_build::rebuild::report(&rows);
    print!("{text}");
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Ctrl-C, or SIGTERM on unix (the container's stop)
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
