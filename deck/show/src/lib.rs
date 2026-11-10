//! The show core (#89, research/SHOW.md): what a live show is and who fires what. RULES A1: the deck editor, the conductor
//! view and a browser only draw this.
//!
//! - [`document`]: a show is a matrix, rows × outlets = cells; a cell is a piece plus values. One document per conductor, and
//!   the seat in the parent's document is the relation to a child's show (owner 2026-11-10). The child does not name its parent.
//! - [`validate`]: one document on its own, and the whole tree following the seats from the top (unknown references, a used
//!   row the child's face lacks, cycles); the tree resolved to exact versions, like a lock file.
//! - [`conductor`]: the bare conductor. Loads a show, takes events, fires rows, always sends a full cell.
pub mod conductor;
pub mod document;
pub mod validate;

pub use conductor::{Clock, Conductor, Entry, Logged, Now, Origin, Out, Says, Suggestion};
pub use document::{parse, Cell, Face, Fire, How, Moment, Outlet, OutletKind, ParseError, Pinned, Row, Seat, Show, Values, Word};
pub use validate::{resolve, validate, Library, Lock, Problem};
