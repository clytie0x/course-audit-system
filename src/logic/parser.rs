//! PDF Transcript Parser
//!
//! Extracts course data from PDF transcripts using Regex patterns.
//! Uses JavaScript interop (via `wasm-bindgen`) to access PDF.js for text extraction,
//! then parses course entries (code, name, credits, grade) from extracted text.

use crate::models::ParsedCourse;
use regex::Regex;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

// Pattern: course code followed by name, section, credit, then grade
// Example: 322-101   CALCULUS I   04   3   B+
fn course_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"([A-Za-z0-9]{3}-?\d{3}[A-Za-z]?\d*[A-Za-z]?)\s+([A-Za-z0-9\s:()&\-\.,'/\*]+?)\s+(\d+)\s+(\d+)\s+([A-D][+]?|[FWPSUGE])",
        )
        .expect("transcript course regex must compile")
    })
}

pub(crate) fn normalize_course_code(raw_code: &str) -> String {
    let trimmed = if raw_code.len() >= 7 && raw_code.chars().nth(3) == Some('-') {
        raw_code[..7].to_string()
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

/// JavaScript interop function exposed by the PDF extractor in the frontend runtime.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = extractTextFromPDF)]
    pub fn extract_text_from_pdf(array_buffer: js_sys::Uint8Array) -> js_sys::Promise;
}

/// Parses transcript text into structured course entries, normalizing codes and
/// greedily numbering special-topic courses (e.g., 344-496 Topic 1, Topic 2).
pub fn parse_transcript(text: &str) -> Vec<ParsedCourse> {
    let mut courses = Vec::new();
    let mut special_topics_count: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    for captures in course_pattern().captures_iter(text) {
        // All captures exist because every capture group in the regex is
        // unconditional (no `?` quantifier at the group level).
        let (Some(raw_code), Some(name), Some(parsed_credit_str), Some(grade)) = (
            captures.get(1),
            captures.get(2),
            captures.get(4),
            captures.get(5),
        ) else {
            continue;
        };
        let raw_code = raw_code.as_str();
        let name = name.as_str().trim();
        let parsed_credit_str = parsed_credit_str.as_str();
        let grade = grade.as_str().to_uppercase();

        let parsed_credit = parsed_credit_str.parse::<f32>().unwrap_or(3.0);

        // Normalize course code by trimming suffix (e.g., 890-103G1 -> 890-103)
        // and applying known equivalence mappings used by the curriculum.
        let normalized_code = normalize_course_code(raw_code);

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

        courses.push(ParsedCourse {
            code: normalized_code,
            name: final_name,
            grade,
            parsed_credit,
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
        let courses = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "322-101");
        assert_eq!(courses[0].name, "CALCULUS I");
        assert_eq!(courses[0].grade, "B+");
        assert_eq!(courses[0].parsed_credit, 3.0);
    }

    #[test]
    fn parse_normalizes_legacy_code() {
        let text = "890-001 ENGLISH I 01 3 A";
        let courses = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "890-101");
    }

    #[test]
    fn parse_numbers_repeated_special_topics() {
        let text = "344-496 TOPIC ALPHA 01 3 A 344-496 TOPIC BETA 02 3 B";
        let courses = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert!(courses[0].name.ends_with("(Topic 1)"));
        assert!(courses[1].name.ends_with("(Topic 2)"));
    }

    #[test]
    fn parse_does_not_tag_non_repeatable_344_49x() {
        let text = "344-491 SENIOR PROJECT 01 3 A";
        let courses = parse_transcript(text);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "SENIOR PROJECT");
        assert!(!courses[0].name.contains("Topic"));
    }

    #[test]
    fn parse_recognizes_non_letter_grades() {
        let text = "200-101 COURSE A 01 3 W 200-102 COURSE B 02 3 P";
        let courses = parse_transcript(text);
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].grade, "W");
        assert_eq!(courses[1].grade, "P");
    }

    #[test]
    fn parse_ignores_malformed_lines() {
        let text = "This line has no course at all.";
        assert!(parse_transcript(text).is_empty());
    }
}
