<script lang="ts">
	import type { AuditResult } from '$lib/audit/models';
	import { t } from '$lib/i18n';
	import { formatCredit, formatGpa } from '$lib/utils/format';

	let { result }: { result: AuditResult } = $props();

	const totalRequired = $derived(result.categories.reduce((sum, c) => sum + c.required_credits, 0));
	const progress = $derived(
		totalRequired > 0 ? Math.min(100, (result.total_credits / totalRequired) * 100) : 0
	);
</script>

<section
	class="rounded-3xl border border-neutral-200 bg-gradient-to-br from-white to-neutral-50 p-6 shadow-medium dark:border-neutral-800 dark:from-neutral-900 dark:to-neutral-950"
>
	<div class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<div class="text-xs uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
				{t('results.totalCredits')}
			</div>
			<div class="mt-1 flex items-baseline gap-2">
				<div class="text-4xl font-semibold tabular-nums">
					{formatCredit(result.total_credits)}
				</div>
				<div class="text-base text-neutral-500 dark:text-neutral-400">
					/ {formatCredit(totalRequired)}
				</div>
			</div>
		</div>
		<div class="flex items-baseline gap-2">
			<div class="text-xs uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
				{t('results.gpa')}
			</div>
			<div class="text-2xl font-semibold tabular-nums">
				{formatGpa(result.gpa)}
			</div>
		</div>
	</div>

	<div class="mt-5 h-2 w-full overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-800">
		<div
			class="h-full rounded-full bg-gradient-to-r from-brand-500 to-brand-700 transition-[width] duration-700"
			style:width="{progress}%"
		></div>
	</div>
</section>
