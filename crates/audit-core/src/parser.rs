//! PDF Transcript Parser
//!
//! Extracts course data from PDF transcript text using Regex patterns.
//! Text extraction itself (PDF.js interop) lives in the web crate's `pdf` module;
//! this module parses course entries (code, name, credits, grade) from that text.
//!
//! Two PDF formats are supported, auto-detected by trying each in order:
//! 1. **Grade report** — `code name section credit(int) grade`
//! 2. **Registration report** — `code name section category credit(float)` (no grade yet).
//!    Registration-matched courses are tagged with `grade = "IP"` (In Progress).

use crate::models::{ParseMode, ParsedCourse};
use regex_lite::Regex;
use std::sync::OnceLock;

// Grade-report pattern: `code name section credit grade`
// Example: 322-101   CALCULUS I   04   3   B+
// Uses [0-9] (not \d) because the regex crate's \d is Unicode-aware and
// would happily match Thai numerals, producing multi-byte captures that
// break downstream byte slicing.
fn grade_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"\b([A-Za-z0-9]{3}-?[0-9]{3}(?:[A-Za-z][0-9]*[A-Za-z]?)?)\b\s+([A-Za-z0-9\s:()&\-\.,'/\*]+?)\s+([0-9]+)\s+([0-9]+)\s+([A-D][+]?|[FWPSUGE])",
        )
        .expect("transcript grade regex must compile")
    })
}

// Registration-report pattern: `code name section category credit(float)`
// Example: 145-101   COMPANION ANIMALS   02   C   3.0
// Category can be `C`, `G4`, `G2A`, `G2B`, `G8` etc. (SIS subject-type codes).
fn registration_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"\b([A-Za-z0-9]{3}-?[0-9]{3}(?:[A-Za-z][0-9]*[A-Za-z]?)?)\b\s+([A-Za-z0-9\s:()&\-\.,'/\*]+?)\s+([0-9]+)\s+([A-Z][0-9]?[A-Z]?)\s+([0-9]+(?:\.[0-9]+)?)",
        )
        .expect("registration regex must compile")
    })
}

// Captures full "ภาคการศึกษา X/YYYY" headers in a single regex so we never
// hand-slice the underlying text — the previous implementation sliced at a
// fixed byte offset which panicked (RuntimeError: unreachable) whenever the
// offset landed inside a Thai multi-byte UTF-8 character.
// English reports say "Semester 1/2024" with a Gregorian year.
fn semester_header_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?:ภาคการศึกษา|Semester)\s*([0-9]{1,2})/([0-9]{4})")
            .expect("semester header regex must compile")
    })
}

/// Returns every `(start_byte, "X/YYYY")` occurrence of a semester marker in
/// the order it appears, with the year in the Buddhist era so Thai and English
/// reports label semesters identically.
fn collect_semester_boundaries(text: &str) -> Vec<(usize, String)> {
    semester_header_pattern()
        .captures_iter(text)
        .filter_map(|cap| {
            let full = cap.get(0)?;
            let term = cap.get(1)?.as_str();
            let year: u32 = cap.get(2)?.as_str().parse().ok()?;
            let year_be = if year < 2400 { year + 543 } else { year };
            Some((full.start(), format!("{term}/{year_be}")))
        })
        .collect()
}

/// Admission cohort (Buddhist year) from the student ID on the report, whose
/// first two digits are the cohort, e.g. "Student Id 67xxxxxxxx" -> 2567.
pub fn detect_cohort(text: &str) -> Option<u32> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let re = PATTERN.get_or_init(|| {
        Regex::new(r"(?:Student Id|รหัสนักศึกษา)\s*([0-9]{2})[0-9]{8}\b")
            .expect("student id regex must compile")
    });
    let text = &normalize_spaces(text);
    let cap = re
        .captures_iter(text)
        .find(|c| !text[c.get(0).map_or(0, |m| m.end())..].starts_with(is_word_char))?;
    let yy: u32 = cap.get(1)?.as_str().parse().ok()?;
    Some(2500 + yy)
}

/// regex-lite's `\s` is ASCII-only, so turn the no-break spaces some PDFs emit
/// into plain spaces before matching.
fn normalize_spaces(text: &str) -> String {
    text.replace('\u{a0}', " ")
}

/// Unicode word character as the full `regex` crate's `\b` saw it; regex-lite's
/// `\b` is ASCII-only, so matches glued to Thai letters are filtered with this.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || ('\u{0E00}'..='\u{0E7F}').contains(&c)
}

/// GE category tag (e.g. "G2A") from a raw transcript code like "315-100G2A".
fn ge_tag(raw_code: &str) -> Option<String> {
    let suffix: String = raw_code.chars().skip(7).collect();
    is_ge_tag(&suffix).then_some(suffix)
}

fn is_ge_tag(s: &str) -> bool {
    let b = s.as_bytes();
    matches!(b, [b'G', d] | [b'G', d, b'A'..=b'Z'] if d.is_ascii_digit())
}

fn semester_for(boundaries: &[(usize, String)], course_start: usize) -> Option<String> {
    // Last boundary whose position is ≤ course_start.
    boundaries
        .iter()
        .rev()
        .find(|(pos, _)| *pos <= course_start)
        .map(|(_, s)| s.clone())
}

pub fn normalize_course_code(raw_code: &str) -> String {
    // The Rust regex crate is Unicode-aware by default, so `\d` can match
    // Thai numerals (๐-๙) — each 3 bytes. Byte-slicing at `[..7]` therefore
    // panics mid-character and compiles to `RuntimeError: unreachable` in
    // WASM. Count chars explicitly and use a char-aware take(7) instead.
    let char_count = raw_code.chars().count();
    let has_dash_at_3 = raw_code.chars().nth(3) == Some('-');
    let trimmed = if char_count >= 7 && has_dash_at_3 {
        raw_code.chars().take(7).collect::<String>()
    } else {
        raw_code.to_string()
    };

    match trimmed.as_str() {
        "890-001" => "890-101".to_string(),
        "890-002" => "890-102".to_string(),
        "890-003" => "890-103".to_string(),
        "890-004" => "890-104".to_string(),
        "890-005" => "890-105".to_string(),
        _ => trimmed,
    }
}

/// Parses transcript text into structured course entries, normalizing codes and
/// greedily numbering special-topic courses (e.g., 344-496 Topic 1, Topic 2).
///
/// Returns the detected `ParseMode` alongside the courses so callers can
/// distinguish registration (grades not yet issued) from grade-report results.
pub fn parse_transcript(text: &str) -> (Vec<ParsedCourse>, ParseMode) {
    let text = &normalize_spaces(text);
    // Prefer the grade-report format when present, otherwise fall back to the
    // registration format — they have disjoint signatures so no overlap.
    let grade_courses = collect_courses(text, grade_pattern(), CourseFormat::Grade);
    if !grade_courses.is_empty() {
        return (grade_courses, ParseMode::Grade);
    }

    let registration_courses =
        collect_courses(text, registration_pattern(), CourseFormat::Registration);
    let mode = if registration_courses.is_empty() {
        ParseMode::Grade
    } else {
        ParseMode::Registration
    };
    (registration_courses, mode)
}

enum CourseFormat {
    /// `code name section credit grade`
    Grade,
    /// `code name section category credit(float)` — grade not yet issued.
    Registration,
}

fn collect_courses(text: &str, pattern: &Regex, format: CourseFormat) -> Vec<ParsedCourse> {
    let mut courses = Vec::new();
    let mut special_topics_count: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    let boundaries = collect_semester_boundaries(text);

    for captures in pattern.captures_iter(text) {
        let Some(raw_code) = captures.get(1) else {
            continue;
        };
        if text[..raw_code.start()].ends_with(is_word_char) {
            continue;
        }
        let Some(name) = captures.get(2) else {
            continue;
        };

        let mut tag = ge_tag(raw_code.as_str());
        let (credit_str, grade) = match format {
            CourseFormat::Grade => {
                // groups: 1=code 2=name 3=section 4=credit(int) 5=grade
                let Some(credit) = captures.get(4) else {
                    continue;
                };
                let Some(g) = captures.get(5) else {
                    continue;
                };
                (credit.as_str(), g.as_str().to_uppercase())
            }
            CourseFormat::Registration => {
                // groups: 1=code 2=name 3=section 4=category 5=credit(float)
                // Registration has no grade yet — tag as "IP" (In Progress).
                let Some(credit) = captures.get(5) else {
                    continue;
                };
                if let Some(category) = captures.get(4).filter(|c| is_ge_tag(c.as_str())) {
                    tag.get_or_insert_with(|| category.as_str().to_string());
                }
                (credit.as_str(), "IP".to_string())
            }
        };

        let name = name.as_str().trim();
        let parsed_credit = credit_str.parse::<f32>().unwrap_or(3.0);

        // Normalize course code by trimming suffix (e.g., 890-103G1 -> 890-103)
        // and applying known equivalence mappings used by the curriculum.
        let normalized_code = normalize_course_code(raw_code.as_str());

        // Greedy match: Special topics (344-496 to 344-499) might be repeated.
        // We handle any course starting with 344-49, EXCEPT the specific Capstone/Core ones.
        let is_special_topic = normalized_code.starts_with("344-49")
            && !matches!(
                normalized_code.as_str(),
                "344-491" | "344-492" | "344-493" | "344-494" | "344-495"
            );

        let final_name = if is_special_topic {
            let counter = special_topics_count
                .entry(normalized_code.clone())
                .or_insert(0);
            *counter += 1;
            format!("{} (Topic {})", name, counter)
        } else {
            name.to_string()
        };

        let semester = semester_for(&boundaries, raw_code.start());

        courses.push(ParsedCourse {
            code: normalized_code,
            name: final_name,
            grade,
            parsed_credit,
            semester,
            tag,
        });
    }

    courses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_suffix() {
        assert_eq!(normalize_course_code("890-103G1"), "890-103");
        assert_eq!(normalize_course_code("344-101"), "344-101");
    }

    #[test]
    fn normalize_maps_legacy_codes() {
        assert_eq!(normalize_course_code("890-001"), "890-101");
        assert_eq!(normalize_course_code("890-002"), "890-102");
        assert_eq!(normalize_course_code("890-003"), "890-103");
        assert_eq!(normalize_course_code("890-004"), "890-104");
        assert_eq!(normalize_course_code("890-005"), "890-105");
    }

    #[test]
    fn normalize_leaves_short_codes_alone() {
        assert_eq!(normalize_course_code("X"), "X");
    }

    #[test]
    fn parse_extracts_basic_course() {
        let text = "322-101 CALCULUS I 04 3 B+";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Grade);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "322-101");
        assert_eq!(courses[0].name, "CALCULUS I");
        assert_eq!(courses[0].grade, "B+");
        assert_eq!(courses[0].parsed_credit, 3.0);
    }

    #[test]
    fn parse_normalizes_legacy_code() {
        let text = "890-001 ENGLISH I 01 3 A";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "890-101");
    }

    #[test]
    fn parse_numbers_repeated_special_topics() {
        let text = "344-496 TOPIC ALPHA 01 3 A 344-496 TOPIC BETA 02 3 B";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert!(courses[0].name.ends_with("(Topic 1)"));
        assert!(courses[1].name.ends_with("(Topic 2)"));
    }

    #[test]
    fn parse_does_not_tag_non_repeatable_344_49x() {
        let text = "344-491 SENIOR PROJECT 01 3 A";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "SENIOR PROJECT");
        assert!(!courses[0].name.contains("Topic"));
    }

    #[test]
    fn parse_recognizes_non_letter_grades() {
        let text = "200-101 COURSE A 01 3 W 200-102 COURSE B 02 3 P";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].grade, "W");
        assert_eq!(courses[1].grade, "P");
    }

    #[test]
    fn parse_ignores_malformed_lines() {
        let text = "This line has no course at all.";
        let (courses, mode) = parse_transcript(text);
        assert!(courses.is_empty());
        assert_eq!(mode, ParseMode::Grade);
    }

    #[test]
    fn parse_registration_tags_courses_as_ip() {
        let text = "145-101 COMPANION ANIMALS 02 C 3.0";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Registration);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "145-101");
        assert_eq!(courses[0].grade, "IP");
        assert_eq!(courses[0].parsed_credit, 3.0);
    }

    #[test]
    fn parse_registration_handles_category_suffixes() {
        // SIS uses categories like C, G4, G2A, G2B, G8 alongside float credits.
        let text = "\
            315-104G4 DIGITAL TECH 05 G4 2.0
            895-222G2B CRITICAL THINKING 02 G2B 2.0
            B03-001G4 GET SMART ON CYBER THREATS 01 G4 2.0";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Registration);
        assert_eq!(courses.len(), 3);
        assert!(courses.iter().all(|c| c.grade == "IP"));
        assert_eq!(courses[0].code, "315-104");
        assert_eq!(courses[2].code, "B03-001");
    }

    #[test]
    fn parse_grade_format_takes_priority_when_both_present() {
        // If any grade-format match is found, use grade mode (ignore registration lines).
        let text = "\
            145-101 COMPANION ANIMALS 02 C 3.0
            322-101 CALCULUS I 04 3 B+";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Grade);
        assert!(courses.iter().all(|c| c.grade != "IP"));
    }

    #[test]
    fn is_in_progress_detects_ip_grade() {
        use crate::models::is_in_progress;
        assert!(is_in_progress("IP"));
        assert!(is_in_progress(" ip "));
        assert!(!is_in_progress("A"));
        assert!(!is_in_progress(""));
    }

    #[test]
    fn is_passing_grade_rejects_ip() {
        use crate::models::is_passing_grade;
        assert!(!is_passing_grade("IP"));
        assert!(is_passing_grade("A"));
        assert!(!is_passing_grade("F"));
    }

    #[test]
    fn parse_tags_courses_with_semester_header() {
        let text = "\
            ภาคการศึกษา 1/2567
            322-101 CALCULUS I 04 3 B+
            ภาคการศึกษา 2/2567
            322-102 CALCULUS II 01 3 A";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].semester.as_deref(), Some("1/2567"));
        assert_eq!(courses[1].semester.as_deref(), Some("2/2567"));
    }

    #[test]
    fn parse_leaves_semester_none_before_first_header() {
        let text = "322-101 CALCULUS I 04 3 B+";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses[0].semester, None);
    }

    #[test]
    fn parse_tags_registration_semesters_too() {
        let text = "\
            ภาคการศึกษา 2/2568
            145-101 COMPANION ANIMALS 02 C 3.0";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Registration);
        assert_eq!(courses[0].semester.as_deref(), Some("2/2568"));
    }

    #[test]
    fn normalize_does_not_panic_on_unicode_input() {
        // Defensive regression: even if the caller someday passes in a string
        // with multi-byte characters, normalize_course_code must not panic.
        // The previous byte-slice `raw_code[..7]` would split a Thai glyph.
        assert_eq!(normalize_course_code("๓๒๒-๑๐๑G1"), "๓๒๒-๑๐๑");
        assert_eq!(normalize_course_code("abc-๑๒๓"), "abc-๑๒๓");
    }

    #[test]
    fn parse_does_not_panic_on_thai_text_after_semester_header() {
        // Regression: the old implementation sliced the text at a fixed byte
        // offset relative to the semester header, which panicked whenever the
        // offset landed inside a Thai multi-byte UTF-8 character. This mirrors
        // the layout produced by pdftotext on a real PSU SIS transcript.
        let text = "\
            ภาคการศึกษา 1/2567
            รหัสวิชา ชื่อวิชา ตอน หน่วยกิต ผลการเรียน
            322-101 CALCULUS I 04 3 B+
            ภาคการศึกษา 2/2567
            322-102 CALCULUS II 01 3 A";
        // The panic used to surface as `RuntimeError: unreachable` in WASM.
        // Running the parser to completion is the assertion.
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].semester.as_deref(), Some("1/2567"));
        assert_eq!(courses[1].semester.as_deref(), Some("2/2567"));
    }

    // Header layout of the English SIS grade report (pdf.js joins items by spaces).
    const EN_REPORT: &str = "Study Result Mister SOMEONE Student Id 6700000001 Prince of Songkla \
        University Hat Yai Campus Faculty of Science Division of Computational Science Computer \
        Science Semester 2/2024 Subject Code Subject Name Section Credit Grade 003-001 VOL LEADER \
        FOR SUS COM DEL 02 3 A 315-100G2A THE ART OF COMPUTING 07 2 B+";

    #[test]
    fn student_id_is_not_read_as_a_course_code() {
        // Regression: "6700000001" used to match as code "670000…", swallowing
        // the header text up to and including the first real course.
        let (courses, _) = parse_transcript(EN_REPORT);
        let codes: Vec<&str> = courses.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["003-001", "315-100"]);
    }

    #[test]
    fn english_semester_header_uses_buddhist_year() {
        let (courses, _) = parse_transcript(EN_REPORT);
        assert_eq!(courses[0].semester.as_deref(), Some("2/2567"));
    }

    #[test]
    fn ge_tag_comes_from_code_suffix() {
        let (courses, _) = parse_transcript(EN_REPORT);
        assert_eq!(courses[0].tag, None);
        assert_eq!(courses[1].tag.as_deref(), Some("G2A"));
    }

    #[test]
    fn ge_tag_comes_from_registration_category() {
        let text = "145-101 COMPANION ANIMALS 02 G8 3.0 344-201 MODULE COMPUTING 01 C 6.0";
        let (courses, mode) = parse_transcript(text);
        assert_eq!(mode, ParseMode::Registration);
        assert_eq!(courses[0].tag.as_deref(), Some("G8"));
        assert_eq!(courses[1].tag, None);
    }

    #[test]
    fn cohort_from_student_id() {
        assert_eq!(detect_cohort(EN_REPORT), Some(2567));
        assert_eq!(detect_cohort("รหัสนักศึกษา 6900000001"), Some(2569));
        assert_eq!(detect_cohort("no id here"), None);
    }

    #[test]
    fn grade_line_with_no_break_space_parses() {
        let text = "344-111\u{a0}PROGRAMMING CONCEPTS\u{a0}6\u{a0}6\u{a0}A";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "344-111");
        assert_eq!(courses[0].grade, "A");
    }

    #[test]
    fn course_code_after_thai_text_parses() {
        let text = "ภาคการศึกษา 1/2567 344-111 PROGRAMMING CONCEPTS 6 6 A";
        let (courses, _) = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].semester.as_deref(), Some("1/2567"));
    }

    #[test]
    fn cohort_ignores_id_glued_to_thai_letter() {
        assert_eq!(detect_cohort("รหัสนักศึกษา 6700000001ก"), None);
    }

    #[test]
    fn course_code_glued_to_thai_letter_is_not_a_course() {
        let text = "วิชา344-111 PROGRAMMING CONCEPTS 6 6 A";
        let (courses, _) = parse_transcript(text);
        assert!(courses.is_empty());
    }
}
