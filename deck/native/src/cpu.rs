//! The process's CPU time, read from the OS on a thread every `SAMPLE_S` and fed to deck-core's load meter: the frame
//! thread only reads the latest reading.
use deck_core::load::{Busy, Meter, Reading, SAMPLE_S};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const NONE: u64 = u64::MAX;

/// The CPU meter's handle; its thread ends when the handle is dropped.
pub struct CpuMeter(Arc<AtomicU64>);

impl CpuMeter {
    /// None where the OS's process times are not read (neither Windows nor Linux)
    pub fn spawn() -> Option<CpuMeter> {
        process_seconds()?;
        let shared = Arc::new(AtomicU64::new(NONE));
        let mine = shared.clone();
        let lanes = std::thread::available_parallelism().map_or(1, |n| n.get());
        std::thread::Builder::new()
            .name("cpu meter".into())
            .spawn(move || {
                let (mut busy, mut meter) = (Busy::default(), Meter::default());
                let (mut cpu, mut at) = (process_seconds().unwrap_or(0.0), Instant::now());
                while Arc::strong_count(&mine) > 1 {
                    std::thread::sleep(Duration::from_secs_f64(SAMPLE_S));
                    let (now_cpu, now) = (process_seconds().unwrap_or(cpu), Instant::now());
                    if let Some((share, span)) = busy.add(now_cpu - cpu, (now - at).as_secs_f64(), lanes) {
                        meter.push(share, span);
                        if let Some(r) = meter.reading() {
                            mine.store(r.to_bits(), Ordering::Relaxed);
                        }
                    }
                    (cpu, at) = (now_cpu, now);
                }
            })
            .ok()?;
        Some(CpuMeter(shared))
    }

    /// None before the first sample
    pub fn reading(&self) -> Option<Reading> {
        Some(self.0.load(Ordering::Relaxed)).filter(|b| *b != NONE).map(Reading::from_bits)
    }
}

/// the process's user + kernel CPU time, all threads (s)
#[cfg(windows)]
fn process_seconds() -> Option<f64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: the pseudo handle of this process and four FILETIMEs it writes
    let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut created, &mut exited, &mut kernel, &mut user) };
    let ticks = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32 | u64::from(f.dwLowDateTime)) as f64;
    (ok != 0).then(|| (ticks(kernel) + ticks(user)) * 1.0e-7)
}

#[cfg(target_os = "linux")]
fn process_seconds() -> Option<f64> {
    // SAFETY: getrusage writes the zeroed struct it is handed
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) } != 0 {
        return None;
    }
    let s = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 * 1.0e-6;
    Some(s(ru.ru_utime) + s(ru.ru_stime))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_seconds() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_busy_thread_shows_on_the_meter() {
        let a = process_seconds().unwrap();
        let t = Instant::now();
        let mut x = 0u64;
        while t.elapsed() < Duration::from_millis(100) {
            x = std::hint::black_box(x.wrapping_add(1));
        }
        let spent = process_seconds().unwrap() - a;
        assert!(spent > 0.05, "{spent} s of CPU in 100 ms of spinning");
        let m = CpuMeter::spawn().unwrap();
        let t = Instant::now();
        while m.reading().is_none() && t.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let r = m.reading().expect("a reading within 5 s");
        assert!((0.0..=1.0).contains(&r.value) && r.peak >= r.value, "{r:?}");
    }
}
