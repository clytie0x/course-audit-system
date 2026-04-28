import { describe, it, expect } from 'vitest';
import { parseTranscript, normalizeCourseCode } from '../../src/lib/audit/parser';
import { isInProgress, isPassingGrade } from '../../src/lib/audit/models';

describe('normalizeCourseCode', () => {
	it('trims suffix', () => {
		expect(normalizeCourseCode('890-103G1')).toBe('890-103');
		expect(normalizeCourseCode('344-101')).toBe('344-101');
	});

	it('maps legacy codes', () => {
		expect(normalizeCourseCode('890-001')).toBe('890-101');
		expect(normalizeCourseCode('890-002')).toBe('890-102');
		expect(normalizeCourseCode('890-003')).toBe('890-103');
		expect(normalizeCourseCode('890-004')).toBe('890-104');
		expect(normalizeCourseCode('890-005')).toBe('890-105');
	});

	it('leaves short codes alone', () => {
		expect(normalizeCourseCode('X')).toBe('X');
	});

	it('does not panic on unicode input', () => {
		expect(normalizeCourseCode('๓๒๒-๑๐๑G1')).toBe('๓๒๒-๑๐๑');
		expect(normalizeCourseCode('abc-๑๒๓')).toBe('abc-๑๒๓');
	});
});

describe('parseTranscript — grade format', () => {
	it('extracts a basic course', () => {
		const text = '322-101 CALCULUS I 04 3 B+';
		const { courses, mode } = parseTranscript(text);
		expect(mode).toBe('grade');
		expect(courses).toHaveLength(1);
		expect(courses[0].code).toBe('322-101');
		expect(courses[0].name).toBe('CALCULUS I');
		expect(courses[0].grade).toBe('B+');
		expect(courses[0].parsed_credit).toBe(3.0);
	});

	it('normalizes legacy code', () => {
		const text = '890-001 ENGLISH I 01 3 A';
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(1);
		expect(courses[0].code).toBe('890-101');
	});

	it('numbers repeated special topics', () => {
		const text = '344-496 TOPIC ALPHA 01 3 A 344-496 TOPIC BETA 02 3 B';
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(2);
		expect(courses[0].name.endsWith('(Topic 1)')).toBe(true);
		expect(courses[1].name.endsWith('(Topic 2)')).toBe(true);
	});

	it('does not tag non-repeatable 344-49x', () => {
		const text = '344-491 SENIOR PROJECT 01 3 A';
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(1);
		expect(courses[0].name).toBe('SENIOR PROJECT');
		expect(courses[0].name).not.toContain('Topic');
	});

	it('recognizes non-letter grades', () => {
		const text = '200-101 COURSE A 01 3 W 200-102 COURSE B 02 3 P';
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(2);
		expect(courses[0].grade).toBe('W');
		expect(courses[1].grade).toBe('P');
	});

	it('ignores malformed lines', () => {
		const text = 'This line has no course at all.';
		const { courses, mode } = parseTranscript(text);
		expect(courses).toHaveLength(0);
		expect(mode).toBe('grade');
	});
});

describe('parseTranscript — registration format', () => {
	it('tags courses as IP', () => {
		const text = '145-101 COMPANION ANIMALS 02 C 3.0';
		const { courses, mode } = parseTranscript(text);
		expect(mode).toBe('registration');
		expect(courses).toHaveLength(1);
		expect(courses[0].code).toBe('145-101');
		expect(courses[0].grade).toBe('IP');
		expect(courses[0].parsed_credit).toBe(3.0);
	});

	it('handles category suffixes', () => {
		const text = [
			'315-104G4 DIGITAL TECH 05 G4 2.0',
			'895-222G2B CRITICAL THINKING 02 G2B 2.0',
			'B03-001G4 GET SMART ON CYBER THREATS 01 G4 2.0'
		].join('\n');
		const { courses, mode } = parseTranscript(text);
		expect(mode).toBe('registration');
		expect(courses).toHaveLength(3);
		expect(courses.every((c) => c.grade === 'IP')).toBe(true);
		expect(courses[0].code).toBe('315-104');
		expect(courses[2].code).toBe('B03-001');
	});

	it('grade format takes priority when both present', () => {
		const text = ['145-101 COMPANION ANIMALS 02 C 3.0', '322-101 CALCULUS I 04 3 B+'].join('\n');
		const { courses, mode } = parseTranscript(text);
		expect(mode).toBe('grade');
		expect(courses.every((c) => c.grade !== 'IP')).toBe(true);
	});
});

describe('grade helpers', () => {
	it('isInProgress detects IP', () => {
		expect(isInProgress('IP')).toBe(true);
		expect(isInProgress(' ip ')).toBe(true);
		expect(isInProgress('A')).toBe(false);
		expect(isInProgress('')).toBe(false);
	});

	it('isPassingGrade rejects IP', () => {
		expect(isPassingGrade('IP')).toBe(false);
		expect(isPassingGrade('A')).toBe(true);
		expect(isPassingGrade('F')).toBe(false);
	});
});

describe('semester tagging', () => {
	it('tags courses with semester header', () => {
		const text = [
			'ภาคการศึกษา 1/2567',
			'322-101 CALCULUS I 04 3 B+',
			'ภาคการศึกษา 2/2567',
			'322-102 CALCULUS II 01 3 A'
		].join('\n');
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(2);
		expect(courses[0].semester).toBe('1/2567');
		expect(courses[1].semester).toBe('2/2567');
	});

	it('leaves semester null before first header', () => {
		const text = '322-101 CALCULUS I 04 3 B+';
		const { courses } = parseTranscript(text);
		expect(courses[0].semester).toBeNull();
	});

	it('tags registration semesters too', () => {
		const text = ['ภาคการศึกษา 2/2568', '145-101 COMPANION ANIMALS 02 C 3.0'].join('\n');
		const { courses, mode } = parseTranscript(text);
		expect(mode).toBe('registration');
		expect(courses[0].semester).toBe('2/2568');
	});

	it('does not panic on Thai text after semester header', () => {
		const text = [
			'ภาคการศึกษา 1/2567',
			'รหัสวิชา ชื่อวิชา ตอน หน่วยกิต ผลการเรียน',
			'322-101 CALCULUS I 04 3 B+',
			'ภาคการศึกษา 2/2567',
			'322-102 CALCULUS II 01 3 A'
		].join('\n');
		const { courses } = parseTranscript(text);
		expect(courses).toHaveLength(2);
		expect(courses[0].semester).toBe('1/2567');
		expect(courses[1].semester).toBe('2/2567');
	});
});
