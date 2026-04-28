<script lang="ts">
	import type { AuditResult } from '$lib/audit/models';
	import HeroCard from './HeroCard.svelte';
	import CategoryCard from './CategoryCard.svelte';
	import WhatIfSimulator from './WhatIfSimulator.svelte';
	import SemesterBreakdown from './SemesterBreakdown.svelte';
	import MissingRequirements from './MissingRequirements.svelte';
	import { t } from '$lib/i18n';
	import { auditStore } from '$lib/stores/auditStore.svelte';
	import { exportJson, printPage } from '$lib/utils/export';

	let { result }: { result: AuditResult } = $props();

	const allCourses = $derived(result.categories.flatMap((c) => c.courses));
</script>

<div class="space-y-5">
	<div class="flex flex-wrap items-center gap-2 print:hidden">
		<button
			type="button"
			onclick={() => exportJson(result)}
			class="rounded-lg px-3 py-1.5 text-xs font-medium text-neutral-700 ring-1 ring-neutral-300 hover:bg-neutral-100 dark:text-neutral-300 dark:ring-neutral-700 dark:hover:bg-neutral-800"
		>
			{t('results.export.json')}
		</button>
		<button
			type="button"
			onclick={printPage}
			class="rounded-lg px-3 py-1.5 text-xs font-medium text-neutral-700 ring-1 ring-neutral-300 hover:bg-neutral-100 dark:text-neutral-300 dark:ring-neutral-700 dark:hover:bg-neutral-800"
		>
			{t('results.export.print')}
		</button>
		<button
			type="button"
			onclick={() => auditStore.reset()}
			class="ml-auto rounded-lg px-3 py-1.5 text-xs font-medium text-rose-700 ring-1 ring-rose-300 hover:bg-rose-50 dark:text-rose-300 dark:ring-rose-900 dark:hover:bg-rose-950/40"
		>
			{t('reset.label')}
		</button>
	</div>

	{#if auditStore.parseMode === 'registration'}
		<div
			class="rounded-xl border border-amber-300 bg-amber-50 px-4 py-2.5 text-xs text-amber-800 dark:border-amber-900/60 dark:bg-amber-950/40 dark:text-amber-200"
			role="alert"
		>
			{t('results.registration.banner')}
		</div>
	{/if}

	<HeroCard {result} />

	<div class="grid gap-4 md:grid-cols-3">
		{#each result.categories as category (category.name)}
			<CategoryCard {category} />
		{/each}
	</div>

	<MissingRequirements missing={result.missing_subjects} />

	<WhatIfSimulator />

	<SemesterBreakdown courses={allCourses} />
</div>
