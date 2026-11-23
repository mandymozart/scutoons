//! A module's instance table on its way to the frame thread: the engine's control step (the producer, on the audio
//! clock) hands the table its module wrote; the space (the consumer) reads it each frame and draws it at the module
//! node's place (space/drawn.rs). The latest table wins: a step that finds the frame thread reading skips its copy and
//! the next step's comes a block later – the producer never waits and never allocates.
use deck_module::Instance;
use std::sync::{Arc, Mutex, MutexGuard};

/// One module node's table.
pub struct Table {
    /// the module node's id in the graph (its place: space/drawn.rs)
    pub node: String,
    capacity: usize,
    rows: Mutex<Vec<Instance>>,
    /// the models its node's `model` slots name, loaded, each slot's set in its order
    pub models: Vec<Model>,
}

/// One model of a `model` slot's set: the slot's index in the module's descriptor, the shape the slot is named after
/// (`shape::NAMES`; none: only instances whose `look` names the slot draw it), a .glb's bytes.
#[derive(Debug, Clone)]
pub struct Model {
    pub slot: u32,
    pub shape: Option<u32>,
    pub bytes: Arc<[u8]>,
}

impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Table").field("node", &self.node).field("capacity", &self.capacity).finish()
    }
}

impl Table {
    /// for node `node`, holding at most `capacity` instances (allocated here, once)
    pub fn new(node: &str, capacity: usize) -> Arc<Table> {
        Table::with_models(node, capacity, Vec::new())
    }

    /// … whose instances draw with its slots' models instead of the placeholder
    pub fn with_models(node: &str, capacity: usize, models: Vec<Model>) -> Arc<Table> {
        Arc::new(Table { node: node.to_string(), capacity, rows: Mutex::new(Vec::with_capacity(capacity)), models })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn rows(&self) -> MutexGuard<'_, Vec<Instance>> {
        self.rows.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The producer's side: `rows` (up to the capacity) become the table. False when the frame thread was reading –
    /// this copy is skipped.
    pub fn publish(&self, rows: &[Instance]) -> bool {
        let Ok(mut held) = self.rows.try_lock() else { return false };
        held.clear();
        held.extend_from_slice(&rows[..rows.len().min(self.capacity)]);
        true
    }

    /// The consumer's side: `f` on the table as last published.
    pub fn with<R>(&self, f: impl FnOnce(&[Instance]) -> R) -> R {
        f(&self.rows())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::instances::shape;

    #[test]
    fn the_latest_table_wins_and_a_busy_reader_skips_a_copy() {
        let t = Table::new("sky", 2);
        let a = Instance::new(shape::PUFF, [1.0, 2.0, 3.0]);
        let b = Instance::new(shape::SEGMENT, [0.0; 3]);
        assert!(t.publish(&[a, b, a]));
        assert_eq!(t.with(|r| r.to_vec()), vec![a, b], "cut at the capacity");
        assert!(t.publish(&[b]));
        t.with(|r| {
            assert_eq!(r, &[b]);
            assert!(!t.publish(&[a]), "the frame thread reads: skipped, not waited for");
        });
        assert_eq!(t.with(|r| r.len()), 1);
    }
}
