//! The transport as a node plays it (#359, research/TRANSPORT.md): the one place that turns the published tempo map and
//! its epoch (`Shared::transport`) into what a block of a node plays – the map of each frame and the transport's edge
//! (stop · start · continue) with the frame it lies on. Every node that follows the transport (`seq`, `click`, `walk`,
//! `events`; the descriptor's `follow_transport`, a node's own param) holds a `Follow` and asks it once per block; a
//! node that does not follow gets a free line at the clock's tempo while the transport stands. No allocation.
use super::TempoMap;

/// a jump of the beat bigger than this between two publishes is a new position (a start, a song position, a snap); a
/// tempo change or a follower's nudge never jumps
pub const JUMP_BEATS: f64 = 0.01;

/// what the transport did
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// stopped: hold the position, release what sounds
    Stop,
    /// running again on another beat: bar 1, a song position (or a jump while it ran)
    Start,
    /// running again on the beat where it stopped
    Continue,
}

/// A block as a node plays it: frames [0, `split`) on `before`, [`split`, n) on `map`; the edge (if any) at `split`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Played {
    pub before: TempoMap,
    pub map: TempoMap,
    pub split: usize,
    pub edge: Option<Edge>,
}

impl Played {
    /// the map frame offset `i` of the block plays on
    pub fn at(&self, i: usize) -> &TempoMap {
        if i < self.split {
            &self.before
        } else {
            &self.map
        }
    }
}

/// One node's view of the transport, block after block.
#[derive(Debug, Clone)]
pub struct Follow {
    on: bool,
    /// the map and epoch of the last block (on)
    last: Option<(TempoMap, u64)>,
    /// the beat it stopped on (on: a run again from there continues, from elsewhere starts)
    stood: f64,
    /// the last running line (off: it goes on while the transport stands)
    free: Option<TempoMap>,
}

impl Follow {
    pub fn new(on: bool) -> Follow {
        Follow { on, last: None, stood: 0.0, free: None }
    }

    /// does the node follow the transport
    pub fn on(&self) -> bool {
        self.on
    }

    /// The block [start, start + n) on the transport's `map` of `epoch`.
    pub fn block(&mut self, map: TempoMap, epoch: u64, start: f64, n: usize) -> Played {
        let whole = |m: TempoMap| Played { before: m, map: m, split: 0, edge: None };
        if !self.on {
            if map.running {
                self.free = Some(map);
                return whole(map);
            }
            // the transport stands: on along the last running line (or from here, at the clock's tempo)
            let free = *self.free.get_or_insert(TempoMap { running: true, ..map });
            return whole(free);
        }
        let Some((prev, was)) = self.last.replace((map, epoch)) else {
            self.stood = map.beat_at(start);
            return whole(map);
        };
        if was == epoch && prev.running == map.running {
            return whole(map);
        }
        // the edge lies where the new map was anchored: inside the block it splits it, before it at its first frame; past
        // its end it has not happened yet – this block plays on the old map, a later one sees the edge
        let k = map.frame - start;
        if k >= n as f64 {
            self.last = Some((prev, was));
            return whole(prev);
        }
        let split = if k > 0.0 { k.ceil() as usize } else { 0 };
        let at = start + split as f64;
        if prev.running && !map.running {
            self.stood = prev.beat_at(at);
        }
        let edge = match (prev.running, map.running) {
            (true, false) => Some(Edge::Stop),
            (false, true) if (self.stood - map.beat_at(at)).abs() <= JUMP_BEATS => Some(Edge::Continue),
            (_, true) => Some(Edge::Start),
            // a position set while it stands: nothing plays, nothing to do
            (false, false) => None,
        };
        Played { before: if split > 0 { prev } else { map }, map, split, edge }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    #[test]
    fn a_follower_sees_stop_continue_and_start_on_their_frames() {
        let run = TempoMap::new(120.0, 4, RATE);
        let mut f = Follow::new(true);
        assert_eq!(f.block(run, 0, 0.0, 128).edge, None);
        assert_eq!(f.block(run.at(128.0), 0, 128.0, 128).edge, None);
        // stopped at frame 300 – inside the block from 256: frames 256..300 still run on the old map
        let stop = run.with_running(300.0, false);
        let p = f.block(stop, 1, 256.0, 128);
        assert_eq!((p.edge, p.split), (Some(Edge::Stop), 44));
        assert!(p.at(43).running && !p.at(44).running);
        assert_eq!(f.block(stop.at(384.0), 1, 384.0, 128).edge, None, "stands: no edge");
        // published ahead of the block: the edge waits for the block it lies in
        let mut g = Follow::new(true);
        g.block(run, 0, 0.0, 128);
        let p = g.block(run.with_running(200.0, false), 1, 0.0, 128);
        assert_eq!((p.edge, p.map.running), (None, true));
        assert_eq!(g.block(run.with_running(200.0, false), 1, 128.0, 128).split, 72);
        // running again on the same beat: continue, at the block's first frame
        let cont = stop.with_running(512.0, true);
        let p = f.block(cont, 2, 512.0, 128);
        assert_eq!((p.edge, p.split), (Some(Edge::Continue), 0));
        // back to bar 1 while running: a start
        let bar1 = cont.with_beat(640.0, 0.0);
        assert_eq!(f.block(bar1, 3, 640.0, 128).edge, Some(Edge::Start));
        // a song position while stopped, then running: a start
        let spp = bar1.with_running(768.0, false);
        f.block(spp, 4, 768.0, 128);
        f.block(spp.with_beat(896.0, 16.0), 5, 896.0, 128);
        assert_eq!(f.block(spp.with_beat(896.0, 16.0).with_running(1024.0, true), 6, 1024.0, 128).edge, Some(Edge::Start));
    }

    #[test]
    fn a_node_that_does_not_follow_runs_on_through_a_stop() {
        let run = TempoMap::new(120.0, 4, RATE);
        let mut f = Follow::new(false);
        f.block(run, 0, 0.0, 128);
        let stop = run.with_running(128.0, false);
        let p = f.block(stop.at(24_000.0), 1, 24_000.0, 128);
        assert_eq!(p.edge, None);
        assert!(p.map.running);
        assert!((p.map.beat_at(24_000.0) - 1.0).abs() < 1e-12, "the free line goes on at the clock's tempo");
        // the transport runs again from bar 1: the node is on its line again
        let p = f.block(stop.with_beat(48_000.0, 0.0).with_running(48_000.0, true), 2, 48_000.0, 128);
        assert_eq!(p.map.beat_at(48_000.0), 0.0);
    }
}
