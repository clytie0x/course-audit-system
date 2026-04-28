export function formatCredit(value: number): string {
	if (Number.isInteger(value)) return value.toString();
	return value.toFixed(1);
}

export function formatGpa(value: number | null | undefined): string {
	if (value === null || value === undefined) return '—';
	return value.toFixed(2);
}

export function gradeColor(grade: string): string {
	const g = grade.trim().toUpperCase();
	if (g.startsWith('A'))
		return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300';
	if (g.startsWith('B')) return 'bg-sky-100 text-sky-700 dark:bg-sky-900/40 dark:text-sky-300';
	if (g.startsWith('C'))
		return 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300';
	if (g.startsWith('D'))
		return 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300';
	if (g === 'IP') return 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-300';
	if (g === 'F' || g === 'E' || g === 'W' || g === 'U')
		return 'bg-rose-100 text-rose-700 dark:bg-rose-900/40 dark:text-rose-300';
	return 'bg-neutral-200 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-300';
}
