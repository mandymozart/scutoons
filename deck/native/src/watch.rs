//! The dev mode's file watch (`--dev`, #185): visuals as data reload while the stage runs – a scene's TOML (the whole
//! show is rebuilt; a scene that doesn't load keeps the running one and shows why) and its shader nodes' WGSL files
//! (only that pass is recompiled; the audio plays on). Polls the files' modification times a few times a second on the
//! frame thread – a handful of `stat`s, no watcher thread, no dependency.
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// how often the files are looked at
const EVERY: Duration = Duration::from_millis(300);

pub struct Watch {
    files: Vec<(PathBuf, Option<SystemTime>)>,
    last: Instant,
}

fn modified(p: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl Watch {
    pub fn new(paths: impl IntoIterator<Item = PathBuf>) -> Watch {
        Watch { files: paths.into_iter().map(|p| (modified(&p), p)).map(|(t, p)| (p, t)).collect(), last: Instant::now() }
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.iter().map(|(p, _)| p)
    }

    /// the indices of the files whose modification time changed since the last look (at most every 300 ms; a file
    /// that appears or disappears counts as changed)
    pub fn changed(&mut self) -> Vec<usize> {
        if self.last.elapsed() < EVERY {
            return Vec::new();
        }
        self.poll()
    }

    /// `changed` without the wait
    pub fn poll(&mut self) -> Vec<usize> {
        self.last = Instant::now();
        let mut out = Vec::new();
        for (i, (p, t)) in self.files.iter_mut().enumerate() {
            let now = modified(p);
            if now != *t {
                *t = now;
                out.push(i);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_file_is_seen_once() {
        let dir = std::env::temp_dir().join(format!("deck-native-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.wgsl"), dir.join("b.toml"));
        std::fs::write(&a, "1").unwrap();
        let mut w = Watch::new([a.clone(), b.clone()]);
        assert!(w.poll().is_empty());
        // a new modification time (set, not waited for: file systems count in coarse steps)
        let f = std::fs::File::options().write(true).open(&a).unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(5)).unwrap();
        drop(f);
        std::fs::write(&b, "x").unwrap();
        assert_eq!(w.poll(), [0, 1], "changed, and appeared");
        assert!(w.poll().is_empty());
        std::fs::remove_file(&b).unwrap();
        assert_eq!(w.poll(), [1], "gone");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
