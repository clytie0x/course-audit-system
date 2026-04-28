/**
 * Curriculum Auditing Engine.
 *
 * Mirrors src/logic/auditor.rs from the previous Rust implementation. Pure
 * function — no side effects — so the What-If simulator can re-run it on
 * every plan change.
 */

import {
	computeGpa,
	freeElectiveDedupeKey,
	isInProgress,
	isPassingGrade,
	type AuditResult,
	type Category,
	type Course,
	type GenEdCurriculum,
	type MajorCurriculum,
	type MissingCourse,
	type ParsedCourse
} from './models';
import { genEdCurriculum, majorCurriculum } from '$lib/data/curricula';

/**
 * A course counts for *matching* (fills a curriculum slot) when it's passing
 * **or** in-progress. Only passing courses contribute credits — see
 * `earnedCredits`.
 */
function matchesCurriculumSlot(parsed: ParsedCourse): boolean {
	return isPassingGrade(parsed.grade) || isInProgress(parsed.grade);
}

/**
 * The lesser of the curriculum-defined credit value and the parsed transcript
 * value. In-progress courses fill their slot but contribute 0 credits.
 */
function earnedCredits(curriculumCredits: number, parsed: ParsedCourse): number {
	if (isInProgress(parsed.grade)) return 0;
	return Math.min(curriculumCredits, parsed.parsed_credit);
}

interface GenEdAuditOutcome {
	completedCredits: number;
	missing: MissingCourse[];
	used: Set<number>;
}

interface MajorAuditOutcome {
	completedCredits: number;
	electiveCredits: number;
	missing: MissingCourse[];
	used: Set<number>;
}

/** Find the first parsed-courses index that matches a target code and isn't used. */
function findUnusedMatch(
	courses: ParsedCourse[],
	used: Set<number>,
	code: string
): { idx: number; parsed: ParsedCourse } | null {
	for (let i = 0; i < courses.length; i++) {
		if (used.has(i)) continue;
		const parsed = courses[i];
		if (parsed.code === code && matchesCurriculumSlot(parsed)) {
			return { idx: i, parsed };
		}
	}
	return null;
}

export function auditGenEd(
	courses: ParsedCourse[],
	curriculum: GenEdCurriculum
): GenEdAuditOutcome {
	let completedCredits = 0;
	const missing: MissingCourse[] = [];
	const used = new Set<number>();
	let genEdElectiveTotalCredits = 0;
	const cat = 'General Education';

	for (const strand of curriculum.strands) {
		const rule = strand.selection_rule ?? 'choose_all';

		if (rule === 'choose_sequential_pair') {
			let satisfied = false;
			const strandCourses = strand.courses ?? [];
			const sequenceGroups = strand.sequence_groups ?? [];

			outer: for (const pair of sequenceGroups) {
				if (pair.length !== 2) continue;

				const foundIndices: number[] = [];
				let creditsSum = 0;
				const tentative = new Set(used);
				for (const code of pair) {
					const def = strandCourses.find((c) => c.code === code);
					if (!def) continue;
					const found = findUnusedMatch(courses, tentative, code);
					if (found) {
						foundIndices.push(found.idx);
						tentative.add(found.idx);
						creditsSum += earnedCredits(def.credits, found.parsed);
					}
				}

				if (foundIndices.length === 2) {
					for (const i of foundIndices) used.add(i);
					completedCredits += creditsSum;
					satisfied = true;
					break outer;
				}
			}

			if (!satisfied && sequenceGroups.length > 0) {
				const pairText = sequenceGroups
					.filter((p) => p.length === 2)
					.map((p) => `${p[0]} + ${p[1]}`)
					.join(' OR ');
				missing.push({
					category: cat,
					description: `${strand.name}: choose one pair (${pairText})`
				});
			}
		} else if (rule === 'choose_one') {
			const strandCourses = strand.courses ?? [];
			let chosen: { idx: number; credits: number } | null = null;
			for (const c of strandCourses) {
				const found = findUnusedMatch(courses, used, c.code);
				if (found) {
					chosen = { idx: found.idx, credits: earnedCredits(c.credits, found.parsed) };
					break;
				}
			}
			if (chosen) {
				completedCredits += chosen.credits;
				used.add(chosen.idx);
			} else {
				const options = strandCourses.map((c) => `${c.code} - ${c.name}`).join(' OR ');
				missing.push({
					category: cat,
					description: `${strand.name}: choose 1 (${options})`
				});
			}
		} else if (rule === 'choose_all_sub_groups') {
			const subGroups = strand.sub_groups ?? [];
			for (const sg of subGroups) {
				let sgCredits = 0;
				for (const c of sg.courses) {
					if (sgCredits >= sg.required_credits) break;
					const found = findUnusedMatch(courses, used, c.code);
					if (found) {
						const earned = earnedCredits(c.credits, found.parsed);
						completedCredits += earned;
						sgCredits += earned;
						used.add(found.idx);
					}
				}
				if (sgCredits < sg.required_credits) {
					const options = sg.courses.map((c) => `${c.code} - ${c.name}`).join(' OR ');
					const remaining = (sg.required_credits - sgCredits).toFixed(1);
					missing.push({
						category: cat,
						description: `${strand.name} > ${sg.name}: missing ${remaining} credits (options: ${options})`
					});
				}
			}
		} else {
			// 'choose_all' (default)
			const strandCourses = strand.courses ?? [];
			for (const c of strandCourses) {
				const found = findUnusedMatch(courses, used, c.code);
				if (found) {
					const earned = earnedCredits(c.credits, found.parsed);
					completedCredits += earned;
					used.add(found.idx);
				} else {
					missing.push({
						category: cat,
						description: `${strand.name}: ${c.code} - ${c.name}`
					});
				}
			}
		}
	}

	for (const sub of curriculum.electives.sub_categories) {
		let subCredits = 0;
		for (const c of sub.courses) {
			const found = findUnusedMatch(courses, used, c.code);
			if (found) {
				const earned = earnedCredits(c.credits, found.parsed);
				completedCredits += earned;
				genEdElectiveTotalCredits += earned;
				subCredits += earned;
				used.add(found.idx);
			}
		}
		if (subCredits < sub.required_credits) {
			const remaining = (sub.required_credits - subCredits).toFixed(1);
			missing.push({
				category: cat,
				description: `GenEd Elective > ${sub.name}: missing ${remaining} credits`
			});
		}
	}

	if (genEdElectiveTotalCredits < curriculum.electives.total_required_credits) {
		const remaining = (
			curriculum.electives.total_required_credits - genEdElectiveTotalCredits
		).toFixed(1);
		missing.push({
			category: cat,
			description: `${curriculum.electives.name}: missing ${remaining} credits`
		});
	}

	if (completedCredits < curriculum.total_required_credits) {
		const hasSummary = missing.some(
			(m) => m.category === cat && m.description.startsWith('Overall General Education')
		);
		if (!hasSummary) {
			const remaining = (curriculum.total_required_credits - completedCredits).toFixed(1);
			missing.push({
				category: cat,
				description: `Overall General Education: missing ${remaining} credits`
			});
		}
	}

	return { completedCredits, missing, used };
}

export function auditMajor(
	courses: ParsedCourse[],
	curriculum: MajorCurriculum
): MajorAuditOutcome {
	let completedCredits = 0;
	let electiveCredits = 0;
	const missing: MissingCourse[] = [];
	const used = new Set<number>();

	for (const c of curriculum.basic_science.courses) {
		const found = findUnusedMatch(courses, used, c.code);
		if (found) {
			completedCredits += earnedCredits(c.credits, found.parsed);
			used.add(found.idx);
		} else {
			missing.push({ category: 'Basic Science', description: `${c.code} - ${c.name}` });
		}
	}

	for (const c of curriculum.core_courses.courses) {
		const found = findUnusedMatch(courses, used, c.code);
		if (found) {
			completedCredits += earnedCredits(c.credits, found.parsed);
			used.add(found.idx);
		} else {
			missing.push({ category: 'Core Courses', description: `${c.code} - ${c.name}` });
		}
	}

	let capstoneDone = false;
	for (const opt of curriculum.capstone.options) {
		const found = findUnusedMatch(courses, used, opt.code);
		if (found) {
			completedCredits += earnedCredits(opt.credits, found.parsed);
			used.add(found.idx);
			capstoneDone = true;
			break;
		}
	}
	if (!capstoneDone) {
		const desc = curriculum.capstone.options.map((o) => `${o.code} (${o.name})`).join(' OR ');
		missing.push({ category: 'Capstone', description: `Choose 1: ${desc}` });
	}

	let completedClusters = 0;
	for (const domain of curriculum.electives.domains) {
		for (const cluster of domain.clusters) {
			let foundInCluster = 0;
			for (const c of cluster.courses) {
				const found = findUnusedMatch(courses, used, c.code);
				if (found) {
					electiveCredits += earnedCredits(c.credits, found.parsed);
					used.add(found.idx);
					foundInCluster += 1;
				} else {
					// Course taken but used elsewhere (or duplicate). Still counts toward
					// cluster completion even though it doesn't add credits.
					const exists = courses.some((p) => p.code === c.code && matchesCurriculumSlot(p));
					if (exists) foundInCluster += 1;
				}
			}
			if (foundInCluster >= cluster.min_courses) completedClusters += 1;
		}
	}

	if (completedClusters < curriculum.electives.clusters_to_complete) {
		missing.push({
			category: 'Major Electives',
			description: `Required: ${curriculum.electives.clusters_to_complete} Clusters, Completed: ${completedClusters}. Please complete all courses within at least ${curriculum.electives.clusters_to_complete} clusters.`
		});
	}

	// Greedy match "others" electives so repeated special topics accumulate credits.
	for (const c of curriculum.electives.others) {
		for (let i = 0; i < courses.length; i++) {
			if (used.has(i)) continue;
			const parsed = courses[i];
			if (parsed.code === c.code && matchesCurriculumSlot(parsed)) {
				electiveCredits += earnedCredits(c.credits, parsed);
				used.add(i);
			}
		}
	}

	return { completedCredits, electiveCredits, missing, used };
}

export function calculateFreeElectives(
	courses: ParsedCourse[],
	usedIndices: Set<number>
): { credits: number; list: string[] } {
	let credits = 0;
	const list: string[] = [];
	const seen = new Set<string>();

	for (let i = 0; i < courses.length; i++) {
		if (usedIndices.has(i)) continue;
		const parsed = courses[i];
		if (!isPassingGrade(parsed.grade)) continue;
		const key = freeElectiveDedupeKey(parsed.code, parsed.name);
		if (seen.has(key)) continue;
		seen.add(key);
		const c = parsed.parsed_credit;
		credits += c;
		list.push(`${parsed.code} (Grade: ${parsed.grade}, ${c} cr)`);
	}
	return { credits, list };
}

/**
 * Runs the full audit pipeline (GenEd → Major → free electives → GPA) over a
 * course list and returns the UI-facing AuditResult.
 */
export function buildAudit(
	parsed: ParsedCourse[],
	genEd: GenEdCurriculum = genEdCurriculum,
	major: MajorCurriculum = majorCurriculum
): AuditResult {
	const ge = auditGenEd(parsed, genEd);
	const mj = auditMajor(parsed, major);

	const allUsed = new Set<number>();
	ge.used.forEach((i) => allUsed.add(i));
	mj.used.forEach((i) => allUsed.add(i));

	const fe = calculateFreeElectives(parsed, allUsed);

	let allMissing: MissingCourse[] = [...ge.missing, ...mj.missing];
	allMissing = allMissing.filter((m) => {
		if (m.category === 'General Education') {
			return ge.completedCredits < genEd.total_required_credits;
		}
		return true;
	});

	const totalCredits = ge.completedCredits + mj.completedCredits + mj.electiveCredits + fe.credits;

	const genEdCourses: Course[] = [];
	const majorCourses: Course[] = [];
	const freeElectiveCourses: Course[] = [];
	const seenFree = new Set<string>();

	for (let i = 0; i < parsed.length; i++) {
		const p = parsed[i];
		const course: Course = {
			code: p.code,
			name: p.name,
			credit: p.parsed_credit,
			grade: p.grade,
			semester: p.semester
		};
		if (ge.used.has(i)) {
			genEdCourses.push(course);
		} else if (mj.used.has(i)) {
			majorCourses.push(course);
		} else if (isPassingGrade(p.grade)) {
			const key = freeElectiveDedupeKey(p.code, p.name);
			if (!seenFree.has(key)) {
				seenFree.add(key);
				freeElectiveCourses.push(course);
			}
		}
	}

	const graded: Course[] = [...genEdCourses, ...majorCourses, ...freeElectiveCourses];
	const gpa = computeGpa(graded);

	const categories: Category[] = [
		{
			name: 'General Education',
			required_credits: genEd.total_required_credits,
			collected_credits: ge.completedCredits,
			courses: genEdCourses
		},
		{
			name: 'Major Courses',
			required_credits: major.total_required_credits,
			collected_credits: mj.completedCredits + mj.electiveCredits,
			courses: majorCourses
		},
		{
			name: 'Free Electives',
			required_credits: 6.0,
			collected_credits: fe.credits,
			courses: freeElectiveCourses
		}
	];

	return {
		total_credits: totalCredits,
		categories,
		missing_subjects: allMissing,
		gpa
	};
}
