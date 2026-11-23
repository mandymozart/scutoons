//! The show core (#89, research/SHOW.md): what a live show is and who fires what. RULES A1: the deck editor, the conductor
//! view and a browser only draw this.
//!
//! - [`document`]: a show is a matrix, rows × outlets = cells; a cell is a piece plus values. One document per conductor, and
//!   the seat in the parent's document is the relation to a child's show (owner 2026-11-10). The child does not name its parent.
//! - [`validate`]: one document on its own, and the whole tree following the seats from the top (unknown references, a used
//!   row the child's face lacks, cycles); the tree resolved to exact versions, like a lock file.
//! - [`conductor`]: the bare conductor. Loads a show, takes events, fires rows, always sends a full cell.
//! - [`subtree`]: one conductor in the tree (#90): its seats below, its parent above, its clock, the word and the lock.
//! - [`flash`], [`address`]: the rule of a flash, what a receiver of an address answers.
//! - [`ready`]: a deck's side of a flash (#91): the show its outlet plays, kept at the pinned version, every piece checked,
//!   the report in words; it never blocks GO.
//! - [`view`]: what the deck editor's Show and Perform workspaces draw: the matrix, the log in words, the inspector (#91).
//! - [`choose`]: choosing what to flash (#471): every device of the tree, the filters, the prompt, what the flash names.
//! - [`roles`]: the conductor's roles in the editor (#478): their filter in words, how many are here.
/// a show one deck plays on its own, as its own conductor: Play's listing, the cells for its outlet
pub mod alone;
pub mod address;
pub mod canvas;
pub mod choose;
pub mod conductor;
pub mod dialog;
pub mod document;
/// a Library piece let go on the row's node view: the cell it becomes the piece of
pub mod drops;
pub mod face;
pub mod far;
pub mod flash;
pub mod flashed;
/// a row as a node graph: the Show workspace's node view
pub mod nodes;
/// #551: a vote as cells of the launcher – what the right click on selected cells offers, the question in the cell inspector
pub mod question;
pub mod ready;
pub mod roles;
/// #537: a show's rows (add, rename, remove) and a cell's mode and piece – the edits, the prompt, the picker's lines
pub mod rows;
pub mod sizes;
/// #539: the toon's story events a row fires on – the API's answer read, the event picker's lines
pub mod story;
pub mod subtree;
pub mod validate;
/// a cell's values against its piece's knobs: the inspector's scrub numbers, a value set, take current, the path warning
pub mod values;
pub mod view;
/// #533: the vote's conductor side – what the relay said of the session's vote, the panel in words; #548: a row's question
/// as `vote.open`, its result as the show's event
pub mod vote;
/// the wiring gestures of the rows band and the node view as row edits: take, land, let go, cut, insert
pub mod wire_ops;
pub mod wiring;

pub use conductor::{Ask, Clock, Conductor, Entry, Logged, Now, Origin, Out, Says, Suggestion};
pub use document::{parse, Cell, Face, Fire, How, Moment, Outlet, OutletKind, ParseError, Pinned, Row, Seat, Show, Tempo, Values, Word};
pub use flash::Assignment;
pub use subtree::{ClockChoice, Subtree};
pub use validate::{resolve, validate, Library, Lock, Problem};
