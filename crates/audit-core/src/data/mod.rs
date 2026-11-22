//! Curriculum revisions of the PSU B.Sc. Computer Science programme.
//!
//! Each revision lives in its own JSON file next to this module (the source of
//! truth, edited by hand from the official TQF2 / มคอ.2) and is compiled into
//! the binary via `include_str!`, then parsed once on first access.
//!
//! - `cs2564.json`: หลักสูตรปรับปรุง พ.ศ. 2564, cohorts 2564–2568
//! - `cs2569.json`: หลักสูตรปรับปรุง พ.ศ. 2569, cohorts 2569+

use crate::models::Curriculum;
use std::sync::OnceLock;

const SOURCES: [(&str, &str); 2] = [
    ("cs2564.json", include_str!("cs2564.json")),
    ("cs2569.json", include_str!("cs2569.json")),
];

/// All known revisions, oldest first.
pub fn curricula() -> &'static [Curriculum] {
    static CACHE: OnceLock<Vec<Curriculum>> = OnceLock::new();
    CACHE.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(file, json)| {
                serde_json::from_str(json)
                    .unwrap_or_else(|e| panic!("{file} must deserialize: {e}"))
            })
            .collect()
    })
}

/// Looks up a revision by id, falling back to the oldest one.
pub fn curriculum(id: &str) -> &'static Curriculum {
    let all = curricula();
    all.iter().find(|c| c.id == id).unwrap_or(&all[0])
}

/// The newest revision whose first cohort is not after `cohort` (Buddhist year).
pub fn curriculum_for_cohort(cohort: u32) -> &'static Curriculum {
    let all = curricula();
    all.iter()
        .rev()
        .find(|c| c.first_cohort <= cohort)
        .unwrap_or(&all[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_curricula_deserialize() {
        let ids: Vec<&str> = curricula().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["cs2564", "cs2569"]);
    }

    #[test]
    fn cohort_picks_revision() {
        assert_eq!(curriculum_for_cohort(2567).id, "cs2564");
        assert_eq!(curriculum_for_cohort(2569).id, "cs2569");
        assert_eq!(curriculum_for_cohort(2575).id, "cs2569");
        assert_eq!(curriculum_for_cohort(2560).id, "cs2564");
    }

    #[test]
    fn credit_totals_match_tqf2() {
        // Total = GenEd + major + free electives, as stated in each TQF2.
        for (id, total) in [("cs2564", 132.0), ("cs2569", 125.0)] {
            let c = curriculum(id);
            let sum = c.gen_ed.total_required_credits
                + c.major.total_required_credits
                + c.free_elective_credits;
            assert_eq!(sum, total, "{id}");
            let core: f32 = c.major.core_courses.courses.iter().map(|x| x.credits).sum();
            assert_eq!(
                core, c.major.core_courses.required_credits,
                "{id} core list"
            );
        }
    }
}
