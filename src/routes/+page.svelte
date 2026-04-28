<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import UploadPanel from '$lib/components/UploadPanel.svelte';
	import ResultsPanel from '$lib/components/ResultsPanel.svelte';
	import { auditStore } from '$lib/stores/auditStore.svelte';
	import { t } from '$lib/i18n';
</script>

<div class="min-h-full">
	<Header />

	<main class="mx-auto grid max-w-7xl gap-6 px-6 py-6 lg:grid-cols-[320px_1fr]">
		<aside class="space-y-4 print:hidden">
			<UploadPanel />

			{#if auditStore.errorMsg}
				<div
					class="rounded-xl border border-rose-300 bg-rose-50 px-4 py-2.5 text-xs text-rose-800 dark:border-rose-900/60 dark:bg-rose-950/40 dark:text-rose-200"
					role="alert"
				>
					<div class="font-semibold">{t('error.title')}</div>
					<div class="mt-0.5">{auditStore.errorMsg}</div>
				</div>
			{/if}
		</aside>

		<section>
			{#if auditStore.auditResult}
				<ResultsPanel result={auditStore.auditResult} />
			{:else}
				<div
					class="flex h-64 flex-col items-center justify-center rounded-2xl border border-dashed border-neutral-300 p-8 text-center dark:border-neutral-700"
				>
					<svg
						viewBox="0 0 24 24"
						class="mb-3 h-10 w-10 text-neutral-300 dark:text-neutral-700"
						fill="none"
						stroke="currentColor"
						stroke-width="1.25"
						aria-hidden="true"
					>
						<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
						<polyline points="14 2 14 8 20 8" />
					</svg>
					<div class="text-sm font-semibold">{t('results.empty.title')}</div>
					<div class="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
						{t('results.empty.body')}
					</div>
				</div>
			{/if}
		</section>
	</main>
</div>
