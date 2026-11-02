//! A child process under limits: a wall-clock budget, a memory budget for the whole process tree, a cap on what it may
//! print, and an outside veto (a live show started). Whatever stops it, the **whole tree** is killed – cargo, rustc, a
//! linker: on unix the child leads its own process group and the group gets SIGKILL; elsewhere `taskkill /T`.
//! The child also runs `nice`d, so the relay beside it in the pod keeps its share of the CPU.
use std::future::Future;
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct Limits {
    pub wall: Duration,
    /// resident memory of the whole tree, bytes (0 = unlimited; measured on Linux only)
    pub memory: u64,
    /// stdout / stderr kept, each; the child is killed when it prints more on stdout
    pub output: usize,
    /// how often memory and the veto are looked at
    pub tick: Duration,
}

/// Why a child was stopped
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Killed {
    Timeout,
    Memory,
    Output,
    Vetoed,
}

#[derive(Debug)]
pub struct Outcome {
    pub status: Option<ExitStatus>,
    pub stdout: String,
    pub stderr: String,
    pub killed: Option<Killed>,
}

impl Outcome {
    pub fn success(&self) -> bool {
        self.killed.is_none() && self.status.is_some_and(|s| s.success())
    }
}

/// Reads up to `cap` bytes (the rest is read and dropped, so the child never blocks on a full pipe); `overflow` is
/// told when more than `cap` came
async fn drain<R: AsyncRead + Unpin>(mut r: R, cap: usize, overflow: Option<tokio::sync::watch::Sender<bool>>) -> Vec<u8> {
    let (mut kept, mut total, mut buf) = (Vec::new(), 0usize, [0u8; 8192]);
    loop {
        match r.read(&mut buf).await {
            Ok(0) | Err(_) => return kept,
            Ok(n) => {
                total += n;
                if kept.len() < cap {
                    kept.extend_from_slice(&buf[..n.min(cap - kept.len())]);
                }
                if total > cap {
                    if let Some(tx) = &overflow {
                        let _ = tx.send(true);
                    }
                }
            }
        }
    }
}

/// Runs `cmd` to the end or until a limit stops it. `veto` is asked every tick; true stops the child (`Vetoed`).
pub async fn run_limited(mut cmd: Command, limits: &Limits, veto: Option<&(dyn Fn() -> BoxFuture<'static, bool> + Send + Sync)>) -> std::io::Result<Outcome> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)]
    {
        cmd.process_group(0);
        // SAFETY: only async-signal-safe calls between fork and exec (`nice` is one)
        unsafe {
            cmd.pre_exec(|| {
                libc::nice(10);
                Ok(())
            });
        }
    }
    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);
    let (over_tx, mut over_rx) = tokio::sync::watch::channel(false);
    let out_task = tokio::spawn(drain(child.stdout.take().expect("piped"), limits.output, Some(over_tx)));
    let err_task = tokio::spawn(drain(child.stderr.take().expect("piped"), limits.output / 4, None));

    let deadline = tokio::time::Instant::now() + limits.wall;
    let mut tick = tokio::time::interval(limits.tick);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let (mut killed, mut status) = (None, None);
    loop {
        tokio::select! {
            s = child.wait() => {
                status = s.ok();
                break;
            }
            _ = tokio::time::sleep_until(deadline) => {
                killed = Some(Killed::Timeout);
                break;
            }
            // (a closed channel is stdout having ended, not an overflow)
            _ = async { if over_rx.changed().await.is_err() { std::future::pending::<()>().await } } => {
                killed = Some(Killed::Output);
                break;
            }
            _ = tick.tick() => {
                if limits.memory > 0 && tree_rss(pid).is_some_and(|rss| rss > limits.memory) {
                    killed = Some(Killed::Memory);
                    break;
                }
                if let Some(veto) = veto {
                    if veto().await {
                        killed = Some(Killed::Vetoed);
                        break;
                    }
                }
            }
        }
    }
    if killed.is_some() {
        kill_tree(pid);
        let _ = child.kill().await;
        status = child.wait().await.ok();
    }
    // a pipe can stay open in an orphan that survived: do not wait for it forever
    let stdout = tokio::time::timeout(Duration::from_secs(2), out_task).await.ok().and_then(Result::ok).unwrap_or_default();
    let stderr = tokio::time::timeout(Duration::from_secs(2), err_task).await.ok().and_then(Result::ok).unwrap_or_default();
    Ok(Outcome { status, stdout: String::from_utf8_lossy(&stdout).into_owned(), stderr: String::from_utf8_lossy(&stderr).into_owned(), killed })
}

/// SIGKILL to the child's process group (it leads one)
#[cfg(unix)]
fn kill_tree(pid: u32) {
    if pid > 1 {
        // SAFETY: kill(2) with a negative pid addresses the group; the group is the one this module created
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_tree(pid: u32) {
    if pid > 0 {
        let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

/// Resident bytes of the process group led by `pid` (Linux: /proc); None where it cannot be measured
#[cfg(target_os = "linux")]
fn tree_rss(pid: u32) -> Option<u64> {
    // SAFETY: sysconf has no preconditions
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as u64;
    let mut total = 0u64;
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else { continue };
        if let Some((pgrp, rss_pages)) = parse_stat(&stat) {
            if pgrp == pid {
                total += rss_pages * page;
            }
        }
    }
    Some(total)
}

#[cfg(not(target_os = "linux"))]
fn tree_rss(_pid: u32) -> Option<u64> {
    None
}

/// (process group, resident pages) of a /proc/<pid>/stat line: `pid (comm) state ppid pgrp … rss …` – the comm can hold
/// spaces and parentheses, so the fields count from the last ')'
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_stat(stat: &str) -> Option<(u32, u64)> {
    let rest = &stat[stat.rfind(')')? + 1..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // after the comm: 0 state, 1 ppid, 2 pgrp, … 21 rss
    Some((fields.get(2)?.parse().ok()?, fields.get(21)?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(wall_ms: u64) -> Limits {
        Limits { wall: Duration::from_millis(wall_ms), memory: 0, output: 1 << 20, tick: Duration::from_millis(50) }
    }

    /// a command that runs for `secs` seconds on both platforms
    fn sleeper(secs: u32) -> Command {
        #[cfg(unix)]
        {
            let mut c = Command::new("sleep");
            c.arg(secs.to_string());
            c
        }
        #[cfg(not(unix))]
        {
            let mut c = Command::new("ping");
            c.args(["-n", &(secs + 1).to_string(), "127.0.0.1"]);
            c
        }
    }

    fn printer(text: &str) -> Command {
        #[cfg(unix)]
        {
            let mut c = Command::new("sh");
            c.args(["-c", &format!("printf '%s' '{text}'")]);
            c
        }
        #[cfg(not(unix))]
        {
            let mut c = Command::new("cmd");
            c.args(["/C", &format!("echo {text}")]);
            c
        }
    }

    #[test]
    fn a_proc_stat_line_is_read_from_the_last_parenthesis() {
        let line = "4242 (tricky) name (x)) S 4200 4210 4210 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 1000 5000000 777 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        assert_eq!(parse_stat(line), Some((4210, 777)));
        assert_eq!(parse_stat("garbage"), None);
    }

    #[tokio::test]
    async fn a_command_that_finishes_gives_its_output() {
        let o = run_limited(printer("hello"), &limits(10_000), None).await.unwrap();
        assert!(o.success(), "{o:?}");
        assert!(o.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn the_wall_clock_stops_it() {
        let started = std::time::Instant::now();
        let o = run_limited(sleeper(30), &limits(300), None).await.unwrap();
        assert_eq!(o.killed, Some(Killed::Timeout));
        assert!(!o.success());
        assert!(started.elapsed() < Duration::from_secs(10), "it did not wait for the sleeper: {:?}", started.elapsed());
    }

    #[tokio::test]
    async fn a_veto_stops_it() {
        let veto: &(dyn Fn() -> BoxFuture<'static, bool> + Send + Sync) = &|| Box::pin(async { true });
        let o = run_limited(sleeper(30), &limits(20_000), Some(veto)).await.unwrap();
        assert_eq!(o.killed, Some(Killed::Vetoed));
    }

    #[tokio::test]
    async fn too_much_output_stops_it() {
        let flood = if cfg!(unix) {
            let mut c = Command::new("sh");
            c.args(["-c", "yes aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]);
            c
        } else {
            let mut c = Command::new("cmd");
            c.args(["/C", "for /L %i in (1,1,100000) do @echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]);
            c
        };
        let l = Limits { output: 4096, ..limits(20_000) };
        let o = run_limited(flood, &l, None).await.unwrap();
        assert_eq!(o.killed, Some(Killed::Output));
        assert!(o.stdout.len() <= 4096);
    }

    #[tokio::test]
    async fn a_missing_program_is_an_io_error() {
        assert!(run_limited(Command::new("definitely-not-a-program-281"), &limits(1000), None).await.is_err());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn the_memory_of_the_tree_is_limited() {
        // a shell that holds a 100 MB variable (the limit is 50 MB) – measured through /proc, so Linux only
        let mut c = Command::new("sh");
        c.args(["-c", "x=$(head -c 100000000 /dev/zero | tr '\\0' a); sleep 20"]);
        let l = Limits { memory: 50 << 20, ..limits(20_000) };
        let o = run_limited(c, &l, None).await.unwrap();
        assert_eq!(o.killed, Some(Killed::Memory), "{o:?}");
    }
}
