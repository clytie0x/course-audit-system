//! Static curriculum data for PSU Computer Science program
//!
//! Contains all required courses and requirements for:
//! - General Education (GenEd) requirements
//! - Major-specific courses

pub mod gen_ed;
pub mod major;

#[cfg(test)]
mod dump {
    // One-shot JSON dumper — invoked manually via
    // `cargo test dump_curricula -- --ignored --nocapture` whenever the
    // in-Rust curriculum data changes. Writes the serialized curriculum back
    // to JSON files that production code loads via include_str! + serde_json.
    use super::gen_ed::get_gen_ed_curriculum;
    use super::major::get_major_curriculum;

    #[test]
    #[ignore]
    fn dump_curricula() {
        let gen = serde_json::to_string_pretty(&get_gen_ed_curriculum()).unwrap();
        let maj = serde_json::to_string_pretty(&get_major_curriculum()).unwrap();
        std::fs::write("src/data/gen_ed.json", gen).unwrap();
        std::fs::write("src/data/major.json", maj).unwrap();
    }
}
