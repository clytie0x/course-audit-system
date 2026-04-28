<script lang="ts">
	import type { Course } from '$lib/audit/models';
	import { computeGpa } from '$lib/audit/models';
	import { t } from '$lib/i18n';
	import { formatCredit, formatGpa, gradeColor } from '$lib/utils/format';

	let { courses }: { courses: Course[] } = $props();

	const semesters = $derived(() => {
		const groups = new Map<string | null, Course[]>();
		for (const c of courses) {
			const key = c.semester ?? null;
			const arr = groups.get(key) ?? [];
			arr.push(c);
			groups.set(key, arr);
		}
		return [...groups.entries()].sort(([a], [b]) => {
			if (a === null) return 1;
			if (b === null) return -1;
			return a.localeCompare(b);
		});
	});

	function totalCredits(list: Course[]): number {
		return list.reduce((s, c) => s + c.credit, 0);
	}
</script>

<section
	class="rounded-2xl border border-neutral-200 bg-white p-5 shadow-soft dark:border-neutral-800 dark:bg-neutral-900"
>
	<h2 class="mb-3 text-sm font-semibold">{t('semester.title')}</h2>

	{#each semesters() as [sem, list] (sem ?? '__none__')}
		{@const gpa = computeGpa(list)}
		<details class="group mb-2 rounded-xl bg-neutral-50 last:mb-0 dark:bg-neutral-800/40">
			<summary
				class="flex cursor-pointer items-center justify-between gap-3 px-4 py-2.5 text-sm font-medium"
			>
				<span class="flex items-center gap-2">
					<span>{sem ?? t('semester.unspecified')}</span>
					<span class="text-xs font-normal text-neutral-500 dark:text-neutral-400">
						{formatCredit(totalCredits(list))}
						{t('semester.credits')}
					</span>
				</span>
				<span class="text-xs text-neutral-500 dark:text-neutral-400">
					{t('semester.gpa')}
					{formatGpa(gpa)}
				</span>
			</summary>
			<ul class="divide-y divide-neutral-200 px-4 py-2 dark:divide-neutral-800">
				{#each list as c, i (c.code + i)}
					<li class="flex items-center gap-3 py-2">
						<div class="min-w-0 flex-1">
							<div class="truncate text-sm font-medium">{c.code}</div>
							<div class="truncate text-xs text-neutral-500 dark:text-neutral-400">
								{c.name}
							</div>
						</div>
						<span class="text-xs text-neutral-500 dark:text-neutral-400">
							{formatCredit(c.credit)} cr
						</span>
						<span class="rounded-md px-2 py-0.5 text-2xs font-semibold {gradeColor(c.grade)}">
							{c.grade}
						</span>
					</li>
				{/each}
			</ul>
		</details>
	{/each}
</section>
