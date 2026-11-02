//! The service's settings, from the environment (the image sets the paths; bunny sets `MODULE_BUILD_TOKEN`).
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// `BUILD_LISTEN`, default `0.0.0.0:7710`
    pub listen: String,
    /// `MODULE_BUILD_TOKEN`: the shared secret the API sends as `X-Build-Token`. The service does not start without it.
    pub token: String,
    /// `BUILD_SDK_DIR` (`/opt/sdk`): holds `module/` (deck-module) and `module-macros/`
    pub sdk_dir: PathBuf,
    /// `BUILD_WORK_DIR` (`/var/build/work`): a temporary folder per compile, deleted after
    pub work_dir: PathBuf,
    /// `BUILD_TARGET_DIR` (`/var/build/target`): cargo's build folder, shared, with the SDK and its dependencies built at image build time
    pub target_dir: PathBuf,
    /// `BUILD_CARGO_HOME` (`/opt/cargo-home`): its `config.toml` replaces crates.io by the vendored closure; None = cargo's own
    pub cargo_home: Option<PathBuf>,
    /// `BUILD_LOCK` (`/opt/lock/Cargo.lock`): the lock copied into every member crate (pins the versions); None = none
    pub lock_file: Option<PathBuf>,
    /// `BUILD_CARGO`, default `cargo`
    pub cargo: String,
    /// `RELAY_STATUS_URL`, default `http://127.0.0.1:7700/status` (the relay beside us in the pod); `off` = no check
    pub status_url: Option<String>,
    /// `BUILD_JOBS`, default 2 (`CARGO_BUILD_JOBS`: the relay beside us must stay smooth)
    pub jobs: u32,
    /// `BUILD_WALL_SECS`, default 60
    pub wall: Duration,
    /// `BUILD_MEMORY_MB`, default 4096: resident memory of cargo + rustc together (Linux; 0 = unlimited)
    pub memory_mb: u64,
    /// `BUILD_MAX_WASM_KB`, default 2048
    pub max_wasm: usize,
    /// `BUILD_QUEUE`, default 4: requests waiting for the one compile slot; a fifth is turned away at once (503 busy)
    pub max_queue: usize,
    /// `BUILD_QUEUE_WAIT_SECS`, default 30: how long a request waits for the slot (503 busy after)
    pub queue_wait: Duration,
}

impl Config {
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let var = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let number = |k: &str, default: u64| -> Result<u64, String> {
            match var(k) {
                Some(v) => v.trim().parse().map_err(|_| format!("{k} must be a number, not {v:?}")),
                None => Ok(default),
            }
        };
        let token = var("MODULE_BUILD_TOKEN").ok_or("MODULE_BUILD_TOKEN is not set: the service refuses to start without its shared secret")?;
        let path = |k: &str, default: &str| PathBuf::from(var(k).unwrap_or_else(|| default.to_string()));
        Ok(Config {
            listen: var("BUILD_LISTEN").unwrap_or_else(|| "0.0.0.0:7710".into()),
            token,
            sdk_dir: path("BUILD_SDK_DIR", "/opt/sdk"),
            work_dir: path("BUILD_WORK_DIR", "/var/build/work"),
            target_dir: path("BUILD_TARGET_DIR", "/var/build/target"),
            cargo_home: Some(path("BUILD_CARGO_HOME", "/opt/cargo-home")).filter(|p| p.is_dir()),
            lock_file: Some(path("BUILD_LOCK", "/opt/lock/Cargo.lock")).filter(|p| p.is_file()),
            cargo: var("BUILD_CARGO").unwrap_or_else(|| "cargo".into()),
            status_url: match var("RELAY_STATUS_URL") {
                Some(v) if v.eq_ignore_ascii_case("off") => None,
                Some(v) => Some(v),
                None => Some("http://127.0.0.1:7700/status".into()),
            },
            jobs: number("BUILD_JOBS", 2)?.max(1) as u32,
            wall: Duration::from_secs(number("BUILD_WALL_SECS", 60)?.max(1)),
            memory_mb: number("BUILD_MEMORY_MB", 4096)?,
            max_wasm: number("BUILD_MAX_WASM_KB", 2048)? as usize * 1024,
            max_queue: number("BUILD_QUEUE", 4)? as usize,
            queue_wait: Duration::from_secs(number("BUILD_QUEUE_WAIT_SECS", 30)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn it_does_not_start_without_its_secret() {
        assert!(Config::from_env(env(&[])).unwrap_err().contains("MODULE_BUILD_TOKEN"));
        assert!(Config::from_env(env(&[("MODULE_BUILD_TOKEN", "  ")])).is_err());
    }

    #[test]
    fn the_defaults_are_the_pods() {
        let c = Config::from_env(env(&[("MODULE_BUILD_TOKEN", "s3cret")])).unwrap();
        assert_eq!((c.listen.as_str(), c.jobs, c.wall.as_secs(), c.memory_mb, c.max_queue, c.queue_wait.as_secs()), ("0.0.0.0:7710", 2, 60, 4096, 4, 30));
        assert_eq!(c.status_url.as_deref(), Some("http://127.0.0.1:7700/status"));
        assert_eq!((c.max_wasm, c.target_dir.to_str()), (2 * 1024 * 1024, Some("/var/build/target")));
    }

    #[test]
    fn everything_can_be_set_and_the_check_switched_off() {
        let c = Config::from_env(env(&[("MODULE_BUILD_TOKEN", "t"), ("BUILD_JOBS", "1"), ("BUILD_WALL_SECS", "5"), ("RELAY_STATUS_URL", "off"), ("BUILD_QUEUE", "0"), ("BUILD_MAX_WASM_KB", "64")])).unwrap();
        assert_eq!((c.jobs, c.wall.as_secs(), c.max_queue, c.max_wasm, c.status_url), (1, 5, 0, 65536, None));
        assert!(Config::from_env(env(&[("MODULE_BUILD_TOKEN", "t"), ("BUILD_JOBS", "many")])).unwrap_err().contains("BUILD_JOBS"));
    }
}
