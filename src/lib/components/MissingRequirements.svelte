<script lang="ts">
	import type { MissingCourse } from '$lib/audit/models';
	import { t, tCategory } from '$lib/i18n';
	import { auditStore } from '$lib/stores/auditStore.svelte';
	import { copyMissingToClipboard } from '$lib/utils/export';

	let { missing }: { missing: MissingCourse[] } = $props();

	let copied = $state(false);

	const grouped = $derived(() => {
		const byCat = new Map<string, MissingCourse[]>();
		for (const m of missing) {
			const arr = byCat.get(m.category) ?? [];
			arr.push(m);
			byCat.set(m.category, arr);
		}
		return [...byCat.entries()];
	});

	async function copy() {
		if (!auditStore.auditResult) return;
		await copyMissingToClipboard(auditStore.auditResult);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<section
	class="rounded-2xl border border-rose-200 bg-rose-50/50 p-5 shadow-soft dark:border-rose-900/50 dark:bg-rose-950/20"
>
	<div class="mb-3 flex items-center justify-between gap-2">
		<h2 class="text-sm font-semibold text-rose-800 dark:text-rose-300">
			{t('missing.title')}
		</h2>
		{#if missing.length > 0}
			<button
				type="button"
				onclick={copy}
				class="rounded-lg px-2.5 py-1 text-2xs font-medium text-rose-700 ring-1 ring-rose-300 hover:bg-rose-100 dark:text-rose-300 dark:ring-rose-900 dark:hover:bg-rose-900/40 print:hidden"
			>
				{copied ? t('results.copied') : t('results.copyMissing')}
			</button>
		{/if}
	</div>

	{#if missing.length === 0}
		<p class="text-xs text-emerald-700 dark:text-emerald-300">
			{t('missing.empty')}
		</p>
	{:else}
		{#each grouped() as [category, items] (category)}
			<div class="mb-3 last:mb-0">
				<div
					class="text-xs font-semibold uppercase tracking-wider text-rose-700 dark:text-rose-300"
				>
					{tCategory(category)}
				</div>
				<ul class="mt-1.5 space-y-1.5 text-sm text-neutral-800 dark:text-neutral-200">
					{#each items as item, i (i)}
						<li class="flex gap-2">
							<span class="text-rose-500" aria-hidden="true">•</span>
							<span>{item.description}</span>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	{/if}
</section>
