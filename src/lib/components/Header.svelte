<script lang="ts">
	import { settingsStore } from '$lib/stores/settingsStore.svelte';
	import { t } from '$lib/i18n';
</script>

<header
	class="sticky top-0 z-30 border-b border-neutral-200 bg-white/80 backdrop-blur dark:border-neutral-800 dark:bg-neutral-950/80"
>
	<div class="mx-auto flex max-w-7xl items-center justify-between gap-4 px-6 py-3">
		<div class="flex items-center gap-3">
			<div
				class="flex h-9 w-9 items-center justify-center rounded-xl bg-brand-600 text-white shadow-glow-brand"
			>
				<svg viewBox="0 0 24 24" class="h-5 w-5" fill="currentColor" aria-hidden="true">
					<path d="M12 3l9 5-9 5-9-5 9-5zm-9 8l9 5 9-5v3l-9 5-9-5v-3z" />
				</svg>
			</div>
			<div class="leading-tight">
				<div class="text-sm font-semibold">{t('app.title')}</div>
				<div class="text-2xs text-neutral-500 dark:text-neutral-400">PSU Computer Science</div>
			</div>
		</div>

		<div class="flex items-center gap-2">
			<span
				class="hidden items-center gap-1 rounded-full bg-emerald-50 px-2.5 py-1 text-2xs font-medium text-emerald-700 ring-1 ring-emerald-200 sm:inline-flex dark:bg-emerald-950/60 dark:text-emerald-300 dark:ring-emerald-900"
			>
				<span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
				{t('app.privacyBadge')}
			</span>

			<button
				type="button"
				class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-neutral-700 ring-1 ring-neutral-200 hover:bg-neutral-100 dark:text-neutral-300 dark:ring-neutral-800 dark:hover:bg-neutral-800"
				aria-label={t('header.toggleLang')}
				onclick={() => settingsStore.toggleLang()}
			>
				{settingsStore.isThai ? 'EN' : 'ไทย'}
			</button>

			<button
				type="button"
				class="rounded-lg p-2 text-neutral-700 ring-1 ring-neutral-200 hover:bg-neutral-100 dark:text-neutral-300 dark:ring-neutral-800 dark:hover:bg-neutral-800"
				aria-label={settingsStore.isDark
					? t('header.toggleTheme.light')
					: t('header.toggleTheme.dark')}
				onclick={() => settingsStore.toggleDark()}
			>
				{#if settingsStore.isDark}
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
						<circle cx="12" cy="12" r="4" />
						<path
							d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
						/>
					</svg>
				{:else}
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
						<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
					</svg>
				{/if}
			</button>
		</div>
	</div>
</header>
