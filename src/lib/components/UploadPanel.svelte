<script lang="ts">
	import { auditStore } from '$lib/stores/auditStore.svelte';
	import { settingsStore } from '$lib/stores/settingsStore.svelte';
	import { t } from '$lib/i18n';
	import { extractTextFromPdf } from '$lib/pdf/extract';
	import { parseTranscript } from '$lib/audit/parser';

	let file = $state<File | null>(null);
	let isDragOver = $state(false);

	function pickFile(f: File | null | undefined) {
		if (!f) return;
		if (!f.name.toLowerCase().endsWith('.pdf')) {
			auditStore.errorMsg = 'Please choose a PDF file.';
			return;
		}
		file = f;
		auditStore.fileName = f.name;
		auditStore.errorMsg = null;
	}

	function onChange(e: Event) {
		const input = e.target as HTMLInputElement;
		pickFile(input.files?.[0]);
	}

	function onDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		pickFile(e.dataTransfer?.files?.[0]);
	}

	async function analyze() {
		if (!file || auditStore.isLoading) return;
		auditStore.isLoading = true;
		auditStore.errorMsg = null;
		try {
			const text = await extractTextFromPdf(file);
			const { courses, mode } = parseTranscript(text);
			auditStore.setParsed(courses, mode);
		} catch (err) {
			console.error('PDF analysis failed', err);
			auditStore.errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			auditStore.isLoading = false;
		}
	}
</script>

<section
	class="rounded-2xl border border-neutral-200 bg-white p-5 shadow-soft dark:border-neutral-800 dark:bg-neutral-900"
>
	<h2 class="mb-3 text-sm font-semibold">{t('upload.drop')}</h2>

	<label
		class="flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed px-4 py-8 text-center transition {isDragOver
			? 'border-brand-500 bg-brand-50 dark:bg-brand-950/40'
			: 'border-neutral-300 hover:border-brand-400 hover:bg-neutral-50 dark:border-neutral-700 dark:hover:bg-neutral-800/50'}"
		ondragover={(e) => {
			e.preventDefault();
			isDragOver = true;
		}}
		ondragleave={() => (isDragOver = false)}
		ondrop={onDrop}
	>
		<svg
			viewBox="0 0 24 24"
			class="mb-3 h-8 w-8 text-neutral-400"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
			<polyline points="14 2 14 8 20 8" />
			<line x1="12" y1="18" x2="12" y2="12" />
			<polyline points="9 15 12 12 15 15" />
		</svg>
		<div class="text-sm font-medium">
			{file ? file.name : t('upload.choose')}
		</div>
		<div class="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
			{t('upload.hint')}
		</div>
		<input type="file" accept="application/pdf,.pdf" class="sr-only" onchange={onChange} />
	</label>

	<button
		type="button"
		disabled={!file || auditStore.isLoading}
		onclick={analyze}
		class="mt-4 flex w-full items-center justify-center gap-2 rounded-xl bg-brand-600 px-4 py-2.5 text-sm font-semibold text-white shadow-medium transition hover:bg-brand-700 disabled:cursor-not-allowed disabled:opacity-50"
	>
		{#if auditStore.isLoading}
			<svg
				class="h-4 w-4 animate-spin"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="10" stroke-opacity="0.25" />
				<path d="M22 12a10 10 0 0 1-10 10" />
			</svg>
			{t('upload.analyzing')}
		{:else}
			{t('upload.analyze')}
		{/if}
	</button>

	<label class="mt-4 flex items-start gap-2 text-xs text-neutral-600 dark:text-neutral-400">
		<input
			type="checkbox"
			class="mt-0.5 h-4 w-4 rounded border-neutral-300"
			checked={settingsStore.saveEnabled}
			onchange={(e) => settingsStore.setSaveEnabled((e.target as HTMLInputElement).checked)}
		/>
		<span>
			<span class="block font-medium text-neutral-800 dark:text-neutral-200">
				{t('sidebar.save.label')}
			</span>
			<span>{t('sidebar.save.hint')}</span>
		</span>
	</label>
</section>
