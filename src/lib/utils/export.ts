import type { AuditResult } from '$lib/audit/models';

export function exportJson(result: AuditResult, filename = 'course-audit.json'): void {
	const json = JSON.stringify(result, null, 2);
	const blob = new Blob([json], { type: 'application/json' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	a.click();
	URL.revokeObjectURL(url);
}

export async function copyMissingToClipboard(result: AuditResult): Promise<void> {
	const lines = result.missing_subjects.map((m) => `[${m.category}] ${m.description}`);
	const text = lines.join('\n');
	await navigator.clipboard.writeText(text);
}

export function printPage(): void {
	window.print();
}
