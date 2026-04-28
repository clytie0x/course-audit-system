import { describe, it, expect } from 'vitest';
import {
	computeGpa,
	gradePoints,
	isInProgress,
	isPassingGrade,
	freeElectiveDedupeKey,
	type Course
} from '../../src/lib/audit/models';

function course(grade: string, credit: number): Course {
	return { code: 'TEST', name: 'TEST', credit, grade, semester: null };
}

describe('gradePoints', () => {
	it('maps letter grades to GPA points', () => {
		expect(gradePoints('A')).toBe(4.0);
		expect(gradePoints('B+')).toBe(3.5);
		expect(gradePoints('C')).toBe(2.0);
		expect(gradePoints('F')).toBe(0.0);
		expect(gradePoints('E')).toBe(0.0);
		expect(gradePoints(' a ')).toBe(4.0);
	});

	it('rejects non-letter grades', () => {
		expect(gradePoints('W')).toBeNull();
		expect(gradePoints('P')).toBeNull();
		expect(gradePoints('G')).toBeNull();
		expect(gradePoints('IP')).toBeNull();
		expect(gradePoints('')).toBeNull();
	});
});

describe('computeGpa', () => {
	it('weights by credits', () => {
		const courses = [course('A', 3.0), course('B', 1.0)];
		expect(computeGpa(courses)).toBeCloseTo(3.75, 6);
	});

	it('ignores non-letter grades', () => {
		const courses = [course('A', 3.0), course('W', 3.0), course('IP', 2.0)];
		expect(computeGpa(courses)).toBe(4.0);
	});

	it('returns null when no graded courses', () => {
		const courses = [course('IP', 3.0), course('W', 3.0)];
		expect(computeGpa(courses)).toBeNull();
	});
});

describe('isPassingGrade / isInProgress', () => {
	it('IP is not passing', () => {
		expect(isPassingGrade('IP')).toBe(false);
		expect(isInProgress('IP')).toBe(true);
		expect(isInProgress('ip')).toBe(true);
		expect(isInProgress(' IP ')).toBe(true);
	});

	it('letter grades pass except F/W/U', () => {
		expect(isPassingGrade('A')).toBe(true);
		expect(isPassingGrade('D')).toBe(true);
		expect(isPassingGrade('F')).toBe(false);
		expect(isPassingGrade('W')).toBe(false);
		expect(isPassingGrade('U')).toBe(false);
	});

	it('empty grade is not passing', () => {
		expect(isPassingGrade('')).toBe(false);
	});
});

describe('freeElectiveDedupeKey', () => {
	it('uses code+name for repeatable special topics', () => {
		expect(freeElectiveDedupeKey('344-496', 'Topic A')).toBe('344-496::Topic A');
		expect(freeElectiveDedupeKey('344-499', 'Topic B')).toBe('344-499::Topic B');
	});

	it('uses code-only for project courses 344-491..495', () => {
		expect(freeElectiveDedupeKey('344-491', 'X')).toBe('344-491');
		expect(freeElectiveDedupeKey('344-495', 'X')).toBe('344-495');
	});

	it('uses code-only for other courses', () => {
		expect(freeElectiveDedupeKey('344-101', 'Calculus')).toBe('344-101');
	});
});
