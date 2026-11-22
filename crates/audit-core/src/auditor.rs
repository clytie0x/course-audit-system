//! Curriculum Auditing Engine
//!
//! Implements the core logic for validating student transcripts against curriculum requirements:
//! - **GenEd Auditing**: Matches courses to 6 strands with support for sub-groups and sequences
//! - **Major Auditing**: Matches courses to Basic Science, Core, Capstone, and Electives
//! - **Free Elective Detection**: Credits unmatched courses as free electives
//! - **Greedy Matching**: Allows repeatable courses to accumulate credits

use crate::data::{gen_ed::get_gen_ed_curriculum, major::get_major_curriculum};
use crate::models::{
    compute_gpa, free_elective_dedupe_key, is_in_progress, is_passing_grade, AuditResult, Category,
    Course, GenEdCurriculum, MajorCurriculum, MissingCourse, ParsedCourse,
};
use std::collections::HashSet;

/// Free-elective credits required by the curriculum.
const FREE_ELECTIVE_CREDITS: f32 = 6.0;

/// Runs the full audit pipeline (GenEd → Major → free electives → GPA) over a
/// course list and returns the UI-facing `AuditResult`. The What-If simulator
/// re-invokes this on every plan change, so it's intentionally pure — no
/// signals, no I/O.
pub fn build_audit(parsed: &[ParsedCourse]) -> AuditResult {
    let gen_ed = get_gen_ed_curriculum();
    let major = get_major_curriculum();

    let (gen_ed_credits, gen_ed_missing, gen_ed_used) = audit_gen_ed(parsed, &gen_ed);
    let (major_credits, elective_credits, major_missing, major_used) = audit_major(parsed, &major);

    let mut all_used_courses = gen_ed_used.clone();
    all_used_courses.extend(major_used.clone());

    let (free_elective_credits, _) = calculate_free_electives(parsed, &all_used_courses);

    let mut all_missing: Vec<MissingCourse> = gen_ed_missing;
    all_missing.extend(major_missing);
    // Drop GenEd-wide misses when the total pool is met; Major/Core misses
    // are strict slots and stay regardless of totals.
    all_missing.retain(|m| match m.category.as_str() {
        "General Education" => gen_ed_credits < gen_ed.total_required_credits,
        _ => true,
    });

    if elective_credits < major.electives.total_required_credits {
        all_missing.push(MissingCourse {
            category: "Major Electives".to_string(),
            description: format!(
                "{}: missing {:.1} credits",
                major.electives.name,
                major.electives.total_required_credits - elective_credits
            ),
        });
    }
    if major_credits + elective_credits < major.total_required_credits {
        all_missing.push(MissingCourse {
            category: "Major Courses".to_string(),
            description: format!(
                "Overall Major Courses: missing {:.1} credits",
                major.total_required_credits - major_credits - elective_credits
            ),
        });
    }
    if free_elective_credits < FREE_ELECTIVE_CREDITS {
        all_missing.push(MissingCourse {
            category: "Free Electives".to_string(),
            description: format!(
                "Free Electives: missing {:.1} credits",
                FREE_ELECTIVE_CREDITS - free_elective_credits
            ),
        });
    }

    let total_credits = gen_ed_credits + major_credits + elective_credits + free_elective_credits;

    let mut gen_ed_courses = Vec::new();
    let mut major_courses = Vec::new();
    let mut free_elective_courses = Vec::new();
    let mut seen_free_electives = used_dedupe_keys(parsed, &all_used_courses);

    for (idx, p) in parsed.iter().enumerate() {
        let course = Course {
            code: p.code.clone(),
            name: p.name.clone(),
            credit: p.parsed_credit,
            grade: p.grade.clone(),
            semester: p.semester.clone(),
        };
        if gen_ed_used.contains(&idx) {
            gen_ed_courses.push(course);
        } else if major_used.contains(&idx) {
            major_courses.push(course);
        } else if is_passing_grade(&p.grade) {
            let dedupe_key = free_elective_dedupe_key(&p.code, &p.name);
            if seen_free_electives.insert(dedupe_key) {
                free_elective_courses.push(course);
            }
        }
    }

    // Every graded attempt counts toward GPA, including failed and retaken ones.
    let attempts: Vec<Course> = parsed
        .iter()
        .map(|p| Course {
            code: p.code.clone(),
            name: p.name.clone(),
            credit: p.parsed_credit,
            grade: p.grade.clone(),
            semester: p.semester.clone(),
        })
        .collect();
    let gpa = compute_gpa(&attempts);

    AuditResult {
        total_credits,
        categories: vec![
            Category {
                name: "General Education".to_string(),
                required_credits: gen_ed.total_required_credits,
                collected_credits: gen_ed_credits,
                courses: gen_ed_courses,
            },
            Category {
                name: "Major Courses".to_string(),
                required_credits: major.total_required_credits,
                collected_credits: major_credits + elective_credits,
                courses: major_courses,
            },
            Category {
                name: "Free Electives".to_string(),
                required_credits: FREE_ELECTIVE_CREDITS,
                collected_credits: free_elective_credits,
                courses: free_elective_courses,
            },
        ],
        missing_subjects: all_missing,
        gpa,
        attempts,
    }
}

/// Dedupe keys of courses already placed in a curriculum slot, so a retake of
/// the same course never also counts as a free elective.
fn used_dedupe_keys(courses: &[ParsedCourse], used_indices: &HashSet<usize>) -> HashSet<String> {
    used_indices
        .iter()
        .map(|&i| free_elective_dedupe_key(&courses[i].code, &courses[i].name))
        .collect()
}

/// A course counts for *matching* (fills a curriculum slot) when it's passing
/// **or** currently in-progress. Only passing courses count toward credit
/// totals — see `earned_credits`.
fn matches_curriculum_slot(parsed: &ParsedCourse) -> bool {
    is_passing_grade(&parsed.grade) || is_in_progress(&parsed.grade)
}

/// Returns the lesser of the curriculum-defined credit value and the parsed
/// transcript value, guarding against PDF-parsing drift. In-progress courses
/// fill their slot but contribute 0 credits until they're graded.
fn earned_credits(curriculum_credits: f32, parsed: &ParsedCourse) -> f32 {
    if is_in_progress(&parsed.grade) {
        0.0
    } else {
        curriculum_credits.min(parsed.parsed_credit)
    }
}

/// Audits courses against the GenEd curriculum, honoring strand sub-groups and
/// sequential strand rules. Credits come from the curriculum (golden data).
pub fn audit_gen_ed(
    courses: &[ParsedCourse],
    curriculum: &GenEdCurriculum,
) -> (f32, Vec<MissingCourse>, HashSet<usize>) {
    let mut completed_credits = 0.0;
    let mut missing_courses: Vec<MissingCourse> = Vec::new();
    let mut used_indices = HashSet::new();
    let mut gen_ed_elective_total_credits = 0.0;

    for strand in &curriculum.strands {
        let selection_rule = strand.selection_rule.as_deref().unwrap_or("choose_all");

        match selection_rule {
            "choose_sequential_pair" => {
                let mut sequence_satisfied = false;

                if let (Some(strand_courses), Some(sequence_groups)) =
                    (&strand.courses, &strand.sequence_groups)
                {
                    'outer: for pair in sequence_groups {
                        if pair.len() != 2 {
                            continue;
                        }

                        let mut found_indices = Vec::new();
                        let mut credits_sum = 0.0;

                        for code in pair {
                            if let Some(def_course) =
                                strand_courses.iter().find(|c| &c.code == code)
                            {
                                if let Some((idx, parsed)) =
                                    courses.iter().enumerate().find(|(idx, parsed)| {
                                        !used_indices.contains(idx)
                                            && parsed.code == *code
                                            && matches_curriculum_slot(parsed)
                                    })
                                {
                                    found_indices.push(idx);
                                    credits_sum += earned_credits(def_course.credits, parsed);
                                }
                            }
                        }

                        if found_indices.len() == 2 {
                            for idx in found_indices {
                                used_indices.insert(idx);
                            }
                            completed_credits += credits_sum;
                            sequence_satisfied = true;

                            break 'outer;
                        }
                    }
                }

                if !sequence_satisfied {
                    if let Some(sequence_groups) = &strand.sequence_groups {
                        let pair_text = sequence_groups
                            .iter()
                            .filter(|p| p.len() == 2)
                            .map(|p| format!("{} + {}", p[0], p[1]))
                            .collect::<Vec<_>>()
                            .join(" OR ");

                        missing_courses.push(MissingCourse {
                            category: "General Education".to_string(),
                            description: format!(
                                "{}: choose one pair ({})",
                                strand.name, pair_text
                            ),
                        });
                    }
                }
            }
            "choose_one" => {
                if let Some(strand_courses) = &strand.courses {
                    if let Some((_course, idx, matched_credits)) =
                        strand_courses.iter().find_map(|course| {
                            courses
                                .iter()
                                .enumerate()
                                .find(|(idx, parsed)| {
                                    !used_indices.contains(idx)
                                        && parsed.code == course.code
                                        && matches_curriculum_slot(parsed)
                                })
                                .map(|(idx, parsed)| {
                                    (course, idx, earned_credits(course.credits, parsed))
                                })
                        })
                    {
                        completed_credits += matched_credits;
                        used_indices.insert(idx);
                    } else {
                        let options = strand_courses
                            .iter()
                            .map(|c| format!("{} - {}", c.code, c.name))
                            .collect::<Vec<_>>()
                            .join(" OR ");

                        missing_courses.push(MissingCourse {
                            category: "General Education".to_string(),
                            description: format!("{}: choose 1 ({})", strand.name, options),
                        });
                    }
                }
            }
            "choose_all_sub_groups" => {
                if let Some(sub_groups) = &strand.sub_groups {
                    for sub_group in sub_groups {
                        let mut sub_group_credits = 0.0;

                        for course in &sub_group.courses {
                            if sub_group_credits >= sub_group.required_credits {
                                break;
                            }

                            if let Some((idx, parsed)) =
                                courses.iter().enumerate().find(|(idx, parsed)| {
                                    !used_indices.contains(idx)
                                        && parsed.code == course.code
                                        && matches_curriculum_slot(parsed)
                                })
                            {
                                let matched_credits = earned_credits(course.credits, parsed);
                                completed_credits += matched_credits;
                                sub_group_credits += matched_credits;
                                used_indices.insert(idx);
                            }
                        }

                        if sub_group_credits < sub_group.required_credits {
                            let options = sub_group
                                .courses
                                .iter()
                                .map(|c| format!("{} - {}", c.code, c.name))
                                .collect::<Vec<_>>()
                                .join(" OR ");

                            missing_courses.push(MissingCourse {
                                category: "General Education".to_string(),
                                description: format!(
                                    "{} > {}: missing {:.1} credits (options: {})",
                                    strand.name,
                                    sub_group.name,
                                    sub_group.required_credits - sub_group_credits,
                                    options
                                ),
                            });
                        }
                    }
                }
            }
            _ => {
                if let Some(strand_courses) = &strand.courses {
                    for course in strand_courses {
                        if let Some((idx, parsed)) =
                            courses.iter().enumerate().find(|(idx, parsed)| {
                                !used_indices.contains(idx)
                                    && parsed.code == course.code
                                    && matches_curriculum_slot(parsed)
                            })
                        {
                            let matched_credits = earned_credits(course.credits, parsed);
                            completed_credits += matched_credits;
                            used_indices.insert(idx);
                        } else {
                            missing_courses.push(MissingCourse {
                                category: "General Education".to_string(),
                                description: format!(
                                    "{}: {} - {}",
                                    strand.name, course.code, course.name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    // GenEd electives stop at the required total; extra ones stay unused and
    // fall through to free electives.
    for sub_cat in &curriculum.electives.sub_categories {
        let mut sub_cat_credits = 0.0;
        for course in &sub_cat.courses {
            if gen_ed_elective_total_credits >= curriculum.electives.total_required_credits {
                break;
            }
            if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
                !used_indices.contains(idx)
                    && parsed.code == course.code
                    && matches_curriculum_slot(parsed)
            }) {
                let matched_credits = earned_credits(course.credits, parsed);
                completed_credits += matched_credits;
                gen_ed_elective_total_credits += matched_credits;
                sub_cat_credits += matched_credits;
                used_indices.insert(idx);
            }
        }

        if sub_cat_credits < sub_cat.required_credits {
            missing_courses.push(MissingCourse {
                category: "General Education".to_string(),
                description: format!(
                    "GenEd Elective > {}: missing {:.1} credits",
                    sub_cat.name,
                    sub_cat.required_credits - sub_cat_credits
                ),
            });
        }
    }

    if gen_ed_elective_total_credits < curriculum.electives.total_required_credits {
        missing_courses.push(MissingCourse {
            category: "General Education".to_string(),
            description: format!(
                "{}: missing {:.1} credits",
                curriculum.electives.name,
                curriculum.electives.total_required_credits - gen_ed_elective_total_credits
            ),
        });
    }

    if completed_credits < curriculum.total_required_credits {
        let has_ge_summary = missing_courses.iter().any(|m| {
            m.category == "General Education"
                && m.description.starts_with("Overall General Education")
        });

        if !has_ge_summary {
            missing_courses.push(MissingCourse {
                category: "General Education".to_string(),
                description: format!(
                    "Overall General Education: missing {:.1} credits",
                    curriculum.total_required_credits - completed_credits
                ),
            });
        }
    }

    (completed_credits, missing_courses, used_indices)
}

/// Audits courses against the major curriculum, including greedy matching for
/// special-topics and other elective buckets. Credits are taken from curriculum
/// data to avoid PDF parsing drift.
pub fn audit_major(
    courses: &[ParsedCourse],
    curriculum: &MajorCurriculum,
) -> (f32, f32, Vec<MissingCourse>, HashSet<usize>) {
    let mut completed_credits = 0.0;
    let mut elective_credits = 0.0;
    let mut missing_courses: Vec<MissingCourse> = Vec::new();
    let mut used_indices = HashSet::new();

    for course in &curriculum.basic_science.courses {
        if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
            !used_indices.contains(idx)
                && parsed.code == course.code
                && matches_curriculum_slot(parsed)
        }) {
            let matched_credits = earned_credits(course.credits, parsed);
            completed_credits += matched_credits;
            used_indices.insert(idx);
        } else {
            missing_courses.push(MissingCourse {
                category: "Basic Science".to_string(),
                description: format!("{} - {}", course.code, course.name),
            });
        }
    }

    for course in &curriculum.core_courses.courses {
        if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
            !used_indices.contains(idx)
                && parsed.code == course.code
                && matches_curriculum_slot(parsed)
        }) {
            let matched_credits = earned_credits(course.credits, parsed);
            completed_credits += matched_credits;
            used_indices.insert(idx);
        } else {
            missing_courses.push(MissingCourse {
                category: "Core Courses".to_string(),
                description: format!("{} - {}", course.code, course.name),
            });
        }
    }

    let mut capstone_completed = false;
    for option in &curriculum.capstone.options {
        if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
            !used_indices.contains(idx)
                && parsed.code == option.code
                && matches_curriculum_slot(parsed)
        }) {
            let matched_credits = earned_credits(option.credits, parsed);
            completed_credits += matched_credits;
            used_indices.insert(idx);
            capstone_completed = true;

            break;
        }
    }

    if !capstone_completed {
        let options_desc = curriculum
            .capstone
            .options
            .iter()
            .map(|o| format!("{} ({})", o.code, o.name))
            .collect::<Vec<_>>()
            .join(" OR ");

        missing_courses.push(MissingCourse {
            category: "Capstone".to_string(),
            description: format!("Choose 1: {}", options_desc),
        });
    }

    let mut completed_clusters_count = 0;
    for domain in &curriculum.electives.domains {
        for cluster in &domain.clusters {
            let mut courses_found_in_cluster = 0;
            for course in &cluster.courses {
                if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
                    !used_indices.contains(idx)
                        && parsed.code == course.code
                        && matches_curriculum_slot(parsed)
                }) {
                    let matched_credits = earned_credits(course.credits, parsed);
                    elective_credits += matched_credits;
                    used_indices.insert(idx);
                    courses_found_in_cluster += 1;
                } else if courses
                    .iter()
                    .any(|c| c.code == course.code && matches_curriculum_slot(c))
                {
                    // Course taken but used elsewhere (or duplicate). Still counts towards completion of the cluster.
                    courses_found_in_cluster += 1;
                }
            }
            if courses_found_in_cluster >= cluster.min_courses {
                completed_clusters_count += 1;
            }
        }
    }

    if completed_clusters_count < curriculum.electives.clusters_to_complete {
        missing_courses.push(MissingCourse {
            category: "Major Electives".to_string(),
            description: format!(
                "Required: {} Clusters, Completed: {}. Please complete all courses within at least {} clusters.",
                curriculum.electives.clusters_to_complete,
                completed_clusters_count,
                curriculum.electives.clusters_to_complete
            ),
        });
    }

    // Greedy match "others" electives so repeated special topics accumulate
    // credits; other repeated codes (retakes) count once.
    let mut seen_others: HashSet<String> = HashSet::new();
    for course in &curriculum.electives.others {
        for (idx, parsed) in courses.iter().enumerate() {
            if !used_indices.contains(&idx)
                && parsed.code == course.code
                && matches_curriculum_slot(parsed)
                && seen_others.insert(free_elective_dedupe_key(&parsed.code, &parsed.name))
            {
                let matched_credits = earned_credits(course.credits, parsed);
                elective_credits += matched_credits;
                used_indices.insert(idx);
            }
        }
    }

    (
        completed_credits,
        elective_credits,
        missing_courses,
        used_indices,
    )
}

/// Calculates free-elective credits from unused courses, pulling credit values
/// directly from the PDF when the course is not mapped elsewhere.
pub fn calculate_free_electives(
    courses: &[ParsedCourse],
    used_indices: &HashSet<usize>,
) -> (f32, Vec<String>) {
    let mut free_elective_credits = 0.0;
    let mut free_elective_list = Vec::new();
    let mut seen_free_electives = used_dedupe_keys(courses, used_indices);

    for (idx, parsed) in courses.iter().enumerate() {
        if !used_indices.contains(&idx) && is_passing_grade(&parsed.grade) {
            let dedupe_key = free_elective_dedupe_key(&parsed.code, &parsed.name);
            if !seen_free_electives.insert(dedupe_key) {
                continue;
            }

            let credits = parsed.parsed_credit;
            free_elective_credits += credits;
            free_elective_list.push(format!(
                "{} (Grade: {}, {} cr)",
                parsed.code, parsed.grade, credits
            ));
        }
    }

    (free_elective_credits, free_elective_list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::gen_ed::get_gen_ed_curriculum;

    fn parsed(code: &str, grade: &str, credit: f32) -> ParsedCourse {
        ParsedCourse {
            code: code.to_string(),
            name: code.to_string(),
            grade: grade.to_string(),
            parsed_credit: credit,
            semester: None,
        }
    }

    fn category<'a>(result: &'a AuditResult, name: &str) -> &'a Category {
        result.categories.iter().find(|c| c.name == name).unwrap()
    }

    fn has_missing(result: &AuditResult, category: &str) -> bool {
        result
            .missing_subjects
            .iter()
            .any(|m| m.category == category)
    }

    #[test]
    fn gpa_counts_failed_attempts() {
        let result = build_audit(&[parsed("322-101", "A", 3.0), parsed("322-102", "F", 3.0)]);
        assert_eq!(result.gpa, Some(2.0));
        assert_eq!(
            result.attempts.len(),
            2,
            "failed attempts stay visible per semester"
        );
    }

    #[test]
    fn e_grade_does_not_fill_slot() {
        let result = build_audit(&[parsed("388-100", "E", 1.0)]);
        assert_eq!(result.total_credits, 0.0);
        assert_eq!(result.gpa, Some(0.0));
    }

    #[test]
    fn retake_after_passing_counts_credits_once() {
        let result = build_audit(&[parsed("322-101", "D", 3.0), parsed("322-101", "B", 3.0)]);
        assert_eq!(result.total_credits, 3.0);
        assert_eq!(category(&result, "Free Electives").collected_credits, 0.0);
        // Both attempts stay in GPA: (1.0 * 3 + 3.0 * 3) / 6.
        assert_eq!(result.gpa, Some(2.0));
    }

    #[test]
    fn repeated_non_topic_elective_counts_once() {
        let result = build_audit(&[parsed("344-493", "A", 3.0), parsed("344-493", "B", 3.0)]);
        assert_eq!(result.total_credits, 3.0);
        assert_eq!(category(&result, "Major Courses").courses.len(), 1);
    }

    #[test]
    fn repeatable_special_topics_still_accumulate() {
        let mut first = parsed("344-496", "A", 3.0);
        first.name = "Special Topics (Topic 1)".to_string();
        let mut second = parsed("344-496", "A", 3.0);
        second.name = "Special Topics (Topic 2)".to_string();
        let result = build_audit(&[first, second]);
        assert_eq!(result.total_credits, 6.0);
    }

    #[test]
    fn gen_ed_electives_overflow_to_free_electives() {
        let gen_ed = get_gen_ed_curriculum();
        let electives: Vec<ParsedCourse> = gen_ed
            .electives
            .sub_categories
            .iter()
            .flat_map(|s| s.courses.iter())
            .filter(|c| c.credits == 2.0)
            .take(5)
            .map(|c| parsed(&c.code, "A", c.credits))
            .collect();
        assert_eq!(electives.len(), 5);

        let result = build_audit(&electives);
        assert_eq!(
            category(&result, "General Education").collected_credits,
            6.0
        );
        assert_eq!(category(&result, "Free Electives").collected_credits, 4.0);
    }

    #[test]
    fn missing_reports_major_and_free_elective_credit_shortfall() {
        let result = build_audit(&[]);
        assert!(has_missing(&result, "Major Courses"));
        assert!(has_missing(&result, "Major Electives"));
        assert!(has_missing(&result, "Free Electives"));

        let result = build_audit(&[parsed("999-001", "A", 3.0), parsed("999-002", "A", 3.0)]);
        assert!(!has_missing(&result, "Free Electives"));
    }

    #[test]
    fn ip_course_fills_slot_but_adds_zero_credits() {
        // 388-100 "Health for All" is a required course in Strand 1.
        let courses = vec![parsed("388-100", "IP", 1.0)];
        let gen_ed = get_gen_ed_curriculum();
        let (credits, missing, used) = audit_gen_ed(&courses, &gen_ed);

        assert!(used.contains(&0), "IP course should fill its slot");
        assert_eq!(credits, 0.0, "IP course must not add credits");
        // Health for All should NOT appear in missing — its slot is filled.
        assert!(
            !missing
                .iter()
                .any(|m| m.description.contains("388-100") && m.description.contains("Health")),
            "Filled slot should not appear in missing list"
        );
    }

    #[test]
    fn passing_course_still_adds_credits() {
        let courses = vec![parsed("388-100", "A", 1.0)];
        let gen_ed = get_gen_ed_curriculum();
        let (credits, _missing, used) = audit_gen_ed(&courses, &gen_ed);

        assert!(used.contains(&0));
        assert_eq!(credits, 1.0, "passing course contributes its credits");
    }

    #[test]
    fn ip_courses_excluded_from_free_electives() {
        // A course with IP grade that isn't in the curriculum should NOT
        // land in free electives — it's not yet graded.
        let courses = vec![parsed("999-999", "IP", 3.0)];
        let used = HashSet::new();
        let (fe_credits, fe_list) = calculate_free_electives(&courses, &used);

        assert_eq!(fe_credits, 0.0);
        assert!(fe_list.is_empty());
    }

    #[test]
    fn failing_grade_does_not_fill_slot() {
        let courses = vec![parsed("388-100", "F", 1.0)];
        let gen_ed = get_gen_ed_curriculum();
        let (credits, missing, used) = audit_gen_ed(&courses, &gen_ed);

        assert!(!used.contains(&0), "F grade must not fill slot");
        assert_eq!(credits, 0.0);
        assert!(
            missing.iter().any(|m| m.description.contains("388-100")),
            "F'd course's slot should still be listed as missing"
        );
    }
}
