//! Course offerings scraped from PSU SIS (`data/offerings/<year>-<term>.json`,
//! produced by `tools/sis-scraper`) plus the timetable helpers the UI uses:
//! search, outstanding required courses, and class / exam clash detection.

use crate::models::{is_passing_grade, Curriculum, ParsedCourse};
use serde::{Deserialize, Serialize};

/// `data/offerings/index.json`: available term keys, newest first ("2569-2").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferingsIndex {
    pub terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermOfferings {
    pub term: String,
    pub campus: String,
    pub source: String,
    pub scraped_at: String,
    pub courses: Vec<Offering>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Offering {
    /// SIS subject id; one code can have several offerings (e.g. per curriculum).
    pub id: String,
    pub code: String,
    pub name_th: Option<String>,
    pub name_en: Option<String>,
    pub credits: Option<f32>,
    pub credit_pattern: Option<String>,
    pub faculty: Option<String>,
    pub department: Option<String>,
    #[serde(default)]
    pub fee: Option<String>,
    pub sections: Vec<Section>,
    /// SIS declared more sections than were captured.
    #[serde(default)]
    pub incomplete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub section: String,
    pub seats: Option<u32>,
    /// Who the section is for, e.g. "CS1 (หลักสูตรปรับปรุง 2569)".
    pub students: Option<String>,
    pub meetings: Vec<Meeting>,
    pub midterm: Option<String>,
    #[serde(rename = "final")]
    pub final_exam: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meeting {
    /// 1 = Monday … 7 = Sunday.
    pub day: u8,
    /// "HH:MM".
    pub start: String,
    pub end: String,
    pub room: String,
    pub teachers: String,
}

/// Minutes since midnight for "HH:MM"; `None` if malformed.
pub fn minutes(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.split_once(':')?;
    Some(h.trim().parse::<u32>().ok()? * 60 + m.trim().parse::<u32>().ok()?)
}

/// Two meetings clash when they share a day and their time ranges overlap.
pub fn meetings_clash(a: &Meeting, b: &Meeting) -> bool {
    match (
        minutes(&a.start),
        minutes(&a.end),
        minutes(&b.start),
        minutes(&b.end),
    ) {
        (Some(s1), Some(e1), Some(s2), Some(e2)) => a.day == b.day && s1 < e2 && s2 < e1,
        _ => false,
    }
}

/// Exam text like "15 ม.ค. 2570 13:30–16:30" -> (date part, start, end minutes).
fn exam_slot(text: &str) -> Option<(&str, u32, u32)> {
    let (date, range) = text.trim().rsplit_once(' ')?;
    let (s, e) = range.split_once(['–', '-'])?;
    Some((date.trim(), minutes(s)?, minutes(e)?))
}

/// Two exam texts clash when they fall on the same date with overlapping times.
pub fn exams_clash(a: &str, b: &str) -> bool {
    match (exam_slot(a), exam_slot(b)) {
        (Some((d1, s1, e1)), Some((d2, s2, e2))) => d1 == d2 && s1 < e2 && s2 < e1,
        _ => false,
    }
}

/// What clashes between two chosen sections, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clash {
    pub class_time: bool,
    pub midterm: bool,
    pub final_exam: bool,
}

pub fn sections_clash(a: &Section, b: &Section) -> Option<Clash> {
    let both = |x: &Option<String>, y: &Option<String>| match (x, y) {
        (Some(x), Some(y)) => exams_clash(x, y),
        _ => false,
    };
    let clash = Clash {
        class_time: a
            .meetings
            .iter()
            .any(|m| b.meetings.iter().any(|n| meetings_clash(m, n))),
        midterm: both(&a.midterm, &b.midterm),
        final_exam: both(&a.final_exam, &b.final_exam),
    };
    (clash.class_time || clash.midterm || clash.final_exam).then_some(clash)
}

/// Offerings whose code or Thai/English name contains `query` (case-insensitive).
pub fn search<'a>(courses: &'a [Offering], query: &str) -> Vec<&'a Offering> {
    let q = query.trim().to_lowercase();
    courses
        .iter()
        .filter(|c| {
            q.is_empty()
                || c.code.contains(&q)
                || c.name_th
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(&q))
                || c.name_en
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(&q))
        })
        .collect()
}

/// Required courses of `curriculum` not yet passed or in progress: basic
/// science, core, non-credit requirements, and every capstone option while
/// none has been taken.
pub fn outstanding_codes(attempts: &[ParsedCourse], curriculum: &Curriculum) -> Vec<String> {
    let done = |code: &str| {
        attempts.iter().any(|a| {
            a.code == code
                && (is_passing_grade(&a.grade) || a.grade.trim().eq_ignore_ascii_case("IP"))
        })
    };
    let m = &curriculum.major;
    let mut codes: Vec<String> = m
        .basic_science
        .courses
        .iter()
        .chain(&m.core_courses.courses)
        .map(|c| c.code.clone())
        .chain(m.noncredit_required.iter().map(|r| r.code.clone()))
        .collect();
    if !m.capstone.options.iter().any(|o| done(&o.code)) {
        codes.extend(m.capstone.options.iter().map(|o| o.code.clone()));
    }
    codes.retain(|c| !done(c));
    codes.dedup();
    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::curriculum;

    fn meeting(day: u8, start: &str, end: &str) -> Meeting {
        Meeting {
            day,
            start: start.into(),
            end: end.into(),
            room: String::new(),
            teachers: String::new(),
        }
    }

    fn section(meetings: Vec<Meeting>, midterm: Option<&str>, final_exam: Option<&str>) -> Section {
        Section {
            section: "01".into(),
            seats: Some(40),
            students: None,
            meetings,
            midterm: midterm.map(Into::into),
            final_exam: final_exam.map(Into::into),
        }
    }

    #[test]
    fn meeting_overlap_rules() {
        assert!(meetings_clash(
            &meeting(2, "10:00", "11:50"),
            &meeting(2, "11:00", "12:50")
        ));
        assert!(
            !meetings_clash(&meeting(2, "10:00", "11:50"), &meeting(2, "11:50", "12:50")),
            "back to back is fine"
        );
        assert!(
            !meetings_clash(&meeting(2, "10:00", "11:50"), &meeting(3, "10:00", "11:50")),
            "different day"
        );
    }

    #[test]
    fn exam_overlap_rules() {
        assert!(exams_clash(
            "15 ม.ค. 2570 13:30–16:30",
            "15 ม.ค. 2570 09:00–14:00"
        ));
        assert!(!exams_clash(
            "15 ม.ค. 2570 13:30–16:30",
            "14 ม.ค. 2570 13:30–16:30"
        ));
        assert!(!exams_clash(
            "15 ม.ค. 2570 09:00–12:00",
            "15 ม.ค. 2570 13:30–16:30"
        ));
        assert!(
            !exams_clash("-", "15 ม.ค. 2570 09:00–12:00"),
            "no exam text never clashes"
        );
    }

    #[test]
    fn section_clash_reports_what_collides() {
        let a = section(
            vec![meeting(4, "13:00", "14:50")],
            Some("08 ส.ค. 2569 09:00–12:00"),
            Some("10 ต.ค. 2569 09:00–12:00"),
        );
        let b = section(
            vec![meeting(4, "14:00", "15:50")],
            Some("09 ส.ค. 2569 09:00–12:00"),
            Some("10 ต.ค. 2569 09:00–12:00"),
        );
        let c = sections_clash(&a, &b).unwrap();
        assert!(c.class_time && !c.midterm && c.final_exam);
        let d = section(vec![meeting(1, "08:00", "09:50")], None, None);
        assert_eq!(sections_clash(&a, &d), None);
    }

    #[test]
    fn deserializes_scraper_output() {
        let json = r#"{"term":"2/2569","campus":"หาดใหญ่","source":"https://sis.psu.ac.th/explore/subject","scraped_at":"2026-11-22",
          "courses":[{"id":"1","code":"344-111","name_th":"พื้นฐาน","name_en":"FUNDAMENTALS","credits":3,"credit_pattern":"3((2)-2-5)",
          "faculty":"คณะวิทยาศาสตร์","department":null,"fee":null,"incomplete":false,
          "sections":[{"section":"01","seats":80,"students":"CS1","midterm":"15 ม.ค. 2570 13:30–16:30","final":null,
          "meetings":[{"day":4,"start":"13:00","end":"14:50","room":"CS201","teachers":"A"}]}]}]}"#;
        let t: TermOfferings = serde_json::from_str(json).unwrap();
        assert_eq!(t.courses[0].sections[0].meetings[0].day, 4);
        assert_eq!(search(&t.courses, "fundam").len(), 1);
        assert_eq!(search(&t.courses, "344-1").len(), 1);
        assert_eq!(search(&t.courses, "พื้น").len(), 1);
        assert!(search(&t.courses, "xyz").is_empty());
    }

    #[test]
    fn outstanding_lists_untaken_required_courses() {
        let c = curriculum("cs2569");
        let all = outstanding_codes(&[], c);
        assert!(all.contains(&"344-111".to_string()) && all.contains(&"344-494".to_string()));
        let taken = [ParsedCourse {
            code: "344-111".into(),
            name: String::new(),
            grade: "B".into(),
            parsed_credit: 3.0,
            semester: None,
            tag: None,
        }];
        assert!(!outstanding_codes(&taken, c).contains(&"344-111".to_string()));
        // 2564 lists both capstone options until one is taken.
        let c64 = curriculum("cs2564");
        let o = outstanding_codes(&[], c64);
        assert!(o.contains(&"344-492".to_string()) && o.contains(&"344-495".to_string()));
    }
}
