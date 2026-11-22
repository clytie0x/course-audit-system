//! Major-specific (CS) curriculum definition.
//!
//! The actual course data lives in `major.json` next to this file and is
//! compiled into the binary at build time via `include_str!`, then parsed
//! once via `OnceLock<MajorCurriculum>` on first access.
//!
//! To edit the curriculum: modify `major.json` directly (preferred) or regen
//! it from Rust data with `cargo test dump_curricula -- --ignored`.

use crate::models::MajorCurriculum;
use std::sync::OnceLock;

const MAJOR_JSON: &str = include_str!("major.json");

fn curriculum() -> &'static MajorCurriculum {
    static CACHE: OnceLock<MajorCurriculum> = OnceLock::new();
    CACHE.get_or_init(|| serde_json::from_str(MAJOR_JSON).expect("major.json must deserialize"))
}

pub fn get_major_curriculum() -> MajorCurriculum {
    curriculum().clone()
}
