//! Core data structures for the Course Audit System
//!
//! This module defines all data types used throughout the application:
//! - `Course`: Individual course with grade and credit info
//! - `Category`: Top-level audit category (GenEd, Major, Electives)
//! - `AuditResult`: Final audit result with all categories and missing courses
//! - Curriculum types: `GenEdCurriculum`, `MajorCurriculum` for static curriculum data

use serde::{Deserialize, Serialize};

/// Represents a single course instance in the transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub code: String,  // Course code (e.g., "344-101")
    pub name: String,  // Course name
    pub credit: f32,   // Credits earned
    pub grade: String, // Letter grade (A, B, C, etc.)
    /// Semester label from the transcript (e.g. "1/2567") or `None` for
    /// planned/unknown. Not persisted by older save payloads — deserialization
    /// falls back to None.
    #[serde(default)]
    pub semester: Option<String>,
}

/// Aggregates courses within a displayable category (e.g., General Education, Major)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,           // Category name
    pub required_credits: f32,  // Total credits required
    pub collected_credits: f32, // Credits earned so far
    pub courses: Vec<Course>,   // Courses in this category
    /// Sub-requirements shown under the category (filled for the major only).
    #[serde(default)]
    pub groups: Vec<CategoryGroup>,
}

/// One sub-requirement of a category, e.g. the major's "Core Courses".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryGroup {
    pub name: String,
    pub required_credits: f32,
    pub collected_credits: f32,
}

impl Category {
    /// Credits that count toward the curriculum: collected, capped at required.
    pub fn counted_credits(&self) -> f32 {
        self.collected_credits.min(self.required_credits)
    }

    /// Credits collected beyond the requirement.
    pub fn overflow_credits(&self) -> f32 {
        (self.collected_credits - self.required_credits).max(0.0)
    }

    pub fn is_complete(&self) -> bool {
        self.collected_credits >= self.required_credits
    }
}

impl AuditResult {
    /// Sum of each category's counted credits.
    pub fn counted_credits(&self) -> f32 {
        self.categories.iter().map(Category::counted_credits).sum()
    }

    /// Sum of each category's required credits.
    pub fn required_credits(&self) -> f32 {
        self.categories.iter().map(|c| c.required_credits).sum()
    }

    /// All credits earned, overflow included.
    pub fn earned_credits(&self) -> f32 {
        self.total_credits
    }
}

/// A single missing required course, tagged with its curriculum category
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MissingCourse {
    pub category: String,    // e.g. "General Education", "Major Courses"
    pub description: String, // e.g. "344-101 - Calculus I"
    /// The one course this item asks for, when it names one.
    #[serde(default)]
    pub code: Option<String>,
    /// Credits still needed (a course's credits, or a credit gap).
    #[serde(default)]
    pub credits: Option<f32>,
}

/// Final audit result containing all categories and missing requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub total_credits: f32,                   // Total credits earned
    pub categories: Vec<Category>,            // All audit categories (GenEd, Major, Electives)
    pub missing_subjects: Vec<MissingCourse>, // Missing courses with their category
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpa: Option<f32>, // Overall GPA across graded courses (None if no grades)
    #[serde(default)]
    pub attempts: Vec<Course>, // Every transcript entry, incl. failed and retaken attempts
}

/// A single General Education course.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdCourse {
    pub code: String,
    pub name: String,
    pub credits: f32,
}

/// A nested sub-group under a GenEd strand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdSubGroup {
    pub name: String,
    pub required_credits: f32,
    pub courses: Vec<GenEdCourse>,
}

/// A GenEd strand which may contain direct courses or sub-groups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdStrand {
    pub id: u32,
    pub name: String,
    pub required_credits: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_groups: Option<Vec<GenEdSubGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub courses: Option<Vec<GenEdCourse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_groups: Option<Vec<Vec<String>>>,
    /// For `selection_rule: "by_tag"`: GE category tags (e.g. "G2A") whose
    /// courses fill this strand, read from the transcript code suffix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Elective sub-category within GenEd (e.g., language electives).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdElectiveSubCategory {
    pub name: String,
    pub required_credits: f32,
    pub min_courses: u32,
    pub max_courses: u32,
    pub courses: Vec<GenEdCourse>,
}

/// Collects GenEd electives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdElectives {
    pub name: String,
    pub total_required_credits: f32,
    pub sub_categories: Vec<GenEdElectiveSubCategory>,
    /// GE category tags (e.g. "G8") that also count as GenEd electives.
    #[serde(default)]
    pub tags: Vec<String>,
    /// At least `language_min_credits` of the tagged electives must be
    /// language courses, recognised by these code prefixes (2569: 2 credits).
    #[serde(default)]
    pub language_prefixes: Vec<String>,
    #[serde(default)]
    pub language_min_credits: f32,
}

/// Top-level General Education curriculum definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenEdCurriculum {
    pub name: String,
    pub total_required_credits: f32,
    pub strands: Vec<GenEdStrand>,
    pub electives: GenEdElectives,
}

/// A course that belongs to the major curriculum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorCourse {
    pub code: String,
    pub name: String,
    pub credits: f32,
}

/// Cluster of courses inside a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorCluster {
    pub id: String,
    pub name: String,
    pub min_courses: u32,
    pub description: Option<String>,
    pub courses: Vec<MajorCourse>,
    /// Courses that must be among those taken for the cluster to count
    /// (e.g. 2564 cluster 4.1: 344-261 and 344-362, plus one more).
    #[serde(default)]
    pub required: Vec<String>,
}

/// Domain grouping clusters of major electives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorDomain {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub clusters: Vec<MajorCluster>,
}

/// Basic science portion of the major.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorBasicScience {
    pub name: String,
    pub required_credits: f32,
    pub courses: Vec<MajorCourse>,
}

/// Core courses required for the major.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorCoreCourses {
    pub name: String,
    pub required_credits: f32,
    pub courses: Vec<MajorCourse>,
}

/// Capstone options for the major (project or co-op).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorCapstone {
    pub name: String,
    pub credits_per_option: f32,
    pub options: Vec<MajorCourse>,
}

/// Elective requirements, including domains and other choices. The credit
/// target is derived: major total − basic science − core − chosen capstone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorElectives {
    pub name: String,
    /// 2564 rule: complete every course of this many clusters.
    pub clusters_to_complete: u32,
    /// 2569 rule: at least this many elective credits inside one domain.
    #[serde(default)]
    pub domain_min_credits: Option<f32>,
    pub domains: Vec<MajorDomain>,
    pub others: Vec<MajorCourse>,
    /// Any other passing course with one of these code prefixes also counts
    /// as a major elective (e.g. "344-").
    #[serde(default)]
    pub prefixes: Vec<String>,
}

/// A course that must be passed but earns no credit (e.g. pre-co-op, graded S/U).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoncreditRequirement {
    pub code: String,
    pub name: String,
    /// Only required when this capstone option was taken (e.g. co-op plan).
    #[serde(default)]
    pub only_with: Option<String>,
}

/// Top-level Major curriculum definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorCurriculum {
    pub name: String,
    pub total_required_credits: f32,
    pub basic_science: MajorBasicScience,
    pub core_courses: MajorCoreCourses,
    pub capstone: MajorCapstone,
    pub electives: MajorElectives,
    #[serde(default)]
    pub noncredit_required: Vec<NoncreditRequirement>,
}

/// One curriculum revision of the programme (e.g. หลักสูตรปรับปรุง 2564).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curriculum {
    pub id: String,
    pub name: String,
    /// First admission cohort (Buddhist year) that uses this revision.
    pub first_cohort: u32,
    pub free_elective_credits: f32,
    pub min_gpa: f32,
    pub gen_ed: GenEdCurriculum,
    pub major: MajorCurriculum,
    /// Prerequisites by course code, transcribed from the TQF2 course descriptions.
    #[serde(default)]
    pub prerequisites: std::collections::BTreeMap<String, Prerequisite>,
}

/// "รายวิชาบังคับเรียนก่อน": the listed courses must have been taken and
/// graded before (any grade), unless the programme consents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub codes: Vec<String>,
    /// `true` when any one of `codes` is enough ("A or B"); otherwise all.
    pub any_of: bool,
    /// Original wording, e.g. "344-111 ... or with the consent of the program".
    pub text: String,
}

/// Parsed course details extracted from the transcript text. Serializable so
/// the UI can persist the transcript (opt-in) and re-audit it after a reload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCourse {
    pub code: String,
    pub name: String,
    pub grade: String,
    pub parsed_credit: f32,
    /// Semester label scraped from the PDF, e.g. "1/2567". `None` for courses
    /// that appeared outside any "ภาคการศึกษา X/YYYY" header, or for
    /// user-added planned courses.
    #[serde(default)]
    pub semester: Option<String>,
    /// GE category tag from the code suffix or registration category column
    /// (e.g. "G2A" from "315-100G2A"); `None` for non-GE courses.
    #[serde(default)]
    pub tag: Option<String>,
}

/// Which PDF format was detected by the parser. Drives UI messaging: in
/// `Registration` mode, in-progress courses fill curriculum slots but do not
/// contribute credits toward the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Grade,
    Registration,
}

// ── Shared utility functions ────────────────────────────────────────────────

/// Sentinel grade for courses from a registration report (no grade issued yet).
pub const IN_PROGRESS_GRADE: &str = "IP";

/// Returns `true` when the grade earns accumulated credit.
///
/// Passing letter grades are D (1.00) and above. Symbols G, P and S pass
/// without grade points, as do credit-bank transfers CS, CE, CT and CP.
/// E/F, U, W, I, R, AU and IP do not earn credit.
pub fn is_passing_grade(grade: &str) -> bool {
    if is_in_progress(grade) {
        return false;
    }
    let g = grade.trim().to_ascii_uppercase();
    matches!(
        g.as_str(),
        "A" | "B+" | "B" | "C+" | "C" | "D+" | "D" | "G" | "P" | "S" | "CS" | "CE" | "CT" | "CP"
    )
}

/// Returns `true` for the sentinel "IP" (In Progress) grade.
pub fn is_in_progress(grade: &str) -> bool {
    grade.trim().eq_ignore_ascii_case(IN_PROGRESS_GRADE)
}

/// Maps a PSU letter grade to a grade point on the 4.0 scale.
/// E and F are 0. Non-letter results (W, I, R, P, S, U, G, IP, AU, and
/// transfer symbols CS/CE/CT/CP) return `None` and are excluded from the GPA.
pub fn grade_points(grade: &str) -> Option<f32> {
    match grade.trim().to_ascii_uppercase().as_str() {
        "A" => Some(4.0),
        "B+" => Some(3.5),
        "B" => Some(3.0),
        "C+" => Some(2.5),
        "C" => Some(2.0),
        "D+" => Some(1.5),
        "D" => Some(1.0),
        "E" | "F" => Some(0.0),
        _ => None,
    }
}

/// Cumulative GPA. A course repeated after D+, D, E or F contributes only its
/// latest result (ข้อบังคับ 2563 ข้อ 38). Semester GPA keeps every attempt in
/// that term; call [`compute_gpa`] for that.
pub fn cumulative_gpa(courses: &[Course]) -> Option<f32> {
    compute_gpa(&latest_attempt_when_repeated_low(courses))
}

/// D+, D, E and F are the grades a later attempt replaces in the cumulative GPA.
fn is_replaced_on_repeat(grade: &str) -> bool {
    matches!(
        grade.trim().to_ascii_uppercase().as_str(),
        "D+" | "D" | "E" | "F"
    )
}

/// W, I, R and IP are not a finished result, so they neither replace an earlier
/// grade nor count as the "latest" attempt.
fn is_finished_result(grade: &str) -> bool {
    let g = grade.trim();
    !g.is_empty()
        && !is_in_progress(g)
        && !matches!(g.to_ascii_uppercase().as_str(), "W" | "I" | "R")
}

fn latest_attempt_when_repeated_low(courses: &[Course]) -> Vec<Course> {
    use std::collections::{HashMap, HashSet};

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, course) in courses.iter().enumerate() {
        if !is_finished_result(&course.grade) {
            continue;
        }
        groups
            .entry(free_elective_dedupe_key(&course.code, &course.name))
            .or_default()
            .push(i);
    }
    let mut drop = HashSet::new();
    for idxs in groups.values() {
        let repeated_low = idxs.len() >= 2
            && idxs
                .iter()
                .any(|&i| is_replaced_on_repeat(&courses[i].grade));
        if repeated_low {
            for &i in &idxs[..idxs.len() - 1] {
                drop.insert(i);
            }
        }
    }
    courses
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop.contains(i))
        .map(|(_, course)| course.clone())
        .collect()
}

/// GPA = Σ(grade_points × credits) / Σ(credits) over courses with letter
/// grades only. Returns `None` when no graded credits are present.
pub fn compute_gpa(courses: &[Course]) -> Option<f32> {
    let mut total_points = 0.0;
    let mut total_credits = 0.0;
    for c in courses {
        if let Some(points) = grade_points(&c.grade) {
            total_points += points * c.credit;
            total_credits += c.credit;
        }
    }
    if total_credits == 0.0 {
        None
    } else {
        Some(total_points / total_credits)
    }
}

/// Builds a deduplication key for a course so that repeatable special-topic
/// courses are keyed by code **and** name, while all other courses are keyed
/// by code alone. Special topics are 344-496…499 in the 2564 curriculum and
/// 344-495…498 in 2569; 344-495 (co-op in 2564) is safe to include because
/// retakes of it share one name and so still collapse to one key.
pub fn free_elective_dedupe_key(code: &str, name: &str) -> String {
    let is_repeatable_special_topic = code.starts_with("344-49")
        && !matches!(code, "344-491" | "344-492" | "344-493" | "344-494");

    if is_repeatable_special_topic {
        format!("{}::{}", code, name)
    } else {
        code.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course(grade: &str, credit: f32) -> Course {
        Course {
            code: "TEST".to_string(),
            name: "TEST".to_string(),
            credit,
            grade: grade.to_string(),
            semester: None,
        }
    }

    fn cat(required: f32, collected: f32) -> Category {
        Category {
            name: "X".to_string(),
            required_credits: required,
            collected_credits: collected,
            courses: vec![],
            groups: vec![],
        }
    }

    #[test]
    fn counted_credits_caps_each_category() {
        let r = AuditResult {
            total_credits: 138.0,
            categories: vec![cat(30.0, 30.0), cat(96.0, 87.0), cat(6.0, 21.0)],
            missing_subjects: vec![],
            gpa: Some(3.81),
            attempts: vec![],
        };
        assert_eq!(r.counted_credits(), 123.0);
        assert_eq!(r.required_credits(), 132.0);
        assert_eq!(r.earned_credits(), 138.0);
        assert_eq!(r.categories[2].overflow_credits(), 15.0);
        assert_eq!(r.categories[1].overflow_credits(), 0.0);
        assert!(!r.categories[1].is_complete());
        assert!(r.categories[2].is_complete());
    }

    #[test]
    fn old_saved_result_without_groups_loads() {
        let json = r#"{"total_credits":3.0,"categories":[{"name":"Major Courses","required_credits":96.0,"collected_credits":3.0,"courses":[]}],"missing_subjects":[{"category":"Core Courses","description":"344-111 - X"}],"attempts":[]}"#;
        let r: AuditResult = serde_json::from_str(json).unwrap();
        assert!(r.categories[0].groups.is_empty());
        assert_eq!(r.gpa, None);
    }

    #[test]
    fn grade_points_maps_letter_grades() {
        assert_eq!(grade_points("A"), Some(4.0));
        assert_eq!(grade_points("B+"), Some(3.5));
        assert_eq!(grade_points("C"), Some(2.0));
        assert_eq!(grade_points("F"), Some(0.0));
        assert_eq!(grade_points("E"), Some(0.0));
        assert_eq!(grade_points(" a "), Some(4.0));
    }

    #[test]
    fn grade_points_rejects_non_letter_grades() {
        assert_eq!(grade_points("W"), None);
        assert_eq!(grade_points("P"), None);
        assert_eq!(grade_points("G"), None);
        assert_eq!(grade_points("S"), None);
        assert_eq!(grade_points("U"), None);
        assert_eq!(grade_points("CS"), None);
        assert_eq!(grade_points("CE"), None);
        assert_eq!(grade_points("CT"), None);
        assert_eq!(grade_points("CP"), None);
        assert_eq!(grade_points("IP"), None);
        assert_eq!(grade_points(""), None);
    }

    #[test]
    fn passing_grades_match_the_2563_regulation() {
        for grade in [
            "A", "B+", "C", "D+", "D", "S", "G", "P", "CS", "CE", "CT", "CP",
        ] {
            assert!(is_passing_grade(grade), "{grade} earns credit");
        }
        for grade in ["E", "F", "U", "W", "I", "R", "AU", "IP", ""] {
            assert!(!is_passing_grade(grade), "{grade} earns no credit");
        }
    }

    #[test]
    fn cumulative_gpa_keeps_only_the_latest_repeat_of_a_low_grade() {
        let repeated = vec![
            Course {
                code: "322-101".into(),
                name: "Calculus".into(),
                credit: 3.0,
                grade: "D+".into(),
                semester: Some("1/2567".into()),
            },
            Course {
                code: "322-101".into(),
                name: "Calculus".into(),
                credit: 3.0,
                grade: "B".into(),
                semester: Some("2/2567".into()),
            },
        ];
        // Latest only: 3.0, not (1.5 + 3.0) / 2.
        assert_eq!(cumulative_gpa(&repeated), Some(3.0));
        // A fail that was never repeated still counts.
        let once = vec![
            Course {
                code: "322-102".into(),
                name: "Calculus II".into(),
                credit: 3.0,
                grade: "A".into(),
                semester: None,
            },
            Course {
                code: "344-111".into(),
                name: "Programming".into(),
                credit: 3.0,
                grade: "E".into(),
                semester: None,
            },
        ];
        assert_eq!(cumulative_gpa(&once), Some(2.0));
    }

    #[test]
    fn compute_gpa_weights_by_credits() {
        // A (4.0) × 3cr + B (3.0) × 1cr = 15.0 / 4cr = 3.75
        let courses = vec![course("A", 3.0), course("B", 1.0)];
        assert_eq!(compute_gpa(&courses), Some(3.75));
    }

    #[test]
    fn compute_gpa_ignores_non_letter_grades() {
        // Only A × 3cr counts. W and IP excluded entirely.
        let courses = vec![course("A", 3.0), course("W", 3.0), course("IP", 2.0)];
        assert_eq!(compute_gpa(&courses), Some(4.0));
    }

    #[test]
    fn compute_gpa_returns_none_when_no_graded_courses() {
        let courses = vec![course("IP", 3.0), course("W", 3.0)];
        assert_eq!(compute_gpa(&courses), None);
    }

    #[test]
    fn free_elective_dedupe_key_regular_courses_use_code_only() {
        // Regular courses collapse to code alone so retakes dedupe.
        assert_eq!(
            free_elective_dedupe_key("344-111", "Programming"),
            "344-111"
        );
        assert_eq!(free_elective_dedupe_key("322-101", "Calculus I"), "322-101");
        // The four explicit exclusions: 344-491 … 344-494 return code only.
        assert_eq!(
            free_elective_dedupe_key("344-491", "Senior Project"),
            "344-491"
        );
        assert_eq!(free_elective_dedupe_key("344-492", "Project"), "344-492");
        assert_eq!(
            free_elective_dedupe_key("344-493", "Job Training"),
            "344-493"
        );
        assert_eq!(
            free_elective_dedupe_key("344-494", "Co-op Preparation"),
            "344-494"
        );
    }

    #[test]
    fn free_elective_dedupe_key_344_49x_excluding_491_to_494_are_topics() {
        // Everything matching 344-49* except 491–494 is treated as a special
        // topic and keyed by code + name so different topics accumulate.
        assert!(free_elective_dedupe_key("344-490", "X").contains("::"));
        assert!(free_elective_dedupe_key("344-495", "Y").contains("::"));
        assert!(free_elective_dedupe_key("344-496", "Z").contains("::"));
        assert!(free_elective_dedupe_key("344-499", "W").contains("::"));
    }

    #[test]
    fn free_elective_dedupe_key_special_topics_differ_by_name() {
        let t1 = "Special Topics (Topic 1)";
        let t2 = "Special Topics (Topic 2)";
        assert_eq!(
            free_elective_dedupe_key("344-496", t1),
            "344-496::Special Topics (Topic 1)"
        );
        assert_eq!(
            free_elective_dedupe_key("344-496", t2),
            "344-496::Special Topics (Topic 2)"
        );
        // Same code + same name → same key (dedupe).
        assert_eq!(
            free_elective_dedupe_key("344-496", t1),
            free_elective_dedupe_key("344-496", t1)
        );
        assert!(free_elective_dedupe_key("344-497", "Topic").starts_with("344-497::"));
        assert!(free_elective_dedupe_key("344-498", "Topic").starts_with("344-498::"));
        assert!(free_elective_dedupe_key("344-499", "Topic").starts_with("344-499::"));
    }
}
