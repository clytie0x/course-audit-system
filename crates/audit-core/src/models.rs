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
}

/// A single missing required course, tagged with its curriculum category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingCourse {
    pub category: String,    // e.g. "General Education", "Major Courses"
    pub description: String, // e.g. "344-101 - Calculus I"
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
}

/// Parsed course details extracted from the transcript text.
#[derive(Debug, Clone)]
pub struct ParsedCourse {
    pub code: String,
    pub name: String,
    pub grade: String,
    pub parsed_credit: f32,
    /// Semester label scraped from the PDF, e.g. "1/2567". `None` for courses
    /// that appeared outside any "ภาคการศึกษา X/YYYY" header, or for
    /// user-added planned courses.
    pub semester: Option<String>,
    /// GE category tag from the code suffix or registration category column
    /// (e.g. "G2A" from "315-100G2A"); `None` for non-GE courses.
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

/// Returns `true` when the grade represents a passing result.
/// Failing markers: E and F (fail), W (withdraw), U (unsatisfactory).
/// In-progress ("IP") courses are not yet passing.
pub fn is_passing_grade(grade: &str) -> bool {
    if is_in_progress(grade) {
        return false;
    }
    grade
        .trim()
        .chars()
        .next()
        .map(|c| !matches!(c.to_ascii_uppercase(), 'E' | 'F' | 'W' | 'U'))
        .unwrap_or(false)
}

/// Returns `true` for the sentinel "IP" (In Progress) grade.
pub fn is_in_progress(grade: &str) -> bool {
    grade.trim().eq_ignore_ascii_case(IN_PROGRESS_GRADE)
}

/// Maps a PSU letter grade to a grade point on the 4.0 scale.
/// Non-letter grades (W, P, S, U, G, E, IP) return `None` and are excluded
/// from GPA calculation.
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
        assert_eq!(grade_points("IP"), None);
        assert_eq!(grade_points(""), None);
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
}
