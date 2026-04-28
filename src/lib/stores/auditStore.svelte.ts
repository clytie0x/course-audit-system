import { browser } from '$app/environment';
import { buildAudit } from '$lib/audit/auditor';
import type { AuditResult, ParsedCourse, ParseMode } from '$lib/audit/models';
import { loadSavedAudit, loadSavedMode, persistence } from './persistence';
import { settingsStore } from './settingsStore.svelte';

function createAuditStore() {
	let fileName = $state<string | null>(null);
	let parseMode = $state<ParseMode>('grade');
	let parsedCourses = $state<ParsedCourse[]>([]);
	let plannedCourses = $state<ParsedCourse[]>([]);
	let auditResult = $state<AuditResult | null>(null);
	let isLoading = $state(false);
	let errorMsg = $state<string | null>(null);

	if (browser) {
		const saved = loadSavedAudit();
		const savedMode = loadSavedMode();
		if (saved) auditResult = saved;
		if (savedMode) parseMode = savedMode;
	}

	function recompute() {
		if (parsedCourses.length === 0 && plannedCourses.length === 0) {
			auditResult = null;
			return;
		}
		const combined = [...parsedCourses, ...plannedCourses];
		const result = buildAudit(combined);
		auditResult = result;

		if (browser && settingsStore.saveEnabled) {
			persistence.writeString(persistence.keys.audit, JSON.stringify(result));
			persistence.writeString(persistence.keys.mode, parseMode);
		}
	}

	function setParsed(courses: ParsedCourse[], mode: ParseMode) {
		parsedCourses = courses;
		parseMode = mode;
		recompute();
	}

	function addPlanned(course: ParsedCourse) {
		plannedCourses = [...plannedCourses, course];
		recompute();
	}

	function removePlanned(index: number) {
		plannedCourses = plannedCourses.filter((_, i) => i !== index);
		recompute();
	}

	function clearPlanned() {
		plannedCourses = [];
		recompute();
	}

	function reset() {
		fileName = null;
		parsedCourses = [];
		plannedCourses = [];
		auditResult = null;
		errorMsg = null;
		if (browser) {
			persistence.remove(persistence.keys.audit);
			persistence.remove(persistence.keys.mode);
		}
	}

	return {
		get fileName() {
			return fileName;
		},
		set fileName(v: string | null) {
			fileName = v;
		},
		get parseMode() {
			return parseMode;
		},
		get parsedCourses() {
			return parsedCourses;
		},
		get plannedCourses() {
			return plannedCourses;
		},
		get auditResult() {
			return auditResult;
		},
		get isLoading() {
			return isLoading;
		},
		set isLoading(v: boolean) {
			isLoading = v;
		},
		get errorMsg() {
			return errorMsg;
		},
		set errorMsg(v: string | null) {
			errorMsg = v;
		},
		setParsed,
		addPlanned,
		removePlanned,
		clearPlanned,
		reset
	};
}

export const auditStore = createAuditStore();
