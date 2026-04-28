import { describe, it, expect } from 'vitest';
import { auditGenEd, calculateFreeElectives, buildAudit } from '../../src/lib/audit/auditor';
import { genEdCurriculum } from '../../src/lib/data/curricula';
import type { ParsedCourse } from '../../src/lib/audit/models';

function parsed(code: string, grade: string, credit: number): ParsedCourse {
	return {
		code,
		name: code,
		grade,
		parsed_credit: credit,
		semester: null
	};
}

describe('auditGenEd', () => {
	it('IP course fills slot but adds zero credits', () => {
		const courses = [parsed('388-100', 'IP', 1.0)];
		const { completedCredits, missing, used } = auditGenEd(courses, genEdCurriculum);

		expect(used.has(0)).toBe(true);
		expect(completedCredits).toBe(0.0);
		const filledStillMissing = missing.some(
			(m) => m.description.includes('388-100') && m.description.includes('Health')
		);
		expect(filledStillMissing).toBe(false);
	});

	it('passing course adds credits', () => {
		const courses = [parsed('388-100', 'A', 1.0)];
		const { completedCredits, used } = auditGenEd(courses, genEdCurriculum);

		expect(used.has(0)).toBe(true);
		expect(completedCredits).toBe(1.0);
	});

	it('failing grade does not fill slot', () => {
		const courses = [parsed('388-100', 'F', 1.0)];
		const { completedCredits, missing, used } = auditGenEd(courses, genEdCurriculum);

		expect(used.has(0)).toBe(false);
		expect(completedCredits).toBe(0.0);
		const stillMissing = missing.some((m) => m.description.includes('388-100'));
		expect(stillMissing).toBe(true);
	});
});

describe('calculateFreeElectives', () => {
	it('IP courses excluded from free electives', () => {
		const courses = [parsed('999-999', 'IP', 3.0)];
		const used = new Set<number>();
		const { credits, list } = calculateFreeElectives(courses, used);

		expect(credits).toBe(0.0);
		expect(list).toHaveLength(0);
	});

	it('passing unused courses become free electives', () => {
		const courses = [parsed('999-999', 'A', 3.0)];
		const used = new Set<number>();
		const { credits, list } = calculateFreeElectives(courses, used);

		expect(credits).toBe(3.0);
		expect(list).toHaveLength(1);
		expect(list[0]).toContain('999-999');
	});

	it('failing courses excluded from free electives', () => {
		const courses = [parsed('999-999', 'F', 3.0)];
		const used = new Set<number>();
		const { credits } = calculateFreeElectives(courses, used);

		expect(credits).toBe(0.0);
	});
});

describe('buildAudit', () => {
	it('produces 3 categories', () => {
		const result = buildAudit([]);
		expect(result.categories).toHaveLength(3);
		expect(result.categories.map((c) => c.name)).toEqual([
			'General Education',
			'Major Courses',
			'Free Electives'
		]);
	});

	it('returns null GPA with no graded courses', () => {
		const result = buildAudit([]);
		expect(result.gpa).toBeNull();
	});
});
