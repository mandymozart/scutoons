//! The solo rule, once (Reaper's): soloing a track keeps the folder it plays into and the bus its send feeds (downstream);
//! soloing a folder keeps its children (upstream). The engine's buses and the editor's mixer strips both ask it.

use std::collections::BTreeSet;

/// Whether a strip is heard: nothing soloed, or it is soloed itself, or a strip feeding it is (upstream), or one it feeds is (downstream).
pub fn heard(any_solo: bool, own: bool, upstream: bool, downstream: bool) -> bool {
    !any_solo || own || upstream || downstream
}

/// every node with a path into `node` over `edges` (from, to); `node` itself only through a cycle
pub fn upstream<K: Clone + Ord>(edges: &[(K, K)], node: &K) -> Vec<K> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![node.clone()];
    let mut found = Vec::new();
    while let Some(t) = stack.pop() {
        for (from, _) in edges.iter().filter(|(_, to)| *to == t) {
            if seen.insert(from.clone()) {
                found.push(from.clone());
                stack.push(from.clone());
            }
        }
    }
    found
}

/// every node `node` feeds, directly (`first`: the targets its sounding output reaches) or further down; never `node` itself
pub fn downstream<K: Clone + Ord>(edges: &[(K, K)], first: &[K], node: &K) -> Vec<K> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<K> = first.to_vec();
    let mut found = Vec::new();
    while let Some(t) = stack.pop() {
        if t == *node || !seen.insert(t.clone()) {
            continue;
        }
        found.push(t.clone());
        stack.extend(edges.iter().filter(|(from, _)| *from == t).map(|(_, to)| to.clone()));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// keys -> fx (a send), drums -> kit (a folder) -> master; the strips are 0 keys, 1 fx, 2 drums, 3 kit
    const EDGES: [(usize, usize); 2] = [(0, 1), (2, 3)];

    fn heard_in(solos: &[usize], node: usize) -> bool {
        let first: Vec<usize> = EDGES.iter().filter(|e| e.0 == node).map(|e| e.1).collect();
        let (up, down) = (upstream(&EDGES, &node), downstream(&EDGES, &first, &node));
        heard(!solos.is_empty(), solos.contains(&node), up.iter().any(|n| solos.contains(n)), down.iter().any(|n| solos.contains(n)))
    }

    #[test]
    fn nothing_soloed_all_heard() {
        assert!((0..4).all(|n| heard_in(&[], n)));
    }

    #[test]
    fn a_track_keeps_the_send_and_the_folder_it_feeds() {
        assert_eq!((0..4).map(|n| heard_in(&[0], n)).collect::<Vec<_>>(), [true, true, false, false], "keys: its send stays");
        assert_eq!((0..4).map(|n| heard_in(&[2], n)).collect::<Vec<_>>(), [false, false, true, true], "drums: its folder stays");
    }

    #[test]
    fn a_folder_or_send_keeps_what_feeds_it() {
        assert_eq!((0..4).map(|n| heard_in(&[3], n)).collect::<Vec<_>>(), [false, false, true, true]);
        assert_eq!((0..4).map(|n| heard_in(&[1], n)).collect::<Vec<_>>(), [true, true, false, false]);
    }

    #[test]
    fn a_cycle_does_not_loop() {
        let edges = [(0, 1), (1, 0)];
        assert_eq!(downstream(&edges, &[1], &0), vec![1]);
        assert_eq!(upstream(&edges, &0).len(), 2);
    }
}
