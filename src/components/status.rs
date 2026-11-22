//! Plain-language status of an audit result, and Thai labels for category
//! and group names. Pure functions so they can be unit-tested natively.

use audit_core::models::AuditResult;

/// Credits without a trailing ".0": 9.0 -> "9", 1.5 -> "1.5".
pub fn fmt_credits(c: f32) -> String {
    let rounded = (c * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

/// Display name of an `audit-core` category or group name.
pub fn label(name: &str, is_thai: bool) -> String {
    let (th, en) = match name {
        "General Education" => ("หมวดศึกษาทั่วไป", "General education"),
        "Major Courses" => ("หมวดวิชาเฉพาะ", "Major courses"),
        "Free Electives" => ("หมวดเลือกเสรี", "Free electives"),
        "Basic Science" => ("วิทยาศาสตร์พื้นฐาน", "Basic science"),
        "Core Courses" => ("วิชาแกน", "Core courses"),
        "Capstone" => ("โครงงานหรือสหกิจ", "Project or co-op"),
        "Major Electives" => ("วิชาเลือกเฉพาะ", "Major electives"),
        "Graduation" => ("เงื่อนไขการจบ", "Graduation"),
        other => return other.to_string(),
    };
    if is_thai { th } else { en }.to_string()
}

/// (curriculum met, headline, overflow note).
pub fn status_line(result: &AuditResult, is_thai: bool) -> (bool, String, Option<String>) {
    let required = result.required_credits();
    let counted = result.counted_credits();
    let short = (required - counted).max(0.0);
    let complete = short == 0.0 && result.missing_subjects.is_empty();
    let head = match (complete, short > 0.0, is_thai) {
        (true, _, true) => "ครบตามหลักสูตรแล้ว".to_string(),
        (true, _, false) => "Curriculum requirements met".to_string(),
        (false, true, true) => format!("ยังไม่ครบตามหลักสูตร เหลืออีก {} หน่วยกิต", fmt_credits(short)),
        (false, true, false) => format!("Not complete yet: {} credits to go", fmt_credits(short)),
        (false, false, true) => format!(
            "ยังไม่ครบตามหลักสูตร เหลืออีก {} รายการ",
            result.missing_subjects.len()
        ),
        (false, false, false) => {
            let n = result.missing_subjects.len();
            format!(
                "Not complete yet: {n} requirement{} left",
                if n == 1 { "" } else { "s" }
            )
        }
    };
    let earned = result.earned_credits();
    let overflow: f32 = result.categories.iter().map(|c| c.overflow_credits()).sum();
    let note = (!complete && earned > required && overflow > 0.0).then(|| {
        let (e, r, o) = (fmt_credits(earned), fmt_credits(required), fmt_credits(overflow));
        if is_thai {
            format!("หน่วยกิตรวม {e} เกินเกณฑ์ {r} แล้ว แต่ {o} หน่วยกิตที่เกินในหมวดเลือกเสรีนำมานับแทนหมวดอื่นไม่ได้")
        } else {
            format!("Total {e} exceeds {r}, but the {o} extra free-elective credits can't count toward other categories")
        }
    });
    (complete, head, note)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_core::models::{Category, MissingCourse};

    fn cat(name: &str, required: f32, collected: f32) -> Category {
        Category {
            name: name.to_string(),
            required_credits: required,
            collected_credits: collected,
            courses: vec![],
            groups: vec![],
        }
    }

    fn result(cats: Vec<Category>, missing: Vec<MissingCourse>, total: f32) -> AuditResult {
        AuditResult {
            total_credits: total,
            categories: cats,
            missing_subjects: missing,
            gpa: Some(3.0),
            attempts: vec![],
        }
    }

    fn sample() -> AuditResult {
        result(
            vec![
                cat("General Education", 30.0, 30.0),
                cat("Major Courses", 96.0, 87.0),
                cat("Free Electives", 6.0, 21.0),
            ],
            vec![MissingCourse {
                category: "Major Electives".into(),
                description: "x".into(),
                credits: Some(9.0),
                ..Default::default()
            }],
            138.0,
        )
    }

    fn done() -> AuditResult {
        result(
            vec![
                cat("General Education", 30.0, 30.0),
                cat("Major Courses", 96.0, 96.0),
                cat("Free Electives", 6.0, 6.0),
            ],
            vec![],
            132.0,
        )
    }

    fn low_gpa() -> AuditResult {
        let mut r = done();
        r.missing_subjects.push(MissingCourse {
            category: "Graduation".into(),
            description: "Cumulative GPA 1.90 is below the minimum 2.00".into(),
            ..Default::default()
        });
        r
    }

    #[test]
    fn incomplete_with_overflow() {
        let (ok, head, note) = status_line(&sample(), true);
        assert!(!ok);
        assert_eq!(head, "ยังไม่ครบตามหลักสูตร เหลืออีก 9 หน่วยกิต");
        assert!(note.unwrap().contains("15"));
    }

    #[test]
    fn complete() {
        let (ok, head, note) = status_line(&done(), false);
        assert!(ok);
        assert_eq!(head, "Curriculum requirements met");
        assert!(note.is_none());
    }

    #[test]
    fn gpa_below_minimum_is_incomplete() {
        let (ok, head, _) = status_line(&low_gpa(), false);
        assert!(!ok);
        assert_eq!(head, "Not complete yet: 1 requirement left");
    }

    #[test]
    fn credits_format_without_trailing_zero() {
        assert_eq!(fmt_credits(9.0), "9");
        assert_eq!(fmt_credits(1.5), "1.5");
    }

    #[test]
    fn thai_labels_for_known_names() {
        assert_eq!(label("Major Courses", true), "หมวดวิชาเฉพาะ");
        assert_eq!(label("Capstone", true), "โครงงานหรือสหกิจ");
        assert_eq!(label("Major Courses", false), "Major courses");
        assert_eq!(label("Unknown thing", true), "Unknown thing");
    }
}
