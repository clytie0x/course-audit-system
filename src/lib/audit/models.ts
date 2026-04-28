/**
 * Core data structures for the Course Audit System.
 *
 * Mirrors src/models.rs from the previous Rust implementation 1:1.
 * Field names use snake_case to match the curriculum JSON files verbatim.
 */

export type Grade = string;

export const IN_PROGRESS_GRADE = 'IP' as const;

export interface Course {
	code: string;
	name: string;
	credit: number;
	grade: Grade;
	semester: string | null;
}

export interface Category {
	name: string;
	required_credits: number;
	collected_credits: number;
	courses: Course[];
}

export interface MissingCourse {
	category: string;
	description: string;
}

export interface AuditResult {
	total_credits: number;
	categories: Category[];
	missing_subjects: MissingCourse[];
	gpa?: number | null;
}

export interface GenEdCourse {
	code: string;
	name: string;
	credits: number;
}

export interface GenEdSubGroup {
	name: string;
	required_credits: number;
	courses: GenEdCourse[];
}

export type SelectionRule =
	| 'choose_all'
	| 'choose_one'
	| 'choose_sequential_pair'
	| 'choose_all_sub_groups';

export interface GenEdStrand {
	id: number;
	name: string;
	required_credits: number;
	sub_groups?: GenEdSubGroup[];
	courses?: GenEdCourse[];
	selection_rule?: SelectionRule;
	sequence_groups?: string[][];
}

export interface GenEdElectiveSubCategory {
	name: string;
	required_credits: number;
	min_courses: number;
	max_courses: number;
	courses: GenEdCourse[];
}

export interface GenEdElectives {
	name: string;
	total_required_credits: number;
	sub_categories: GenEdElectiveSubCategory[];
}

export interface GenEdCurriculum {
	name: string;
	total_required_credits: number;
	strands: GenEdStrand[];
	electives: GenEdElectives;
}

export interface MajorCourse {
	code: string;
	name: string;
	credits: number;
}

export interface MajorCluster {
	id: string;
	name: string;
	min_courses: number;
	description: string | null;
	courses: MajorCourse[];
}

export interface MajorDomain {
	id: number;
	name: string;
	description: string | null;
	clusters: MajorCluster[];
}

export interface MajorBasicScience {
	name: string;
	required_credits: number;
	courses: MajorCourse[];
}

export interface MajorCoreCourses {
	name: string;
	required_credits: number;
	courses: MajorCourse[];
}

export interface MajorCapstone {
	name: string;
	credits_per_option: number;
	options: MajorCourse[];
}

export interface MajorElectives {
	name: string;
	total_required_credits: number;
	clusters_to_complete: number;
	domains: MajorDomain[];
	others: MajorCourse[];
}

export interface MajorCurriculum {
	name: string;
	total_required_credits: number;
	basic_science: MajorBasicScience;
	core_courses: MajorCoreCourses;
	capstone: MajorCapstone;
	electives: MajorElectives;
}

/**
 * Parsed course extracted from transcript text. Lives only in memory between
 * parse and audit; not serialized.
 */
export interface ParsedCourse {
	code: string;
	name: string;
	grade: Grade;
	parsed_credit: number;
	semester: string | null;
}

/**
 * PDF format detected by the parser. In `registration` mode, in-progress
 * courses fill curriculum slots but do not contribute credits to the total.
 */
export type ParseMode = 'grade' | 'registration';

/**
 * Returns true when the grade represents a passing result.
 * Failing markers: F, W, U. IP (in-progress) is not yet passing.
 */
export function isPassingGrade(grade: string): boolean {
	if (isInProgress(grade)) return false;
	const trimmed = grade.trim();
	if (trimmed.length === 0) return false;
	const first = trimmed.charAt(0).toUpperCase();
	return first !== 'F' && first !== 'W' && first !== 'U';
}

export function isInProgress(grade: string): boolean {
	return grade.trim().toUpperCase() === IN_PROGRESS_GRADE;
}

/**
 * Maps a PSU letter grade to a grade point on the 4.0 scale.
 * Non-letter grades (W, P, S, U, G, IP) return `null` and are excluded
 * from GPA calculation.
 */
export function gradePoints(grade: string): number | null {
	const g = grade.trim().toUpperCase();
	switch (g) {
		case 'A':
			return 4.0;
		case 'B+':
			return 3.5;
		case 'B':
			return 3.0;
		case 'C+':
			return 2.5;
		case 'C':
			return 2.0;
		case 'D+':
			return 1.5;
		case 'D':
			return 1.0;
		case 'E':
		case 'F':
			return 0.0;
		default:
			return null;
	}
}

/**
 * GPA = Σ(grade_points × credits) / Σ(credits) over courses with letter
 * grades only. Returns `null` when no graded credits are present.
 */
export function computeGpa(courses: Course[]): number | null {
	let totalPoints = 0;
	let totalCredits = 0;
	for (const c of courses) {
		const pts = gradePoints(c.grade);
		if (pts !== null) {
			totalPoints += pts * c.credit;
			totalCredits += c.credit;
		}
	}
	if (totalCredits === 0) return null;
	return totalPoints / totalCredits;
}

/**
 * Deduplication key so that repeatable special-topic courses
 * (344-496 … 344-499, excluding 344-491…495) are keyed by code+name,
 * while all other courses are keyed by code alone.
 */
export function freeElectiveDedupeKey(code: string, name: string): string {
	const isRepeatableSpecialTopic =
		code.startsWith('344-49') &&
		code !== '344-491' &&
		code !== '344-492' &&
		code !== '344-493' &&
		code !== '344-494' &&
		code !== '344-495';
	return isRepeatableSpecialTopic ? `${code}::${name}` : code;
}
