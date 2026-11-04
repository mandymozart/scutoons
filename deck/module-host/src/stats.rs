//! `ModuleStats` (#307, ABI-V2 §6): one fixed-size record per instance, updated by the host around each call – no
//! allocation, plain numbers a stats reader copies out (the browser's mirror and the per-path rollup are #307's).
//! The clock is the backend's (`clock_us`); the ABI contributes the allocation count and the event counters.

/// buckets of the time histogram: a call's µs as a share of the budget, `< 1/16, < 1/8, < 1/4, < 1/2, < 1, < 2, < 4, ≥ 4`
pub const HIST_BUCKETS: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModuleStats {
    /// calls that ran the module
    pub calls: u64,
    /// µs: the sum, the slowest, the last
    pub us_sum: u64,
    pub us_max: u32,
    pub us_last: u32,
    /// how the calls' µs fell against the budget (`HIST_BUCKETS`)
    pub hist: [u32; HIST_BUCKETS],
    /// calls past half the quantum (a module getting expensive)
    pub soft: u32,
    /// calls past the budget (0.75 × the quantum): in a show counted and accepted, in strict mode the epoch stops them
    pub overruns: u32,
    /// the module's memory now and at its largest, bytes
    pub memory: u64,
    pub memory_peak: u64,
    /// event records handed in / taken out, inputs the host clamped to frame 0 (late), records past a capacity
    pub events_in: u64,
    pub events_out: u64,
    pub late: u64,
    pub dropped_in: u64,
    pub dropped_out: u64,
    /// allocations inside calls (counted in a show; a trap when strict)
    pub allocs: u64,
    /// control outputs written (cables and `path` targets)
    pub written: u64,
    /// traps and runaways (and failed rebuilds), restarts swapped in (a show)
    pub traps: u32,
    pub restarts: u32,
    /// blocks skipped because a `needs` sensor was not live
    pub unavailable: u64,
}

impl ModuleStats {
    /// one call that returned after `us` µs against a budget of `budget_us` (the quantum is `budget_us / 0.75`)
    pub fn record(&mut self, us: u32, budget_us: u32) {
        self.calls += 1;
        self.us_sum += u64::from(us);
        self.us_max = self.us_max.max(us);
        self.us_last = us;
        let b = u64::from(budget_us.max(1));
        let u = u64::from(us);
        // thresholds b/16, b/8, … 4b: the bucket is how many lie at or below the call's µs
        let bucket = [b / 16, b / 8, b / 4, b / 2, b, 2 * b, 4 * b].iter().take_while(|&&t| u >= t).count();
        self.hist[bucket] += 1;
        // half the quantum = two thirds of the budget
        if 3 * u > 2 * b {
            self.soft += 1;
        }
        if u > b {
            self.overruns += 1;
        }
    }

    pub fn memory(&mut self, bytes: usize) {
        self.memory = bytes as u64;
        self.memory_peak = self.memory_peak.max(self.memory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_land_in_their_bucket_and_count_soft_and_hard_overruns() {
        let mut s = ModuleStats::default();
        for us in [0, 100, 1000, 1500, 2000, 2001, 9000] {
            s.record(us, 2000);
        }
        // 0 → <1/16 · 100 → <1/16 (125) · 1000 → [1/2, 1) · 1500 → [1/2, 1) · 2000 → [1, 2) · 2001 → [1, 2) · 9000 → ≥ 4
        assert_eq!(s.hist, [2, 0, 0, 0, 2, 2, 0, 1]);
        assert_eq!((s.calls, s.us_max, s.us_last, s.soft, s.overruns), (7, 9000, 9000, 4, 2));
        s.memory(65536);
        s.memory(4096);
        assert_eq!((s.memory, s.memory_peak), (4096, 65536));
    }
}
