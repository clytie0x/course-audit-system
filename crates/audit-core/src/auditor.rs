//! Curriculum Auditing Engine
//!
//! Implements the core logic for validating student transcripts against curriculum requirements:
//! - **GenEd Auditing**: Matches courses to 6 strands with support for sub-groups and sequences
//! - **Major Auditing**: Matches courses to Basic Science, Core, Capstone, and Electives
//! - **Free Elective Detection**: Credits unmatched courses as free electives
//! - **Greedy Matching**: Allows repeatable courses to accumulate credits

use crate::models::{
    compute_gpa, free_elective_dedupe_key, is_in_progress, is_passing_grade, AuditResult, Category,
    Course, Curriculum, GenEdCurriculum, MajorCourse, MajorCurriculum, MissingCourse, ParsedCourse,
};
use std::collections::HashSet;

/// Runs the full audit pipeline (GenEd → Major → free electives → GPA) over a
/// course list for one curriculum revision and returns the UI-facing
/// `AuditResult`. The What-If simulator re-invokes this on every plan change,
/// so it's intentionally pure — no signals, no I/O.
pub fn build_audit(parsed: &[ParsedCourse], curriculum: &Curriculum) -> AuditResult {
    let gen_ed = &curriculum.gen_ed;
    let major = &curriculum.major;
    let free_required = curriculum.free_elective_credits;

    let (gen_ed_earned, gen_ed_missing, gen_ed_used) = audit_gen_ed(parsed, gen_ed);
    let gen_ed_credits = gen_ed_earned.min(gen_ed.total_required_credits);
    let mj = audit_major(parsed, major, &gen_ed_used);
    let major_credits = mj.completed_credits;
    let elective_credits = mj.elective_credits.min(mj.elective_required);
    // GenEd and major-elective credits beyond what the plan requires count as
    // free electives.
    let surplus = (gen_ed_earned - gen_ed_credits) + (mj.elective_credits - elective_credits);
    let major_used = mj.used;

    let mut all_used_courses = gen_ed_used.clone();
    all_used_courses.extend(major_used.iter().copied());

    let (unused_credits, _) = calculate_free_electives(parsed, &all_used_courses);
    let free_elective_credits = unused_credits + surplus;

    let mut all_missing: Vec<MissingCourse> = gen_ed_missing;
    all_missing.extend(mj.missing);
    // Drop GenEd-wide misses when the total pool is met; Major/Core misses
    // are strict slots and stay regardless of totals.
    all_missing.retain(|m| match m.category.as_str() {
        "General Education" => gen_ed_credits < gen_ed.total_required_credits,
        _ => true,
    });

    if elective_credits < mj.elective_required {
        all_missing.push(MissingCourse {
            category: "Major Electives".to_string(),
            description: format!(
                "{}: missing {:.1} credits",
                major.electives.name,
                mj.elective_required - elective_credits
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
    if free_elective_credits < free_required {
        all_missing.push(MissingCourse {
            category: "Free Electives".to_string(),
            description: format!(
                "Free Electives: missing {:.1} credits",
                free_required - free_elective_credits
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
    if let Some(g) = gpa.filter(|g| *g < curriculum.min_gpa) {
        all_missing.push(MissingCourse {
            category: "Graduation".to_string(),
            description: format!(
                "Cumulative GPA {g:.2} is below the minimum {:.2}",
                curriculum.min_gpa
            ),
        });
    }

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
                required_credits: free_required,
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

fn has_tag(parsed: &ParsedCourse, tags: &[String]) -> bool {
    parsed.tag.as_ref().is_some_and(|t| tags.contains(t))
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
            "by_tag" => {
                // GE category courses identified by the transcript code suffix
                // (e.g. 315-100G2A -> "G2A"); credits come from the transcript.
                let tags = strand.tags.as_deref().unwrap_or_default();
                let mut strand_credits = 0.0;
                for (idx, parsed) in courses.iter().enumerate() {
                    if strand_credits >= strand.required_credits {
                        break;
                    }
                    if !used_indices.contains(&idx)
                        && has_tag(parsed, tags)
                        && matches_curriculum_slot(parsed)
                    {
                        let matched_credits = earned_credits(parsed.parsed_credit, parsed);
                        completed_credits += matched_credits;
                        strand_credits += matched_credits;
                        used_indices.insert(idx);
                    }
                }
                if strand_credits < strand.required_credits {
                    missing_courses.push(MissingCourse {
                        category: "General Education".to_string(),
                        description: format!(
                            "{}: missing {:.1} credits",
                            strand.name,
                            strand.required_credits - strand_credits
                        ),
                    });
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

    // Tag-based GenEd electives (e.g. any "G8" course).
    for (idx, parsed) in courses.iter().enumerate() {
        if gen_ed_elective_total_credits >= curriculum.electives.total_required_credits {
            break;
        }
        if !used_indices.contains(&idx)
            && has_tag(parsed, &curriculum.electives.tags)
            && matches_curriculum_slot(parsed)
        {
            let matched_credits = earned_credits(parsed.parsed_credit, parsed);
            completed_credits += matched_credits;
            gen_ed_elective_total_credits += matched_credits;
            used_indices.insert(idx);
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

/// Result of `audit_major`.
pub struct MajorOutcome {
    /// Basic science + core + capstone credits.
    pub completed_credits: f32,
    /// All major-elective credits earned (may exceed `elective_required`).
    pub elective_credits: f32,
    /// Elective credits this plan needs: major total − basic − core − capstone.
    pub elective_required: f32,
    pub missing: Vec<MissingCourse>,
    pub used: HashSet<usize>,
}

/// Audits courses against the major curriculum, including greedy matching for
/// special-topics and other elective buckets. Credits are taken from curriculum
/// data to avoid PDF parsing drift. Indices in `exclude` (already placed in
/// GenEd) are never claimed.
pub fn audit_major(
    courses: &[ParsedCourse],
    curriculum: &MajorCurriculum,
    exclude: &HashSet<usize>,
) -> MajorOutcome {
    let mut completed_credits = 0.0;
    let mut elective_credits = 0.0;
    let mut missing_courses: Vec<MissingCourse> = Vec::new();
    let mut used_indices = exclude.clone();

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

    let mut capstone_taken: Option<&MajorCourse> = None;
    for option in &curriculum.capstone.options {
        if let Some((idx, parsed)) = courses.iter().enumerate().find(|(idx, parsed)| {
            !used_indices.contains(idx)
                && parsed.code == option.code
                && matches_curriculum_slot(parsed)
        }) {
            let matched_credits = earned_credits(option.credits, parsed);
            completed_credits += matched_credits;
            used_indices.insert(idx);
            capstone_taken = Some(option);

            break;
        }
    }

    // The plan follows the capstone taken; before that, assume the first option.
    let capstone_credits = capstone_taken
        .or(curriculum.capstone.options.first())
        .map_or(0.0, |o| o.credits);
    let elective_required = curriculum.total_required_credits
        - curriculum.basic_science.required_credits
        - curriculum.core_courses.required_credits
        - capstone_credits;

    for req in &curriculum.noncredit_required {
        if req
            .only_with
            .as_ref()
            .is_some_and(|code| capstone_taken.is_none_or(|t| &t.code != code))
        {
            continue;
        }
        // Earns no credit, but is claimed so it never counts as a free elective.
        if let Some(idx) = courses.iter().enumerate().position(|(idx, parsed)| {
            !used_indices.contains(&idx)
                && parsed.code == req.code
                && matches_curriculum_slot(parsed)
        }) {
            used_indices.insert(idx);
        } else {
            missing_courses.push(MissingCourse {
                category: "Core Courses".to_string(),
                description: format!("{} - {} (non-credit, must pass)", req.code, req.name),
            });
        }
    }

    if capstone_taken.is_none() && !curriculum.capstone.options.is_empty() {
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
    let mut best_domain_credits: f32 = 0.0;
    for domain in &curriculum.electives.domains {
        let mut domain_credits = 0.0;
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
                    domain_credits += matched_credits;
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
        best_domain_credits = best_domain_credits.max(domain_credits);
    }

    if let Some(min) = curriculum.electives.domain_min_credits {
        if best_domain_credits < min {
            missing_courses.push(MissingCourse {
                category: "Major Electives".to_string(),
                description: format!(
                    "At least {min:.0} elective credits within one specialisation domain (best so far: {best_domain_credits:.0})"
                ),
            });
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

    // Any other course from the programme's own prefix (e.g. "344-") is an elective.
    for (idx, parsed) in courses.iter().enumerate() {
        if !used_indices.contains(&idx)
            && curriculum
                .electives
                .prefixes
                .iter()
                .any(|p| parsed.code.starts_with(p.as_str()))
            && matches_curriculum_slot(parsed)
            && seen_others.insert(free_elective_dedupe_key(&parsed.code, &parsed.name))
        {
            elective_credits += earned_credits(parsed.parsed_credit, parsed);
            used_indices.insert(idx);
        }
    }

    used_indices.retain(|i| !exclude.contains(i));
    MajorOutcome {
        completed_credits,
        elective_credits,
        elective_required,
        missing: missing_courses,
        used: used_indices,
    }
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
    use crate::data::curriculum;

    fn cs64() -> &'static Curriculum {
        curriculum("cs2564")
    }

    fn parsed(code: &str, grade: &str, credit: f32) -> ParsedCourse {
        ParsedCourse {
            code: code.to_string(),
            name: code.to_string(),
            grade: grade.to_string(),
            parsed_credit: credit,
            semester: None,
            tag: None,
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
        let result = build_audit(
            &[parsed("322-101", "A", 3.0), parsed("322-102", "F", 3.0)],
            cs64(),
        );
        assert_eq!(result.gpa, Some(2.0));
        assert_eq!(
            result.attempts.len(),
            2,
            "failed attempts stay visible per semester"
        );
    }

    #[test]
    fn e_grade_does_not_fill_slot() {
        let result = build_audit(&[parsed("388-100", "E", 1.0)], cs64());
        assert_eq!(result.total_credits, 0.0);
        assert_eq!(result.gpa, Some(0.0));
    }

    #[test]
    fn retake_after_passing_counts_credits_once() {
        let result = build_audit(
            &[parsed("322-101", "D", 3.0), parsed("322-101", "B", 3.0)],
            cs64(),
        );
        assert_eq!(result.total_credits, 3.0);
        assert_eq!(category(&result, "Free Electives").collected_credits, 0.0);
        // Both attempts stay in GPA: (1.0 * 3 + 3.0 * 3) / 6.
        assert_eq!(result.gpa, Some(2.0));
    }

    #[test]
    fn repeated_non_topic_elective_counts_once() {
        // 344-493 Job Training is 2 credits in the 2564 TQF2.
        let result = build_audit(
            &[parsed("344-493", "A", 2.0), parsed("344-493", "B", 2.0)],
            cs64(),
        );
        assert_eq!(result.total_credits, 2.0);
        assert_eq!(category(&result, "Major Courses").courses.len(), 1);
    }

    #[test]
    fn repeatable_special_topics_still_accumulate() {
        let mut first = parsed("344-496", "A", 3.0);
        first.name = "Special Topics (Topic 1)".to_string();
        let mut second = parsed("344-496", "A", 3.0);
        second.name = "Special Topics (Topic 2)".to_string();
        let result = build_audit(&[first, second], cs64());
        assert_eq!(result.total_credits, 6.0);
    }

    #[test]
    fn gen_ed_electives_overflow_to_free_electives() {
        let gen_ed = cs64().gen_ed.clone();
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

        let result = build_audit(&electives, cs64());
        assert_eq!(
            category(&result, "General Education").collected_credits,
            6.0
        );
        assert_eq!(category(&result, "Free Electives").collected_credits, 4.0);
    }

    fn cs69() -> &'static Curriculum {
        curriculum("cs2569")
    }

    fn tagged(code: &str, grade: &str, credit: f32, tag: &str) -> ParsedCourse {
        ParsedCourse {
            tag: Some(tag.to_string()),
            ..parsed(code, grade, credit)
        }
    }

    fn missing_text(result: &AuditResult) -> String {
        result
            .missing_subjects
            .iter()
            .map(|m| format!("[{}] {}\n", m.category, m.description))
            .collect()
    }

    #[test]
    fn elective_target_follows_2564_plan() {
        // 96 - 12 basic - 48 core - capstone: project (3) -> 33, co-op (6) -> 30.
        let none = missing_text(&build_audit(&[], cs64()));
        assert!(none.contains("Electives: missing 33.0"), "{none}");
        let project = missing_text(&build_audit(&[parsed("344-492", "A", 3.0)], cs64()));
        assert!(project.contains("Electives: missing 33.0"), "{project}");
        let coop = missing_text(&build_audit(&[parsed("344-495", "S", 6.0)], cs64()));
        assert!(coop.contains("Electives: missing 30.0"), "{coop}");
    }

    #[test]
    fn pre_coop_required_only_on_coop_plan() {
        let coop = missing_text(&build_audit(&[parsed("344-495", "A", 6.0)], cs64()));
        assert!(coop.contains("344-494"), "{coop}");
        let project = missing_text(&build_audit(&[parsed("344-492", "A", 3.0)], cs64()));
        assert!(!project.contains("344-494"), "{project}");
    }

    #[test]
    fn pre_coop_earns_no_credit() {
        let result = build_audit(
            &[parsed("344-495", "A", 6.0), parsed("344-494", "S", 1.0)],
            cs64(),
        );
        assert_eq!(result.total_credits, 6.0);
        assert_eq!(category(&result, "Free Electives").collected_credits, 0.0);
        assert!(!missing_text(&result).contains("344-494"));
    }

    #[test]
    fn other_programme_courses_count_as_major_electives() {
        let result = build_audit(&[parsed("344-999", "A", 3.0)], cs64());
        assert_eq!(category(&result, "Major Courses").collected_credits, 3.0);
        assert_eq!(category(&result, "Free Electives").collected_credits, 0.0);
    }

    #[test]
    fn surplus_major_electives_become_free_electives() {
        // 12 x 3-credit electives = 36 against the 33 the project plan needs.
        let electives: Vec<ParsedCourse> = (0..12)
            .map(|i| parsed(&format!("344-9{i:02}"), "A", 3.0))
            .collect();
        let result = build_audit(&electives, cs64());
        assert_eq!(category(&result, "Major Courses").collected_credits, 33.0);
        assert_eq!(category(&result, "Free Electives").collected_credits, 3.0);
    }

    #[test]
    fn gpa_below_minimum_is_reported() {
        let low = build_audit(&[parsed("322-101", "D", 3.0)], cs64());
        assert!(has_missing(&low, "Graduation"));
        let ok = build_audit(&[parsed("322-101", "C", 3.0)], cs64());
        assert!(!has_missing(&ok, "Graduation"));
    }

    #[test]
    fn gen_ed_2569_counts_by_tag() {
        let courses = [
            tagged("890-103", "A", 2.0, "G1"),
            tagged("890-104", "A", 2.0, "G1"),
            tagged("315-100", "A", 2.0, "G2A"),
            tagged("315-202", "B", 2.0, "G2B"),
            tagged("193-031", "A", 2.0, "G8"),
            tagged("895-881", "A", 2.0, "G8"),
            tagged("890-831", "A", 2.0, "G8"),
            tagged("820-100", "A", 2.0, "G8"),
        ];
        let result = build_audit(&courses, cs69());
        // 4 (G1) + 2 (G2A) + 2 (G2B) + 6 (G8, capped) = 14; the 4th G8 overflows.
        assert_eq!(
            category(&result, "General Education").collected_credits,
            14.0
        );
        assert_eq!(category(&result, "Free Electives").collected_credits, 2.0);
        let text = missing_text(&result);
        assert!(
            text.contains("GE3 Entrepreneurial Mindset: missing 2.0"),
            "{text}"
        );
        assert!(!text.contains("GE1"), "{text}");
    }

    #[test]
    fn domain_rule_2569_needs_12_credits_in_one_domain() {
        let split = build_audit(
            &[
                parsed("344-214", "A", 3.0),
                parsed("344-215", "A", 3.0),
                parsed("344-252", "A", 3.0),
                parsed("344-351", "A", 3.0),
            ],
            cs69(),
        );
        assert!(missing_text(&split).contains("within one specialisation domain"));
        let focused = build_audit(
            &[
                parsed("344-214", "A", 3.0),
                parsed("344-215", "A", 3.0),
                parsed("344-311", "A", 3.0),
                parsed("344-342", "A", 3.0),
            ],
            cs69(),
        );
        assert!(!missing_text(&focused).contains("within one specialisation domain"));
    }

    #[test]
    fn curriculum_2569_targets() {
        let text = missing_text(&build_audit(&[], cs69()));
        assert!(text.contains("Electives: missing 27.0"), "{text}");
        assert!(
            text.contains("344-494"),
            "pre-co-op is required in 2569: {text}"
        );
        assert!(
            !text.contains("Choose 1:"),
            "2569 has no capstone choice: {text}"
        );
    }

    #[test]
    fn missing_reports_major_and_free_elective_credit_shortfall() {
        let result = build_audit(&[], cs64());
        assert!(has_missing(&result, "Major Courses"));
        assert!(has_missing(&result, "Major Electives"));
        assert!(has_missing(&result, "Free Electives"));

        let result = build_audit(
            &[parsed("999-001", "A", 3.0), parsed("999-002", "A", 3.0)],
            cs64(),
        );
        assert!(!has_missing(&result, "Free Electives"));
    }

    #[test]
    fn ip_course_fills_slot_but_adds_zero_credits() {
        // 388-100 "Health for All" is a required course in Strand 1.
        let courses = vec![parsed("388-100", "IP", 1.0)];
        let gen_ed = cs64().gen_ed.clone();
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
        let gen_ed = cs64().gen_ed.clone();
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
        let gen_ed = cs64().gen_ed.clone();
        let (credits, missing, used) = audit_gen_ed(&courses, &gen_ed);

        assert!(!used.contains(&0), "F grade must not fill slot");
        assert_eq!(credits, 0.0);
        assert!(
            missing.iter().any(|m| m.description.contains("388-100")),
            "F'd course's slot should still be listed as missing"
        );
    }
}
