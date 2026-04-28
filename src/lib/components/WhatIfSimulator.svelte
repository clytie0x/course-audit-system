<script lang="ts">
	import { auditStore } from '$lib/stores/auditStore.svelte';
	import { t } from '$lib/i18n';
	import { formatCredit, gradeColor } from '$lib/utils/format';

	let code = $state('');
	let name = $state('');
	let credit = $state(3);
	let grade = $state('A');

	function add() {
		if (!code.trim()) return;
		auditStore.addPlanned({
			code: code.trim(),
			name: name.trim() || code.trim(),
			grade: grade.trim() || 'A',
			parsed_credit: Number.isFinite(credit) ? credit : 3,
			semester: null
		});
		code = '';
		name = '';
		credit = 3;
		grade = 'A';
	}
</script>

<section
	class="rounded-2xl border border-neutral-200 bg-white p-5 shadow-soft dark:border-neutral-800 dark:bg-neutral-900 print:hidden"
>
	<h2 class="text-sm font-semibold">{t('whatif.title')}</h2>
	<p class="mt-1 text-xs text-neutral-500 dark:text-neutral-400">{t('whatif.body')}</p>

	<div class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-4">
		<input
			type="text"
			placeholder={t('whatif.code')}
			bind:value={code}
			class="rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm placeholder:text-neutral-400 focus:border-brand-500 focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
		/>
		<input
			type="text"
			placeholder={t('whatif.name')}
			bind:value={name}
			class="rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm placeholder:text-neutral-400 focus:border-brand-500 focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
		/>
		<input
			type="number"
			min="0"
			step="0.5"
			placeholder={t('whatif.credit')}
			bind:value={credit}
			class="rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm placeholder:text-neutral-400 focus:border-brand-500 focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
		/>
		<select
			bind:value={grade}
			class="rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm focus:border-brand-500 focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
		>
			{#each ['A', 'B+', 'B', 'C+', 'C', 'D+', 'D', 'F', 'IP'] as g (g)}
				<option value={g}>{g}</option>
			{/each}
		</select>
	</div>

	<div class="mt-3 flex flex-wrap gap-2">
		<button
			type="button"
			onclick={add}
			class="rounded-lg bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700"
		>
			{t('whatif.add')}
		</button>
		{#if auditStore.plannedCourses.length > 0}
			<button
				type="button"
				onclick={() => auditStore.clearPlanned()}
				class="rounded-lg px-3 py-1.5 text-xs font-medium text-neutral-700 ring-1 ring-neutral-300 hover:bg-neutral-100 dark:text-neutral-300 dark:ring-neutral-700 dark:hover:bg-neutral-800"
			>
				{t('whatif.clear')}
			</button>
		{/if}
	</div>

	{#if auditStore.plannedCourses.length === 0}
		<p class="mt-3 text-xs text-neutral-500 dark:text-neutral-400">{t('whatif.empty')}</p>
	{:else}
		<ul class="mt-3 divide-y divide-neutral-200 dark:divide-neutral-800">
			{#each auditStore.plannedCourses as c, i (c.code + i)}
				<li class="flex items-center gap-3 py-2">
					<div class="min-w-0 flex-1">
						<div class="truncate text-sm font-medium">{c.code}</div>
						<div class="truncate text-xs text-neutral-500 dark:text-neutral-400">{c.name}</div>
					</div>
					<span class="text-xs text-neutral-500 dark:text-neutral-400">
						{formatCredit(c.parsed_credit)} cr
					</span>
					<span class="rounded-md px-2 py-0.5 text-2xs font-semibold {gradeColor(c.grade)}">
						{c.grade}
					</span>
					<button
						type="button"
						onclick={() => auditStore.removePlanned(i)}
						class="rounded-md p-1 text-neutral-400 hover:bg-neutral-100 hover:text-rose-600 dark:hover:bg-neutral-800"
						aria-label={t('whatif.remove')}
					>
						<svg
							viewBox="0 0 24 24"
							class="h-4 w-4"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<line x1="18" y1="6" x2="6" y2="18" />
							<line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</section>
