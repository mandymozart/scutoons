//! The audio threads at the OS's audio priority (#208): Windows' MMCSS task "Pro Audio" at high priority (what DAWs and
//! the WASAPI engine use – the scheduler keeps it ahead of the frame thread, the GPU driver's threads and a busy CPU);
//! Linux SCHED_FIFO (the deck's user is in `realtime`, rtprio 98 – deck/os; PipeWire's own threads get theirs from
//! rtkit). Elsewhere nothing. Called once by the thread itself, never per block.

/// Raise the calling thread; Ok(what it got) or Err(why not – the thread keeps its priority).
pub fn raise() -> Result<&'static str, String> {
    imp::raise()
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
}
