<script lang="ts">
	import type { Category } from '$lib/audit/models';
	import { tCategory, t } from '$lib/i18n';
	import { formatCredit, gradeColor } from '$lib/utils/format';

	let { category }: { category: Category } = $props();

	let expanded = $state(false);

	const progress = $derived(
		category.required_credits > 0
			? Math.min(100, (category.collected_credits / category.required_credits) * 100)
			: 0
	);
</script>

<section
	class="rounded-2xl border border-neutral-200 bg-white shadow-soft dark:border-neutral-800 dark:bg-neutral-900"
>
	<button
		type="button"
		class="flex w-full items-center justify-between gap-3 px-5 py-4 text-left"
		aria-expanded={expanded}
		onclick={() => (expanded = !expanded)}
	>
		<div class="min-w-0 flex-1">
			<div class="truncate text-sm font-semibold">{tCategory(category.name)}</div>
			<div class="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
				{formatCredit(category.collected_credits)} / {formatCredit(category.required_credits)}
				{t('semester.credits')}
			</div>
			<div
				class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-800"
			>
				<div
					class="h-full rounded-full bg-brand-500 transition-[width] duration-700"
					style:width="{progress}%"
				></div>
			</div>
		</div>
		<svg
			viewBox="0 0 24 24"
			class="h-4 w-4 shrink-0 text-neutral-400 transition-transform {expanded ? 'rotate-90' : ''}"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<polyline points="9 6 15 12 9 18" />
		</svg>
	</button>

	{#if expanded}
		<div class="border-t border-neutral-200 px-5 py-3 dark:border-neutral-800">
			{#if category.courses.length === 0}
				<p class="py-3 text-center text-xs text-neutral-500 dark:text-neutral-400">
					— No courses —
				</p>
			{:else}
				<ul class="divide-y divide-neutral-200 dark:divide-neutral-800">
					{#each category.courses as course (course.code + course.name)}
						<li class="flex items-center gap-3 py-2.5">
							<div class="min-w-0 flex-1">
								<div class="truncate text-sm font-medium">{course.code}</div>
								<div class="truncate text-xs text-neutral-500 dark:text-neutral-400">
									{course.name}
								</div>
							</div>
							<span class="text-xs text-neutral-500 dark:text-neutral-400">
								{formatCredit(course.credit)} cr
							</span>
							<span
								class="rounded-md px-2 py-0.5 text-2xs font-semibold {gradeColor(course.grade)}"
							>
								{course.grade}
							</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</section>
