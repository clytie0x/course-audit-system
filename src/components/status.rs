//! Plain-language status of an audit result, and Thai labels for category
//! and group names. Pure functions so they can be unit-tested natively.

use audit_core::models::{AuditResult, Course, MissingCourse};

/// Attempts grouped by semester label, ordered by year then term. Results
/// saved before `attempts` existed fall back to the category course lists.
pub fn by_semester(result: &AuditResult) -> Vec<(String, Vec<Course>)> {
    let source: Vec<&Course> = if result.attempts.is_empty() {
        result
            .categories
            .iter()
            .flat_map(|c| c.courses.iter())
            .collect()
    } else {
        result.attempts.iter().collect()
    };
    let mut by_sem: Vec<(String, Vec<Course>)> = Vec::new();
    for course in source {
        if let Some(sem) = &course.semester {
            if let Some(bucket) = by_sem.iter_mut().find(|(s, _)| s == sem) {
                bucket.1.push(course.clone());
            } else {
                by_sem.push((sem.clone(), vec![course.clone()]));
            }
        }
    }
    let key = |s: &str| -> (u32, u32) {
        let mut parts = s.split('/');
        let term = parts.next().and_then(|t| t.parse().ok()).unwrap_or(0);
        let year = parts.next().and_then(|y| y.parse().ok()).unwrap_or(0);
        (year, term)
    };
    by_sem.sort_by_key(|(s, _)| key(s));
    by_sem
}

/// One row of "What to do next".
#[derive(Debug, Clone, PartialEq)]
pub enum Todo {
    Course {
        code: String,
        name: String,
        category: String,
        credits: f32,
    },
    Gap {
        category: String,
        credits: f32,
    },
    Rule {
        category: String,
        description: String,
    },
}

/// Missing items as courses first, then credit gaps, then other rules.
/// "Overall …" totals are skipped: the rows above them already add up to it.
pub fn todo_rows(missing: &[MissingCourse]) -> Vec<Todo> {
    let mut courses = Vec::new();
    let mut gaps = Vec::new();
    let mut rules = Vec::new();
    for m in missing {
        // Results saved before `code`/`credits` existed carry them only in the text.
        let code = m.code.clone().or_else(|| code_in(&m.description));
        let credits = m.credits.or_else(|| missing_credits_in(&m.description));
        match (code, credits) {
            _ if m.description.starts_with("Overall ") => {}
            (Some(code), credits) => courses.push(Todo::Course {
                code,
                name: m
                    .description
                    .split_once(" - ")
                    .map_or(m.description.clone(), |(_, n)| n.to_string()),
                category: m.category.clone(),
                credits: credits.unwrap_or(0.0),
            }),
            (None, Some(credits)) => gaps.push(Todo::Gap {
                category: m.category.clone(),
                credits,
            }),
            (None, None) => rules.push(Todo::Rule {
                category: m.category.clone(),
                description: m.description.clone(),
            }),
        }
    }
    courses.into_iter().chain(gaps).chain(rules).collect()
}

/// "344-223 - Name" or "Strand: 344-223 - Name" -> "344-223".
fn code_in(description: &str) -> Option<String> {
    let (left, _) = description.split_once(" - ")?;
    let code = left.rsplit(' ').next()?;
    let b = code.as_bytes();
    let ok = b.len() == 7
        && b[3] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 3 || c.is_ascii_digit());
    ok.then(|| code.to_string())
}

/// "...: missing 3.0 credits" -> 3.0.
fn missing_credits_in(description: &str) -> Option<f32> {
    let rest = description.rsplit_once("missing ")?.1;
    rest.strip_suffix(" credits")?.parse().ok()
}

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

    fn attempt(code: &str, sem: Option<&str>) -> audit_core::models::Course {
        audit_core::models::Course {
            code: code.into(),
            name: code.into(),
            credit: 3.0,
            grade: "A".into(),
            semester: sem.map(Into::into),
        }
    }

    #[test]
    fn semesters_group_and_sort_by_year_then_term() {
        let mut r = done();
        r.attempts = vec![
            attempt("a", Some("1/2568")),
            attempt("b", Some("2/2567")),
            attempt("c", Some("1/2567")),
            attempt("d", Some("2/2567")),
            attempt("e", None),
        ];
        let sems = by_semester(&r);
        let labels: Vec<_> = sems.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(labels, ["1/2567", "2/2567", "1/2568"]);
        assert_eq!(sems[1].1.len(), 2);
    }

    #[test]
    fn todo_rows_sort_courses_gaps_rules_and_skip_overall() {
        let m = |category: &str, description: &str, code: Option<&str>, credits: Option<f32>| {
            MissingCourse {
                category: category.into(),
                description: description.into(),
                code: code.map(Into::into),
                credits,
            }
        };
        let rows = todo_rows(&[
            m("Capstone", "Choose 1: 344-492 OR 344-495", None, None),
            m(
                "Major Electives",
                "Major Electives: missing 3.0 credits",
                None,
                Some(3.0),
            ),
            m(
                "Major Courses",
                "Overall Major Courses: missing 9.0 credits",
                None,
                Some(9.0),
            ),
            m(
                "Core Courses",
                "344-223 - Fundamentals of Computer Security",
                Some("344-223"),
                Some(2.0),
            ),
        ]);
        assert_eq!(
            rows,
            vec![
                Todo::Course {
                    code: "344-223".into(),
                    name: "Fundamentals of Computer Security".into(),
                    category: "Core Courses".into(),
                    credits: 2.0,
                },
                Todo::Gap {
                    category: "Major Electives".into(),
                    credits: 3.0,
                },
                Todo::Rule {
                    category: "Capstone".into(),
                    description: "Choose 1: 344-492 OR 344-495".into(),
                },
            ]
        );
    }

    #[test]
    fn todo_rows_read_old_saves_without_code_or_credits() {
        let m = |category: &str, description: &str| MissingCourse {
            category: category.into(),
            description: description.into(),
            ..Default::default()
        };
        let rows = todo_rows(&[
            m(
                "Major Courses",
                "Overall Major Courses: missing 93.0 credits",
            ),
            m(
                "Core Courses",
                "344-223 - Fundamentals of Computer Security",
            ),
            m("Major Electives", "Major Electives: missing 3.0 credits"),
        ]);
        assert_eq!(
            rows,
            vec![
                Todo::Course {
                    code: "344-223".into(),
                    name: "Fundamentals of Computer Security".into(),
                    category: "Core Courses".into(),
                    credits: 0.0,
                },
                Todo::Gap {
                    category: "Major Electives".into(),
                    credits: 3.0,
                },
            ]
        );
    }

    #[test]
    fn thai_labels_for_known_names() {
        assert_eq!(label("Major Courses", true), "หมวดวิชาเฉพาะ");
        assert_eq!(label("Capstone", true), "โครงงานหรือสหกิจ");
        assert_eq!(label("Major Courses", false), "Major courses");
        assert_eq!(label("Unknown thing", true), "Unknown thing");
    }
}
