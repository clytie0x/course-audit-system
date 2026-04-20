//! General-Education curriculum definition.
//!
//! The actual course data lives in `gen_ed.json` next to this file and is
//! compiled into the binary at build time via `include_str!`, then parsed
//! once via `OnceLock<GenEdCurriculum>` on first access.
//!
//! To edit the curriculum: modify `gen_ed.json` directly (preferred) or regen
//! it from Rust data with `cargo test dump_curricula -- --ignored`.

use crate::models::GenEdCurriculum;
use std::sync::OnceLock;

const GEN_ED_JSON: &str = include_str!("gen_ed.json");

fn curriculum() -> &'static GenEdCurriculum {
    static CACHE: OnceLock<GenEdCurriculum> = OnceLock::new();
    CACHE.get_or_init(|| serde_json::from_str(GEN_ED_JSON).expect("gen_ed.json must deserialize"))
}

pub fn get_gen_ed_curriculum() -> GenEdCurriculum {
    curriculum().clone()
}
