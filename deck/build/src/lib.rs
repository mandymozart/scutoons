//! scutoons-build – the rig module compile service (#281, agents/research/RIG-EDITING.md §4, §7): a creator's Rust files in,
//! a `.wasm` module with its descriptor out (or rustc's errors with their lines). A second container in the relay's bunny
//! app (`scutoons-live`), reached through the API (`POST /api/modules/compile`), which checks that the caller is a creator
//! and stores the result. Member code is compiled, never run here; see `compile` for the rules and `service` for the routes.
pub mod compile;
pub mod config;
pub mod diag;
pub mod files;
pub mod http;
pub mod live;
pub mod runner;
pub mod scan;
pub mod service;
