//! The audio threads at the OS's audio priority (#208): Windows' MMCSS task "Pro Audio" at high priority (what DAWs and
//! the WASAPI engine use – the scheduler keeps it ahead of the frame thread, the GPU driver's threads and a busy CPU);
//! Linux SCHED_FIFO (the deck's user is in `realtime`, rtprio 98 – deck/os; PipeWire's own threads get theirs from
//! rtkit). Elsewhere nothing. Called once by the thread itself, never per block.

/// Raise the calling thread; Ok(what it got) or Err(why not – the thread keeps its priority).
pub fn raise() -> Result<&'static str, String> {
    imp::raise()
}

/// The deck image's audio core (deck/os deck-session: `DECK_AUDIO_CPUS`, e.g. "3,7" – one physical core's hyperthreads; the
/// session starts the deck on the other cores): the calling thread moves onto it. Unset (a dev PC) = nothing. Ok(the cores) or
/// Err(why not); None when unset.
pub fn pin_audio() -> Option<Result<String, String>> {
    let cpus = audio_cpus()?;
    Some(imp::pin(cpus).map(|_| format!("CPU {}", cpus.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","))))
}

/// PipeWire's data thread (our `process` callback), once at its first call on the deck image: onto the audio core, and
/// SCHED_FIFO when rtkit / the client's module-rt left it without (above the render thread, under the daemon's 88).
/// Unset `DECK_AUDIO_CPUS` = nothing. Ok(what it did) or Err.
pub fn place_data_thread() -> Option<Result<String, String>> {
    let cpus = audio_cpus()?;
    Some(imp::pin(cpus).and_then(|_| imp::ensure_fifo(DATA_PRIO)).map(|how| format!("on CPU {cpus:?}, {how}")))
}

/// the data thread's FIFO priority if it has none: above the render thread (70), under PipeWire's own (88)
const DATA_PRIO: i32 = 80;

/// `DECK_AUDIO_CPUS` parsed once ("3,7", "2-3"); None when unset, empty or unreadable
pub fn audio_cpus() -> Option<&'static [usize]> {
    static CPUS: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
    let v = CPUS.get_or_init(|| std::env::var("DECK_AUDIO_CPUS").ok().map(|s| parse_cpus(&s)).unwrap_or_default());
    (!v.is_empty()).then_some(v.as_slice())
}

/// a CPU list as Linux writes it ("3,7", "0-2,5"); anything unreadable = empty
pub fn parse_cpus(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let range = match part.split_once('-') {
            Some((a, b)) => a.trim().parse::<usize>().ok().zip(b.trim().parse::<usize>().ok()),
            None => part.parse::<usize>().ok().map(|a| (a, a)),
        };
        let Some((a, b)) = range.filter(|(a, b)| a <= b && *b < 1024) else { return Vec::new() };
        out.extend(a..=b);
    }
    out
}

/// High, but not audio (#236: the events' sequencer and dispatcher): ahead of the frame thread and the loaders, behind
/// the render thread – Windows THREAD_PRIORITY_HIGHEST, Linux nice −10 (the deck's `realtime` group may).
pub fn raise_high() -> Result<&'static str, String> {
    imp::raise_high()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Threading::{AvSetMmThreadCharacteristicsW, AvSetMmThreadPriority, AVRT_PRIORITY_HIGH};

    /// "Pro Audio\0" as UTF-16 (a static: no allocation in an audio thread)
    const TASK: [u16; 10] = [80, 114, 111, 32, 65, 117, 100, 105, 111, 0];

    pub fn raise() -> Result<&'static str, String> {
        let mut index = 0u32;
        // SAFETY: TASK is NUL-terminated and lives forever; index is ours to write
        let h = unsafe { AvSetMmThreadCharacteristicsW(TASK.as_ptr(), &mut index) };
        if h.is_null() {
            // SAFETY: plain call
            return Err(format!("AvSetMmThreadCharacteristicsW failed ({})", unsafe { GetLastError() }));
        }
        // SAFETY: h is the MMCSS handle just returned
        if unsafe { AvSetMmThreadPriority(h, AVRT_PRIORITY_HIGH) } == 0 {
            return Ok("MMCSS Pro Audio");
        }
        Ok("MMCSS Pro Audio, high")
    }

    pub fn pin(_: &[usize]) -> Result<(), String> {
        Err("no audio core on Windows".into())
    }

    pub fn ensure_fifo(_: i32) -> Result<&'static str, String> {
        Err("no SCHED_FIFO on Windows".into())
    }

    pub fn raise_high() -> Result<&'static str, String> {
        use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
        // SAFETY: the pseudo handle of the calling thread, always valid
        if unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) } == 0 {
            // SAFETY: plain call
            return Err(format!("SetThreadPriority failed ({})", unsafe { GetLastError() }));
        }
        Ok("THREAD_PRIORITY_HIGHEST")
    }

    #[cfg(test)]
    #[test]
    fn the_task_name_is_pro_audio() {
        let s: String = char::decode_utf16(TASK[..9].iter().copied()).map(|c| c.unwrap()).collect();
        assert_eq!(s, "Pro Audio");
    }
}

#[cfg(target_os = "linux")]
mod imp {
    /// under PipeWire's data threads (rt.prio 88, rootfs/etc/pipewire) – the device's own thread comes first
    const PRIO: i32 = 70;

    pub fn raise() -> Result<&'static str, String> {
        let param = libc::sched_param { sched_priority: PRIO };
        // SAFETY: pthread_self is always valid; param is a valid sched_param
        let r = unsafe { libc::pthread_setschedparam(libc::pthread_self(), libc::SCHED_FIFO, &param) };
        match r {
            0 => Ok("SCHED_FIFO 70"),
            e => Err(format!("SCHED_FIFO refused ({e}: no rtprio limit – the user is not in `realtime`)")),
        }
    }

    /// the calling thread onto `cpus`
    pub fn pin(cpus: &[usize]) -> Result<(), String> {
        // SAFETY: a zeroed cpu_set_t is the empty set; CPU_SET stays inside it (cpus < 1024 = CPU_SETSIZE, parse_cpus)
        let r = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for &c in cpus {
                libc::CPU_SET(c, &mut set);
            }
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set)
        };
        match r {
            0 => Ok(()),
            _ => Err(format!("sched_setaffinity refused ({})", std::io::Error::last_os_error())),
        }
    }

    /// the calling thread at SCHED_FIFO `prio` unless it is realtime already
    pub fn ensure_fifo(prio: i32) -> Result<&'static str, String> {
        // SAFETY: plain call on the calling thread
        let now = unsafe { libc::sched_getscheduler(0) };
        if now == libc::SCHED_FIFO || now == libc::SCHED_RR {
            return Ok("realtime already");
        }
        let param = libc::sched_param { sched_priority: prio };
        // SAFETY: pthread_self is always valid; param is a valid sched_param
        match unsafe { libc::pthread_setschedparam(libc::pthread_self(), libc::SCHED_FIFO, &param) } {
            0 => Ok("SCHED_FIFO 80 (it had none)"),
            e => Err(format!("SCHED_FIFO refused ({e})")),
        }
    }

    pub fn raise_high() -> Result<&'static str, String> {
        // SAFETY: plain calls; a thread's nice value on Linux is per thread (its tid)
        let r = unsafe { libc::setpriority(libc::PRIO_PROCESS, libc::gettid() as libc::id_t, -10) };
        match r {
            0 => Ok("nice -10"),
            _ => Err("nice -10 refused (no nice limit – the user is not in `realtime`)".into()),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn raise() -> Result<&'static str, String> {
        Err("no audio priority on this OS".into())
    }

    pub fn raise_high() -> Result<&'static str, String> {
        Err("no thread priority on this OS".into())
    }

    pub fn pin(_: &[usize]) -> Result<(), String> {
        Err("no audio core on this OS".into())
    }

    pub fn ensure_fifo(_: i32) -> Result<&'static str, String> {
        Err("no SCHED_FIFO on this OS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_cpus;

    #[test]
    fn cpu_lists_read_as_linux_writes_them() {
        assert_eq!(parse_cpus("3,7"), [3, 7]);
        assert_eq!(parse_cpus("0-2, 5"), [0, 1, 2, 5]);
        assert!(parse_cpus("").is_empty());
        assert!(parse_cpus("3,x").is_empty(), "unreadable: nothing pinned");
        assert!(parse_cpus("5-2").is_empty());
        assert!(parse_cpus("4096").is_empty(), "past CPU_SETSIZE");
    }
}
