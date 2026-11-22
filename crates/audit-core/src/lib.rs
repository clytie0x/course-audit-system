//! Pure transcript parsing and curriculum auditing for PSU Computer Science.
//!
//! No browser or wasm dependencies, so `cargo test -p audit-core` runs natively.

pub mod auditor;
pub mod data;
pub mod models;
pub mod parser;
