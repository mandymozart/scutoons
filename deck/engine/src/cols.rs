//! The control half's columns and dependency levels (the data-oriented model of #183, #182 §4b): every input of a batch is a
//! column per node (`Col`), and `levels` splits a graph into the levels whose nodes never read each other. Moved here from
//! deck-core's `batch.rs` with the reference evaluator (#324); the engine's control half (`control.rs`) and the audio
//! half's level order (`lib.rs`) are the only users.

/// One input port of a batch, a column per node: `values[src] * scale + offset`. A constant is a link to a slot that
/// stays 0 (slot 0 by convention) with scale 0 and the constant as the offset – no branch on the input's kind.
#[derive(Default, Debug, Clone)]
pub struct Col {
    src: Vec<u32>,
    scale: Vec<f32>,
    offset: Vec<f32>,
}

impl Col {
    /// the next node's input: `values[src] * scale + offset` (a constant: `push(0, 0.0, value)`, slot 0 stays 0)
    pub fn push(&mut self, src: u32, scale: f32, offset: f32) {
        self.src.push(src);
        self.scale.push(scale);
        self.offset.push(offset);
    }

    /// node `i`'s input becomes `values[src] * scale + offset`
    pub fn set(&mut self, i: usize, src: u32, scale: f32, offset: f32) {
        (self.src[i], self.scale[i], self.offset[i]) = (src, scale, offset);
    }

    /// room for `n` more nodes – a batch that nodes move into while it runs (deck-engine's LFO shapes) never allocates then
    pub fn reserve(&mut self, n: usize) {
        self.src.reserve(n);
        self.scale.reserve(n);
        self.offset.reserve(n);
    }

    /// node `i`'s input out of the column, the last node's taking its place (its index becomes `i`): (src, scale, offset)
    pub fn swap_remove(&mut self, i: usize) -> (u32, f32, f32) {
        (self.src.swap_remove(i), self.scale.swap_remove(i), self.offset.swap_remove(i))
    }

    /// node `i`'s input this step
    #[inline(always)]
    pub fn get(&self, v: &[f32], i: usize) -> f32 {
        v[self.src[i] as usize] * self.scale[i] + self.offset[i]
    }
}

/// Each node's dependency level (Kahn's order): 0 for a node with no dependencies, else 1 + its deepest dependency's.
/// `deps[t]` = the nodes `t` reads (repeats allowed). Nodes of one level never read each other, so a level's nodes of
/// one kind can run as one batch. A cycle is an error.
pub fn levels(deps: &[Vec<usize>]) -> Result<Vec<usize>, String> {
    let n = deps.len();
    let mut users: Vec<Vec<usize>> = vec![vec![]; n];
    for (t, d) in deps.iter().enumerate() {
        for &f in d {
            users[f].push(t);
        }
    }
    let mut level = vec![0usize; n];
    let mut indeg: Vec<usize> = deps.iter().map(Vec::len).collect();
    let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut seen = 0;
    while let Some(i) = ready.pop() {
        seen += 1;
        for &t in &users[i] {
            level[t] = level[t].max(level[i] + 1);
            indeg[t] -= 1;
            if indeg[t] == 0 {
                ready.push(t);
            }
        }
    }
    if seen < n {
        return Err("the links form a cycle".into());
    }
    Ok(level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_and_columns() {
        // 0 → 1 → 3, 2 → 3 (read twice), 4 alone
        assert_eq!(levels(&[vec![], vec![0], vec![], vec![1, 2, 2], vec![]]).unwrap(), vec![0, 1, 0, 2, 0]);
        assert!(levels(&[vec![1], vec![0]]).is_err());
        let mut c = Col::default();
        c.push(1, 2.0, 0.5);
        c.push(0, 0.0, 7.0); // a constant: slot 0 scaled by 0, plus the value
        let v = [0.0, 3.0];
        assert_eq!((c.get(&v, 0), c.get(&v, 1)), (6.5, 7.0));
        c.set(1, 1, 1.0, 0.0);
        assert_eq!(c.get(&v, 1), 3.0);
    }
}
