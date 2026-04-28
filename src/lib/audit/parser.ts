/**
 * PDF Transcript Parser.
 *
 * Mirrors src/logic/parser.rs from the previous Rust implementation.
 * Supports two PDF formats, auto-detected by trying each in order:
 *   1. Grade report — `code name section credit(int) grade`
 *   2. Registration report — `code name section category credit(float)` (no grade yet).
 *      Registration courses are tagged grade = "IP" (In Progress).
 *
 * Regexes use `[0-9]` (not `\d`) wherever digit-only matching is required, so
 * Thai numerals never sneak in. The semester header itself uses Thai literals
 * inside the pattern, which JavaScript handles with the `u` flag.
 */

import type { ParsedCourse, ParseMode } from './models';

const GRADE_PATTERN =
	/([A-Za-z0-9]{3}-?[0-9]{3}[A-Za-z]?[0-9]*[A-Za-z]?)\s+([A-Za-z0-9\s:()&\-.,'/*]+?)\s+([0-9]+)\s+([0-9]+)\s+([A-D][+]?|[FWPSUGE])/dg;

const REGISTRATION_PATTERN =
	/([A-Za-z0-9]{3}-?[0-9]{3}[A-Za-z]?[0-9]*[A-Za-z]?)\s+([A-Za-z0-9\s:()&\-.,'/*]+?)\s+([0-9]+)\s+([A-Z][0-9]?[A-Z]?)\s+([0-9]+(?:\.[0-9]+)?)/dg;

const SEMESTER_HEADER_PATTERN = /ภาคการศึกษา\s*([0-9]{1,2}\/[0-9]{4})/gu;

type CourseFormat = 'grade' | 'registration';

interface SemesterBoundary {
	readonly position: number;
	readonly semester: string;
}

function collectSemesterBoundaries(text: string): SemesterBoundary[] {
	const out: SemesterBoundary[] = [];
	for (const match of text.matchAll(SEMESTER_HEADER_PATTERN)) {
		const start = match.index ?? 0;
		const sem = match[1];
		if (sem !== undefined) out.push({ position: start, semester: sem });
	}
	return out;
}

function semesterFor(boundaries: SemesterBoundary[], coursePos: number): string | null {
	for (let i = boundaries.length - 1; i >= 0; i--) {
		const b = boundaries[i];
		if (b.position <= coursePos) return b.semester;
	}
	return null;
}

/**
 * Code normalization: trim suffix (e.g. 890-103G1 → 890-103) and apply legacy
 * mappings. Char-aware via spread to safely handle Thai code points without
 * splitting multi-unit characters.
 */
export function normalizeCourseCode(rawCode: string): string {
	const codePoints = [...rawCode];
	const hasDashAt3 = codePoints[3] === '-';
	const trimmed = codePoints.length >= 7 && hasDashAt3 ? codePoints.slice(0, 7).join('') : rawCode;

	switch (trimmed) {
		case '890-001':
			return '890-101';
		case '890-002':
			return '890-102';
		case '890-003':
			return '890-103';
		case '890-004':
			return '890-104';
		case '890-005':
			return '890-105';
		default:
			return trimmed;
	}
}

function isRepeatableSpecialTopic(code: string): boolean {
	return (
		code.startsWith('344-49') &&
		code !== '344-491' &&
		code !== '344-492' &&
		code !== '344-493' &&
		code !== '344-494' &&
		code !== '344-495'
	);
}

function collectCourses(text: string, pattern: RegExp, format: CourseFormat): ParsedCourse[] {
	const courses: ParsedCourse[] = [];
	const specialTopicsCount = new Map<string, number>();
	const boundaries = collectSemesterBoundaries(text);

	for (const match of text.matchAll(pattern)) {
		const rawCode = match[1];
		const rawName = match[2];
		if (rawCode === undefined || rawName === undefined) continue;

		let creditStr: string | undefined;
		let grade: string;
		if (format === 'grade') {
			creditStr = match[4];
			grade = (match[5] ?? '').toUpperCase();
		} else {
			creditStr = match[5];
			grade = 'IP';
		}
		if (creditStr === undefined) continue;

		const name = rawName.trim();
		const parsedCreditNum = Number.parseFloat(creditStr);
		const parsedCredit = Number.isFinite(parsedCreditNum) ? parsedCreditNum : 3.0;
		const normalizedCode = normalizeCourseCode(rawCode);

		let finalName: string;
		if (isRepeatableSpecialTopic(normalizedCode)) {
			const next = (specialTopicsCount.get(normalizedCode) ?? 0) + 1;
			specialTopicsCount.set(normalizedCode, next);
			finalName = `${name} (Topic ${next})`;
		} else {
			finalName = name;
		}

		// `match.indices` is populated because patterns use the `d` flag.
		const codeStart = match.indices?.[1]?.[0] ?? match.index ?? 0;
		const semester = semesterFor(boundaries, codeStart);

		courses.push({
			code: normalizedCode,
			name: finalName,
			grade,
			parsed_credit: parsedCredit,
			semester
		});
	}

	return courses;
}

/**
 * Parses transcript text into structured course entries. Returns the detected
 * `ParseMode` alongside the courses so callers can distinguish registration
 * (no grades issued yet) from grade-report results.
 */
export function parseTranscript(text: string): {
	courses: ParsedCourse[];
	mode: ParseMode;
} {
	const gradeCourses = collectCourses(text, GRADE_PATTERN, 'grade');
	if (gradeCourses.length > 0) {
		return { courses: gradeCourses, mode: 'grade' };
	}
	const registrationCourses = collectCourses(text, REGISTRATION_PATTERN, 'registration');
	const mode: ParseMode = registrationCourses.length === 0 ? 'grade' : 'registration';
	return { courses: registrationCourses, mode };
}
